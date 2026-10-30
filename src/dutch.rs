//! **Dutch grammar**: what the Dutch word file cannot say (ADR 0057, #495).
//!
//! Dutch writes a food as one long word whose main word is at its end, so
//! *tarwebloem* is a kind of *bloem*, and it joins the two with `-en-` or `-s-`,
//! so *kippenbouillon* is a *bouillon*. It makes a plural in `-en`, doubling or
//! undoubling a letter on the way: *tomaat*, *tomaten*; *kip*, *kippen*. And it
//! makes a small one of anything: *uitje*, *teentje*, *balletje*.
//!
//! Everything here compares **stems**, which are no words of Dutch and are never
//! shown. A stem answers one question: do these two written words name one
//! thing? *Tomaat* and *tomaten* both stem to `tomat`. It is the smallest rule
//! that does that, and it errs towards keeping two words apart.

use crate::core::folded_for_search;
use crate::words::Words;

fn is_vowel(c: char) -> bool {
    "aeiouy".contains(c)
}

/// The word with its plural ending off, if it has one Dutch would write there.
///
/// `-s` is the plural after a vowel or an unstressed `-e`, `-el`, `-en`, `-em`
/// or `-er`, *paprika's*, *appels*. A word that doubles its vowel before the
/// `s` is no plural: *kaas* stays. `-en` is any other, and takes what stands
/// before it for the word, however little: *uien* is *ui*.
fn without_plural(word: &str) -> String {
    let characters: Vec<char> = word.chars().collect();
    let n = characters.len();
    if n >= 4 && word.ends_with('s') {
        let before = characters[n - 2];
        let doubled = characters[n - 3] == before;
        let unstressed = n >= 3 && characters[n - 3] == 'e' && "lnmr".contains(before);
        if (is_vowel(before) && !doubled) || unstressed {
            return characters[..n - 1].iter().collect();
        }
    }
    // `-een` is a long vowel, *teen*, and no plural.
    if n >= 4 && word.ends_with("en") && characters[n - 3] != 'e' {
        return characters[..n - 2].iter().collect();
    }
    word.to_string()
}

/// The word with its small ending off, or nothing where it has none.
fn without_diminutive(word: &str) -> Option<String> {
    let word = word.strip_suffix('s').unwrap_or(word);
    let long_vowel = |stem: &str| {
        ["aa", "ee", "oo", "uu"]
            .iter()
            .any(|vowel| stem.ends_with(vowel))
    };
    if let Some(stem) = word.strip_suffix("etje").filter(|stem| stem.len() >= 2) {
        return Some(stem.to_string());
    }
    if let Some(stem) = word.strip_suffix("pje").filter(|stem| stem.ends_with('m')) {
        return Some(stem.to_string());
    }
    if let Some(stem) = word.strip_suffix("tje").filter(|stem| stem.len() >= 2) {
        // A long vowel keeps the `t` it had: *tomaatje* is *tomaat*, and
        // *uitje* is *ui*.
        return Some(if long_vowel(stem) {
            format!("{stem}t")
        } else {
            stem.to_string()
        });
    }
    word.strip_suffix("je")
        .filter(|stem| stem.len() >= 2)
        .map(str::to_string)
}

/// The end of a word as its plural and its singular agree on it: a doubled
/// consonant single and a doubled vowel single before the last consonant, `v`
/// as `f` and `z` as `s` at the end, and no final `e`.
fn settled(word: &str) -> String {
    let mut characters: Vec<char> = word.chars().collect();
    if characters.len() > 2 && characters.last() == Some(&'e') {
        characters.pop();
    }
    let n = characters.len();
    if n >= 2 && characters[n - 1] == characters[n - 2] && !is_vowel(characters[n - 1]) {
        characters.pop();
    }
    let n = characters.len();
    if n >= 3 && characters[n - 2] == characters[n - 3] && is_vowel(characters[n - 2]) {
        characters.remove(n - 2);
    }
    match characters.last_mut() {
        Some(last @ 'v') => *last = 'f',
        Some(last @ 'z') => *last = 's',
        _ => {}
    }
    characters.into_iter().collect()
}

/// **What two written forms of one Dutch word share**, plural and diminutive
/// off, from a word already folded for search.
pub fn stem(lists: &Words, word: &str) -> String {
    let word = word.replace(['\'', '\u{2019}'], "");
    if let Some(stem) = lists.irregular.get(&word) {
        return settled(stem);
    }
    match without_diminutive(&word) {
        Some(stem) => settled(&stem),
        None => settled(&without_plural(&word)),
    }
}

/// What a written Unit is known by, so `1 blik` and `2 blikken` add: its plural
/// off, and nothing else, since a *blikje* is no *blik*.
pub fn unit_stem(word: &str) -> String {
    settled(&without_plural(word))
}

/// **What a Dutch Food's name is matched on**: the name folded, its last word,
/// the main one, stemmed. So *2 grote uien* and *1 ui* are one Food, as a Food
/// Match in any other Language would need a second name to make them.
pub fn food_key(lists: &Words, folded_name: &str) -> String {
    match folded_name.rsplit_once(' ') {
        Some((first, last)) => format!("{first} {}", stem(lists, &folded_for_search(last))),
        None => stem(lists, &folded_for_search(folded_name)),
    }
}

/// Whether a Step's `word` is the main word of the line's compound `whole`:
/// *bloem* in *tarwebloem*, *bouillon* in *kippenbouillon*. Both are folded.
/// `-en-` and `-s-` need no rule, since whatever joins the two parts stands
/// before the main word.
pub fn is_main_word_of(lists: &Words, word: &str, whole: &str) -> bool {
    if lists.not_compounds.has(whole) {
        return false;
    }
    let wanted = stem(lists, word);
    let length = without_plural(word).chars().count();
    let long_enough = wanted.chars().count() >= 3 || lists.short_foods.has(&wanted);
    if word.chars().count() < 3 || !long_enough {
        return false;
    }
    // The end of the compound has to be the word, plural or not: as a stem,
    // `keer` would be found in *suiker*.
    whole
        .char_indices()
        .skip(2)
        .map(|(at, _)| &whole[at..])
        .any(|tail| tail.chars().count() >= length && stem(lists, tail) == wanted)
}

/// Whether a Step's `word` is what the line's compound `whole` is a part of:
/// *kip* in *kipfilet*, *laurier* in *laurierblaadjes*, *spek* in
/// *spekblokjes*. The part is a word of [`Words::cuts_and_parts`], which gives
/// way here as it does between two written words, and *kippendijfilet* gives
/// way twice.
pub fn is_cut_of(lists: &Words, word: &str, whole: &str) -> bool {
    let word = stem(lists, word);
    let mut rest = whole;
    while let Some((at, _)) = rest
        .char_indices()
        .skip(2)
        .find(|(at, _)| lists.cuts_and_parts.has(&rest[*at..]))
    {
        rest = &rest[..at];
        let joined = rest.strip_suffix('s').unwrap_or(rest);
        if stem(lists, rest) == word || stem(lists, joined) == word {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words;

    fn same(a: &str, b: &str) -> bool {
        let lists = words::of("nl");
        stem(lists, a) == stem(lists, b)
    }

    #[test]
    fn a_plural_and_a_small_one_are_the_one_word() {
        for (one, other) in [
            ("ui", "uien"),
            ("ui", "uitje"),
            ("ui", "uitjes"),
            ("ei", "eieren"),
            ("ei", "eitje"),
            ("tomaat", "tomaten"),
            ("tomaat", "tomaatje"),
            ("tomaat", "tomaatjes"),
            ("kip", "kippen"),
            ("kip", "kippetje"),
            ("boon", "bonen"),
            ("kaas", "kazen"),
            ("druif", "druiven"),
            ("appel", "appels"),
            ("aardappel", "aardappelen"),
            ("aardappels", "aardappelen"),
            ("wortel", "worteltjes"),
            ("groente", "groenten"),
            ("paprika", "paprikas"),
            ("teen", "tenen"),
            ("teen", "teentjes"),
            ("blad", "blaadjes"),
            ("blad", "bladeren"),
            ("bal", "balletjes"),
            ("takje", "takjes"),
            ("radijs", "radijsjes"),
        ] {
            assert!(same(one, other), "{one} and {other}");
        }
    }

    #[test]
    fn words_that_only_look_alike_stay_apart() {
        for (one, other) in [
            ("peer", "peper"),
            ("ui", "uur"),
            ("ei", "prei"),
            ("melk", "melk chocolade"),
            ("bloem", "bloemkool"),
        ] {
            assert!(!same(one, other), "{one} and {other}");
        }
    }

    #[test]
    fn a_compound_is_named_by_its_main_word() {
        let lists = words::of("nl");
        for (word, whole) in [
            ("bloem", "tarwebloem"),
            ("bouillon", "kippenbouillon"),
            ("bouillon", "groentebouillon"),
            ("olie", "olijfolie"),
            ("worst", "rookworst"),
            ("uien", "zilveruien"),
            ("bonen", "sperziebonen"),
        ] {
            assert!(is_main_word_of(lists, word, whole), "{word} in {whole}");
        }
        // The first part of a compound is no main word, and a potato is no
        // apple.
        for (word, whole) in [
            ("tarwe", "tarwebloem"),
            ("appel", "aardappel"),
            ("appels", "aardappels"),
            ("bloem", "bloem"),
            ("ei", "prei"),
            ("keer", "suiker"),
        ] {
            assert!(!is_main_word_of(lists, word, whole), "{word} in {whole}");
        }
    }

    #[test]
    fn a_part_of_a_food_gives_way_to_the_food() {
        let lists = words::of("nl");
        for (word, whole) in [
            ("kip", "kipfilet"),
            ("kip", "kippendijfilet"),
            ("kip", "kipdijen"),
            ("laurier", "laurierblaadjes"),
            ("spek", "spekblokjes"),
            ("spek", "spekreepjes"),
            ("knoflook", "knoflooktenen"),
            ("zalm", "zalmmoten"),
            ("komijn", "komijnzaad"),
        ] {
            assert!(is_cut_of(lists, word, whole), "{word} in {whole}");
        }
        assert!(!is_cut_of(lists, "citroen", "citroensap"));
        assert!(!is_cut_of(lists, "kip", "varkenshaas"));
    }

    #[test]
    fn a_food_is_matched_on_its_main_word_stemmed() {
        let lists = words::of("nl");
        assert_eq!(food_key(lists, "uien"), food_key(lists, "ui"));
        assert_eq!(food_key(lists, "rode uien"), food_key(lists, "rode ui"));
        assert_ne!(food_key(lists, "rode ui"), food_key(lists, "ui"));
    }
}
