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
    /// Where this row is drawn on each side's page (#337). Each page reads its
    /// own lines in its own order, with the other side's Ghosts slotted in, so
    /// a line one cook moved sits in a different place on each.
    pub place_mine: usize,
    pub place_theirs: usize,
    /// Both sides have this line word for word, in places the rest of the two
    /// lists do not agree on: somebody moved it.
    pub moved: bool,
}

impl Row {
    fn new(kind: &str, mine: Option<Line>, theirs: Option<Line>, from_branch_point: bool) -> Row {
        Row {
            kind: kind.to_string(),
            mine,
            theirs,
            from_branch_point,
            place_mine: 0,
            place_theirs: 0,
            moved: false,
        }
    }

    /// What this row is, decided once here so no screen has to work it out
    /// again — and so both Doors and every client agree.
    pub fn state(&self) -> &'static str {
        if self.moved {
            return "moved";
        }
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
            "place": { "mine": self.place_mine, "theirs": self.place_theirs },
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
    // The longest run in the same order on both is matched first, so a line
    // written twice pairs with the copy in its own place (#337).
    let words = |lines: &[Line]| -> Vec<(String, String)> {
        lines
            .iter()
            .map(|line| (line.kind.clone(), normalise(&line.text)))
            .collect()
    };
    let (side_words, base_words) = (words(side), words(base));
    let mut run = vec![vec![0usize; base.len() + 1]; side.len() + 1];
    for i in (0..side.len()).rev() {
        for b in (0..base.len()).rev() {
            run[i][b] = if side_words[i] == base_words[b] {
                run[i + 1][b + 1] + 1
            } else {
                run[i + 1][b].max(run[i][b + 1])
            };
        }
    }
    let (mut i, mut b) = (0, 0);
    while i < side.len() && b < base.len() {
        if side_words[i] == base_words[b] {
            from[i] = Some(b);
            used[b] = true;
            (i, b) = (i + 1, b + 1);
        } else if run[i + 1][b] >= run[i][b + 1] {
            i += 1;
        } else {
            b += 1;
        }
    }
    // What is left that still matches exactly was moved word for word.
    for (i, line) in side_words.iter().enumerate() {
        if from[i].is_some() {
            continue;
        }
        if let Some(b) = (0..base.len()).find(|&b| !used[b] && base_words[b] == *line) {
            from[i] = Some(b);
            used[b] = true;
        }
    }
    let exact: Vec<bool> = from.iter().map(Option::is_some).collect();

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
        // Two reworded lines whose pairs cross would be lines both moved and
        // reworded. That is an uncertain reading, and an uncertain reading
        // declines (#303). Against a line matched exactly, which of the two
        // moved is the page's to say (#337).
        let crosses = from
            .iter()
            .enumerate()
            .any(|(j, o)| !exact[j] && o.is_some_and(|c| (j < i) != (c < b)));
        if crosses {
            continue;
        }
        from[i] = Some(b);
        used[b] = true;
    }

    from
}

/// Lay two Branches over each other by reading both against their Branch Point.
///
/// The answer is in the order of `mine`'s page, and every row says where it is
/// drawn on each page. A Ghost — a line only the other side has — sits under
/// the line it follows over there, counted among the lines that sit in the
/// same place on both pages. That is a best effort, and ADR 0014 says so out
/// loud: Kamosu should read well when it guesses wrong rather than assert an
/// order it cannot know.
pub fn read(base: &[Line], mine: &[Line], theirs: &[Line]) -> Vec<Row> {
    let from_mine = origins(base, mine);
    let from_theirs = origins(base, theirs);
    let mut rows: Vec<Row> = Vec::new();

    // Every Branch Point line at least one side still has.
    for (b, base_line) in base.iter().enumerate() {
        let mine_line = from_mine.iter().position(|o| *o == Some(b));
        let theirs_line = from_theirs.iter().position(|o| *o == Some(b));
        if mine_line.is_none() && theirs_line.is_none() {
            continue; // both took it out, so there is nothing to show
        }
        rows.push(Row::new(
            &base_line.kind,
            mine_line.map(|i| mine[i].clone()),
            theirs_line.map(|j| theirs[j].clone()),
            true,
        ));
    }

    // Did any of my new lines and any of theirs turn out to be the same line,
    // both of us having written it after we parted? Only if they really look it.
    let new_theirs: Vec<&Line> = theirs
        .iter()
        .zip(&from_theirs)
        .filter(|(_, o)| o.is_none())
        .map(|(line, _)| line)
        .collect();
    let mut claimed = vec![false; new_theirs.len()];
    for (line, _) in mine.iter().zip(&from_mine).filter(|(_, o)| o.is_none()) {
        let mut best: Option<usize> = None;
        let mut best_score = ARRIVED;
        for (j, other) in new_theirs.iter().enumerate() {
            if claimed[j] || other.kind != line.kind {
                continue;
            }
            let score = similarity(&line.text, &other.text);
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
        rows.push(Row::new(
            &line.kind,
            Some(line.clone()),
            best.map(|j| new_theirs[j].clone()),
            false,
        ));
    }
    for (j, line) in new_theirs.iter().enumerate() {
        if !claimed[j] {
            rows.push(Row::new(&line.kind, None, Some((*line).clone()), false));
        }
    }

    // Which pairs sit in the same place on both pages: the longest run of them
    // in the same order on both sides. Every other pair was moved (#337), and
    // marking only those marks the fewest lines that explain the new order.
    let mut pairs: Vec<(usize, usize, usize)> = rows
        .iter()
        .enumerate()
        .filter_map(|(r, row)| Some((row.mine.as_ref()?.index, row.theirs.as_ref()?.index, r)))
        .collect();
    pairs.sort();
    // Where two runs are as long, the one keeping more reworded pairs in place
    // wins: a reworded pair out of place is split in two below, and one moved
    // word for word stays a single marked line. Then the one keeping the
    // Branch Point's earlier lines, which both Branches see alike, so the
    // same line reads as moved whichever of the two asks.
    let rank = |row: &Row| {
        row.mine
            .as_ref()
            .and_then(|line| from_mine[line.index])
            .filter(|_| row.from_branch_point)
            .unwrap_or(base.len())
    };
    let rising = longest_rising(
        &pairs
            .iter()
            .map(|&(_, j, r)| (j, rows[r].state() == "changed", rank(&rows[r])))
            .collect::<Vec<_>>(),
    );
    let mut in_place = vec![false; rows.len()];
    for (at, &(_, _, r)) in pairs.iter().enumerate() {
        if rising.contains(&at) {
            in_place[r] = true;
        }
    }

    // A moved line reworded as well is two lines, as a reworded line that
    // crosses another is (#303): an uncertain reading declines. As there, a
    // half still word for word the Branch Point's line descends from it, and
    // a reworded half reads as written after parting.
    let kept = |line: &Line, from: &[Option<usize>]| {
        from[line.index].is_some_and(|b| normalise(&base[b].text) == normalise(&line.text))
    };
    for &(_, _, r) in &pairs {
        if in_place[r] {
            continue;
        }
        let row = &mut rows[r];
        let (mine_line, theirs_line) = (row.mine.as_ref().unwrap(), row.theirs.as_ref().unwrap());
        if normalise(&mine_line.text) == normalise(&theirs_line.text) {
            row.moved = true;
            continue;
        }
        let theirs_kept = kept(theirs_line, &from_theirs);
        row.from_branch_point = kept(mine_line, &from_mine);
        let (kind, theirs_line) = (row.kind.clone(), row.theirs.take());
        rows.push(Row::new(&kind, None, theirs_line, theirs_kept));
    }

    let order_mine = page(&rows, |row| row.mine.as_ref(), |row| row.theirs.as_ref());
    let order_theirs = page(&rows, |row| row.theirs.as_ref(), |row| row.mine.as_ref());
    for (at, &r) in order_mine.iter().enumerate() {
        rows[r].place_mine = at;
    }
    for (at, &r) in order_theirs.iter().enumerate() {
        rows[r].place_theirs = at;
    }
    rows.sort_by_key(|row| row.place_mine);
    rows
}

/// One side's page: its own lines in its own order, and each of the other
/// side's Ghosts straight under the line it follows over there, Ghosts that
/// follow one another kept in their order. A Ghost that follows no line of
/// this page goes to the top.
fn page<'a>(
    rows: &'a [Row],
    own: impl Fn(&'a Row) -> Option<&'a Line>,
    other: impl Fn(&'a Row) -> Option<&'a Line>,
) -> Vec<usize> {
    let mut own_rows: Vec<(usize, usize)> = Vec::new();
    let mut other_rows: Vec<(usize, usize)> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        if let Some(line) = own(row) {
            own_rows.push((line.index, r));
        }
        if let Some(line) = other(row) {
            other_rows.push((line.index, r));
        }
    }
    own_rows.sort();
    other_rows.sort();

    // Each Ghost under the last line above it over there that this page has.
    let mut under: Vec<(Option<usize>, usize)> = Vec::new();
    let mut anchor = None;
    for &(_, r) in &other_rows {
        if own(&rows[r]).is_none() {
            under.push((anchor, r));
        } else {
            anchor = Some(r);
        }
    }
    let ghosts = |anchor: Option<usize>| {
        under
            .iter()
            .filter(move |(a, _)| *a == anchor)
            .map(|&(_, r)| r)
    };

    let mut out: Vec<usize> = ghosts(None).collect();
    for &(_, r) in &own_rows {
        out.push(r);
        out.extend(ghosts(Some(r)));
    }
    out
}

/// The positions of the longest strictly rising run of `(value, marked,
/// rank)`, by value. Among runs as long, the one holding the most marked,
/// then the lowest ranks, then the earliest.
fn longest_rising(values: &[(usize, bool, usize)]) -> Vec<usize> {
    // `best[k]` is the run ending at k, as (length, marked held, ranks held
    // reversed), and `back[k]` the step before.
    type Score = (usize, usize, std::cmp::Reverse<usize>);
    let score = |&(_, marked, rank): &(usize, bool, usize)| -> Score {
        (1, usize::from(marked), std::cmp::Reverse(rank))
    };
    let mut best: Vec<Score> = values.iter().map(score).collect();
    let mut back: Vec<Option<usize>> = vec![None; values.len()];
    for k in 0..values.len() {
        for p in 0..k {
            let here = score(&values[k]);
            let through = (
                best[p].0 + here.0,
                best[p].1 + here.1,
                std::cmp::Reverse(best[p].2.0 + here.2.0),
            );
            if values[p].0 < values[k].0 && through > best[k] {
                best[k] = through;
                back[k] = Some(p);
            }
        }
    }
    let Some(mut at) = (0..values.len()).max_by_key(|&k| (best[k], std::cmp::Reverse(k))) else {
        return Vec::new();
    };
    let mut run = vec![at];
    while let Some(p) = back[at] {
        run.push(p);
        at = p;
    }
    run.reverse();
    run
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

    /// The order each side's page reads its rows in, by that side's `place`.
    fn page(rows: &[Row], mine: bool) -> Vec<String> {
        let mut seen: Vec<(usize, String)> = rows
            .iter()
            .map(|row| {
                let at = if mine {
                    row.place_mine
                } else {
                    row.place_theirs
                };
                let own = if mine { &row.mine } else { &row.theirs };
                let text = match own {
                    Some(line) => line.text.clone(),
                    None => format!(
                        "~{}",
                        row.mine.as_ref().or(row.theirs.as_ref()).unwrap().text
                    ),
                };
                (at, text)
            })
            .collect();
        seen.sort();
        seen.into_iter().map(|(_, text)| text).collect()
    }

    /// #337: a Step dragged to the top and not reworded. Each page reads its
    /// own order, and only the Step that moved is marked.
    #[test]
    fn a_moved_step_keeps_each_sides_own_order() {
        let source = steps(&["A", "B", "C"]);
        let variation = steps(&["C", "A", "B"]);

        let rows = read(&source, &variation, &source);

        assert_eq!(page(&rows, true), vec!["C", "A", "B"]);
        assert_eq!(page(&rows, false), vec!["A", "B", "C"]);
        let states: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (row.mine.as_ref().unwrap().text.as_str(), row.state()))
            .collect();
        assert_eq!(states, vec![("C", "moved"), ("A", "same"), ("B", "same")]);
        // The answer's own order is the first side's page.
        let places: Vec<usize> = rows.iter().map(|row| row.place_mine).collect();
        assert_eq!(places, vec![0, 1, 2]);
    }

    /// A line moved on one side and reworded on the other is two lines, as a
    /// reworded line that crosses is (#303): new where it sits, gone where it
    /// sat. The half still word for word the Branch Point's descends from it.
    #[test]
    fn a_moved_line_reworded_on_the_other_side_is_two_lines() {
        let source = steps(&[
            "Preheat the oven.",
            "Chop the onions.",
            "Fry the onions slowly.",
        ]);
        let moved = steps(&[
            "Fry the onions slowly.",
            "Preheat the oven.",
            "Chop the onions.",
        ]);
        let reworded = steps(&[
            "Preheat the oven.",
            "Chop the onions.",
            "Fry the onions slowly in butter.",
        ]);

        let rows = read(&source, &moved, &reworded);

        assert_eq!(
            page(&rows, true),
            vec![
                "Fry the onions slowly.",
                "Preheat the oven.",
                "Chop the onions.",
                "~Fry the onions slowly in butter."
            ]
        );
        assert_eq!(
            page(&rows, false),
            vec![
                "~Fry the onions slowly.",
                "Preheat the oven.",
                "Chop the onions.",
                "Fry the onions slowly in butter."
            ]
        );
        assert!(rows.iter().all(|row| row.state() != "moved"));
        let lineage = |text: &str| {
            rows.iter()
                .find(|row| {
                    row.mine.as_ref().or(row.theirs.as_ref()).unwrap().text == text
                        && (row.mine.is_none() || row.theirs.is_none())
                })
                .unwrap()
                .from_branch_point
        };
        assert!(lineage("Fry the onions slowly."));
        assert!(!lineage("Fry the onions slowly in butter."));
    }

    /// A swap on one side while the other rewords one of the two: keeping the
    /// reworded pair in place reads as one line changed and one moved, rather
    /// than one moved and two halves of a split.
    #[test]
    fn a_tie_keeps_the_reworded_pair_in_place() {
        let source = steps(&["Preheat the oven to 200 °C.", "Sear the steak."]);
        let swapped = steps(&["Sear the steak.", "Preheat the oven to 200 °C."]);
        let reworded = steps(&["Preheat the oven to 180 °C.", "Sear the steak."]);

        let rows = read(&source, &swapped, &reworded);

        let mut states: Vec<&str> = rows.iter().map(Row::state).collect();
        states.sort();
        assert_eq!(states, vec!["changed", "moved"]);
    }

    /// A Ghost sits straight under the line it follows over there, wherever
    /// this page has put that line.
    #[test]
    fn a_ghost_follows_the_line_it_follows_over_there() {
        let source = steps(&["A", "B", "C"]);
        let mine = steps(&["C", "A", "B"]);
        let theirs = steps(&["A", "B", "C", "D"]);

        let rows = read(&source, &mine, &theirs);

        assert_eq!(page(&rows, true), vec!["C", "~D", "A", "B"]);
        assert_eq!(page(&rows, false), vec!["A", "B", "C", "D"]);
    }

    /// Both sides wrote the same new line after parting, in different places
    /// (#337's rarer case). It is one line, drawn where each side put it.
    #[test]
    fn a_line_both_sides_added_in_different_places_is_moved() {
        let source = steps(&["A", "B"]);
        let mine = steps(&["Season to taste.", "A", "B"]);
        let theirs = steps(&["A", "B", "Season to taste."]);

        let rows = read(&source, &mine, &theirs);

        assert_eq!(page(&rows, true), vec!["Season to taste.", "A", "B"]);
        assert_eq!(page(&rows, false), vec!["A", "B", "Season to taste."]);
        assert_eq!(rows[0].state(), "moved");
        assert!(!rows[0].from_branch_point);
    }

    /// A Step moved over another that was only reworded: the reworded one is
    /// still the same line changed, not declined as a crossing.
    #[test]
    fn a_reworded_step_under_a_moved_one_still_pairs() {
        let source = steps(&[
            "Make the sauce.",
            "Coat the chicken.",
            "Fry the chicken until golden.",
        ]);
        let variation = steps(&[
            "Fry the chicken until golden.",
            "Make the sauce.",
            "Coat the chicken in panko.",
        ]);

        let rows = read(&source, &variation, &source);

        let states: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (row.mine.as_ref().unwrap().text.as_str(), row.state()))
            .collect();
        assert_eq!(
            states,
            vec![
                ("Fry the chicken until golden.", "moved"),
                ("Make the sauce.", "same"),
                ("Coat the chicken in panko.", "changed"),
            ]
        );
    }

    /// A line written twice, one of them taken out: nothing moved.
    #[test]
    fn a_repeated_line_taken_out_once_moves_nothing() {
        let source = steps(&["A", "1 tsp salt", "B", "1 tsp salt"]);
        let mine = steps(&["A", "B", "1 tsp salt"]);

        let rows = read(&source, &mine, &source);

        assert!(rows.iter().all(|row| row.state() != "moved"), "{rows:#?}");
        assert_eq!(
            page(&rows, true),
            vec!["A", "~1 tsp salt", "B", "1 tsp salt"]
        );
    }

    /// Two Steps swapped are the same rows asked from either Branch: the
    /// same one is marked moved.
    #[test]
    fn a_swap_marks_the_same_line_from_either_side() {
        let source = steps(&["A", "B"]);
        let swapped = steps(&["B", "A"]);

        let moved = |rows: Vec<Row>| -> Vec<String> {
            rows.into_iter()
                .filter(|row| row.state() == "moved")
                .map(|row| row.mine.unwrap().text)
                .collect()
        };
        assert_eq!(
            moved(read(&source, &swapped, &source)),
            moved(read(&source, &source, &swapped))
        );
    }
}
