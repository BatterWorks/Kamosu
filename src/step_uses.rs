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
//!   garlic (#294).
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

/// One word of folded text, whether it sits in an *until* or a *when* clause,
/// and the clause it sits in.
struct Word {
    text: String,
    not_going_in: bool,
    clause: usize,
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
        if until_in.is_none() && lists.step_until.has(text) {
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
}

/// A run of a name's words that may stand for the line: whether it is the
/// whole name, and whether a word of the name it leaves out makes the line
/// something [`reading::made_from`] the food the run names (#439).
struct Run {
    words: Vec<String>,
    whole: bool,
    made_from: bool,
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
                let mut left_out = words[..from].iter().chain(&words[to..]);
                runs.push(Run {
                    words: run.to_vec(),
                    whole,
                    made_from: left_out.any(|word| reading::made_from(lists, word)),
                });
            }
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
}

impl Mention {
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
fn mentions_of(lists: &Words, line: &Line, step: &[Word]) -> Vec<Mention> {
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
            if there
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
                let target = reading.get("target")?.as_str()?;
                let (words, runs) = runs_of(lists, target, language);
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
                    one_food: name_words(lists, target).1.len() == 1,
                })
            })
            .collect(),
    };

    let mut used_before: Vec<usize> = Vec::new();
    let mut own_section = None;
    steps
        .iter()
        .map(|row| {
            if row["kind"] != "step" {
                return None;
            }
            let text = row["text"].as_str()?;
            let uses = recipe.uses_of(&words_of(lists, text), &used_before, &mut own_section);
            used_before.extend(&uses);
            Some(uses)
        })
        .collect()
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
        let a_stock = |word: &Word| word.clause == clause && lists.step_stocks.has(&word.text);
        let after = || {
            let next = &step[mention.start + mention.len..];
            next.first().is_some_and(a_stock)
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
                .any(|word| made_here(word) || a_stock(word))
        };
        match self.language {
            Some("en") => after(),
            Some("fr" | "es") => before(),
            _ => after() || before(),
        }
    }

    /// The lines one Step uses, given the lines earlier Steps already used
    /// and the section the Step before worked in, which this updates to the
    /// section this Step works in.
    fn uses_of(
        &self,
        step: &[Word],
        used_before: &[usize],
        own_section: &mut Option<usize>,
    ) -> Vec<usize> {
        let lists = self.lists;
        let sections_named = self.sections_named(step);
        let mentions: Vec<Mention> = self
            .lines
            .iter()
            .flat_map(|line| mentions_of(lists, line, step))
            .filter(|mention| !step[mention.start].not_going_in)
            .filter(|mention| !self.is_for(step, mention))
            .filter(|mention| !self.describes_what_is_made(step, mention))
            .filter(|mention| {
                !sections_named.iter().any(|(_, words)| {
                    words.start < mention.start + mention.len && mention.start < words.end
                })
            })
            .collect();
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
            .filter(|(mention, by_the_dish)| !(*by_the_dish && mention.made_from))
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
        }

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
            if let [only] = leading[..] {
                *own_section = Some(only);
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
        uses
    }
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
                    "Cocina hasta que esté dorado luego agrega el ajo.",
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
            [step(&[2]), step(&[4]), step(&[7]), step(&[8]), step(&[8])],
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
                    "Brown the chicken.",
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
                step(&[10]),
                step(&[]),
                step(&[17]),
                step(&[18, 19]),
                step(&[20]),
                step(&[16]),
                step(&[10, 21]),
                step(&[]),
                step(&[24, 25, 26, 27, 28, 29]),
                step(&[30, 31, 32]),
                step(&[]),
                step(&[10]),
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
                step(&[1]),
                step(&[7]),
                step(&[8]),
                step(&[9, 10, 11, 12, 13, 14, 15, 16]),
                step(&[]),
                step(&[1]),
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
}
