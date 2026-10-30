//! Reading a Paprika export (#414, the spec is #411): Paprika's own
//! `.paprikarecipes`, made with File › Export on a computer or Settings ›
//! Export Recipes on a phone, in Paprika Recipe Format. It is a zip of one
//! `.paprikarecipe` per recipe, each a gzip of that recipe's JSON. A zip of
//! loose `.paprikarecipe` files, as Mealie's sample is, reads the same way.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files described in `tests/fixtures/paprika/README.md`.
//!
//! - **Ingredients and directions are one block of text each.** Each line of
//!   the ingredients is an Ingredient Line as written, and no line is guessed
//!   to be a heading. Each paragraph or line of the directions is a Step, as
//!   written too, its own number included where the cook typed one.
//! - **The photo inside the file is the Main Photo.** `photo_data` is the
//!   cook's picture. `image_url` is the source website's, not a larger copy of
//!   the cook's, so it is handed over to be downloaded only when the file holds
//!   no photo of its own.
//! - **A time is Paprika's free text**, read the way a Step's timer is
//!   ([`crate::units::step_duration`]), and a bare number as minutes, which is
//!   what Paprika's own field shows it as.
//! - **The servings are free text too** (`Serves 4`, `about 20 rolls`): the
//!   first number is the Yield's amount, the lower end of a range, and the
//!   words after it its noun, servings where they say no more.
//! - **The description and the notes become the Note**, and categories Tags.
//! - **What a cook typed that no field holds goes into the Note** after them,
//!   word for word, each a line of its own led in the recipe's Language
//!   (#434): the total time, a time Kamosu cannot read (`1 day`) or reads
//!   only in part (`35 to 40 mins`), servings with no count or a range's
//!   upper end, the difficulty, and the free-text nutrition, which is never
//!   read as a number.
//! - **Extra photos, the rating and the favourite mark are counted on the
//!   Report**, never written into the recipe.
//!
//! Every real file has one shape, the same from 2021 and 2023: the fields
//! below, each recipe with its `uid`. A file of Paprika's in any other shape
//! is refused with the update reason.

use base64::Engine;
use serde_json::{Value, json};

use crate::other_apps::{self, Budget, Handed, LeftOut, Reader, Refusal};

/// Paprika among the readers of another app's export.
pub struct Paprika;

const APP: &str = "Paprika";

/// The largest single recipe read out of the zip, packed or unpacked. The
/// 746-recipe archive's largest is half a megabyte, its photo inside it, so
/// this refuses only a file that claims to be enormous.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

type Archive = zip::ZipArchive<Box<dyn other_apps::ReadSeek>>;

impl Reader for Paprika {
    fn names(&self) -> &'static [&'static str] {
        &[APP]
    }

    fn read(&self, handed: &Handed) -> Result<Option<other_apps::Export>, Refusal> {
        if !handed.is_zip() {
            return Ok(None);
        }
        let Ok(file) = handed.open() else {
            return Ok(None);
        };
        let Ok(mut archive) = Archive::new(file) else {
            return Ok(None);
        };
        let entries = recipe_entries(&archive);
        let mut budget = Budget::new(other_apps::MOST_UNPACKED);
        // The first recipe that reads says whose file this is, and in which
        // shape. One damaged recipe is that recipe's loss, not the file's.
        let mut first = None;
        for entry in &entries {
            if let Ok(recipe) = unpack_recipe(&mut archive, entry.index, &mut budget) {
                first = Some(recipe);
                break;
            }
        }
        let Some(first) = first else {
            return Ok(None);
        };
        if !is_paprika_recipe(&first) {
            return Ok(None);
        }
        if !is_known_shape(&first) {
            return Err(Refusal::UntestedShape { app: APP });
        }

        let total = entries.len();
        let recipes = entries
            .into_iter()
            .map(move |entry| read_entry(&mut archive, &entry, &mut budget));
        Ok(Some(other_apps::Export {
            app: "paprika",
            total,
            recipes: Box::new(recipes),
            left_out: Vec::new(),
        }))
    }
}

/// One `.paprikarecipe` of the zip: its archive index, and its file's name
/// without the extension, which a recipe that cannot be read is named by.
struct Entry {
    index: usize,
    name: String,
}

/// Every `.paprikarecipe` in the zip, in name order, in whichever folder. A
/// zip made on a Mac, as Mealie's sample is, holds `__MACOSX/` copies and
/// `._` resource forks beside them that are not recipes.
fn recipe_entries(archive: &Archive) -> Vec<Entry> {
    let mut entries: Vec<(String, Entry)> = (0..archive.len())
        .filter_map(|index| {
            let path = archive.name_for_index(index)?;
            if path.starts_with("__MACOSX/") {
                return None;
            }
            let file = path.rsplit('/').next().unwrap_or(path);
            let stem = file
                .len()
                .checked_sub(".paprikarecipe".len())
                .filter(|&at| file.is_char_boundary(at))
                .filter(|&at| file[at..].eq_ignore_ascii_case(".paprikarecipe"))
                .map(|at| &file[..at])?;
            if stem.is_empty() || file.starts_with("._") {
                return None;
            }
            Some((
                path.to_string(),
                Entry {
                    index,
                    name: stem.to_string(),
                },
            ))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().map(|(_, entry)| entry).collect()
}

const DAMAGED: &str = "the file is damaged and none of it could be read; export the \
                       recipes from Paprika again and import that";

/// One recipe of the zip, out of the zip and out of its gzip, each through
/// the export's budget: a gzip inside a zip is as deep as Paprika nests, so
/// no deeper level is ever opened. The error is a sentence for the Report.
fn unpack_recipe(
    archive: &mut Archive,
    index: usize,
    budget: &mut Budget,
) -> Result<Value, String> {
    let too_much = |budget: &Budget| {
        format!(
            "the export unpacks to more than {} GB in all, more than any Paprika export \
             holds, so this recipe and those after it were not read",
            budget.most() / (1000 * 1000 * 1000)
        )
    };
    let Ok(mut entry) = archive.by_index(index) else {
        return Err(DAMAGED.to_string());
    };
    let too_large = || "a file in it is too large to be one recipe".to_string();
    let packed = match budget.unpack(&mut entry, MAX_FILE_BYTES) {
        Err(other_apps::TooMuch) => return Err(too_much(budget)),
        Ok((bytes, _)) if bytes.len() as u64 > MAX_FILE_BYTES => return Err(too_large()),
        Ok((_, false)) => return Err(DAMAGED.to_string()),
        Ok((bytes, true)) => bytes,
    };
    drop(entry);
    let unpacked = match budget.unpack(
        flate2::read::GzDecoder::new(packed.as_slice()),
        MAX_FILE_BYTES,
    ) {
        Err(other_apps::TooMuch) => return Err(too_much(budget)),
        Ok((bytes, _)) if bytes.len() as u64 > MAX_FILE_BYTES => return Err(too_large()),
        Ok((_, false)) => return Err(DAMAGED.to_string()),
        Ok((bytes, true)) => bytes,
    };
    serde_json::from_slice(&unpacked).map_err(|_| {
        "the recipe is not one Kamosu can read: it may be damaged, so export it from \
         Paprika again and import that"
            .to_string()
    })
}

/// Whether a recipe's JSON is one Paprika wrote, in whichever shape.
fn is_paprika_recipe(recipe: &Value) -> bool {
    recipe["uid"].is_string()
        && recipe.get("ingredients").is_some()
        && recipe.get("directions").is_some()
}

/// Whether a recipe has the shape every real file has.
fn is_known_shape(recipe: &Value) -> bool {
    let text_or_null = |field: &str| recipe[field].is_string() || recipe[field].is_null();
    recipe["name"].is_string()
        && [
            "ingredients",
            "directions",
            "notes",
            "description",
            "nutritional_info",
            "prep_time",
            "cook_time",
            "servings",
            "source",
            "source_url",
            "image_url",
            "photo_data",
        ]
        .iter()
        .all(|field| text_or_null(field))
        && (recipe["categories"].is_array() || recipe["categories"].is_null())
        && (recipe["photos"].is_array() || recipe["photos"].is_null())
}

/// Read one `.paprikarecipe`.
fn read_entry(
    archive: &mut Archive,
    entry: &Entry,
    budget: &mut Budget,
) -> Result<other_apps::Recipe, other_apps::Unread> {
    let unread = |foreign_id: Option<String>, reason: String| other_apps::Unread {
        foreign_id,
        name: Some(entry.name.clone()),
        reason,
    };
    let recipe =
        unpack_recipe(archive, entry.index, budget).map_err(|reason| unread(None, reason))?;
    let foreign_id = text(&recipe, "uid");
    if !is_paprika_recipe(&recipe) || !is_known_shape(&recipe) {
        return Err(unread(
            foreign_id,
            Refusal::UntestedShape { app: APP }.sentence(),
        ));
    }
    let Some(foreign_id) = foreign_id else {
        return Err(unread(
            None,
            "the recipe carries no Paprika id, so Kamosu could not tell whether it had \
             brought it in before"
                .to_string(),
        ));
    };
    Ok(read_recipe(&recipe, foreign_id))
}

/// One recipe as Kamosu's content, with its photo.
fn read_recipe(recipe: &Value, foreign_id: String) -> other_apps::Recipe {
    let title = text(recipe, "name").unwrap_or_else(|| "Untitled recipe".to_string());
    let ingredients: Vec<Value> = lines(recipe, "ingredients")
        .map(|line| json!({ "kind": "ingredient", "text": line }))
        .collect();
    let steps: Vec<Value> = lines(recipe, "directions")
        .map(|step| json!({ "kind": "step", "text": step, "photo": null }))
        .collect();
    let source = match (text(recipe, "source"), text(recipe, "source_url")) {
        (Some(name), link) => json!({ "text": name, "link": link }),
        (None, Some(link)) => other_apps::source_of_link(&link),
        (None, None) => Value::Null,
    };
    let mut candidate = json!({
        "title": title,
        "prep_time_minutes": minutes(recipe, "prep_time"),
        "cook_time_minutes": minutes(recipe, "cook_time"),
        "source": source,
        "note": as_note(own_note(recipe)),
        "nutrition": null,
        "ingredients": ingredients,
        "steps": steps,
    });
    // The recipe's own Language words its Yield's servings.
    let language = crate::language::detect_content(&candidate);
    candidate["yield"] = text(recipe, "servings")
        .and_then(|servings| recipe_yield(&servings, language))
        .unwrap_or(Value::Null);
    // The Note holds only the cook's own words until the Language is known,
    // which words the lines added to it.
    candidate["note"] = note(recipe, &candidate, language);

    let mut tags: Vec<String> = Vec::new();
    for category in recipe["categories"].as_array().into_iter().flatten() {
        if let Some(name) = category.as_str().map(str::trim).filter(|n| !n.is_empty())
            && !tags.iter().any(|tag| tag == name)
        {
            tags.push(name.to_string());
        }
    }

    let mut left_out = Vec::new();
    // The cook's own picture first; the website's only where there is none.
    let photo = match text(recipe, "photo_data") {
        Some(data) => {
            let data: String = data.split_whitespace().collect();
            match base64::engine::general_purpose::STANDARD.decode(data) {
                Ok(bytes) => Some(other_apps::Photo::Bytes(bytes)),
                Err(_) => {
                    left_out.push(LeftOut::UnreadablePhotos(1));
                    None
                }
            }
        }
        None => text(recipe, "image_url").map(other_apps::Photo::Address),
    };
    let extra_photos = recipe["photos"].as_array().map_or(0, Vec::len);
    if extra_photos > 0 {
        left_out.push(LeftOut::ExtraPhotos(extra_photos));
    }
    if recipe["rating"].as_f64().is_some_and(|rating| rating > 0.0) {
        left_out.push(LeftOut::Ratings(1));
    }
    if recipe["on_favorites"].as_bool() == Some(true)
        || recipe["on_favorites"].as_i64().is_some_and(|mark| mark > 0)
    {
        left_out.push(LeftOut::Favourites(1));
    }
    other_apps::Recipe {
        foreign_id,
        cookings: Vec::new(),
        candidate,
        tags,
        photo,
        step_photos: Vec::new(),
        components: Vec::new(),
        left_out,
    }
}

/// The lines of a block of text, each trimmed, the empty ones left out.
/// Paprika breaks a line with a newline, and sometimes with the Unicode line
/// separator a Mac pastes.
fn lines<'a>(recipe: &'a Value, field: &str) -> impl Iterator<Item = String> + 'a {
    recipe[field]
        .as_str()
        .unwrap_or("")
        .split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
}

/// One of Paprika's free-text times in whole minutes: `1 hr 30 mins`, or a
/// bare number, which Paprika shows as minutes.
fn minutes(recipe: &Value, field: &str) -> Option<i64> {
    let written = text(recipe, field)?;
    let minutes = if written.bytes().all(|b| b.is_ascii_digit()) {
        written.parse::<i64>().ok()?
    } else {
        let lines = [
            text(recipe, "name"),
            text(recipe, "ingredients"),
            text(recipe, "directions"),
        ];
        let language =
            crate::language::detect(&lines.into_iter().flatten().collect::<Vec<_>>().join("\n"))
                .unwrap_or(crate::language::UNKNOWN);
        (crate::units::step_duration(&written, language)? as f64 / 60.0).round() as i64
    };
    (minutes > 0).then_some(minutes)
}

/// The Yield from Paprika's free-text servings: the first count, the lower
/// end of a range, and the words after it as the noun. `Serves 4`, `4`,
/// `Servings: 4` and `4 servings` are all four servings, worded in the
/// recipe's Language; `about 20 rolls` is twenty rolls, and `Two 10-inch
/// pizzas` two `10-inch pizzas`. Text with no count in it, `a big pot` or
/// `(1/2-cup servings)`, is no Yield.
fn recipe_yield(written: &str, language: Option<&str>) -> Option<Value> {
    let (amount, rest) = first_count(written)?;
    let noun = past_range(rest)
        .trim_start_matches([':', ',', ')', ' '])
        .trim();
    let servings = match language {
        Some("fr") => "portions",
        Some("es") => "raciones",
        Some("nl") => "porties",
        _ => "servings",
    };
    let noun = match noun.to_lowercase().as_str() {
        "" | "serving" | "servings" | "people" | "persons" => servings.to_string(),
        _ => noun.to_string(),
    };
    Some(json!({ "amount": amount, "noun": noun }))
}

/// The numbers a cook writes out, which count where a figure after them is a
/// size: `Two 10-inch pizzas`.
const COUNTING_WORDS: [&str; 12] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve",
];

/// The first count in `written`, as the Yield's amount, and the text after
/// it, a range's upper end included.
fn first_count(written: &str) -> Option<(String, &str)> {
    let figure_at = written.find(|c: char| c.is_ascii_digit());
    let word = written
        .split_inclusive(char::is_whitespace)
        .scan(0, |at, word| {
            let start = *at;
            *at += word.len();
            Some((start, word))
        })
        .take_while(|(start, _)| figure_at.is_none_or(|figure| *start < figure))
        .find_map(|(start, word)| {
            let bare = word.trim().trim_matches(|c: char| !c.is_alphabetic());
            let count = COUNTING_WORDS
                .iter()
                .position(|counting| bare.eq_ignore_ascii_case(counting))?;
            Some((count + 1, &written[start + word.len()..]))
        });
    if let Some((count, rest)) = word {
        return Some((count.to_string(), rest.trim_start()));
    }

    let at = figure_at?;
    let figure_end = |text: &str| {
        text.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',' || c == '/'))
            .unwrap_or(text.len())
    };
    let end = at + figure_end(&written[at..]);
    let mut amount = written[at..end]
        .trim_end_matches(['.', ',', '/'])
        .to_string();
    let mut rest = &written[end..];
    // A figure joined to what it measures is a size, not a count:
    // `1/2-cup servings`, `12" pizza`.
    let sized = rest
        .strip_prefix('-')
        .is_some_and(|after| after.starts_with(char::is_alphabetic));
    if sized || rest.starts_with('"') {
        return None;
    }
    // A whole number and a fraction: `1 1/2 cups`.
    if let Some(after) = rest.strip_prefix(' ') {
        let fraction = &after[..figure_end(after)];
        if fraction.contains('/') && fraction.starts_with(|c: char| c.is_ascii_digit()) {
            amount = format!("{amount} {fraction}");
            rest = &after[fraction.len()..];
        }
    }
    Some((amount, rest.trim_start()))
}

/// Whether the Yield read from `written` is the lower end of a range, whose
/// upper end it cannot hold: `Serves 4 to 6`.
fn is_a_range(written: &str) -> bool {
    first_count(written).is_some_and(|(_, rest)| past_range(rest).len() != rest.len())
}

/// The text after the upper end of a range, where `rest` opens with one:
/// `-6`, `– 6`, `to 6`, `to six`. Otherwise `rest` itself.
fn past_range(rest: &str) -> &str {
    for joint in ["-", "–", "to "] {
        let Some(after) = rest.strip_prefix(joint) else {
            continue;
        };
        let after = after.trim_start();
        let figure = after
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',' || c == '/'))
            .unwrap_or(after.len());
        if figure > 0 {
            return after[figure..].trim_start();
        }
        let word = after
            .find(|c: char| !c.is_alphabetic())
            .unwrap_or(after.len());
        if COUNTING_WORDS
            .iter()
            .any(|counting| after[..word].eq_ignore_ascii_case(counting))
        {
            return after[word..].trim_start();
        }
        return rest;
    }
    rest
}

/// The description and Paprika's own notes, each a paragraph.
fn own_note(recipe: &Value) -> Vec<String> {
    ["description", "notes"]
        .iter()
        .filter_map(|field| text(recipe, field))
        .collect()
}

/// The Note: the description and Paprika's own notes, and then each thing a
/// cook typed that no field of `candidate` took (#434), worded in the
/// recipe's Language: the total time, a prep or cook time Kamosu could not
/// read, servings no Yield says whole, the difficulty and the nutrition. Each
/// is a paragraph.
fn note(recipe: &Value, candidate: &Value, language: Option<&str>) -> Value {
    let words = crate::schema_org::words(language);
    let mut paragraphs = own_note(recipe);
    if let Some(total) = typed_time(recipe, "total_time") {
        paragraphs.push(format!("{} {total}", words.total));
    }
    // A time that landed is said again only where its field could not hold
    // all of it: `35 to 40 mins`, `40 mins (plus proving)`.
    for (field, lead) in [("prep_time", words.prep), ("cook_time", words.cook)] {
        if let Some(written) = typed_time(recipe, field)
            && (candidate[format!("{field}_minutes")].is_null() || !is_only_a_duration(&written))
        {
            paragraphs.push(format!("{lead} {written}"));
        }
    }
    if let Some(servings) = text(recipe, "servings")
        && (candidate["yield"].is_null() || is_a_range(&servings))
    {
        paragraphs.push(format!("{} {servings}", words.servings_label));
    }
    if let Some(difficulty) = text(recipe, "difficulty") {
        paragraphs.push(format!("{} {difficulty}", words.difficulty));
    }
    // Free text, written as it stands; a cook who labelled it is not
    // labelled again.
    if let Some(nutrition) = text(recipe, "nutritional_info") {
        let labelled = nutrition
            .lines()
            .next()
            .is_some_and(|first| first.to_lowercase().contains("nutri"));
        paragraphs.push(if labelled {
            nutrition
        } else {
            format!("{} {nutrition}", words.nutrition)
        });
    }
    as_note(paragraphs)
}

/// The words a time is made of where it says nothing but its length.
const DURATION_WORDS: [&str; 26] = [
    "h", "hr", "hrs", "hour", "hours", "heure", "heures", "hora", "horas", "m", "min", "mins",
    "minute", "minutes", "minuto", "minutos", "s", "sec", "secs", "second", "seconds", "and", "et",
    "y", "seconde", "secondes",
];

/// Whether `written` says a length of time and nothing else, so the minutes
/// read from it hold all of it: `1 hr 30 mins`, `1 1/2 hours`, `3 h, 30 m`.
/// A range, a word such as `about` or `overnight`, or a note in brackets is
/// more than the minutes hold.
fn is_only_a_duration(written: &str) -> bool {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut numeric = false;
    for c in written.chars() {
        let is_numeric = c.is_ascii_digit() || c == '.' || c == '/';
        if c.is_whitespace() || c == ',' {
            words.push(std::mem::take(&mut word));
        } else if !word.is_empty() && is_numeric != numeric {
            words.push(std::mem::replace(&mut word, c.to_string()));
        } else {
            word.push(c);
        }
        numeric = is_numeric;
    }
    words.push(word);
    let words: Vec<String> = words
        .into_iter()
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect();
    words.iter().all(|word| {
        word.starts_with(|c: char| c.is_ascii_digit()) || DURATION_WORDS.contains(&word.as_str())
    })
}

/// Paragraphs as a Note, or none.
fn as_note(paragraphs: Vec<String>) -> Value {
    if paragraphs.is_empty() {
        Value::Null
    } else {
        json!(paragraphs.join("\n\n"))
    }
}

/// One of Paprika's times as the cook typed it, where it says anything: `0`
/// and `0 minutes` are no time at all.
fn typed_time(recipe: &Value, field: &str) -> Option<String> {
    text(recipe, field).filter(|written| {
        let mut digits = written.chars().filter(char::is_ascii_digit).peekable();
        digits.peek().is_none() || digits.any(|digit| digit != '0')
    })
}

/// A trimmed, non-empty text field.
fn text(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::other_apps::{Photo, Recipe, Unread};
    use std::io::{Read, Write};

    fn sample() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/paprika/mealie-sample.zip");
        std::fs::read(path).unwrap()
    }

    /// The real sample with the recipe titled `title` changed by `change`, as
    /// Paprika would have written it had the cook done so: every other file
    /// as it was.
    fn sample_with(title: &str, change: impl Fn(&mut Value)) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(sample())).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                if name.ends_with(".paprikarecipe") && !name.contains("__MACOSX") {
                    let mut json = String::new();
                    flate2::read::GzDecoder::new(bytes.as_slice())
                        .read_to_string(&mut json)
                        .unwrap();
                    let mut recipe: Value = serde_json::from_str(&json).unwrap();
                    if recipe["name"].as_str().unwrap().trim() == title {
                        change(&mut recipe);
                        let mut gz = flate2::write::GzEncoder::new(
                            Vec::new(),
                            flate2::Compression::default(),
                        );
                        gz.write_all(recipe.to_string().as_bytes()).unwrap();
                        bytes = gz.finish().unwrap();
                    }
                }
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(&bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }

    fn read_all(bytes: Vec<u8>) -> Vec<Result<Recipe, Unread>> {
        let export = Paprika
            .read(&Handed::bytes(bytes))
            .unwrap_or_else(|_| panic!("the file was refused"))
            .unwrap_or_else(|| panic!("the file was not recognised"));
        assert_eq!(export.app, "paprika");
        let total = export.total;
        let recipes: Vec<_> = export.recipes.collect();
        assert_eq!(recipes.len(), total);
        recipes
    }

    fn titled<'a>(recipes: &'a [Result<Recipe, Unread>], title: &str) -> &'a Recipe {
        recipes
            .iter()
            .flatten()
            .find(|recipe| recipe.candidate["title"] == title)
            .unwrap_or_else(|| panic!("no {title}"))
    }

    fn texts(recipe: &Recipe, field: &str) -> Vec<String> {
        recipe.candidate[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["text"].as_str().unwrap().to_string())
            .collect()
    }

    fn left_out(recipe: &Recipe) -> Vec<String> {
        recipe
            .left_out
            .iter()
            .map(|left| match left {
                LeftOut::ExtraPhotos(n) => format!("extra photos {n}"),
                LeftOut::UnreadablePhotos(n) => format!("unreadable photos {n}"),
                LeftOut::Ratings(1) => "rating".to_string(),
                LeftOut::Favourites(1) => "favourite".to_string(),
                LeftOut::Nutrition => "nutrition".to_string(),
                _ => "something else".to_string(),
            })
            .collect()
    }

    #[test]
    fn every_recipe_of_mealies_sample_is_read_with_its_own_photo() {
        let recipes = read_all(sample());
        assert_eq!(recipes.len(), 5);
        for recipe in &recipes {
            let recipe = recipe
                .as_ref()
                .unwrap_or_else(|unread| panic!("{}", unread.reason));
            assert!(
                matches!(&recipe.photo, Some(Photo::Bytes(bytes)) if bytes.starts_with(&[0xff, 0xd8, 0xff])),
                "{} has no photo of its own",
                recipe.candidate["title"]
            );
            assert!(recipe.left_out.is_empty(), "{}", recipe.candidate["title"]);
        }
    }

    #[test]
    fn a_recipe_reads_field_by_field() {
        let recipes = read_all(sample());
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert_eq!(
            puffer.foreign_id,
            "D818BC90-6060-4399-9167-D3B23E0C3F3A-39102-0000D5ABA0F928D4"
        );
        assert_eq!(
            texts(puffer, "ingredients"),
            [
                "400 g Zucchini",
                "400 g Kartoffel(n)",
                "200 g Frühlingszwiebel(n)",
                "2 Ei(er)",
                "2 EL Mehl",
                "Salz",
                "Pfeffer",
                "Crème fraîche",
                "Fett"
            ]
        );
        assert_eq!(texts(puffer, "steps").len(), 1);
        assert!(texts(puffer, "steps")[0].starts_with("Zucchini und Kartoffeln grob raffeln."));
        assert_eq!(
            puffer.candidate["yield"],
            json!({ "amount": "4", "noun": "servings" })
        );
        assert_eq!(
            puffer.candidate["source"],
            json!({
                "text": "Chefkoch.de",
                "link": "http://www.chefkoch.de/rezepte/317951113441309/Zucchini.html"
            })
        );
        assert_eq!(puffer.candidate["prep_time_minutes"], Value::Null);
        assert_eq!(puffer.candidate["note"], Value::Null);
        assert_eq!(puffer.candidate["nutrition"], Value::Null);
        assert_eq!(
            puffer.tags,
            [
                "Alltagstauglich",
                "Kartoffelgerichte",
                "Snacks",
                "Vegetarisch"
            ]
        );

        let rolls = titled(&recipes, "Zucchini Rolls with Goat Cheese and Mint");
        assert_eq!(rolls.candidate["prep_time_minutes"], 30);
        assert_eq!(rolls.candidate["cook_time_minutes"], 30);
        assert_eq!(
            rolls.candidate["yield"],
            json!({ "amount": "20", "noun": "rolls" })
        );
        // A trailing newline Paprika kept on a name is not part of the title.
        titled(&recipes, "Zucchini-Kartoffel-Frittata");
    }

    #[test]
    fn directions_become_steps_and_lines_arrive_as_written() {
        let recipes = read_all(sample());
        let rolls = titled(&recipes, "Zucchini Rolls with Goat Cheese and Mint");
        let steps = texts(rolls, "steps");
        assert_eq!(steps.len(), 4);
        assert!(steps[0].starts_with("1. Bring a small saucepan"));
        assert!(steps[3].starts_with("4. Place 1 piece of arugula"));
        // A line that reads like a heading stays an Ingredient Line.
        let braten = titled(&recipes, "Zwiebelrostbraten mit selbstgemachten Spätzle");
        let lines = texts(braten, "ingredients");
        assert_eq!(lines[0], "für den Braten:");
        assert!(lines.contains(&"für die Spätzle:".to_string()));
        assert!(
            braten.candidate["ingredients"]
                .as_array()
                .unwrap()
                .iter()
                .all(|line| line["kind"] == "ingredient")
        );
        // A Mac's line separator breaks a Step as a newline does.
        let hähnchen = titled(
            &recipes,
            "Zweimal gebratenes Hähnchen mit Gemüse und Nudeln",
        );
        let steps = texts(hähnchen, "steps");
        assert_eq!(steps.len(), 10);
        assert!(steps.iter().all(|step| !step.contains('\u{2028}')));
    }

    #[test]
    fn the_website_photo_is_handed_over_only_where_the_file_holds_none() {
        let recipes = read_all(sample());
        let rolls = titled(&recipes, "Zucchini Rolls with Goat Cheese and Mint");
        // It has both, and its own wins.
        assert!(matches!(rolls.photo, Some(Photo::Bytes(_))));

        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["photo_data"] = Value::Null;
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert!(matches!(
            &puffer.photo,
            Some(Photo::Address(address))
                if address == "http://cdn.chefkoch.de/ck.de/rezepte/31/31795/274746-bigfix-zucchini.jpg"
        ));
    }

    #[test]
    fn extra_photos_a_rating_and_a_favourite_are_counted() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["rating"] = json!(4);
            recipe["on_favorites"] = json!(true);
            let photo = recipe["photo_data"].clone();
            recipe["photos"] = json!([
                { "name": "1", "filename": "a.jpg", "hash": "a", "data": photo },
                { "name": "2", "filename": "b.jpg", "hash": "b", "data": photo },
            ]);
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert_eq!(left_out(puffer), ["extra photos 2", "rating", "favourite"]);
    }

    #[test]
    fn a_difficulty_is_kept_in_the_note() {
        let recipes = read_all(sample());
        let frittata = titled(&recipes, "Zucchini-Kartoffel-Frittata");
        assert_eq!(frittata.candidate["note"], json!("Difficulty: Easy"));
    }

    #[test]
    fn what_no_field_holds_goes_into_the_note_word_for_word() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["description"] = json!("Crisp and quick.");
            recipe["notes"] = json!("Fry in batches.");
            recipe["total_time"] = json!("1 hr 15 mins");
            recipe["prep_time"] = json!("1 day");
            recipe["cook_time"] = json!("20 mins");
            recipe["difficulty"] = json!("Medium");
            recipe["nutritional_info"] = json!("Calories: 250\nProtein: 8 g");
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        // A cook time that landed in its field is not said again.
        assert_eq!(puffer.candidate["cook_time_minutes"], 20);
        assert_eq!(puffer.candidate["prep_time_minutes"], Value::Null);
        assert_eq!(
            puffer.candidate["note"],
            json!(
                "Crisp and quick.\n\nFry in batches.\n\nTotal time: 1 hr 15 mins\n\n\
                 Prep time: 1 day\n\nDifficulty: Medium\n\nNutrition: Calories: 250\n\
                 Protein: 8 g"
            )
        );
        // Nothing of the nutrition stays in Paprika, so the Report counts none.
        assert!(left_out(puffer).is_empty());
        assert_eq!(puffer.candidate["nutrition"], Value::Null);
    }

    #[test]
    fn a_total_time_is_always_kept_as_typed() {
        let recipes = read_all(sample_with(
            "Zucchini Rolls with Goat Cheese and Mint",
            |recipe| recipe["total_time"] = json!("60"),
        ));
        let rolls = titled(&recipes, "Zucchini Rolls with Goat Cheese and Mint");
        // Prep and cook both landed, and the total is kept all the same.
        assert_eq!(rolls.candidate["note"], json!("Total time: 60"));
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["total_time"] = json!("0 minutes");
            recipe["cook_time"] = json!("0");
        }));
        assert_eq!(
            titled(&recipes, "Zucchinipuffer").candidate["note"],
            Value::Null
        );
    }

    #[test]
    fn a_time_that_landed_in_part_is_kept_whole_in_the_note() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["prep_time"] = json!("40 mins (plus proving)");
            recipe["cook_time"] = json!("35 to 40 mins");
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert_eq!(puffer.candidate["prep_time_minutes"], 40);
        assert_eq!(puffer.candidate["cook_time_minutes"], 35);
        assert_eq!(
            puffer.candidate["note"],
            json!("Prep time: 40 mins (plus proving)\n\nCook time: 35 to 40 mins")
        );
    }

    #[test]
    fn a_time_said_whole_is_only_a_duration() {
        for whole in [
            "30 minutes",
            "1 hr 30 mins",
            "1hr 20mins",
            "4.5hrs",
            "1 1/2 hours",
            "3 h, 30 m",
            "20",
            "1 heure 30 minutes",
        ] {
            assert!(is_only_a_duration(whole), "{whole}");
        }
        for more in [
            "about 1 1/2 hours",
            "35 to 40 mins",
            "1-2hrs",
            "1 hour + salting",
            "40 mins (plus proving)",
            "10 servings 30 minutes",
        ] {
            assert!(!is_only_a_duration(more), "{more}");
        }
    }

    #[test]
    fn nutrition_already_labelled_is_not_labelled_twice() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["nutritional_info"] = json!("Nutritional Info:\nCalories: 400");
        }));
        assert_eq!(
            titled(&recipes, "Zucchinipuffer").candidate["note"],
            json!("Nutritional Info:\nCalories: 400")
        );
    }

    #[test]
    fn servings_no_yield_says_whole_are_kept_as_written() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["servings"] = json!("a big pot");
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert_eq!(puffer.candidate["yield"], Value::Null);
        assert_eq!(puffer.candidate["note"], json!("Servings: a big pot"));
        // A range's upper end has nowhere else to go.
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["servings"] = json!("Serves 4 to 6");
        }));
        let puffer = titled(&recipes, "Zucchinipuffer");
        assert_eq!(
            puffer.candidate["yield"],
            json!({ "amount": "4", "noun": "servings" })
        );
        assert_eq!(puffer.candidate["note"], json!("Servings: Serves 4 to 6"));
    }

    #[test]
    fn a_french_recipes_lines_are_worded_in_french() {
        let recipes = read_all(sample_with("Zucchinipuffer", |recipe| {
            recipe["name"] = json!("Galettes de courgettes");
            recipe["ingredients"] = json!(
                "400 g de courgettes\n400 g de pommes de terre\n2 œufs\n2 cuillères à soupe de farine\nsel et poivre"
            );
            recipe["directions"] = json!(
                "Râper les courgettes et les pommes de terre, puis bien les égoutter.\nAjouter les œufs et la farine, saler et poivrer, et faire cuire dans la poêle."
            );
            recipe["total_time"] = json!("50 minutes");
            recipe["difficulty"] = json!("Facile");
            recipe["servings"] = json!("pour une tablée");
        }));
        assert_eq!(
            titled(&recipes, "Galettes de courgettes").candidate["note"],
            json!("Temps total : 50 minutes\n\nPortions : pour une tablée\n\nDifficulté : Facile")
        );
    }

    #[test]
    fn a_paprika_recipe_in_another_shape_is_refused_with_the_update_reason() {
        let changed = |recipe: &mut Value| recipe["ingredients"] = json!(["400 g Zucchini"]);
        // Read first, it says the whole file is in a shape no test file has.
        let bytes = sample_with("Zucchini Rolls with Goat Cheese and Mint", changed);
        let refused = Paprika.read(&Handed::bytes(bytes)).err().unwrap();
        assert_eq!(refused, Refusal::UntestedShape { app: APP });
        // Further on, it is that recipe's alone.
        let recipes = read_all(sample_with(
            "Zwiebelrostbraten mit selbstgemachten Spätzle",
            changed,
        ));
        let unread: Vec<_> = recipes.iter().filter_map(|r| r.as_ref().err()).collect();
        assert_eq!(unread.len(), 1);
        assert!(unread[0].reason.contains("update Paprika"));
    }

    #[test]
    fn another_apps_zip_is_not_paprikas() {
        for name in ["mealie/v3-export.zip", "mealie/pre-v1-export.zip"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name);
            assert!(
                matches!(Paprika.read(&Handed::file(path)), Ok(None)),
                "{name}"
            );
        }
        assert!(matches!(
            Paprika.read(&Handed::bytes(b"not a zip".to_vec())),
            Ok(None)
        ));
    }

    #[test]
    fn a_yield_reads_from_free_text() {
        let read = |written: &str| {
            recipe_yield(written, None).map(|y| {
                format!(
                    "{} {}",
                    y["amount"].as_str().unwrap(),
                    y["noun"].as_str().unwrap()
                )
            })
        };
        assert_eq!(read("4").as_deref(), Some("4 servings"));
        assert_eq!(read("Serves 4").as_deref(), Some("4 servings"));
        assert_eq!(read("Servings: 6 servings").as_deref(), Some("6 servings"));
        assert_eq!(read("Yield: 8 servings").as_deref(), Some("8 servings"));
        assert_eq!(read("Serves 4 to 6").as_deref(), Some("4 servings"));
        assert_eq!(read("4-6").as_deref(), Some("4 servings"));
        assert_eq!(read("about 20 rolls").as_deref(), Some("20 rolls"));
        assert_eq!(read("1 loaf").as_deref(), Some("1 loaf"));
        assert_eq!(read("500ml").as_deref(), Some("500 ml"));
        assert_eq!(read("a big pot"), None);
        assert_eq!(read("About 1 1/2 cups").as_deref(), Some("1 1/2 cups"));
        assert_eq!(
            read("Two 10-inch pizzas").as_deref(),
            Some("2 10-inch pizzas")
        );
        assert_eq!(
            read("Eight 6-ounce cookies").as_deref(),
            Some("8 6-ounce cookies")
        );
        assert_eq!(read("Serves four").as_deref(), Some("4 servings"));
        assert_eq!(read("Servings (1/2-cup servings)"), None);
        assert_eq!(
            read("12 8\" Tortillas").as_deref(),
            Some("12 8\" Tortillas")
        );
        assert_eq!(read("Someone's 4").as_deref(), Some("4 servings"));
        assert_eq!(read("Serves four to six").as_deref(), Some("4 servings"));
        assert_eq!(read("Makes four - six loaves").as_deref(), Some("4 loaves"));
        let french = recipe_yield("6 personnes", Some("fr")).unwrap();
        assert_eq!(french, json!({ "amount": "6", "noun": "personnes" }));
        let french = recipe_yield("6", Some("fr")).unwrap();
        assert_eq!(french, json!({ "amount": "6", "noun": "portions" }));
    }

    #[test]
    fn a_time_reads_from_free_text() {
        let read = |written: &str| minutes(&json!({ "prep_time": written }), "prep_time");
        assert_eq!(read("15 mins"), Some(15));
        assert_eq!(read("1 hr 30 mins"), Some(90));
        assert_eq!(read("1 1/2 hours"), Some(90));
        assert_eq!(read("15"), Some(15));
        assert_eq!(read("0 minutes"), None);
        assert_eq!(read("0"), None);
        assert_eq!(read("inf"), None);
        assert_eq!(read("1e30"), None);
        assert_eq!(read(""), None);
    }
}
