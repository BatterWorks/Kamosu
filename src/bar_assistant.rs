//! Reading a Bar Assistant export (#416, the spec is #411), made with
//! `bar:export-recipes` or the export on its Settings screen. It is a zip of
//! one folder per cocktail, `cocktails/<slug>_<id>/`, each holding the
//! cocktail's `recipe.json` and its pictures, and a `_meta.json` that says
//! Bar Assistant made it and in which format.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files described in `tests/fixtures/schema-org/README.md`.
//!
//! - **Two of its formats are read.** *JSON-LD* is schema.org's Recipe, and
//!   each cocktail is read as every recipe file is ([`crate::schema_org`]).
//!   *Schema* is Bar Assistant's own cocktail format, which keeps more: the
//!   source link, the garnish, the glass, an ingredient's note, its range and
//!   whether it is optional, and its substitutes. Its Datapack, Markdown, XML
//!   and YAML exports are not read.
//! - **A cocktail is known by its folder's name**, `negroni_2`, Bar
//!   Assistant's own id for it, in either format.
//! - **Each ingredient is written as a bartender writes it**, `15-30 ml Simple
//!   Syrup, optional (or 15 ml Honey Syrup)`. Every cocktail makes one drink.
//! - **The instructions are one block of text**, and each line is a Step, as
//!   written.
//! - **The description becomes the Note**, and the garnish, the glass, the
//!   method, the bar it comes from and the photo's credit each a line of it
//!   (#434). Tags are Tags.

use serde_json::{Value, json};

use crate::other_apps::{self, Budget, Handed, LeftOut, Reader, Refusal};
use crate::schema_org::{self, App};

/// Bar Assistant among the readers of another app's export.
pub struct BarAssistant;

const APP: &str = "Bar Assistant";

type Archive = zip::ZipArchive<Box<dyn other_apps::ReadSeek>>;

/// Which of Bar Assistant's formats an export is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    JsonLd,
    Schema,
}

impl Reader for BarAssistant {
    fn names(&self) -> &'static [&'static str] {
        &["Bar Assistant"]
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
        let mut budget = Budget::new(other_apps::MOST_UNPACKED);
        let Some(format) = format(&mut archive, &mut budget) else {
            return Ok(None);
        };
        let paths: Vec<(usize, String)> = (0..archive.len())
            .filter_map(|index| Some((index, archive.name_for_index(index)?.to_string())))
            .collect();
        let cocktails: Vec<(usize, String, Vec<usize>)> = paths
            .iter()
            .filter_map(|(index, path)| {
                let folder = path.strip_suffix("/recipe.json")?;
                let id = folder.strip_prefix("cocktails/")?;
                (!id.is_empty() && !id.contains('/')).then(|| {
                    (
                        *index,
                        id.to_string(),
                        schema_org::pictures_in(&paths, folder),
                    )
                })
            })
            .collect();
        if cocktails.is_empty() {
            return Ok(None);
        }

        let total = cocktails.len();
        let recipes = cocktails.into_iter().map(move |(index, id, pictures)| {
            let unread = |reason: String| other_apps::Unread {
                foreign_id: Some(id.clone()),
                name: None,
                reason,
            };
            let bytes = schema_org::unpack(&mut archive, index, &mut budget).map_err(unread)?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| {
                unread(
                    "the cocktail is not one Kamosu can read: it may be damaged, so export \
                     it from Bar Assistant again and import that"
                        .to_string(),
                )
            })?;
            let (files, unreadable) =
                schema_org::unpack_pictures(&mut archive, pictures, &mut budget);
            match format {
                Format::JsonLd => {
                    let mut node = crate::web_import::search_for_recipe(&value)
                        .ok_or_else(|| unread(Refusal::UntestedShape { app: APP }.sentence()))?;
                    without_its_own_marks(&mut node);
                    Ok(schema_org::recipe(
                        &node,
                        App::BarAssistant,
                        id.clone(),
                        files,
                        unreadable,
                    ))
                }
                Format::Schema => {
                    if !value["name"].is_string() || !value["ingredients"].is_array() {
                        return Err(unread(Refusal::UntestedShape { app: APP }.sentence()));
                    }
                    Ok(cocktail(&value, id.clone(), files, unreadable))
                }
            }
        });
        Ok(Some(other_apps::Export {
            app: App::BarAssistant.kind(),
            total,
            recipes: Box::new(recipes),
            left_out: Vec::new(),
        }))
    }
}

/// Take out what Bar Assistant's JSON-LD writes the same on every cocktail,
/// which no cook typed: itself as the author, and `Drink` and `Cocktail` as
/// the category and the cuisine.
fn without_its_own_marks(node: &mut Value) {
    let Value::Object(fields) = node else {
        return;
    };
    for (field, mark) in [("recipeCategory", "Drink"), ("recipeCuisine", "Cocktail")] {
        if fields.get(field).and_then(Value::as_str) == Some(mark) {
            fields.remove(field);
        }
    }
    if fields["author"]["name"] == "Recipe exported from Bar Assistant" {
        fields.remove("author");
    }
}

/// The format `_meta.json` names, where Bar Assistant wrote it and the format
/// is one this reads.
fn format(archive: &mut Archive, budget: &mut Budget) -> Option<Format> {
    let index = archive.index_for_name("_meta.json")?;
    let meta: Value =
        serde_json::from_slice(&schema_org::unpack(archive, index, budget).ok()?).ok()?;
    if !meta["called_from"]
        .as_str()
        .is_some_and(|from| from.starts_with("Kami\\Cocktail\\"))
    {
        return None;
    }
    match meta["type"].as_str()? {
        "json-ld" => Some(Format::JsonLd),
        "schema" => Some(Format::Schema),
        _ => None,
    }
}

/// The words a cocktail's Language puts around what Bar Assistant kept.
struct Words {
    drink: &'static str,
    optional: &'static str,
    or: &'static str,
    garnish: &'static str,
    glass: &'static str,
    method: &'static str,
    bar: &'static str,
    photo: &'static str,
}

fn words(language: Option<&str>) -> Words {
    match language {
        Some("fr") => Words {
            drink: "verre",
            optional: "facultatif",
            or: "ou",
            garnish: "Garniture :",
            glass: "Verre :",
            method: "Méthode :",
            bar: "Bar :",
            photo: "Photo :",
        },
        Some("es") => Words {
            drink: "bebida",
            optional: "opcional",
            or: "o",
            garnish: "Decoración:",
            glass: "Vaso:",
            method: "Método:",
            bar: "Bar:",
            photo: "Foto:",
        },
        _ => Words {
            drink: "drink",
            optional: "optional",
            or: "or",
            garnish: "Garnish:",
            glass: "Glass:",
            method: "Method:",
            bar: "Bar:",
            photo: "Photo:",
        },
    }
}

/// One cocktail in Bar Assistant's own format, as Kamosu's content.
fn cocktail(
    value: &Value,
    foreign_id: String,
    mut pictures: Vec<Vec<u8>>,
    unreadable: usize,
) -> other_apps::Recipe {
    let title = text(value, "name").unwrap_or_else(|| "Untitled recipe".to_string());
    let description = text(value, "description");
    let steps: Vec<Value> = text(value, "instructions")
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| json!({ "kind": "step", "text": line, "photo": null }))
        .collect();
    let ingredients = |words: &Words| -> Vec<Value> {
        value["ingredients"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|ingredient| line(ingredient, words))
            .map(|line| json!({ "kind": "ingredient", "text": line }))
            .collect()
    };
    // The Language is read from what the cook wrote, none of the words this
    // adds in it, and then words the lines' own additions.
    let language = crate::language::detect_content(&json!({
        "title": title,
        "note": description,
        "ingredients": ingredients(&words(None)),
        "steps": steps,
    }));
    let words = words(language);

    let mut left_out = Vec::new();
    let images: Vec<&Value> = value["images"].as_array().into_iter().flatten().collect();
    let photo = if pictures.is_empty() {
        images
            .first()
            .and_then(|image| text(image, "uri"))
            .filter(|uri| schema_org::is_web_address(uri))
            .map(other_apps::Photo::Address)
    } else {
        Some(other_apps::Photo::Bytes(pictures.remove(0)))
    };
    let extra = if pictures.is_empty() {
        images.len().saturating_sub(1)
    } else {
        pictures.len()
    };
    if extra > 0 {
        left_out.push(LeftOut::ExtraPhotos(extra));
    }
    if unreadable > 0 {
        left_out.push(LeftOut::UnreadablePhotos(unreadable));
    }

    let mut paragraphs: Vec<String> = description.into_iter().collect();
    for (field, lead) in [
        ("garnish", words.garnish),
        ("glass", words.glass),
        ("method", words.method),
        ("origin_bar", words.bar),
    ] {
        if let Some(written) = text(value, field) {
            paragraphs.push(format!("{lead} {written}"));
        }
    }
    if let Some(credit) = images.first().and_then(|image| text(image, "copyright")) {
        paragraphs.push(format!("{} {credit}", words.photo));
    }

    let mut tags: Vec<String> = Vec::new();
    for tag in value["tags"].as_array().into_iter().flatten() {
        if let Some(tag) = tag.as_str().map(str::trim).filter(|tag| !tag.is_empty())
            && !tags.iter().any(|kept| kept == tag)
        {
            tags.push(tag.to_string());
        }
    }

    other_apps::Recipe {
        foreign_id,
        cookings: Vec::new(),
        candidate: json!({
            "title": title,
            "yield": { "amount": "1", "noun": words.drink },
            "source": source(value),
            "note": if paragraphs.is_empty() { Value::Null } else { json!(paragraphs.join("\n\n")) },
            "ingredients": ingredients(&words),
            "steps": steps,
        }),
        tags,
        photo,
        step_photos: Vec::new(),
        components: Vec::new(),
        left_out,
    }
}

/// An ingredient as a bartender writes it: the amount or its range, the unit,
/// the ingredient, whether it is optional, its note, and what can stand in
/// for it.
fn line(ingredient: &Value, words: &Words) -> Option<String> {
    let mut written = measure(ingredient)?;
    if ingredient["optional"].as_bool() == Some(true) {
        written.push_str(&format!(", {}", words.optional));
    }
    if let Some(note) = text(ingredient, "note") {
        written.push_str(&format!(", {note}"));
    }
    let substitutes: Vec<String> = ingredient["substitutes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(measure)
        .map(|substitute| format!("{} {substitute}", words.or))
        .collect();
    if !substitutes.is_empty() {
        written.push_str(&format!(" ({})", substitutes.join(", ")));
    }
    Some(written)
}

/// The amount, its range, the unit and the name: `15-30 ml Simple Syrup`.
fn measure(ingredient: &Value) -> Option<String> {
    let name = text(ingredient, "name")?;
    let units = text(ingredient, "units");
    let positive = |field: &str| {
        ingredient[field]
            .as_f64()
            .filter(|amount| amount.is_finite() && *amount > 0.0)
    };
    let mut parts = Vec::new();
    if let Some(amount) = positive("amount") {
        let mut written = crate::units::amount_for(amount, units.as_deref());
        if let Some(most) = positive("amount_max").filter(|most| *most > amount) {
            written.push('-');
            written.push_str(&crate::units::amount_for(most, units.as_deref()));
        }
        parts.push(written);
        parts.extend(units);
    }
    parts.push(name);
    Some(parts.join(" "))
}

/// The Source: who made the cocktail and where it comes from, a link where
/// that is one.
fn source(value: &Value) -> Value {
    let author = text(value, "author");
    let source = text(value, "source");
    let link = source
        .clone()
        .filter(|source| schema_org::is_web_address(source));
    let named = source.filter(|_| link.is_none());
    let text = match (author, named) {
        (Some(author), Some(named)) => Some(format!("{author}, {named}")),
        (author, named) => author
            .or(named)
            .or_else(|| link.as_deref().and_then(other_apps::host)),
    };
    match text {
        Some(text) => json!({ "text": text, "link": link }),
        None => Value::Null,
    }
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
    use crate::other_apps::{Photo, Recipe};
    use std::io::Write;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/schema-org")
                .join(name),
        )
        .unwrap()
    }

    fn cocktails(bytes: Vec<u8>) -> Vec<Recipe> {
        let export = BarAssistant
            .read(&Handed::bytes(bytes))
            .unwrap_or_else(|_| panic!("refused"))
            .unwrap_or_else(|| panic!("not recognised"));
        assert_eq!(export.app, "bar_assistant");
        export.recipes.map(|r| r.ok().unwrap()).collect()
    }

    fn named<'a>(cocktails: &'a [Recipe], title: &str) -> &'a Recipe {
        cocktails
            .iter()
            .find(|recipe| recipe.candidate["title"] == title)
            .unwrap()
    }

    fn texts(content: &Value, field: &str) -> Vec<String> {
        content[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["text"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn its_own_format_keeps_the_garnish_the_glass_and_every_amount() {
        let cocktails = cocktails(fixture("bar-assistant-6.9.1-schema.zip"));
        assert_eq!(cocktails.len(), 2);
        let coffee = named(&cocktails, "Vietnamese iced coffee");
        assert_eq!(coffee.foreign_id, "vietnamese-iced-coffee_1");
        assert_eq!(
            texts(&coffee.candidate, "ingredients"),
            ["60 ml Espresso", "15-30 ml Simple Syrup, optional"]
        );
        assert_eq!(
            texts(&coffee.candidate, "steps"),
            [
                "1. Brew the coffee strong, straight onto the condensed milk.",
                "2. Stir until the milk dissolves.",
                "3. Pour over a glass full of ice."
            ]
        );
        assert_eq!(
            coffee.candidate["yield"],
            json!({ "amount": "1", "noun": "drink" })
        );
        assert_eq!(
            coffee.candidate["source"],
            json!({ "text": "A cook", "link": "https://example.com/vietnamese-coffee" })
        );
        assert_eq!(
            coffee.candidate["note"],
            "Strong coffee over sweet condensed milk, the way the cafés in Hanoi make it.\n\n\
             Garnish: None\n\nGlass: Highball\n\nMethod: Build\n\nPhoto: Simon Law, CC BY-SA 2.0"
        );
        assert_eq!(coffee.tags, ["coffee", "non-alcoholic"]);
        assert!(matches!(&coffee.photo, Some(Photo::Bytes(bytes)) if bytes.len() == 66594));
        assert!(coffee.left_out.is_empty());

        let negroni = named(&cocktails, "Negroni");
        assert_eq!(
            negroni.candidate["source"],
            json!({ "text": "en.wikipedia.org", "link": "https://en.wikipedia.org/wiki/Negroni" })
        );
        assert!(negroni.photo.is_none());
    }

    #[test]
    fn its_json_ld_is_read_as_a_recipe_file_and_names_no_source_for_bar_assistant_itself() {
        let cocktails = cocktails(fixture("bar-assistant-6.9.1-json-ld.zip"));
        let coffee = named(&cocktails, "Vietnamese iced coffee");
        // Known by the same id in both formats, so either matches the other.
        assert_eq!(coffee.foreign_id, "vietnamese-iced-coffee_1");
        assert_eq!(
            texts(&coffee.candidate, "ingredients"),
            ["60 ml Espresso", "15 ml Simple Syrup"]
        );
        assert_eq!(coffee.candidate["source"], Value::Null);
        assert_eq!(
            coffee.candidate["note"],
            "Strong coffee over sweet condensed milk, the way the cafés in Hanoi make it.\n\n\
             Method: Build\n\nPhoto: Simon Law, CC BY-SA 2.0"
        );
        // Bar Assistant's own category and cuisine, the same on every
        // cocktail, are no Tags.
        assert_eq!(coffee.tags, ["coffee", "non-alcoholic"]);
        assert!(matches!(&coffee.photo, Some(Photo::Bytes(bytes)) if bytes.len() == 66594));
    }

    #[test]
    fn an_export_in_a_format_it_does_not_read_is_not_its() {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(fixture(
            "bar-assistant-6.9.1-schema.zip",
        )))
        .unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
                if name == "_meta.json" {
                    let mut meta: Value = serde_json::from_slice(&bytes).unwrap();
                    meta["type"] = json!("yaml");
                    bytes = meta.to_string().into_bytes();
                }
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(&bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        let handed = Handed::bytes(out.into_inner());
        assert!(BarAssistant.read(&handed).unwrap().is_none());
        assert!(
            BarAssistant
                .read(&Handed::bytes(
                    std::fs::read(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("tests/fixtures/tandoor/v2.6-export.zip")
                    )
                    .unwrap()
                ))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_substitute_is_written_after_what_it_stands_in_for() {
        let words = words(None);
        let ingredient = json!({
            "name": "Simple Syrup", "amount": 15, "units": "ml", "optional": false,
            "amount_max": null, "note": "rich",
            "substitutes": [{ "name": "Honey Syrup", "amount": 10, "units": "ml", "amount_max": null }]
        });
        assert_eq!(
            line(&ingredient, &words).as_deref(),
            Some("15 ml Simple Syrup, rich (or 10 ml Honey Syrup)")
        );
        let to_taste = json!({ "name": "Salt", "amount": 0, "units": "", "substitutes": [] });
        assert_eq!(line(&to_taste, &words).as_deref(), Some("Salt"));
    }
}
