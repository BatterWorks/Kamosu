//! **Each Language's words, in a file of their own** (ADR 0057, #495).
//!
//! What Kamosu reads a recipe with comes in two halves. A Language's words are
//! data: the files beside this one, `src/words/*.toml`, one per Language, built
//! into the binary and read once. Its grammar is code: where a name's main word
//! sits, how numbers are written, what an elision does. That split is why a
//! missing word is a line in one file, and why a new Language starts from a new
//! file rather than from forty-five lists.
//!
//! **Which files read which recipe.** [`of`] answers that and nothing else asks
//! it. English, French and Spanish are read with their three files together,
//! as they always were, and so is a recipe marked Unknown or with no Language:
//! `2 cups farine` is two cups. `every.toml` holds the Unit spellings every
//! Language writes, and goes with whichever files read a recipe. A word the
//! code confines to one Language stays confined by that code, such as French
//! *pour* in `step_uses.rs`.
//!
//! Reading a word file fails at the first use, naming the file and the place,
//! and a test uses every one of them: a spelling mistake in a list's name or in
//! a Unit's cannot reach a build.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use serde::Deserialize;

use crate::language::UNKNOWN;
use crate::units::{self, UNITS, Unit};

/// Fold a word for comparison against a [`List`]: the Unicode fold
/// [`crate::units`] uses, then everything that is not a letter or a digit
/// thrown away, combining marks included, so neither punctuation nor an accent
/// can make one spelling two. `pincée`, `pincee` and `Pincée` are one word
/// here, which is what lets each list be written once.
///
/// [`Words::accented_names`] is the one list an accent does tell apart, and it
/// is compared through [`folded_keeping_accents`].
pub(crate) fn folded(word: &str) -> String {
    fold(word, false)
}

/// [`folded`], with the accents left on: `Thé,` and `thé` are one word here,
/// and `the` is another.
pub(crate) fn folded_keeping_accents(word: &str) -> String {
    fold(word, true)
}

fn fold(word: &str, keep_accents: bool) -> String {
    use caseless::Caseless;
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;
    word.chars()
        .nfd()
        .default_case_fold()
        .nfd()
        .filter(|c| c.is_alphanumeric() || (keep_accents && is_combining_mark(*c)))
        .collect()
}

/// A list a word is looked up in by [`folded`], whatever its accents or
/// punctuation.
#[derive(Default)]
pub struct List(HashSet<String>);

impl List {
    pub fn has(&self, word: &str) -> bool {
        let word = folded(word);
        !word.is_empty() && self.0.contains(&word)
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D: serde::Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let words = Vec::<String>::deserialize(from)?;
        Ok(List(words.iter().map(|word| folded(word)).collect()))
    }
}

/// A list a word already folded by [`units::fold_loosely`] is looked up in, as
/// the timer and the Step linker hold their words.
#[derive(Default)]
pub struct Loose(HashSet<String>);

impl Loose {
    pub fn has(&self, word: &str) -> bool {
        self.0.contains(word)
    }
}

impl<'de> Deserialize<'de> for Loose {
    fn deserialize<D: serde::Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let words = Vec::<String>::deserialize(from)?;
        Ok(Loose(
            words.iter().map(|word| units::fold_loosely(word)).collect(),
        ))
    }
}

/// A list kept exactly as the file spells it.
#[derive(Default, Deserialize)]
#[serde(transparent)]
pub struct Exact(Vec<String>);

impl Exact {
    pub fn has(&self, word: &str) -> bool {
        self.0.iter().any(|entry| entry == word)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

/// A list of phrases, each written in a file as its words with spaces between.
#[derive(Default)]
pub struct Phrases(Vec<Vec<String>>);

impl Phrases {
    pub fn iter(&self) -> impl Iterator<Item = &[String]> {
        self.0.iter().map(Vec::as_slice)
    }
}

impl<'de> Deserialize<'de> for Phrases {
    fn deserialize<D: serde::Deserializer<'de>>(from: D) -> Result<Self, D::Error> {
        let phrases = Vec::<String>::deserialize(from)?;
        Ok(Phrases(
            phrases
                .iter()
                .map(|phrase| phrase.split_whitespace().map(folded).collect())
                .collect(),
        ))
    }
}

/// What each word for a length of time is worth.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Durations {
    second: Loose,
    minute: Loose,
    hour: Loose,
}

/// Adding one file's words to another's.
trait Gather {
    fn gather(&mut self, from: Self);
}

impl Gather for List {
    fn gather(&mut self, from: Self) {
        self.0.extend(from.0);
    }
}

impl Gather for Loose {
    fn gather(&mut self, from: Self) {
        self.0.extend(from.0);
    }
}

impl Gather for Exact {
    fn gather(&mut self, from: Self) {
        self.0.extend(from.0);
    }
}

impl Gather for Phrases {
    fn gather(&mut self, from: Self) {
        self.0.extend(from.0);
    }
}

impl Gather for Durations {
    fn gather(&mut self, from: Self) {
        self.second.gather(from.second);
        self.minute.gather(from.minute);
        self.hour.gather(from.hour);
    }
}

impl Gather for HashMap<String, Vec<String>> {
    fn gather(&mut self, from: Self) {
        for (unit, spellings) in from {
            self.entry(unit).or_default().extend(spellings);
        }
    }
}

impl Gather for HashMap<String, f64> {
    fn gather(&mut self, from: Self) {
        self.extend(from);
    }
}

/// Every list a word file may hold. Declared once, so a list is added to the
/// struct, to the reading of a file and to gathering files together by adding
/// it here, and a list name a file misspells refuses that file.
macro_rules! word_lists {
    ($($(#[$meta:meta])* $name:ident: $kind:ty,)*) => {
        /// The words one recipe is read with: one Language's file, or several
        /// together, as [`of`] decides. See the files for the words.
        #[derive(Default, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        pub struct Words {
            $($(#[$meta])* pub $name: $kind,)*
            /// Every spelling of the closed set, folded once. Built from
            /// `units`, never read from a file.
            #[serde(skip)]
            by_spelling: HashMap<String, &'static Unit>,
        }

        impl Words {
            fn gather(&mut self, from: Words) {
                $(self.$name.gather(from.$name);)*
            }
        }
    };
}

word_lists! {
    /// Units outside the closed set that a recipe writes constantly: a vessel,
    /// a piece, a handful. None of them converts to anything, a *gousse* is no
    /// quantity of anything measurable, and that is fine: ADR 0016 keeps the
    /// set of Units open precisely so these are ordinary rather than
    /// exceptional.
    ///
    /// The fold strips accents but a French keyboard produces both, so one
    /// spelling covers them.
    open_units: List,

    /// Words that stand exactly where a Unit stands and are not one: they
    /// describe the thing rather than measure it. Dropped so `1 large onion`
    /// and `2 onions` point at one Food rather than two.
    ///
    /// Every form a French or Spanish size takes is listed, plurals included,
    /// because the fold strips accents but not endings: without `petits`,
    /// `2 petits poireaux` named a Food *petits poireaux* (#185).
    sizes: List,

    /// The [`sizes`](Self::sizes) French and Spanish write after the food
    /// rather than before it, `2 tomates moyennes` and `1 cebolla grande`,
    /// dropped from the end of the Food's name as the others are from its
    /// start (#185).
    ///
    /// **`entier` is not one of them.** After the food it says *whole* the way
    /// a product does: `farine de blé entier` is whole-wheat flour, and `lait
    /// entier` whole milk, neither the same thing to buy as plain flour or
    /// milk. Nor is `gros`, for the same reason: `sel gros` is coarse salt.
    /// English `whole` before the food is still dropped as a size, as it was
    /// before #185, so `whole milk` reads as *milk*; that was left alone
    /// there.
    ///
    /// Kept as its own list rather than derived from `sizes`: a new French or
    /// Spanish size goes into both.
    sizes_after: List,

    /// Words of praise for a measure, or for how full it is: `1 good pinch
    /// salt`, `1 heaping tablespoon sugar`, `1 scant cup flour`, `1 bonne
    /// poignée de basilic`. Like a size, such a word says nothing about which
    /// Food to buy, but it is dropped only where a measure makes it one
    /// (#186): straight before a Unit, and at the start of a Food that follows
    /// an amount or a Unit. Anywhere else it stays in the name, which is the
    /// choice on #186: *poulet à la bonne femme* is a dish, not a good woman's
    /// chicken.
    ///
    /// Kept apart from `sizes` for that reason, since a size is dropped from
    /// the front of any Food. Every form is listed, as for `sizes`. Words that
    /// are as often part of a name are left out: *Nice* biscuits are a
    /// biscuit.
    praise: List,

    /// The ways a recipe says how warm a food should be when it goes in. How
    /// warm the butter is says nothing about which butter to buy, so a phrase
    /// at either end of a Food's name is dropped: `100 g de beurre froid` is a
    /// Reading of *beurre* (#185).
    ///
    /// **`hot` and `glacé` are left out on purpose.** Hot sauce and a hot dog
    /// are things to buy, and the fold makes `glacé` one word with the `glace`
    /// of *sucre glace*, which is icing sugar. `chaud` is in, although
    /// *chocolat chaud* is a drink: the question answered on #185 named
    /// `chaud` among the words to drop, and an Ingredient Line far more often
    /// means warm milk than hot chocolate.
    ///
    /// A word that only says *very* goes with the temperature it strengthens,
    /// so `eau très chaude` is *eau* and not *eau très* ([`very`](Self::very)).
    ///
    /// Only temperature, and never strength: `fond de volaille corsé` and
    /// `moutarde forte` keep their names, because a strong mustard is not the
    /// mustard beside it on the shelf.
    warmth: Phrases,

    /// The article or preposition gluing an amount to what it is an amount
    /// of. A name never keeps one: `200 g de farine` is a Reading of *farine*,
    /// and a Food called *de farine* would never match the *farine* beside it.
    ///
    /// This is the whole of why the corpus's French lines read correctly here
    /// and do not in either library measured in ADR 0036.
    ///
    /// **`d` and `l` are deliberately absent.** They are glue only as `d'` and
    /// `l'`, which the reader leaves carrying their apostrophe and recognises
    /// by that apostrophe, because a bare `l` is a litre, and `1 l milk` is a
    /// real line in the corpus.
    ///
    /// Ask `reading::listed_as_glue` whether a word is one of these, never
    /// this list: [`accented_names`](Self::accented_names) fold onto three of
    /// them and are not glue (#213).
    glue: List,

    /// Words that fold onto a word of [`glue`](Self::glue) and are no such
    /// thing: French for tea, for diced pieces and for one of them. Each is
    /// glue once the fold throws its accent away, which named a tea after its
    /// colour, `1 thé vert` as *vert*, and left `200 ml de thé` naming nothing
    /// (#213).
    ///
    /// Only the accented spelling is kept. `1 the vert` is what an English
    /// article looks like, and goes on reading as *vert*.
    accented_names: Exact,

    /// Words that open a line to say *a few* or *about*. Kamosu can do no
    /// arithmetic on *a few*, so one is never the amount; left standing it hid
    /// the Unit behind it, and `quelques feuilles de menthe` named a Food
    /// after the leaves (#181).
    ///
    /// Dropped only at the start of the line and **before** the amount is
    /// read, never as glue: `unos 200 g de harina` is Spanish for *about*
    /// 200 g, and the 200 has to reach the amount rather than the Food.
    a_few: List,

    /// The ways a line opens to say *a little*. Like [`a_few`](Self::a_few)
    /// it is no amount, and left standing its second word opened the Food:
    /// `un peu de sirop d'érable` named *peu de sirop d'érable* (#208).
    ///
    /// Only the whole phrase, and only at the start of the line. `peu de sel`
    /// and `1 little gem lettuce` read as they always have.
    a_little: Phrases,

    /// The ways a recipe says *as much as you like*. Such a phrase measures
    /// nothing and names nothing, so it is dropped from the front or the end
    /// of what a Reading reads (#163): `to taste sea salt` and `salt and
    /// pepper to taste` read exactly as `sea salt` and `salt and pepper` do.
    /// Inside brackets or after a comma it was already the cook's aside.
    ///
    /// Only the two ends, because in the middle a phrase can mean something
    /// else: `huile au goût de truffe` is truffle-flavoured oil, one thing to
    /// buy.
    to_taste: Phrases,

    /// Words that say how a food is prepared or bought, never what it is: a
    /// Food made of nothing else is not a Food (#163). `4 cloves, minced`
    /// measures garlic that the line never names, and a Food called *minced*
    /// would sit on the shopping list meaning nothing.
    ///
    /// Beside a food they stay part of its name, as they always have: `minced
    /// garlic` and `boneless chicken thighs` are what the cook wrote. Only a
    /// name that is **all** of these is refused, and where a comma cut such a
    /// run off from its food, as in `boneless, skinless chicken thighs`, the
    /// Food is read on past the comma instead.
    ///
    /// Every form a French or Spanish word takes is listed, because the fold
    /// strips accents but not endings. A word that is also a food is left out,
    /// whatever else it means: *fondue*, *cocido*, *confit*, *cortado* the
    /// coffee and *picada* the Catalan sauce.
    describing: List,

    /// What may join two [`describing`](Self::describing) words without
    /// naming anything itself: `peeled and chopped`.
    describing_joins: List,

    /// The elided articles a Language writes against the next word. Split off
    /// so glue can see them, and glued back wherever one survives inside a
    /// name: `un filet d'huile d'olive` keeps its *d'huile*.
    elisions: Exact,

    /// Names that open with a size or warmth word which is no size or
    /// temperature there: *petits pois* are green peas, not small *pois*, and
    /// *cold cuts* are not cold *cuts* (#185). Neither end-stripping touches
    /// the opening word of one.
    fixed_names: Phrases,

    /// The words that strengthen a [`warmth`](Self::warmth) word and mean
    /// nothing without one.
    very: List,

    /// Words that say which kind of a food, or what it is like, and are never
    /// what a cook calls the food by on its own: nobody adds *the crispy* or
    /// *the Greek*. Beside its food such a word stays part of the name, as the
    /// [`describing`](Self::describing) words do; this list only stops a
    /// Step's `uses` from linking *crispy chilli oil* to *the crispy dumpling
    /// base* (#184).
    ///
    /// Colours, textures, sweetness, fat and where a food is from, then the
    /// words that name a form or a use rather than a food (*paste*, *cubes*,
    /// *baking*, *for serving*). Every French and Spanish ending is written
    /// out, for the reason `describing` gives.
    names_nothing_alone: List,

    /// Words for a cut, a part or a form of a food, which a recipe writes
    /// beside the food and a Step leaves off: *chicken thighs* is *the
    /// chicken* by the second Step, and *hauts de cuisse de poulet* is *le
    /// poulet* (#212). Where a name's main word would be one of these, the
    /// word beside it is a main word too.
    ///
    /// A word belongs here only when what is left without it is still the
    /// same food. *Meal*, *stock*, *juice* and *zest* do not: almond meal is
    /// not almonds and lemon juice is not a lemon. A word listed here by
    /// mistake links a Step to one line too many, and a word missing from here
    /// leaves a Step linking only by the cut. The forms `names_nothing_alone`
    /// already holds (*extract*, *powder*, *paste*, *fillet*) and the measures
    /// (*cloves*, *leaves*, *gousses*) give way the same way and are not
    /// repeated.
    ///
    /// French *pâte* and Spanish *pasta* are left out on purpose. *Pâte de
    /// miso* is a form of miso, but *pâte à pizza* is no form of a pizza, and
    /// one list cannot tell them apart.
    cuts_and_parts: List,

    /// Foods whose own name is all a Step needs, though the list writes the
    /// kind of thing after it: *Parmesan cheese* is *the parmesan* (#212).
    /// Such a word is a main word of its name wherever it sits.
    named_kinds: List,

    /// Words that make a name **something made from a food** rather than the
    /// food: curry *powder* is no curry, and lemon *zest* is no lemon. A word
    /// of the title that a Step names a line by still names the dish where the
    /// line is one of these, and a Step naming the food links the food itself
    /// before something made from it, so *the garlic* is the cloves and not
    /// the powder (#439). [`cuts_and_parts`](Self::cuts_and_parts) holds the
    /// other kind, the food itself in pieces.
    ///
    /// French *pâte* and Spanish *pasta* are left out for the reason they are
    /// left out there.
    made_from: List,

    /// Words a recipe calls something it makes itself: *the sauce* is the one
    /// in the pan, not the soy sauce in the list. French *pâte* and Spanish
    /// *masa* are left out, since *la pâte* is as often the *pâte brisée* the
    /// list buys. Such a word never names a line on its own, though a line's
    /// whole name still does, and a food named just before it is describing
    /// it: *the tomato-spice mixture* adds no tomato (#439).
    made_here: List,

    /// The words after which an English name says what the food is for, or
    /// gives an example of it, and no longer names it: *oil for frying*, *dry
    /// sherry such as Amontillado*, *a neutral oil like canola*. Kept to these
    /// three: *to* sits inside *ready to wok udon noodles*, and *in* and *on*
    /// inside *bone-in* and *skin-on*, all before the food they describe.
    name_tails: List,

    /// The words that join the two ends of a range written out, in each
    /// Language: `2 to 3`, `2 ou 3`, `2 o 3`, and `2 a 3`, which the fold
    /// makes French `2 à 3` too. `or`, `ou` and `o` are
    /// [`choices`](Self::choices) as well: between two numbers they join a
    /// range, and anywhere else they offer a choice.
    amount_ranges: List,

    /// The word that offers the cook a choice.
    choices: List,

    /// The words after a French `c.` that say it is a *cuillère* and not the
    /// cup of an American card (#326): `à`, a spoon's name written without
    /// it, `c. soupe`, or the `de` of a spoon left unnamed, `1 c. de sel`. A
    /// cup in French is a `tasse`, so no French line is a lone `c` of
    /// something.
    spoon_after_c: List,

    /// The words that end what an amount in a Step is an amount of: *1 cup of
    /// water **to** the flour* is water, and without this the words would run
    /// on to the flour and weigh the water by its Cup Weight. Prepositions and
    /// conjunctions; `or` is not one, because *homemade or store-bought
    /// marinara sauce* is one Food.
    food_ends: List,

    /// What a degree is called, between a number and its `C` or `F`.
    degrees: Exact,

    /// The words that join the two ends of a timer's range, beside a dash and
    /// a slash. `a` is here for the Spanish *de 5 a 7 minutos* and takes the
    /// French *à* with it, since the fold drops the accent.
    timer_ranges: Loose,

    /// The one word a cook is allowed to slip between the number and its unit:
    /// *cook for 3 more minutes*. Four real Steps in the export do it, all of
    /// them with `more`, and none of them with two words; the rest of this
    /// list is that one habit written in the other Languages rather than a
    /// guess at what else might appear.
    timer_more: Loose,

    /// The half a cook says rather than writes as a number: *2 heures et
    /// demie*, *2 minutos y medio*. A phrase counts only directly against its
    /// duration: after the unit word (*2 minutes and a half*) or between the
    /// number and it (*2 and a half minutes*). The set was settled in #157;
    /// *trois quarts d'heure* and the other spelled-out fractions are not in
    /// it.
    spoken_half: Phrases,

    /// The quarter a cook says, *1 hour and a quarter*, as for
    /// [`spoken_half`](Self::spoken_half).
    spoken_quarter: Phrases,

    /// The words that may follow a duration's trailing number without taking
    /// it for their own: *2 minutes 30 **de** chaque côté*, *2 min 30 **per**
    /// side*.
    ///
    /// Any other word claims the number: *2 minutes 30 **g** de beurre* is two
    /// minutes and a weight, *stir 2 minutes 3 **times*** two minutes and a
    /// count. A list of the short words that join a sentence is closed and can
    /// be written down; a list of every noun a number might count is not, so
    /// this names the first and treats everything else as the second. Missing
    /// a word here costs a timer that stops at the whole minutes, which is
    /// what every Step offered before #157.
    ///
    /// `d` and `jusqu` are what is left of *d'un* and *jusqu'à* once the
    /// apostrophe splits them.
    timer_joins: Loose,

    /// The words for a half that stand against a duration word with no number:
    /// *une demi-heure*, *media hora*, *medio minuto*. English puts an article
    /// between, which [`half_with_article`](Self::half_with_article) holds.
    half_words: Loose,

    /// A half with an article between it and the duration word: *half an
    /// hour*.
    half_with_article: Phrases,

    /// A half joined to the duration word by a hyphen: *une demi-heure*.
    half_hyphened: Phrases,

    /// The words that open a clause describing how things should look, never
    /// what goes in: *until*, *jusqu'à* (folded to two words, the first
    /// `jusqu`), and *hasta*. *Till* is English's short *until*.
    ///
    /// French and Spanish also write these for *up to* before an amount, as in
    /// *ajoutez jusqu'à 100 ml d'eau*, where the water does go in. So a number
    /// straight after one, past an `a`, opens no such clause.
    step_until: Loose,

    /// The words that open a clause saying when to do the next thing, which
    /// holds nothing going in either: *when the rice and chicken are ready,
    /// remove the chicken* (#439). It ends where a
    /// [`step_until`](Self::step_until) clause does. One before a word ending
    /// in *-ing* opens none, since *whisk while adding the oil* adds the oil.
    /// French *lorsqu'* is folded to `lorsqu`.
    step_when: Loose,

    /// The words before which a food is what something is for, not what goes
    /// in: *reserving 1 tablespoon for the rice* (#439). French *pour* is also
    /// English *pour*, so `step_uses.rs` counts it only on a Branch written in
    /// French.
    step_for: Loose,

    /// The words for a stock, which a food named beside it describes: *chicken
    /// stock*, *bouillon de poulet*, *caldo de pollo* (#439).
    step_stocks: Loose,

    /// What joins a French or Spanish thing to the food that describes it, as
    /// *de* does in *le mélange de tomates*.
    step_of: Loose,

    /// What follows a section's heading, or comes before it, where a Step
    /// names the section whole: *all the spice blend ingredients* (#439).
    /// Folded, so French *ingrédients* is here too.
    step_ingredients: Loose,

    /// The articles that may stand between
    /// [`step_for`](Self::step_for) and its food.
    step_articles: Loose,

    /// The words that end a [`step_until`](Self::step_until) clause where no
    /// mark does, since what follows is the next thing to do: *whisk until
    /// pale then fold in the flour* (#294). Spanish *después* is left out,
    /// since *después de* is *after*.
    step_then: Loose,

    /// The things a cook does that end a [`step_until`](Self::step_until)
    /// clause after *and*, *et* or *y*: *cook until soft and add the garlic*.
    /// *And* alone does not, since it joins two foods inside the condition as
    /// often, as in *until the butter and sugar are pale*, and nor does a word
    /// as often a noun, as *top* is in *until the bottom and top are crisp*
    /// (#294).
    step_next_actions: Loose,

    /// What joins the condition to one of the
    /// [`step_next_actions`](Self::step_next_actions).
    step_and: Loose,

    /// The heading a recipe puts over its list of ingredients. It is a Section
    /// like [`paste_sections`](Self::paste_sections), and above it a page says
    /// what it likes, none of which is an Ingredient Line (#301).
    paste_ingredients: Exact,

    /// A line that is exactly one of these, or of
    /// [`paste_ingredients`](Self::paste_ingredients), is a Section, colon or
    /// none. The words a recipe page puts over its two lists: `Préparation`
    /// has no colon, and was read as a step.
    paste_sections: Exact,

    /// A block under one of these is about the recipe rather than in it, and
    /// goes to its `note`.
    paste_notes: Exact,

    /// The first words of a line saying how many a recipe feeds, how long it
    /// takes, how hard it is or who wrote it, with or without a colon after:
    /// never its name, however short.
    paste_not_a_name: Exact,

    /// Each Unit of the closed set's spellings, by the Unit's id. Written the
    /// way a person reads them, since [`units::fold`] is applied to both these
    /// and the cook's word, so punctuation and capitals need no thought.
    ///
    /// Accented and unaccented forms ARE both listed, rather than being folded
    /// together: elsewhere in Kamosu accents are meaning (a Food Match will
    /// not equate *maïs* with *mais*), and a unit table that quietly stripped
    /// them would be the one place that rule did not hold. Listing `cuillere a
    /// cafe` beside `cuillère à café` says the same forgiveness out loud.
    units: HashMap<String, Vec<String>>,

    /// What one cup of a staple weighs, about, in grams, by its name in this
    /// Language: see [`units::shipped_cup_weight`].
    cup_weights: HashMap<String, f64>,

    /// What each word for a length of time is worth. The bare `h` is here
    /// although the real 86-recipe export contains not one: `1 h 30` is the
    /// ordinary French spelling and its absence from 579 real Steps is a fact
    /// about those recipes rather than about French. What is *not* here is a
    /// bare `m` or `s`: a metre and a gram's neighbour are too close, and a
    /// duration nobody offers is cheaper than a timer offered on `500 g`. The
    /// one place a bare `s` is read is right after minutes, `2 min 30 s`,
    /// where it cannot be anything else.
    durations: Durations,
}

impl Words {
    /// One or more files read as one set of words.
    fn gathered(files: &[(&str, &str)]) -> Words {
        let mut words = Words::default();
        for (name, text) in files {
            let file: Words = toml::from_str(text)
                .unwrap_or_else(|fault| panic!("the word file {name} does not read: {fault}"));
            words.gather(file);
        }
        words.by_spelling = spellings_of(&words.units);
        words
    }

    /// The Unit a written word names, or nothing, which is not an error. A
    /// word outside the closed set is no less real a Unit; it simply never
    /// converts.
    pub fn unit(&self, word: &str) -> Option<&'static Unit> {
        if let Some(letter) = units::lone_spoon(word) {
            let id = if letter == "t" {
                "teaspoon"
            } else {
                "tablespoon"
            };
            return Some(units::unit_by_id(id));
        }
        let folded = units::fold(word);
        if folded.is_empty() {
            return None;
        }
        self.by_spelling.get(&folded).copied()
    }

    /// What one of this duration word is worth in seconds, or nothing where
    /// the word names no duration.
    pub fn seconds_each(&self, word: &str) -> Option<i64> {
        let durations = &self.durations;
        if durations.second.has(word) {
            Some(1)
        } else if durations.minute.has(word) {
            Some(60)
        } else if durations.hour.has(word) {
            Some(3600)
        } else {
            None
        }
    }
}

/// Every spelling in one set of files, folded once, pointing at its Unit.
///
/// Built rather than searched so that the files can be written the way a
/// person reads them, accented, punctuated, in several languages, while every
/// lookup still compares fold against fold. A spelling across two Units would
/// be a table Kamosu could not answer consistently, so it stops the program at
/// the first use rather than being left to shadow silently, and a test uses
/// every set of files.
fn spellings_of(spelled: &HashMap<String, Vec<String>>) -> HashMap<String, &'static Unit> {
    let mut index: HashMap<String, &'static Unit> = HashMap::new();
    for (id, spellings) in spelled {
        let unit = UNITS
            .iter()
            .find(|unit| unit.id == id)
            .unwrap_or_else(|| panic!("a word file spells a Unit Kamosu has not: {id:?}"));
        for spelling in spellings {
            let folded = units::fold(spelling);
            assert!(
                !folded.is_empty(),
                "a Unit spelling folds away to nothing: {spelling:?}"
            );
            if let Some(taken) = index.insert(folded.clone(), unit)
                && taken.id != unit.id
            {
                panic!(
                    "the spelling {folded:?} names two Units: {} and {}",
                    taken.id, unit.id
                );
            }
        }
    }
    index
}

const EVERY: (&str, &str) = ("every.toml", include_str!("words/every.toml"));
const EN: (&str, &str) = ("en.toml", include_str!("words/en.toml"));
const FR: (&str, &str) = ("fr.toml", include_str!("words/fr.toml"));
const ES: (&str, &str) = ("es.toml", include_str!("words/es.toml"));

/// English, French and Spanish read as one.
static TOGETHER: LazyLock<Words> = LazyLock::new(|| Words::gathered(&[EVERY, EN, FR, ES]));

/// **The words a recipe in this Language is read with.** The one place that
/// says which files read which recipe (ADR 0057). `language` is a Branch's, or
/// [`UNKNOWN`] where a caller knows none.
pub fn of(language: &str) -> &'static Words {
    let _ = language;
    &TOGETHER
}

/// The words of a caller that knows no Language.
pub fn unknown() -> &'static Words {
    of(UNKNOWN)
}

/// The shipped staples of every word file, folded once, with what a cup of each
/// weighs. A Food is known by its names in any Language, whichever recipe asks.
pub static CUP_WEIGHTS: LazyLock<HashMap<String, f64>> = LazyLock::new(|| {
    TOGETHER
        .cup_weights
        .iter()
        .map(|(staple, grams)| (units::fold(staple), *grams))
        .collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_file_reads() {
        assert!(of("en").unit("g").is_some());
        assert!(!CUP_WEIGHTS.is_empty());
    }

    #[test]
    fn a_list_a_file_misspells_refuses_the_file() {
        assert!(toml::from_str::<Words>("sizez = [\"large\"]").is_err());
    }
}
