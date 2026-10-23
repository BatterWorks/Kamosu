//! Reading a Mealie export (#413, the spec is #411): Data Management, Recipes,
//! Export, which any cook in a household may make. It is a zip of one folder
//! per recipe, `recipes/<slug>/<slug>.json` beside that recipe's `images/` and
//! `assets/`, and it holds the exporter's own household alone.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files in `tests/fixtures/mealie/`, which its README
//! describes.
//!
//! - **A line reads as Mealie shows it today**: its `display`, never its
//!   `original_text`, which is the wording first imported before any edit.
//!   Mealie writes a fraction there in superscript and subscript digits
//!   (`1 ¹/₂ tbsp`), which its screen draws as ½; the line keeps the ½.
//! - **A line that points at another recipe** names it the way Mealie's screen
//!   does. Mealie's `display` leaves the name out (`1 batch`), and its screen
//!   puts it where the food goes (`1 batch Pizza dough`). The line becomes a
//!   Component once the file has landed, and Mealie's copy of that recipe,
//!   which every such line carries whole, is handed over beside it.
//! - **A title on a line or a Step opens a Section**, as Mealie draws it. A
//!   Step's `summary` is its own short heading, kept as the Step's first
//!   sentence, since a Step in Kamosu is one paragraph.
//! - **A picture in a Step** is an `<img>` Mealie writes into the Step's text,
//!   pointing at a file among the recipe's assets. The first becomes that
//!   Step's photo. Every other picture among the assets is an extra photo,
//!   and every other file an attached one, both counted on the Report.
//! - **Tools become an "Equipment:" line in the Note**, after the description
//!   and Mealie's own notes. Categories and tags both become Tags.
//! - **A time is Mealie's free text**, read the way a Step's timer is
//!   ([`crate::units::step_duration`]): a range at its lower end, and nothing
//!   where the text names no duration Kamosu reads, such as `1 day`.
//! - **Nutrition keeps the calories alone**, per serving as Mealie counts it,
//!   and only where they are a number.
//! - **Rating and comments are counted on the Report**, never written into the
//!   recipe. The export holds no cooking history: Mealie's backup does (#418).
//!
//! Two shapes are read, each one a real file has: Mealie 1 and later (a 2025
//! export, and one made with Mealie 3.29.0), and the shape before Mealie 1
//! (Mealie's own sample of December 2021), where a line is a `note` with a
//! `disable_amount` flag and an id is a number. Anything else Mealie wrote is
//! refused with the update reason.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::other_apps::{self, Budget, Component, Handed, LeftOut, Reader, Refusal, StepPhoto};

/// Mealie among the readers of another app's export.
pub struct Mealie;

const APP: &str = "Mealie";

/// The largest single file read out of the zip. A recipe's JSON is a few
/// kilobytes and its pictures a few hundred, so this refuses only a file that
/// claims to be enormous.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

type Archive = zip::ZipArchive<Box<dyn other_apps::ReadSeek>>;

impl Reader for Mealie {
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
        let folders = recipe_folders(&archive);
        let mut budget = Budget::new(other_apps::MOST_UNPACKED);
        // The first recipe that reads says whose file this is, and in which
        // shape. One damaged recipe is that recipe's loss, not the file's.
        let Some(first) = folders.iter().find_map(|folder| {
            unpack(&mut archive, folder.json, &mut budget)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        }) else {
            return Ok(None);
        };
        if !is_mealie_recipe(&first) {
            return Ok(None);
        }
        let shape = Shape::of(&first).ok_or(Refusal::UntestedShape { app: APP })?;

        let total = folders.len();
        let recipes = folders
            .into_iter()
            .map(move |folder| read_folder(&mut archive, &folder, shape, &mut budget));
        Ok(Some(other_apps::Export {
            app: "mealie",
            total,
            recipes: Box::new(recipes),
            left_out: Vec::new(),
        }))
    }
}

/// The two shapes of Mealie's recipe a real file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Mealie 1 and later: an id is a UUID, and every line has its `display`.
    Current,
    /// Before Mealie 1: an id is a number, and a line is a `note` with a
    /// `disable_amount` flag.
    BeforeV1,
}

impl Shape {
    fn of(recipe: &Value) -> Option<Shape> {
        let lines = recipe.get("recipe_ingredient")?.as_array()?;
        let every = |field: &str| lines.iter().all(|line| line.get(field).is_some());
        match recipe.get("id")? {
            Value::String(_) if lines.iter().all(|line| line["display"].is_string()) => {
                Some(Shape::Current)
            }
            Value::Number(_) if every("note") && every("disable_amount") => Some(Shape::BeforeV1),
            _ => None,
        }
    }
}

/// Whether a JSON file is a recipe Mealie wrote, in whichever shape.
fn is_mealie_recipe(recipe: &Value) -> bool {
    recipe["slug"].is_string()
        && recipe["recipe_ingredient"].is_array()
        && recipe.get("recipe_instructions").is_some()
}

/// One recipe's folder: everything it holds starts with `prefix`, and `json`
/// is the archive index of its recipe.
struct Folder {
    prefix: String,
    slug: String,
    json: usize,
}

/// Every `recipes/<slug>/<slug>.json` in the zip, in name order. A zip made on
/// a Mac, as Mealie's own sample is, holds the export inside a folder of its
/// own and beside `__MACOSX/` copies that are not recipes.
fn recipe_folders(archive: &Archive) -> Vec<Folder> {
    let mut folders: Vec<Folder> = (0..archive.len())
        .filter_map(|index| {
            let name = archive.name_for_index(index)?;
            if name.starts_with("__MACOSX/") {
                return None;
            }
            let (folder, file) = name.rsplit_once('/')?;
            let stem = file.strip_suffix(".json")?;
            let (parent, slug) = folder.rsplit_once('/').unwrap_or(("", folder));
            let under_recipes = parent == "recipes" || parent.ends_with("/recipes");
            (slug == stem && under_recipes).then(|| Folder {
                prefix: format!("{folder}/"),
                slug: slug.to_string(),
                json: index,
            })
        })
        .collect();
    folders.sort_by(|a, b| a.prefix.cmp(&b.prefix));
    folders
}

const DAMAGED: &str = "the file is damaged and none of it could be read; export the \
                       recipes from Mealie again and import that";

/// One file of the zip, through the export's budget.
fn unpack(archive: &mut Archive, index: usize, budget: &mut Budget) -> Result<Vec<u8>, String> {
    let mut entry = archive.by_index(index).map_err(|_| DAMAGED.to_string())?;
    match budget.unpack(&mut entry, MAX_FILE_BYTES) {
        Err(other_apps::TooMuch) => Err(format!(
            "the export unpacks to more than {} GB in all, more than any Mealie export \
             holds, so this recipe and those after it were not read",
            budget.most() / (1000 * 1000 * 1000)
        )),
        Ok((_, false)) => Err(DAMAGED.to_string()),
        Ok((bytes, true)) if bytes.len() as u64 > MAX_FILE_BYTES => {
            Err("a file in it is too large to belong to one recipe".to_string())
        }
        Ok((bytes, true)) => Ok(bytes),
    }
}

/// One file of the recipe's folder by its name there, or nothing where the
/// folder has no such file.
fn unpack_named(
    archive: &mut Archive,
    folder: &Folder,
    name: &str,
    budget: &mut Budget,
) -> Option<Result<Vec<u8>, String>> {
    let index = archive.index_for_name(&format!("{}{name}", folder.prefix))?;
    Some(unpack(archive, index, budget))
}

/// Read one recipe's folder: its JSON, its Main Photo and its Steps' photos.
fn read_folder(
    archive: &mut Archive,
    folder: &Folder,
    shape: Shape,
    budget: &mut Budget,
) -> Result<other_apps::Recipe, other_apps::Unread> {
    let unread = |foreign_id: Option<String>, reason: String| other_apps::Unread {
        foreign_id,
        name: Some(folder.slug.clone()),
        reason,
    };
    let bytes = unpack(archive, folder.json, budget).map_err(|reason| unread(None, reason))?;
    let recipe: Value = serde_json::from_slice(&bytes).map_err(|_| {
        unread(
            None,
            "the recipe is not one Kamosu can read: it may be damaged, so export it \
             from Mealie again and import that"
                .to_string(),
        )
    })?;
    let foreign_id = id_of(&recipe);
    if Shape::of(&recipe) != Some(shape) {
        return Err(unread(
            foreign_id,
            Refusal::UntestedShape { app: APP }.sentence(),
        ));
    }
    let ReadRecipe {
        mut recipe,
        step_pictures,
    } = read_recipe(&recipe, shape).map_err(|reason| unread(foreign_id.clone(), reason))?;

    let mut unreadable = 0;
    match unpack_named(archive, folder, "images/original.webp", budget) {
        Some(Ok(bytes)) => recipe.photo = Some(other_apps::Photo::Bytes(bytes)),
        Some(Err(reason)) if reason != DAMAGED => return Err(unread(foreign_id, reason)),
        Some(Err(_)) => unreadable += 1,
        None => {}
    }
    for (step, file_name) in step_pictures {
        match unpack_named(archive, folder, &format!("assets/{file_name}"), budget) {
            Some(Ok(bytes)) => recipe.step_photos.push(StepPhoto { step, bytes }),
            Some(Err(reason)) if reason != DAMAGED => return Err(unread(foreign_id, reason)),
            _ => unreadable += 1,
        }
    }
    if unreadable > 0 {
        recipe.left_out.push(LeftOut::UnreadablePhotos(unreadable));
    }
    // The export carries each "I Made This" picture under `images/timeline/`,
    // without the entry it belongs to (Mealie's backup has those, #418): an
    // extra photo of the recipe, counted with the others.
    let made_this: HashSet<&str> = archive
        .file_names()
        .filter_map(|name| name.strip_prefix(&format!("{}images/timeline/", folder.prefix)))
        .filter_map(|rest| rest.split_once('/').map(|(event, _)| event))
        .collect();
    if !made_this.is_empty() {
        match recipe.left_out.iter_mut().find_map(|left| match left {
            LeftOut::ExtraPhotos(count) => Some(count),
            _ => None,
        }) {
            Some(count) => *count += made_this.len(),
            None => recipe.left_out.push(LeftOut::ExtraPhotos(made_this.len())),
        }
    }
    Ok(recipe)
}

/// Mealie's id for a recipe: a UUID, or before Mealie 1 a number.
fn id_of(recipe: &Value) -> Option<String> {
    match recipe.get("id")? {
        Value::String(id) => Some(id.trim().to_string()).filter(|id| !id.is_empty()),
        Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}

/// A recipe read from its JSON, before any file of its folder is opened.
struct ReadRecipe {
    /// With no photo of any kind yet.
    recipe: other_apps::Recipe,
    /// Each Step's picture, by the Step's place and the asset's file name.
    step_pictures: Vec<(usize, String)>,
}

/// One recipe as Kamosu's content. The error is a sentence for the Report.
fn read_recipe(recipe: &Value, shape: Shape) -> Result<ReadRecipe, String> {
    let foreign_id = id_of(recipe).ok_or_else(|| {
        "the recipe carries no Mealie id, so Kamosu could not tell whether it had brought \
         it in before"
            .to_string()
    })?;
    let title = text(recipe, "name").unwrap_or_else(|| "Untitled recipe".to_string());

    let mut ingredients = Vec::new();
    let mut components = Vec::new();
    for line in items(recipe, "recipe_ingredient") {
        if let Some(section) = text(line, "title") {
            ingredients.push(json!({ "kind": "section", "text": section }));
        }
        let written = match shape {
            Shape::Current => current_line(line),
            Shape::BeforeV1 => line_before_v1(line),
        };
        if written.is_empty() {
            continue;
        }
        if let Some(component) = component_of(line, ingredients.len()) {
            components.push(component);
        }
        ingredients.push(json!({ "kind": "ingredient", "text": written }));
    }

    let assets: Vec<(String, bool)> = items(recipe, "assets")
        .filter_map(|asset| {
            let file_name = text(asset, "file_name")?;
            let picture = is_picture(&file_name);
            Some((file_name, picture))
        })
        .collect();
    let mut steps = Vec::new();
    let mut step_pictures = Vec::new();
    for step in items(recipe, "recipe_instructions") {
        if let Some(section) = text(step, "title") {
            steps.push(json!({ "kind": "section", "text": section }));
        }
        let (words, pictures) = without_asset_pictures(step["text"].as_str().unwrap_or(""));
        let words = match text(step, "summary") {
            Some(summary) if words.is_empty() => summary,
            Some(summary) => format!("{} {words}", as_sentence(&summary)),
            None => words,
        };
        // A Step of a picture alone has no words to stand on, and its
        // picture is counted with the extra photos.
        if words.is_empty() {
            continue;
        }
        if let Some(first) = pictures.into_iter().next() {
            step_pictures.push((steps.len(), first));
        }
        steps.push(json!({ "kind": "step", "text": words, "photo": null }));
    }
    let placed: HashSet<&str> = step_pictures
        .iter()
        .map(|(_, name)| name.as_str())
        .collect();

    let mut candidate = json!({
        "title": title,
        "prep_time_minutes": minutes(recipe, "prep_time"),
        "cook_time_minutes": minutes(recipe, "perform_time").or_else(|| minutes(recipe, "cook_time")),
        "source": text(recipe, "org_url").map_or(Value::Null, |link| other_apps::source_of_link(&link)),
        "nutrition": nutrition(recipe),
        "ingredients": ingredients,
        "steps": steps,
    });
    // The recipe's own Language words its Yield and its Equipment line.
    let language = crate::language::detect_content(&candidate);
    candidate["yield"] = recipe_yield(recipe, language);
    candidate["note"] = note(recipe, language);

    let mut tags: Vec<String> = Vec::new();
    for tag in items(recipe, "recipe_category").chain(items(recipe, "tags")) {
        if let Some(name) = name_of(tag)
            && !tags.contains(&name)
        {
            tags.push(name);
        }
    }

    let mut left_out = Vec::new();
    let extra_photos = assets
        .iter()
        .filter(|(name, picture)| *picture && !placed.contains(name.as_str()))
        .count();
    if extra_photos > 0 {
        left_out.push(LeftOut::ExtraPhotos(extra_photos));
    }
    let attached = assets
        .iter()
        .filter(|(name, picture)| !picture && !placed.contains(name.as_str()))
        .count();
    if attached > 0 {
        left_out.push(LeftOut::AttachedFiles(attached));
    }
    if recipe["rating"].as_f64().is_some_and(|rating| rating > 0.0) {
        left_out.push(LeftOut::Rating);
    }
    let comments = items(recipe, "comments").count();
    if comments > 0 {
        left_out.push(LeftOut::Comments(comments));
    }

    Ok(ReadRecipe {
        recipe: other_apps::Recipe {
            foreign_id,
            candidate,
            tags,
            photo: None,
            step_photos: Vec::new(),
            components,
            left_out,
        },
        step_pictures,
    })
}

/// The line at `line` as a Component, where it points at another recipe.
/// Mealie copies that recipe whole into the line, and the copy is read as any
/// recipe is, so it can land where the file holds it nowhere else. Its photos
/// are not in this recipe's folder, and are counted as not in the export.
fn component_of(line: &Value, at: usize) -> Option<Component> {
    let referenced = line.get("referenced_recipe").filter(|r| r.is_object())?;
    let foreign_id = id_of(referenced)?;
    let copy = Shape::of(referenced)
        .and_then(|shape| read_recipe(referenced, shape).ok())
        .map(|read| {
            let mut copy = read.recipe;
            // Its pictures are in its own folder, which is not in the file.
            let had_pictures = referenced["image"]
                .as_str()
                .is_some_and(|image| !image.is_empty())
                || !read.step_pictures.is_empty();
            if had_pictures {
                copy.photo = Some(other_apps::Photo::NotInExport);
            }
            Box::new(copy)
        });
    Some(Component {
        line: at,
        foreign_id,
        times: line["quantity"].as_f64().filter(|times| *times > 0.0),
        copy,
    })
}

/// A line as Mealie's screen shows it: its `display`, with the name of the
/// recipe it points at put where the food goes, and its fractions plain.
fn current_line(line: &Value) -> String {
    let display = line["display"].as_str().unwrap_or("");
    let written = match line["referenced_recipe"]["name"].as_str().map(str::trim) {
        Some(name) if !name.is_empty() => with_recipe_named(display, line, name),
        _ => display.to_string(),
    };
    one_line(&plain_fractions(&written))
}

/// Mealie's `display` is the quantity, the unit, the food and the note, in
/// that order; its screen draws a pointed-at recipe's name in the food's place.
fn with_recipe_named(display: &str, line: &Value, name: &str) -> String {
    let mut head = display.trim();
    let note = line["note"].as_str().map(str::trim).unwrap_or("");
    if !note.is_empty() {
        head = head.strip_suffix(note).unwrap_or(head).trim_end();
    }
    for food in [&line["food"]["plural_name"], &line["food"]["name"]] {
        if let Some(food) = food.as_str().map(str::trim).filter(|f| !f.is_empty()) {
            head = head.strip_suffix(food).unwrap_or(head).trim_end();
        }
    }
    [head, name, note]
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A line before Mealie 1: the note alone where the amount was turned off,
/// as every line of the real sample is, and otherwise its parts in Mealie's
/// order.
fn line_before_v1(line: &Value) -> String {
    let note = line["note"].as_str().unwrap_or("");
    if line["disable_amount"] == json!(true) {
        return one_line(note);
    }
    let quantity = line["quantity"]
        .as_f64()
        .filter(|quantity| *quantity > 0.0)
        .map(crate::units::as_fraction);
    let parts = [
        quantity,
        name_of(&line["unit"]),
        name_of(&line["food"]),
        Some(note.to_string()),
    ];
    one_line(&parts.into_iter().flatten().collect::<Vec<_>>().join(" "))
}

/// Mealie's fractions as a cook writes them: `1 ¹/₂` is `1½`, as Mealie's own
/// screen draws it, and the line Kamosu's reader reads.
fn plain_fractions(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < chars.len() {
        let Some((part, end)) = superscript_fraction(&chars, at) else {
            out.push(chars[at]);
            at += 1;
            continue;
        };
        // A whole number written just before it, as in `1 ¹/₂`.
        let mut whole = 0.0;
        let kept = out.trim_end_matches(' ');
        if kept.len() < out.len() {
            let digits = kept.len() - kept.chars().rev().take_while(char::is_ascii_digit).count();
            let starts_a_word = digits == 0 || kept[..digits].ends_with(' ');
            if digits < kept.len() && starts_a_word {
                whole = kept[digits..].parse().unwrap_or(0.0);
                out.truncate(digits);
            }
        }
        out.push_str(&crate::units::as_fraction(whole + part));
        at = end;
    }
    out
}

/// The value of a fraction such as `¹/₂` written at `at`, and where it ends.
fn superscript_fraction(chars: &[char], at: usize) -> Option<(f64, usize)> {
    let digits = |from: usize, digit: fn(char) -> Option<u32>| {
        let mut value = 0u32;
        let mut end = from;
        while let Some(d) = chars.get(end).copied().and_then(digit) {
            value = value.checked_mul(10)?.checked_add(d)?;
            end += 1;
        }
        (end > from).then_some((value, end))
    };
    let (top, slash) = digits(at, superscript)?;
    if chars.get(slash) != Some(&'/') {
        return None;
    }
    let (bottom, end) = digits(slash + 1, subscript)?;
    (bottom > 0).then(|| (f64::from(top) / f64::from(bottom), end))
}

fn superscript(c: char) -> Option<u32> {
    match c {
        '⁰' => Some(0),
        '¹' => Some(1),
        '²' => Some(2),
        '³' => Some(3),
        '⁴'..='⁹' => Some(c as u32 - '⁴' as u32 + 4),
        _ => None,
    }
}

fn subscript(c: char) -> Option<u32> {
    ('₀'..='₉').contains(&c).then(|| c as u32 - '₀' as u32)
}

/// A Step's or a description's text with each picture Mealie placed in it
/// taken out, and the asset files those pictures show, in order. Mealie
/// writes one as `<img src=".../api/media/recipes/<id>/assets/<file>" …/>`.
fn without_asset_pictures(text: &str) -> (String, Vec<String>) {
    let mut words = String::new();
    let mut pictures = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("<img") {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        let tag = &rest[open..open + close + 1];
        let Some(file_name) = asset_of(tag) else {
            words.push_str(&rest[..open + close + 1]);
            rest = &rest[open + close + 1..];
            continue;
        };
        words.push_str(&rest[..open]);
        pictures.push(file_name);
        rest = &rest[open + close + 1..];
    }
    words.push_str(rest);
    (words.trim().to_string(), pictures)
}

/// The asset file an `<img>` tag shows, where it shows one of the recipe's.
fn asset_of(tag: &str) -> Option<String> {
    let source = tag.split_once("src=\"")?.1.split('"').next()?;
    if !source.contains("/api/media/recipes/") {
        return None;
    }
    let file_name = source.rsplit_once("/assets/")?.1;
    let file_name = file_name.split(['?', '#']).next()?;
    (!file_name.is_empty()).then(|| file_name.to_string())
}

/// Whether an asset is a picture, by its file's extension.
fn is_picture(file_name: &str) -> bool {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    matches!(
        extension.as_deref(),
        Some("jpg" | "jpeg" | "png" | "webp" | "gif" | "heic" | "heif" | "avif")
    )
}

/// A heading as the first sentence of a paragraph.
fn as_sentence(heading: &str) -> String {
    if heading.ends_with(['.', '!', '?', ':', '…']) {
        heading.to_string()
    } else {
        format!("{heading}.")
    }
}

/// One of Mealie's free-text times in whole minutes, or the seconds Mealie 3
/// keeps beside it where it has them.
fn minutes(recipe: &Value, field: &str) -> Option<i64> {
    let seconds = recipe[format!("{field}_seconds")]
        .as_f64()
        .filter(|seconds| *seconds > 0.0)
        .or_else(|| {
            let written = text(recipe, field)?;
            crate::units::step_duration(&written).map(|seconds| seconds as f64)
        })?;
    let minutes = (seconds / 60.0).round() as i64;
    (minutes > 0).then_some(minutes)
}

/// The Yield: what the recipe makes where Mealie says (`2 balls`), and its
/// servings otherwise. Before Mealie 1 it was one piece of text, `4
/// serving(s)`.
fn recipe_yield(recipe: &Value, language: Option<&str>) -> Value {
    let servings = || match language {
        Some("fr") => "portions",
        Some("es") => "raciones",
        _ => "servings",
    };
    let positive = |field: &str| recipe[field].as_f64().filter(|amount| *amount > 0.0);
    let noun = text(recipe, "recipe_yield");
    if let Some(quantity) = positive("recipe_yield_quantity") {
        let noun = noun.unwrap_or_else(|| servings().to_string());
        return json!({ "amount": crate::units::as_decimal(quantity), "noun": noun });
    }
    if let Some(amount) = positive("recipe_servings") {
        return json!({ "amount": crate::units::as_decimal(amount), "noun": servings() });
    }
    // `4 serving(s)`: the number, and the rest as its noun.
    let Some(written) = noun else {
        return Value::Null;
    };
    let digits = written
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(written.len());
    let Ok(amount) = written[..digits].parse::<f64>() else {
        return Value::Null;
    };
    let rest = written[digits..].replace("(s)", "s");
    let rest = rest.trim();
    let noun = match rest.to_lowercase().as_str() {
        "" | "serving" | "servings" => servings().to_string(),
        _ => rest.to_string(),
    };
    json!({ "amount": crate::units::as_decimal(amount), "noun": noun })
}

/// The Note: the description, Mealie's own notes, and the Equipment line from
/// its tools, each a paragraph.
fn note(recipe: &Value, language: Option<&str>) -> Value {
    let mut paragraphs = Vec::new();
    if let Some(description) = recipe["description"].as_str() {
        let (words, _) = without_asset_pictures(description);
        if !words.is_empty() {
            paragraphs.push(words);
        }
    }
    for note in items(recipe, "notes") {
        let lines: Vec<String> = [text(note, "title"), text(note, "text")]
            .into_iter()
            .flatten()
            .collect();
        if !lines.is_empty() {
            paragraphs.push(lines.join("\n"));
        }
    }
    let tools: Vec<String> = items(recipe, "tools").filter_map(name_of).collect();
    if !tools.is_empty() {
        let lead = match language {
            Some("fr") => "Matériel :",
            Some("es") => "Utensilios:",
            _ => "Equipment:",
        };
        paragraphs.push(format!("{lead} {}.", tools.join(", ")));
    }
    if paragraphs.is_empty() {
        Value::Null
    } else {
        json!(paragraphs.join("\n\n"))
    }
}

/// The calories per serving, where Mealie holds them as a number.
fn nutrition(recipe: &Value) -> Value {
    let calories = text(&recipe["nutrition"], "calories").and_then(|calories| {
        let end = calories
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
            .unwrap_or(calories.len());
        let number = calories[..end].trim_end_matches(',');
        // `1,200` is a thousand and two hundred; any other comma is not a
        // number Kamosu can be sure of.
        let thousands = number
            .split(',')
            .skip(1)
            .all(|group| group.len() == 3 && group.bytes().all(|b| b.is_ascii_digit()));
        if number.contains(',') && !thousands {
            return None;
        }
        number.replace(',', "").parse::<f64>().ok()
    });
    match calories {
        Some(calories) if calories >= 0.0 => {
            json!({ "calories": calories, "basis": "per_serving" })
        }
        _ => Value::Null,
    }
}

/// The items of a list field, or none.
fn items<'a>(value: &'a Value, field: &str) -> impl Iterator<Item = &'a Value> {
    value[field].as_array().into_iter().flatten()
}

/// A category's, tag's, tool's, unit's or food's name: an object's `name`, or
/// before Mealie 1 sometimes the bare word.
fn name_of(value: &Value) -> Option<String> {
    match value {
        Value::String(name) => Some(name.trim().to_string()).filter(|name| !name.is_empty()),
        _ => text(value, "name"),
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

/// Words on one line with single spaces, as a cook types an Ingredient Line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::other_apps::{Photo, Recipe, Unread};

    fn fixture(name: &str) -> Handed {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mealie")
            .join(name);
        Handed::file(path)
    }

    fn read_all(name: &str) -> Vec<Result<Recipe, Unread>> {
        let export = Mealie
            .read(&fixture(name))
            .unwrap_or_else(|_| panic!("{name} was refused"))
            .unwrap_or_else(|| panic!("{name} was not recognised"));
        assert_eq!(export.app, "mealie");
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

    fn lines(recipe: &Recipe, field: &str) -> Vec<String> {
        recipe.candidate[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| format!("{}: {}", row["kind"], row["text"].as_str().unwrap()))
            .collect()
    }

    fn left_out(recipe: &Recipe) -> Vec<String> {
        recipe
            .left_out
            .iter()
            .map(|left| match left {
                LeftOut::ExtraPhotos(n) => format!("extra photos {n}"),
                LeftOut::AttachedFiles(n) => format!("attached files {n}"),
                LeftOut::Rating => "rating".to_string(),
                LeftOut::Comments(n) => format!("comments {n}"),
                LeftOut::UnreadablePhotos(n) => format!("unreadable photos {n}"),
                _ => "something else".to_string(),
            })
            .collect()
    }

    #[test]
    fn the_export_made_with_mealie_3_reads_as_mealie_showed_it() {
        let recipes = read_all("v3-export.zip");
        assert_eq!(recipes.len(), 3);
        let pizza = titled(&recipes, "Margherita pizza");
        assert_eq!(pizza.foreign_id, "c1acc761-fb3d-4735-b53b-b61059d5716a");
        assert_eq!(
            lines(pizza, "ingredients"),
            [
                "\"section\": Base",
                "\"ingredient\": 1 batch Pizza dough",
                "\"ingredient\": 1 tbsp semolina for dusting",
                "\"section\": Toppings",
                "\"ingredient\": 200 g tomato passata",
                "\"ingredient\": 125 g mozzarella torn",
                "\"ingredient\": A few basil leaves",
                "\"ingredient\": 1½ tbsp olive oil",
            ]
        );
        assert_eq!(
            lines(pizza, "steps"),
            [
                "\"section\": Shape",
                "\"step\": Heat the oven to 250 °C with the stone inside for an hour.",
                "\"step\": Stretch one ball of dough on a floured peel.",
                "\"section\": Bake",
                "\"step\": Into the oven. Spread the passata, scatter the mozzarella and slide onto the stone.",
                "\"step\": Bake for 8 minutes, then add the basil and the oil.",
            ]
        );
        let c = &pizza.candidate;
        assert_eq!(c["yield"], json!({ "amount": "2", "noun": "servings" }));
        assert_eq!(c["prep_time_minutes"], json!(15));
        assert_eq!(c["cook_time_minutes"], json!(8));
        assert_eq!(
            c["source"],
            json!({ "text": "example.com", "link": "https://www.example.com/margherita" })
        );
        assert_eq!(
            c["nutrition"],
            json!({ "calories": 850.0, "basis": "per_serving" })
        );
        assert_eq!(
            c["note"],
            json!(
                "The pizza we make every Friday.\n\nOven\nA home oven at its hottest, with the \
                 stone heated an hour.\n\nEquipment: Pizza peel, Pizza stone."
            )
        );
        assert_eq!(pizza.tags, ["Dinner", "Italian", "Weeknight"]);
        assert!(matches!(&pizza.photo, Some(Photo::Bytes(bytes)) if bytes.starts_with(b"RIFF")));
        // The stretched dough is the second Step's photo, the Section counted.
        assert_eq!(pizza.step_photos.len(), 1);
        assert_eq!(pizza.step_photos[0].step, 2);
        assert!(pizza.step_photos[0].bytes.starts_with(&[0xff, 0xd8]));
        assert_eq!(
            left_out(pizza),
            ["extra photos 2", "attached files 1", "rating", "comments 2"]
        );

        // The dough line points at the dough, which the pizza carries a copy of.
        let [component] = pizza.components.as_slice() else {
            panic!("one Component");
        };
        assert_eq!(component.line, 1);
        assert_eq!(component.times, Some(1.0));
        let dough = titled(&recipes, "Pizza dough");
        assert_eq!(component.foreign_id, dough.foreign_id);
        let copy = component.copy.as_ref().expect("the copy Mealie keeps");
        assert_eq!(copy.foreign_id, dough.foreign_id);
        assert_eq!(
            copy.candidate["ingredients"],
            dough.candidate["ingredients"]
        );
        assert_eq!(
            dough.candidate["yield"],
            json!({ "amount": "2", "noun": "balls" })
        );
        // `1 day` is no duration Kamosu reads, and the dough has no cook time.
        assert_eq!(dough.candidate["prep_time_minutes"], json!(20));
        assert_eq!(dough.candidate["cook_time_minutes"], Value::Null);
        assert!(dough.photo.is_none());

        let tarte = titled(&recipes, "Tarte aux pommes");
        assert_eq!(
            tarte.candidate["yield"],
            json!({ "amount": "6", "noun": "portions" })
        );
        assert_eq!(tarte.candidate["cook_time_minutes"], json!(45));
        assert_eq!(
            tarte.candidate["ingredients"][0]["text"],
            json!("1 pâte brisée")
        );
    }

    #[test]
    fn the_export_of_2025_is_read() {
        let recipes = read_all("v2-export.zip");
        assert_eq!(recipes.len(), 10);
        assert!(recipes.iter().all(Result::is_ok));
        let boulettes = titled(&recipes, "Boulettes de Courgettes au Four");
        // A picture in the description is taken out of it, and counted.
        assert_eq!(boulettes.candidate["note"], Value::Null);
        assert_eq!(left_out(boulettes), ["extra photos 1", "rating"]);
        assert_eq!(boulettes.candidate["cook_time_minutes"], json!(20));
        assert_eq!(
            boulettes.candidate["steps"][0]["text"],
            json!(
                "Préparer les courgettes. Râpez les courgettes et salez-les légèrement.\n\
                 Laissez-les dégorger 10 minutes dans une passoire.\nPressez-les ensuite \
                 entre vos mains ou dans un torchon pour retirer un maximum d’eau."
            )
        );
        let rice = titled(&recipes, "Thai Pineapple Fried Rice");
        assert_eq!(
            rice.candidate["ingredients"][2]["text"],
            json!("1½ Cup fresh pineapple chopped")
        );
        assert_eq!(
            rice.candidate["ingredients"][4]["text"],
            json!("¾ Cup green onions chopped")
        );
        let bowl = titled(&recipes, "Griechenland Bowl");
        assert_eq!(bowl.tags, ["Hauptgang", "Salat", "Fleisch"]);
    }

    #[test]
    fn the_export_from_before_mealie_1_is_read() {
        let recipes = read_all("pre-v1-export.zip");
        assert_eq!(recipes.len(), 3);
        let stew = titled(&recipes, "Old-Fashioned Beef Stew");
        assert_eq!(stew.foreign_id, "3");
        assert_eq!(
            stew.candidate["yield"],
            json!({ "amount": "4", "noun": "servings" })
        );
        assert_eq!(
            stew.candidate["ingredients"][0]["text"],
            json!("1/4 cup all-purpose flour")
        );
        let pizza = titled(&recipes, "PIZZA DOUGH — Basics With Babish");
        assert_eq!(
            pizza.candidate["steps"][0],
            json!({ "kind": "section", "text": "Preparation", })
        );
        assert_eq!(
            pizza.candidate["ingredients"][4]["text"],
            json!("4 g kosher salt")
        );
        assert_eq!(pizza.candidate["prep_time_minutes"], Value::Null);
        assert_eq!(pizza.candidate["cook_time_minutes"], json!(10));
        assert_eq!(pizza.tags, ["babish"]);
        let cookies = titled(&recipes, "Levain Chocolate Chip Cookies");
        assert_eq!(cookies.tags, ["Dessert", "Cookies"]);
        assert!(
            recipes
                .iter()
                .flatten()
                .all(|recipe| recipe.photo.is_some())
        );
    }

    #[test]
    fn a_backup_is_not_an_export() {
        assert!(Mealie.read(&fixture("v3-backup.zip")).unwrap().is_none());
    }

    #[test]
    fn a_shape_no_real_file_has_is_refused_with_the_update_reason() {
        // Mealie 1's early exports kept an id but no `display` on a line.
        let recipe = json!({
            "id": "6a4f1d7e-0000-0000-0000-000000000000", "slug": "soup", "name": "Soup",
            "recipe_ingredient": [{ "note": "1 onion", "quantity": 1 }],
            "recipe_instructions": [],
        });
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            use std::io::Write;
            let mut writer = zip::ZipWriter::new(&mut zip);
            writer
                .start_file(
                    "recipes/soup/soup.json",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(recipe.to_string().as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        let refused = Mealie.read(&Handed::bytes(zip.into_inner())).err();
        assert_eq!(refused, Some(Refusal::UntestedShape { app: "Mealie" }));
    }

    #[test]
    fn a_damaged_recipe_is_that_recipes_loss_and_not_the_files() {
        let pizza = std::io::Read::bytes(
            zip::ZipArchive::new(
                std::fs::File::open(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures/mealie/v3-export.zip"),
                )
                .unwrap(),
            )
            .unwrap()
            .by_name("recipes/margherita-pizza/margherita-pizza.json")
            .unwrap(),
        )
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            use std::io::Write;
            let mut writer = zip::ZipWriter::new(&mut zip);
            for (name, bytes) in [
                ("recipes/broken/broken.json", b"{ \"id\": ".as_slice()),
                (
                    "recipes/margherita-pizza/margherita-pizza.json",
                    pizza.as_slice(),
                ),
            ] {
                writer
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(bytes).unwrap();
            }
            writer.finish().unwrap();
        }
        let export = Mealie
            .read(&Handed::bytes(zip.into_inner()))
            .unwrap_or_else(|_| panic!("refused"))
            .expect("recognised by the recipe that reads");
        let recipes: Vec<_> = export.recipes.collect();
        assert!(matches!(&recipes[0], Err(unread) if unread.name.as_deref() == Some("broken")));
        assert!(recipes[1].is_ok());
    }

    #[test]
    fn calories_are_a_number_or_nothing() {
        let calories = |written: &str| nutrition(&json!({ "nutrition": { "calories": written } }));
        assert_eq!(calories("1,200 kcal")["calories"], json!(1200.0));
        assert_eq!(calories("850")["calories"], json!(850.0));
        assert_eq!(calories("1,5"), Value::Null);
        assert_eq!(calories("about 300"), Value::Null);
    }

    #[test]
    fn fractions_read_as_mealie_draws_them() {
        assert_eq!(plain_fractions("1 ¹/₂ Cup flour"), "1½ Cup flour");
        assert_eq!(
            plain_fractions("³/₄ Cup green onions"),
            "¾ Cup green onions"
        );
        assert_eq!(plain_fractions("2 eggs"), "2 eggs");
        assert_eq!(plain_fractions("x1 ¹/₃ cup"), "x1 ⅓ cup");
        assert_eq!(plain_fractions("¹/₀ cup"), "¹/₀ cup");
    }

    #[test]
    fn a_pointed_at_recipe_is_named_where_the_food_goes() {
        let line = |display: &str, note: &str| {
            json!({
                "display": display, "note": note, "food": null,
                "referenced_recipe": { "name": "Pizza dough" },
            })
        };
        assert_eq!(current_line(&line("1 batch", "")), "1 batch Pizza dough");
        assert_eq!(
            current_line(&line("2 batches rested overnight", "rested overnight")),
            "2 batches Pizza dough rested overnight"
        );
        assert_eq!(current_line(&line("", "")), "Pizza dough");
    }

    #[test]
    fn pictures_mealie_placed_in_text_are_taken_out() {
        let (words, pictures) = without_asset_pictures(
            "Stretch it.<img src=\"/api/media/recipes/abc/assets/one-jpg.jpg\" height=\"100%\"/> \
             Then <img src=\"https://elsewhere.example/x.jpg\"/> bake.",
        );
        assert_eq!(
            words,
            "Stretch it. Then <img src=\"https://elsewhere.example/x.jpg\"/> bake."
        );
        assert_eq!(pictures, ["one-jpg.jpg"]);
    }
}
