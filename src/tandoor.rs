//! Reading a Tandoor export (#415, the spec is #411): Tandoor's default
//! format, made with Settings › Space › Export, type Default, All Recipes. It
//! is a zip of zips, one per recipe, each named after the recipe's id and
//! holding its `recipe.json` and its picture.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files described in `tests/fixtures/tandoor/README.md`.
//!
//! - **The recipe is known by its inner zip's name and its own**,
//!   `102487 Rohlik s maslem`: Tandoor numbers recipes from 1 in every
//!   install, so the number alone would mix two Tandoors' recipes. Its
//!   picture is the file whose name begins `image`: Tandoor 1 wrote
//!   `image.jpeg`, Tandoor 2.0 `image` with no extension, Tandoor 2.6
//!   `image.jpg`.
//! - **Tandoor keeps ingredients inside each step, and no written line.** They
//!   come together into one list. A named step's ingredients are a Section
//!   named after the step, and so is a header row. An unnamed step's
//!   ingredients after a Section are a Section of their own, named by the
//!   step's number as Tandoor numbers its steps, so they are not filed under
//!   the step before. Each line is rebuilt from amount, unit, food and note
//!   the way the Crouton reader rebuilds one: `225 g butter, soft`.
//! - **A step keeps its text**, as written, and its name is a Section above
//!   it. A step's own time, which Kamosu keeps nowhere else, is a last line of
//!   its text, where cooking mode makes a timer of it.
//! - **Working time is the prep time and waiting time the cook time.**
//! - **The calories are the Nutrition**, per serving as Tandoor counts it. The
//!   other figures go into the Note as one line (#434). A figure of zero is
//!   one nobody filled in, which is what Tandoor stores for an empty field.
//! - **The description becomes the Note, keywords Tags, servings the Yield.**
//!
//! Every real file has one shape: the fields below. Tandoor 1 wrote no
//! `source_url`; Tandoor 2.0 added it, and 2.6 left out two display settings
//! of each line. A file of Tandoor's in any other shape is refused with the
//! update reason.

use std::io::Read;

use serde_json::{Value, json};

use crate::other_apps::{self, Budget, Handed, LeftOut, Reader, Refusal};

/// Tandoor among the readers of another app's export.
pub struct Tandoor;

const APP: &str = "Tandoor";

/// The largest single file read out of the export, a recipe's zip or a file
/// inside it. The largest real one is a 128 KB zip, its picture inside it, so
/// this refuses only a file that claims to be enormous.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

type Archive = zip::ZipArchive<Box<dyn other_apps::ReadSeek>>;

impl Reader for Tandoor {
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
        if entries.is_empty() {
            return Ok(None);
        }
        let mut budget = Budget::new(other_apps::MOST_UNPACKED);
        // The first recipe that reads says whose file this is, and in which
        // shape. One damaged recipe is that recipe's loss, not the file's.
        let mut first = None;
        for entry in &entries {
            if let Ok(inner) = unpack(&mut archive, entry.index, &mut budget)
                && let Ok((recipe, _)) = open_recipe(inner, &mut budget)
            {
                first = Some(recipe);
                break;
            }
        }
        let Some(first) = first else {
            return Ok(None);
        };
        if !is_tandoor_recipe(&first) {
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
            app: "tandoor",
            total,
            recipes: Box::new(recipes),
            left_out: Vec::new(),
        }))
    }
}

/// One recipe's zip in the export: its archive index, and its name without
/// `.zip`, which is the recipe's id in Tandoor.
struct Entry {
    index: usize,
    id: String,
}

/// Every `.zip` in the export, in the order of the ids Tandoor gave them,
/// which is the order the recipes were made in. Tandoor puts them at the top,
/// and a cook whose Mac unpacked the download and who zipped the folder again
/// has them in that folder, beside `__MACOSX/` copies that are not recipes.
fn recipe_entries(archive: &Archive) -> Vec<Entry> {
    let mut entries: Vec<Entry> = (0..archive.len())
        .filter_map(|index| {
            let path = archive.name_for_index(index)?;
            if path.starts_with("__MACOSX/") {
                return None;
            }
            let file = path.rsplit('/').next().unwrap_or(path);
            let id = file
                .len()
                .checked_sub(".zip".len())
                .filter(|&at| file.is_char_boundary(at))
                .filter(|&at| file[at..].eq_ignore_ascii_case(".zip"))
                .map(|at| &file[..at])?;
            (!id.is_empty() && !file.starts_with("._")).then(|| Entry {
                index,
                id: id.to_string(),
            })
        })
        .collect();
    entries
        .sort_by(|a, b| (a.id.parse::<u64>().ok(), &a.id).cmp(&(b.id.parse::<u64>().ok(), &b.id)));
    entries
}

const DAMAGED: &str = "the file is damaged and none of it could be read; export the \
                       recipes from Tandoor again and import that";

/// The sentence for an export that unpacks to more than its budget.
fn too_much(budget: &Budget) -> String {
    format!(
        "the export unpacks to more than {} GB in all, more than any Tandoor export \
         holds, so this recipe and those after it were not read",
        budget.most() / (1000 * 1000 * 1000)
    )
}

/// Unpack one file of an archive through the export's budget. The error is a
/// sentence for the Report.
fn unpack<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    index: usize,
    budget: &mut Budget,
) -> Result<Vec<u8>, String> {
    let Ok(mut entry) = archive.by_index(index) else {
        return Err(DAMAGED.to_string());
    };
    match budget.unpack(&mut entry, MAX_FILE_BYTES) {
        Err(other_apps::TooMuch) => Err(too_much(budget)),
        Ok((bytes, _)) if bytes.len() as u64 > MAX_FILE_BYTES => {
            Err("a file in it is too large to be one recipe".to_string())
        }
        Ok((_, false)) => Err(DAMAGED.to_string()),
        Ok((bytes, true)) => Ok(bytes),
    }
}

/// A recipe's zip opened: its `recipe.json`, and its picture's bytes where it
/// has one and they unpacked. A zip inside the export is as deep as Tandoor
/// nests, so nothing in it is opened as an archive.
fn open_recipe(inner: Vec<u8>, budget: &mut Budget) -> Result<(Value, Picture), String> {
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(inner)) else {
        return Err(DAMAGED.to_string());
    };
    let Some(json_at) = archive.index_for_name("recipe.json") else {
        return Err(
            "the recipe's zip holds no recipe.json, so it is not one Tandoor \
                    wrote: export the recipes from Tandoor again and import that"
                .to_string(),
        );
    };
    let recipe = unpack(&mut archive, json_at, budget)?;
    let recipe: Value = serde_json::from_slice(&recipe).map_err(|_| {
        "the recipe is not one Kamosu can read: it may be damaged, so export it from \
         Tandoor again and import that"
            .to_string()
    })?;
    // The first file named `image…`, as Tandoor itself takes it back in.
    let image_at = (0..archive.len()).find(|&index| {
        archive
            .name_for_index(index)
            .is_some_and(|name| name.starts_with("image"))
    });
    let picture = match image_at {
        None => Picture::None,
        Some(index) => match unpack(&mut archive, index, budget) {
            Ok(bytes) => Picture::Bytes(bytes),
            Err(_) => Picture::Unreadable,
        },
    };
    Ok((recipe, picture))
}

/// A recipe's picture as its zip holds it.
enum Picture {
    None,
    Bytes(Vec<u8>),
    Unreadable,
}

/// Whether a recipe's JSON is one Tandoor wrote, in whichever shape.
fn is_tandoor_recipe(recipe: &Value) -> bool {
    recipe["steps"].is_array()
        && recipe["keywords"].is_array()
        && recipe.get("working_time").is_some()
        && recipe.get("waiting_time").is_some()
}

/// Whether a recipe has the shape every real file has.
fn is_known_shape(recipe: &Value) -> bool {
    let text_or_null = |value: &Value| value.is_string() || value.is_null();
    let named_or_null = |value: &Value| value.is_null() || value["name"].is_string();
    let number_or_null = |value: &Value| value.is_number() || value.is_null();
    let line = |line: &Value| {
        number_or_null(&line["amount"])
            && text_or_null(&line["note"])
            && named_or_null(&line["food"])
            && named_or_null(&line["unit"])
            && line["is_header"].is_boolean()
    };
    let step = |step: &Value| {
        text_or_null(&step["name"])
            && text_or_null(&step["instruction"])
            && number_or_null(&step["time"])
            && step["ingredients"]
                .as_array()
                .is_some_and(|lines| lines.iter().all(line))
    };
    let nutrition = &recipe["nutrition"];
    recipe["name"].is_string()
        && text_or_null(&recipe["description"])
        && text_or_null(&recipe["servings_text"])
        && text_or_null(&recipe["source_url"])
        && number_or_null(&recipe["working_time"])
        && number_or_null(&recipe["waiting_time"])
        && number_or_null(&recipe["servings"])
        && (nutrition.is_null() || nutrition.is_object())
        && recipe["keywords"]
            .as_array()
            .is_some_and(|keywords| keywords.iter().all(|k| k["name"].is_string()))
        && recipe["steps"]
            .as_array()
            .is_some_and(|steps| steps.iter().all(step))
}

/// Read one recipe's zip.
fn read_entry(
    archive: &mut Archive,
    entry: &Entry,
    budget: &mut Budget,
) -> Result<other_apps::Recipe, other_apps::Unread> {
    let unread = |name: Option<String>, reason: String| other_apps::Unread {
        foreign_id: Some(entry.id.clone()),
        name,
        reason,
    };
    let inner = unpack(archive, entry.index, budget).map_err(|reason| unread(None, reason))?;
    let (recipe, picture) = open_recipe(inner, budget).map_err(|reason| unread(None, reason))?;
    if !is_tandoor_recipe(&recipe) || !is_known_shape(&recipe) {
        return Err(unread(
            text(&recipe, "name"),
            Refusal::UntestedShape { app: APP }.sentence(),
        ));
    }
    Ok(read_recipe(
        &recipe,
        foreign_id(&entry.id, &recipe),
        picture,
    ))
}

/// What Kamosu knows the recipe by: its id and its name, `1 Orange cake`.
/// Every Tandoor numbers its recipes from 1, so the id alone would take recipe
/// 1 of a second Tandoor for recipe 1 of the first and offer it as a change to
/// a different recipe. The cost is that a recipe renamed in Tandoor between
/// two imports arrives a second time, which a cook sees and deletes.
fn foreign_id(id: &str, recipe: &Value) -> String {
    match text(recipe, "name") {
        Some(name) => format!("{id} {name}"),
        None => id.to_string(),
    }
}

/// One recipe as Kamosu's content, with its photo.
fn read_recipe(recipe: &Value, foreign_id: String, picture: Picture) -> other_apps::Recipe {
    // The recipe's own Language words its Yield, its Sections by number, its
    // steps' times and its nutrition line, so it is read from what the cook
    // wrote first, none of those words in it.
    let language = crate::language::detect_content(&as_written(recipe));
    let candidate = content(recipe, language);

    let mut tags: Vec<String> = Vec::new();
    for keyword in recipe["keywords"].as_array().into_iter().flatten() {
        if let Some(name) = text(keyword, "name")
            && !tags.contains(&name)
        {
            tags.push(name);
        }
    }

    let mut left_out = Vec::new();
    let photo = match picture {
        Picture::None => None,
        Picture::Bytes(bytes) => Some(other_apps::Photo::Bytes(bytes)),
        Picture::Unreadable => {
            left_out.push(LeftOut::UnreadablePhotos(1));
            None
        }
    };

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

/// The words a recipe's Language puts around what Tandoor kept.
struct Words {
    servings: &'static str,
    step: &'static str,
    time: &'static str,
    minutes: &'static str,
    nutrition: &'static str,
    carbohydrates: &'static str,
    fats: &'static str,
    proteins: &'static str,
}

/// Tandoor's own names for each, in its English, French and Spanish screens.
fn words(language: Option<&str>) -> Words {
    match language {
        Some("fr") => Words {
            servings: "portions",
            step: "Étape",
            time: "Temps :",
            minutes: "minutes",
            nutrition: "Valeurs nutritionnelles :",
            carbohydrates: "glucides",
            fats: "matières grasses",
            proteins: "protéines",
        },
        Some("es") => Words {
            servings: "raciones",
            step: "Paso",
            time: "Tiempo:",
            minutes: "minutos",
            nutrition: "Nutrición:",
            carbohydrates: "carbohidratos",
            fats: "grasas",
            proteins: "proteínas",
        },
        _ => Words {
            servings: "servings",
            step: "Step",
            time: "Time:",
            minutes: "minutes",
            nutrition: "Nutrition:",
            carbohydrates: "carbohydrates",
            fats: "fats",
            proteins: "proteins",
        },
    }
}

/// The cook's own words, as content, which the recipe's Language is read
/// from: the title, the description, every line and every step's text.
fn as_written(recipe: &Value) -> Value {
    let steps = recipe["steps"].as_array().into_iter().flatten();
    let lines: Vec<Value> = steps
        .clone()
        .flat_map(|step| step["ingredients"].as_array().into_iter().flatten())
        .filter(|line| line["is_header"].as_bool() != Some(true))
        .map(|line| json!({ "kind": "ingredient", "text": rebuild_line(line) }))
        .collect();
    let instructions: Vec<Value> = steps
        .filter_map(|step| text(step, "instruction"))
        .map(|instruction| json!({ "kind": "step", "text": instruction, "photo": null }))
        .collect();
    json!({
        "title": text(recipe, "name"),
        "note": text(recipe, "description"),
        "ingredients": lines,
        "steps": instructions,
    })
}

/// The recipe content, its added words in `language`.
fn content(recipe: &Value, language: Option<&str>) -> Value {
    let words = words(language);
    let mut steps: Vec<&Value> = recipe["steps"].as_array().into_iter().flatten().collect();
    steps.sort_by_key(|step| step["order"].as_i64().unwrap_or(0));

    let mut ingredients = Vec::new();
    let mut step_rows = Vec::new();
    for (place, step) in steps.iter().enumerate() {
        let name = text(step, "name");
        let mut lines: Vec<&Value> = step["ingredients"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        lines.sort_by_key(|line| line["order"].as_i64().unwrap_or(0));
        if !lines.is_empty() {
            let in_a_section = ingredients
                .iter()
                .any(|row: &Value| row["kind"] == "section");
            match &name {
                Some(name) => ingredients.push(json!({ "kind": "section", "text": name })),
                None if in_a_section => ingredients.push(
                    json!({ "kind": "section", "text": format!("{} {}", words.step, place + 1) }),
                ),
                None => {}
            }
        }
        for line in lines {
            if line["is_header"].as_bool() == Some(true) {
                if let Some(heading) = text(line, "note").or_else(|| text(&line["food"], "name")) {
                    ingredients.push(json!({ "kind": "section", "text": heading }));
                }
                continue;
            }
            let written = rebuild_line(line);
            if !written.is_empty() {
                ingredients.push(json!({ "kind": "ingredient", "text": written }));
            }
        }

        if let Some(name) = name {
            step_rows.push(json!({ "kind": "section", "text": name, "photo": null }));
        }
        let mut written: Vec<String> = text(step, "instruction").into_iter().collect();
        if let Some(minutes) = whole_minutes(&step["time"]) {
            written.push(format!("{} {minutes} {}", words.time, words.minutes));
        }
        if !written.is_empty() {
            step_rows.push(json!({ "kind": "step", "text": written.join("\n\n"), "photo": null }));
        }
    }

    let recipe_yield = match recipe["servings"]
        .as_f64()
        .filter(|servings| *servings > 0.0)
    {
        Some(servings) => json!({
            "amount": crate::units::as_decimal(servings),
            "noun": text(recipe, "servings_text").unwrap_or_else(|| words.servings.to_string()),
        }),
        None => Value::Null,
    };

    json!({
        "title": text(recipe, "name").unwrap_or_else(|| "Untitled recipe".to_string()),
        "yield": recipe_yield,
        "prep_time_minutes": minutes(&recipe["working_time"]),
        "cook_time_minutes": minutes(&recipe["waiting_time"]),
        "source": text(recipe, "source_url").map_or(Value::Null, |link| other_apps::source_of_link(&link)),
        "note": note(recipe, &words),
        "nutrition": nutrition(recipe),
        "ingredients": ingredients,
        "steps": step_rows,
    })
}

/// Write the line Tandoor does not keep, the way the Crouton reader writes
/// one: amount, unit, food, then the note after a comma. The plural of the
/// unit and the food where there is more than one and Tandoor knows it, and
/// no amount where Tandoor says there is none or holds zero.
fn rebuild_line(line: &Value) -> String {
    let amount = number(&line["amount"])
        .filter(|amount| amount.is_finite() && *amount > 0.0)
        .filter(|_| line["no_amount"].as_bool() != Some(true));
    let many = amount.is_some_and(|amount| amount > 1.0);
    let worded = |part: &Value, plural_setting: &str| {
        let name = text(part, "name")?;
        let plural = many || line[plural_setting].as_bool() == Some(true);
        Some(match text(part, "plural_name") {
            Some(plural_name) if plural => plural_name,
            _ => name,
        })
    };
    let unit = worded(&line["unit"], "always_use_plural_unit");
    let food = worded(&line["food"], "always_use_plural_food");

    let mut parts: Vec<String> = Vec::new();
    if let Some(amount) = amount {
        parts.push(crate::units::amount_for(amount, unit.as_deref()));
    }
    // A note after a food or a unit says more about it; a note after a bare
    // amount, or alone, is what the amount counts.
    let named = unit.is_some() || food.is_some();
    parts.extend(unit);
    parts.extend(food);
    let mut written = parts.join(" ");
    if let Some(note) = text(line, "note") {
        written = match (written.is_empty(), named) {
            (true, _) => note,
            (false, true) => format!("{written}, {note}"),
            (false, false) => format!("{written} {note}"),
        };
    }
    written
}

/// The Note: the description, then the nutrition the Nutrition field does not
/// hold, each a paragraph.
fn note(recipe: &Value, words: &Words) -> Value {
    let mut paragraphs: Vec<String> = text(recipe, "description").into_iter().collect();
    let nutrition = &recipe["nutrition"];
    let figures: Vec<String> = [
        ("carbohydrates", words.carbohydrates),
        ("fats", words.fats),
        ("proteins", words.proteins),
    ]
    .into_iter()
    .filter_map(|(field, name)| {
        let figure = number(&nutrition[field]).filter(|figure| *figure > 0.0)?;
        Some(format!("{name} {}", crate::units::as_decimal(figure)))
    })
    .collect();
    if !figures.is_empty() {
        let mut line = format!("{} {}", words.nutrition, figures.join(", "));
        if let Some(source) = text(nutrition, "source") {
            line.push_str(&format!(" ({source})"));
        }
        paragraphs.push(line);
    }
    if paragraphs.is_empty() {
        Value::Null
    } else {
        json!(paragraphs.join("\n\n"))
    }
}

/// The calories per serving, where Tandoor holds more than none.
fn nutrition(recipe: &Value) -> Value {
    match number(&recipe["nutrition"]["calories"]).filter(|calories| *calories > 0.0) {
        Some(calories) => json!({ "calories": calories, "basis": "per_serving" }),
        None => Value::Null,
    }
}

/// Whole minutes, where there are more than none.
fn minutes(value: &Value) -> Value {
    whole_minutes(value).map_or(Value::Null, |minutes| json!(minutes))
}

fn whole_minutes(value: &Value) -> Option<i64> {
    let minutes = number(value)?;
    (minutes.is_finite() && minutes >= 1.0).then(|| minutes.round() as i64)
}

/// A number, which Tandoor writes as a number and a decimal field may write
/// as text.
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
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
    use std::io::Write;

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tandoor")
            .join(name);
        std::fs::read(path).unwrap()
    }

    /// `name`'s recipes, with the recipe of id `id` changed by `change` as
    /// Tandoor would have written it had the cook done so: every other file
    /// as it was.
    fn fixture_with(name: &str, id: &str, change: impl Fn(&mut Value)) -> Vec<u8> {
        let mut outer = zip::ZipArchive::new(std::io::Cursor::new(fixture(name))).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for index in 0..outer.len() {
                let mut entry = outer.by_index(index).unwrap();
                let entry_name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                if entry_name == format!("{id}.zip") {
                    bytes = rewritten(bytes, &change);
                }
                zip.start_file(entry_name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(&bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        out.into_inner()
    }

    /// One recipe's zip with its `recipe.json` changed.
    fn rewritten(inner: Vec<u8>, change: &impl Fn(&mut Value)) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(inner)).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                if name == "recipe.json" {
                    let mut recipe: Value = serde_json::from_slice(&bytes).unwrap();
                    change(&mut recipe);
                    bytes = recipe.to_string().into_bytes();
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
        let export = Tandoor
            .read(&Handed::bytes(bytes))
            .unwrap_or_else(|_| panic!("the file was refused"))
            .unwrap_or_else(|| panic!("the file was not recognised"));
        assert_eq!(export.app, "tandoor");
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

    /// Each row of a list as `text`, a Section as `## text`.
    fn rows(recipe: &Recipe, field: &str) -> Vec<String> {
        recipe.candidate[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let text = row["text"].as_str().unwrap();
                if row["kind"] == "section" {
                    format!("## {text}")
                } else {
                    text.to_string()
                }
            })
            .collect()
    }

    /// A JPEG, or the WebP Tandoor 2.0 keeps.
    fn has_photo(recipe: &Recipe) -> bool {
        matches!(&recipe.photo, Some(Photo::Bytes(bytes))
            if bytes.starts_with(&[0xff, 0xd8, 0xff])
                || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")))
    }

    #[test]
    fn every_recipe_of_every_real_file_is_read() {
        for (name, count, with_photo) in [
            ("v1-mealie-sample.zip", 1, 1),
            ("v2-export.zip", 10, 5),
            ("v2.6-export.zip", 2, 2),
        ] {
            let recipes = read_all(fixture(name));
            assert_eq!(recipes.len(), count, "{name}");
            let recipes: Vec<&Recipe> = recipes
                .iter()
                .map(|recipe| {
                    recipe
                        .as_ref()
                        .unwrap_or_else(|unread| panic!("{name}: {}", unread.reason))
                })
                .collect();
            assert_eq!(
                recipes.iter().filter(|recipe| has_photo(recipe)).count(),
                with_photo,
                "{name}"
            );
            assert!(recipes.iter().all(|recipe| recipe.left_out.is_empty()));
        }
    }

    #[test]
    fn a_recipe_reads_field_by_field() {
        let recipes = read_all(fixture("v2.6-export.zip"));
        let cake = titled(&recipes, "Orange cake");
        assert_eq!(cake.foreign_id, "1 Orange cake");
        assert_eq!(
            cake.candidate["yield"],
            json!({ "amount": "8", "noun": "slices" })
        );
        assert_eq!(cake.candidate["prep_time_minutes"], 25);
        assert_eq!(cake.candidate["cook_time_minutes"], 45);
        assert_eq!(
            cake.candidate["source"],
            json!({ "text": "example.org", "link": "https://example.org/orange-cake" })
        );
        assert_eq!(
            cake.candidate["nutrition"],
            json!({ "calories": 320.0, "basis": "per_serving" })
        );
        assert_eq!(
            cake.candidate["note"],
            "A moist cake soaked in a sticky orange syrup.\n\n\
             Nutrition: carbohydrates 42, fats 15, proteins 5"
        );
        assert_eq!(cake.tags, ["Baking", "Cake"]);
        assert_eq!(
            rows(cake, "ingredients"),
            [
                "## Cake",
                "## Wet",
                "225 g butter, soft",
                "225 g caster sugar",
                "4 eggs",
                "## Dry",
                "225 g self-raising flour",
                "salt",
                "2 oranges, zest only",
                "## Syrup",
                "2 oranges, juice only",
                "½ cup icing sugar",
            ]
        );
        assert_eq!(
            rows(cake, "steps"),
            [
                "## Cake",
                "Heat the oven to 180°C. Beat the butter and sugar until pale, then beat in the \
                 eggs one at a time.\n\nFold in the flour, the salt and the zest.",
                "Bake in a lined 20 cm tin until a skewer comes out clean.\n\nTime: 45 minutes",
                "## Syrup",
                "Warm the juice with the sugar until it dissolves. Prick the warm cake all over \
                 and pour the syrup on.",
            ]
        );
        assert!(has_photo(cake));
    }

    #[test]
    fn a_french_recipe_is_worded_in_french() {
        let recipes = read_all(fixture("v2.6-export.zip"));
        let pasta = titled(&recipes, "Pâtes à la tomate");
        assert_eq!(
            pasta.candidate["yield"],
            json!({ "amount": "2", "noun": "portions" })
        );
        assert_eq!(pasta.candidate["source"], Value::Null);
        assert_eq!(
            pasta.candidate["note"],
            "Le plat du mardi soir.\n\n\
             Valeurs nutritionnelles : glucides 80, matières grasses 9, protéines 14"
        );
        // An unnamed step's lines after a Section are not filed under it.
        assert_eq!(
            rows(pasta, "ingredients"),
            [
                "## Sauce",
                "400 g tomates concassées",
                "2 gousse ail, écrasées",
                "huile d'olive, un filet",
                "## Étape 2",
                "200 g spaghetti",
            ]
        );
        assert_eq!(
            rows(pasta, "steps")[2],
            "Cuire les pâtes dans une grande casserole d’eau salée.\n\nTemps : 12 minutes"
        );
    }

    #[test]
    fn tandoor_ones_lines_read_as_tandoor_showed_them() {
        let recipes = read_all(fixture("v1-mealie-sample.zip"));
        let chili = titled(&recipes, "“Texas Red” Chili");
        assert_eq!(chili.foreign_id, "1 “Texas Red” Chili");
        let lines = rows(chili, "ingredients");
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[0], "2 oz New Mexico chile, dry, 57 g");
        assert_eq!(lines[1], "1½ cup (12 fl oz/355 ml) doppelbock");
        assert_eq!(lines[4], "2 yellow onions, small dice");
        // An amount of zero is no amount.
        assert_eq!(lines[7], "Two 15 oz (425 g) cans diced tomato");
        assert_eq!(lines[11], "Scallions, thinly sliced on the diagonal");
        assert_eq!(chili.candidate["prep_time_minutes"], Value::Null);
        assert_eq!(chili.candidate["cook_time_minutes"], 2);
        assert_eq!(
            chili.candidate["yield"],
            json!({ "amount": "1", "noun": "servings" })
        );
        assert_eq!(chili.candidate["note"], Value::Null);
        assert_eq!(chili.candidate["nutrition"], Value::Null);
        assert_eq!(
            chili.tags,
            [
                "American",
                "Course: Main",
                "Southwest",
                "Technique: cocotte"
            ]
        );
        let steps = rows(chili, "steps");
        assert_eq!(steps.len(), 1);
        assert!(steps[0].starts_with("Peheat the oven to 350°F"));
        assert!(steps[0].contains("### Source"));
    }

    #[test]
    fn a_picture_named_without_an_extension_is_the_photo() {
        let recipes = read_all(fixture("v2-export.zip"));
        let rohlik = titled(&recipes, "Rohlik s maslem");
        assert_eq!(rohlik.foreign_id, "102487 Rohlik s maslem");
        assert!(has_photo(rohlik));
        assert_eq!(rohlik.candidate["prep_time_minutes"], 2);
        assert_eq!(rows(rohlik, "steps").len(), 3);
        assert!(
            rohlik.candidate["ingredients"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_tandoor_recipe_in_another_shape_is_refused_with_the_update_reason() {
        let changed = |recipe: &mut Value| recipe["working_time"] = json!({ "minutes": 5 });
        // Read first, it says the whole file is in a shape no test file has.
        let refused = Tandoor
            .read(&Handed::bytes(fixture_with(
                "v2-export.zip",
                "102486",
                changed,
            )))
            .err()
            .unwrap();
        assert_eq!(refused, Refusal::UntestedShape { app: APP });
        // Further on, it is that recipe's alone.
        let recipes = read_all(fixture_with("v2-export.zip", "102490", changed));
        let unread: Vec<_> = recipes.iter().filter_map(|r| r.as_ref().err()).collect();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].foreign_id.as_deref(), Some("102490"));
        assert_eq!(unread[0].name.as_deref(), Some("Thiep"));
        assert!(unread[0].reason.contains("update Tandoor"));
    }

    #[test]
    fn an_export_unpacked_and_zipped_again_on_a_mac_reads_the_same() {
        // Safari unpacks a downloaded zip into a folder; the cook zips the
        // folder again, and the Mac adds its own copies beside each file.
        let mut outer =
            zip::ZipArchive::new(std::io::Cursor::new(fixture("v2.6-export.zip"))).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for index in 0..outer.len() {
                let mut entry = outer.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                for path in [
                    format!("export_2026-10-23/{name}"),
                    format!("__MACOSX/export_2026-10-23/._{name}"),
                ] {
                    zip.start_file(path, zip::write::SimpleFileOptions::default())
                        .unwrap();
                    zip.write_all(&bytes).unwrap();
                }
            }
            zip.finish().unwrap();
        }
        let recipes = read_all(out.into_inner());
        assert_eq!(recipes.len(), 2);
        assert_eq!(titled(&recipes, "Orange cake").foreign_id, "1 Orange cake");
    }

    #[test]
    fn a_short_french_recipe_is_worded_in_french() {
        // Little of the cook's own French, and what Kamosu adds is not read
        // as the recipe's Language.
        let recipes = read_all(fixture_with("v2.6-export.zip", "2", |recipe| {
            recipe["description"] = json!("");
            recipe["name"] = json!("Pâtes");
            for step in recipe["steps"].as_array_mut().unwrap() {
                step["name"] = json!("");
                step["instruction"] = json!("Cuire.");
                step["time"] = json!(5);
            }
        }));
        let pasta = titled(&recipes, "Pâtes");
        assert_eq!(
            pasta.candidate["note"],
            "Valeurs nutritionnelles : glucides 80, matières grasses 9, protéines 14"
        );
        assert_eq!(rows(pasta, "steps")[0], "Cuire.\n\nTemps : 5 minutes");
    }

    #[test]
    fn a_line_is_rebuilt_as_a_cook_writes_it() {
        let line = |line: Value| rebuild_line(&line);
        let food = |name: &str, plural: &str| json!({ "name": name, "plural_name": plural });
        assert_eq!(
            line(
                json!({ "amount": 0.25, "unit": { "name": "cup" }, "food": food("sugar", ""), "note": "" })
            ),
            "¼ cup sugar"
        );
        assert_eq!(
            line(
                json!({ "amount": 2.5, "unit": { "name": "g" }, "food": food("yeast", ""), "note": "" })
            ),
            "2.5 g yeast"
        );
        assert_eq!(
            line(json!({ "amount": 1, "food": food("egg", "eggs"), "note": "" })),
            "1 egg"
        );
        assert_eq!(
            line(
                json!({ "amount": 0, "food": food("egg", "eggs"), "always_use_plural_food": true })
            ),
            "eggs"
        );
        assert_eq!(
            line(
                json!({ "amount": 3, "no_amount": true, "food": food("salt", ""), "note": "to taste" })
            ),
            "salt, to taste"
        );
        assert_eq!(
            line(json!({ "amount": 0, "unit": { "name": "Bund" }, "food": food("Basilikum", "") })),
            "Bund Basilikum"
        );
        assert_eq!(
            line(json!({ "amount": 0, "unit": null, "food": null, "note": "a little oil" })),
            "a little oil"
        );
        // A note with nothing but an amount before it is the rest of the line.
        assert_eq!(
            line(json!({ "amount": 2, "unit": null, "food": null, "note": "eggs" })),
            "2 eggs"
        );
        // Tandoor keeps sixteen decimal places, and shows three.
        assert_eq!(
            line(json!({ "amount": 0.375, "unit": { "name": "kg" }, "food": food("flour", "") })),
            "0.375 kg flour"
        );
        assert_eq!(
            line(json!({ "amount": 1.125, "unit": { "name": "pinch" }, "food": food("salt", "") })),
            "1⅛ pinch salt"
        );
        assert_eq!(
            line(json!({ "amount": 0.0667, "unit": null, "food": food("lemon", "") })),
            "0.067 lemon"
        );
    }

    #[test]
    fn another_apps_zip_is_not_tandoors() {
        for name in [
            "mealie/v3-export.zip",
            "mealie/pre-v1-export.zip",
            "paprika/mealie-sample.zip",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name);
            assert!(
                matches!(Tandoor.read(&Handed::file(path)), Ok(None)),
                "{name}"
            );
        }
        assert!(matches!(
            Tandoor.read(&Handed::bytes(b"not a zip".to_vec())),
            Ok(None)
        ));
    }
}
