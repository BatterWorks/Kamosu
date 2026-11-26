//! **Which Ingredient Lines a Step uses**, read out of the Step's own words
//! and the Readings beside it (ADR 0011, GLOSSARY.md "Step"). Nothing is stored
//! and nobody types a link (ADR 0019): this runs on every read of a Version.
//!
//! A recipe names an ingredient in full once, in its list, and shortens it
//! from then on. *Maple syrup* is *the syrup* by the second Step and *cacao en
//! poudre* is *le cacao*, so a Step that had to repeat the whole name linked
//! almost nothing (#184). Here a run of words inside the name counts, as
//! long as it holds a word a cook would call the food by
//! ([`reading::names_nothing_alone`] is the rest) and the name's main word,
//! the thing the Food is (#212): *vinegar* stands for *rice vinegar*, and
//! *rice* does not. Four rules then keep that looseness from linking what
//! the Step never adds:
//!
//! - **The longer mention wins.** *The vegetable oil* is one mention of the
//!   vegetable oil, not also one of the sesame oil's *oil*. A match on an
//!   ingredient's whole name always stands, as it did before #184.
//! - **One food on two lines links one of them.** Soy sauce in the filling and
//!   again in the dipping sauce: a Step naming it takes the first such line no
//!   earlier Step has used, or the first line again when every one is used.
//!   Two *different* foods sharing a short word (*sugar* with brown and white
//!   sugar) are both linked, and the cook picks. Both were chosen
//!   on #184.
//! - **What comes after *until* is not going in.** *Steam until most of the
//!   water has evaporated* adds no water, so a mention after *until*,
//!   *jusqu'à* or *hasta* counts for nothing up to the next comma or full
//!   stop, or the next thing to do: *until soft and add the garlic* adds the
//!   garlic (#294). *Besides* and *except* leave a food out the same way, and
//!   *more chili oil* after a Step that used the chili oil is beyond the
//!   line's amount, so it is not listed again (#697).
//! - **A word the title holds names the dish.** *Ladle the curry* in a
//!   *Vietnamese beef curry* serves the dish, not the curry powder, so a part
//!   of a name that the recipe's title also holds links nothing. The whole
//!   name still does: *add the garlic* in *Garlic bread* adds the garlic. The
//!   price, chosen on #357, is *add the lemon* in a *Lemon tart* no longer
//!   linking the lemon zest; a missing link leaves the line on screen, where a
//!   wrong one puts curry powder in the serving Step.
//!
//! Whole words only, as always: *rice* is never found in *price*, nor *ail* in
//! half the French language. One trailing `s` on a word is forgiven, so a line
//! read as *egg* is used by a Step that says *eggs*.
//!
//! A line offering a choice, *coconut milk or heavy cream*, has a Reading that
//! names no Food (#148), and a Step finds it by either option, each read as a
//! line of its own. A Step may also call a food something its name never
//! holds, *the mince* for *ground pork*, from the word files'
//! `step_other_names`. Both came from #697.
//!
//! A line an earlier Step cooked is the cooked thing from then on, and a
//! later Step naming it again adds nothing (#704): *the cooked mince*, *put
//! the meat back*, or *combine the noodles* after *boil noodles*. A Step that
//! only cut it, *cut the meat*, or took a part of it, *a little salt*, leaves
//! the line to go in later, and so does a later Step taking a part, *the
//! green portion of scallions*. Nor does a word beside a part of a name that
//! says another kind of the food, *a sweet wine* against the line's *red
//! wine*, find the line.

use std::ops::Range;

use serde_json::Value;

use crate::core::folded_for_search;
use crate::dutch;
use crate::language::UNKNOWN;
use crate::reading;
use crate::words::{self, Grammar, Words};

/// Whether a folded word is one of [`Words::step_for`] on a Branch in `language`.
fn opens_a_purpose(lists: &Words, word: &str, language: Option<&str>) -> bool {
    lists.step_for.has(word) || (language == Some("fr") && word == "pour")
}

/// What ends such a clause: a comma, the end of a sentence, or anything that
/// sets words apart as firmly as one.
const CLAUSE_ENDS: &[char] = &[',', '.', ';', ':', '!', '?', '(', ')', '\n'];

/// What ends a sentence, which a [`Words::step_back`] word reaches across
/// commas: *put the meat, speck and onions back*.
const SENTENCE_ENDS: &[char] = &['.', ';', '!', '?', '\n'];

/// One word of folded text, whether it sits in an *until* or a *when* clause,
/// and the clause and sentence it sits in.
struct Word {
    text: String,
    not_going_in: bool,
    clause: usize,
    sentence: usize,
}

/// What sets two foods apart inside one name, as *salt & pepper* and
/// *thyme, basil* are written. A word that joins two foods does the same
/// ([`reading::joins_two_foods`]).
const NAME_BREAKS: &[char] = &[',', '&', '/', '+', ';', '(', ')'];

/// The folded words of `text`, split at anything that is not a letter, a
/// digit or an accent typed as its own character, each with the number of
/// `breaks` that came before it.
fn split_words(lists: &Words, text: &str, breaks: &[char]) -> Vec<(String, usize)> {
    use unicode_normalization::char::is_combining_mark;
    let mut words: Vec<(String, usize)> = Vec::new();
    let mut part = 0;
    let mut current = String::new();
    let mut close = |current: &mut String, part: usize| {
        let word = folded_word(lists, &std::mem::take(current));
        if !word.is_empty() {
            words.push((word, part));
        }
    };
    for c in text.chars() {
        // Dutch writes a plural with an apostrophe, *paprika's*, one word.
        let in_a_word = lists.is_dutch() && matches!(c, '\'' | '\u{2019}');
        if c.is_alphanumeric() || is_combining_mark(c) || in_a_word {
            current.push(c);
            continue;
        }
        close(&mut current, part);
        if breaks.contains(&c) {
            part += 1;
        }
    }
    close(&mut current, part);
    words
}

/// One word folded for comparison: case and accents gone, as a search folds
/// it. A word [`reading::accented_name`] knows keeps its accent, since the
/// accent is all that tells *thé* from *the* (#230).
fn folded_word(lists: &Words, word: &str) -> String {
    let plural = word
        .strip_suffix(['s', 'S'])
        .and_then(|word| reading::accented_name(lists, word));
    match (reading::accented_name(lists, word), plural) {
        (Some(name), _) => name.to_string(),
        (None, Some(name)) => format!("{name}s"),
        (None, None) => folded_for_search(word),
    }
}

/// Whether [`folded_word`] kept this word's accent, one trailing `s` forgiven.
fn kept_its_accent(lists: &Words, word: &str) -> bool {
    reading::accented_name(lists, word).is_some()
        || reading::accented_name(lists, singular(word)).is_some()
}

/// The folded words of a Step, each marked with whether an [`Words::step_until`] or a
/// [`Words::step_when`] clause holds it.
fn words_of(lists: &Words, text: &str) -> Vec<Word> {
    // Each word, and the clause it sits in.
    let words = split_words(lists, text, CLAUSE_ENDS);
    let sentences = split_words(lists, text, SENTENCE_ENDS);

    let mut marked = Vec::with_capacity(words.len());
    let mut until_in: Option<usize> = None;
    // Whether the clause open is a *when* clause, which also ends at the next
    // thing to do with no comma before it: *when the butter melts add the
    // flour* (#439).
    let mut when_open = false;
    for (at, (text, clause)) in words.iter().enumerate() {
        let next_action = lists.step_then.has(text)
            || (when_open && lists.step_next_actions.has(text))
            || (lists.step_and.has(text)
                && words
                    .get(at + 1)
                    .is_some_and(|(next, _)| lists.step_next_actions.has(next)));
        if until_in.is_some_and(|open| open != *clause) || next_action {
            until_in = None;
            when_open = false;
        }
        // *Besides the thyme* leaves the thyme out as *until* does (#697),
        // and *except 2 tbsp of the butter* adds the rest of it.
        let leaves_out = lists.step_until.has(text) || lists.step_except.has(text);
        if until_in.is_none() && leaves_out {
            let next = words[at + 1..]
                .iter()
                .find(|(word, _)| word != "a")
                .map(|(word, _)| word.as_str());
            let an_amount = next.is_some_and(|word| word.starts_with(|c: char| c.is_ascii_digit()));
            if !an_amount {
                until_in = Some(*clause);
            }
        }
        if until_in.is_none() && lists.step_when.has(text) {
            let doing = !lists.is_dutch()
                && words
                    .get(at + 1)
                    .is_some_and(|(next, _)| next.ends_with("ing"));
            if !doing {
                until_in = Some(*clause);
                when_open = true;
            }
        }
        marked.push(Word {
            text: text.clone(),
            not_going_in: until_in.is_some(),
            clause: *clause,
            sentence: sentences[at].1,
        });
    }
    marked
}

/// Two folded words are one word here if they differ by at most one trailing
/// `s` — the plural tolerance a Step has always had. An accent
/// [`folded_word`] kept is not looked at, so *the vert* typed without it is
/// still *thé vert*.
fn same_word(lists: &Words, a: &str, b: &str) -> bool {
    if lists.is_dutch() {
        return dutch::stem(lists, a) == dutch::stem(lists, b);
    }
    same_to_the_accent(a, b)
        // Only a kept accent is left to tell two folded words apart.
        || ((kept_its_accent(lists, a) || kept_its_accent(lists, b))
            && same_to_the_accent(&folded_for_search(a), &folded_for_search(b)))
}

/// Two folded words that differ by at most one trailing `s`, an accent
/// [`folded_word`] kept counting as a difference: *thé* and *thés* are one
/// word here, and *the* is another.
fn same_to_the_accent(a: &str, b: &str) -> bool {
    a == b || singular(a) == b || singular(b) == a || plural_in_oes(a, b) || plural_in_oes(b, a)
}

/// Whether `plural` is `word` written with English's `-oes`: *tomatoes* is
/// *tomato*, though *sloes* is still *sloe* by its `s` (#439).
fn plural_in_oes(plural: &str, word: &str) -> bool {
    word.ends_with('o') && plural.strip_suffix("es") == Some(word)
}

/// A word with one trailing `s` taken off, for telling whether two lines hold
/// one food.
fn singular(word: &str) -> &str {
    word.strip_suffix('s')
        .filter(|w| !w.is_empty())
        .unwrap_or(word)
}

/// One Ingredient Line a Step could name: its index, every run of its name's
/// folded words that may stand for it, the key two lines holding the same
/// food share, and the section of the list it sits in.
struct Line {
    index: usize,
    runs: Vec<Run>,
    food: String,
    section: usize,
    /// Whether the name holds one food, where *salt and pepper* holds two.
    one_food: bool,
    /// Whether the line offers a choice and is named by its options (#697).
    choice: bool,
}

/// A run of a name's words that may stand for the line: whether it is the
/// whole name, and whether a word of the name it leaves out makes the line
/// something [`reading::made_from`] the food the run names (#439).
struct Run {
    words: Vec<String>,
    whole: bool,
    made_from: bool,
    /// Whether the run is a name the line's own words never hold, one of
    /// [`Words::step_other_names`] (#697).
    other_name: bool,
    /// The words of the name the run leaves out that say which kind of the
    /// food it is, the *red* of *red wine*: a Step naming another kind,
    /// *a sweet wine*, names another food (#704).
    kinds_left_out: Vec<String>,
}

/// The folded words of a name, and where each food in it begins and ends: a
/// name is one food unless a [`NAME_BREAKS`] mark or a word that
/// [`reading::joins_two_foods`] sets two apart, as in *fresh thyme and basil*.
/// A split made in error only loosens: it gives a name one main word more.
fn name_words(lists: &Words, target: &str) -> (Vec<String>, Vec<Range<usize>>) {
    let mut words = Vec::new();
    let mut foods: Vec<Range<usize>> = Vec::new();
    let mut last_part = None;
    for (at, (word, part)) in split_words(lists, target, NAME_BREAKS)
        .into_iter()
        .enumerate()
    {
        let joins = reading::joins_two_foods(lists, &word);
        if joins || last_part != Some(part) {
            foods.push(at..at);
        }
        last_part = Some(part);
        foods.last_mut().expect("just pushed").end = at + 1;
        words.push(word);
    }
    (words, foods)
}

/// Which words of a name are its **main words**: the thing the Food is, which
/// a part of the name must hold to stand for it (#212). *Vinegar* in *rice
/// vinegar*, so a Step's *steamed rice* is no mention of it.
///
/// Which end the main word sits at is the Language's: the last word in
/// English, the first in French and Spanish (*vinaigre de riz*, *vinagre de
/// arroz*). Two things move it in English. A name written with *of* is read
/// from the front, *bicarbonate of soda* being *the bicarbonate*. And what
/// follows a word that [`reading::opens_a_tail`] is no part of the name, so
/// *oil for frying* is *the oil*. A Branch with no Language, or one stated
/// Unknown, is read from both ends.
///
/// From that end, a word that [`reading::gives_way`] is a main word and so is
/// the next one in: *thighs* and *chicken* in *chicken thighs*, *hauts*,
/// *cuisse* and *poulet* in *hauts de cuisse de poulet*. Words that only join
/// are stepped over. A word that [`reading::names_its_kind`] is a main word
/// wherever it sits. A name holding two foods has the main words of each.
fn main_words(
    lists: &Words,
    words: &[String],
    foods: &[Range<usize>],
    language: Option<&str>,
) -> Vec<bool> {
    let mut main: Vec<bool> = words
        .iter()
        .map(|word| reading::names_its_kind(lists, word))
        .collect();
    let mut walk = |inward: &mut dyn Iterator<Item = usize>| {
        for at in inward {
            if reading::only_joins(lists, &words[at]) {
                continue;
            }
            // *Dés de jambon* is called *les dés* and *le jambon* alike, so
            // the walk marks such a word and goes on past it, to the word
            // that was the main one while this passed for a joining word
            // (#230).
            if kept_its_accent(lists, &words[at]) {
                main[at] = true;
                continue;
            }
            main[at] = true;
            if !reading::gives_way(lists, &words[at]) {
                break;
            }
        }
    };
    for food in foods {
        // The food as English reads it: up to the first word that opens a tail.
        let tail = food
            .clone()
            .find(|&at| at > food.start && reading::opens_a_tail(lists, &words[at]))
            .unwrap_or(food.end);
        let headed = food.start..tail;
        let written_with_of = words[headed.clone()].iter().any(|word| word == "of");
        match language {
            Some("fr" | "es") => walk(&mut food.clone()),
            Some("en") if written_with_of => walk(&mut headed.clone()),
            Some("en" | "nl") => walk(&mut headed.rev()),
            _ => {
                walk(&mut food.clone());
                walk(&mut headed.rev());
            }
        }
    }
    main
}

/// The runs of a name's words a Step may name it by: the whole name, and
/// every shorter run that [`names_the_food`] allows and that holds one of the
/// name's [`main_words`]. Worked out once per line rather than once per Step,
/// since it depends on nothing a Step says.
fn runs_of(lists: &Words, target: &str, language: Option<&str>) -> (Vec<String>, Vec<Run>) {
    let (words, foods) = name_words(lists, target);
    let n = words.len();
    let main = main_words(lists, &words, &foods, language);
    let mut runs = Vec::new();
    for from in 0..n {
        for to in (from + 1)..=n {
            let run = &words[from..to];
            let whole = from == 0 && to == n;
            if whole || (main[from..to].contains(&true) && names_the_food(lists, run)) {
                let left_out = || words[..from].iter().chain(&words[to..]);
                runs.push(Run {
                    words: run.to_vec(),
                    whole,
                    made_from: left_out().any(|word| reading::made_from(lists, word)),
                    other_name: false,
                    kinds_left_out: left_out()
                        .filter(|word| lists.step_kinds.has(word))
                        .cloned()
                        .collect(),
                });
            }
        }
    }
    // A name a Step calls the food by, where the line's name holds one of
    // the names it stands for, a main word of it among them: *the mince* of
    // *ground pork* (#697), and *the meat* of the beef but not of the beef
    // stock cubes (#704).
    for (other, stands_for) in &lists.step_other_names {
        let holds = stands_for.iter().any(|name| {
            let name: Vec<String> = split_words(lists, name, &[])
                .into_iter()
                .map(|(word, _)| word)
                .collect();
            !name.is_empty()
                && words.windows(name.len()).enumerate().any(|(at, there)| {
                    main[at..at + name.len()].contains(&true)
                        && there
                            .iter()
                            .zip(&name)
                            .all(|(word, want)| same_word(lists, word, want))
                })
        });
        if holds {
            runs.push(Run {
                words: vec![folded_word(lists, other)],
                whole: false,
                made_from: false,
                other_name: true,
                kinds_left_out: Vec::new(),
            });
        }
    }
    (words, runs)
}

/// One place a Step names a line: the words `start..start + len`, whether
/// that was the line's whole name, and whether the line is something
/// [`reading::made_from`] what those words name.
#[derive(Clone, Copy)]
struct Mention {
    line: usize,
    start: usize,
    len: usize,
    whole: bool,
    made_from: bool,
    other_name: bool,
}

impl Mention {
    /// Whether this mention's words all lie inside `other`'s.
    fn within(&self, other: &Mention) -> bool {
        other.start <= self.start && self.start + self.len <= other.start + other.len
    }

    fn overlaps(&self, other: &Mention) -> bool {
        self.start < other.start + other.len && other.start < self.start + self.len
    }

    /// Whether `other` is the better reading of the words both cover: longer,
    /// or as long and the other line's whole name where this is only part of
    /// its own.
    fn beaten_by(&self, other: &Mention) -> bool {
        other.len > self.len || (other.len == self.len && other.whole && !self.whole)
    }
}

/// Whether a run of a name's words may stand for the whole name: it holds a
/// word a cook would call the food by, and neither begins nor ends on a word
/// that only joins two others. *Shredded coconut* may stand for *unsweetened
/// shredded coconut*; *de coco* may not stand for anything. Nor may a word a
/// recipe uses for what it makes itself, so *the sauce* is no soy sauce
/// (#439).
fn names_the_food(lists: &Words, run: &[String]) -> bool {
    let only_made_here = run
        .iter()
        .filter(|word| !reading::names_nothing_alone(lists, word))
        .all(|word| reading::made_here(lists, word));
    if only_made_here {
        return false;
    }
    match run {
        [] => false,
        [only] => {
            (only.chars().count() >= 3
                || kept_its_accent(lists, only)
                || lists.short_foods.has(only))
                && !reading::names_nothing_alone(lists, only)
        }
        [first, .., last] => {
            !reading::only_joins(lists, first)
                && !reading::only_joins(lists, last)
                && run
                    .iter()
                    .any(|word| !reading::names_nothing_alone(lists, word))
        }
    }
}

/// Every place in `step` this line is named, whole or by a part of its name.
fn mentions_of(lists: &Words, line: &Line, step: &[Word], language: Option<&str>) -> Vec<Mention> {
    let mut found = Vec::new();
    for run in &line.runs {
        // A run of nothing but words an accent sets apart from a joining
        // word is named only with the accent: *the* alone is no tea (#230).
        let by_accent = run.words.iter().all(|word| kept_its_accent(lists, word));
        let same = |word: &str, want: &str| {
            if by_accent {
                same_to_the_accent(word, want)
            } else {
                same_word(lists, word, want)
            }
        };
        for start in 0..step.len().saturating_sub(run.words.len() - 1) {
            let there = &step[start..start + run.words.len()];
            // A Step's word that names nothing alone is no part of a name
            // either, though its plural is: *the green portion* is no
            // *mustard greens* (#697).
            let names_nothing = !run.whole
                && there
                    .iter()
                    .all(|word| reading::names_nothing_alone(lists, &word.text));
            // *A sweet wine* is no red wine: the word beside the run names
            // another kind of the food than the one the line left out
            // (#704), before it in English and Dutch and after it in French
            // and Spanish, *un vin doux*. *The chopped onions* still name the
            // sliced ones, since a cut is no kind.
            let another = |beside: Option<&Word>| {
                beside.is_some_and(|beside| {
                    beside.clause == there[0].clause
                        && lists.step_kinds.has(&beside.text)
                        && !run
                            .kinds_left_out
                            .iter()
                            .any(|kind| same_word(lists, kind, &beside.text))
                })
            };
            let before = || another(start.checked_sub(1).map(|at| &step[at]));
            let after = || another(step.get(start + run.words.len()));
            let another_kind = !run.kinds_left_out.is_empty()
                && match language {
                    Some("fr" | "es") => after(),
                    Some("en" | "nl") => before(),
                    _ => before() || after(),
                };
            if !names_nothing
                && !another_kind
                && there
                    .iter()
                    .zip(&run.words)
                    .all(|(word, want)| same(&word.text, want))
            {
                found.push(Mention {
                    line: line.index,
                    start,
                    len: run.words.len(),
                    whole: run.whole,
                    made_from: run.made_from,
                    other_name: run.other_name,
                });
            }
        }
    }
    if lists.is_dutch() {
        found.extend(dutch_mentions(lists, line, step));
    }
    found
}

/// Every place in `step` a Dutch compound of this line is named by one part of
/// it: *de bloem* for *tarwebloem*, *de kip* for *kipfilet*. A part of a name,
/// so a line named whole wins over it.
fn dutch_mentions(lists: &Words, line: &Line, step: &[Word]) -> Vec<Mention> {
    let compounds = line.runs.iter().filter(|run| run.words.len() == 1);
    let mut found = Vec::new();
    for run in compounds {
        let whole = &run.words[0];
        for (start, word) in step.iter().enumerate() {
            let names = dutch::is_main_word_of(lists, &word.text, whole)
                || dutch::is_cut_of(lists, &word.text, whole);
            if names && names_the_food(lists, std::slice::from_ref(&word.text)) {
                found.push(Mention {
                    line: line.index,
                    start,
                    len: 1,
                    whole: false,
                    made_from: run.made_from,
                    other_name: false,
                });
            }
        }
    }
    found
}

/// **Which Ingredient Lines each Step uses**: one slot per row of `steps`,
/// `None` on a row that is not a Step, and otherwise the lines' indices in
/// order. `ingredients` is the list as written, whose section rows say which
/// section each line sits in, and `readings` is one slot per row of it, a
/// Reading or null. `language` is the Branch's, which says where a name's
/// main word sits, and `title` is the recipe's, whose words name the dish.
pub fn step_uses(
    steps: &[Value],
    ingredients: &[Value],
    readings: &[Value],
    language: Option<&str>,
    title: &str,
) -> Vec<Option<Vec<usize>>> {
    let lists = words::of(language.unwrap_or(UNKNOWN));
    let dish: Vec<String> = split_words(lists, title, &[])
        .into_iter()
        .map(|(word, _)| word)
        .collect();
    let mut section = 0;
    let sections: Vec<usize> = (0..readings.len())
        .map(|index| {
            if ingredients
                .get(index)
                .is_some_and(|row| row["kind"] == "section")
            {
                section += 1;
            }
            section
        })
        .collect();
    // Each section's heading, less the *for the* it is often written with.
    let mut numbered = 0;
    let headings: Vec<(usize, Vec<String>)> = ingredients
        .iter()
        .filter(|row| row["kind"] == "section")
        .map(|row| {
            numbered += 1;
            let words: Vec<String> =
                split_words(lists, row["text"].as_str().unwrap_or_default(), &[])
                    .into_iter()
                    .map(|(word, _)| word)
                    .skip_while(|word| {
                        opens_a_purpose(lists, word, language) || lists.step_articles.has(word)
                    })
                    .collect();
            // *Marinade ingredients* is named as *the marinade ingredients*.
            let words = match words.split_last() {
                Some((last, rest)) if lists.step_ingredients.has(last) => rest.to_vec(),
                _ => words,
            };
            (numbered, words)
        })
        .filter(|(_, words)| !words.is_empty())
        .collect();
    let recipe = Recipe {
        lists,
        dish,
        language,
        has_sections: section > 0,
        headings,
        lines: readings
            .iter()
            .enumerate()
            .filter_map(|(index, reading)| {
                // A Food stored as a choice, `yacai or preserved mustard
                // greens`, as an agent or a person may set it, is read as the
                // choice it is.
                let target = reading.get("target").and_then(Value::as_str);
                let options = match target {
                    Some(target) => reading::choice_names(target, language.unwrap_or(UNKNOWN)),
                    None => choice_options(ingredients.get(index)?, reading, language),
                };
                let (words, runs, one_food, choice) = match target {
                    Some(target) if options.is_empty() => {
                        let (words, runs) = runs_of(lists, target, language);
                        let one_food = name_words(lists, target).1.len() == 1;
                        (words, runs, one_food, false)
                    }
                    // Each option is read as a name of its own, and none
                    // is the line's whole name: *milk* in *milk or water*
                    // is no more made from water than *soft* is a tofu.
                    _ => {
                        let mut words = Vec::new();
                        let mut runs = Vec::new();
                        // *Chicken or vegetable stock* may be chicken stock:
                        // a one-word option before a last option ending in
                        // what is made from a food is, maybe, made from it
                        // too, so *the chicken* is the chicken where a line
                        // holds one (#697).
                        let made_like_the_last = options.last().is_some_and(|last| {
                            let last = split_words(lists, last, &[]);
                            last.len() > 1
                                && last[1..]
                                    .iter()
                                    .any(|(word, _)| reading::made_from(lists, word))
                        });
                        for (at, option) in options.iter().enumerate() {
                            let (option_words, option_runs) = runs_of(lists, option, language);
                            let maybe_made = made_like_the_last
                                && at + 1 < options.len()
                                && option_words.len() == 1;
                            words.extend(option_words);
                            runs.extend(
                                option_runs
                                    .into_iter()
                                    .map(|run| Run {
                                        whole: false,
                                        made_from: run.made_from || maybe_made,
                                        ..run
                                    })
                                    .filter(|run| {
                                        run.other_name || names_the_food(lists, &run.words)
                                    }),
                            );
                        }
                        (words, runs, false, true)
                    }
                };
                (!words.is_empty()).then(|| Line {
                    index,
                    food: words
                        .iter()
                        .map(|w| match lists.grammar {
                            Grammar::Dutch => dutch::stem(lists, w),
                            Grammar::Together => singular(w).to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                    runs,
                    section: sections[index],
                    one_food,
                    choice,
                })
            })
            .collect(),
    };

    let mut earlier = Earlier::default();
    let mut own_section = None;
    steps
        .iter()
        .map(|row| {
            if row["kind"] != "step" {
                return None;
            }
            let text = row["text"].as_str()?;
            let uses = recipe.uses_of(&words_of(lists, text), &earlier, &mut own_section);
            earlier.used.extend(&uses.lines);
            earlier.cooked.extend(&uses.cooked);
            Some(uses.lines)
        })
        .collect()
}

/// The lines the Steps before the one being read used, and the ones among
/// them a Step used up by cooking them ([`StepUses`]).
#[derive(Default)]
struct Earlier {
    used: Vec<usize>,
    cooked: Vec<usize>,
}

/// The options a line offering a choice is found by, `ground pork` and
/// `beef` (#697). Its Reading names no Food, on purpose (#148), and a Step
/// names it by either option. Nothing for a line pointing at a recipe, or one
/// offering no choice.
fn choice_options(row: &Value, reading: &Value, language: Option<&str>) -> Vec<String> {
    if row["kind"] != "ingredient" || reading["lineage_id"].is_string() {
        return Vec::new();
    }
    let text = row["text"].as_str().unwrap_or_default();
    reading::choice_names(text, language.unwrap_or(UNKNOWN))
}

/// What every Step of one recipe is read against: the words of its title,
/// its Language, whether its list has sections and what each is headed, and
/// its lines.
struct Recipe<'a> {
    lists: &'static Words,
    dish: Vec<String>,
    language: Option<&'a str>,
    has_sections: bool,
    headings: Vec<(usize, Vec<String>)>,
    lines: Vec<Line>,
}

/// Whether the title's words hold `words` as a run, one trailing `s`
/// forgiven. An accent [`folded_word`] kept counts, so a title's *de* is not
/// a Step's *dés* (#230).
fn names_the_dish(words: &[Word], dish: &[String]) -> bool {
    dish.windows(words.len()).any(|there| {
        there
            .iter()
            .zip(words)
            .all(|(dish, word)| same_to_the_accent(&word.text, dish))
    })
}

/// One place a Step names a food, and every line it may mean there.
struct Named {
    lines: Vec<usize>,
    /// The lines among them named by a part of the name the title holds,
    /// which name the dish unless the Step works in their section.
    by_the_dish: Vec<usize>,
}

impl Recipe<'_> {
    fn line(&self, index: usize) -> &Line {
        self.lines
            .iter()
            .find(|line| line.index == index)
            .expect("a mention names a line of the recipe")
    }

    /// The sections a Step names whole, as *all the spice blend ingredients*
    /// or *les ingrédients de la sauce*, and the words that name each (#439).
    fn sections_named(&self, step: &[Word]) -> Vec<(usize, Range<usize>)> {
        let lists = self.lists;
        // A heading is named whole, or by its main word where no other
        // heading holds that word there: *the sauce ingredients* of a list
        // headed *Teriyaki sauce*.
        let main_word = |heading: &[String]| match self.language {
            Some("fr" | "es") => heading.first().cloned(),
            _ => heading.last().cloned(),
        };
        let mut forms: Vec<(usize, Vec<String>)> = self.headings.clone();
        for (section, heading) in &self.headings {
            let Some(word) = main_word(heading).filter(|_| heading.len() > 1) else {
                continue;
            };
            let shared = self
                .headings
                .iter()
                .filter(|(_, other)| main_word(other).is_some_and(|w| same_word(lists, &w, &word)))
                .count()
                > 1;
            if !shared {
                forms.push((*section, vec![word]));
            }
        }
        let mut named = Vec::new();
        for (section, heading) in &forms {
            for start in 0..step.len().saturating_sub(heading.len() - 1) {
                let end = start + heading.len();
                let there = &step[start..end];
                if !there
                    .iter()
                    .zip(heading)
                    .all(|(word, want)| same_word(lists, &word.text, want))
                {
                    continue;
                }
                let after = step
                    .get(end)
                    .is_some_and(|word| lists.step_ingredients.has(&word.text));
                let before = step[..start]
                    .iter()
                    .rev()
                    .find(|word| {
                        !lists.step_articles.has(&word.text)
                            && !lists.step_of.has(&word.text)
                            && !opens_a_purpose(lists, &word.text, self.language)
                    })
                    .is_some_and(|word| lists.step_ingredients.has(&word.text));
                if after || before {
                    named.push((*section, start..end));
                }
            }
        }
        named
    }

    /// Whether this mention names what something is for rather than what goes
    /// in: *for the rice*, *para el arroz*, *pour le riz* (#439).
    fn is_for(&self, step: &[Word], mention: &Mention) -> bool {
        let lists = self.lists;
        let before = step[..mention.start]
            .iter()
            .rev()
            .find(|word| !lists.step_articles.has(&word.text));
        before.is_some_and(|word| {
            word.clause == step[mention.start].clause
                && opens_a_purpose(lists, &word.text, self.language)
        })
    }

    /// Whether this mention only describes a thing the recipe makes, named
    /// beside it: *the tomato-spice mixture*, *le mélange de tomates* (#439).
    /// English writes the thing after, French and Spanish before, and a
    /// Branch with no Language is read both ways.
    fn describes_what_is_made(&self, step: &[Word], mention: &Mention) -> bool {
        let lists = self.lists;
        let clause = step[mention.start].clause;
        let made_here =
            |word: &Word| word.clause == clause && reading::made_here(lists, &word.text);
        // A stock is made from what is named beside it, and holds none of
        // the line: *the chicken stock* is no chicken thighs.
        // Nor is *the beef fat* any beef (#704).
        let holds_none_of_it = |word: &Word| {
            word.clause == clause
                && (lists.step_stocks.has(&word.text) || lists.step_left_in_the_pan.has(&word.text))
        };
        let after = || {
            let next = &step[mention.start + mention.len..];
            next.first().is_some_and(holds_none_of_it)
                || next
                    .iter()
                    .take(2)
                    .take_while(|word| !reading::only_joins(lists, &word.text))
                    .any(made_here)
        };
        let before = || {
            step[..mention.start]
                .iter()
                .rev()
                .take(2)
                .skip_while(|word| lists.step_of.has(&word.text))
                .take(1)
                .any(|word| made_here(word) || holds_none_of_it(word))
        };
        match self.language {
            Some("en") => after(),
            Some("fr" | "es") => before(),
            _ => after() || before(),
        }
    }

    /// Whether the Branch is English, or of no Language Kamosu knows, which
    /// English rules may read.
    fn english(&self) -> bool {
        matches!(self.language, None | Some("en" | UNKNOWN))
    }

    /// Whether a mention by one of [`Words::step_other_names`] is the verb
    /// spelt the same: *mince the garlic*, *mince garlic, then…*, *finely
    /// mince garlic*. The verb opens what the cook does, after *then* or a
    /// word ending in *-ly* at most, or is followed by an article or a food. The noun
    /// is what something is done to: *cook mince until browned*, *fry the
    /// mince 5 minutes*.
    fn is_a_verb(&self, step: &[Word], mention: &Mention, found: &[Mention]) -> bool {
        let lists = self.lists;
        let at = mention.start;
        let clause = step[at].clause;
        // An English adverb, *finely*, may stand before the verb.
        let adverb = |word: &&Word| self.english() && word.text.ends_with("ly");
        let before = step[..at]
            .iter()
            .rev()
            .take_while(|word| word.clause == clause)
            .find(|word| !adverb(word));
        // After *and* it is the verb only before what it is done to: *peel
        // and mince the garlic*, but *brown the onion and mince*. With
        // nothing before it in its clause, it opens the Step, or it is one
        // food in a list: *add the onion, garlic, mince and tomato paste*.
        let opens = match before {
            Some(word) => lists.step_then.has(&word.text),
            None => step[..at].iter().all(|word| adverb(&word)),
        };
        // What the verb is done to sits in its clause: *put the meat, speck
        // and onions back* names three foods (#704).
        let after = at + mention.len;
        opens
            || step.get(after).is_some_and(|next| {
                next.clause == clause
                    && (lists.step_articles.has(&next.text)
                        || found
                            .iter()
                            .any(|other| other.line != mention.line && other.start == after))
            })
    }

    /// Whether a mention by one word of a line offering a choice is only a
    /// word describing another line's food: *smoked* of *smoked or sweet
    /// paprika* in *add the smoked sausage* (#697). Another line is then
    /// named straight after it. Anything else after it, *brown the beef 5
    /// minutes*, *le beurre fondu*, leaves the mention standing.
    fn describes_another(&self, step: &[Word], mention: &Mention, found: &[Mention]) -> bool {
        if mention.len != 1 || !self.line(mention.line).choice {
            return false;
        }
        let after = mention.start + 1;
        step.get(after).is_some_and(|next| {
            next.clause == step[mention.start].clause
                && found
                    .iter()
                    .any(|other| other.line != mention.line && other.start == after)
        })
    }

    /// Whether a mention is of more than the line holds: *serve with more
    /// chili oil* after a Step that already used the chili oil (#697). A
    /// line no earlier Step used is still the line, *more* or not.
    /// The mention may be a part of a longer one of the same line, *oil* in
    /// *more chili oil*, which `mentions` holds.
    fn more_than_written(
        &self,
        step: &[Word],
        mention: &Mention,
        mentions: &[Mention],
        used_before: &[usize],
    ) -> bool {
        let lists = self.lists;
        // *Davantage d'huile*: the word for more stands before *de*.
        let more_before = |start: usize| {
            let mut before = step[..start]
                .iter()
                .rev()
                .skip_while(|word| lists.step_of.has(&word.text));
            let more = before
                .next()
                .is_some_and(|word| lists.step_more.has(&word.text));
            // *2 more eggs* is the rest of the line's amount.
            let counted = before
                .next()
                .is_some_and(|word| word.text.starts_with(|c: char| c.is_ascii_digit()));
            more && !counted
        };
        used_before.contains(&mention.line)
            && mentions.iter().any(|longer| {
                longer.line == mention.line && mention.within(longer) && more_before(longer.start)
            })
    }

    /// Whether a mention names a part of its line's amount: a
    /// [`Words::step_portions`] word or an amount before it in its clause,
    /// *a little salt*, *the rest of the butter*, *2 tbsp of the oil*. A line
    /// a Step named only in part is not used up by it (#704).
    fn named_in_part(&self, step: &[Word], mention: &Mention) -> bool {
        let lists = self.lists;
        let clause = step[mention.start].clause;
        step[..mention.start]
            .iter()
            .rev()
            .take_while(|word| word.clause == clause)
            .any(|word| {
                lists.step_portions.has(&word.text)
                    || word.text.starts_with(|c: char| c.is_ascii_digit())
            })
    }

    /// Whether a mention names what an earlier Step already made of its line,
    /// which went in then (#704): *the cooked mince*, *put the meat back*, or
    /// *the noodles* after a Step that boiled them. A part of the line's
    /// amount is going in again, *the green portion of scallions*, and so is a
    /// line an earlier Step only cut or mixed: *brown the meat* after *cut
    /// the meat*.
    fn already_made(&self, step: &[Word], mention: &Mention, earlier: &Earlier) -> bool {
        let lists = self.lists;
        if !earlier.used.contains(&mention.line) {
            return false;
        }
        if self.named_in_part(step, mention) {
            return false;
        }
        let here = &step[mention.start];
        let before = step[..mention.start]
            .last()
            .filter(|word| word.clause == here.clause);
        // *Back* reaches across the commas of the one thing the cook does,
        // from the word that opens it to the next: *then put the meat, speck
        // and onions back*, but not the salt added before *then*.
        let opens = |word: &Word| {
            lists.step_then.has(&word.text) || lists.step_next_actions.has(&word.text)
        };
        let from = step[..mention.start]
            .iter()
            .rposition(|word| word.sentence != here.sentence || opens(word))
            .map_or(0, |at| at + 1);
        let put_back = step[from..]
            .iter()
            .take_while(|word| word.sentence == here.sentence)
            .enumerate()
            .take_while(|(at, word)| from + at <= mention.start || !opens(word))
            .any(|(_, word)| lists.step_back.has(&word.text));
        // *The noodles*, *cooked mince*: a line an earlier Step cooked,
        // named after an article or a word saying what was done to it. *The
        // chopped onion* after *chop the onion* is still going in.
        let the_cooked_one = earlier.cooked.contains(&mention.line)
            && before.is_some_and(|word| {
                lists.step_articles.has(&word.text) || lists.describing.has(&word.text)
            });
        put_back || the_cooked_one
    }

    /// The lines a shared tail names beside the one it names whole: *light
    /// and dark soy sauce* is the dark soy sauce, and the light soy sauce
    /// too (#697). English alone: French and Spanish put the main word first
    /// and the shared word after (*sauce soja claire et foncée*), and Dutch
    /// writes such a food as one word. Only a line whose whole
    /// name the first word and the tail make, the first written in place of
    /// the tail's first word, and only where no other line is named by that
    /// first word. Each is a mention of that one word.
    fn shared_tails(&self, step: &[Word], mentions: &[Mention]) -> Vec<Mention> {
        let lists = self.lists;
        if !self.english() {
            return Vec::new();
        }
        let mut found = Vec::new();
        for tail in mentions
            .iter()
            .filter(|mention| mention.whole && mention.len > 1)
        {
            let Some(first) = tail.start.checked_sub(2) else {
                continue;
            };
            if !lists.step_and.has(&step[tail.start - 1].text)
                || mentions
                    .iter()
                    .any(|other| other.start <= first && first < other.start + other.len)
            {
                continue;
            }
            let rest = &step[tail.start + 1..tail.start + tail.len];
            for line in &self.lines {
                let names_it = line.runs.iter().any(|run| {
                    run.whole
                        && run.words.len() == tail.len
                        && same_word(lists, &step[first].text, &run.words[0])
                        && rest
                            .iter()
                            .zip(&run.words[1..])
                            .all(|(word, want)| same_word(lists, &word.text, want))
                });
                if names_it && line.index != tail.line {
                    found.push(Mention {
                        line: line.index,
                        start: first,
                        len: 1,
                        whole: true,
                        made_from: false,
                        other_name: false,
                    });
                }
            }
        }
        found
    }

    /// The lines one Step uses, given the lines earlier Steps already used
    /// and the section the Step before worked in, which this updates to the
    /// section this Step works in.
    fn uses_of(
        &self,
        step: &[Word],
        earlier: &Earlier,
        own_section: &mut Option<usize>,
    ) -> StepUses {
        let lists = self.lists;
        let used_before = &earlier.used[..];
        let sections_named = self.sections_named(step);
        let found: Vec<Mention> = self
            .lines
            .iter()
            .flat_map(|line| mentions_of(lists, line, step, self.language))
            .collect();
        let mut mentions: Vec<Mention> = found
            .iter()
            .copied()
            .filter(|mention| !mention.other_name || !self.is_a_verb(step, mention, &found))
            .filter(|mention| !self.describes_another(step, mention, &found))
            .filter(|mention| !step[mention.start].not_going_in)
            .filter(|mention| !self.is_for(step, mention))
            .filter(|mention| !self.describes_what_is_made(step, mention))
            .filter(|mention| {
                !sections_named.iter().any(|(_, words)| {
                    words.start < mention.start + mention.len && mention.start < words.end
                })
            })
            .collect();
        mentions.extend(self.shared_tails(step, &mentions));
        // A part naming the dish still claims its words from a shorter
        // mention it overlaps, and only then is weighed itself. A whole name
        // gives way only to a longer whole name of one food: *the tomato
        // paste* is no mention of the line *Tomatoes*, though *salt and
        // pepper* is still one of *Salt* (#439).
        let kept: Vec<(Mention, bool)> = mentions
            .iter()
            .filter(|mention| {
                !mentions.iter().any(|other| {
                    other.line != mention.line
                        && mention.overlaps(other)
                        && mention.beaten_by(other)
                        && (!mention.whole
                            || (other.whole
                                && other.len > mention.len
                                && self.line(other.line).one_food))
                })
            })
            .map(|mention| {
                let by_the_dish = !mention.whole
                    && names_the_dish(&step[mention.start..][..mention.len], &self.dish);
                (*mention, by_the_dish)
            })
            // Curry powder is no curry: a part of its name the title holds
            // names the dish (#357). The food itself in the title still
            // links, as chicken thighs do in a chicken dish (#439).
            // A choice line's option is only maybe made from the other, so
            // *butter* of *butter or olive oil* still links in a dish named
            // after garlic butter (#697).
            .filter(|(mention, by_the_dish)| {
                !(*by_the_dish && mention.made_from && !self.line(mention.line).choice)
            })
            .collect();
        // *More chili oil* claims its words from the neutral oil's *oil*
        // first, and only then goes (#697).
        let kept: Vec<(Mention, bool)> = kept
            .iter()
            .copied()
            .filter(|(mention, _)| !self.more_than_written(step, mention, &mentions, used_before))
            .collect();
        // *Sesame* inside *sesame paste* is the same place, and votes once
        // for its section (#697).
        let kept: Vec<(Mention, bool)> = kept
            .iter()
            .copied()
            .filter(|(mention, _)| {
                !kept.iter().any(|(longer, _)| {
                    longer.line == mention.line
                        && longer.len > mention.len
                        && mention.within(longer)
                })
            })
            .collect();
        // What an earlier Step already made, read on the longest mention, so
        // *sum* of *the choy sum* goes with it (#704).
        let made_before: Vec<Mention> = kept
            .iter()
            .map(|(mention, _)| *mention)
            .filter(|mention| self.already_made(step, mention, earlier))
            .collect();

        // Each place a food is named, and the lines it may mean there: the
        // food itself before something made from it, so *the garlic* is the
        // cloves and not the powder (#439).
        let mut places: Vec<((usize, usize), Named)> = Vec::new();
        for (mention, by_the_dish) in &kept {
            let at = (mention.start, mention.len);
            let place = match places.iter().position(|(there, _)| *there == at) {
                Some(found) => &mut places[found].1,
                None => {
                    places.push((
                        at,
                        Named {
                            lines: Vec::new(),
                            by_the_dish: Vec::new(),
                        },
                    ));
                    &mut places.last_mut().expect("just pushed").1
                }
            };
            if !place.lines.contains(&mention.line) {
                place.lines.push(mention.line);
            }
            if *by_the_dish {
                place.by_the_dish.push(mention.line);
            }
        }
        for (at, place) in &mut places {
            let itself: Vec<usize> = kept
                .iter()
                .filter(|(mention, _)| (mention.start, mention.len) == *at && !mention.made_from)
                .map(|(mention, _)| mention.line)
                .collect();
            if !itself.is_empty() {
                place.lines.retain(|line| itself.contains(line));
            }
            // Only then does the food an earlier Step made go, so *put the
            // onions back* names neither the sliced onions nor the carrot
            // and onion mix (#704).
            place.lines.retain(|&line| {
                !made_before
                    .iter()
                    .any(|made| (made.start, made.len) == *at && made.line == line)
            });
        }
        places.retain(|(_, place)| !place.lines.is_empty());

        // The section this Step works in: the one most of the places it names
        // a food point to, or the last Step's where no one section leads
        // (#439). A place points to each section holding a line it may mean
        // that no earlier Step used, or to every such section when all are
        // used, so *the garlic* of the roasted garlic is the second garlic.
        if self.has_sections {
            let mut votes: Vec<(usize, usize)> = Vec::new();
            for (_, place) in &places {
                let unused: Vec<usize> = place
                    .lines
                    .iter()
                    .copied()
                    .filter(|line| !used_before.contains(line))
                    .collect();
                let lines = if unused.is_empty() {
                    &place.lines
                } else {
                    &unused
                };
                let mut sections: Vec<usize> =
                    lines.iter().map(|&line| self.line(line).section).collect();
                sections.sort_unstable();
                sections.dedup();
                for section in sections {
                    match votes.iter_mut().find(|(there, _)| *there == section) {
                        Some((_, count)) => *count += 1,
                        None => votes.push((section, 1)),
                    }
                }
            }
            let most = votes.iter().map(|&(_, count)| count).max();
            let leading: Vec<usize> = votes
                .iter()
                .filter(|&&(_, count)| Some(count) == most)
                .map(|&(section, _)| section)
                .collect();
            match leading[..] {
                [only] => *own_section = Some(only),
                // With no section settled yet, the list's order does: the
                // first Step's *oil* is the neutral oil at the top, not the
                // sauce's sesame oil (#697).
                [..] if own_section.is_none() => *own_section = leading.iter().min().copied(),
                _ => {}
            }
        }

        // A section named whole: every line it holds.
        let mut uses: Vec<usize> = self
            .lines
            .iter()
            .filter(|line| {
                sections_named
                    .iter()
                    .any(|(section, _)| *section == line.section)
            })
            .map(|line| line.index)
            .collect();
        if let Some((section, _)) = sections_named.first() {
            *own_section = Some(*section);
        }
        let own = own_section.filter(|_| self.has_sections);
        for (_, place) in &mut places {
            if let Some(own) = own {
                // A word of the title names the dish outside the section
                // its line sits in: the serving Step's *bed of rice* is the
                // cooked rice, not the cup of basmati.
                let by_the_dish = std::mem::take(&mut place.by_the_dish);
                place
                    .lines
                    .retain(|&line| self.line(line).section == own || !by_the_dish.contains(&line));
                // Two lines named alike: the one in this Step's own section.
                if place
                    .lines
                    .iter()
                    .any(|&line| self.line(line).section == own)
                {
                    place.lines.retain(|&line| self.line(line).section == own);
                }
            }
            // One food on several lines: the first no earlier Step used.
            for &index in &place.lines {
                let food = &self.line(index).food;
                let same: Vec<usize> = place
                    .lines
                    .iter()
                    .copied()
                    .filter(|&other| self.line(other).food == *food)
                    .collect();
                let chosen = same
                    .iter()
                    .copied()
                    .find(|line| !used_before.contains(line))
                    .unwrap_or(same[0]);
                if chosen == index {
                    uses.push(index);
                }
            }
        }
        uses.sort_unstable();
        uses.dedup();
        let cooked = uses
            .iter()
            .copied()
            .filter(|&line| {
                kept.iter().any(|(mention, _)| {
                    mention.line == line
                        && !self.named_in_part(step, mention)
                        && self.cooks(step, step[mention.start].sentence)
                })
            })
            .collect();
        StepUses {
            lines: uses,
            cooked,
        }
    }

    /// Whether a sentence of a Step cooks what it names: one of
    /// [`Words::step_cooks`] stands where a verb does, opening a clause or
    /// after *then* or *and*. *Bring the water to the boil*, *off the heat*
    /// and *add the brown sugar* cook nothing (#704).
    fn cooks(&self, step: &[Word], sentence: usize) -> bool {
        let lists = self.lists;
        step.iter().enumerate().any(|(at, word)| {
            word.sentence == sentence
                && !word.not_going_in
                && lists.step_cooks.has(&word.text)
                && at
                    .checked_sub(1)
                    .map(|before| &step[before])
                    .is_none_or(|before| {
                        before.clause != word.clause
                            || lists.step_then.has(&before.text)
                            || lists.step_and.has(&before.text)
                    })
        })
    }
}

/// The lines one Step uses, and those among them it used up by cooking them,
/// named in a sentence that cooks with no part of their amount said: the
/// cooked thing from then on (#704).
struct StepUses {
    lines: Vec<usize>,
    cooked: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A recipe's Steps and Readings as `step_uses` takes them: every step a
    /// Step and every target a Reading (a `#` prefix makes either a Section,
    /// and an empty target leaves the line unread).
    fn uses(steps: &[&str], targets: &[&str]) -> Vec<Option<Vec<usize>>> {
        uses_in(None, steps, targets)
    }

    /// The title every test but the dish's own is given: it holds none of
    /// their words, so it takes no mention away (#357).
    const NO_DISH: &str = "Untitled";

    /// The same, on a Branch written in `language`.
    fn uses_in(
        language: Option<&str>,
        steps: &[&str],
        targets: &[&str],
    ) -> Vec<Option<Vec<usize>>> {
        uses_titled(NO_DISH, language, steps, targets)
    }

    /// The same, on a recipe called `title`.
    fn uses_titled(
        title: &str,
        language: Option<&str>,
        steps: &[&str],
        targets: &[&str],
    ) -> Vec<Option<Vec<usize>>> {
        let steps: Vec<Value> = steps
            .iter()
            .map(|text| match text.strip_prefix('#') {
                Some(section) => json!({ "kind": "section", "text": section }),
                None => json!({ "kind": "step", "text": text }),
            })
            .collect();
        let ingredients: Vec<Value> = targets
            .iter()
            .map(|target| match target.strip_prefix('#') {
                Some(section) => json!({ "kind": "section", "text": section }),
                None => json!({ "kind": "ingredient", "text": target }),
            })
            .collect();
        let readings: Vec<Value> = targets
            .iter()
            .map(|target| {
                if target.is_empty() || target.starts_with('#') {
                    Value::Null
                } else {
                    json!({ "target": target })
                }
            })
            .collect();
        step_uses(&steps, &ingredients, &readings, language, title)
    }

    fn step(indices: &[usize]) -> Option<Vec<usize>> {
        Some(indices.to_vec())
    }

    /// Chocolate coconut snack cake, as written on prod on 2026-09-27: the
    /// first case #184 reported.
    const CAKE_TARGETS: &[&str] = &[
        "eggs",
        "maple syrup",
        "Greek yogurt",
        "vanilla extract",
        "almond flour",
        "unsweetened shredded coconut",
        "cocoa powder",
        "brown sugar",
        "baking soda",
        "salt",
        "espresso powder",
    ];
    const CAKE_STEPS: &[&str] = &[
        "Preheat your oven to 350°F. Line a 7×7 inch brownie pan with parchment paper.",
        "In a large bowl, whisk together the eggs, syrup, yogurt and vanilla until smooth.",
        "Add in the almond flour, shredded coconut, cocoa powder, brown sugar, baking soda, salt and espresso powder, and mix together until incorporated.",
        "Transfer the mixture to the brownie pan and spread out evenly.",
        "Bake for 25-30 minutes or until a toothpick comes out clean.",
        "Allow to set in the pan for 15 minutes.",
        "Carefully transfer to a wire rack and allow to cool completely before slicing.",
        "Garnish with shredded coconut.",
    ];

    #[test]
    fn a_step_links_an_ingredient_by_the_short_name_the_recipe_uses_for_it() {
        assert_eq!(
            uses(CAKE_STEPS, CAKE_TARGETS),
            [
                step(&[]),
                step(&[0, 1, 2, 3]),
                step(&[4, 5, 6, 7, 8, 9, 10]),
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[5]),
            ]
        );
    }

    #[test]
    fn an_accented_word_spelt_like_a_joining_word_names_its_food() {
        // Tea and diced pieces in French: `thé` and `dés` are the food, where
        // `the` and `des` are articles (#230).
        const TEA: &str = "Faites infuser le thé 3 minutes, puis ajoutez le miel.";
        const DICE: &str = "Ajoutez les dés et le miel.";
        for language in [None, Some("fr")] {
            assert_eq!(
                uses_in(language, &[TEA], &["thé vert", "miel"]),
                [step(&[0, 1])]
            );
            assert_eq!(
                uses_in(language, &[DICE], &["dés de jambon", "miel"]),
                [step(&[0, 1])]
            );
            // The accent typed as its own character is the same word.
            assert_eq!(
                uses_in(
                    language,
                    &["Faites infuser le the\u{301} 3 minutes, puis ajoutez le miel."],
                    &["thé vert", "miel"]
                ),
                [step(&[0, 1])]
            );
            assert_eq!(
                uses_in(
                    language,
                    &["Ajoutez les de\u{301}s et le miel."],
                    &["dés de jambon", "miel"]
                ),
                [step(&[0, 1])]
            );
            assert_eq!(
                uses_in(language, &["Incorporez le dé."], &["dé de beurre"]),
                [step(&[0])]
            );
            assert_eq!(
                uses_in(language, &["Servez les thés."], &["thé"]),
                [step(&[0])]
            );
            assert_eq!(
                uses_in(language, &["SERVEZ LES THÉS."], &["thé vert"]),
                [step(&[0])]
            );
            // The other noun of such a name still names it.
            assert_eq!(
                uses_in(
                    language,
                    &["Faites revenir le jambon.", "Ajoutez le beurre."],
                    &["dés de jambon", "dé de beurre"]
                ),
                [step(&[0]), step(&[1])]
            );
        }
    }

    #[test]
    fn a_joining_word_with_no_accent_names_no_food() {
        for language in [None, Some("en"), Some("fr")] {
            assert_eq!(
                uses_in(
                    language,
                    &["Add the honey to the pan."],
                    &["thé vert", "honey"]
                ),
                [step(&[1])]
            );
            assert_eq!(
                uses_in(language, &["Add the honey to the pan."], &["thé", "honey"]),
                [step(&[1])]
            );
            assert_eq!(
                uses_in(
                    language,
                    &["Ajoutez des oignons."],
                    &["dés de jambon", "dé de beurre", "oignons"]
                ),
                [step(&[2])]
            );
        }
        // Beside another word of the name it is the name still, as a Step
        // typed without its accents has always been read.
        assert_eq!(
            uses_in(Some("fr"), &["Faites infuser le the vert."], &["thé vert"]),
            [step(&[0])]
        );
    }

    /// The cake's French Translation.
    const FRENCH_CAKE_TARGETS: &[&str] = &[
        "œufs",
        "sirop d'érable",
        "yaourt grec",
        "extrait de vanille",
        "farine d'amande",
        "noix de coco râpée non sucrée",
        "cacao en poudre",
        "cassonade",
        "bicarbonate de soude",
        "sel",
        "espresso en poudre",
    ];
    const FRENCH_CAKE_STEPS: &[&str] = &[
        "Préchauffez le four à 350°F. Chemisez un moule carré de 7×7 pouces de papier sulfurisé.",
        "Dans un grand saladier, fouettez les œufs, le sirop, le yaourt et la vanille jusqu'à obtenir un mélange lisse.",
        "Ajoutez la farine d'amande, la noix de coco râpée, le cacao, la cassonade, le bicarbonate, le sel et l'espresso en poudre, puis mélangez jusqu'à ce que tout soit incorporé.",
        "Versez la pâte dans le moule et étalez-la uniformément.",
        "Enfournez pour 25-30 minutes, jusqu'à ce qu'un cure-dent en ressorte propre.",
        "Parsemez de noix de coco râpée.",
    ];

    #[test]
    fn a_french_step_links_le_cacao_and_le_bicarbonate() {
        assert_eq!(
            uses(FRENCH_CAKE_STEPS, FRENCH_CAKE_TARGETS),
            [
                step(&[]),
                step(&[0, 1, 2, 3]),
                step(&[4, 5, 6, 7, 8, 9, 10]),
                step(&[]),
                step(&[]),
                step(&[5]),
            ]
        );
    }

    /// No-fold crispy dumplings, as written on prod: soy sauce and spring
    /// onion each on two lines, ginger and garlic read as *fresh ginger* and
    /// *garlic clove*, and water named only in passing.
    const DUMPLING_TARGETS: &[&str] = &[
        "",
        "pork mince",
        "soy sauce",
        "sesame oil",
        "salt",
        "spring onions",
        "fresh ginger",
        "garlic clove",
        "vegetable oil",
        "gyoza wrappers",
        "spring onion",
        "",
        "cornflour",
        "plain flour",
        "water",
        "",
        "soy sauce",
        "Chinese black vinegar",
        "crispy chilli oil",
    ];
    const DUMPLING_STEPS: &[&str] = &[
        "#Make the filling",
        "In a medium bowl, mix the pork mince, soy sauce, sesame oil, salt, spring onions, ginger and garlic until the mixture is well combined and sticky. This helps ensure the filling stays juicy and tender.",
        "#Scoop and sear",
        "Heat the vegetable oil in a large (about 25 cm or 10-inch) non-stick frying pan with a lid over medium-high heat.",
        "Using a small ice cream scoop or two spoons, place scoops of the filling (about 1 tablespoon each) directly into the pan, arranging them in a circular pattern with a little space between each one.",
        "Cook for 2-3 minutes without moving them, until the bottoms are lightly browned.",
        "Quickly lay the gyoza wrappers over the top of each scoop of filling, pressing down gently so they adhere.",
        "#Make the crispy skirt",
        "In a jug, whisk together the cornflour, plain flour and water to make the slurry.",
        "Pour the slurry all over the dumplings in the pan. It should bubble up immediately.",
        "Cover with the lid, reduce the heat to low and steam for 5-7 minutes, or until the pork is cooked through and most of the water has evaporated.",
        "Remove the lid, increase the heat slightly and cook for another 2-3 minutes, until the skirt is golden and crispy. You can check by gently lifting an edge with a spatula.",
        "#The big reveal",
        "Turn off the heat and run a spatula around the edge of the pan to make sure nothing is sticking.",
        "Place a large serving plate over the top of the pan. With one hand firmly on the plate and the other on the pan handle, flip the pan over confidently and quickly.",
        "Lift the pan away to reveal the crispy dumpling base.",
        "Mix the soy sauce, vinegar and chilli oil for the dipping sauce.",
        "Drizzle over the dumplings and scatter with spring onion. Serve immediately.",
    ];

    #[test]
    fn a_food_on_two_lines_links_the_one_no_earlier_step_used() {
        assert_eq!(
            uses(DUMPLING_STEPS, DUMPLING_TARGETS),
            [
                None,
                step(&[1, 2, 3, 4, 5, 6, 7]),
                None,
                step(&[8]),
                step(&[]),
                step(&[]),
                step(&[9]),
                None,
                step(&[12, 13, 14]),
                step(&[]),
                step(&[]),
                step(&[]),
                None,
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[16, 17, 18]),
                step(&[10]),
            ]
        );
    }

    /// The dumplings' French Translation.
    const FRENCH_DUMPLING_TARGETS: &[&str] = &[
        "",
        "porc haché",
        "sauce soja",
        "huile de sésame",
        "sel",
        "oignons nouveaux",
        "gingembre",
        "ail",
        "huile végétale",
        "feuilles à gyoza",
        "oignon nouveau",
        "",
        "fécule de maïs",
        "farine",
        "eau",
        "",
        "sauce soja",
        "vinaigre noir chinois",
        "chili crisp",
    ];
    const FRENCH_DUMPLING_STEPS: &[&str] = &[
        "#Préparer la farce",
        "Dans un saladier moyen, mélangez le porc haché, la sauce soja, l'huile de sésame, le sel, les oignons nouveaux, le gingembre et l'ail jusqu'à ce que le mélange soit homogène et collant. La farce reste ainsi juteuse et tendre.",
        "#Déposer et saisir",
        "Faites chauffer l'huile végétale à feu moyen-vif dans une grande poêle antiadhésive (environ 25 cm ou 10 pouces) munie d'un couvercle.",
        "À l'aide d'une petite cuillère à glace ou de deux cuillères, déposez des boules de farce (environ 1 cuillère à soupe chacune) directement dans la poêle, en cercle, en laissant un peu d'espace entre elles.",
        "Faites cuire 2-3 minutes sans les bouger, jusqu'à ce que le dessous soit légèrement doré.",
        "Posez rapidement une feuille à gyoza sur chaque boule de farce, en appuyant doucement pour qu'elle adhère.",
        "#Faire la jupe croustillante",
        "Dans un pichet, fouettez la fécule de maïs, la farine et l'eau pour obtenir l'appareil.",
        "Versez l'appareil sur tous les raviolis dans la poêle. Il doit bouillonner immédiatement.",
        "Couvrez, baissez le feu et laissez cuire à la vapeur 5-7 minutes, jusqu'à ce que le porc soit cuit et que l'essentiel de l'eau se soit évaporé.",
        "Retirez le couvercle, montez légèrement le feu et poursuivez la cuisson 2-3 minutes, jusqu'à ce que la jupe soit dorée et croustillante. Vérifiez en soulevant délicatement un bord avec une spatule.",
        "#Le démoulage",
        "Éteignez le feu et passez une spatule tout autour de la poêle pour que rien n'accroche.",
        "Posez une grande assiette de service sur la poêle. Une main bien à plat sur l'assiette, l'autre sur le manche, retournez la poêle d'un geste sûr et rapide.",
        "Soulevez la poêle pour découvrir la base croustillante.",
        "Mélangez la sauce soja, le vinaigre et le chili crisp pour la sauce.",
        "Arrosez-en les raviolis et parsemez d'oignon nouveau. Servez aussitôt.",
    ];

    #[test]
    fn the_french_dumplings_link_the_same_lines() {
        assert_eq!(
            uses(FRENCH_DUMPLING_STEPS, FRENCH_DUMPLING_TARGETS),
            [
                None,
                step(&[1, 2, 3, 4, 5, 6, 7]),
                None,
                step(&[8]),
                step(&[]),
                step(&[]),
                step(&[9]),
                None,
                step(&[12, 13, 14]),
                step(&[]),
                step(&[]),
                step(&[]),
                None,
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[16, 17, 18]),
                step(&[10]),
            ]
        );
    }

    #[test]
    fn a_short_word_two_different_foods_share_links_both() {
        assert_eq!(
            uses(
                &["Stir in the sugar.", "Add the white sugar."],
                &["brown sugar", "white sugar"]
            ),
            [step(&[0, 1]), step(&[1])],
            "the choice on #184: both, and the cook picks; a longer \
             mention settles it where the Step gives one"
        );
    }

    #[test]
    fn a_whole_name_beats_a_part_of_another_name() {
        assert_eq!(
            uses(
                &["Add the sugar.", "Heat the vegetable oil."],
                &["sugar", "brown sugar", "sesame oil", "vegetable oil"]
            ),
            [step(&[0]), step(&[3])],
        );
    }

    #[test]
    fn a_food_every_line_of_which_is_used_links_the_first_again() {
        assert_eq!(
            uses(
                &[
                    "Salt the water.",
                    "Season the sauce with salt.",
                    "Salt to finish."
                ],
                &["salt", "salt", "water"]
            ),
            [step(&[0, 2]), step(&[1]), step(&[0])],
        );
    }

    #[test]
    fn a_mention_after_until_is_not_going_in_up_to_the_next_comma() {
        assert_eq!(
            uses(
                &[
                    "Cook until the onions soften, then add the garlic.",
                    "Simmer hasta que el agua se evapore.",
                ],
                &["onion", "garlic", "agua"]
            ),
            [step(&[1]), step(&[])],
        );
        assert_eq!(
            uses(
                &[
                    "Ajoutez jusqu'à 100 ml d'eau.",
                    "Añade hasta 200 ml de agua.",
                    "Laissez cuire jusqu'à ce que l'eau soit évaporée.",
                ],
                &["eau", "agua"]
            ),
            [step(&[0]), step(&[1]), step(&[])],
            "`jusqu'à` and `hasta` before an amount say *up to*, and the water \
             goes in"
        );
    }

    #[test]
    fn an_until_clause_ends_at_the_next_thing_the_cook_does() {
        assert_eq!(
            uses(
                &[
                    "Whisk the eggs until pale then fold in the flour and the sugar.",
                    "Cook the onions until soft and add the garlic.",
                    "Beat until the butter and sugar are pale.",
                ],
                &["egg", "flour", "sugar", "onion", "garlic", "butter"]
            ),
            [step(&[0, 1, 2]), step(&[3, 4]), step(&[])],
            "*and* alone stays inside the condition: the butter and sugar \
             are what should look pale, not what goes in"
        );
        assert_eq!(
            uses(
                &[
                    "Fouettez jusqu'à ce que le beurre et le sucre blanchissent puis ajoutez la farine.",
                    "Cuisez jusqu'à ce que l'oignon soit tendre et ajoutez l'ail.",
                    "Bate hasta que la mantequilla y el azúcar blanqueen y añade la harina.",
                    "Cocina hasta que esté dorado, luego agrega el ajo.",
                ],
                &[
                    "beurre",
                    "sucre",
                    "farine",
                    "oignon",
                    "ail",
                    "mantequilla",
                    "azúcar",
                    "harina",
                    "ajo"
                ]
            ),
            [step(&[2]), step(&[4]), step(&[7]), step(&[8])],
        );
        // On its own, since after the Step before it *el ajo* would be the
        // garlic that Step cooked (#704).
        assert_eq!(
            uses(
                &["Cocina hasta que esté dorado luego agrega el ajo."],
                &["ajo"]
            ),
            [step(&[0])],
        );
        assert_eq!(
            uses(
                &[
                    "Cook until the bottom and top of the tortilla are crisp.",
                    "Hierve hasta que se evapore después de añadir el vino.",
                ],
                &["tortilla", "vino"]
            ),
            [step(&[]), step(&[])],
            "*top* after *and* is part of the condition, and *después de* is \
             *after*, not *then*"
        );
    }

    #[test]
    fn a_describing_word_alone_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Fry until crispy. Serve with the crispy base and fresh herbs.",
                    "Pour in the Greek dressing.",
                ],
                &["crispy chilli oil", "fresh ginger", "Greek yogurt"]
            ),
            [step(&[]), step(&[])],
        );
    }

    #[test]
    fn a_measure_or_a_form_alone_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Line a baking sheet. You can cut the bread into cubes and set aside.",
                    "Stir to a smooth paste.",
                ],
                &[
                    "baking soda",
                    "can chickpeas",
                    "beef broth cubes",
                    "tomato paste"
                ]
            ),
            [step(&[]), step(&[])],
        );
    }

    #[test]
    fn a_fragment_is_never_a_mention_and_a_step_naming_nothing_links_nothing() {
        assert_eq!(
            uses(
                &[
                    "Beat the eggs.",
                    "Check the price of the eggshell substitute.",
                    "Faire revenir l'ail.",
                    "Cover and leave it alone.",
                ],
                &["egg", "rice", "ail"]
            ),
            [step(&[0]), step(&[]), step(&[2]), step(&[])],
        );
    }

    #[test]
    fn an_english_step_links_a_food_by_its_main_word_and_not_by_a_qualifier() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Serve the chicken with steamed rice.",
                    "Add the chicken.",
                    "Shred the beef, sprinkle over almonds and add the vegetables.",
                    "Add the vinegar, then the broth.",
                ],
                &[
                    "chicken thighs",
                    "rice vinegar",
                    "chicken broth",
                    "beef stock",
                    "almond meal",
                    "vegetable oil",
                ]
            ),
            [step(&[0]), step(&[0]), step(&[]), step(&[1, 2])],
        );
    }

    #[test]
    fn a_french_or_spanish_step_links_a_food_by_the_first_word_of_its_name() {
        assert_eq!(
            uses_in(
                Some("fr"),
                &[
                    "Servez le poulet avec du riz vapeur.",
                    "Ajoutez le vinaigre et le bouillon.",
                ],
                &[
                    "hauts de cuisse de poulet",
                    "vinaigre de riz",
                    "bouillon de poulet",
                ]
            ),
            [step(&[0]), step(&[1, 2])],
        );
        assert_eq!(
            uses_in(
                Some("es"),
                &["Sirve el pollo con arroz.", "Añade el vinagre y el caldo."],
                &["muslos de pollo", "vinagre de arroz", "caldo de pollo"]
            ),
            [step(&[0]), step(&[1, 2])],
        );
    }

    /// A recipe's title, its Language, one Step, its Readings, and the lines
    /// that Step links.
    type Titled<'a> = (&'a str, &'a str, &'a str, &'a [&'a str], &'a [usize]);

    /// The rows of #357: a part of an ingredient's name that the title also
    /// holds names the dish, and a part the title does not hold still links.
    #[test]
    fn a_step_naming_the_dish_by_a_word_of_its_title_links_nothing() {
        let rows: &[Titled] = &[
            (
                "Vietnamese beef curry",
                "en",
                "Ladle the curry into a soup tureen and serve immediately.",
                &["beef", "curry powder", "coconut milk"],
                &[],
            ),
            (
                "Curry de bœuf vietnamien",
                "fr",
                "Versez le curry dans une soupière et servez aussitôt.",
                &["bœuf", "curry en poudre"],
                &[],
            ),
            (
                "Chocolate cake",
                "en",
                "Sift in the cocoa.",
                &["cocoa powder", "sugar"],
                &[0],
            ),
            (
                "Gâteau au chocolat",
                "fr",
                "Ajoutez le cacao.",
                &["cacao en poudre"],
                &[0],
            ),
            (
                "Thai red curry",
                "en",
                "Ladle the curry into bowls.",
                &["red curry paste", "coconut milk"],
                &[],
            ),
            (
                "Hot cocoa",
                "en",
                "Pour the cocoa into mugs.",
                &["cocoa powder", "milk"],
                &[],
            ),
        ];
        for (title, language, text, targets, linked) in rows {
            assert_eq!(
                uses_titled(title, Some(language), &[text], targets),
                [step(linked)],
                "{text:?} in {title:?}"
            );
        }
    }

    #[test]
    fn an_ingredients_whole_name_links_in_a_dish_named_after_it() {
        assert_eq!(
            uses_titled(
                "Vietnamese beef curry",
                Some("en"),
                &["Add the curry powder and stir for a minute."],
                &["beef", "curry powder", "coconut milk"],
            ),
            [step(&[1])],
        );
        assert_eq!(
            uses_titled(
                "Garlic bread",
                Some("en"),
                &["Add the garlic to the butter."],
                &["garlic", "butter"],
            ),
            [step(&[0, 1])],
        );
    }

    /// A title's word is compared as the Step's mention was found: *de* in
    /// the title is not *les dés* in the Step, as *the* is no tea (#230).
    #[test]
    fn a_title_word_an_accent_sets_apart_is_another_word() {
        assert_eq!(
            uses_titled(
                "Salade de pâtes au jambon",
                Some("fr"),
                &["Ajoutez les dés et le miel."],
                &["dés de jambon", "miel"],
            ),
            [step(&[0, 1])],
        );
    }

    /// A part of a name the title holds still claims its words from a
    /// shorter part of another name it overlaps: the dish is named, so
    /// nothing is.
    #[test]
    fn a_mention_naming_the_dish_still_beats_a_shorter_one_it_overlaps() {
        assert_eq!(
            uses_titled(
                "Thai red curry",
                Some("en"),
                &["Stir in the Thai red curry leaves."],
                &["Thai red curry paste", "fresh curry leaves"],
            ),
            [step(&[])],
        );
    }

    #[test]
    fn an_empty_title_takes_no_mention_away() {
        for title in ["", "   "] {
            assert_eq!(
                uses_titled(
                    title,
                    Some("en"),
                    &["Ladle the curry into bowls."],
                    &["curry powder"],
                ),
                [step(&[0])],
            );
        }
    }

    #[test]
    fn a_step_still_links_a_food_it_names_without_its_cut_or_its_form() {
        let targets = [
            "chicken thighs",
            "chicken breast",
            "chicken drumsticks",
            "garlic cloves",
            "kale leaves",
            "salmon fillet",
            "pork mince",
            "vanilla extract",
            "miso paste",
            "Parmesan cheese",
            "cheddar cheese",
            "soy sauce",
            "bicarbonate of soda",
        ];
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    // Seasoned, not cooked, so *sear the thighs* still adds
                    // them (#704).
                    "Season the chicken.",
                    "Add the garlic, kale, salmon and pork.",
                    "Stir in the vanilla and the miso.",
                    "Grate over the parmesan.",
                    "Melt the cheese.",
                    "Sear the thighs, then add the soy and the bicarbonate.",
                ],
                &targets
            ),
            [
                step(&[0, 1, 2]),
                step(&[3, 4, 5, 6]),
                step(&[7, 8]),
                step(&[9]),
                step(&[9, 10]),
                step(&[0, 12]),
            ],
            "the cut by itself still links its line; *the soy* no longer \
             links soy sauce, the one loss accepted on #212"
        );
    }

    #[test]
    fn the_recipes_of_184_link_the_same_lines_read_in_their_own_language() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("en", CAKE_STEPS, CAKE_TARGETS),
            ("fr", FRENCH_CAKE_STEPS, FRENCH_CAKE_TARGETS),
            ("en", DUMPLING_STEPS, DUMPLING_TARGETS),
            ("fr", FRENCH_DUMPLING_STEPS, FRENCH_DUMPLING_TARGETS),
        ];
        for (language, steps, targets) in cases {
            assert_eq!(
                uses_in(Some(language), steps, targets),
                uses(steps, targets),
                "{language}: {}",
                targets[1]
            );
        }
        assert_eq!(
            uses_in(
                Some("en"),
                &["Grate the ginger."],
                &["fresh ginger", "ginger beer"]
            ),
            [step(&[0])],
        );
    }

    #[test]
    fn a_branch_with_no_language_reads_a_name_from_both_ends() {
        let steps = ["Serve with rice.", "Ajoutez le vinaigre, puis le sel."];
        let targets = ["rice vinegar", "vinaigre de riz", "sel de mer"];
        for language in [None, Some("unknown")] {
            assert_eq!(
                uses_in(language, &steps, &targets),
                [step(&[0]), step(&[1, 2])],
                "with no Language to say which end the main word sits at, \
                 either end stands for the name"
            );
        }
        assert_eq!(
            uses_in(Some("en"), &steps, &targets),
            [step(&[]), step(&[])]
        );
    }

    /// Names the reader left untidy on a development library, each of which
    /// lost a right link to the first cut of the main-word rule (#212).
    #[test]
    fn a_name_holding_two_foods_an_amount_or_a_tail_keeps_its_main_words() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Season with a pinch of salt, then the thyme.",
                    "Slice the chicken.",
                    "Add sherry and toss the panko with the oil. Serve with udon noodles.",
                ],
                &[
                    "salt & freshly ground black pepper",
                    "fresh thyme and basil",
                    "chicken drumsticks and bone-in thigh pieces",
                    "dry sherry such as Amontillado",
                    "panko bread crumbs",
                    "vegetable oil for frying",
                    "pouches ready to wok udon noodles",
                    "chicken or vegetable stock",
                ]
            ),
            [step(&[0, 1]), step(&[2]), step(&[3, 4, 5, 6])],
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Disposez les tomates, puis les enoki.", "Chauffez l'huile."],
                &[
                    "3 paquets d'Enoki",
                    "peu huile pour la cuisson",
                    "pâte à pizza",
                    "pâte de miso"
                ]
            ),
            [step(&[0]), step(&[1])],
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Garnissez la pizza.", "Délayez le miso."],
                &["pâte à pizza", "pâte de miso"]
            ),
            [step(&[]), step(&[1])],
        );
    }

    /// One-pot spicy halal cart chicken over rice, as imported on 2026-10-23:
    /// one food on lines of several sections, and a title holding the rice
    /// and the chicken (#439).
    const HALAL_TITLE: &str = "One-pot spicy halal cart chicken over rice";
    const HALAL_TARGETS: &[&str] = &[
        "#Spice blend",
        "cumin",
        "ground coriander",
        "smoked paprika",
        "curry powder",
        "cayenne pepper",
        "dried oregano",
        "dried parsley flakes",
        "black pepper",
        "#For the chicken",
        "boneless skinless chicken thighs",
        "Kosher salt",
        "olive oil",
        "lemon juice",
        "spice blend",
        "#For the rice",
        "basmati rice",
        "tomato paste",
        "smoked paprika",
        "spice blend",
        "chicken bouillon cubes",
        "water",
        "Neutral oil",
        "#For the white sauce",
        "mayo",
        "fresh dill",
        "fresh parsley",
        "garlic",
        "lemon zest",
        "Lemon juice",
        "water",
        "Salt",
        "Black pepper",
        "#For serving",
        "Tomatoes",
        "shredded iceberg lettuce",
        "red onion",
        "Hot sauce",
    ];
    const HALAL_STEPS: &[&str] = &[
        "Combine all the spice blend ingredients in a small bowl and mix well, then set aside.",
        "Coat the chicken thighs in the olive oil, lemon juice and all of the spice blend, reserving 1 tablespoon for the rice.",
        "For deeper flavor, refrigerate for 30 minutes or longer (overnight is even better). If you're short on time, proceed straight to searing.",
        "Heat a generous drizzle of neutral oil in a large heavy-bottomed pot over high heat.",
        "Sear the chicken thighs until deeply caramelized, about 4 to 5 minutes per side, cooking them about 70% of the way through.",
        "Remove the chicken and set aside.",
        "Turn the heat down to medium.",
        "Add the tomato paste to the same pot and fry, stirring constantly, for 2 to 3 minutes until it darkens slightly.",
        "Add the reserved spice blend and the extra smoked paprika and cook for another minute until fragrant.",
        "Add the bouillon cubes and stir to combine.",
        "Add the washed and drained rice and stir to coat in the tomato-spice mixture.",
        "Add the water. Once the water level is even with the rice, nestle the chicken thighs back in, cover, and reduce to the lowest heat.",
        "Cook for 20 minutes, then remove from the heat and let steam, covered, for another 10 minutes.",
        "While the rice and chicken cook, make the white sauce: whisk together the mayo, dill, parsley, grated garlic, lemon zest and lemon juice.",
        "Add the water a tablespoon at a time until pourable but still creamy. Season with salt and pepper.",
        "Refrigerate until ready to serve.",
        "When the rice and chicken are ready, remove the chicken and chop into small pieces.",
        "Fluff the rice with a fork.",
        "To serve: a bed of rice, chicken on top, with shredded lettuce, diced tomato and sliced red onion on the side.",
        "Drizzle generously with white sauce and hot sauce.",
    ];

    #[test]
    fn a_step_links_the_line_of_the_section_it_works_in() {
        assert_eq!(
            uses_titled(HALAL_TITLE, Some("en"), HALAL_STEPS, HALAL_TARGETS),
            [
                step(&[1, 2, 3, 4, 5, 6, 7, 8]),
                step(&[10, 12, 13, 14]),
                step(&[]),
                step(&[22]),
                step(&[10]),
                // The seared chicken is taken out, nestled back in and taken
                // out again, and goes in once (#704).
                step(&[]),
                step(&[]),
                step(&[17]),
                step(&[18, 19]),
                step(&[20]),
                step(&[16]),
                step(&[21]),
                step(&[]),
                step(&[24, 25, 26, 27, 28, 29]),
                step(&[30, 31, 32]),
                step(&[]),
                step(&[]),
                step(&[16]),
                step(&[34, 35, 36]),
                step(&[37]),
            ]
        );
    }

    /// Honey lemon garlic chicken, as imported on 2026-10-23 (#439).
    #[test]
    fn a_title_word_links_the_food_itself_and_the_sauce_is_the_recipes_own() {
        let targets = [
            "#Chicken",
            "chicken thighs",
            "Salt",
            "Black pepper",
            "Cornstarch",
            "Neutral oil",
            "#Sauce",
            "butter",
            "garlic",
            "soy sauce",
            "mirin",
            "honey",
            "lemon",
            "lemon zest",
            "paprika",
            "chili flakes",
            "black pepper",
            "#Garnish",
            "Sesame seeds",
            "Chives",
        ];
        let steps = [
            "Season the chicken thighs with salt and pepper.",
            "Coat the chicken with cornstarch and shake off the excess.",
            "Heat the oil in a pan over medium-high heat.",
            "Add the chicken skin side down and fry each side for 7 minutes or until golden brown.",
            "Set the chicken aside.",
            "Melt the butter in a clean pan over medium heat.",
            "Sauté the garlic until fragrant, for 1 minute.",
            "Add the soy sauce, mirin, honey, lemon juice, lemon zest, paprika, chili flakes and black pepper.",
            "Cook the sauce until it thickens.",
            "Add the chicken back to the pan and baste it with the sauce.",
            "Serve with hot steamed rice.",
            "Garnish with sesame seeds and chives.",
        ];
        assert_eq!(
            uses_titled("Honey lemon garlic chicken", Some("en"), &steps, &targets),
            [
                step(&[1, 2, 3]),
                step(&[1, 4]),
                step(&[5]),
                step(&[1]),
                // Fried in the Step before, the chicken is set aside and put
                // back, and goes in no more (#704).
                step(&[]),
                step(&[7]),
                step(&[8]),
                step(&[9, 10, 11, 12, 13, 14, 15, 16]),
                step(&[]),
                step(&[]),
                step(&[]),
                step(&[18, 19]),
            ]
        );
    }

    /// Beef and broccoli, as imported on 2026-10-23 (#439).
    #[test]
    fn a_word_links_the_food_itself_before_what_is_made_from_it() {
        let targets = [
            "beef",
            "cornstarch",
            "paprika",
            "Garlic powder",
            "Black pepper",
            "olive oil",
            "garlic cloves",
            "honey",
            "soy sauce",
            "Broccoli",
        ];
        let steps = [
            "Toss the shaved steak with the cornstarch, paprika, garlic powder and black pepper until it is lightly coated.",
            "Preheat the olive oil in a pan, then add the beef.",
            "Add three cloves of garlic.",
            "Once the beef is almost fully cooked, add the soy sauce and honey.",
        ];
        assert_eq!(
            uses_titled("Beef and broccoli", Some("en"), &steps, &targets),
            [
                step(&[1, 2, 3, 4]),
                step(&[0, 5]),
                step(&[6]),
                step(&[7, 8]),
            ]
        );
        // With only the powder in the list, the garlic is the powder.
        assert_eq!(
            uses_in(Some("en"), &["Add the garlic."], &["Garlic powder", "salt"]),
            [step(&[0])]
        );
    }

    /// Red curry cabbage cups, as imported on 2026-10-23 (#439).
    #[test]
    fn a_step_naming_a_section_s_ingredients_links_every_line_of_it() {
        let targets = [
            "#Cabbage",
            "cabbage",
            "salt",
            "#Chicken filling",
            "ground chicken",
            "panko",
            "spring onion",
            "coriander",
            "red curry paste",
            "ginger",
            "brown sugar",
            "fish sauce",
            "lime",
            "egg",
            "#Red curry sauce",
            "red curry paste",
            "coconut milk",
            "soy sauce",
            "rice vinegar",
        ];
        let steps = [
            "In a bowl, mix all chicken filling ingredients until just combined.",
            "Whisk all red curry sauce ingredients and pour over cabbage cups.",
            "Spoon sauce over to serve.",
        ];
        assert_eq!(
            uses_titled("Red curry cabbage cups", Some("en"), &steps, &targets),
            [
                step(&[4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
                step(&[1, 15, 16, 17, 18]),
                step(&[]),
            ]
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Fouettez les ingrédients de la sauce."],
                &[
                    "#Pour la sauce",
                    "sauce soja",
                    "miel",
                    "#Garniture",
                    "sésame"
                ]
            ),
            [step(&[1, 2])]
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &["Mix all the marinade ingredients."],
                &[
                    "#Marinade ingredients",
                    "soy sauce",
                    "honey",
                    "#To serve",
                    "rice"
                ]
            ),
            [step(&[1, 2])]
        );
    }

    #[test]
    fn a_when_clause_ends_at_the_next_thing_to_do() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "When the butter melts add the flour.",
                    "Once cool, stir in the butter."
                ],
                &["butter", "flour"]
            ),
            [step(&[1]), step(&[0])]
        );
    }

    /// Cases a library of 171 recipes showed while #439 was built.
    #[test]
    fn what_a_real_library_showed() {
        assert_eq!(
            uses_in(
                Some("en"),
                &["Add the sauce ingredients and the sugar."],
                &[
                    "chicken thighs",
                    "#Teriyaki sauce",
                    "soy sauce",
                    "mirin",
                    "#To serve",
                    "rice"
                ]
            ),
            [step(&[2, 3])],
            "a heading named by its main word"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Mix the soy sauce, salt, and pepper.",
                    "Fry the salmon with salt and pepper."
                ],
                &[
                    "soy sauce",
                    "Salt",
                    "Pepper",
                    "Salmon fillet",
                    "Salt and pepper"
                ]
            ),
            [step(&[0, 1, 2, 4]), step(&[1, 2, 3, 4])],
            "a name holding two foods beats neither, as before #439"
        );
        assert_eq!(
            uses_titled(
                "Coq au vin",
                None,
                &["Add the chicken stock.", "Ajoutez le bouillon de poulet."],
                &["chicken drumsticks", "poulet"]
            ),
            [step(&[]), step(&[])],
            "a stock holds none of what it is made from"
        );
        assert_eq!(
            uses_in(Some("fr"), &["Étaler la pâte."], &["pâte brisée", "pommes"]),
            [step(&[0])],
            "la pâte is the bought pastry"
        );
    }

    #[test]
    fn a_plural_in_oes_is_its_singular() {
        assert_eq!(
            uses_in(
                Some("en"),
                &["Add the diced tomato.", "Peel the potatoes."],
                &["Tomatoes", "tomato paste", "potato"]
            ),
            [step(&[0]), step(&[2])]
        );
        assert_eq!(
            uses_in(Some("en"), &["Prick each sloe."], &["sloes"]),
            [step(&[0])],
            "a singular in -oe keeps its plural in -s"
        );
    }

    #[test]
    fn a_food_naming_what_the_recipe_makes_or_what_it_is_for_is_not_going_in() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Pour the egg mixture into the pan.",
                    "Keep the butter for the rice.",
                    "Whisk while adding the oil.",
                ],
                &["eggs", "butter", "rice", "oil"]
            ),
            [step(&[]), step(&[1]), step(&[3])]
        );
        assert_eq!(
            uses_in(
                Some("fr"),
                &[
                    "Versez le mélange de tomates.",
                    "Gardez le beurre pour le riz."
                ],
                &["tomates", "beurre", "riz"]
            ),
            [step(&[]), step(&[1])]
        );
    }

    /// The whole-name rule as it stood before #184, kept here to hold the new
    /// one to its promise: every link it made is still made, except where one
    /// food on two lines now links one of them, or the mention sits after
    /// *until*.
    fn linked_before(steps: &[&str], targets: &[&str]) -> Vec<Vec<usize>> {
        steps
            .iter()
            .map(|text| {
                let text = folded_for_search(text);
                targets
                    .iter()
                    .enumerate()
                    .filter(|(_, target)| !target.is_empty())
                    .filter(|(_, target)| {
                        let target = folded_for_search(target.trim());
                        let target = target.strip_suffix('s').unwrap_or(&target);
                        text.match_indices(target).any(|(at, _)| {
                            let mut end = at + target.len();
                            if text[end..].starts_with('s') {
                                end += 1;
                            }
                            !text[..at]
                                .chars()
                                .next_back()
                                .is_some_and(char::is_alphanumeric)
                                && !text[end..]
                                    .chars()
                                    .next()
                                    .is_some_and(char::is_alphanumeric)
                        })
                    })
                    .map(|(index, _)| index)
                    .collect()
            })
            .collect()
    }

    /// One recipe's Steps and Readings, and the (Step, line) links it may lose.
    type Case<'a> = (&'a [&'a str], &'a [&'a str], &'a [(usize, usize)]);

    #[test]
    fn every_link_the_whole_name_rule_made_is_still_made() {
        // Each case, with the (Step, line) links it is allowed to lose: one
        // food on two lines now linking one of them, or a mention after
        // *until* (choices 2 and 3 on #184). Nothing else may go.
        let cases: &[Case] = &[
            (CAKE_STEPS, CAKE_TARGETS, &[]),
            (
                DUMPLING_STEPS,
                DUMPLING_TARGETS,
                &[
                    // The filling takes the filling's spring onions and soy
                    // sauce, not those to serve or for the dipping sauce.
                    (1, 10),
                    (1, 16),
                    // `until ... the water has evaporated`.
                    (10, 14),
                    // The dipping sauce's soy sauce, not the filling's.
                    (16, 2),
                    // The spring onion to serve, not the filling's.
                    (17, 5),
                ],
            ),
            (
                &[
                    "Coat the chicken breast in panko.",
                    "Pour in the water and simmer for about 7 minutes.",
                    "Cover and leave it alone.",
                ],
                &["", "chicken", "panko", "water", "", ""],
                &[],
            ),
            (
                &[
                    "Whisk the eggs until pale then fold in the flour and the sugar.",
                    "Cook the onions until soft and add the garlic.",
                ],
                &["egg", "flour", "sugar", "onion", "garlic"],
                &[],
            ),
            (
                &[
                    "Brown the minced garlic and the chopped onion in olive oil.",
                    "Add the brown sugar, the sugar and a pinch of sea salt.",
                    "Pour in the double cream and the tomatoes.",
                ],
                &[
                    "minced garlic",
                    "onion",
                    "olive oil",
                    "sugar",
                    "brown sugar",
                    "sea salt",
                    "double cream",
                    "tomates",
                ],
                &[],
            ),
        ];
        for (steps, targets, may_lose) in cases {
            let before = linked_before(steps, targets);
            let now = uses(steps, targets);
            let mut lost = Vec::new();
            for (at, (before, now)) in before.iter().zip(&now).enumerate() {
                let Some(now) = now else {
                    continue;
                };
                lost.extend(
                    before
                        .iter()
                        .filter(|line| !now.contains(line))
                        .map(|&line| (at, line)),
                );
            }
            assert_eq!(
                lost, *may_lose,
                "the (Step, line) links the whole-name rule made that are gone"
            );
        }
    }

    #[test]
    fn a_dutch_step_names_a_compound_by_its_main_word() {
        let nl = Some("nl");
        assert_eq!(
            uses_in(nl, &["Voeg de bloem toe."], &["tarwebloem"]),
            vec![Some(vec![0])]
        );
        assert_eq!(
            uses_in(nl, &["Giet de bouillon erbij."], &["kippenbouillon"]),
            vec![Some(vec![0])]
        );
        // The other end of a compound is no main word, and a potato is no
        // apple.
        assert_eq!(
            uses_in(nl, &["Voeg de tarwe toe."], &["tarwebloem"]),
            vec![Some(vec![])]
        );
        assert_eq!(
            uses_in(nl, &["Schil de appels."], &["aardappels"]),
            vec![Some(vec![])]
        );
        // A word for what the recipe makes is no line of the list.
        assert_eq!(
            uses_in(nl, &["Roer de saus."], &["sojasaus"]),
            vec![Some(vec![])]
        );
    }

    #[test]
    fn a_line_named_whole_wins_over_a_compound_of_the_same_word() {
        assert_eq!(
            uses_in(
                Some("nl"),
                &["Voeg de bloem toe."],
                &["tarwebloem", "bloem"]
            ),
            vec![Some(vec![1])]
        );
    }

    #[test]
    fn a_dutch_step_names_the_food_of_a_part_of_it() {
        let nl = Some("nl");
        assert_eq!(
            uses_in(nl, &["Bak de kip."], &["kipfilet"]),
            vec![Some(vec![0])]
        );
        assert_eq!(
            uses_in(nl, &["Voeg de laurier toe."], &["laurierblaadjes"]),
            vec![Some(vec![0])]
        );
    }

    #[test]
    fn a_dutch_plural_and_small_one_name_the_line() {
        let nl = Some("nl");
        for (step, target) in [
            ("Snipper de ui.", "uien"),
            ("Snipper de uien.", "ui"),
            ("Klop de eieren.", "ei"),
            ("Snijd de tomaten.", "tomaat"),
            ("Snijd de paprika.", "paprika's"),
            ("Fruit het uitje.", "ui"),
        ] {
            assert_eq!(
                uses_in(nl, &[step], &[target]),
                vec![Some(vec![0])],
                "{step}"
            );
        }
        assert_eq!(
            uses_in(nl, &["Snipper de ui."], &["rode ui"]),
            vec![Some(vec![0])]
        );
    }

    #[test]
    fn what_comes_after_tot_is_not_going_in() {
        let nl = Some("nl");
        assert_eq!(
            uses_in(nl, &["Kook tot het water verdampt is."], &["water"]),
            vec![Some(vec![])]
        );
        // A number after it is a range, or an amount going in.
        assert_eq!(
            uses_in(nl, &["Voeg tot 100 ml water toe."], &["water"]),
            vec![Some(vec![0])]
        );
        // `als laatste` adds the last thing.
        assert_eq!(
            uses_in(nl, &["Voeg als laatste de ananas toe."], &["ananas"]),
            vec![Some(vec![0])]
        );
    }

    #[test]
    fn an_english_recipe_finds_nothing_inside_a_word() {
        assert_eq!(
            uses_in(Some("en"), &["Add the bloem."], &["tarwebloem"]),
            vec![Some(vec![])]
        );
    }

    /// A recipe as it is stored: each line read by the real reader, so a line
    /// offering a choice arrives with no Food, as it does from a save. A `#`
    /// prefix makes a line a Section.
    fn uses_as_read(
        title: &str,
        language: &str,
        steps: &[&str],
        lines: &[&str],
    ) -> Vec<Option<Vec<usize>>> {
        let steps: Vec<Value> = steps
            .iter()
            .map(|text| json!({ "kind": "step", "text": text }))
            .collect();
        let ingredients: Vec<Value> = lines
            .iter()
            .map(|line| match line.strip_prefix('#') {
                Some(section) => json!({ "kind": "section", "text": section }),
                None => json!({ "kind": "ingredient", "text": line }),
            })
            .collect();
        let readings: Vec<Value> = lines
            .iter()
            .map(|line| match reading::read_line(line, language) {
                Some(read) if !line.starts_with('#') => json!({
                    "amount": read.amount,
                    "unit": read.unit,
                    "target": read.target,
                    "lineage_id": null,
                }),
                _ => Value::Null,
            })
            .collect();
        step_uses(&steps, &ingredients, &readings, Some(language), title)
    }

    /// The Sample Library's Dan Dan Noodles, as `sample-library/story.json`
    /// writes it: the recipe #697 was filed from.
    const DAN_DAN_LINES: &[&str] = &[
        "2 tbsp neutral oil",
        "2 scallions (sliced; white for cooking and green for garnish)",
        "1/2 lbs ground pork or beef",
        "2 tsp minced ginger",
        "1/4 cup yacai or preserved mustard greens",
        "2 tbsp shoaxing wine",
        "1 tsp light soy sauce",
        "1/2 tsp dark soy sauce",
        "1 tsp sugar (optional)",
        "2 bundles fresh wheat noodles",
        "Choy sum",
        "Crushed peanuts",
        "#Sauce",
        "3 tsp sesame paste",
        "2 tbsp light soy sauce",
        "1/2 tsp dark soy sauce",
        "1 tbsp Chinese black vinegar",
        "1 tsp sesame oil",
        "5 tbsp Sichuan chili oil",
        "1 tsp sugar",
        "2 cloves garlic (grated)",
    ];
    const DAN_DAN_STEPS: &[&str] = &[
        "In a pan, heat oil on medium heat and cook mince until the water has evaporated and the fat starts to render and sizzle (6-8 minutes).",
        "Add ginger, white portion of scallions, and yacai. Sauté for a couple of minutes until fragrant.",
        "Add shoaxing wine and once alcohol has evaporated, season with light and dark soy sauce and sugar. Set aside on low heat to keep warm.",
        "Combine all sauce ingredients.",
        "Boil noodles according to package instructions and cook choy sum in the last 45 seconds of boiling.",
        "Combine the noodles with the sauce and don\u{2019}t be afraid to add a little bit of the water if the sauce is too thick. Plate with the choy sum, cooked mince, green portion of scallions, crushed peanuts, and more chili oil.",
    ];

    #[test]
    fn the_dan_dan_noodles_link_the_mince_the_yacai_and_each_soy_sauce() {
        assert_eq!(
            uses_as_read("Dan Dan Noodles", "en", DAN_DAN_STEPS, DAN_DAN_LINES),
            [
                // The neutral oil alone: the sauce's oils are the sauce's.
                step(&[0, 2]),
                step(&[1, 3, 4]),
                // *Light and dark soy sauce* is two lines.
                step(&[5, 6, 7, 8]),
                step(&[13, 14, 15, 16, 17, 18, 19, 20]),
                step(&[9, 10]),
                // *More chili oil* is beyond the sauce's five spoons, and the
                // noodles, the choy sum and the *cooked mince* went in
                // earlier: only the scallions' green portion and the peanuts
                // are new (#704).
                step(&[1, 11]),
            ]
        );
    }

    #[test]
    fn the_dan_dan_noodles_link_the_same_with_the_foods_the_sample_library_sets() {
        // `sample-library/story.json` sets seven of these by hand, the two
        // choices among them, and the reader reads the rest.
        let targets = &[
            "neutral oil",
            "scallions",
            "ground pork or beef",
            "ginger",
            "yacai or preserved mustard greens",
            "Chinese Shaoxing wine",
            "light soy sauce",
            "dark soy sauce",
            "sugar",
            "fresh wheat noodles",
            "Choy sum",
            "peanuts",
            "#Sauce",
            "Chinese sesame paste",
            "light soy sauce",
            "dark soy sauce",
            "Chinese black vinegar",
            "sesame oil",
            "chili oil",
            "sugar",
            "garlic",
        ];
        assert_eq!(
            uses_titled("Dan Dan Noodles", Some("en"), DAN_DAN_STEPS, targets),
            [
                step(&[0, 2]),
                step(&[1, 3, 4]),
                // *Shoaxing wine* is the *Chinese Shaoxing wine* by its wine.
                step(&[5, 6, 7, 8]),
                step(&[13, 14, 15, 16, 17, 18, 19, 20]),
                step(&[9, 10]),
                step(&[1, 11]),
            ]
        );
    }

    #[test]
    fn the_tomato_soup_links_the_coconut_milk_and_leaves_the_thyme_out() {
        let lines = &[
            "500-700 g cherry tomatoes",
            "1 head of garlic",
            "1 yellow onion",
            "1/4 red chile",
            "olive oil",
            "fresh thyme and basil",
            "salt",
            "pepper",
            "½ cup full fat canned coconut milk or heavy cream",
            "⅔ cup vegetable stock",
        ];
        let steps = &[
            "Preheat the oven at 250°C or 480°F.",
            "Drizzle some oil onto a baking pan, add in the yellow onion, head of garlic (cut side down), fresh basil and thyme, red chile, cherry tomatoes, more olive oil on top, a sprinkle of salt and pepper. Bake everything until the tomatoes are slightly charred for 25 to 35 minutes.",
            "Add the coconut milk, vegetable stock and all the baked ingredients, besides the thyme, in a blender and blend everything until smooth, for 3-5 minutes. Serve with a piece of toast or a grilled cheese sandwich.",
        ];
        assert_eq!(
            uses_as_read("Tomato soup", "en", steps, lines),
            [step(&[]), step(&[0, 1, 2, 3, 4, 5, 6, 7]), step(&[8, 9])]
        );
    }

    #[test]
    fn a_line_offering_a_choice_is_named_by_either_option() {
        let lines = &["1 cup butter or oil", "soft or silken tofu", "flour"];
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &["Melt the butter.", "Heat the oil.", "Press the tofu."],
                lines
            ),
            [step(&[0]), step(&[0]), step(&[1])]
        );
        // The option *soft* is no tofu.
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Knead until soft, then rest it."], lines),
            [step(&[])]
        );
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Keep it soft."], lines),
            [step(&[])]
        );
    }

    #[test]
    fn mince_is_the_ground_meat_and_never_the_verb() {
        let lines = &["500 g ground beef", "2 cloves garlic"];
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &[
                    "Brown the mince.",
                    "Mince the garlic.",
                    "Finely mince garlic.",
                    "Mince 2 cloves of garlic."
                ],
                lines
            ),
            [step(&[0]), step(&[1]), step(&[1]), step(&[1])]
        );
    }

    #[test]
    fn more_of_a_line_no_step_used_yet_is_that_line() {
        assert_eq!(
            uses(
                &["Toss the noodles.", "Serve with more chili oil."],
                &["noodles", "chili oil"]
            ),
            [step(&[0]), step(&[1])]
        );
        assert_eq!(
            uses(
                &["Toss in the chili oil.", "Serve with more chili oil."],
                &["chili oil"]
            ),
            [step(&[0]), step(&[])]
        );
    }

    #[test]
    fn a_first_step_naming_a_word_two_sections_share_takes_the_first_section() {
        assert_eq!(
            uses(
                &["Heat the oil.", "Combine all sauce ingredients."],
                &["neutral oil", "#Sauce", "sesame oil"]
            ),
            [step(&[0]), step(&[2])]
        );
    }

    #[test]
    fn a_food_left_out_with_besides_or_except_is_not_going_in() {
        assert_eq!(
            uses(
                &[
                    "Blend it all, besides the thyme, with the stock.",
                    "Add everything except the salt."
                ],
                &["thyme", "stock", "salt"]
            ),
            [step(&[1]), step(&[])]
        );
    }

    #[test]
    fn a_first_step_tie_takes_the_earliest_section_whatever_the_order_named() {
        let lines = &["neutral oil", "#Sauce", "sesame paste", "sesame oil"];
        // *Paste* gives the sauce the lead, whichever word comes first.
        assert_eq!(
            uses(&["Mix the sesame paste and oil."], lines),
            [step(&[2, 3])]
        );
        assert_eq!(
            uses(&["Warm the oil and sesame paste."], lines),
            [step(&[2, 3])]
        );
        // A tie goes to the first section, though the sauce was named first.
        let lines = &[
            "#Noodles",
            "noodles",
            "peanut oil",
            "#Sauce",
            "sesame paste",
            "sesame oil",
        ];
        assert_eq!(
            uses(&["Add the sesame paste, the noodles and the oil."], lines),
            [step(&[1, 2, 4])]
        );
    }

    #[test]
    fn a_shared_tail_left_out_is_not_going_in() {
        assert_eq!(
            uses(
                &["Add everything besides light and dark soy sauce."],
                &["light soy sauce", "dark soy sauce"]
            ),
            [step(&[])]
        );
    }

    #[test]
    fn a_shared_tail_names_both_lines() {
        assert_eq!(
            uses(
                &["Season with light and dark soy sauce."],
                &["light soy sauce", "dark soy sauce"]
            ),
            [step(&[0, 1])]
        );
    }

    #[test]
    fn a_food_stored_as_a_choice_is_named_by_either_option() {
        // The Sample Library's Dan Dan Noodles holds these Foods as set.
        assert_eq!(
            uses_in(
                Some("en"),
                &["Cook mince.", "Add ginger and yacai."],
                &[
                    "ground pork or beef",
                    "ginger",
                    "yacai or preserved mustard greens"
                ]
            ),
            [step(&[0]), step(&[1, 2])]
        );
    }

    #[test]
    fn a_one_word_option_before_a_made_food_gives_way_to_the_food_itself() {
        let lines = &[
            "1 cup butter or olive oil",
            "4 chicken thighs",
            "2 cups chicken or vegetable stock",
        ];
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &[
                    "Melt the butter.",
                    "Brown the chicken.",
                    "Pour in the stock."
                ],
                lines
            ),
            [step(&[0]), step(&[1]), step(&[2])]
        );
    }

    #[test]
    fn a_choice_option_followed_by_anything_but_another_food_still_links() {
        let pork = &["1/2 lbs ground pork or beef"];
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Brown the beef 5 minutes."], pork),
            [step(&[0])]
        );
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Fry the mince 5 minutes."], pork),
            [step(&[0])]
        );
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &["Stir in the cream gently."],
                &["½ cup coconut milk or heavy cream"]
            ),
            [step(&[0])]
        );
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "fr",
                &["Ajoutez le beurre fondu."],
                &["200 g de beurre ou d'huile"]
            ),
            [step(&[0])]
        );
        // Nor does a title holding it hide it.
        assert_eq!(
            uses_as_read(
                "Garlic butter noodles",
                "en",
                &["Melt the butter."],
                &["2 tbsp butter or olive oil"]
            ),
            [step(&[0])]
        );
    }

    #[test]
    fn mince_in_a_list_of_foods_is_the_meat() {
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &["Add the onion, garlic, mince and tomato paste."],
                &[
                    "500 g ground beef",
                    "1 onion",
                    "2 cloves garlic",
                    "1 tbsp tomato paste"
                ]
            ),
            [step(&[0, 1, 2, 3])]
        );
        assert_eq!(
            uses_as_read(
                NO_DISH,
                "en",
                &["Plate with the noodles, mince, and peanuts."],
                &["500 g ground beef", "noodles", "peanuts"]
            ),
            [step(&[0, 1, 2])]
        );
    }

    #[test]
    fn except_an_amount_adds_the_rest() {
        assert_eq!(
            uses(
                &["Add everything except 2 tbsp of the butter."],
                &["butter"]
            ),
            [step(&[0])]
        );
    }

    #[test]
    fn a_word_of_a_choice_describing_another_food_is_no_mention_of_it() {
        let lines = &["1 tsp smoked or sweet paprika", "1 sausage"];
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Add the smoked sausage."], lines),
            [step(&[1])]
        );
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Stir in the smoked paprika."], lines),
            [step(&[0])]
        );
    }

    #[test]
    fn an_option_of_a_choice_is_read_apart_from_the_other() {
        // *Milk* is no more made from *water* than a dish named after it.
        assert_eq!(
            uses_as_read(
                "Milk bread",
                "en",
                &["Warm the milk."],
                &["1 cup milk or water"]
            ),
            [step(&[0])]
        );
    }

    #[test]
    fn more_after_a_count_is_the_rest_of_the_line() {
        assert_eq!(
            uses(
                &["Beat in 2 eggs.", "Fold in the flour, then 2 more eggs."],
                &["eggs", "flour"]
            ),
            [step(&[0]), step(&[0, 1])]
        );
        // French writes the food after *davantage de*.
        assert_eq!(
            uses_in(
                Some("fr"),
                &["Versez l'huile.", "Servez avec davantage d'huile."],
                &["huile"]
            ),
            [step(&[0]), step(&[])]
        );
    }

    #[test]
    fn mince_opening_what_the_cook_does_is_the_verb() {
        let lines = &["500 g ground beef", "1 onion"];
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Fry the mince 5 minutes."], lines),
            [step(&[0])]
        );
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Mince garlic, then add the onion."], lines),
            [step(&[1])]
        );
        assert_eq!(
            uses_as_read(NO_DISH, "en", &["Brown the onion and mince."], lines),
            [step(&[0, 1])]
        );
    }

    /// The Sample Library's Beef bourguignon, as `sample-library/story.json`
    /// writes its two-day rework: the recipe #704 was filed from.
    const BOURGUIGNON_LINES: &[&str] = &[
        "1.2 kg beef (half sucadelappen, half runderriblappen)",
        "200 g sliced speck",
        "400 g mushrooms",
        "400 g leeks",
        "500 g carrot and onion mix",
        "300 g sliced onions",
        "1 bottle red wine",
        "sage",
        "thyme",
        "pepper",
        "salt",
        "3 beef stock cubes",
        "#Optional seasoning adjustments",
        "chilli powder",
        "fish sauce",
        "soy sauce",
        "miso",
    ];
    const BOURGUIGNON_STEPS: &[&str] = &[
        "Cut the meat into cubes of about 5 cm.",
        "Brown the meat in a very hot pan until it is well coloured outside but not cooked through. Deglaze the pan with a sweet wine or brandy.",
        "Cook the speck and onions in the beef fat. Lift them out and keep the fat in the pan.",
        "Brown the rest of the vegetables, mushrooms and leeks included, in that fat. A little salt helps draw the water out.",
        "Add the wine, the herbs, salt, pepper and the stock cubes, then put the meat, speck and onions back.",
        "Make sure the liquid covers everything, topping up with boiled water if needed. Simmer with the lid on for 45 minutes to an hour.",
        "Taste and adjust: fish sauce, soy sauce, pepper, salt or miso, and chilli powder now if you want it.",
        "Simmer another hour or more, two to three hours in all on the first day.",
        "On the second day, simmer for a couple more hours. If the meat is still chewy, let it rest and give it another hour or two later.",
    ];
    /// What each Step of it adds, in English and in its French translation.
    const BOURGUIGNON_USES: &[&[usize]] = &[
        // *The meat* is the beef, and the beef stock cubes are no meat.
        &[0],
        // A sweet wine is no red wine.
        &[0],
        // The beef fat is no beef, and the onions are the sliced ones, not
        // the carrot and onion mix.
        &[1, 5],
        &[2, 3, 10],
        // *The herbs* are the sage and the thyme, and what is put back went
        // in before.
        &[6, 7, 8, 9, 10, 11],
        &[],
        &[9, 10, 13, 14, 15, 16],
        &[],
        // *If the meat is still chewy* adds no meat.
        &[],
    ];

    #[test]
    fn the_bourguignon_lists_each_line_at_the_step_it_goes_in() {
        let want: Vec<_> = BOURGUIGNON_USES.iter().map(|uses| step(uses)).collect();
        assert_eq!(
            uses_as_read(
                "Beef bourguignon",
                "en",
                BOURGUIGNON_STEPS,
                BOURGUIGNON_LINES
            ),
            want
        );
    }

    #[test]
    fn the_french_bourguignon_lists_the_same_lines() {
        let lines = &[
            "1,2 kg de bœuf (moitié sucadelappen, moitié runderriblappen)",
            "200 g de speck en tranches",
            "400 g de champignons",
            "400 g de poireaux",
            "500 g de mélange carottes et oignons",
            "300 g d'oignons émincés",
            "1 bouteille de vin rouge",
            "sauge",
            "thym",
            "poivre",
            "sel",
            "3 cubes de bouillon de bœuf",
            "#Ajustements facultatifs",
            "piment en poudre",
            "sauce de poisson",
            "sauce soja",
            "miso",
        ];
        let steps = &[
            "Coupez la viande en cubes d'environ 5 cm.",
            "Saisissez la viande dans une poêle très chaude jusqu'à ce qu'elle soit bien colorée sans être cuite à cœur. Déglacez avec un vin doux ou du cognac.",
            "Faites revenir le speck et les oignons dans la graisse du bœuf. Retirez-les en gardant la graisse.",
            "Faites dorer le reste des légumes, champignons et poireaux compris, dans cette graisse. Un peu de sel les aide à rendre leur eau.",
            "Ajoutez le vin, les herbes, le sel, le poivre et les cubes de bouillon, puis remettez la viande, le speck et les oignons.",
            "Le liquide doit tout couvrir : complétez avec de l'eau bouillante si besoin. Laissez mijoter à couvert 45 minutes à une heure.",
            "Goûtez et rectifiez : sauce de poisson, sauce soja, poivre, sel ou miso, et le piment maintenant si vous en voulez.",
            "Laissez mijoter encore une heure ou plus, deux à trois heures en tout le premier jour.",
            "Le deuxième jour, laissez mijoter encore deux heures. Si la viande résiste, laissez-la reposer et reprenez la cuisson une heure ou deux plus tard.",
        ];
        let want: Vec<_> = BOURGUIGNON_USES.iter().map(|uses| step(uses)).collect();
        // *Un peu de sel* uses a little of the salt, so *le sel* of the next
        // Step still adds it.
        assert_eq!(uses_as_read("Bœuf bourguignon", "fr", steps, lines), want);
    }

    #[test]
    fn another_kind_of_the_food_is_another_food() {
        let lines = &["red wine", "sliced onions"];
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Deglaze with a sweet wine.",
                    "Add the red wine.",
                    "Add the wine and the chopped onions.",
                ],
                lines
            ),
            [step(&[]), step(&[0]), step(&[0, 1])],
            "a cut is no kind, so *the chopped onions* are the sliced ones"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &["Pour in the hot stock."],
                &["low-sodium chicken stock"]
            ),
            [step(&[0])],
            "a warmth is no kind either"
        );
        assert_eq!(
            uses_in(Some("fr"), &["Déglacez avec un vin doux."], &["vin rouge"]),
            [step(&[])]
        );
    }

    #[test]
    fn what_a_food_leaves_in_the_pan_is_no_part_of_it() {
        assert_eq!(
            uses_in(
                Some("en"),
                &["Cook the onions in the bacon fat."],
                &["bacon", "onions"]
            ),
            [step(&[1])]
        );
    }

    #[test]
    fn the_meat_and_the_herbs_name_the_lines_they_stand_for() {
        let lines = &["chuck", "beef stock cubes", "thyme", "bay leaves", "salt"];
        assert_eq!(
            uses_in(
                Some("en"),
                &["Cut the meat into cubes.", "Add the herbs and the salt.",],
                lines
            ),
            [step(&[0]), step(&[2, 3, 4])],
            "a stock cube holds beef, but beef is no main word of it"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &["Add the herbs."],
                &["1 tsp ground coriander", "parsley"]
            ),
            [step(&[1])],
            "ground coriander is a spice"
        );
    }

    #[test]
    fn a_line_an_earlier_step_cooked_is_not_listed_again() {
        let lines = &["noodles", "ground pork", "chicken", "peanuts"];
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Boil the noodles.",
                    "Fry the mince.",
                    "Brown the chicken, then set the chicken aside.",
                    "Combine the noodles with the cooked mince and crushed peanuts. Put the chicken back.",
                ],
                lines
            ),
            [step(&[0]), step(&[1]), step(&[2]), step(&[3])]
        );
    }

    #[test]
    fn a_line_an_earlier_step_only_cut_or_named_in_part_is_listed_again() {
        assert_eq!(
            uses_in(Some("en"), &["Cut the beef.", "Brown the beef."], &["beef"]),
            [step(&[0]), step(&[0])],
            "the beef goes in the pan in the Step that browns it"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &["Chop the onion.", "Fry the chopped onion in the oil."],
                &["onion", "oil"]
            ),
            [step(&[0]), step(&[0, 1])],
            "*the chopped onion* is the onion the Step before only cut"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &["Melt the butter.", "Add the butter to the flour."],
                &["butter", "flour"]
            ),
            [step(&[0]), step(&[0, 1])],
            "melted butter is no finished part"
        );
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Melt half the butter with a little salt.",
                    "Add the rest of the butter and the salt.",
                ],
                &["butter", "salt"]
            ),
            [step(&[0, 1]), step(&[0, 1])],
            "half the butter and a little salt leave the rest of both to add"
        );
    }

    #[test]
    fn only_a_cooking_word_where_a_verb_stands_cooks_its_sentence() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Bring a large pan of salted water to the boil and chop the parsley.",
                    "Cook the spaghetti.",
                    "Toss the spaghetti with the parsley.",
                ],
                &["spaghetti", "parsley"]
            ),
            [step(&[1]), step(&[0]), step(&[1])],
            "*to the boil* cooks nothing, and the spaghetti is cooked"
        );
        assert_eq!(
            uses_in(
                Some("nl"),
                &["Snijd het fruit.", "Leg het fruit op de taart."],
                &["fruit"]
            ),
            [step(&[0]), step(&[0])],
            "*het fruit* is the noun, not *fruit* as in fry"
        );
    }

    #[test]
    fn back_reaches_only_the_thing_the_cook_does_with_it() {
        assert_eq!(
            uses_in(
                Some("en"),
                &[
                    "Brown the beef with the salt.",
                    "Add salt, then put the beef back.",
                ],
                &["beef", "salt"]
            ),
            [step(&[0, 1]), step(&[1])]
        );
    }
}
