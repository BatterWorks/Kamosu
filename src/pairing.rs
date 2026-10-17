//! Pairing: working out which line is which, by reading (ADR 0019).
//!
//! Nothing is stapled to a line of a recipe. An Ingredient Line and a Step carry
//! no id, no key and no serial number — not in the database, not in a Bundle,
//! not in the Vault. So when Kamosu needs to know that your
//! `1 cup potato starch (or corn starch)` and Marc's `¾ cup potato starch` are
//! the same line changed, rather than one line removed and another added, it
//! works it out from the words, against the **Branch Point** — the last Version
//! the two Branches shared.
//!
//! Three rules hold this up:
//!
//! 1. **The Branch Point makes the guessing surface small.** Any line neither
//!    side touched is identical on both sides and matches exactly, for free.
//!    Only the lines somebody actually edited need reading, and in a fifteen-line
//!    list that is typically one to three.
//! 2. **Kind is part of identity.** A Section heading never pairs with an
//!    Ingredient Line, and a Step never pairs with either.
//! 3. **Where the reading is uncertain, Kamosu declines.** Two lines that both
//!    arrived after the Branch Point are paired only if they genuinely resemble
//!    each other. Otherwise both stand, unjoined, and nothing is ever labelled a
//!    guess — the texts are the evidence.
//!
//! This module is mostly exercised through the `divergence` Operation against
//! real Versions in a real database, because what matters is what a cook is
//! shown, not what a matcher returns. The few tests below hold real lists that
//! once paired wrong.

use serde_json::{Value, json};

/// How alike two lines must be to be read as one line. Two numbers, on purpose.
///
/// `EDITED` compares one known Branch Point line against a line that survived on
/// one side. The candidate pool is tiny and already ordered, so a generous bar is
/// safe: `1 cup potato starch (or corn starch)` → `¾ cup potato starch` scores
/// about 0.64 and must land.
const EDITED: f64 = 0.5;

/// `ARRIVED` compares a line that is new on one side against a line that is new
/// on the other. Nothing anchors these to each other, so the bar is high. This is
/// the number that decides whether `1 Tbsp rice vinegar` and `1 tsp gochugaru`
/// — which score 0.375 — get falsely joined. The two failure modes are not
/// symmetric: an unpaired line costs a few seconds of extra reading with both
/// texts still on the page, while a bad Pairing asserts that two unrelated
/// ingredients are one ingredient and puts a *take his* button underneath it.
const ARRIVED: f64 = 0.72;

/// One line of a recipe, as it is written and stored: a kind and some text, and
/// nothing else. There is deliberately no id here to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: String,
    pub text: String,
    /// Where this line sits in the list it came from. Not an identity — it is
    /// not carried between Branches and nothing is matched by it — but the
    /// interface needs it to find the line's Reading, which is stored by index.
    pub index: usize,
}

impl Line {
    /// Read one side's list out of a Version's stored content.
    pub fn list_from(content: &Value, field: &str) -> Vec<Line> {
        content
            .get(field)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        Some(Line {
                            kind: item.get("kind")?.as_str()?.to_string(),
                            text: item.get("text")?.as_str()?.to_string(),
                            index,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn to_json(&self) -> Value {
        json!({ "kind": self.kind, "text": self.text, "index": self.index })
    }
}

/// One place in the two recipes laid over each other.
///
/// `mine` and `theirs` are the line each side has here, and either may be
/// absent. An absent side is what a Ghost is made of: a line the other recipe
/// has and this one has not, shown struck through in the position it holds over
/// there.
#[derive(Debug, Clone)]
pub struct Row {
    pub kind: String,
    pub mine: Option<Line>,
    pub theirs: Option<Line>,
    /// Whether this row descends from a line that was already at the Branch
    /// Point. It separates "the other person took this out" from "the other
    /// person never had it" — the same Ghost, but a different sentence, and only
    /// the first can be carried across as a removal.
    pub from_branch_point: bool,
}

impl Row {
    /// What this row is, decided once here so no screen has to work it out
    /// again — and so both Doors and every client agree.
    pub fn state(&self) -> &'static str {
        match (&self.mine, &self.theirs) {
            (Some(mine), Some(theirs)) => {
                if normalise(&mine.text) == normalise(&theirs.text) {
                    "same"
                } else {
                    "changed"
                }
            }
            (Some(_), None) => "only-mine",
            (None, Some(_)) => "only-theirs",
            (None, None) => "same",
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind,
            "state": self.state(),
            "from_branch_point": self.from_branch_point,
            "mine": self.mine.as_ref().map(Line::to_json),
            "theirs": self.theirs.as_ref().map(Line::to_json),
        })
    }
}

/// Case and run-of-whitespace are not a difference anybody wrote. Retyping a
/// line exactly must produce nothing at all, which is the load-bearing claim of
/// ADR 0019 and the reason a stored id was refused.
fn normalise(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Sørensen–Dice over character bigrams: 0 shares nothing, 1 is identical.
///
/// Character bigrams rather than words because the edits that matter here are
/// mostly inside a word or a number — `¾ cup` for `1 cup` still shares nearly
/// every bigram of `cup potato starch`, which is exactly the signal wanted.
fn similarity(a: &str, b: &str) -> f64 {
    let (a, b) = (normalise(a), normalise(b));
    if a == b {
        return 1.0;
    }
    let bigrams = |s: &str| -> Vec<[char; 2]> {
        let chars: Vec<char> = s.chars().collect();
        chars.windows(2).map(|w| [w[0], w[1]]).collect()
    };
    let (left, right) = (bigrams(&a), bigrams(&b));
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut remaining = left.clone();
    let mut shared = 0usize;
    for gram in &right {
        if let Some(at) = remaining.iter().position(|g| g == gram) {
            remaining.remove(at);
            shared += 1;
        }
    }
    (2.0 * shared as f64) / (left.len() + right.len()) as f64
}

/// Where did each of this side's lines come from? Answers one slot per line:
/// the index in the Branch Point's list it descends from, or `None` for a line
/// that arrived afterwards.
fn origins(base: &[Line], side: &[Line]) -> Vec<Option<usize>> {
    let mut from: Vec<Option<usize>> = vec![None; side.len()];
    let mut used = vec![false; base.len()];

    // Pass one — the lines nobody touched, matched exactly. Most of the list,
    // always, and the reason the reading that follows has so little to do.
    for (i, line) in side.iter().enumerate() {
        for (b, base_line) in base.iter().enumerate() {
            if used[b] || base_line.kind != line.kind {
                continue;
            }
            if normalise(&base_line.text) == normalise(&line.text) {
                from[i] = Some(b);
                used[b] = true;
                break;
            }
        }
    }

    // Pass two — the handful somebody edited, matched by reading. Every pair
    // that clears the bar is a candidate, and the closest are made first: taken
    // in the side's own order, a line that merely shares "the steak … minutes"
    // could take the line another one plainly rewrites (#303).
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for (i, line) in side.iter().enumerate() {
        if from[i].is_some() {
            continue;
        }
        for (b, base_line) in base.iter().enumerate() {
            if used[b] || base_line.kind != line.kind {
                continue;
            }
            let score = similarity(&base_line.text, &line.text);
            if score > EDITED {
                candidates.push((score, i, b));
            }
        }
    }
    // Closest first; a tie goes to the side's earlier line, then the Branch
    // Point's, so the same two lists always pair the same way.
    candidates.sort_by(|&(score_x, i_x, b_x), &(score_y, i_y, b_y)| {
        score_y
            .total_cmp(&score_x)
            .then((i_x, b_x).cmp(&(i_y, b_y)))
    });
    for (_, i, b) in candidates {
        if from[i].is_some() || used[b] {
            continue;
        }
        // The rows are laid on the Branch Point's order, so a pair that crosses
        // one already made would draw this side's own lines out of their order.
        // That is an uncertain reading, and an uncertain reading declines.
        let crosses = from
            .iter()
            .enumerate()
            .any(|(j, o)| o.is_some_and(|c| (j < i) != (c < b)));
        if crosses {
            continue;
        }
        from[i] = Some(b);
        used[b] = true;
    }

    from
}

/// A line that arrived after the Branch Point, remembering the last line above
/// it that both sides can still locate. That anchor is what puts a Ghost "in the
/// position it holds over there" — a best effort, and ADR 0014 says so out loud:
/// where the neighbour is not in both recipes the position is a guess, and
/// Kamosu should read well when it guesses wrong rather than assert an order it
/// cannot know.
struct Arrival<'a> {
    line: &'a Line,
    anchor: Option<usize>,
}

fn arrivals<'a>(side: &'a [Line], from: &[Option<usize>]) -> Vec<Arrival<'a>> {
    let mut out = Vec::new();
    let mut anchor = None;
    for (i, line) in side.iter().enumerate() {
        match from[i] {
            Some(b) => anchor = Some(b),
            None => out.push(Arrival { line, anchor }),
        }
    }
    out
}

/// Lay two Branches over each other by reading both against their Branch Point.
pub fn read(base: &[Line], mine: &[Line], theirs: &[Line]) -> Vec<Row> {
    let from_mine = origins(base, mine);
    let from_theirs = origins(base, theirs);

    // The spine: every Branch Point line at least one side still has, in order.
    let mut rows: Vec<Row> = Vec::new();
    let mut row_at_base: Vec<Option<usize>> = vec![None; base.len()];
    for (b, base_line) in base.iter().enumerate() {
        let mine_line = from_mine
            .iter()
            .position(|o| *o == Some(b))
            .map(|i| mine[i].clone());
        let theirs_line = from_theirs
            .iter()
            .position(|o| *o == Some(b))
            .map(|i| theirs[i].clone());
        if mine_line.is_none() && theirs_line.is_none() {
            continue; // both took it out, so there is nothing to show
        }
        row_at_base[b] = Some(rows.len());
        rows.push(Row {
            kind: base_line.kind.clone(),
            mine: mine_line,
            theirs: theirs_line,
            from_branch_point: true,
        });
    }

    // Did any of my new lines and any of theirs turn out to be the same line,
    // both of us having written it after we parted? Only if they really look it.
    let new_mine = arrivals(mine, &from_mine);
    let new_theirs = arrivals(theirs, &from_theirs);
    let mut claimed = vec![false; new_theirs.len()];
    let mut extra: Vec<(Option<usize>, Row)> = Vec::new();

    for arrival in &new_mine {
        let mut best: Option<usize> = None;
        let mut best_score = ARRIVED;
        for (j, other) in new_theirs.iter().enumerate() {
            if claimed[j] || other.line.kind != arrival.line.kind {
                continue;
            }
            let score = similarity(&arrival.line.text, &other.line.text);
            if score > best_score {
                best_score = score;
                best = Some(j);
            }
        }
        if let Some(j) = best {
            claimed[j] = true;
        }
        // Where `best` is None this is the declining branch: my line stands on
        // its own, theirs stands on its own, and neither is marked as a maybe.
        extra.push((
            arrival.anchor,
            Row {
                kind: arrival.line.kind.clone(),
                mine: Some(arrival.line.clone()),
                theirs: best.map(|j| new_theirs[j].line.clone()),
                from_branch_point: false,
            },
        ));
    }
    for (j, arrival) in new_theirs.iter().enumerate() {
        if claimed[j] {
            continue;
        }
        extra.push((
            arrival.anchor,
            Row {
                kind: arrival.line.kind.clone(),
                mine: None,
                theirs: Some(arrival.line.clone()),
                from_branch_point: false,
            },
        ));
    }

    // Slot each arrival in under its anchor, keeping the order it was written
    // in. A line with no anchor at all was written above everything both sides
    // still share, so it goes to the top.
    let mut offsets: Vec<(Option<usize>, usize)> = Vec::new();
    for (anchor, row) in extra {
        let offset = {
            let slot = offsets.iter_mut().find(|(a, _)| *a == anchor);
            match slot {
                Some((_, n)) => {
                    *n += 1;
                    *n
                }
                None => {
                    offsets.push((anchor, 1));
                    1
                }
            }
        };
        let at = match anchor.and_then(|b| row_at_base[b]) {
            Some(index) => index + offset,
            None => offset - 1,
        };
        let target = at.min(rows.len());
        rows.insert(target, row);
        for slot in row_at_base.iter_mut().flatten() {
            if *slot >= target {
                *slot += 1;
            }
        }
    }

    rows
}

/// A single value — Title, Yield, times, Source, Note — is same or not, with
/// nothing to pair and nothing to get wrong (ADR 0019). Photographs compare
/// exactly for free, since a Photograph is known by its own contents (ADR 0017).
pub fn compare_field(mine: &Value, theirs: &Value) -> Value {
    json!({
        "same": mine == theirs,
        "mine": mine.clone(),
        "theirs": theirs.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(texts: &[&str]) -> Vec<Line> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| Line {
                kind: "step".to_string(),
                text: text.to_string(),
                index,
            })
            .collect()
    }

    /// #303: a variation that reworded all six Steps of its source. Read in the
    /// side's own order, Step 3 took the source's rest Step on shared words, and
    /// Step 6 then paired with the source's first Step and was drawn on top.
    #[test]
    fn a_reworded_method_keeps_its_own_order() {
        let source = steps(&[
            "Take the steak out of the fridge about 45 minutes before cooking.",
            "Preheat the oven to 200 °C on 4D hot air.",
            "Pat the steak dry. Salt it just before it goes in the pan.",
            "Heat the cast iron pan until it is smoking hot. Sear the steak 2½ minutes per side.",
            "Put the pan in the oven for 4 minutes.",
            "Rest the steak for 8 minutes before cutting.",
        ]);
        let variation = steps(&[
            "Salt the steak and leave it uncovered in the fridge for 24 hours to dry.",
            "Put the oil in a cast iron pan on the wok burner at 5 and heat until it smokes.",
            "Stand the steak on its fat edge for 1½ minutes, still at 5.",
            "Lay the steak flat and cook for 4 minutes at 5, flipping every 30 seconds.",
            "Turn the burner down to 4 and cook for 1 more minute, still flipping every 30 seconds.",
            "Rest the steak for 5 minutes before cutting.",
        ]);

        let rows = read(&source, &variation, &source);

        let order: Vec<usize> = rows
            .iter()
            .filter_map(|row| row.mine.as_ref().map(|line| line.index))
            .collect();
        assert_eq!(order, vec![0, 1, 2, 3, 4, 5]);

        let rest = rows
            .iter()
            .find(|row| row.mine.as_ref().is_some_and(|line| line.index == 5))
            .unwrap();
        assert_eq!(
            rest.theirs.as_ref().map(|line| line.text.as_str()),
            Some("Rest the steak for 8 minutes before cutting.")
        );
    }

    /// Two Steps swapped and each reworded: both pairs clear the bar, but made
    /// together they cross, and the second would draw this side's Step 2 above
    /// its Step 1. The closer pair stands and the other declines.
    #[test]
    fn a_pair_that_crosses_one_already_made_declines() {
        let source = steps(&[
            "Preheat the oven to 200 °C.",
            "Sear the steak 2 minutes per side.",
        ]);
        let swapped = steps(&[
            "Sear the steak 3 minutes per side.",
            "Preheat the oven to 180 °C on fan.",
        ]);

        let rows = read(&source, &swapped, &source);

        let order: Vec<usize> = rows
            .iter()
            .filter_map(|row| row.mine.as_ref().map(|line| line.index))
            .collect();
        assert_eq!(order, vec![0, 1]);
        let paired: Vec<(usize, usize)> = rows
            .iter()
            .filter_map(|row| Some((row.mine.as_ref()?.index, row.theirs.as_ref()?.index)))
            .collect();
        assert_eq!(paired, vec![(0, 1)]);
    }
}
