//! Units, scaling and conversion — ADR 0016, #49.
//!
//! **The set of Units is open; the set Kamosu can convert is closed.** Whatever
//! the cook wrote is a Unit and is kept exactly as written: *poignée*, *noix*,
//! *sachet* and *bottle* are all perfectly legitimate, and simply never offer a
//! conversion. Against that open set sits [`UNITS`], a closed list Kamosu knows
//! well enough to do arithmetic on, whose spellings each Language keeps in its
//! word file ([`crate::words`]) — so `g`, `gr`, `gramme`, `grammes` and `grams`
//! are one Unit and spelling is not a data-entry problem.
//!
//! **A conversion is never a rewrite.** Nothing here touches an Ingredient Line
//! (ADR 0002). What it produces is one subordinate line — *how much, for you,
//! right now* — which is absent whenever it would only repeat the line
//! unchanged.
//!
//! **Scaling and conversion are one act, so they get one slot.** Both answer
//! the same question, and given one line they never have to negotiate:
//! scaled-not-converted, converted-not-scaled and both look identical.
//!
//! ## The two arithmetic decisions worth knowing
//!
//! **Round last, once.** Scale, then convert, then round the number actually
//! shown. Rounding twice turns three cups into 720 ml when it is 710.
//!
//! **The cup is 236.588 ml, not 240.** ADR 0016 states both figures and they
//! agree: 236.588 is the arithmetic, 240 is what one cup *displays* as once the
//! ladder in [`round_to`] has run — and the ADR's own worked example ("three
//! cups is 710, not 720") only holds on 236.588. The same fall-out gives a
//! tablespoon 15 ml and a teaspoon 5 ml, exactly as the ADR's consequences say.
//! A tablespoon is a sixteenth of a cup and a teaspoon a third of that, so one
//! Cup Weight yields every volume measure of a Food by arithmetic.
//!
//! ## What is deliberately not here
//!
//! No test seam. #49 requires all of this exercised through Operations, so the
//! tables below carry no unit tests of their own — `tests/behaviour.rs` drives
//! them through `get_recipe` against a real database, which is the only way to
//! prove both Doors got it.

#[cfg(test)]
use crate::language::UNKNOWN;
use crate::words::{self, Words};

/// Which arithmetic a Unit takes part in. Cross-family conversion — a volume of
/// a dry good into grams — is the one crossing, and it runs off a Cup Weight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    /// Base unit: the gram.
    Mass,
    /// Base unit: the millilitre.
    Volume,
}

/// Where a Unit comes from, which decides who needs it converted. A metric
/// reader needs `2 cups` turned into grams; she needs nothing done to `250 ml`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    Metric,
    /// American, imperial and the explicitly-named regional variants — every
    /// Unit a metric kitchen has no equipment for.
    Customary,
}

/// One member of the closed convertible set: what it is and how big it is. How
/// each Language spells it is in the word files ([`crate::words`]).
pub struct Unit {
    /// Stable internal name. Never shown; [`display`] renders what is.
    pub id: &'static str,
    pub family: Family,
    pub system: System,
    /// How many grams (Mass) or millilitres (Volume) one of these is.
    pub base: f64,
}

/// The closed set. ADR 0016 names it: gram, kilogram, millilitre, litre,
/// teaspoon, tablespoon, cup, fluid ounce, ounce, pound, and a few explicitly
/// named regional variants.
///
/// Centilitre and decilitre are here for one measured reason: French recipes
/// write `20 cl` where English ones write `200 ml`, and Kamosu's corpus is
/// French-heavy. They are ordinary metric members, not variants. The US quart
/// is here for the same kind of reason: `1 USqt Water` was a real line, and
/// four cups is arithmetic Kamosu already does (#182).
///
/// `imperial tablespoon` and `Australian tablespoon` are ordinary members too,
/// at the cost of one line each — a recipe that names one gets it right without
/// anyone being asked anything. The accepted cost ADR 0016 records is the
/// Australian tablespoon nobody *named*: three tablespoons of soy sauce read as
/// 45 ml where the author meant 60.
pub const UNITS: &[Unit] = &[
    // --- Mass -------------------------------------------------------------
    Unit {
        id: "gram",
        family: Family::Mass,
        system: System::Metric,
        base: 1.0,
    },
    Unit {
        id: "kilogram",
        family: Family::Mass,
        system: System::Metric,
        base: 1000.0,
    },
    Unit {
        id: "ounce",
        family: Family::Mass,
        system: System::Customary,
        base: 28.349_523_125,
    },
    Unit {
        id: "pound",
        family: Family::Mass,
        system: System::Customary,
        base: 453.592_37,
    },
    // --- Volume -----------------------------------------------------------
    Unit {
        id: "millilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 1.0,
    },
    Unit {
        id: "centilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 10.0,
    },
    Unit {
        id: "decilitre",
        family: Family::Volume,
        system: System::Metric,
        base: 100.0,
    },
    Unit {
        id: "litre",
        family: Family::Volume,
        system: System::Metric,
        base: 1000.0,
    },
    Unit {
        id: "teaspoon",
        family: Family::Volume,
        system: System::Customary,
        // A third of a tablespoon, which is a sixteenth of a cup.
        base: 236.588_236_5 / 48.0,
    },
    Unit {
        id: "tablespoon",
        family: Family::Volume,
        system: System::Customary,
        // A sixteenth of a cup — which is what makes one Cup Weight enough to
        // answer every volume measure of a Food (ADR 0016).
        base: 236.588_236_5 / 16.0,
    },
    Unit {
        id: "cup",
        family: Family::Volume,
        system: System::Customary,
        base: 236.588_236_5,
    },
    // The US liquid quart, four cups (#182). The bare word `quart` is left
    // out on purpose: in French it is a quarter, and a Step reading *laisser
    // reposer 1 quart d'heure* would be offered a conversion into litres.
    Unit {
        id: "quart",
        family: Family::Volume,
        system: System::Customary,
        base: 236.588_236_5 * 4.0,
    },
    Unit {
        id: "fluid ounce",
        family: Family::Volume,
        system: System::Customary,
        base: 29.573_529_562_5,
    },
    Unit {
        id: "imperial tablespoon",
        family: Family::Volume,
        system: System::Customary,
        base: 17.758_164_0,
    },
    Unit {
        id: "australian tablespoon",
        family: Family::Volume,
        system: System::Customary,
        base: 20.0,
    },
];

/// How a person measures, held on their account beside their Reading Language.
/// The default is American, a stated convention rather than a guess about
/// anybody (ADR 0016).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measures {
    Us,
    Metric,
    /// Leave every Unit alone. Scaling still happens: a recipe at double Yield
    /// has a different number on it whatever system it is written in.
    AsWritten,
}

impl Measures {
    /// Read the stored column. Anything unrecognised reads as the stated
    /// default rather than failing a recipe — nothing downstream of a
    /// preference should ever be able to stop a line rendering.
    pub fn from_stored(value: &str) -> Self {
        match value {
            "metric" => Measures::Metric,
            "as_written" => Measures::AsWritten,
            _ => Measures::Us,
        }
    }
}

/// Fold a written word for comparison against the tables below: the same
/// Unicode fold Kamosu uses on every other word (NFD, then the default case
/// fold, then NFD again — see `core::folded_word`), and additionally forgiving
/// about the punctuation and spacing a unit abbreviation carries, so `c. à c.`
/// and `càc` are one spelling.
///
/// The Unicode half is load-bearing rather than decorative. An *é* typed as one
/// character and an *e* followed by a combining accent look identical on screen
/// and are different bytes, and a French cookbook meets both depending on the
/// keyboard — so a table holding `maïzena` or `pépites de chocolat` and folding
/// only by `to_lowercase` would silently miss half the Foods it names. **Both
/// sides go through this**: the tables are folded at lookup time too, so a
/// literal written here can never be a spelling nothing can reach.
pub fn fold(word: &str) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    let cased: String = word.chars().nfd().default_case_fold().nfd().collect();
    let mut folded = String::with_capacity(cased.len());
    let mut wants_space = false;
    for character in cased.chars() {
        if character.is_whitespace() || character == '.' {
            // A separator only matters between two kept characters, so a
            // trailing one can never survive: `c. à c.` folds to `c à c`.
            wants_space = !folded.is_empty();
            continue;
        }
        if wants_space {
            folded.push(' ');
            wants_space = false;
        }
        folded.push(character);
    }
    folded
}

impl Unit {
    /// Whether this is a spoon — teaspoon, tablespoon and the regional
    /// tablespoons. Every kitchen has spoons, so a spoon says nothing about
    /// which system a recipe was written in ([`written_in`]).
    pub fn is_spoon(&self) -> bool {
        matches!(
            self.id,
            "teaspoon" | "tablespoon" | "imperial tablespoon" | "australian tablespoon"
        )
    }
}

/// The Unit a written word names in a line of no known Language, or nothing.
/// Everything that knows the line's Language asks [`crate::words::of`].
pub fn recognise(word: &str) -> Option<&'static Unit> {
    words::unknown().unit(word)
}

/// **The one Unit whose case matters** (#208). American recipes write a
/// teaspoon as a lone `t` and a tablespoon as a lone `T`, with or without a
/// full stop, and [`fold`] would make the two one word. So this is asked
/// before anything is folded, and every other spelling stays blind to case:
/// `Tbsp` and `TBSP` are the same tablespoon.
///
/// [`Words::unit`] asks it first, so reading a line and working out what a
/// stored `T` is worth cannot answer differently.
///
/// The answer is the letter with its full stop gone, `t` or `T`. Two callers
/// need more than the Unit: the reader, because a single letter is a Unit
/// only where an amount stands before it, and the shopping list, because the
/// letter's case has to survive where two Units are compared by spelling.
pub fn lone_spoon(word: &str) -> Option<&'static str> {
    match word.trim().trim_end_matches('.') {
        "t" => Some("t"),
        "T" => Some("T"),
        _ => None,
    }
}

/// Whether a word is a Unit written as one character, `t`, `T`, `c` or `#`:
/// a Unit only where an amount stands before it, since with none a single
/// letter opens a name, `T bone steak` (#208, #326).
pub fn lone_letter(word: &str) -> bool {
    lone_spoon(word).is_some() || lone_cup(word) || fold(word) == "#"
}

/// Whether a word is the lone `c` or `C` of a cup, with or without its full
/// stop (#326). French writes the same letter for a *cuillère*, so the reader
/// asks what follows it, and a Step does not read it at all: `180 C` there is
/// an oven.
pub fn lone_cup(word: &str) -> bool {
    fold(word) == "c"
}

// --- Reading a quantity ------------------------------------------------------

/// The vulgar fractions a recipe actually contains, and what each is worth.
/// Both directions use this table: reading `4½` off a line, and writing `4½`
/// back out for a reader whose cups are marked that way.
const FRACTIONS: &[(char, f64)] = &[
    ('½', 0.5),
    ('⅓', 1.0 / 3.0),
    ('⅔', 2.0 / 3.0),
    ('¼', 0.25),
    ('¾', 0.75),
    ('⅕', 0.2),
    ('⅖', 0.4),
    ('⅗', 0.6),
    ('⅘', 0.8),
    ('⅙', 1.0 / 6.0),
    ('⅚', 5.0 / 6.0),
    ('⅛', 0.125),
    ('⅜', 0.375),
    ('⅝', 0.625),
    ('⅞', 0.875),
];

/// How much a written amount is worth, or nothing where Kamosu cannot read it.
///
/// Nothing is the ordinary answer for a great deal of real writing — `a pinch`,
/// `2-3`, `to taste` — and it is not a failure. A quantity Kamosu could not
/// read is shown whole rather than guessed at (ADR 0002), which here means no
/// subordinate line at all.
///
/// It reads `2`, `2.5`, `2,5` (the decimal comma), `1/2`, `1 1/2`, `½`
/// and `4½`.
pub fn parse_amount(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // A trailing vulgar fraction, with or without a whole number before it.
    if let Some(last) = text.chars().next_back()
        && let Some((_, value)) = FRACTIONS.iter().find(|(glyph, _)| *glyph == last)
    {
        let whole = text[..text.len() - last.len_utf8()].trim();
        if whole.is_empty() {
            return Some(*value);
        }
        return parse_plain(whole).map(|number| number + value);
    }

    // `1 1/2` — a whole number and a written fraction.
    if let Some((whole, rest)) = text.split_once(' ')
        && let Some(number) = parse_plain(whole)
        && let Some(fraction) = parse_ratio(rest.trim())
    {
        return Some(number + fraction);
    }

    parse_ratio(text).or_else(|| parse_plain(text))
}

/// `1/2`, and nothing else that contains a slash.
fn parse_ratio(text: &str) -> Option<f64> {
    let (top, bottom) = text.split_once('/')?;
    let top = parse_plain(top.trim())?;
    let bottom = parse_plain(bottom.trim())?;
    (bottom != 0.0).then_some(top / bottom)
}

/// A plain number, accepting the French decimal comma.
fn parse_plain(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

// --- Cup Weight --------------------------------------------------------------

/// The shipped Cup Weight for a Food, found by any of its names: what one cup of
/// a staple weighs, about, from the word files. Nothing is the ordinary answer
/// and is not a gap: a Food with no Cup Weight is a line that offers millilitres
/// instead of grams.
///
/// ADR 0016 costed this precisely: of the 118 real cup rows in the corpus, 42
/// are liquids (pure arithmetic to millilitres) and 58 are baking staples, so
/// roughly twenty hand-checked entries plus plain arithmetic covers 100 of 118.
/// The remaining 18 — sliced mushrooms, spinach, blueberries — are where a cup
/// is a loose handful and a gram figure would be false precision. They are not
/// a gap to close; they are lines that offer millilitres instead.
///
/// A figure stored on the Food always beats the figure here (ADR 0016: "an
/// override always beats the shipped figure"), and #47 already owns that
/// column. The list is curated, checked against USDA SR Legacy's
/// `food_portion.csv`, not an ingest of it. No milk, water, cream or yoghurt:
/// giving a liquid a Cup Weight would turn `2 cups milk` into grams, which is
/// not how anybody measures milk. No salt, vanilla, baking powder or spice mix
/// either: nobody wants those in grams.
///
/// The match is exact on the fold, exactly as a Food Match is (GLOSSARY.md) —
/// `farine T55` is its own Food and gets silence rather than a guess at which
/// flour somebody meant. That silence is the designed failure mode, not a bug.
pub fn shipped_cup_weight<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<f64> {
    names
        .into_iter()
        .find_map(|name| words::CUP_WEIGHTS.get(&fold(name)).copied())
}

// --- How much of a Component is wanted ---------------------------------------

/// **How much of an inner recipe a Component's line asks for**, worked out from
/// the Reading over the line and the inner recipe's Yield — and stored nowhere
/// (ADR 0008). `1.0` is the whole recipe.
///
/// `None` is not a failure. It is the ordinary answer whenever the two cannot
/// be compared — `2 poignées de pâte` against a Yield in kilograms, or an inner
/// recipe with no Yield at all — and it means the inner recipe is shown as
/// written, with Kamosu saying it could not work it out. Refusing to guess here
/// is the same refusal [`measured_line`] makes one layer up.
///
/// It lives in this module rather than beside the unfolding because the
/// question is a Units question: *is this quantity commensurable with that
/// one, and by what factor* — the same closed set of Units, the same base
/// values, the same silence outside them (ADR 0016, ADR 0036).
#[cfg(test)]
pub fn how_much_of(
    amount: Option<&str>,
    unit: Option<&str>,
    yield_amount: Option<&str>,
    yield_noun: Option<&str>,
) -> Option<f64> {
    how_much_of_in(UNKNOWN, UNKNOWN, amount, unit, yield_amount, yield_noun)
}

/// [`how_much_of`], for a line written in `line_language` measured against a
/// Yield written in `yield_language`.
pub fn how_much_of_in(
    line_language: &str,
    yield_language: &str,
    amount: Option<&str>,
    unit: Option<&str>,
    yield_amount: Option<&str>,
    yield_noun: Option<&str>,
) -> Option<f64> {
    // No quantity on the line means the whole recipe. 28% of real Ingredient
    // rows carry none, so this is the common case rather than the edge one.
    let Some(amount) = amount else {
        return Some(1.0);
    };
    let wanted = parse_amount(amount)?;
    if wanted <= 0.0 {
        return None;
    }
    // An inner recipe with no Yield gives nothing to divide by.
    let made = parse_amount(yield_amount?)?;
    if made <= 0.0 {
        return None;
    }
    let noun = yield_noun?.trim();

    // Mass against mass, volume against volume — through the same base values
    // everything else here converts on, so `500 g` of a dough yielding `1 kg`
    // is half of it and `3 tbsp` of an oil yielding `250 ml` is about a fifth.
    if let Some(from) = unit.and_then(|unit| words::of(line_language).unit(unit))
        && let Some(to) = words::of(yield_language).unit(noun)
        && from.family == to.family
    {
        return Some((wanted * from.base) / (made * to.base));
    }

    // **The Yield's own noun, used as a Unit**: `12 cookies' worth` of a recipe
    // that yields `24 cookies`. A line whose quantity carries no Unit at all
    // takes the Yield's noun, because a bare number beside a recipe can mean
    // nothing else.
    match unit {
        None => Some(wanted / made),
        Some(written) if same_noun(written, noun) => Some(wanted / made),
        // A Unit Kamosu knows, measured against a noun it does not — `500 g`
        // of a dough that yields `4 servings`. There is no honest factor here
        // and inventing one would silently mis-scale a whole recipe.
        Some(_) => None,
    }
}

/// Whether a Reading's Unit word and a Yield's noun are the same noun.
/// `servings` and `serving` are one word; so are `pizzas` and `pizza`. Folded
/// first, so capitals and punctuation are already gone.
fn same_noun(written: &str, noun: &str) -> bool {
    let trim = |word: &str| fold(word).trim_end_matches('s').to_string();
    !noun.is_empty() && trim(written) == trim(noun)
}

/// **The one line beneath a Component's written line**: which Recipe it names,
/// and how much of it — or the one sentence saying why there is no unfolding.
///
/// **Worded here rather than by either screen**, for the reason [`measured_line`]
/// is: the recipe page is rendered twice, once as a Svelte screen and once as a
/// Rust Share Link page, and an agent reads the same recipe at the MCP door. One
/// gradient rendered twice is the drift `assets/app.css` exists to prevent; one
/// sentence worded three times would be the same mistake in prose. So the Core
/// says it once and every reading of it prints what it said.
///
/// The Language is the reader's on a screen and the recipe's on a Share Link,
/// which is the same rule the rest of that page follows: a stranger arriving
/// from a link has told Kamosu nothing, and the one thing known about what they
/// are reading is what it is written in.
///
/// **What belongs here is prose computed from data, not every label.** The
/// heading over a Component's Steps is a page label like *Ingredients* and
/// *Method*, and those already live twice on purpose — once in
/// `share_page::Words` because that page is Rust, once in Paraglide because the
/// app is TypeScript. A third copy of one of them here would be the drift, not
/// the cure.
///
/// The vulgar fractions are the ones already on the measuring cups in
/// [`FRACTIONS`], reused rather than reinvented — a half is a half whether it is
/// half a cup or half a dough. A ratio with no glyph falls back to a plain
/// decimal, which is honest about being a number nobody would write on a jug.
pub fn component_line(
    held: bool,
    stopped: bool,
    title: Option<&str>,
    share: Option<f64>,
    language: &str,
) -> String {
    let title = title.unwrap_or("");
    match language {
        "fr" => component_words(held, stopped, title, share, FR_COMPONENT, language),
        "es" => component_words(held, stopped, title, share, ES_COMPONENT, language),
        "nl" => component_words(held, stopped, title, share, NL_COMPONENT, language),
        _ => component_words(held, stopped, title, share, EN_COMPONENT, language),
    }
}

/// One Language's four sentences: not held, stopped at a repeat, held but with
/// no factor, and held with one. The fourth carries `{title}` and `{share}`.
struct ComponentWords {
    missing: &'static str,
    stopped: &'static str,
    unscaled: &'static str,
    whole: &'static str,
    part: &'static str,
}

const EN_COMPONENT: ComponentWords = ComponentWords {
    missing: "Kamosu does not have this recipe.",
    stopped: "{title} is already open above — Kamosu stops here.",
    unscaled: "{title} — Kamosu could not work out how much, so this is the recipe as written.",
    whole: "{title} · the whole recipe",
    part: "{title} · {share} of the recipe",
};

const FR_COMPONENT: ComponentWords = ComponentWords {
    missing: "Kamosu n'a pas cette recette.",
    stopped: "{title} est déjà ouverte plus haut — Kamosu s'arrête ici.",
    unscaled: "{title} — Kamosu n'a pas pu établir la quantité : voici la recette telle qu'elle est écrite.",
    whole: "{title} · la recette entière",
    part: "{title} · {share} de la recette",
};

const ES_COMPONENT: ComponentWords = ComponentWords {
    missing: "Kamosu no tiene esta receta.",
    stopped: "{title} ya está abierta más arriba — Kamosu se detiene aquí.",
    unscaled: "{title} — Kamosu no pudo calcular la cantidad: esta es la receta tal como está escrita.",
    whole: "{title} · la receta entera",
    part: "{title} · {share} de la receta",
};

const NL_COMPONENT: ComponentWords = ComponentWords {
    missing: "Kamosu heeft dit recept niet.",
    stopped: "{title} staat hierboven al open. Kamosu stopt hier.",
    unscaled: "{title}: Kamosu kon de hoeveelheid niet bepalen, dus dit is het recept zoals het is geschreven.",
    whole: "{title} · het hele recept",
    part: "{title} · {share} van het recept",
};

fn component_words(
    held: bool,
    stopped: bool,
    title: &str,
    share: Option<f64>,
    words: ComponentWords,
    language: &str,
) -> String {
    if !held {
        return words.missing.to_string();
    }
    if stopped {
        return words.stopped.replace("{title}", title);
    }
    match share {
        None => words.unscaled.replace("{title}", title),
        Some(share) if (share - 1.0).abs() < 0.005 => words.whole.replace("{title}", title),
        Some(share) => words
            .part
            .replace("{title}", title)
            .replace("{share}", &share_written(share, language)),
    }
}

// --- The one subordinate line ------------------------------------------------

/// What the subordinate line ends up saying, before it is worded.
struct Measured {
    quantity: f64,
    /// The Unit it is in, where that is one Kamosu knows.
    unit: Option<&'static Unit>,
    /// The cook's own word, kept for a Unit outside the closed set — the
    /// quantity multiplies and the unit is untouched (ADR 0016).
    as_written: Option<String>,
}

/// **The one line beneath the written line**, or nothing.
///
/// Nothing is the right answer far more often than something. It is absent
/// where Kamosu read no quantity, where it could not read the quantity it
/// found, and — the common case — where the line is already in this reader's
/// measures at the Yield they are cooking, so a line would only repeat what is
/// already there.
///
/// `scale` is how far the Yield being cooked is from the Yield as written; 1.0
/// is a recipe read rather than scaled. `cup_weight_grams` is the Food's
/// effective Cup Weight — the override where somebody set one, the shipped
/// figure otherwise — and its absence is what turns a cup of something into
/// millilitres rather than into grams.
#[cfg(test)]
pub fn measured_line(
    amount: Option<&str>,
    unit: Option<&str>,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    measured_line_in(
        UNKNOWN,
        amount,
        unit,
        scale,
        measures,
        language,
        cup_weight_grams,
    )
}

/// [`measured_line`], for a line written in `written_in_language`, whose words
/// say which Unit it names. `language` is the reader's, whose words say the
/// answer.
pub fn measured_line_in(
    written_in_language: &str,
    amount: Option<&str>,
    unit: Option<&str>,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    let figure = figure_in(
        written_in_language,
        amount?,
        unit,
        scale,
        measures,
        cup_weight_grams,
    )?;
    // Absent whenever it would only repeat the line unchanged: same Unit, same
    // number.
    if figure.unchanged {
        return None;
    }
    Some(figure.said(language))
}

/// [`measured_line_in`] said even where it is the measure written: for a
/// Step given its line's measure, which its own words do not say (#705).
pub fn measured_said_in(
    written_in_language: &str,
    amount: &str,
    unit: &str,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    figure_in(
        written_in_language,
        amount,
        Some(unit),
        scale,
        measures,
        cup_weight_grams,
    )
    .map(|figure| figure.said(language))
}

/// **An Ingredient Line read whole at another Yield** (#680): the written line
/// with its amount scaled, and its Unit too where the reader's measures
/// converted it, so `1 head of garlic` at half again is `1½ heads of garlic`.
///
/// It stands in for the line wherever a screen leads with the scaled amount,
/// as the cooking screen does, so it has to say what it is an amount of. The
/// short [`measured_line_in`] was written to sit beside the line and says only
/// the figure.
///
/// The word that counts goes plural or singular with the new number in
/// English: the Unit's own word where the line has one (`head`, `tablespoon`),
/// and otherwise the Food's last word (`onion`). A Unit of the closed set
/// changes only where the line spells its name out, so `tbsp` stays `tbsp`.
/// Other Languages keep their words as written.
///
/// Where the amount cannot be found in the line, as after a Reading corrected
/// by hand, the figure is followed by the Food. Nothing is answered where the
/// amount could not be read.
#[allow(clippy::too_many_arguments)]
pub fn scaled_line_in(
    text: &str,
    written_in_language: &str,
    amount: Option<&str>,
    unit: Option<&str>,
    target: Option<&str>,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    let amount = amount?;
    let mut figure = figure_in(
        written_in_language,
        amount,
        unit,
        scale,
        measures,
        cup_weight_grams,
    )?;
    let number = figure.number(language);
    let english = written_in_language == "en";
    let now_plural = plural(figure.rounded_high(), written_in_language);
    let was_plural = plural(figure.written_high, written_in_language);
    let recount = |word: &str| -> String {
        if !english || now_plural == was_plural {
            word.to_string()
        } else if now_plural {
            english_plural(word)
        } else {
            english_singular(word)
        }
    };

    let Some(at) = amount_at(text, amount, unit) else {
        let said = match target {
            Some(target) => format!("{number} {} {target}", figure.unit_word(language)),
            None => format!("{number} {}", figure.unit_word(language)),
        };
        let said = said.split_whitespace().collect::<Vec<_>>().join(" ");
        return Some(figure.about_if_due(said, language));
    };
    let before = &text[..at];
    let mut rest = text[at + amount.len()..].to_string();

    // The Unit, where the line writes it straight after the amount, or a
    // word or two later, `2 (heaped) tbsp flour`.
    let mut unit_said = String::new();
    if let Some(unit) = unit {
        let gap = rest.len() - rest.trim_start().len();
        if rest[gap..].starts_with(unit) {
            unit_said = if figure.converted {
                figure.unit_word(language)
            } else if figure.source.is_none_or(|source| spells_out(unit, source)) {
                // A Unit of the closed set is counted only where the line
                // spells its name out: `tbsp` has no plural, `tablespoon` has.
                recount(unit)
            } else if english
                && !now_plural
                && let Some(one) = unit.strip_suffix('s')
                && let Some(source) = figure.source
                && one.eq_ignore_ascii_case(&display(source, language, false))
            {
                // `1/2 lbs` doubled is `1 lb` (#698).
                one.to_string()
            } else {
                unit.to_string()
            };
            rest.replace_range(..gap + unit.len(), "");
        } else if figure.converted {
            unit_said = figure.unit_word(language);
            if let Some(later) = word_at(&rest, unit, true) {
                let end = later + unit.len();
                let start = rest[..later].trim_end().len();
                rest.replace_range(start..end, "");
            }
        }
    } else if figure.converted {
        unit_said = figure.unit_word(language);
    }

    // A measure the line restates in the other system, `(2t)` or `or 2 tsp`,
    // goes, and where it is the reader's own measure it is the one scaled
    // (#698): an American is handed teaspoons of yeast, not ounces.
    let mut number = number;
    if let Some(source) = figure.source
        && let Some(restated) = restated_after(&rest, written_in_language, source, false)
    {
        rest.replace_range(..restated.end, "");
        if restated.is_readers_own(measures)
            && let Some(own) = figure_in(
                written_in_language,
                &restated.amount,
                Some(&restated.unit),
                scale,
                measures,
                cup_weight_grams,
            )
        {
            figure = own;
            number = figure.number(language);
            unit_said = figure.unit_word(language);
        }
    }
    // Any other measure the line restates, `(240 ml)` or `/ 0.9-1 lb`, would
    // now say a different amount from the one just scaled, so it goes. Only
    // after a Unit of the closed set: `1 can (14 oz)` is the size of the can.
    if figure.source.is_some() {
        rest = without_restated_measures(&rest, written_in_language);
    }

    // With no Unit, the Food is what is counted: `1 yellow onion`.
    if unit.is_none()
        && let Some(target) = target
        && let Some(found) = word_at(&rest, target, false)
    {
        let last = target
            .char_indices()
            .rfind(|(_, c)| c.is_whitespace())
            .map_or(found, |(space, c)| found + space + c.len_utf8());
        // The whole word the line writes, so a Food read as `egg` on the
        // line `2 eggs` counts `eggs`.
        let end = rest[last..]
            .find(|c: char| !c.is_alphanumeric())
            .map_or(rest.len(), |length| last + length);
        let counted = recount(&rest[last..end]);
        rest.replace_range(last..end, &counted);
    }

    let figure_said = if unit_said.is_empty() {
        number
    } else {
        format!("{number} {unit_said}")
    };
    let figure_said = figure.about_if_due(figure_said, language);
    Some(format!("{before}{figure_said}{rest}"))
}

/// Where `amount` stands in the line as a number of its own: not inside
/// `10`, `1½` or `T45`.
///
/// A number inside brackets is no candidate: the line reader reads none
/// there, so `(serves 4) 4 eggs` scales the eggs.
fn amount_at(text: &str, amount: &str, unit: Option<&str>) -> Option<usize> {
    text.match_indices(amount).map(|(at, _)| at).find(|&at| {
        let depth: i32 = text[..at]
            .chars()
            .map(|c| match c {
                '(' | '[' | '{' => 1,
                ')' | ']' | '}' => -1,
                _ => 0,
            })
            .sum();
        if depth > 0 {
            return false;
        }
        let before = text[..at].chars().next_back();
        let rest = &text[at + amount.len()..];
        let after = rest.chars().next();
        // `400g`: the Unit written against the amount ends it as a space would.
        let glued = unit
            .is_some_and(|unit| unit.starts_with(char::is_alphabetic) && rest.starts_with(unit));
        !before.is_some_and(|c| c.is_alphanumeric() || c == '.' || c == ',')
            && (glued
                || !after.is_some_and(|c| {
                    c.is_alphanumeric() || FRACTIONS.iter().any(|(glyph, _)| *glyph == c)
                }))
    })
}

/// Where `words` start in `text` as whole words, first from the left.
///
/// The end is free where the words end in a letter the line may go on
/// after, as the plural `eggs` does after the Food `egg`: [`scaled_line_in`]
/// counts the whole word. `ends` asks for the end to be a boundary too.
fn word_at(text: &str, words: &str, ends: bool) -> Option<usize> {
    text.match_indices(words).map(|(at, _)| at).find(|&at| {
        let starts = !text[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric);
        let ended = !ends
            || !text[at + words.len()..]
                .chars()
                .next()
                .is_some_and(char::is_alphanumeric);
        starts && ended
    })
}

/// Whether the line spells a Unit of the closed set out in full, `tablespoon`
/// or `liters`, rather than abbreviating it as `tbsp` or `l`.
fn spells_out(written: &str, unit: &Unit) -> bool {
    let named = english_singular(&written.to_lowercase());
    let american = unit.id.strip_suffix("re").map(|stem| format!("{stem}er"));
    named == unit.id || american.is_some_and(|american| named == american)
}

/// `rest` with every measure in it that only restates the line's own amount
/// taken out: a bracketed `(240 ml)` or `(about 1½ cups)`, and `/ 0.9-1 lb`
/// straight after the Unit.
fn without_restated_measures(rest: &str, language: &str) -> String {
    let mut out = rest.to_string();
    // A slash straight after the Unit restates it.
    let trimmed = out.trim_start();
    if let Some(after) = trimmed.strip_prefix('/') {
        let words: Vec<(usize, &str)> = word_spans(after);
        let texts: Vec<&str> = words.iter().map(|(_, word)| *word).collect();
        if let Some(taken) = measure_words(language, &texts) {
            let (start, word) = words[taken - 1];
            let end = out.len() - trimmed.len() + 1 + start + word.len();
            out.replace_range(..end, "");
        }
    }
    // Every bracketed aside that is a measure and nothing else.
    let mut from = 0;
    while let Some(open) = out[from..].find('(').map(|at| from + at) {
        let Some(close) = out[open..].find(')').map(|at| open + at) else {
            break;
        };
        let inner: Vec<&str> = out[open + 1..close].split_whitespace().collect();
        // `about` or `approx.` may come before the figure, and nothing else:
        // `(plus 2 tbsp)` says something more.
        let figure_starts = usize::from(
            inner
                .first()
                .is_some_and(|word| words::of(language).approximately.has(word)),
        );
        let restates = inner.len() > figure_starts
            && measure_words(language, &inner[figure_starts..])
                == Some(inner.len() - figure_starts);
        if restates {
            let start = out[..open].trim_end().len();
            out.replace_range(start..=close, "");
            from = start;
        } else {
            from = close + 1;
        }
    }
    out
}

/// Each word of `text` with the byte where it starts.
fn word_spans(text: &str) -> Vec<(usize, &str)> {
    text.split_whitespace()
        .map(|word| (word.as_ptr() as usize - text.as_ptr() as usize, word))
        .collect()
}

/// How many leading words are an amount and a Unit of the closed set,
/// `240 ml`, `0.9-1 lb` or `1 2/3c`, or nothing where they are not.
fn measure_words(language: &str, words: &[&str]) -> Option<usize> {
    (1..=words.len().min(5))
        .rev()
        .find(|&taken| split_measure(language, &words[..taken].join(" ")).is_some())
}

/// **A measure, as its amount and its Unit**, `240 ml` or `0.9-1 lb`, or
/// the two written against each other, `2t` and `1 2/3c` (#698). Each is a
/// slice of `text`. Nothing where `text` is anything more or less than an
/// amount, single or a range, and a Unit of the closed set.
fn split_measure<'a>(language: &str, text: &'a str) -> Option<(&'a str, &'a str)> {
    let lists = words::of(language);
    let text = text.trim().trim_end_matches([',', ';']);
    text.char_indices().skip(1).find_map(|(at, character)| {
        // The Unit starts with a letter, so the amount before it is whole.
        if !character.is_alphabetic() && character != '#' {
            return None;
        }
        let amount = text[..at].trim();
        let unit = text[at..].trim();
        let reads = parse_amount(amount).is_some() || parse_range_in(language, amount).is_some();
        (reads && lists.unit(unit).is_some()).then_some((amount, unit))
    })
}

/// **A measure straight after an amount's Unit that says the amount again
/// in the other system** (#698): ` (1 2/3c)`, ` (about 1½ cups)`, and, where
/// `brackets_only` is false, ` / 0.9-1 lb`, ` or 2 tsp` and the bare
/// ` 6Tbsp` of *100g 6Tbsp tomato paste*.
///
/// Only the other system counts. In the same one a second measure adds to
/// the first, `1 lb 8 oz`, or says it again with no news for anybody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restated {
    /// Where it ends in the text after the Unit, in bytes. It starts at the
    /// start of that text, with the space before it.
    pub end: usize,
    pub amount: String,
    pub unit: String,
    /// The system its Unit is in, never the amount's own.
    pub system: System,
}

/// [`Restated`] in `rest`, the text after an amount written in `source`.
pub fn restated_after(
    rest: &str,
    language: &str,
    source: &Unit,
    brackets_only: bool,
) -> Option<Restated> {
    let lists = words::of(language);
    let trimmed = rest.trim_start();
    let offset = |inner: &str| inner.as_ptr() as usize - rest.as_ptr() as usize;
    let (amount, unit, end) = if let Some(inside) = trimmed.strip_prefix('(') {
        let close = inside.find(')')?;
        let inner = inside[..close].trim();
        let figure = match inner.split_whitespace().next() {
            Some(word) if lists.approximately.has(word) => inner[word.len()..].trim(),
            _ => inner,
        };
        let (amount, unit) = split_measure(language, figure)?;
        (amount, unit, offset(inside) + close + 1)
    } else if brackets_only {
        return None;
    } else {
        let after = match trimmed.strip_prefix('/') {
            Some(after) => after,
            None => match trimmed.split_whitespace().next() {
                Some(word) if lists.choices.has(word) => &trimmed[word.len()..],
                _ => trimmed,
            },
        };
        let words = word_spans(after);
        let texts: Vec<&str> = words.iter().map(|(_, word)| *word).collect();
        let taken = measure_words(language, &texts)?;
        let (from, _) = words[0];
        let (last, word) = words[taken - 1];
        let (amount, unit) = split_measure(language, &after[from..last + word.len()])?;
        (amount, unit, offset(unit) + unit.len())
    };
    let restated = lists.unit(unit)?;
    (restated.system != source.system).then(|| Restated {
        end,
        amount: amount.to_string(),
        unit: unit.to_string(),
        system: restated.system,
    })
}

impl Restated {
    /// Whether a reader in `measures` is better given this than the amount
    /// it restates: it is in their own system, and so, being in the other
    /// one, the amount is not (#698). An American is handed the teaspoons
    /// the line wrote, not an ounce figure made from its grams.
    pub fn is_readers_own(&self, measures: Measures) -> bool {
        match measures {
            Measures::Metric => self.system == System::Metric,
            Measures::Us => self.system == System::Customary,
            Measures::AsWritten => false,
        }
    }
}

/// The short line beside an Ingredient Line, [`measured_line_in`], made from
/// the measure the line restates where that is the reader's own
/// ([`Restated::is_readers_own`]): `7g (2t)` read as written, by an American,
/// needs nothing beside it (#698).
#[allow(clippy::too_many_arguments)]
pub fn measured_line_of(
    text: &str,
    written_in_language: &str,
    amount: Option<&str>,
    unit: Option<&str>,
    scale: f64,
    measures: Measures,
    language: &str,
    cup_weight_grams: Option<f64>,
) -> Option<String> {
    let restated = amount.zip(unit).and_then(|(amount, unit)| {
        readers_own_in_line(text, written_in_language, amount, unit, measures)
    });
    let (amount, unit) = match &restated {
        Some(restated) => (Some(restated.amount.as_str()), Some(restated.unit.as_str())),
        None => (amount, unit),
    };
    measured_line_in(
        written_in_language,
        amount,
        unit,
        scale,
        measures,
        language,
        cup_weight_grams,
    )
}

/// **The measure an Ingredient Line restates its amount in, where that is
/// the reader's own** ([`Restated::is_readers_own`]): the `5 3/4c` of
/// `680g (5 3/4c) All-purpose flour`, for an American. A Step naming the
/// line's own amount is given it too, so the list and the Step say one
/// figure (#705).
pub fn readers_own_in_line(
    text: &str,
    written_in_language: &str,
    amount: &str,
    unit: &str,
    measures: Measures,
) -> Option<Restated> {
    let source = words::of(written_in_language).unit(unit)?;
    let at = amount_at(text, amount, Some(unit))?;
    let rest = text[at + amount.len()..].trim_start().strip_prefix(unit)?;
    restated_after(rest, written_in_language, source, false)
        .filter(|restated| restated.is_readers_own(measures))
}

/// Whether two amounts, each with its Unit as written in `language`, are
/// the same measure: `680` `g` and `680` `grams`.
pub fn same_measure(language: &str, one: (&str, &str), other: (&str, &str)) -> bool {
    let lists = words::of(language);
    let unit_of = |unit: &str| lists.unit(unit).map(|unit| unit.id);
    let ends = |amount: &str| {
        parse_amount(amount)
            .map(|quantity| (quantity, quantity))
            .or_else(|| parse_range_in(language, amount))
    };
    let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
    unit_of(one.1).is_some()
        && unit_of(one.1) == unit_of(other.1)
        && ends(one.0)
            .zip(ends(other.0))
            .is_some_and(|((low, high), (other_low, other_high))| {
                close(low, other_low) && close(high, other_high)
            })
}

/// **One amount, scaled and converted, before it is worded.** A range keeps
/// both its ends, each in the Unit the larger one is worded in.
struct Figure {
    low: Option<f64>,
    high: f64,
    /// The larger end as written, before scaling.
    written_high: f64,
    unit: Option<&'static Unit>,
    as_written: Option<String>,
    source: Option<&'static Unit>,
    /// Into another Unit than the one written.
    converted: bool,
    /// Kamosu rounded or converted, so the figure is not exact (#680).
    about: bool,
    /// The line already says this, in this Unit and at this number.
    unchanged: bool,
    /// What is left after a spoon carried up into a bigger tool, in the
    /// smaller ones: the `2 tbsp` of `½ cup + 2 tbsp` (#705).
    smaller: Vec<(f64, &'static Unit)>,
    /// A range converted into ounces, said in whole ounces (#705).
    whole_ounces: bool,
}

impl Figure {
    /// One end rounded to the marks it is said on.
    fn round(&self, quantity: f64) -> f64 {
        if self.whole_ounces {
            quantity.round().max(1.0)
        } else {
            round_line(quantity, self.unit)
        }
    }

    fn rounded_high(&self) -> f64 {
        self.round(self.high)
    }

    fn number(&self, language: &str) -> String {
        let one = |quantity: f64| number_for(self.round(quantity), self.unit, language);
        match self.low {
            Some(low) => format!("{}-{}", one(low), one(self.high)),
            None => one(self.high),
        }
    }

    /// The Unit's word, and the rest in each smaller tool after it where a
    /// spoon was carried up: `cup + 2 tbsp`.
    fn unit_word(&self, language: &str) -> String {
        let word = match (&self.unit, &self.as_written) {
            (Some(unit), _) => display(unit, language, plural(self.rounded_high(), language)),
            (None, Some(written)) => written.clone(),
            (None, None) => String::new(),
        };
        self.smaller.iter().fold(word, |said, &(quantity, unit)| {
            let rounded = round_line(quantity, Some(unit));
            format!(
                "{said} + {} {}",
                number_for(rounded, Some(unit), language),
                display(unit, language, plural(rounded, language))
            )
        })
    }

    /// The figure and its Unit, with "about" where it is due.
    fn said(&self, language: &str) -> String {
        let number = self.number(language);
        let unit = self.unit_word(language);
        let said = if unit.is_empty() {
            number
        } else {
            format!("{number} {unit}")
        };
        self.about_if_due(said, language)
    }

    /// "about" before a figure Kamosu rounded or converted, and nothing
    /// before one that is exact (#680).
    fn about_if_due(&self, said: String, language: &str) -> String {
        if self.about {
            format!("{} {said}", about(language))
        } else {
            said
        }
    }
}

/// **The tool in an American drawer that measures this** (#698), the
/// largest that says it: a cup, then a tablespoon, then a teaspoon, or a
/// pound, then an ounce. The Unit written is held to the same test, so a
/// sixteenth of a cup goes on to the tablespoon. Nothing where the Unit
/// written is the one, or is neither a spoon, a cup nor a weight.
///
/// A tool says it when every end of the amount, `share` being the lower
/// end's share of the higher in a range, sits at or above its smallest
/// measure (a quarter cup, one tablespoon, one pound) and, for a cup made
/// from spoons, on a cup the set has (a quarter or a third), and when it is
/// no further from the arithmetic than the Unit written. So `6 tsp` is
/// `2 tbsp`, `15 tsp` is `5 tbsp`, `16 tsp` is `⅓ cup`, while `4 tsp` stays
/// four rather than become about 1¼ tablespoons and `6 tbsp` is no ⅜ cup.
fn drawers_tool(
    quantity: f64,
    written: &'static Unit,
    share: Option<f64>,
) -> Option<&'static Unit> {
    let tools: &[&'static str] = match written.id {
        "teaspoon" | "tablespoon" | "cup" => &["cup", "tablespoon", "teaspoon"],
        "ounce" | "pound" => &["pound", "ounce"],
        _ => return None,
    };
    let base = quantity * written.base;
    let ends = |unit: &'static Unit| {
        let high = base / unit.base;
        [Some(high), share.map(|share| high * share)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    };
    let off = |unit: &'static Unit| -> f64 {
        ends(unit)
            .iter()
            .map(|end| (round_line(*end, Some(unit)) - end).abs() * unit.base)
            .sum()
    };
    let says_it = |unit: &'static Unit| {
        let smallest = match unit.id {
            "cup" => 0.25,
            "tablespoon" | "pound" => 1.0,
            _ => 0.0,
        };
        let whole = |end: f64| (end.round() - end).abs() < 1e-9;
        // Spoons become cups only on a cup the set has. A cup the recipe
        // wrote keeps the eighths every cup line is read to.
        let on_a_mark = |end: f64| {
            unit.id != "cup" || unit.id == written.id || whole(end * 4.0) || whole(end * 3.0)
        };
        ends(unit)
            .iter()
            .all(|&end| end >= smallest - 1e-9 && on_a_mark(end))
            && off(unit) <= off(written) + 1e-9
    };
    tools
        .iter()
        .map(|id| unit_by_id(id))
        .find(|unit| says_it(unit))
        .filter(|unit| unit.id != written.id)
}

/// [`Figure`] for an amount as written, a single one or a range, or nothing
/// where it is neither.
fn figure_in(
    written_in_language: &str,
    amount: &str,
    unit: Option<&str>,
    scale: f64,
    measures: Measures,
    cup_weight_grams: Option<f64>,
) -> Option<Figure> {
    let (low, high) = match parse_amount(amount) {
        Some(quantity) => (None, quantity),
        None => {
            let (low, high) = parse_range_in(written_in_language, amount)?;
            (Some(low), high)
        }
    };
    if !scale.is_finite() || scale <= 0.0 || high <= 0.0 {
        return None;
    }
    let source = unit.and_then(|unit| words::of(written_in_language).unit(unit));
    let mut measured = convert(high * scale, source, unit, measures, cup_weight_grams);
    // Too little for the reader's Unit, `about 0 oz` of yeast: the Unit
    // written says it better than a figure of nothing (#698).
    if measured.unit.map(|u| u.system) != source.map(|u| u.system)
        && rounds_to_nothing(measured.quantity, measured.unit)
    {
        measured = Measured {
            quantity: high * scale,
            unit: source,
            as_written: None,
        };
    }
    // Scaled, an American measure is said with the tool that measures it,
    // `6 tsp` as `2 tbsp` (#698). Read as written it is left as the cook
    // wrote it, the rule [`convert`] keeps.
    if scale != 1.0
        && measures == Measures::Us
        && let Some(written) = source
        && measured.unit.map(|u| u.id) == Some(written.id)
        && let Some(tool) = drawers_tool(measured.quantity, written, low.map(|low| low / high))
    {
        measured = Measured {
            quantity: measured.quantity * written.base / tool.base,
            unit: Some(tool),
            as_written: None,
        };
    }
    // A scaled spoon that reaches a bigger tool is carried up into it, the
    // rest said with the smaller ones (#705): `10 tbsp` is `½ cup + 2 tbsp`.
    let mut smaller = Vec::new();
    let mut carried_inexact = false;
    if scale != 1.0
        && measures == Measures::Us
        && low.is_none()
        && let Some(carried) = carried_up(measured.quantity, measured.unit)
    {
        measured = Measured {
            quantity: carried.count,
            unit: Some(carried.big),
            as_written: None,
        };
        smaller = carried.smaller;
        carried_inexact = carried.inexact;
    }
    // Every conversion is a ratio, so the lower end is worth the same share
    // of the larger one after it as before.
    let low_measured = low.map(|low| measured.quantity * low / high);

    let converted = measured.unit.map(|u| u.id) != source.map(|u| u.id);
    // Another Unit of the same system is exact arithmetic, `6 tsp` being
    // `2 tbsp`; only another system, or rounding, makes a figure "about".
    let crossed = measured.unit.map(|u| u.system) != source.map(|u| u.system);
    let rounded = |quantity: f64| (round_line(quantity, measured.unit) - quantity).abs() > 1e-9;
    let about = crossed
        || rounded(measured.quantity)
        || low_measured.is_some_and(rounded)
        || carried_inexact;
    // A range converted into ounces is a guess at both ends already, so it
    // is said in the whole ounces a kitchen scale shows (#705), where whole
    // ounces still tell its ends apart: `20-30 g` is no `1-1 oz`.
    let whole_ounces = crossed
        && measured.unit.is_some_and(|unit| unit.id == "ounce")
        && low_measured
            .is_some_and(|low| low.round() >= 2.0 && low.round() < measured.quantity.round());
    // The comparison is on the rounded figure, because that is the one a
    // reader would be comparing against the line above.
    let same = |now: f64, then: f64| round_line(now, measured.unit) == round_line(then, source);
    let unchanged = !converted
        && same(measured.quantity, high)
        && match (low_measured, low) {
            (Some(now), Some(then)) => same(now, then),
            _ => true,
        };
    Some(Figure {
        low: low_measured,
        high: measured.quantity,
        written_high: high,
        unit: measured.unit,
        as_written: measured.as_written,
        source,
        converted,
        about,
        unchanged,
        smaller,
        whole_ounces,
    })
}

/// **A spoon amount carried up into bigger tools** (#705), as an American
/// recipe writes `½ cup + 2 tbsp`: as many quarter cups as it holds, then
/// whole tablespoons, then the teaspoons left. Only an amount that reaches
/// a tablespoon, from teaspoons, or a quarter cup, from tablespoons, is
/// carried.
struct Carried {
    /// The biggest tool, and how many of it.
    big: &'static Unit,
    count: f64,
    /// The rest, in each smaller tool that holds some.
    smaller: Vec<(f64, &'static Unit)>,
    /// Rounding the teaspoons left changed the amount.
    inexact: bool,
}

/// [`Carried`] for an amount, or nothing where it is no spoon or reaches no
/// bigger tool.
fn carried_up(quantity: f64, unit: Option<&'static Unit>) -> Option<Carried> {
    let unit = unit?;
    let cup = unit_by_id("cup");
    let tablespoon = unit_by_id("tablespoon");
    let teaspoon = unit_by_id("teaspoon");
    let quarter_cup = cup.base / 4.0;
    let millilitres = quantity * unit.base;
    let reaches = match unit.id {
        "teaspoon" => tablespoon.base,
        "tablespoon" => quarter_cup,
        _ => return None,
    };
    if millilitres < reaches - 1e-9 {
        return None;
    }
    // The teaspoons past the last whole tablespoon are rounded to a
    // teaspoon's marks first, and may round up to one more tablespoon.
    let teaspoons = millilitres / teaspoon.base;
    let whole_spoons = (teaspoons / 3.0 + 1e-9).floor();
    // In plain eighths, so a remainder of nothing, or of next to nothing
    // left by the arithmetic, is no teaspoon at all.
    let rounded = whole_spoons * 3.0 + ((teaspoons - whole_spoons * 3.0) / 0.125).round() * 0.125;
    let inexact = (rounded - teaspoons).abs() > 1e-9;
    let mut left = rounded * teaspoon.base;
    let mut take = |mark: f64| {
        let marks = (left / mark + 1e-9).floor();
        left -= marks * mark;
        marks
    };
    let quarters = take(quarter_cup);
    let spoons = take(tablespoon.base);
    let mut parts = [
        (quarters / 4.0, cup),
        (spoons, tablespoon),
        (left / teaspoon.base, teaspoon),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 1e-9);
    let (count, big) = parts.next()?;
    Some(Carried {
        big,
        count,
        smaller: parts.collect(),
        inexact,
    })
}

/// **A range written as an amount**, `500-700` or `2 to 3`, as its two ends
/// (ADR 0060). The same test the line reader makes: two amounts joined by a
/// hyphen, a dash or one of the Language's range words.
fn parse_range_in(language: &str, text: &str) -> Option<(f64, f64)> {
    let lists = words::of(language);
    let parts: Vec<&str> = text.split_whitespace().collect();
    let joined = parts
        .iter()
        .position(|word| lists.amount_ranges.has(word))
        .map(|at| (parts[..at].join(" "), parts[at + 1..].join(" ")));
    let (low, high) = joined.or_else(|| {
        let (low, high) = text.split_once(['-', '\u{2013}', '\u{2014}'])?;
        Some((low.to_string(), high.to_string()))
    })?;
    let (low, high) = (parse_amount(&low)?, parse_amount(&high)?);
    (low <= high).then_some((low, high))
}

/// The English plural of the word a line counts with: `head` to `heads`,
/// `pinch` to `pinches`, `cherry` to `cherries`. The plurals no rule
/// reaches are words, in the English word file's `irregular` list.
fn english_plural(word: &str) -> String {
    let lower = word.to_lowercase();
    if let Some((many, _)) = words::of("en")
        .irregular
        .iter()
        .find(|(_, one)| **one == lower)
    {
        return with_capital_of(word, many);
    }
    if ["ss", "sh", "ch", "x", "z"]
        .iter()
        .any(|end| lower.ends_with(end))
    {
        return format!("{word}es");
    }
    if lower.ends_with('s') {
        return word.to_string();
    }
    if let Some(stem) = lower.strip_suffix('y')
        && stem
            .chars()
            .next_back()
            .is_some_and(|c| !"aeiou".contains(c))
    {
        return format!("{}ies", &word[..word.len() - 1]);
    }
    format!("{word}s")
}

/// The other way: `heads` to `head`, `cherries` to `cherry`, and a word that
/// only ends in an s, `asparagus`, left as it is.
fn english_singular(word: &str) -> String {
    let lower = word.to_lowercase();
    if let Some(one) = words::of("en").irregular.get(&lower) {
        return with_capital_of(word, one);
    }
    if ["ss", "us", "is"].iter().any(|end| lower.ends_with(end)) {
        return word.to_string();
    }
    if lower.len() > 4 && lower.ends_with("ies") {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if ["sses", "shes", "ches", "xes", "zes", "oes"]
        .iter()
        .any(|end| lower.ends_with(end))
    {
        return word[..word.len() - 2].to_string();
    }
    if lower.ends_with('s') {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// **A Yield's noun for one and for many** (#705), in the recipe's Language,
/// whichever of the two it wrote: a screen counting servings one at a time
/// says `1 serving` and `2 servings` from it. The grammar is here and the
/// plurals no rule reaches are the word files' `irregular` lists. Only the
/// last word counts, so `ring cake` is `ring cakes`.
pub struct YieldNouns {
    pub one: String,
    pub many: String,
}

pub fn yield_nouns(amount: &str, noun: &str, language: &str) -> YieldNouns {
    let (head, last) = match noun.rsplit_once(' ') {
        Some((head, last)) => (format!("{head} "), last),
        None => (String::new(), noun),
    };
    // A noun after one, or after a figure in the singular, is the one.
    let written_one = parse_amount(amount).is_some_and(|quantity| !plural(quantity, language));
    let (one, many) = if written_one {
        (last.to_string(), noun_many(last, language))
    } else {
        (noun_one(last, language), last.to_string())
    };
    YieldNouns {
        one: format!("{head}{one}"),
        many: format!("{head}{many}"),
    }
}

/// The singular of a plural noun in `language`.
fn noun_one(word: &str, language: &str) -> String {
    let lower = word.to_lowercase();
    if let Some(one) = words::of(language).irregular.get(&lower) {
        return with_capital_of(word, one);
    }
    let strip = |end: &str| word[..word.len() - end.len()].to_string();
    match language {
        "en" => english_singular(word),
        "fr" if lower.ends_with("eaux") || lower.ends_with("eux") => strip("x"),
        "fr" if lower.len() > 2 && lower.ends_with('s') => strip("s"),
        "es" if lower.ends_with("ones") => format!("{}ón", strip("ones")),
        "es" if lower.ends_with("ces") => format!("{}z", strip("ces")),
        // *Panes* is *pan* and *unidades* *unidad*, a vowel before the
        // last consonant, while *postres* is *postre*.
        "es" if lower.len() > 3
            && lower.ends_with("es")
            && lower[..lower.len() - 2].ends_with(['l', 'n', 'r', 'd', 'j'])
            && lower[..lower.len() - 3].ends_with(|c: char| "aeiouáéíóú".contains(c)) =>
        {
            strip("es")
        }
        "es" if lower.len() > 2 && lower.ends_with('s') => strip("s"),
        "nl" => crate::dutch::singular(words::of(language), word),
        _ => word.to_string(),
    }
}

/// The plural of a singular noun in `language`, or the word itself where
/// the grammar here cannot say.
fn noun_many(word: &str, language: &str) -> String {
    let lower = word.to_lowercase();
    if let Some((many, _)) = words::of(language)
        .irregular
        .iter()
        .find(|(_, one)| **one == lower)
    {
        return with_capital_of(word, many);
    }
    let vowel = |c: char| "aeiouáéíóú".contains(c);
    match language {
        "en" => english_plural(word),
        "fr" if lower.ends_with(['s', 'x', 'z']) => word.to_string(),
        "fr" if lower.ends_with("eau") => format!("{word}x"),
        "fr" => format!("{word}s"),
        "es" if lower.ends_with("ón") => {
            format!("{}ones", &word[..word.len() - "ón".len()])
        }
        "es" if lower.ends_with('z') => format!("{}ces", &word[..word.len() - 1]),
        "es" if lower.ends_with(vowel) => format!("{word}s"),
        "es" => format!("{word}es"),
        "nl" => crate::dutch::plural(words::of(language), word),
        _ => word.to_string(),
    }
}

/// Scale, then convert — never the other way round, and never rounding in
/// between (ADR 0016: "round last, once").
fn convert(
    scaled: f64,
    source: Option<&'static Unit>,
    written: Option<&str>,
    measures: Measures,
    cup_weight_grams: Option<f64>,
) -> Measured {
    // A Unit outside the closed set, or a reader who asked for no conversion:
    // the quantity multiplies and the unit is untouched.
    let Some(source) = source.filter(|_| measures != Measures::AsWritten) else {
        return Measured {
            quantity: scaled,
            unit: source,
            as_written: written.map(str::to_string),
        };
    };

    // A recipe already in your measures is untouched (ADR 0016). Kamosu
    // converts BETWEEN systems and never re-expresses within one: an American
    // told that her `4 tsp` is `about 1¼ tbsp` has been given a second way to
    // say a thing she already has the spoons for, and a line that says nothing
    // new is exactly the line the ADR wanted absent. Scaling still applies —
    // it is what the fall-through below carries.
    let reader = match measures {
        Measures::Metric => System::Metric,
        Measures::Us => System::Customary,
        Measures::AsWritten => unreachable!("as-written left before conversion"),
    };
    if source.system == reader {
        return Measured {
            quantity: scaled,
            unit: Some(source),
            as_written: None,
        };
    }

    let base = scaled * source.base;
    match (measures, source.family) {
        // A metric kitchen has scales, so a customary volume of a dry good
        // becomes a weight — the one cross-family crossing there is, and the
        // whole reason a Cup Weight exists.
        (Measures::Metric, Family::Volume) if source.system == System::Customary => {
            match crossed_into_grams(base, source, measures, cup_weight_grams) {
                Some(grams) => in_metric_mass(grams),
                // A blank Cup Weight is not a gap to close. It is a line that
                // offers millilitres instead, and says nothing about grams.
                None => in_metric_volume(base),
            }
        }
        (Measures::Metric, Family::Volume) => in_metric_volume(base),
        (Measures::Metric, Family::Mass) => in_metric_mass(base),
        // An American kitchen has cups and a set of spoons, so a metric measure
        // becomes one of those. Nothing crosses families this way: she is not
        // short of a cup, and turning her cups into grams would take away the
        // equipment she has.
        (Measures::Us, Family::Volume) => in_customary_volume(base),
        (Measures::Us, Family::Mass) => in_customary_mass(base),
        (Measures::AsWritten, _) => unreachable!("as-written left before conversion"),
    }
}

fn in_metric_mass(grams: f64) -> Measured {
    let unit = if grams >= 1000.0 { "kilogram" } else { "gram" };
    at(grams, unit, if grams >= 1000.0 { 1000.0 } else { 1.0 })
}

fn in_metric_volume(millilitres: f64) -> Measured {
    let big = millilitres >= 1000.0;
    at(
        millilitres,
        if big { "litre" } else { "millilitre" },
        if big { 1000.0 } else { 1.0 },
    )
}

/// Which piece of American equipment this is a measure of. The thresholds are
/// the drawer, not the arithmetic: under a tablespoon you reach for teaspoons,
/// under a quarter cup for tablespoons, and above that for the cup itself.
fn in_customary_volume(millilitres: f64) -> Measured {
    let tablespoon = unit_by_id("tablespoon").base;
    let cup = unit_by_id("cup").base;
    if millilitres < tablespoon {
        at(millilitres, "teaspoon", unit_by_id("teaspoon").base)
    } else if millilitres < cup / 4.0 {
        at(millilitres, "tablespoon", tablespoon)
    } else {
        at(millilitres, "cup", cup)
    }
}

fn in_customary_mass(grams: f64) -> Measured {
    let pound = unit_by_id("pound").base;
    if grams < pound {
        at(grams, "ounce", unit_by_id("ounce").base)
    } else {
        at(grams, "pound", pound)
    }
}

fn at(base_quantity: f64, id: &'static str, per: f64) -> Measured {
    Measured {
        quantity: base_quantity / per,
        unit: Some(unit_by_id(id)),
        as_written: None,
    }
}

/// **Which bucket one contribution to a Shopping Row belongs in** (#73).
///
/// `Some(grams)` puts it with the masses; `None` puts it with the volumes. It
/// is the same crossing [`convert`] makes and no other: a customary volume of a
/// Food with a Cup Weight becomes a weight for a metric kitchen, which has
/// scales, and nothing ever crosses the other way, because an American kitchen
/// is not short of a cup.
///
/// `base` is the amount already in the Unit's own base — grams for a Mass,
/// millilitres for a Volume.
///
/// It takes `measures` for the reason [`convert`] does: which bucket an amount
/// belongs in is a fact about the kitchen reading the list, not about the
/// recipe. A metric reader adds `2 cups flour` to `500 g flour` and gets one
/// weight; an American reader keeps them apart, because they are two things she
/// measures with two different tools.
pub fn in_grams(
    base: f64,
    unit: &'static Unit,
    measures: Measures,
    cup_weight_grams: Option<f64>,
) -> Option<f64> {
    match unit.family {
        Family::Mass => Some(base),
        Family::Volume => crossed_into_grams(base, unit, measures, cup_weight_grams),
    }
}

/// **The one crossing between families Kamosu makes, decided in one place.**
///
/// A metric kitchen has scales, so a *customary* volume of a Food with a Cup
/// Weight becomes a weight — and that is the whole of it. A metric volume is
/// left alone for the reason ADR 0016 gives: Kamosu converts *between* systems
/// and never re-expresses within one, and a reader with a jug is not short of a
/// millilitre. An American keeps her cups, so nothing crosses for her either.
///
/// Two callers need this answered and they need the same answer: [`convert`],
/// wording one line under a recipe, and [`in_grams`], deciding which amount a
/// Shopping Row adds a contribution into. Two spellings of it would be two
/// chances for a cup of flour to weigh one thing on a recipe page and another
/// in a shop.
fn crossed_into_grams(
    base: f64,
    unit: &Unit,
    measures: Measures,
    cup_weight_grams: Option<f64>,
) -> Option<f64> {
    let per_cup = cup_weight_grams?;
    (unit.family == Family::Volume
        && measures == Measures::Metric
        && unit.system == System::Customary)
        .then(|| base / unit_by_id("cup").base * per_cup)
}

/// A Shopping Row's mass, worded for one reader (#73).
pub fn worded_mass(grams: f64, measures: Measures, language: &str) -> String {
    let measured = match measures {
        Measures::Us => in_customary_mass(grams),
        _ => in_metric_mass(grams),
    };
    word_it(measured, language)
}

/// A Shopping Row's volume, worded for one reader (#73).
pub fn worded_volume(millilitres: f64, measures: Measures, language: &str) -> String {
    let measured = match measures {
        Measures::Us => in_customary_volume(millilitres),
        _ => in_metric_volume(millilitres),
    };
    word_it(measured, language)
}

/// A number with no Unit of Kamosu's own beside it: a count of eggs, or the
/// figure in front of a word Kamosu does not know. Written the way an unknown
/// measure is written everywhere else — in eighths, because a cook's own
/// measure is as likely to be halved as anything else.
pub fn plain_number(quantity: f64) -> String {
    as_fraction(round_to(quantity, None))
}

/// **What a row says where nobody wrote an amount** (ADR 0024) — 28% of real
/// Ingredient Lines (#5). Not a blank and not a zero: a statement that the
/// recipe never said, so the thing still gets bought.
///
/// French and Spanish say *no amount written* rather than reaching for *un
/// peu* or *un poco*, which would be Kamosu guessing at a size nobody gave it.
pub fn no_amount_written(language: &str) -> &'static str {
    match language {
        "fr" => "sans quantité",
        "es" => "sin cantidad",
        "nl" => "zonder hoeveelheid",
        _ => "some",
    }
}

pub(crate) fn unit_by_id(id: &'static str) -> &'static Unit {
    by_id(id).expect("every id named here is in UNITS")
}

/// The Unit of the closed set with this id.
pub fn by_id(id: &str) -> Option<&'static Unit> {
    UNITS.iter().find(|unit| unit.id == id)
}

// --- Rounding, which happens last and once -----------------------------------

/// Round to something a scale or a jug can actually show.
///
/// Every ladder here is equipment, not arithmetic. A kitchen scale reads to the
/// gram under 100 g and nobody trusts its last digit above that; a measuring jug
/// is marked every 10 ml through its useful range; and the cups in the drawer
/// are marked in eighths, which is why an American measure rounds to a fraction
/// and a metric one does not.
///
/// Kept as one function rather than a field on [`Unit`] deliberately: half of
/// these rules turn on the *magnitude* as well as the Unit — a gram rounds to 1
/// under 100 and to 5 above it — so a per-Unit field would carry a rule that
/// still needed reading here. One table of equipment, read top to bottom, is
/// what makes it checkable against a real drawer.
fn round_to(quantity: f64, unit: Option<&'static Unit>) -> f64 {
    let step = step_of(quantity, unit);
    (quantity / step).round() * step
}

/// The mark [`round_to`] rounds to, which is the equipment's.
fn step_of(quantity: f64, unit: Option<&'static Unit>) -> f64 {
    match unit.map(|unit| unit.id) {
        Some("gram") if quantity < 100.0 => 1.0,
        Some("gram") => 5.0,
        Some("kilogram") | Some("litre") => 0.05,
        Some("millilitre") if quantity < 25.0 => 1.0,
        Some("millilitre") if quantity < 100.0 => 5.0,
        Some("millilitre") => 10.0,
        Some("cup") => 0.125,
        Some("teaspoon") | Some("tablespoon") | Some("ounce") | Some("pound") => 0.25,
        // A Unit Kamosu does not know still scales, and the cook's own measure
        // is as likely to be halved as anything else.
        _ => 0.125,
    }
}

/// **[`round_to`] for one amount of one line**, an Ingredient Line's or a
/// Step's, which a cook measures out (#698). Three marks are finer or coarser
/// than a sum on the Shopping List needs, which keeps [`round_to`]:
///
/// - a set of spoons goes down to an eighth of a teaspoon, and a set of
///   cups holds a third and two thirds;
/// - a count, or a Unit Kamosu does not know, is cut in quarters, since nobody
///   cuts ⅜ of a chile;
/// - a positive amount is never said as nothing: under half the smallest mark
///   it is that mark, and "about" says so.
fn round_line(quantity: f64, unit: Option<&'static Unit>) -> f64 {
    let step = line_step(quantity, unit);
    if rounds_to_nothing(quantity, unit) {
        return step;
    }
    let rounded = (quantity / step).round() * step;
    // A set of cups has a third and two thirds beside its eighths.
    let thirds = (quantity * 3.0).round() / 3.0;
    if unit.is_some_and(|unit| unit.id == "cup")
        && (thirds - quantity).abs() < (rounded - quantity).abs()
    {
        return thirds;
    }
    rounded
}

/// Whether one line's `quantity` is under half of the smallest mark
/// [`round_line`] reads its Unit to, so rounding would say it as nothing.
fn rounds_to_nothing(quantity: f64, unit: Option<&'static Unit>) -> bool {
    quantity > 0.0 && (quantity / line_step(quantity, unit)).round() == 0.0
}

/// The mark [`round_line`] rounds to.
fn line_step(quantity: f64, unit: Option<&'static Unit>) -> f64 {
    match unit.map(|unit| unit.id) {
        Some("teaspoon") if quantity < 1.0 => 0.125,
        None => 0.25,
        _ => step_of(quantity, unit),
    }
}

// --- Wording it --------------------------------------------------------------

/// "about", in the Languages Kamosu words things in.
///
/// A line beneath an Ingredient Line or a Step says it where Kamosu rounded
/// or converted, and nowhere else (ADR 0060): `1 × 1.5` is exactly `1½`, and
/// its absence is then a promise Kamosu keeps. A temperature on the oven
/// ladder and a Shopping Row's sum always say it. Spanish uses *aprox.*
/// rather than *unos*, which would have to agree with a noun this code does
/// not know the gender of.
fn about(language: &str) -> &'static str {
    match language {
        "fr" => "environ",
        "es" => "aprox.",
        "nl" => "ongeveer",
        _ => "about",
    }
}

/// How a Unit is written for a reader, singular and plural. Only the cup needs
/// both: every other member of the closed set is written as an invariant
/// symbol or abbreviation in every Language.
fn display(unit: &Unit, language: &str, plural: bool) -> String {
    let word = match (unit.id, language, plural) {
        ("cup", "fr", false) => "tasse",
        ("cup", "fr", true) => "tasses",
        ("cup", "es", false) => "taza",
        ("cup", "es", true) => "tazas",
        ("cup", "nl", false) => "kopje",
        ("cup", "nl", true) => "kopjes",
        ("cup", _, false) => "cup",
        ("cup", _, true) => "cups",
        ("gram", _, _) => "g",
        ("kilogram", _, _) => "kg",
        ("millilitre", _, _) => "ml",
        ("centilitre", _, _) => "cl",
        ("decilitre", _, _) => "dl",
        ("litre", _, _) => "l",
        ("ounce", _, _) => "oz",
        ("pound", _, _) => "lb",
        ("fluid ounce", _, _) => "fl oz",
        ("quart", _, _) => "qt",
        ("teaspoon", "fr", _) => "c. à c.",
        ("teaspoon", "es", _) => "cdta",
        ("teaspoon", "nl", _) => "tl",
        ("teaspoon", _, _) => "tsp",
        ("tablespoon", "fr", _) => "c. à s.",
        ("tablespoon", "es", _) => "cda",
        ("tablespoon", "nl", _) => "el",
        ("tablespoon", _, _) => "tbsp",
        ("imperial tablespoon", _, _) => "imperial tbsp",
        ("australian tablespoon", _, _) => "australian tbsp",
        (other, _, _) => other,
    };
    word.to_string()
}

/// Whether a Unit is written in the plural after this quantity, which each
/// Language rules on differently (#147): English and Spanish go plural above
/// one (*½ cup*, *1½ cups*), French only from two (*1½ tasse*, *2 tasses*).
/// Only the cup has a plural for this to choose between. The phone's offline
/// copy is `plural` in `ui/src/lib/offline/shopping.ts`, and `shopping_parity`
/// holds the two to the same cases.
fn plural(quantity: f64, language: &str) -> bool {
    match language {
        "fr" => quantity >= 2.0,
        _ => quantity > 1.0,
    }
}

/// The cases the plural rule is held to, in cups and each Language's word for
/// them (#147). `shopping_parity` writes them out for the phone's copy too.
#[cfg(test)]
pub(crate) const CUP_PLURALS: &[(f64, &str, &str)] = &[
    (0.5, "en", "about ½ cup"),
    (1.0, "en", "about 1 cup"),
    (1.5, "en", "about 1½ cups"),
    (2.0, "en", "about 2 cups"),
    (0.5, "fr", "environ ½ tasse"),
    (1.0, "fr", "environ 1 tasse"),
    (1.5, "fr", "environ 1½ tasse"),
    (2.0, "fr", "environ 2 tasses"),
    (0.5, "es", "aprox. ½ taza"),
    (1.0, "es", "aprox. 1 taza"),
    (1.5, "es", "aprox. 1½ tazas"),
    (2.0, "es", "aprox. 2 tazas"),
    (0.5, "nl", "ongeveer ½ kopje"),
    (1.0, "nl", "ongeveer 1 kopje"),
    (1.5, "nl", "ongeveer 1½ kopjes"),
    (2.0, "nl", "ongeveer 2 kopjes"),
];

/// A number of cups as millilitres, for a test that words them back.
#[cfg(test)]
pub(crate) fn cups_in_millilitres(cups: f64) -> f64 {
    cups * unit_by_id("cup").base
}

/// `listed` with a capital where `written` opens with one, `Tomatoes`.
fn with_capital_of(written: &str, listed: &str) -> String {
    let mut letters = listed.chars();
    match (written.chars().next(), letters.next()) {
        (Some(first), Some(head)) if first.is_uppercase() => {
            head.to_uppercase().chain(letters).collect()
        }
        _ => listed.to_string(),
    }
}

/// A rounded figure as its reader writes it. Metric stays whole numbers; the
/// cups in the drawer are marked in fractions, so an American measure, or a
/// count, is written the way the cup is marked.
fn number_for(quantity: f64, unit: Option<&'static Unit>, language: &str) -> String {
    let fractional = unit.is_none_or(|unit| unit.system == System::Customary);
    let number = if fractional {
        as_fraction(quantity)
    } else {
        as_decimal(quantity)
    };
    with_decimal_mark(number, language)
}

fn word_it(measured: Measured, language: &str) -> String {
    let quantity = round_to(measured.quantity, measured.unit);
    let number = number_for(quantity, measured.unit, language);
    let unit = match (&measured.unit, &measured.as_written) {
        (Some(unit), _) => display(unit, language, plural(quantity, language)),
        // The cook's own word, exactly as written.
        (None, Some(written)) => written.clone(),
        (None, None) => String::new(),
    };
    if unit.is_empty() {
        format!("{} {number}", about(language))
    } else {
        format!("{} {number} {unit}", about(language))
    }
}

/// A written number with the decimal mark its reader writes: `1,5` in Dutch
/// (#496), French and Spanish (#599), `1.5` in English. The
/// phone's offline copy is `withDecimalMark` in `ui/src/lib/offline/shopping.ts`,
/// and `shopping_parity` holds the two to [`DECIMAL_MARKS`].
fn with_decimal_mark(number: String, language: &str) -> String {
    match language {
        "fr" | "es" | "nl" => number.replace('.', ","),
        _ => number,
    }
}

/// How much of a Component's recipe a line takes, as its reader writes it:
/// `½`, or `0,3` in French, Spanish and Dutch where the share has no glyph (#496, #599).
pub fn share_written(share: f64, language: &str) -> String {
    with_decimal_mark(as_fraction(share), language)
}

/// The cases the decimal mark is held to: millilitres, worded for a metric
/// reader in each Language (#496, #599). `shopping_parity` writes them out for the phone's copy too.
#[cfg(test)]
pub(crate) const DECIMAL_MARKS: &[(f64, &str, &str)] = &[
    (1500.0, "en", "about 1.5 l"),
    (1500.0, "fr", "environ 1,5 l"),
    (1500.0, "es", "aprox. 1,5 l"),
    (1500.0, "nl", "ongeveer 1,5 l"),
    (2000.0, "nl", "ongeveer 2 l"),
];

/// `4½`, because that is how the cups in the drawer are marked. Public so the
/// Crouton importer writes a rebuilt line's amount the way this module reads
/// it back (#69).
pub fn as_fraction(quantity: f64) -> String {
    let whole = quantity.trunc();
    let part = quantity - whole;
    let glyph = FRACTIONS
        .iter()
        .find(|(_, value)| (value - part).abs() < 0.005)
        .map(|(glyph, _)| *glyph);
    match glyph {
        Some(glyph) if whole == 0.0 => glyph.to_string(),
        Some(glyph) => format!("{whole:.0}{glyph}"),
        None => as_decimal(quantity),
    }
}

/// An amount as a cook writes it before `unit`, an app's export having
/// kept it as a number (#415, #416): metric as a decimal, anything else as the
/// fraction on the cup where there is one. A decimal keeps three places, as
/// Tandoor shows them: `0.375 kg`, where [`as_decimal`] would make it `0.38`.
/// It writes the very lines a Language is read from, so it knows none, and a
/// Unit it does not recognise is written as a fraction the reader reads back.
pub fn amount_for(amount: f64, unit: Option<&str>) -> String {
    let metric = unit
        .and_then(recognise)
        .is_some_and(|unit| unit.system == System::Metric);
    if !metric {
        let fraction = as_fraction(amount);
        if !fraction.is_ascii() {
            return fraction;
        }
    }
    let rendered = format!("{amount:.3}");
    rendered
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// A plain number with no trailing zeroes — `250`, `1.5`, never `1.50`.
pub fn as_decimal(quantity: f64) -> String {
    let rendered = format!("{quantity:.2}");
    rendered
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

// --- Temperatures in Step text -----------------------------------------------

/// The conventional oven ladder, never arithmetic (ADR 0016). 350°F becomes
/// 180°C because that is the number on the dial; 400°F becomes 200°C for the
/// same reason, where the arithmetic would offer 205°C and send the cook back
/// to make the judgement Kamosu was there to make.
///
/// The corpus contradicts itself here — `180C/350F` sits alongside
/// `350°F (175°C)`, four authors and two answers for one temperature — and that
/// disagreement is itself the evidence that no arithmetic result is expected.
const OVEN_LADDER: &[(i32, i32)] = &[
    (200, 95),
    (225, 110),
    (250, 120),
    (275, 140),
    (300, 150),
    (325, 160),
    (350, 180),
    (375, 190),
    (400, 200),
    (425, 220),
    (450, 230),
    (475, 245),
    (500, 260),
    // A pizza at the oven's hottest (#698).
    (525, 275),
    (550, 290),
];

/// The other direction, and a table of its own rather than [`OVEN_LADDER`] read
/// backwards. Each rung is a setting a metric oven actually has, paired with the
/// American dial number nearest to it — so a French recipe's `170 °C` and
/// `210 °C`, neither of which appears anywhere in the American ladder, still
/// answer with a number a US cook can set.
///
/// This is still the ladder rather than the arithmetic, which is the whole of
/// ADR 0016's rule: every answer here is a real dial position, and none of them
/// is the 338°F or 410°F the conversion would otherwise hand back.
const METRIC_LADDER: &[(i32, i32)] = &[
    (90, 200),
    (100, 200),
    (110, 225),
    (120, 250),
    (130, 275),
    (140, 275),
    (150, 300),
    (160, 325),
    (170, 350),
    (180, 350),
    (190, 375),
    (200, 400),
    (210, 400),
    (220, 425),
    (230, 450),
    (240, 475),
    (250, 475),
    (260, 500),
    // The top of a metric dial, where a pizza is baked (#698).
    (270, 525),
    (280, 550),
    (290, 550),
];

/// A temperature found in a Step, and which system it was written in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Scale {
    Fahrenheit,
    Celsius,
}

/// **The conversion offered beside a Step**, or nothing — an addition beside the
/// sentence and never written into it (GLOSSARY.md: a Step's truth is its text).
///
/// Nothing is the answer where the step carries no temperature, where it
/// already carries both — which 20 of the 39 real steps with a temperature do —
/// and where the reader asked for no conversion.
///
/// `written_in` is the system the recipe's own Readings are in, where they all
/// agree ([`written_in`]). It is what decides a bare `180°`, which could be
/// either dial.
#[cfg(test)]
pub fn step_temperature(
    text: &str,
    measures: Measures,
    language: &str,
    written_in: Option<System>,
) -> Option<StepTemperature> {
    step_temperature_in(UNKNOWN, text, measures, language, written_in)
}

/// [`step_temperature`], for a Step written in `written_in_language`.
pub fn step_temperature_in(
    written_in_language: &str,
    text: &str,
    measures: Measures,
    language: &str,
    written_in: Option<System>,
) -> Option<StepTemperature> {
    let wanted = match measures {
        Measures::Metric => Scale::Celsius,
        Measures::Us => Scale::Fahrenheit,
        Measures::AsWritten => return None,
    };
    let found = temperatures_written(words::of(written_in_language), text, written_in);
    if found.is_empty() {
        return None;
    }
    // Already carrying both: a recipe writer who dual-printed has said
    // everything there is to say, and an addition would repeat them.
    if found.iter().any(|temperature| temperature.scale == wanted) {
        return None;
    }
    // The first temperature that lands on a rung, rather than simply the first
    // number found. A French step can say `1 c. à s. d'huile ... à 180 °C`, and
    // the `1 c` there is a spoonful — the ladder is what tells the two apart,
    // because no oven is set to one degree.
    let (written, converted) = found.iter().find_map(|temperature| {
        let converted = match (temperature.scale, wanted) {
            (Scale::Fahrenheit, Scale::Celsius) => ladder_from_fahrenheit(temperature.degrees),
            (Scale::Celsius, Scale::Fahrenheit) => ladder_from_celsius(temperature.degrees),
            _ => None,
        }?;
        Some((temperature, converted))
    })?;
    let symbol = match wanted {
        Scale::Celsius => "°C",
        Scale::Fahrenheit => "°F",
    };
    Some(StepTemperature {
        start: written.start,
        end: written.end,
        measured: format!("{} {converted} {symbol}", about(language)),
    })
}

/// The oven a Step offers in the other system, and where in the Step's text
/// the temperature it converts was written, in bytes, so the addition can sit
/// straight after it (#150).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTemperature {
    pub start: usize,
    pub end: usize,
    pub measured: String,
}

impl StepTemperature {
    /// The temperature exactly as the Step wrote it.
    pub fn written<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

/// One temperature written in a Step: how many degrees, on which dial, and
/// where in the text, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FoundTemperature {
    degrees: i32,
    scale: Scale,
    start: usize,
    end: usize,
}

/// Read an American dial and answer with the metric one. The rungs are 25°F
/// apart, so a temperature more than 12°F from one was never a dial setting —
/// a 700°F pizza oven is not something this table can honestly answer, and
/// silence beats a number nobody can set.
fn ladder_from_fahrenheit(degrees: i32) -> Option<i32> {
    nearest_rung(OVEN_LADDER, degrees, 12, |(fahrenheit, _)| *fahrenheit)
        .map(|(_, celsius)| *celsius)
}

/// Read a metric dial and answer with the American one, off [`METRIC_LADDER`]
/// rather than by reversing [`OVEN_LADDER`]. Reversing it leaves holes exactly
/// where French recipes live: its Celsius column is the *American* ladder
/// wearing metric numbers, and it has no 170 and no 210 — two perfectly
/// ordinary settings on an oven sold in France.
fn ladder_from_celsius(degrees: i32) -> Option<i32> {
    nearest_rung(METRIC_LADDER, degrees, 5, |(celsius, _)| *celsius)
        .map(|(_, fahrenheit)| *fahrenheit)
}

/// The rung within `tolerance` of the temperature written, or nothing.
fn nearest_rung(
    ladder: &'static [(i32, i32)],
    degrees: i32,
    tolerance: i32,
    of: impl Fn(&(i32, i32)) -> i32,
) -> Option<&'static (i32, i32)> {
    ladder
        .iter()
        .min_by_key(|rung| (of(rung) - degrees).abs())
        .filter(|rung| (of(rung) - degrees).abs() <= tolerance)
}

/// **Which system a recipe is written in**, read off the Units of its own
/// Readings: the one they all agree on, or nothing where they disagree or there
/// are none.
///
/// Spoons cast no vote. A French recipe measures its oil in `c. à s.` beside
/// its grams, and Kamosu files every spoon as customary, so counting them would
/// call half the French library mixed.
#[cfg(test)]
pub fn written_in<'a>(units: impl IntoIterator<Item = &'a str>) -> Option<System> {
    written_in_words(words::of(UNKNOWN), units)
}

/// [`written_in`], for Units read with `lists`.
pub fn written_in_words<'a>(
    lists: &Words,
    units: impl IntoIterator<Item = &'a str>,
) -> Option<System> {
    let mut systems = units
        .into_iter()
        .filter_map(|unit| lists.unit(unit))
        .filter(|unit| !unit.is_spoon())
        .map(|unit| unit.system);
    let first = systems.next()?;
    systems.all(|system| system == first).then_some(first)
}

/// Every temperature in a Step's text, in the order they were written.
///
/// Without a degree sign a temperature needs its letter. `turn 90 degrees` and
/// `gas 5` both appear in the real corpus and neither is a temperature.
///
/// A degree sign with no letter is how Bon Appétit and most American magazines
/// print every oven (#150), so it counts where only one dial fits: see
/// [`bare_degrees`].
#[cfg(test)]
fn temperatures_in(text: &str, written_in: Option<System>) -> Vec<FoundTemperature> {
    temperatures_written(words::of(UNKNOWN), text, written_in)
}

/// [`temperatures_in`], read with `lists`.
fn temperatures_written(
    lists: &Words,
    text: &str,
    written_in: Option<System>,
) -> Vec<FoundTemperature> {
    // Dutch sites type a temperature with the ordinal sign as often as the
    // degree sign: `160 ºC`. The two are as long as each other in bytes.
    let characters: Vec<char> = text
        .chars()
        .map(|c| {
            if lists.is_dutch() && c == 'º' {
                '°'
            } else {
                c
            }
        })
        .collect();
    // Where each character starts in bytes, and the text's end after the last.
    let bytes: Vec<usize> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain([text.len()])
        .collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if !characters[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < characters.len() && characters[index].is_ascii_digit() {
            index += 1;
        }
        let digits: String = characters[start..index].iter().collect();
        let Ok(degrees) = digits.parse::<i32>() else {
            continue;
        };
        let marked = scale_after(lists, &characters, index)
            .or_else(|| {
                let end = degree_sign_after(&characters, index)?;
                let written_in = written_in.or(lists.is_dutch().then_some(System::Metric));
                bare_degrees(degrees, written_in).map(|scale| (scale, end))
            })
            .or_else(|| dutch_degrees(lists, &characters, index, degrees));
        if let Some((scale, end)) = marked {
            found.push(FoundTemperature {
                degrees,
                scale,
                start: bytes[start],
                end: bytes[end],
            });
        }
    }
    found
}

/// **A Dutch `180 graden`**, with no `C` or `F` after it, which is Celsius, as
/// a Dutch oven's dial is. It has to be an oven: `draai het deeg 90 graden` is
/// no temperature, and a rung of [`METRIC_LADDER`] from 100 up is.
fn dutch_degrees(
    lists: &Words,
    characters: &[char],
    index: usize,
    degrees: i32,
) -> Option<(Scale, usize)> {
    if !lists.is_dutch() || degrees < 100 {
        return None;
    }
    ladder_from_celsius(degrees)?;
    let mut at = index;
    while characters.get(at) == Some(&' ') {
        at += 1;
    }
    let word: String = characters[at..]
        .iter()
        .take_while(|c| c.is_alphabetic())
        .collect();
    lists
        .degrees
        .has(&word.to_lowercase())
        .then_some((Scale::Celsius, at + word.chars().count()))
}

/// Where a degree sign following a number ends, spaces allowed between, or
/// nothing where there is none.
fn degree_sign_after(characters: &[char], mut index: usize) -> Option<usize> {
    while characters.get(index) == Some(&' ') {
        index += 1;
    }
    (characters.get(index) == Some(&'°')).then_some(index + 1)
}

/// **Which dial a bare `425°` is on**, or nothing.
///
/// - Above 300 it can only be Fahrenheit: no domestic oven goes that high in
///   Celsius. It still has to be a rung of [`OVEN_LADDER`] to count.
/// - From 150 to 300 it is a setting on both dials — `180°`, `200°`, `220°` —
///   and the recipe's own Readings decide: all cups and pounds is an American
///   oven, all grams a metric one, and anything else is left alone.
/// - Below 150 it is nothing. The one bare degree sign in the 599 Steps of the
///   Crouton export is a Thermomix's `100°`, which is not an oven at all.
fn bare_degrees(degrees: i32, written_in: Option<System>) -> Option<Scale> {
    match degrees {
        301.. => ladder_from_fahrenheit(degrees).map(|_| Scale::Fahrenheit),
        150..=300 => match written_in? {
            System::Customary => Some(Scale::Fahrenheit),
            System::Metric => Some(Scale::Celsius),
        },
        _ => None,
    }
}

/// What follows a number, where what follows it is a temperature marker:
/// optional spaces, an optional degree sign, then `C`, `F`, or the word
/// `degrees` followed by one of those. Answers the scale and where the marker
/// ends, the whole word where the letter is spelled out (`Celsius`).
fn scale_after(lists: &Words, characters: &[char], mut index: usize) -> Option<(Scale, usize)> {
    let skip_spaces = |index: &mut usize| {
        while characters.get(*index).is_some_and(|c| *c == ' ') {
            *index += 1;
        }
    };
    skip_spaces(&mut index);
    // `350 degrees F`, and the French and Spanish words for it.
    for word in lists.degrees.iter() {
        let letters: Vec<char> = word.chars().collect();
        if characters[index..].starts_with(&letters[..]) {
            index += letters.len();
            skip_spaces(&mut index);
            break;
        }
    }
    if characters.get(index) == Some(&'°') {
        index += 1;
        skip_spaces(&mut index);
    }
    let letter = characters.get(index)?;
    let scale = match letter {
        'C' | 'c' => Scale::Celsius,
        'F' | 'f' => Scale::Fahrenheit,
        _ => return None,
    };
    // `Celsius` and `centigrade` are the letter spelled out; `cook`, `cup` and
    // `for` are not temperatures, so the letter must not begin another word.
    let next = characters.get(index + 1);
    let ends_here = next.is_none_or(|c| !c.is_alphanumeric());
    let spelled_out = ["celsius", "centigrade", "fahrenheit"].iter().any(|word| {
        let letters: Vec<char> = word.chars().collect();
        characters[index..]
            .iter()
            .map(|c| c.to_ascii_lowercase())
            .take(letters.len())
            .eq(letters.iter().copied())
    });
    let mut end = index + 1;
    while spelled_out && characters.get(end).is_some_and(|c| c.is_alphabetic()) {
        end += 1;
    }
    (ends_here || spelled_out).then_some((scale, end))
}

// --- Durations in Step text --------------------------------------------------

/// The denominators a written fraction of an hour uses — *1/2 hour*,
/// *3/4 hour*, *1 1/8 hours* — the common cooking ones and no others. A slash
/// between two numbers is otherwise a range (*cuire 20/25 minutes*), which is
/// how French writes one.
///
/// **Only an hour is written as a fraction** (#157, chosen there). Before
/// minutes the slash stays a range, because *cuire 3/4 minutes* is French for
/// three to four, and a written fraction of a minute is rare where that range
/// is common.
const DENOMINATORS: &[f64] = &[2.0, 3.0, 4.0, 8.0];

/// One piece of a Step's text, as the duration reader sees it. Whitespace
/// produces nothing at all, so `20-30 min` and `20 - 30 min` are one shape.
enum Piece {
    Number(f64),
    Word(String),
    /// A dash: the punctuation half of a range.
    Dash,
    /// A slash, which is a range too (*20/25 minutes*) unless the two numbers
    /// either side of it read as a fraction (*1/2 hour*).
    Slash,
    /// A mark that ends a clause: a comma, a bracket, a full stop.
    Stop,
    /// Anything else, such as an apostrophe. Present rather than skipped, so
    /// that a range's two ends must genuinely be adjacent.
    Other,
}

/// [`fold`], with combining marks dropped as well — the same second, looser
/// rule `core::folded_for_search` runs on, and for the same reason: *à* and *a*
/// are one range word, and *heures* must read the same however the accent was
/// typed.
///
/// It is not a call to that function because the dependency runs the other way:
/// `core` uses `units`, never the reverse. What the two share is the rule, and
/// the rule is written down in both places rather than in neither.
pub(crate) fn fold_loosely(word: &str) -> String {
    use unicode_normalization::char::is_combining_mark;
    fold(word)
        .chars()
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

/// **The timer a Step offers**, in seconds, or nothing.
///
/// Read out of the Step's own text at display time and stored nowhere
/// (ADR 0011, GLOSSARY.md "Step"): nothing is typed beside the sentence and the
/// Step's truth stays its text. One tap starts it; Kamosu never writes it down.
///
/// Nothing is the answer for roughly three Steps in four — 425 of the 579 real
/// Steps in the real export carry no duration at all — and that is not a
/// failure. A Step with no timer simply offers none.
///
/// **A range answers with its lower end.** 154 real Steps carry a duration and
/// 64 of them are ranges: *simmer for 5-8 minutes*, *knead for 3–4 minutes*.
/// The lower end is the one a cook can act on, because a timer that goes off at
/// five minutes sends you to look at a pan that may need three more, while one
/// that goes off at eight has already let it burn.
///
/// **The first duration in the text wins**, not the largest and not the last.
/// A Step reads in order and the timer belongs to the thing it is telling you
/// to do now.
///
/// **A duration is everything written against its unit**, not only the number
/// before it (#157): *2 minutes 30*, *2 min 30 s*, *2 heures et demie*, *2 and
/// a half minutes*, *1 1/2 hours* and *une demi-heure* each read whole. A sear
/// timer that drops the half minute is a fifth short.
pub fn step_duration(text: &str, language: &str) -> Option<i64> {
    let lists = words::of(language);
    let pieces = tokenise(text);
    for (index, piece) in pieces.iter().enumerate() {
        let Piece::Word(word) = piece else { continue };
        let Some(unit_seconds) = lists.seconds_each(word) else {
            continue;
        };
        let Some((start, written)) = count_before(lists, &pieces, index, unit_seconds) else {
            continue;
        };
        let quantity = lower_end(lists, &pieces, start, written).unwrap_or(written);
        if waited_already(lists, &pieces, start) {
            continue;
        }
        let seconds =
            quantity * unit_seconds as f64 + rest_after_unit(lists, &pieces, index, unit_seconds);
        if !(1.0..=LONGEST_TIMER_SECONDS as f64).contains(&seconds) {
            continue;
        }
        return Some(seconds.round() as i64);
    }
    None
}

/// **Whether the clause puts this duration in the past** (#698): the wait it
/// names happened before this Step, so a timer for it would send the cook into
/// a second one. The pizza dough's *After the total fermentation time of 90
/// minutes, turn the dough out* names the two rests before it.
///
/// The clause is the pieces back to the last [`Piece::Stop`], a comma, a
/// bracket or a full stop. It is in the past where it opens with one of
/// [`Words::past_wait_opens`] (*after*, *once*, *when*) and then either says
/// the wait is done, with one of [`Words::past_wait_done`] at most two words
/// before the number (*once it **has** rested for 2 hours*), or names the wait
/// as a thing right before the number, one of [`Words::past_wait_names`] and
/// then one of [`Words::past_wait_of`] (*the fermentation **time of** 90
/// minutes*). So *when the water has boiled add the pasta and cook for 10
/// minutes*, *after a total of 10 minutes* and *after 10 minutes* each keep
/// their timer.
fn waited_already(lists: &Words, pieces: &[Piece], start: usize) -> bool {
    let clause_start = pieces[..start]
        .iter()
        .rposition(|piece| matches!(piece, Piece::Stop))
        .map_or(0, |stop| stop + 1);
    let clause = &pieces[clause_start..start];
    let word = |piece: &Piece| match piece {
        Piece::Word(word) => Some(word.clone()),
        _ => None,
    };
    let Some(opens) = clause
        .iter()
        .position(|piece| word(piece).is_some_and(|w| lists.past_wait_opens.has(&w)))
    else {
        return false;
    };
    let after_opening = &clause[opens + 1..];
    let done = after_opening
        .iter()
        .rev()
        .take(3)
        .any(|piece| word(piece).is_some_and(|w| lists.past_wait_done.has(&w)));
    let named = match after_opening {
        [.., name, of] => {
            word(name).is_some_and(|w| lists.past_wait_names.has(&w))
                && word(of).is_some_and(|w| lists.past_wait_of.has(&w))
        }
        _ => false,
    };
    done || named
}

/// The longest duration this offers a timer for: a week.
///
/// Not a guess at what a recipe means — a 48-hour ferment and an overnight
/// marinade are real, and the longest in the real export is twelve hours. It
/// is a floor under nonsense: past a week the number was not a duration
/// somebody meant to time, and a countdown running for years is worse than no
/// timer at all.
const LONGEST_TIMER_SECONDS: i64 = 7 * 24 * 60 * 60;

/// What is written after a duration's unit and still belongs to it, in
/// seconds, or zero: the spoken half or quarter (*2 heures et demie*), a
/// written fraction (*2 heures 1/2*), or a number of the next smaller unit,
/// named (*1 hour 30 minutes*, *2 min 30 s*) or left unnamed (*1 h 30*,
/// *2 minutes 30*).
///
/// An unnamed number counts only under sixty, and only where nothing after it
/// claims it ([`claimed`]), so `2 hours 200 g of flour` cannot become three and
/// a bit hours. One under ten counts only where no word follows it at all:
/// *1 h 5.* is an hour and five minutes, but *cook 5 minutes 2 at a time* is a
/// count, and an hour's minutes and a sear's seconds are written in tens.
///
/// None of these is in the real 86-recipe export, which is why they are
/// handled from the language rather than from the corpus: `1 h 30` is how
/// French writes an hour and a half, and *2 minutes 30* how it writes a sear.
fn rest_after_unit(lists: &Words, pieces: &[Piece], unit: usize, unit_seconds: i64) -> f64 {
    let fraction = || fraction_after(pieces, unit).filter(|_| unit_seconds == 3600);
    if let Some((end, part)) = spoken_part_after(lists, pieces, unit).or_else(fraction) {
        return if claimed(lists, pieces, end) {
            0.0
        } else {
            part * unit_seconds as f64
        };
    }
    let smaller = match unit_seconds {
        3600 => 60,
        60 => 1,
        _ => return 0.0,
    };
    let Some(Piece::Number(count)) = pieces.get(unit + 1) else {
        return 0.0;
    };
    if !(1.0..60.0).contains(count) {
        return 0.0;
    }
    let next = pieces.get(unit + 2);
    let named = matches!(next, Some(Piece::Word(word))
        if lists.seconds_each(word) == Some(smaller) || (smaller == 1 && word == "s"));
    let unnamed = !claimed(lists, pieces, unit + 2)
        && (*count >= 10.0 || !matches!(next, Some(Piece::Word(_))));
    if named || unnamed {
        count * smaller as f64
    } else {
        0.0
    }
}

/// Whether what sits at `after` takes the number just before it as its own
/// quantity — `30 g`, `3 times`, `35°C`, `1 1/2 cups` — rather than leaving it
/// to the duration. A range is looked past to what names both its ends:
/// *3 or 4 times* is a count, *1 h 30 - 2 h* a range of durations.
fn claimed(lists: &Words, pieces: &[Piece], after: usize) -> bool {
    if joins_a_range(lists, pieces.get(after))
        && matches!(pieces.get(after + 1), Some(Piece::Number(_)))
    {
        return !matches!(pieces.get(after + 2), Some(Piece::Word(word))
            if lists.seconds_each(word).is_some());
    }
    match pieces.get(after) {
        Some(Piece::Number(_)) => true,
        Some(Piece::Word(word)) => !lists.timer_joins.has(word),
        _ => false,
    }
}

/// How many of this duration word the text counts, and the piece where that
/// count begins: `2`, `3 more`, `2 and a half`, `1/2` and `1 1/2` of an hour,
/// or a bare `half an` / `demi-` / `media` with no number at all.
fn count_before(
    lists: &Words,
    pieces: &[Piece],
    unit: usize,
    unit_seconds: i64,
) -> Option<(usize, f64)> {
    let spoken = spoken_part_before(lists, pieces, unit);
    let at = match spoken {
        Some((start, _)) => start.checked_sub(1),
        None => unit.checked_sub(1).and_then(|at| match pieces.get(at) {
            Some(Piece::Word(word)) if lists.timer_more.has(word) => at.checked_sub(1),
            _ => Some(at),
        }),
    };
    let part = spoken.map_or(0.0, |(_, part)| part);
    match at.map(|at| (at, pieces.get(at))) {
        // *Een uur of 2* is about two hours, which no timer can say.
        Some((_, Some(Piece::Word(word))))
            if lists.timer_one.has(word) && joins_a_range(lists, pieces.get(unit + 1)) =>
        {
            None
        }
        Some((at, Some(Piece::Word(word)))) if lists.timer_one.has(word) => Some((at, 1.0 + part)),
        Some((at, Some(Piece::Word(word)))) if lists.timer_one_and_a_half.has(word) => {
            Some((at, 1.5 + part))
        }
        Some((at, Some(Piece::Number(number)))) => Some(
            fraction_ending_at(pieces, at)
                .filter(|_| unit_seconds == 3600)
                .map_or((at, number + part), |(start, fraction)| {
                    (start, fraction + part)
                }),
        ),
        _ => half_alone(lists, pieces, unit).map(|start| (start, 0.5)),
    }
}

/// A written fraction whose denominator is the number at `at` — `1/2`, `3/4`,
/// and `1 1/2` with its whole number — and where it begins.
fn fraction_ending_at(pieces: &[Piece], at: usize) -> Option<(usize, f64)> {
    let top_at = at.checked_sub(2)?;
    let (Some(Piece::Number(top)), Some(Piece::Slash), Some(Piece::Number(bottom))) =
        (pieces.get(top_at), pieces.get(top_at + 1), pieces.get(at))
    else {
        return None;
    };
    let fraction = written_fraction(*top, *bottom)?;
    if let Some(whole_at) = top_at.checked_sub(1)
        && let Some(Piece::Number(whole)) = pieces.get(whole_at)
        && whole.fract() == 0.0
        && *whole >= 1.0
    {
        return Some((whole_at, whole + fraction));
    }
    Some((top_at, fraction))
}

/// A written fraction right after the duration word at `unit` — *2 heures
/// 1/2*, which is one way French writes two and a half hours — as the piece
/// after it and what it is worth.
fn fraction_after(pieces: &[Piece], unit: usize) -> Option<(usize, f64)> {
    let (Some(Piece::Number(top)), Some(Piece::Slash), Some(Piece::Number(bottom))) = (
        pieces.get(unit + 1),
        pieces.get(unit + 2),
        pieces.get(unit + 3),
    ) else {
        return None;
    };
    Some((unit + 4, written_fraction(*top, *bottom)?))
}

/// What `top/bottom` is worth, where the slash reads as a fraction rather than
/// a range: a whole numerator smaller than a denominator from [`DENOMINATORS`],
/// in lowest terms. Nobody writes a fraction unreduced, so `2/4 heures` is the
/// French *2 to 4*, not half an hour.
fn written_fraction(top: f64, bottom: f64) -> Option<f64> {
    let reduced = top % 2.0 != 0.0 || bottom % 2.0 != 0.0;
    (top.fract() == 0.0 && top >= 1.0 && top < bottom && DENOMINATORS.contains(&bottom) && reduced)
        .then_some(top / bottom)
}

/// Where a half with no number before it begins — *une demi-heure*, *media
/// hora*, *half an hour* — right against the duration word at `unit`.
fn half_alone(lists: &Words, pieces: &[Piece], unit: usize) -> Option<usize> {
    let before = unit.checked_sub(1)?;
    if matches!(pieces.get(before), Some(Piece::Word(word)) if lists.half_words.has(word)) {
        return Some(before);
    }
    let start = before.checked_sub(1)?;
    let hyphenated = matches!(pieces.get(before), Some(Piece::Dash))
        && lists
            .half_hyphened
            .iter()
            .any(|half| spells(pieces, start, half));
    let with_article = lists
        .half_with_article
        .iter()
        .any(|half| spells(pieces, start, half));
    (hyphenated || with_article).then_some(start)
}

/// The half and the quarter a cook says rather than writes as a number, and
/// what each adds, as a share of the unit it sits against.
fn spoken_parts(lists: &Words) -> impl Iterator<Item = (&[String], f64)> {
    let halves = lists.spoken_half.iter().map(|phrase| (phrase, 0.5));
    let quarters = lists.spoken_quarter.iter().map(|phrase| (phrase, 0.25));
    halves.chain(quarters)
}

/// The spoken half or quarter written directly after the duration word at
/// `unit`, as the piece after it and the share it adds.
fn spoken_part_after(lists: &Words, pieces: &[Piece], unit: usize) -> Option<(usize, f64)> {
    spoken_parts(lists).find_map(|(words, part)| {
        spells(pieces, unit + 1, words).then_some((unit + 1 + words.len(), part))
    })
}

/// The spoken half or quarter written between a number and the duration word
/// at `unit` — *2 and a half minutes* — as where it begins and the share it adds.
fn spoken_part_before(lists: &Words, pieces: &[Piece], unit: usize) -> Option<(usize, f64)> {
    spoken_parts(lists).find_map(|(words, part)| {
        let start = unit.checked_sub(words.len())?;
        spells(pieces, start, words).then_some((start, part))
    })
}

/// Whether the pieces from `start` are exactly these words, in order.
fn spells(pieces: &[Piece], start: usize, words: &[String]) -> bool {
    words.iter().enumerate().all(|(offset, expected)| {
        matches!(pieces.get(start + offset), Some(Piece::Word(word)) if word == expected)
    })
}

/// Whether this piece joins the two ends of a range: a dash, a slash, or one
/// of [`Words::timer_ranges`].
fn joins_a_range(lists: &Words, piece: Option<&Piece>) -> bool {
    match piece {
        Some(Piece::Dash | Piece::Slash) => true,
        Some(Piece::Word(word)) => lists.timer_ranges.has(word),
        _ => false,
    }
}

/// The lower end of a range whose upper end, worth `upper`, begins at the
/// piece `start`, where the two pieces before it are a range's other half —
/// `5`, `-`, `8` or `1`, `to`, `3`. Nothing where this is a plain duration, and
/// nothing where the supposed lower end is not actually lower, since
/// `30-20 minutes` is not a range anybody wrote.
fn lower_end(lists: &Words, pieces: &[Piece], start: usize, upper: f64) -> Option<f64> {
    if !joins_a_range(lists, pieces.get(start.checked_sub(1)?)) {
        return None;
    }
    match pieces.get(start.checked_sub(2)?)? {
        Piece::Number(lower) if *lower > 0.0 && *lower < upper => Some(*lower),
        _ => None,
    }
}

/// Split a Step's text into the pieces the duration reader compares. Numbers
/// take the French decimal comma the same way [`parse_amount`] does, and a
/// vulgar fraction written against a number — `1½ hours` — is added to it.
fn tokenise(text: &str) -> Vec<Piece> {
    let characters: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() {
            index += 1;
        } else if character.is_ascii_digit() {
            let start = index;
            while index < characters.len()
                && (characters[index].is_ascii_digit()
                    || ((characters[index] == '.' || characters[index] == ',')
                        && characters.get(index + 1).is_some_and(char::is_ascii_digit)))
            {
                index += 1;
            }
            let digits: String = characters[start..index].iter().collect();
            let mut number = parse_plain(&digits).unwrap_or(0.0);
            if let Some(fraction) = characters
                .get(index)
                .and_then(|c| FRACTIONS.iter().find(|(glyph, _)| glyph == c))
            {
                number += fraction.1;
                index += 1;
            }
            pieces.push(Piece::Number(number));
        } else if character.is_alphabetic() {
            let start = index;
            while index < characters.len() && characters[index].is_alphabetic() {
                index += 1;
            }
            let word: String = characters[start..index].iter().collect();
            pieces.push(Piece::Word(fold_loosely(&word)));
        } else {
            index += 1;
            pieces.push(match character {
                '-' | '\u{2013}' | '\u{2014}' => Piece::Dash,
                '/' => Piece::Slash,
                // A sign that names what the number before it measures, as a
                // word would: the 35 in `30 minutes 35°C` is a temperature.
                '°' | '%' => Piece::Word(character.to_string()),
                ',' | '.' | ';' | ':' | '!' | '?' | '(' | ')' => Piece::Stop,
                _ => Piece::Other,
            });
        }
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(text: &str, written_in: Option<System>) -> Option<String> {
        step_temperature(text, Measures::Metric, "en", written_in).map(|found| found.measured)
    }

    fn us(text: &str, written_in: Option<System>) -> Option<String> {
        step_temperature(text, Measures::Us, "en", written_in).map(|found| found.measured)
    }

    /// The temperature as the Step wrote it, which the addition sits after.
    fn written(text: &str, measures: Measures) -> Option<&str> {
        step_temperature(text, measures, "en", None).map(|found| found.written(text))
    }

    #[test]
    fn a_temperature_answers_where_it_was_written() {
        let parm = "Place a rack in lower third of oven; preheat to 425°. Place 1 lb. chicken.";
        assert_eq!(written(parm, Measures::Metric), Some("425°"));
        assert_eq!(
            written("Preheat the oven to 350°F.", Measures::Metric),
            Some("350°F")
        );
        assert_eq!(
            written("Raise it to 400 degrees F.", Measures::Metric),
            Some("400 degrees F")
        );
        assert_eq!(
            written("Préchauffer à 180 °C.", Measures::Us),
            Some("180 °C")
        );
        assert_eq!(
            written("Bake at 190 Celsius, fan on.", Measures::Us),
            Some("190 Celsius")
        );
    }

    #[test]
    fn a_bare_degree_sign_above_any_celsius_oven_is_fahrenheit() {
        // Bon Appétit's chicken parm (#150), whatever the recipe's other units.
        for written_in in [None, Some(System::Metric), Some(System::Customary)] {
            assert_eq!(
                metric(
                    "Arrange a rack in center of oven; preheat to 425°.",
                    written_in
                ),
                Some("about 220 °C".to_string()),
            );
        }
        assert_eq!(
            metric("Bake at 375° for 20 minutes.", None),
            Some("about 190 °C".into())
        );
        // Above 300 but on no rung: silence beats a number nobody can set.
        assert_eq!(metric("A pizza oven reaches 700°.", None), None);
    }

    #[test]
    fn the_hottest_home_oven_is_on_both_ladders() {
        // The Sample Library's pizza, step 12 (#698).
        let pizza = "Bake at the oven's hottest, 275 °C here: 12 to 14 minutes for the pan pizza, 11 to 12 on a stone or steel.";
        assert_eq!(us(pizza, None), Some("about 525 °F".into()));
        assert_eq!(us("Bake at 285°C.", None), Some("about 550 °F".into()));
        assert_eq!(
            metric("Bake in a preheated oven at 550°F.", None),
            Some("about 290 °C".into())
        );
    }

    #[test]
    fn a_bare_degree_sign_both_dials_have_is_read_by_the_recipes_own_units() {
        // No Readings, or Readings that disagree: no telling which oven.
        assert_eq!(metric("Bake at 180°.", None), None);
        assert_eq!(us("Bake at 180°.", None), None);
        // All grams: a metric oven, so an American is offered her dial.
        assert_eq!(
            us("Bake at 180°.", Some(System::Metric)),
            Some("about 350 °F".into())
        );
        assert_eq!(metric("Bake at 180°.", Some(System::Metric)), None);
        // All cups: an American oven. 180°F is below every rung, so nothing
        // is offered either way — and certainly not the 350 °F it would be as
        // Celsius. Which dial it was read on is asked directly, since silence
        // alone cannot tell the two apart.
        assert_eq!(us("Bake at 180°.", Some(System::Customary)), None);
        assert_eq!(metric("Bake at 180°.", Some(System::Customary)), None);
        let scales = |written_in| {
            temperatures_in("Bake at 180°.", written_in)
                .iter()
                .map(|found| (found.degrees, found.scale))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            scales(Some(System::Customary)),
            vec![(180, Scale::Fahrenheit)]
        );
        assert_eq!(scales(Some(System::Metric)), vec![(180, Scale::Celsius)]);
        assert_eq!(scales(None), vec![]);
        // 200° is a rung on both dials, and answers each way round.
        assert_eq!(
            metric("Keep warm at 200°.", Some(System::Customary)),
            Some("about 95 °C".into())
        );
        assert_eq!(
            us("Keep warm at 200°.", Some(System::Metric)),
            Some("about 400 °F".into())
        );
    }

    #[test]
    fn what_is_not_an_oven_stays_silent() {
        for written_in in [None, Some(System::Metric), Some(System::Customary)] {
            for text in [
                "Turn the tray 90 degrees.",
                "Cook on gas 5 for 20 minutes.",
                // The one bare degree sign in the Crouton export: a Thermomix.
                "Régler 25 min / 100°, vitesse 1.",
            ] {
                assert_eq!(metric(text, written_in), None, "{text}");
                assert_eq!(us(text, written_in), None, "{text}");
            }
        }
    }

    #[test]
    fn a_recipe_is_written_in_the_system_its_units_agree_on() {
        assert_eq!(written_in(["lb", "oz", "cup"]), Some(System::Customary));
        assert_eq!(written_in(["g", "ml", "kg"]), Some(System::Metric));
        // Spoons are in every kitchen, and cast no vote.
        assert_eq!(written_in(["g", "c. à s.", "tsp"]), Some(System::Metric));
        assert_eq!(written_in(["g", "cup"]), None);
        assert_eq!(written_in(["tbsp", "clove"]), None);
        assert_eq!(written_in([]), None);
    }

    #[test]
    fn a_cup_goes_plural_where_each_language_says_it_does() {
        for &(cups, language, worded) in CUP_PLURALS {
            assert_eq!(
                worded_volume(cups_in_millilitres(cups), Measures::Us, language),
                worded,
                "{cups} cups in {language}"
            );
        }
    }

    #[test]
    fn a_hundred_and_twenty_millilitres_is_half_a_cup_not_half_cups() {
        // The line that found this, from production (#147).
        assert_eq!(worded_volume(120.0, Measures::Us, "en"), "about ½ cup");
    }

    fn timer(text: &str) -> Option<i64> {
        step_duration(text, "nl")
    }

    #[test]
    fn a_dutch_timer_reads_the_way_a_dutch_recipe_writes_one() {
        for (text, seconds) in [
            ("Bak 30 minuten in de oven.", 1800),
            ("Laat 1 uur rusten.", 3600),
            ("Laat een kwartier staan.", 900),
            ("Laat een kwartiertje staan.", 900),
            ("Laat anderhalf uur pruttelen.", 5400),
            ("Bak 2 à 3 minuten.", 120),
            ("Bak 10 tot 15 minuten.", 600),
            ("Laat een half uur rusten.", 1800),
            ("Laat een half uurtje rusten.", 1800),
            ("Bak nog een minuut mee.", 60),
            ("Laat een uur rusten.", 3600),
            ("1 hr 30 mins", 5400),
            ("Laat er de ui een 10tal minuten zweten.", 600),
            ("Bak 10-15 minuutjes.", 600),
            ("Verwarm 30 seconden.", 30),
        ] {
            assert_eq!(timer(text), Some(seconds), "{text}");
        }
        for text in [
            "Bak een paar minuten.",
            "Roer een keer om.",
            "Voeg 2 el olie toe.",
            "Laat een minuut of 5 staan.",
        ] {
            assert_eq!(timer(text), None, "{text}");
        }
    }

    #[test]
    fn a_wait_the_step_says_is_over_offers_no_timer() {
        // The Sample Library's pizza dough, step 6: the 90 minutes are the
        // two rests before it, already waited (#698).
        for (text, language) in [
            (
                "After the total fermentation time of 90 minutes, turn the dough out onto a lightly floured surface.",
                "en",
            ),
            (
                "Once the dough has rested for 2 hours, shape it into a ball.",
                "en",
            ),
            (
                "When the onions have cooked for 10 minutes, add the garlic.",
                "en",
            ),
            (
                "Après le temps de repos de 90 minutes, dégazez la pâte.",
                "fr",
            ),
            (
                "Después del tiempo de reposo de 90 minutos, desgasifica la masa.",
                "es",
            ),
            ("Na de rijstijd van 90 minuten het deeg uitstorten.", "nl"),
        ] {
            assert_eq!(step_duration(text, language), None, "{text}");
        }
    }

    #[test]
    fn a_wait_that_starts_in_the_step_keeps_its_timer() {
        for (text, seconds) in [
            ("After 10 minutes, flip the steak.", 600),
            (
                "Cover and let sit at room temperature for 60 minutes.",
                3600,
            ),
            (
                "Cover the bowl with a lid and let the dough ferment on the counter for 30 minutes.",
                1800,
            ),
            // The past wait is passed over for the one still to come.
            (
                "Once the dough has rested for 2 hours, bake it for 20 minutes.",
                1200,
            ),
            // A clause of its own after the comma is a wait still to come.
            (
                "Once the dough has doubled, about 1 hour, punch it down.",
                3600,
            ),
            ("Après 10 minutes de cuisson, retournez la viande.", 600),
            // Found at review: a done wait far from the number is another
            // thing, and a total or a minimum is no named wait.
            (
                "When the water has boiled add the pasta and cook for 10 minutes.",
                600,
            ),
            ("Once you have added the eggs beat for 2 minutes.", 120),
            ("After a minimum of 30 minutes, punch down the dough.", 1800),
            ("After a total of 10 minutes, flip the steak.", 600),
        ] {
            let language = if text.starts_with("Après") {
                "fr"
            } else {
                "en"
            };
            assert_eq!(step_duration(text, language), Some(seconds), "{text}");
        }
    }

    #[test]
    fn dutch_timer_words_are_no_timer_in_another_language() {
        assert_eq!(step_duration("Laat een kwartier staan.", "en"), None);
        assert_eq!(step_duration("Laat 1 uur rusten.", "fr"), None);
    }

    fn dutch_oven(text: &str, measures: Measures) -> Option<String> {
        step_temperature_in("nl", text, measures, "en", None).map(|found| found.measured)
    }

    #[test]
    fn a_dutch_oven_needs_no_letter() {
        assert_eq!(
            dutch_oven("Verwarm de oven voor op 180 graden.", Measures::Us),
            Some("about 350 °F".into())
        );
        assert_eq!(
            dutch_oven("Bak op 200 graden boven- en onderwarmte.", Measures::Us),
            Some("about 400 °F".into())
        );
        assert_eq!(
            dutch_oven("Verwarm de oven voor op 200 °C.", Measures::Us),
            Some("about 400 °F".into())
        );
        assert_eq!(
            dutch_oven("Verwarm de oven voor op 160 ºC.", Measures::Us),
            Some("about 325 °F".into())
        );
        assert_eq!(
            step_temperature_in("nl", "Op 180 graden.", Measures::Us, "en", None)
                .map(|found| found.written("Op 180 graden.").to_string()),
            Some("180 graden".into())
        );
        // Already in a metric reader's measures, and no oven at all.
        assert_eq!(dutch_oven("Op 180 graden.", Measures::Metric), None);
        assert_eq!(dutch_oven("Draai het deeg 90 graden.", Measures::Us), None);
    }

    #[test]
    fn grados_with_no_letter_stay_no_temperature() {
        for language in ["es", UNKNOWN] {
            assert_eq!(
                step_temperature_in(language, "Hornear a 180 grados.", Measures::Us, "en", None),
                None
            );
        }
    }

    /// A scaled line reads whole (#680), its counted word in step with the
    /// new number where English has a rule for it.
    #[test]
    fn a_scaled_line_keeps_what_it_is_an_amount_of() {
        let line = |text: &str, amount, unit, target, scale| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                unit,
                target,
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(
            line(
                "2 tablespoons butter",
                "2",
                Some("tablespoons"),
                Some("butter"),
                0.5
            ),
            Some("1 tablespoon butter".into())
        );
        assert_eq!(
            line("1 tbsp oil", "1", Some("tbsp"), Some("oil"), 2.0),
            Some("2 tbsp oil".into()),
            "an abbreviation has no plural"
        );
        assert_eq!(
            line(
                "2 to 3 cloves garlic",
                "2 to 3",
                Some("cloves"),
                Some("garlic"),
                2.0
            ),
            Some("4-6 cloves garlic".into())
        );
        assert_eq!(
            line("1 pinch of salt", "1", Some("pinch"), Some("salt"), 3.0),
            Some("3 pinches of salt".into())
        );
        assert_eq!(
            line("2 large eggs, beaten", "2", None, Some("eggs"), 0.5),
            Some("1 large egg, beaten".into())
        );
        assert_eq!(
            line("1 tsp salt", "1", Some("tsp"), Some("salt"), 1.1),
            Some("about 1 tsp salt".into()),
            "rounded back to the written figure, and still said whole"
        );
        // A Reading corrected by hand to an amount the line does not write.
        assert_eq!(
            line(
                "a handful of basil",
                "1",
                Some("handful"),
                Some("basil"),
                2.0
            ),
            Some("2 handful basil".into())
        );
        assert_eq!(line("salt to taste", "some", None, Some("salt"), 2.0), None);

        // The Tomato soup's lines, at six servings for four (#680).
        assert_eq!(
            line("1 head of garlic", "1", Some("head"), Some("garlic"), 1.5),
            Some("1½ heads of garlic".into())
        );
        assert_eq!(
            line("1 yellow onion", "1", None, Some("yellow onion"), 1.5),
            Some("1½ yellow onions".into())
        );
        assert_eq!(
            line(
                "500-700 g cherry tomatoes",
                "500-700",
                Some("g"),
                Some("cherry tomatoes"),
                1.5
            ),
            Some("about 1¾-2¼ lb cherry tomatoes".into()),
            "a range converts at both ends for an American reader too"
        );

        // The words counted anew.
        for (text, target, scale, said) in [
            ("2 cookies", "cookies", 0.5, "1 cookie"),
            ("2 pies", "pies", 0.5, "1 pie"),
            ("2 mangoes", "mangoes", 0.5, "1 mango"),
            ("2 asparagus", "asparagus", 0.5, "1 asparagus"),
            ("1 tomato", "tomato", 2.0, "2 tomatoes"),
            ("1 cherry", "cherry", 2.0, "2 cherries"),
            ("2 eggs", "egg", 0.5, "1 egg"),
        ] {
            assert_eq!(
                line(
                    text,
                    text.split(' ').next().unwrap(),
                    None,
                    Some(target),
                    scale
                ),
                Some(said.to_string())
            );
        }
        // The amount is a number of its own, not the 1 inside 10.
        assert_eq!(
            line(
                "10 cm ginger, 1 piece",
                "1",
                Some("piece"),
                Some("ginger"),
                2.0
            ),
            Some("10 cm ginger, 2 pieces".into())
        );
    }

    /// A measure the line restates says the same amount scaled, so it goes
    /// rather than contradict the figure before it (#680). The size of an
    /// open Unit, `1 can (14 oz)`, is no restatement and stays.
    #[test]
    fn a_scaled_line_drops_a_measure_it_only_restated() {
        let metric = |text: &str, amount, unit, target| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                Some(unit),
                Some(target),
                2.0,
                Measures::Metric,
                "en",
                None,
            )
        };
        assert_eq!(
            metric("1 cup (240 ml) milk", "1", "cup", "milk"),
            Some("480 ml milk".into()),
            "the millilitres the line wrote, scaled (#698)"
        );
        assert_eq!(
            metric("400-450 g / 0.9-1 lb onions", "400-450", "g", "onions"),
            Some("800-900 g onions".into())
        );
        assert_eq!(
            metric(
                "8 oz. mozzarella, coarsely grated (about 1–1½ cups)",
                "8",
                "oz.",
                "mozzarella"
            ),
            Some("about 455 g mozzarella, coarsely grated".into())
        );
        assert_eq!(
            metric("2 (heaped) tbsp flour", "2", "tbsp", "flour"),
            Some("about 60 ml (heaped) flour".into()),
            "the Unit converted is said once"
        );
        assert_eq!(
            metric("1 can (14 oz) tomatoes", "1", "can", "tomatoes"),
            Some("2 cans (14 oz) tomatoes".into())
        );
        assert_eq!(
            metric("1 cup (plus 2 tbsp) milk", "1", "cup", "milk"),
            Some("about 470 ml (plus 2 tbsp) milk".into()),
            "a measure that adds to the amount is no restatement"
        );
    }

    /// The lines a Reading set by hand, or a bracket, could trip (#680).
    #[test]
    fn a_scaled_line_survives_what_a_cook_can_type() {
        let us = |text: &str, amount, unit, target: &str, scale| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                unit,
                Some(target),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(
            us("2 spring\u{a0}onions", "2", None, "spring\u{a0}onion", 0.5),
            Some("1 spring\u{a0}onion".into()),
            "a Food set by hand with a non-breaking space"
        );
        assert_eq!(
            us("(serves 4) 4 eggs", "4", None, "eggs", 0.5),
            Some("(serves 4) 2 eggs".into())
        );
        assert_eq!(
            us("1 Tomato", "1", None, "Tomato", 2.0),
            Some("2 Tomatoes".into())
        );
    }

    /// A figure too small for the reader's Unit keeps the Unit written, and
    /// no amount is ever said as nothing (#698): the pizza dough's 7 g of
    /// yeast halved was "about 0 oz".
    #[test]
    fn a_scaled_figure_never_says_nothing() {
        let short = |amount, unit, scale| {
            measured_line(Some(amount), Some(unit), scale, Measures::Us, "en", None)
        };
        assert_eq!(short("7", "g", 0.5), Some("about 4 g".into()));
        assert_eq!(short("12", "g", 0.25), Some("3 g".into()));
        assert_eq!(short("1/4", "tsp", 0.25), Some("about ⅛ tsp".into()));
        assert_eq!(short("1/2", "tsp", 0.125), Some("about ⅛ tsp".into()));
        // Big enough for an ounce, still in ounces.
        assert_eq!(short("7", "g", 2.0), Some("about ½ oz".into()));
    }

    /// A scaled American measure is said with the spoon, cup or weight that
    /// measures it, as the drawer holds them (#698): six teaspoons is two
    /// tablespoons. Exact arithmetic says no "about".
    #[test]
    fn a_scaled_measure_is_said_with_the_tool_that_measures_it() {
        let us = |text: &str, amount, unit, target, scale| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                Some(unit),
                Some(target),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        // Dan Dan Noodles, doubled.
        assert_eq!(
            us("3 tsp sesame paste", "3", "tsp", "sesame paste", 2.0),
            Some("2 tbsp sesame paste".into())
        );
        assert_eq!(
            us(
                "2 tbsp light soy sauce",
                "2",
                "tbsp",
                "light soy sauce",
                4.0
            ),
            Some("½ cup light soy sauce".into())
        );
        assert_eq!(
            us("3 tbsp chilli oil", "3", "tbsp", "chilli oil", 2.0),
            Some("¼ cup + 2 tbsp chilli oil".into()),
            "no set of cups has a ⅜ cup, found live on Dan Dan Noodles"
        );
        // Found at the second review: past a quarter cup the tablespoon is
        // still tried, a third of a cup is in every set, and both ends of a
        // range must land on a mark.
        assert_eq!(
            us("5 tsp salt", "5", "tsp", "salt", 3.0),
            Some("¼ cup + 1 tbsp salt".into())
        );
        assert_eq!(
            us("4 tsp sugar", "4", "tsp", "sugar", 4.0),
            Some("⅓ cup sugar".into())
        );
        assert_eq!(
            us("6 tbsp butter", "6", "tbsp", "butter", 3.0),
            Some("1 cup + 2 tbsp butter".into())
        );
        assert_eq!(
            us("5 3/4 c flour", "5 3/4", "c", "flour", 0.5),
            Some("2⅞ c flour".into()),
            "a cup the recipe wrote keeps its eighths"
        );
        assert_eq!(
            us("3-4 tbsp chilli oil", "3-4", "tbsp", "chilli oil", 2.0),
            Some("6-8 tbsp chilli oil".into())
        );
        assert_eq!(
            us("1 cup stock", "1", "cup", "stock", 1.0 / 3.0),
            Some("⅓ cup stock".into())
        );
        assert_eq!(
            us("1/4 cup yacai", "1/4", "cup", "yacai", 0.25),
            Some("1 tbsp yacai".into())
        );
        assert_eq!(
            us(
                "1/2 lbs ground pork or beef",
                "1/2",
                "lbs",
                "ground pork",
                0.25
            ),
            Some("2 oz ground pork or beef".into())
        );
        assert_eq!(
            us(
                "1/2 lbs ground pork or beef",
                "1/2",
                "lbs",
                "ground pork",
                2.0
            ),
            Some("1 lb ground pork or beef".into()),
            "one pound is no plural"
        );
        assert_eq!(
            us("2 ts salt", "2", "ts", "salt", 0.5),
            Some("1 ts salt".into()),
            "`ts` is no plural of `t`"
        );
        assert_eq!(
            us("2 tbs oil", "2", "tbs", "oil", 0.5),
            Some("1 tbs oil".into())
        );
        assert_eq!(
            us(
                "1/2 tsp dark soy sauce",
                "1/2",
                "tsp",
                "dark soy sauce",
                2.0
            ),
            Some("1 tsp dark soy sauce".into())
        );
        assert_eq!(
            measured_line(Some("3"), Some("tsp"), 2.0, Measures::Us, "en", None),
            Some("2 tbsp".into())
        );
        // Read as written, nothing is re-said: three teaspoons stay three.
        assert_eq!(
            measured_line(Some("3"), Some("tsp"), 1.0, Measures::Us, "en", None),
            None
        );
        assert_eq!(
            scaled_line_in(
                "3 tsp sesame paste",
                "en",
                Some("3"),
                Some("tsp"),
                Some("sesame paste"),
                2.0,
                Measures::AsWritten,
                "en",
                None,
            ),
            Some("6 tsp sesame paste".into()),
            "a reader who asked for no conversion keeps the Unit written"
        );
    }

    /// A Yield's noun worded for one and for many, whichever the recipe wrote
    /// (#705): the cooking screen said `1 servings` once a recipe for four was
    /// stepped down to one.
    #[test]
    fn a_yields_noun_is_worded_for_one_and_for_many() {
        let nouns = |amount, noun, language| {
            let worded = yield_nouns(amount, noun, language);
            (worded.one, worded.many)
        };
        let pair = |one: &str, many: &str| (one.to_string(), many.to_string());
        assert_eq!(nouns("4", "servings", "en"), pair("serving", "servings"));
        assert_eq!(nouns("4", "pizzas", "en"), pair("pizza", "pizzas"));
        assert_eq!(nouns("2", "loaves", "en"), pair("loaf", "loaves"));
        assert_eq!(nouns("4", "people", "en"), pair("person", "people"));
        assert_eq!(nouns("1", "loaf", "en"), pair("loaf", "loaves"));
        assert_eq!(
            nouns("1", "ring cake", "en"),
            pair("ring cake", "ring cakes")
        );
        assert_eq!(nouns("8", "slices", "en"), pair("slice", "slices"));
        assert_eq!(nouns("4", "personnes", "fr"), pair("personne", "personnes"));
        assert_eq!(nouns("6", "gâteaux", "fr"), pair("gâteau", "gâteaux"));
        assert_eq!(nouns("1", "plat", "fr"), pair("plat", "plats"));
        assert_eq!(nouns("4", "porciones", "es"), pair("porción", "porciones"));
        assert_eq!(nouns("4", "personas", "es"), pair("persona", "personas"));
        assert_eq!(nouns("2", "panes", "es"), pair("pan", "panes"));
        assert_eq!(nouns("4", "porties", "nl"), pair("portie", "porties"));
        assert_eq!(nouns("4", "personen", "nl"), pair("persoon", "personen"));
        assert_eq!(nouns("4", "pizza's", "nl"), pair("pizza", "pizza's"));
        // Found at review.
        assert_eq!(nouns("4", "taarten", "nl"), pair("taart", "taarten"));
        assert_eq!(nouns("8", "stukken", "nl"), pair("stuk", "stukken"));
        assert_eq!(nouns("1", "taart", "nl"), pair("taart", "taarten"));
        assert_eq!(
            nouns("1", "stuk", "nl"),
            pair("stuk", "stuks"),
            "the word file's plural, as Dutch counts pieces"
        );
        assert_eq!(nouns("1", "portie", "nl"), pair("portie", "porties"));
        assert_eq!(nouns("4", "schalen", "nl"), pair("schaal", "schalen"));
        assert_eq!(nouns("1", "schaal", "nl"), pair("schaal", "schalen"));
        assert_eq!(nouns("6", "tomaten", "nl"), pair("tomaat", "tomaten"));
        assert_eq!(nouns("1", "tomaat", "nl"), pair("tomaat", "tomaten"));
        assert_eq!(nouns("6", "rozen", "nl"), pair("roos", "rozen"));
        assert_eq!(nouns("1", "roos", "nl"), pair("roos", "rozen"));
        assert_eq!(nouns("4", "glazen", "nl"), pair("glas", "glazen"));
        assert_eq!(nouns("12", "muffins", "nl"), pair("muffin", "muffins"));
        assert_eq!(nouns("1", "muffin", "nl"), pair("muffin", "muffins"));
        assert_eq!(nouns("4", "postres", "es"), pair("postre", "postres"));
        assert_eq!(nouns("4", "unidades", "es"), pair("unidad", "unidades"));
        assert_eq!(
            nouns("a dozen", "cookies", "en"),
            pair("cookie", "cookies"),
            "an amount that is no number leaves the noun as the many"
        );
    }

    /// A scaled spoon that reaches the next tool is carried up into it, the
    /// rest said with the smaller one, as an American recipe writes it
    /// (#705): Dan Dan Noodles doubled is `½ cup + 2 tbsp` of chili oil, not
    /// `10 tbsp`, and `1 tbsp + 1 tsp` of ginger, not `4 tsp`.
    #[test]
    fn a_scaled_spoon_is_carried_up_into_the_next_tool() {
        let us = |text: &str, amount, unit, target, scale| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                Some(unit),
                Some(target),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(
            us(
                "5 tbsp Sichuan chili oil",
                "5",
                "tbsp",
                "Sichuan chili oil",
                2.0
            ),
            Some("½ cup + 2 tbsp Sichuan chili oil".into())
        );
        assert_eq!(
            us("2 tsp minced ginger", "2", "tsp", "minced ginger", 2.0),
            Some("1 tbsp + 1 tsp minced ginger".into())
        );
        assert_eq!(
            us("1 1/2 tsp cumin", "1 1/2", "tsp", "cumin", 3.0),
            Some("1½ tbsp cumin".into()),
            "a half tablespoon is a mark of the tablespoon, so needs no teaspoon"
        );
        assert_eq!(
            us("1 tbsp soy sauce", "1", "tbsp", "soy sauce", 1.5),
            Some("1½ tbsp soy sauce".into()),
            "under a quarter cup a tablespoon stays a tablespoon"
        );
        assert_eq!(
            us("7 tsp sugar", "7", "tsp", "sugar", 3.0),
            Some("¼ cup + 3 tbsp sugar".into()),
            "left over from a cup, whole tablespoons"
        );
        assert_eq!(
            us("2 1/4 tbsp honey", "2 1/4", "tbsp", "honey", 2.0),
            Some("¼ cup + 1½ tsp honey".into()),
            "less than a tablespoon left over from a cup, in teaspoons"
        );
        assert_eq!(
            us("1 tsp sugar", "1", "tsp", "sugar", 2.0),
            Some("2 tsp sugar".into()),
            "under a tablespoon a teaspoon stays a teaspoon"
        );
        assert_eq!(
            us("2 tsp salt", "2", "tsp", "salt", 1.5),
            Some("1 tbsp salt".into()),
            "a whole tablespoon needs no teaspoon beside it"
        );
        assert_eq!(
            us("3-4 tbsp chilli oil", "3-4", "tbsp", "chilli oil", 2.0),
            Some("6-8 tbsp chilli oil".into()),
            "a range stays in one tool"
        );
        // Found at review: teaspoons past a quarter cup go on to the cup, and
        // teaspoons rounded up into a tablespoon say "about".
        assert_eq!(
            us("4 tsp sugar", "4", "tsp", "sugar", 5.0),
            Some("¼ cup + 2 tbsp + 2 tsp sugar".into())
        );
        assert_eq!(
            us("1 tsp salt", "1", "tsp", "salt", 5.9),
            Some("about 2 tbsp salt".into())
        );
        assert_eq!(
            measured_line(Some("5"), Some("tbsp"), 2.0, Measures::Us, "en", None),
            Some("½ cup + 2 tbsp".into()),
            "a Step's amount is carried up as a line's is"
        );
        assert_eq!(
            scaled_line_in(
                "5 tbsp huile pimentée",
                "fr",
                Some("5"),
                Some("tbsp"),
                Some("huile pimentée"),
                2.0,
                Measures::Us,
                "fr",
                None,
            ),
            Some("½ tasse + 2 c. à s. huile pimentée".into()),
            "each tool in the reader's words"
        );
    }

    /// A range converted into ounces is said in whole ounces, the marks a
    /// kitchen scale shows, since its ends are a guess already (#705): the
    /// tomato soup's `500-700 g` halved was `about 8¾-12¼ oz`.
    #[test]
    fn a_converted_range_rounds_to_whole_ounces() {
        let tomatoes = |scale| {
            scaled_line_in(
                "500-700 g cherry tomatoes",
                "en",
                Some("500-700"),
                Some("g"),
                Some("cherry tomatoes"),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(tomatoes(0.5), Some("about 9-12 oz cherry tomatoes".into()));
        assert_eq!(
            tomatoes(1.5),
            Some("about 1¾-2¼ lb cherry tomatoes".into()),
            "pounds keep their quarters"
        );
        assert_eq!(
            measured_line(Some("250"), Some("g"), 1.0, Measures::Us, "en", None),
            Some("about 8¾ oz".into()),
            "a single figure keeps its quarter ounces"
        );
        // Found at review: where whole ounces would say one figure twice, or
        // more than the range holds, the quarters stay.
        let short =
            |amount, scale| measured_line(Some(amount), Some("g"), scale, Measures::Us, "en", None);
        assert_eq!(short("20-30", 1.0), Some("about ¾-1 oz".into()));
        assert_eq!(short("250-260", 1.0), Some("about 8¾-9¼ oz".into()));
    }

    /// The pizza dough's lines write each amount against its Unit and
    /// restate it in the other system (#698). The amount is found in the line
    /// all the same, and a reader whose own measure the line already gives
    /// is given that one, scaled, rather than a weight of yeast.
    #[test]
    fn a_line_restating_its_amount_is_scaled_in_the_readers_own_measure() {
        let line = |text: &str, amount, unit, target, scale, measures| {
            scaled_line_in(
                text,
                "en",
                Some(amount),
                Some(unit),
                target,
                scale,
                measures,
                "en",
                None,
            )
        };
        let water = "400g (1 2/3c) Warm Water, 86F/30C";
        assert_eq!(
            line(water, "400", "g", Some("Water"), 2.0, Measures::Us),
            Some("3⅓ cups Warm Water, 86F/30C".into())
        );
        assert_eq!(
            line(water, "400", "g", Some("Water"), 2.0, Measures::Metric),
            Some("800 g Warm Water, 86F/30C".into()),
            "a metric reader keeps the grams, and the cups that now say less go"
        );
        let yeast = "7g (2t) Instant Yeast";
        assert_eq!(
            line(yeast, "7", "g", Some("Instant Yeast"), 0.5, Measures::Us),
            Some("1 tsp Instant Yeast".into())
        );
        assert_eq!(
            line(yeast, "7", "g", Some("Instant Yeast"), 2.0, Measures::Us),
            Some("1 tbsp + 1 tsp Instant Yeast".into()),
            "four teaspoons are a tablespoon and a teaspoon (#705)"
        );
        assert_eq!(
            line(
                "680g (5 3/4c) All-purpose flour",
                "680",
                "g",
                Some("All-purpose flour"),
                2.0,
                Measures::Us
            ),
            Some("11½ cups All-purpose flour".into())
        );
        assert_eq!(
            line("12g or 2 tsp salt", "12", "g", None, 2.0, Measures::Us),
            Some("1 tbsp + 1 tsp salt".into())
        );
        assert_eq!(
            line(
                "100g 6Tbsp tomato paste",
                "100",
                "g",
                Some("6Tbsp tomato paste"),
                2.0,
                Measures::Us
            ),
            Some("¾ cup tomato paste".into())
        );
        assert_eq!(
            line(
                "1 cup (240 ml) milk",
                "1",
                "cup",
                Some("milk"),
                2.0,
                Measures::Metric
            ),
            Some("480 ml milk".into()),
            "a metric reader is given the millilitres the line wrote"
        );
    }

    /// Beside a line read as written, a line that already says the reader's
    /// own measure gets nothing (#698).
    #[test]
    fn a_line_already_in_the_readers_measure_needs_nothing_beside_it() {
        let short = |text: &str, amount, unit, scale| {
            measured_line_of(
                text,
                "en",
                Some(amount),
                Some(unit),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(short("7g (2t) Instant Yeast", "7", "g", 1.0), None);
        assert_eq!(
            short("7g (2t) Instant Yeast", "7", "g", 2.0),
            Some("1 tbsp + 1 tsp".into())
        );
        assert_eq!(
            short("7 g instant yeast", "7", "g", 2.0),
            Some("about ½ oz".into())
        );
    }

    /// A count is cut in quarters, not eighths: nobody cuts ⅜ of a chile
    /// (#698).
    #[test]
    fn a_scaled_count_is_said_in_quarters() {
        let count = |text: &str, target, scale| {
            scaled_line_in(
                text,
                "en",
                Some("1"),
                None,
                Some(target),
                scale,
                Measures::Us,
                "en",
                None,
            )
        };
        assert_eq!(
            count("1 red chile", "red chile", 0.375),
            Some("about ½ red chile".into())
        );
        assert_eq!(
            count("1 red chile", "red chile", 0.25),
            Some("¼ red chile".into())
        );
        // The Sample Library's own count line.
        assert_eq!(
            count("1 chili pepper", "chili pepper", 0.375),
            Some("about ½ chili pepper".into())
        );
    }

    /// "about" says Kamosu rounded or converted, and only then (#680).
    #[test]
    fn about_marks_a_rounded_or_converted_figure_only() {
        let line = |amount, unit, scale, measures| {
            measured_line(Some(amount), unit, scale, measures, "en", None)
        };
        assert_eq!(
            line("1", Some("cup"), 1.5, Measures::Us),
            Some("1½ cups".into())
        );
        assert_eq!(
            line("1", Some("tsp"), 1.3, Measures::Us),
            Some("about 1¼ tsp".into())
        );
        assert_eq!(
            line("1", Some("cup"), 1.0, Measures::Metric),
            Some("about 240 ml".into())
        );
        assert_eq!(
            line("1-2", Some("cups"), 1.0, Measures::Metric),
            Some("about 240-470 ml".into()),
            "a range converts at both ends"
        );
    }

    #[test]
    fn a_dutch_spoon_scales_and_converts_in_a_dutch_recipe() {
        let line = |unit, language| {
            measured_line_in(
                language,
                Some("2"),
                Some(unit),
                1.0,
                Measures::Metric,
                "en",
                None,
            )
        };
        assert_eq!(line("el", "nl"), Some("about 30 ml".to_string()));
        assert_eq!(line("kopje", "nl"), Some("about 470 ml".to_string()));
        assert_eq!(line("el", "es"), None);
    }

    /// A Component's share with no glyph is written with the reader's
    /// decimal mark (#496, #599), and a half stays a half.
    #[test]
    fn a_components_share_takes_the_readers_decimal_mark() {
        assert_eq!(
            component_line(true, false, Some("Deeg"), Some(0.3), "nl"),
            "Deeg · 0,3 van het recept"
        );
        assert_eq!(
            component_line(true, false, Some("Deeg"), Some(0.5), "nl"),
            "Deeg · ½ van het recept"
        );
        assert!(component_line(true, false, Some("Pâte"), Some(0.3), "fr").contains("0,3"));
        assert!(component_line(true, false, Some("Masa"), Some(0.3), "es").contains("0,3"));
        assert!(component_line(true, false, Some("Dough"), Some(0.3), "en").contains("0.3"));
    }
}
