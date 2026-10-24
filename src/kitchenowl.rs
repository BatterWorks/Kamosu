//! Reading a KitchenOwl export (#417, the spec is #411): one household's
//! export, made by any of its members from the household's settings, Danger
//! Zone, Export. It is one JSON file, `<household>_export.json`, holding the
//! household's recipes beside its settings, the names of its shopping lists,
//! its catalogue of items and its expenses.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files described in `tests/fixtures/kitchenowl/README.md`.
//!
//! - **The description is the recipe's whole text, in markdown**, and holds
//!   both what the cook wrote about the recipe and its method. Where it holds
//!   a list, each item of the list is a Step, a heading among them a Section,
//!   and the paragraphs before the list and after it are the Note. Where it
//!   holds none, each line is a Step, as Paprika's directions are.
//! - **A line is the item's own words joined to its name**: KitchenOwl keeps
//!   a free-text description beside each item, `225 g` beside `Butter`. One
//!   that opens with a number is the amount and goes first, the rest of it
//!   after a comma, `2 Oranges, zest and juice`; any other goes after the name,
//!   `Icing sugar, to dust`. KitchenOwl keeps the items of a recipe in no
//!   order of the cook's, and its export lists them by name.
//! - **Optional items are a Section of their own**, last, as KitchenOwl shows
//!   them under Optional ingredients.
//! - **KitchenOwl's `time` is the total time**, which no field of Kamosu's
//!   holds, so it goes into the Note as a line of its own (#434), unless it is
//!   only the prep and cook times added up. Times and yields are whole
//!   numbers, and none where they are zero.
//! - **The recipe is known by its household's name and its own**, `Home ›
//!   Orange cake`, since the export gives it no id, and two households may
//!   each have a recipe of one name (#417). A recipe, or a household, renamed
//!   in KitchenOwl between two imports arrives twice. Two of one name in one
//!   file are numbered.
//! - **The export holds no photo**, only the name of each recipe's picture on
//!   the KitchenOwl server, which only a member signed in there can fetch. The
//!   Report counts the recipes that had one, and the shopping lists, items and
//!   expenses the household kept.
//!
//! Every real file has one shape, KitchenOwl 0.7.10's: the fields below. A
//! file of KitchenOwl's in any other shape is refused with the update reason.

use serde_json::{Value, json};

use crate::other_apps::{self, FileLeftOut, Handed, Reader, Refusal};

/// KitchenOwl among the readers of another app's export.
pub struct KitchenOwl;

const APP: &str = "KitchenOwl";

/// The largest export read whole. A KitchenOwl export holds no picture, and
/// the real one of three recipes and 660 items is 44 KB, so this is tens of
/// thousands of recipes.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

impl Reader for KitchenOwl {
    fn names(&self) -> &'static [&'static str] {
        &[APP]
    }

    fn read(&self, handed: &Handed) -> Result<Option<other_apps::Export>, Refusal> {
        let Some(household) = handed.json(MAX_FILE_BYTES).filter(Value::is_object) else {
            return Ok(None);
        };
        if !is_kitchenowls(&household) {
            return Ok(None);
        }
        if !is_known_shape(&household) {
            return Err(Refusal::UntestedShape { app: APP });
        }

        let recipes = household["recipes"].as_array().cloned().unwrap_or_default();
        let ids = unique_ids(text(&household, "name").as_deref(), &recipes);
        let total = recipes.len();
        let count = |field: &str| household[field].as_array().map_or(0, Vec::len);
        let left_out: Vec<FileLeftOut> = [
            FileLeftOut::ShoppingLists(count("shoppinglists")),
            FileLeftOut::Items(count("items")),
            FileLeftOut::Expenses(count("expenses")),
        ]
        .into_iter()
        .filter(|left| left.count() > 0)
        .collect();
        let recipes = recipes
            .into_iter()
            .zip(ids)
            .map(|(recipe, foreign_id)| Ok(read_recipe(&recipe, foreign_id)));
        Ok(Some(other_apps::Export {
            app: "kitchenowl",
            total,
            recipes: Box::new(recipes),
            left_out,
        }))
    }
}

/// Whether the file is a KitchenOwl household's export, in whichever shape:
/// its recipes beside the household's members and shopping lists, which no
/// other app's export holds together.
fn is_kitchenowls(household: &Value) -> bool {
    household["recipes"].is_array()
        && household["member"].is_array()
        && household["shoppinglists"].is_array()
}

/// Whether the export has the shape every real file has.
fn is_known_shape(household: &Value) -> bool {
    let whole_or_null = |value: &Value| value.is_u64() || value.is_null();
    let text_or_null = |value: &Value| value.is_string() || value.is_null();
    let is_recipe = |recipe: &Value| {
        recipe["name"].is_string()
            && text_or_null(&recipe["description"])
            && text_or_null(&recipe["photo"])
            && text_or_null(&recipe["source"])
            && ["time", "cook_time", "prep_time", "yields"]
                .iter()
                .all(|field| whole_or_null(&recipe[field]))
            && recipe["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().all(Value::is_string))
            && recipe["items"].as_array().is_some_and(|items| {
                items.iter().all(|item| {
                    item["name"].is_string()
                        && text_or_null(&item["description"])
                        && item["optional"].is_boolean()
                })
            })
    };
    household["recipes"]
        .as_array()
        .is_some_and(|recipes| recipes.iter().all(is_recipe))
        && household["items"].is_array()
        && household["expenses"].is_array()
}

/// What each recipe is known by: its household's name and its own, two of one
/// name numbered in the order the file holds them.
fn unique_ids(household: Option<&str>, recipes: &[Value]) -> Vec<String> {
    other_apps::numbered(
        recipes
            .iter()
            .map(|recipe| {
                let name = text(recipe, "name").unwrap_or_else(|| "Untitled recipe".to_string());
                match household {
                    Some(household) => format!("{household} › {name}"),
                    None => name,
                }
            })
            .collect(),
    )
}

/// One recipe as Kamosu's content.
fn read_recipe(recipe: &Value, foreign_id: String) -> other_apps::Recipe {
    let title = text(recipe, "name").unwrap_or_else(|| "Untitled recipe".to_string());
    let written = Written::read(recipe["description"].as_str().unwrap_or(""));
    let items = recipe["items"].as_array().map_or(&[][..], Vec::as_slice);
    let needed: Vec<Value> = items
        .iter()
        .filter(|item| item["optional"] != json!(true))
        .filter_map(line)
        .collect();
    let optional: Vec<Value> = items
        .iter()
        .filter(|item| item["optional"] == json!(true))
        .filter_map(line)
        .collect();

    // The recipe's own Language words what is added to it, so it is read from
    // what the cook wrote first, none of those words in it.
    let language = crate::language::detect_content(&json!({
        "title": title,
        "note": written.note.join("\n\n"),
        "ingredients": needed.iter().chain(&optional).collect::<Vec<_>>(),
        "steps": written.steps,
    }));
    let words = crate::schema_org::words(language);

    let mut ingredients = needed;
    if !optional.is_empty() {
        let section = match language {
            Some("fr") => "Facultatif",
            Some("es") => "Opcional",
            _ => "Optional",
        };
        ingredients.push(json!({ "kind": "section", "text": section }));
        ingredients.extend(optional);
    }

    // A whole number, none where it is zero.
    let whole = |field: &str| recipe[field].as_i64().filter(|whole| *whole > 0);
    let (prep, cook) = (whole("prep_time"), whole("cook_time"));
    let mut paragraphs = written.note;
    if let Some(total) = whole("time")
        && total != prep.unwrap_or(0) + cook.unwrap_or(0)
    {
        paragraphs.push(format!(
            "{} {}",
            words.total,
            crate::schema_org::spoken(total, &words)
        ));
    }
    let source = match text(recipe, "source") {
        Some(link) if link.starts_with("http://") || link.starts_with("https://") => {
            other_apps::source_of_link(&link)
        }
        Some(text) => json!({ "text": text, "link": null }),
        None => Value::Null,
    };
    let candidate = json!({
        "title": title,
        "yield": whole("yields")
            .map(|amount| json!({ "amount": amount.to_string(), "noun": words.servings })),
        "prep_time_minutes": prep,
        "cook_time_minutes": cook,
        "source": source,
        "note": if paragraphs.is_empty() { Value::Null } else { json!(paragraphs.join("\n\n")) },
        "nutrition": null,
        "ingredients": ingredients,
        "steps": written.steps,
    });

    let mut tags: Vec<String> = Vec::new();
    for tag in recipe["tags"].as_array().into_iter().flatten() {
        if let Some(name) = tag.as_str().map(str::trim).filter(|n| !n.is_empty())
            && !tags.iter().any(|tag| tag == name)
        {
            tags.push(name.to_string());
        }
    }

    other_apps::Recipe {
        foreign_id,
        cookings: Vec::new(),
        candidate,
        tags,
        photo: text(recipe, "photo").map(|_| other_apps::Photo::NotInExport),
        step_photos: Vec::new(),
        components: Vec::new(),
        left_out: Vec::new(),
    }
}

/// An item as an Ingredient Line: `225 g Butter`, `2 Oranges, zest and juice`,
/// `Icing sugar, to dust`, or the name alone.
fn line(item: &Value) -> Option<Value> {
    let name = text(item, "name")?;
    let text = match text(item, "description") {
        None => name,
        Some(said) if said.starts_with(|c: char| c.is_ascii_digit() || is_fraction(c)) => {
            match aside(&said) {
                Some(at) if !said[at + 1..].trim().is_empty() => {
                    format!(
                        "{} {name}, {}",
                        said[..at].trim_end(),
                        said[at + 1..].trim()
                    )
                }
                _ => format!("{} {name}", said.trim_end_matches(',')),
            }
        }
        Some(said) => format!("{name}, {said}"),
    };
    Some(json!({ "kind": "ingredient", "text": text }))
}

/// Where an amount's words turn to an aside: the first comma outside
/// brackets that no figure follows. `2, zest and juice` turns at its comma;
/// `1,5 kg`, `1,000 g` and `2 (about 1, maybe 2)` do not.
fn aside(said: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (at, c) in said.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 && !said[at + 1..].starts_with(|c: char| c.is_ascii_digit()) => {
                return Some(at);
            }
            _ => {}
        }
    }
    None
}

/// A fraction written as one character: `½`, `⅓`.
fn is_fraction(c: char) -> bool {
    matches!(c, '¼'..='¾' | '⅐'..='⅞')
}

/// The description, read as the Note and the Steps.
#[derive(Debug, Default, PartialEq)]
struct Written {
    note: Vec<String>,
    steps: Vec<Value>,
}

/// One piece of the description's markdown.
#[derive(Debug, PartialEq)]
enum Block {
    Heading(String),
    /// An item of a list, numbered or not, its marker left off.
    Item(String),
    /// A paragraph, its lines as written.
    Text(Vec<String>),
}

impl Written {
    fn read(description: &str) -> Written {
        let blocks = blocks(description);
        let step = |text: &str| json!({ "kind": "step", "text": text, "photo": null });
        let section = |text: &str| json!({ "kind": "section", "text": text, "photo": null });
        let mut written = Written::default();
        let items: Vec<usize> = (0..blocks.len())
            .filter(|&at| matches!(blocks[at], Block::Item(_)))
            .collect();
        let (Some(&first), Some(&last)) = (items.first(), items.last()) else {
            // No list: each line is a Step.
            for block in &blocks {
                match block {
                    Block::Heading(text) => written.steps.push(section(text)),
                    Block::Item(_) => {}
                    Block::Text(lines) => written.steps.extend(lines.iter().map(|l| step(l))),
                }
            }
            return written;
        };
        // The headings right above the list name its first Section.
        let mut opens = first;
        while opens > 0 && matches!(blocks[opens - 1], Block::Heading(_)) {
            opens -= 1;
        }
        for (at, block) in blocks.iter().enumerate() {
            let in_list = (opens..=last).contains(&at);
            match block {
                Block::Item(text) => written.steps.push(step(text)),
                Block::Heading(text) if in_list => written.steps.push(section(text)),
                Block::Text(lines) if in_list => {
                    written.steps.extend(lines.iter().map(|l| step(l)));
                }
                Block::Heading(text) => written.note.push(text.clone()),
                Block::Text(lines) => written.note.push(lines.join("\n")),
            }
        }
        written
    }
}

/// The description's markdown as headings, list items and paragraphs. A line
/// that follows a list item with no blank line between, or that is indented
/// under one, carries on that item.
fn blocks(description: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    // Whether the last block is still open to the next line.
    let mut open = false;
    for raw in description.lines() {
        let line = raw.trim();
        let rule = line.len() >= 3
            && (line.chars().all(|c| c == '-' || c == ' ')
                || line.chars().all(|c| c == '*' || c == ' ')
                || line.chars().all(|c| c == '_' || c == ' '));
        if line.is_empty() || rule {
            open = false;
            continue;
        }
        if let Some(heading) = heading(line) {
            blocks.push(Block::Heading(heading.to_string()));
            open = false;
        } else if let Some(item) = list_item(line) {
            blocks.push(Block::Item(item.to_string()));
            open = true;
        } else if let Some(Block::Item(item)) = blocks.last_mut()
            && (open || raw.starts_with([' ', '\t']))
        {
            item.push(' ');
            item.push_str(line);
            open = true;
        } else if let (true, Some(Block::Text(lines))) = (open, blocks.last_mut()) {
            lines.push(line.to_string());
        } else {
            blocks.push(Block::Text(vec![line.to_string()]));
            open = true;
        }
    }
    blocks
}

/// A markdown heading's text: `## Syrup` is `Syrup`.
fn heading(line: &str) -> Option<&str> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    let text = rest
        .strip_prefix([' ', '\t'])?
        .trim()
        .trim_end_matches('#')
        .trim();
    (!text.is_empty()).then_some(text)
}

/// A list item's text, its marker left off: `1. Heat the oven`, `2) Beat`,
/// `- Beat`, `* Beat`, `+ Beat`.
fn list_item(line: &str) -> Option<&str> {
    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = if (1..=9).contains(&digits) {
        line[digits..].strip_prefix(['.', ')'])?
    } else {
        line.strip_prefix(['-', '*', '+'])?
    };
    let text = rest.strip_prefix([' ', '\t'])?.trim();
    (!text.is_empty()).then_some(text)
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

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/kitchenowl")
            .join(name);
        std::fs::read(path).unwrap()
    }

    fn export_of(bytes: Vec<u8>) -> other_apps::Export {
        KitchenOwl
            .read(&Handed::bytes(bytes))
            .unwrap_or_else(|_| panic!("the file was refused"))
            .unwrap_or_else(|| panic!("the file was not recognised"))
    }

    fn read_all(bytes: Vec<u8>) -> Vec<Result<Recipe, Unread>> {
        let export = export_of(bytes);
        assert_eq!(export.app, "kitchenowl");
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

    /// Each row of a list as its text, a Section as `## text`.
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

    /// The real export with its household changed by `change`, as KitchenOwl
    /// would have written it had the cook done so.
    fn export_with(change: impl Fn(&mut Value)) -> Vec<u8> {
        let mut household: Value = serde_json::from_slice(&fixture("v0.7.10-export.json")).unwrap();
        change(&mut household);
        household.to_string().into_bytes()
    }

    #[test]
    fn every_recipe_of_the_real_export_is_read() {
        let recipes = read_all(fixture("v0.7.10-export.json"));
        let titles: Vec<&str> = recipes
            .iter()
            .flatten()
            .map(|recipe| recipe.candidate["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, ["Orange cake", "Pâtes à la tomate", "Lemonade"]);
        let ids: Vec<&str> = recipes
            .iter()
            .flatten()
            .map(|recipe| recipe.foreign_id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "Home › Orange cake",
                "Home › Pâtes à la tomate",
                "Home › Lemonade"
            ]
        );
    }

    #[test]
    fn a_list_is_the_steps_and_the_paragraphs_around_it_the_note() {
        let recipes = read_all(fixture("v0.7.10-export.json"));
        let cake = titled(&recipes, "Orange cake");
        assert_eq!(
            rows(cake, "steps"),
            [
                "## Cake",
                "Heat the oven to 180 °C and line a 20 cm tin.",
                "Beat the butter and sugar until pale, then beat in the eggs one at a time.",
                "Fold in the flour and the orange zest, and scrape into the tin.",
                "Bake for 45 minutes.",
                "## Syrup",
                "Warm the orange juice with the sugar until it dissolves.",
                "Pour over the warm cake.",
            ]
        );
        // The total is the prep and cook times added up, and says nothing more.
        assert_eq!(
            cake.candidate["note"],
            json!("A moist cake with a sharp orange syrup.\n\nKeeps for three days in a tin.")
        );

        let pasta = titled(&recipes, "Pâtes à la tomate");
        assert_eq!(
            rows(pasta, "steps"),
            [
                "Faire revenir l'ail dans l'huile d'olive.",
                "Ajouter les tomates et laisser mijoter dix minutes.",
                "Cuire les pâtes dans une grande casserole d'eau salée.",
                "Mélanger les pâtes avec la sauce.",
            ]
        );
        assert_eq!(
            pasta.candidate["note"],
            json!(
                "Un plat rapide pour les soirs de semaine.\n\nBon appétit !\n\n\
                 Temps total : 20 minutes"
            )
        );
    }

    #[test]
    fn with_no_list_each_line_is_a_step() {
        let recipes = read_all(fixture("v0.7.10-export.json"));
        let lemonade = titled(&recipes, "Lemonade");
        assert_eq!(
            rows(lemonade, "steps"),
            [
                "Squeeze the lemons.",
                "Stir in the sugar until it dissolves.",
                "Top up with cold water and serve over ice.",
            ]
        );
        // KitchenOwl's total time is the only time it has.
        assert_eq!(lemonade.candidate["prep_time_minutes"], Value::Null);
        assert_eq!(lemonade.candidate["cook_time_minutes"], Value::Null);
        assert_eq!(lemonade.candidate["note"], json!("Total time: 10 minutes"));
    }

    #[test]
    fn amounts_and_names_join_into_lines_and_optional_items_come_last() {
        let recipes = read_all(fixture("v0.7.10-export.json"));
        assert_eq!(
            rows(titled(&recipes, "Orange cake"), "ingredients"),
            [
                "225 g Butter",
                "4 Eggs",
                "2 Oranges, zest and juice",
                "Salt",
                "225 g Self-raising flour",
                "225 g Sugar",
                "## Optional",
                "Icing sugar, to dust",
            ]
        );
        assert_eq!(
            rows(titled(&recipes, "Pâtes à la tomate"), "ingredients"),
            [
                "2 gousses Ail",
                "2 c. à soupe Huile d'olive",
                "200 g Pâtes",
                "1 boîte Tomates concassées",
                "## Facultatif",
                "Parmesan, un peu",
            ]
        );
    }

    #[test]
    fn the_rest_of_a_recipe_lands_in_its_fields() {
        let recipes = read_all(fixture("v0.7.10-export.json"));
        let cake = titled(&recipes, "Orange cake");
        assert_eq!(
            cake.candidate["yield"],
            json!({ "amount": "8", "noun": "servings" })
        );
        assert_eq!(cake.candidate["prep_time_minutes"], json!(20));
        assert_eq!(cake.candidate["cook_time_minutes"], json!(45));
        assert_eq!(
            cake.candidate["source"],
            json!({ "text": "example.com", "link": "https://example.com/orange-cake" })
        );
        assert_eq!(cake.tags, ["Baking", "Dessert"]);
        assert!(matches!(cake.photo, Some(Photo::NotInExport)));
        assert!(cake.left_out.is_empty());

        let pasta = titled(&recipes, "Pâtes à la tomate");
        assert_eq!(
            pasta.candidate["yield"],
            json!({ "amount": "2", "noun": "portions" })
        );
        assert_eq!(pasta.candidate["source"], Value::Null);
        assert_eq!(pasta.tags, ["Pâtes", "Rapide"]);

        // A source that is no web address is kept as words.
        let lemonade = titled(&recipes, "Lemonade");
        assert_eq!(
            lemonade.candidate["source"],
            json!({ "text": "Grandma's notebook", "link": null })
        );
        assert!(lemonade.photo.is_none());
    }

    #[test]
    fn what_the_household_kept_beside_its_recipes_is_counted() {
        let export = export_of(fixture("v0.7.10-export.json"));
        let rows: Vec<Value> = export.left_out.iter().map(FileLeftOut::row).collect();
        assert_eq!(
            rows,
            [
                json!({ "what": "shopping_lists", "count": 2 }),
                json!({ "what": "items", "count": 660 }),
                json!({ "what": "expenses", "count": 1 }),
            ]
        );
    }

    #[test]
    fn two_recipes_of_one_name_are_numbered() {
        let file = export_with(|household| {
            let recipes = household["recipes"].as_array_mut().unwrap();
            let mut again = recipes[2].clone();
            again["description"] = json!("Another way.");
            recipes.push(again);
            let mut third = recipes[2].clone();
            third["name"] = json!("Lemonade (2)");
            recipes.push(third);
        });
        let ids: Vec<String> = read_all(file)
            .into_iter()
            .map(|recipe| recipe.ok().unwrap().foreign_id)
            .collect();
        assert_eq!(
            ids,
            [
                "Home › Orange cake",
                "Home › Pâtes à la tomate",
                "Home › Lemonade",
                "Home › Lemonade (3)",
                "Home › Lemonade (2)",
            ]
        );
    }

    #[test]
    fn a_kitchenowl_export_in_another_shape_is_refused() {
        let file = export_with(|household| {
            household["recipes"][0]["yields"] = json!("8 slices");
        });
        let refused = KitchenOwl.read(&Handed::bytes(file)).err().unwrap();
        assert_eq!(refused, Refusal::UntestedShape { app: APP });
    }

    #[test]
    fn another_file_is_not_kitchenowls() {
        for bytes in [
            b"{\"recipes\": []}".to_vec(),
            b"[{\"name\": \"x\"}]".to_vec(),
            b"not json".to_vec(),
            fixture("../tandoor/v2.6-schema-org.json"),
            fixture("../paprika/mealie-sample.zip"),
        ] {
            assert!(matches!(KitchenOwl.read(&Handed::bytes(bytes)), Ok(None)));
        }
    }

    #[test]
    fn markdown_is_read_as_a_cook_wrote_it() {
        let steps = |description: &str| -> Vec<String> {
            Written::read(description)
                .steps
                .iter()
                .map(|row| row["text"].as_str().unwrap().to_string())
                .collect()
        };
        // A paragraph inside the list is a step of its own, a line carrying on
        // an item with no blank line between joins it, and a rule is nothing.
        assert_eq!(
            steps("* Mix\nwell.\n\nRest it.\n\n---\n\n2) Bake"),
            ["Mix well.", "Rest it.", "Bake"]
        );
        // `**bold**` and `#hashtag` are not a list or a heading.
        assert_eq!(
            steps("**Mix** the dough.\n#rest overnight"),
            ["**Mix** the dough.", "#rest overnight"]
        );
        let written = Written::read(
            "## Notes\nOld family recipe.\n\n# Method\n1. Mix\n\n## Tip\nServe cold.",
        );
        assert_eq!(
            written.note,
            ["Notes", "Old family recipe.", "Tip", "Serve cold."]
        );
        assert_eq!(written.steps.len(), 2);
        assert_eq!(written.steps[0]["kind"], "section");
        assert_eq!(written.steps[0]["text"], "Method");
        assert_eq!(Written::read(""), Written::default());
    }

    #[test]
    fn a_line_is_written_as_a_cook_writes_one() {
        let line = |description: &str| {
            line(&json!({ "name": "Flour", "description": description, "optional": false }))
                .unwrap()["text"]
                .as_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(line("200 g"), "200 g Flour");
        assert_eq!(line("½ cup"), "½ cup Flour");
        assert_eq!(line("1 cup, sifted"), "1 cup Flour, sifted");
        // A decimal comma, a thousands comma and a comma in brackets are the
        // amount's own.
        assert_eq!(line("1,5 kg"), "1,5 kg Flour");
        assert_eq!(line("1,000 g, sifted"), "1,000 g Flour, sifted");
        assert_eq!(line("2 (about 1, maybe 2)"), "2 (about 1, maybe 2) Flour");
        assert_eq!(line("a pinch"), "Flour, a pinch");
        assert_eq!(line(""), "Flour");
        assert_eq!(line("  "), "Flour");
    }
}
