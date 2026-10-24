//! Reading a Mealie backup (#418, the spec is #411): Admin, Backups, which
//! only an administrator makes. It is a zip of `database.json`, every table
//! of the instance written out row by row, beside `data/`, which holds each
//! recipe's pictures under `data/recipes/<id>/`.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). The files
//! in `tests/fixtures/mealie/`, which its README describes, are what it was
//! tested against.
//!
//! - **A recipe reads as Mealie's export writes it.** Each one is put back
//!   together from its tables into the shape of the export's recipe, and read
//!   by the export's own reader ([`crate::mealie`]), so an export and a backup
//!   of the same Mealie give the same recipes, under the same ids. The backup
//!   keeps no line as Mealie shows it, so each is written the way Mealie's
//!   screen writes it, from the quantity, the unit, the food and the note
//!   ([`display`]).
//! - **Only what a recipe needs is read.** Each table is read into a shape
//!   naming the columns this reader uses, and every other column is skipped
//!   unread: of the users, their id, full name, group and household. No
//!   password, token or secret column is kept, and `data/.secret` is never
//!   opened.
//! - **A recipe belongs to its author's household**, and before Mealie 2,
//!   which had none, to its group. Where the backup holds the recipes of more
//!   than one, nothing is imported until the cook says which is theirs
//!   ([`Refusal::WhichHousehold`]), and then that one's alone.
//! - **"I Made This" becomes a cooking**: Mealie's `comment` timeline entries,
//!   each with its message and photo, which land as finished Attempts that
//!   name their cook (ADR 0053). An entry Mealie wrote from its meal plan is
//!   an `info` one, and is counted on the Report. The entry's subject is
//!   never read: Mealie writes it in the cook's language.
//! - **A revision is read only when a backup of it is among the test files**:
//!   the database revision a backup carries says which shape its tables have,
//!   and any other is refused with the update reason.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::mealie::{self, Archive, Made};
use crate::other_apps::{
    self, Budget, Cooking, FileLeftOut, Handed, Household, LeftOut, Reader, Refusal,
};

/// Mealie's backup among the readers of another app's export.
pub struct MealieBackup;

const APP: &str = "Mealie";

/// The database revisions a real backup of is among the test files, oldest
/// first, with what changed in each that this reader reads. Every other
/// revision is refused.
const TESTED: &[&str] = &[
    // August 2022: no timeline, no households, no plural names.
    "44e8d670719d",
    // August 2023: "I Made This" has its timeline.
    "bcfdad6b7355",
    // October 2023: units and foods have plural names.
    "ba1e4a6cfe99",
    // March 2024, zipped in a folder of its own by Safari.
    "09aba125b57a",
    // October 2024: households, and each cook's rating and favourite.
    "86054b40fd06",
    // September 2025: a line may point at another recipe.
    "1d9a002d7234",
    // October 2026, Mealie 3.29.0.
    "1e69ea7cd45e",
];

/// The revisions older than Mealie's data fix of July 2025 (`d7b3ce6fa31a`),
/// which turned the quantity 1 of every line of a recipe that hides its
/// amounts into nothing: older Mealie wrote 1 on a line that was its note
/// alone. A backup of one of these, restored into Mealie today, has the fix
/// run on it, so the reader runs it too.
const BEFORE_QUANTITY_FIX: &[&str] = &[
    "44e8d670719d",
    "bcfdad6b7355",
    "ba1e4a6cfe99",
    "09aba125b57a",
    "86054b40fd06",
];

/// The largest `database.json` read. A recipe's rows are a few kilobytes, so
/// this is some hundred thousand recipes.
const MAX_DATABASE_BYTES: u64 = 1024 * 1024 * 1024;

impl Reader for MealieBackup {
    fn names(&self) -> &'static [&'static str] {
        // The export's reader names Mealie for both.
        &[]
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
        let Some((root, index)) = database_of(&archive) else {
            return Ok(None);
        };
        let mut budget = Budget::new(other_apps::MOST_UNPACKED);
        let Ok(bytes) = mealie::unpack_at_most(
            &mut archive,
            index,
            &mut budget,
            Made::Backup,
            MAX_DATABASE_BYTES,
        ) else {
            return Ok(None);
        };
        let Ok(database) = serde_json::from_slice::<Database>(&bytes) else {
            return Ok(None);
        };
        drop(bytes);
        let Some(revision) = database.alembic_version.first() else {
            return Ok(None);
        };
        if !TESTED.contains(&revision.version_num.as_str()) {
            return Err(Refusal::UntestedShape { app: APP });
        }

        let quantity_fix = BEFORE_QUANTITY_FIX.contains(&revision.version_num.as_str());
        let library = Library::of(database, quantity_fix);
        let chosen = library.chosen(handed.household())?;
        let folders = recipe_folders(&archive, &root);
        let total = chosen.recipes.len();
        let left_out = [
            FileLeftOut::MealPlans(chosen.meal_plans),
            FileLeftOut::ShoppingLists(chosen.shopping_lists),
        ]
        .into_iter()
        .filter(|left| left.count() > 0)
        .collect();
        let recipes = chosen.recipes.into_iter().map(move |at| {
            let folder_of = |id: &str| {
                folders
                    .get(id)
                    .map(|folder| format!("{root}data/recipes/{folder}/"))
            };
            library.read(&mut archive, &library.recipes[at], &folder_of, &mut budget)
        });
        Ok(Some(other_apps::Export {
            app: "mealie",
            total,
            recipes: Box::new(recipes),
            left_out,
        }))
    }
}

/// Where the backup's `database.json` is: the folder everything sits in,
/// which is the zip's top unless Safari put it all in a folder of its own,
/// and the file's place in the zip.
fn database_of(archive: &Archive) -> Option<(String, usize)> {
    (0..archive.len())
        .filter_map(|index| {
            let name = archive.name_for_index(index)?;
            if name.starts_with("__MACOSX/") {
                return None;
            }
            let root = name.strip_suffix("database.json")?;
            let shallow = root.is_empty() || root.matches('/').count() == 1;
            (shallow && (root.is_empty() || root.ends_with('/'))).then(|| (root.to_string(), index))
        })
        .min_by_key(|(root, _)| root.len())
}

/// Each recipe's folder under `data/recipes/`, by the recipe's id as this
/// reader writes it. Mealie names the folder by the id with or without its
/// dashes, by version.
fn recipe_folders(archive: &Archive, root: &str) -> HashMap<String, String> {
    let under = format!("{root}data/recipes/");
    archive
        .file_names()
        .filter_map(|name| name.strip_prefix(&under)?.split_once('/'))
        .filter_map(|(folder, _)| Some((uuid(&json!(folder))?, folder.to_string())))
        .collect()
}

/// An id as Mealie's export writes it: a UUID with its dashes, which its
/// tables keep without them, or a number as it is.
fn uuid(id: &Value) -> Option<String> {
    let id = match id {
        Value::String(id) => id.trim().to_ascii_lowercase(),
        Value::Number(id) => return Some(id.to_string()),
        _ => return None,
    };
    if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Some(format!(
            "{}-{}-{}-{}-{}",
            &id[..8],
            &id[8..12],
            &id[12..16],
            &id[16..20],
            &id[20..]
        ));
    }
    (!id.is_empty()).then_some(id)
}

// --- The tables -------------------------------------------------------------
//
// Each names only the columns this reader uses; serde skips every other one
// without keeping it. A table or a column a revision does not have reads as
// empty.

#[derive(Deserialize, Default)]
#[serde(default)]
struct Database {
    alembic_version: Vec<Revision>,
    groups: Vec<Group>,
    households: Vec<HouseholdRow>,
    users: Vec<User>,
    recipes: Vec<RecipeRow>,
    recipes_ingredients: Vec<LineRow>,
    ingredient_units: Vec<Unit>,
    ingredient_foods: Vec<Food>,
    recipe_instructions: Vec<StepRow>,
    recipe_nutrition: Vec<Map<String, Value>>,
    notes: Vec<NoteRow>,
    recipe_assets: Vec<AssetRow>,
    categories: Vec<Named>,
    recipes_to_categories: Vec<ToCategory>,
    tags: Vec<Named>,
    recipes_to_tags: Vec<ToTag>,
    tools: Vec<Named>,
    recipes_to_tools: Vec<ToTool>,
    recipe_comments: Vec<OfRecipe>,
    recipe_settings: Vec<Settings>,
    users_to_recipes: Vec<Judged>,
    users_to_favorites: Vec<OfRecipe>,
    recipe_timeline_events: Vec<Event>,
    group_meal_plans: Vec<Owned>,
    shopping_lists: Vec<Owned>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Revision {
    version_num: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Group {
    id: Value,
    name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HouseholdRow {
    id: Value,
    name: Option<String>,
    group_id: Value,
}

/// A user: who they are and where, and nothing else of the row.
#[derive(Deserialize, Default)]
#[serde(default)]
struct User {
    id: Value,
    full_name: Option<String>,
    group_id: Value,
    household_id: Value,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RecipeRow {
    #[serde(deserialize_with = "an_id")]
    id: String,
    user_id: Value,
    group_id: Value,
    slug: Option<String>,
    name: Option<String>,
    description: Option<String>,
    image: Value,
    total_time: Value,
    prep_time: Value,
    perform_time: Value,
    cook_time: Value,
    total_time_seconds: Value,
    prep_time_seconds: Value,
    perform_time_seconds: Value,
    recipe_yield: Value,
    recipe_yield_quantity: Value,
    recipe_servings: Value,
    org_url: Value,
    rating: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LineRow {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    position: Option<i64>,
    title: Option<String>,
    note: Option<String>,
    quantity: Option<f64>,
    unit_id: Value,
    food_id: Value,
    referenced_recipe_id: Value,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct Unit {
    id: Value,
    name: Option<String>,
    plural_name: Option<String>,
    abbreviation: Option<String>,
    plural_abbreviation: Option<String>,
    use_abbreviation: Option<bool>,
    fraction: Option<bool>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct Food {
    id: Value,
    name: Option<String>,
    plural_name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct StepRow {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    position: Option<i64>,
    title: Option<String>,
    text: Option<String>,
    summary: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NoteRow {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    title: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct AssetRow {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    name: Option<String>,
    file_name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Named {
    id: Value,
    name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ToCategory {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    category_id: Value,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ToTag {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    tag_id: Value,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ToTool {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    tool_id: Value,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OfRecipe {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
}

/// Whether a recipe hides its amounts.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Settings {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    disable_amount: Option<bool>,
}

/// One cook's rating and favourite of one recipe.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Judged {
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    rating: Option<f64>,
    is_favorite: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Event {
    id: Value,
    #[serde(deserialize_with = "an_id")]
    recipe_id: String,
    user_id: Value,
    event_type: Option<String>,
    message: Option<String>,
    timestamp: Option<String>,
    created_at: Option<String>,
}

/// A meal plan or a shopping list, and whose it is.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Owned {
    group_id: Value,
    household_id: Value,
    user_id: Value,
}

/// An id column, read as [`uuid`] writes it, or empty where it holds none.
fn an_id<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(uuid(&Value::deserialize(deserializer)?).unwrap_or_default())
}

// --- The library ------------------------------------------------------------

/// What a household's cook chose, read out of the backup.
struct Chosen {
    /// Each recipe's place in [`Library::recipes`].
    recipes: Vec<usize>,
    meal_plans: usize,
    shopping_lists: usize,
}

/// The backup's tables, gathered by recipe.
struct Library {
    recipes: Vec<RecipeRow>,
    by_id: HashMap<String, usize>,
    lines: HashMap<String, Vec<LineRow>>,
    units: HashMap<String, Unit>,
    foods: HashMap<String, Food>,
    steps: HashMap<String, Vec<StepRow>>,
    nutrition: HashMap<String, Map<String, Value>>,
    notes: HashMap<String, Vec<NoteRow>>,
    assets: HashMap<String, Vec<AssetRow>>,
    categories: HashMap<String, Vec<String>>,
    tags: HashMap<String, Vec<String>>,
    tools: HashMap<String, Vec<String>>,
    comments: HashMap<String, usize>,
    ratings: HashMap<String, usize>,
    favourites: HashMap<String, usize>,
    events: HashMap<String, Vec<Event>>,
    /// Each user's full name, by their id.
    cooks: HashMap<String, String>,
    homes: Homes,
    meal_plans: Vec<Option<String>>,
    shopping_lists: Vec<Option<String>>,
}

/// The households a recipe can belong to: Mealie's households, or before
/// Mealie 2 its groups.
struct Homes {
    /// Each home's id, name and cooks, in the backup's order.
    all: Vec<(String, String, Vec<String>)>,
    /// Each user's home.
    of_user: HashMap<String, String>,
    /// Whether a group is a home, as before Mealie 2.
    groups_are_homes: bool,
}

impl Homes {
    fn of(database: &Database) -> Homes {
        let groups_are_homes = database.households.is_empty();
        let group_names: HashMap<String, String> = database
            .groups
            .iter()
            .filter_map(|group| Some((uuid(&group.id)?, group.name.clone().unwrap_or_default())))
            .collect();
        let of_user: HashMap<String, String> = database
            .users
            .iter()
            .filter_map(|user| {
                let home = if groups_are_homes {
                    &user.group_id
                } else {
                    &user.household_id
                };
                Some((uuid(&user.id)?, uuid(home)?))
            })
            .collect();
        let rows: Vec<(String, String, Option<String>)> = if groups_are_homes {
            database
                .groups
                .iter()
                .filter_map(|group| {
                    Some((
                        uuid(&group.id)?,
                        group.name.clone().unwrap_or_default(),
                        None,
                    ))
                })
                .collect()
        } else {
            database
                .households
                .iter()
                .filter_map(|household| {
                    Some((
                        uuid(&household.id)?,
                        household.name.clone().unwrap_or_default(),
                        uuid(&household.group_id),
                    ))
                })
                .collect()
        };
        let all = rows
            .into_iter()
            .map(|(home, name, group)| {
                let cooks: Vec<String> = database
                    .users
                    .iter()
                    .filter(|user| {
                        uuid(&user.id).and_then(|user| of_user.get(&user)) == Some(&home)
                    })
                    .filter_map(|user| user.full_name.clone())
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect();
                let group = group.and_then(|group| group_names.get(&group).cloned());
                (
                    home,
                    name.trim().to_string(),
                    group.unwrap_or_default(),
                    cooks,
                )
            })
            .collect::<Vec<_>>();
        // Mealie names every group's first household "Family", so a name two
        // groups share is said with its group's.
        let named = all
            .iter()
            .map(|(home, name, group, cooks)| {
                let shared = all.iter().filter(|(_, other, ..)| other == name).count() > 1;
                let name = if name.is_empty() {
                    home.clone()
                } else if shared && !group.is_empty() {
                    format!("{name} ({group})")
                } else {
                    name.clone()
                };
                (home.clone(), name, cooks.clone())
            })
            .collect();
        Homes {
            all: named,
            of_user,
            groups_are_homes,
        }
    }

    /// The home of something a user made in a group, or that names its
    /// household itself.
    fn home(&self, household: &Value, user: &Value, group: &Value) -> Option<String> {
        if !self.groups_are_homes
            && let Some(household) = uuid(household)
        {
            return Some(household);
        }
        if let Some(home) = uuid(user).and_then(|user| self.of_user.get(&user)) {
            return Some(home.clone());
        }
        self.groups_are_homes.then(|| uuid(group)).flatten()
    }
}

impl Library {
    fn of(database: Database, quantity_fix: bool) -> Library {
        let homes = Homes::of(&database);
        // In the order of the linked rows' ids, which is the order Mealie's
        // export lists them in.
        let by_recipe = |mut rows: Vec<(String, String, String)>| {
            rows.sort_by(|a, b| a.1.cmp(&b.1));
            let mut gathered: HashMap<String, Vec<String>> = HashMap::new();
            for (recipe, _, name) in rows {
                let names = gathered.entry(recipe).or_default();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            gathered
        };
        let names = |rows: &[Named]| -> HashMap<String, String> {
            rows.iter()
                .filter_map(|row| Some((uuid(&row.id)?, row.name.clone()?.trim().to_string())))
                .filter(|(_, name)| !name.is_empty())
                .collect()
        };
        let category_names = names(&database.categories);
        let tag_names = names(&database.tags);
        let tool_names = names(&database.tools);
        let counted = |recipes: &mut dyn Iterator<Item = &String>| {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for recipe in recipes {
                *counts.entry(recipe.clone()).or_default() += 1;
            }
            counts
        };
        let hides_amounts: std::collections::HashSet<&str> = database
            .recipe_settings
            .iter()
            .filter(|settings| quantity_fix && settings.disable_amount == Some(true))
            .map(|settings| settings.recipe_id.as_str())
            .collect();
        let mut lines: HashMap<String, Vec<LineRow>> = HashMap::new();
        for mut line in database.recipes_ingredients {
            if line.quantity == Some(1.0) && hides_amounts.contains(line.recipe_id.as_str()) {
                line.quantity = Some(0.0);
            }
            lines.entry(line.recipe_id.clone()).or_default().push(line);
        }
        for rows in lines.values_mut() {
            rows.sort_by_key(|line| line.position.unwrap_or(0));
        }
        let mut steps: HashMap<String, Vec<StepRow>> = HashMap::new();
        for step in database.recipe_instructions {
            steps.entry(step.recipe_id.clone()).or_default().push(step);
        }
        for rows in steps.values_mut() {
            rows.sort_by_key(|step| step.position.unwrap_or(0));
        }
        let mut notes: HashMap<String, Vec<NoteRow>> = HashMap::new();
        for note in database.notes {
            notes.entry(note.recipe_id.clone()).or_default().push(note);
        }
        let mut assets: HashMap<String, Vec<AssetRow>> = HashMap::new();
        for asset in database.recipe_assets {
            assets
                .entry(asset.recipe_id.clone())
                .or_default()
                .push(asset);
        }
        let mut events: HashMap<String, Vec<Event>> = HashMap::new();
        for event in database.recipe_timeline_events {
            events
                .entry(event.recipe_id.clone())
                .or_default()
                .push(event);
        }
        let nutrition = database
            .recipe_nutrition
            .into_iter()
            .filter_map(|mut row| {
                let recipe = uuid(&row.remove("recipe_id")?)?;
                for bookkeeping in ["id", "created_at", "update_at"] {
                    row.remove(bookkeeping);
                }
                Some((recipe, row))
            })
            .collect();
        let mut recipes = database.recipes;
        recipes.retain(|recipe| !recipe.id.is_empty());
        // In the order the export writes its folders, which is by slug.
        recipes.sort_by(|a, b| a.slug.cmp(&b.slug).then_with(|| a.id.cmp(&b.id)));
        let by_id = recipes
            .iter()
            .enumerate()
            .map(|(at, recipe)| (recipe.id.clone(), at))
            .collect();
        Library {
            by_id,
            lines,
            units: database
                .ingredient_units
                .into_iter()
                .filter_map(|unit| Some((uuid(&unit.id)?, unit)))
                .collect(),
            foods: database
                .ingredient_foods
                .into_iter()
                .filter_map(|food| Some((uuid(&food.id)?, food)))
                .collect(),
            steps,
            nutrition,
            notes,
            assets,
            categories: by_recipe(
                database
                    .recipes_to_categories
                    .iter()
                    .filter_map(|link| {
                        let linked = uuid(&link.category_id)?;
                        let name = category_names.get(&linked)?;
                        Some((link.recipe_id.clone(), linked, name.clone()))
                    })
                    .collect(),
            ),
            tags: by_recipe(
                database
                    .recipes_to_tags
                    .iter()
                    .filter_map(|link| {
                        let linked = uuid(&link.tag_id)?;
                        let name = tag_names.get(&linked)?;
                        Some((link.recipe_id.clone(), linked, name.clone()))
                    })
                    .collect(),
            ),
            tools: by_recipe(
                database
                    .recipes_to_tools
                    .iter()
                    .filter_map(|link| {
                        let linked = uuid(&link.tool_id)?;
                        let name = tool_names.get(&linked)?;
                        Some((link.recipe_id.clone(), linked, name.clone()))
                    })
                    .collect(),
            ),
            comments: counted(&mut database.recipe_comments.iter().map(|row| &row.recipe_id)),
            ratings: counted(
                &mut database
                    .users_to_recipes
                    .iter()
                    .filter(|row| row.rating.is_some_and(|rating| rating > 0.0))
                    .map(|row| &row.recipe_id),
            ),
            favourites: counted(
                &mut database
                    .users_to_recipes
                    .iter()
                    .filter(|row| row.is_favorite == Some(true))
                    .map(|row| &row.recipe_id)
                    .chain(database.users_to_favorites.iter().map(|row| &row.recipe_id)),
            ),
            events,
            cooks: database
                .users
                .iter()
                .filter_map(|user| {
                    let name = user.full_name.as_deref()?.trim();
                    (!name.is_empty()).then(|| Some((uuid(&user.id)?, name.to_string())))?
                })
                .collect(),
            meal_plans: database
                .group_meal_plans
                .iter()
                .map(|plan| homes.home(&plan.household_id, &plan.user_id, &plan.group_id))
                .collect(),
            shopping_lists: database
                .shopping_lists
                .iter()
                .map(|list| homes.home(&list.household_id, &list.user_id, &list.group_id))
                .collect(),
            homes,
            recipes,
        }
    }

    /// The recipes of the household the cook chose, or of the only one that
    /// has any. Where several have recipes and none is chosen, or one the
    /// backup does not hold, they are asked which.
    fn chosen(&self, household: Option<&str>) -> Result<Chosen, Refusal> {
        let home_of = |recipe: &RecipeRow| {
            self.homes
                .home(&Value::Null, &recipe.user_id, &recipe.group_id)
        };
        let homes: Vec<&(String, String, Vec<String>)> = self
            .homes
            .all
            .iter()
            .filter(|(home, ..)| {
                self.recipes
                    .iter()
                    .any(|recipe| home_of(recipe).as_ref() == Some(home))
            })
            .collect();
        let every = |of: &[Option<String>]| of.len();
        if homes.len() <= 1 {
            return Ok(Chosen {
                recipes: (0..self.recipes.len()).collect(),
                meal_plans: every(&self.meal_plans),
                shopping_lists: every(&self.shopping_lists),
            });
        }
        let Some((home, ..)) = homes
            .iter()
            .find(|(_, name, _)| Some(name.as_str()) == household)
        else {
            return Err(Refusal::WhichHousehold(
                homes
                    .iter()
                    .map(|(home, name, cooks)| Household {
                        name: name.clone(),
                        recipes: self
                            .recipes
                            .iter()
                            .filter(|recipe| home_of(recipe).as_ref() == Some(home))
                            .count(),
                        cooks: cooks.clone(),
                    })
                    .collect(),
            ));
        };
        let in_home = |of: &[Option<String>]| {
            of.iter()
                .filter(|owner| owner.as_ref() == Some(home))
                .count()
        };
        Ok(Chosen {
            recipes: (0..self.recipes.len())
                .filter(|&at| home_of(&self.recipes[at]).as_ref() == Some(home))
                .collect(),
            meal_plans: in_home(&self.meal_plans),
            shopping_lists: in_home(&self.shopping_lists),
        })
    }

    /// One recipe, its photos and its cookings, read out of the backup.
    fn read(
        &self,
        archive: &mut Archive,
        row: &RecipeRow,
        folder_of: &dyn Fn(&str) -> Option<String>,
        budget: &mut Budget,
    ) -> Result<other_apps::Recipe, other_apps::Unread> {
        let prefix = folder_of(&row.id);
        let prefix = prefix.as_deref();
        let unread = |reason: String| other_apps::Unread {
            foreign_id: Some(row.id.clone()),
            name: row.name.clone().or_else(|| row.slug.clone()),
            reason,
        };
        let read = mealie::read_current(&self.as_exported(row, true)).map_err(unread)?;
        let mut recipe = match prefix {
            Some(prefix) => mealie::with_photos(archive, prefix, read, budget, Made::Backup),
            None => Ok(read.recipe),
        }
        .map_err(unread)?;
        // A recipe pointed at from another household lands from its copy,
        // and its photo is in the backup, unlike in an export.
        for component in &mut recipe.components {
            let Some(copy) = component.copy.as_mut() else {
                continue;
            };
            if matches!(copy.photo, Some(other_apps::Photo::NotInExport))
                && let Some(folder) = folder_of(&component.foreign_id)
                && let Some(index) =
                    archive.index_for_name(&format!("{folder}images/original.webp"))
                && let Ok(bytes) = mealie::unpack(archive, index, budget, Made::Backup)
            {
                copy.photo = Some(other_apps::Photo::Bytes(bytes));
            }
        }

        let count = |of: &HashMap<String, usize>| of.get(&row.id).copied().unwrap_or(0);
        let ratings = match count(&self.ratings) {
            // Before Mealie 2, the recipe kept one rating of its own.
            0 => usize::from(row.rating.is_some_and(|rating| rating > 0.0)),
            ratings => ratings,
        };
        for (count, left) in [
            (ratings, LeftOut::Ratings as fn(usize) -> LeftOut),
            (count(&self.favourites), LeftOut::Favourites),
            (count(&self.comments), LeftOut::Comments),
        ] {
            if count > 0 {
                recipe.left_out.push(left(count));
            }
        }

        let mut planned = 0;
        for event in self.events.get(&row.id).into_iter().flatten() {
            match event.event_type.as_deref() {
                Some("comment") => {}
                Some("info") => {
                    planned += 1;
                    continue;
                }
                _ => continue,
            }
            let Some(foreign_id) = uuid(&event.id) else {
                continue;
            };
            let photo = prefix.and_then(|prefix| {
                let folders = [foreign_id.clone(), foreign_id.replace('-', "")];
                folders.iter().find_map(|folder| {
                    let name = format!("{prefix}images/timeline/{folder}/original.webp");
                    let index = archive.index_for_name(&name)?;
                    mealie::unpack(archive, index, budget, Made::Backup).ok()
                })
            });
            let cook = uuid(&event.user_id).and_then(|user| self.cooks.get(&user));
            let mut note = cooked_by(cook.map(String::as_str));
            if let Some(message) = event.message.as_deref().map(str::trim)
                && !message.is_empty()
            {
                note = format!("{note}\n\n{message}");
            }
            recipe.cookings.push(Cooking {
                foreign_id,
                cooked_at: event
                    .timestamp
                    .clone()
                    .or_else(|| event.created_at.clone())
                    .unwrap_or_default(),
                note,
                photo,
            });
        }
        if planned > 0 {
            recipe.left_out.push(LeftOut::MealPlanTimeline(planned));
        }
        Ok(recipe)
    }

    /// A recipe in the shape Mealie's export writes it, which the export's
    /// reader reads. A line pointing at another recipe carries that recipe
    /// whole where `whole`, as the export's does, and its name alone in the
    /// copy, so a recipe pointing back at itself is no endless copy.
    fn as_exported(&self, row: &RecipeRow, whole: bool) -> Value {
        let of = |field: &HashMap<String, Vec<String>>| {
            field
                .get(&row.id)
                .into_iter()
                .flatten()
                .map(|name| json!({ "name": name }))
                .collect::<Vec<_>>()
        };
        let lines: Vec<Value> = self
            .lines
            .get(&row.id)
            .into_iter()
            .flatten()
            .map(|line| {
                let unit = uuid(&line.unit_id).and_then(|unit| self.units.get(&unit));
                let food = uuid(&line.food_id).and_then(|food| self.foods.get(&food));
                let referenced = uuid(&line.referenced_recipe_id)
                    .and_then(|recipe| self.by_id.get(&recipe))
                    .map(|&at| {
                        let recipe = &self.recipes[at];
                        if whole {
                            self.as_exported(recipe, false)
                        } else {
                            json!({ "id": recipe.id, "name": recipe.name })
                        }
                    });
                json!({
                    "title": line.title,
                    "note": line.note,
                    "quantity": line.quantity,
                    "unit": unit.map(|unit| json!({ "name": unit.name, "plural_name": unit.plural_name })),
                    "food": food.map(|food| json!({ "name": food.name, "plural_name": food.plural_name })),
                    "referenced_recipe": referenced,
                    "display": display(line.quantity, unit, food, line.note.as_deref()),
                })
            })
            .collect();
        let steps: Vec<Value> = self
            .steps
            .get(&row.id)
            .into_iter()
            .flatten()
            .map(|step| json!({ "title": step.title, "text": step.text, "summary": step.summary }))
            .collect();
        json!({
            "id": row.id,
            "slug": row.slug,
            "name": row.name,
            "description": row.description,
            "image": row.image,
            "total_time": row.total_time,
            "prep_time": row.prep_time,
            "perform_time": row.perform_time,
            "cook_time": row.cook_time,
            "total_time_seconds": row.total_time_seconds,
            "prep_time_seconds": row.prep_time_seconds,
            "perform_time_seconds": row.perform_time_seconds,
            "recipe_yield": row.recipe_yield,
            "recipe_yield_quantity": row.recipe_yield_quantity,
            "recipe_servings": row.recipe_servings,
            "org_url": row.org_url,
            "nutrition": self.nutrition.get(&row.id),
            "recipe_ingredient": lines,
            "recipe_instructions": steps,
            "notes": self.notes.get(&row.id).into_iter().flatten()
                .map(|note| json!({ "title": note.title, "text": note.text }))
                .collect::<Vec<_>>(),
            "assets": self.assets.get(&row.id).into_iter().flatten()
                .map(|asset| json!({ "name": asset.name, "file_name": asset.file_name }))
                .collect::<Vec<_>>(),
            "tools": of(&self.tools),
            "recipe_category": of(&self.categories),
            "tags": of(&self.tags),
        })
    }
}

/// The first words of a cooking's Attempt, which say who cooked it and where:
/// the importer's own cookings say so too, since Kamosu cannot tell which
/// Mealie cook they were (ADR 0053). A cook Mealie kept no name for, which its
/// screens do not allow, is said with none.
fn cooked_by(cook: Option<&str>) -> String {
    match cook {
        Some(cook) => format!("Cooked by {cook} in Mealie."),
        None => "Cooked in Mealie.".to_string(),
    }
}

// --- A line as Mealie writes it ----------------------------------------------
//
// Mealie's `RecipeIngredientBase._format_display`, in
// `mealie/schema/recipe/recipe_ingredient.py` at v3.29.0: the quantity, the
// unit where there is a quantity, the food and the note, with single spaces.
// Mealie words a food's plural by the locale it answers in; a backup keeps
// none, so this takes the rule Mealie falls back on, a plural food where no
// unit is shown.

/// The most a fraction's denominator may be, and the decimals a quantity of
/// a unit with no fractions is rounded to, as Mealie has them.
const MAX_DENOMINATOR: u128 = 32;
const DECIMALS: i32 = 3;

fn display(
    quantity: Option<f64>,
    unit: Option<&Unit>,
    food: Option<&Food>,
    note: Option<&str>,
) -> String {
    let quantity = quantity.filter(|quantity| *quantity != 0.0);
    let mut parts: Vec<String> = Vec::new();
    if let Some(quantity) = quantity {
        parts.push(quantity_shown(quantity, unit));
        if let Some(unit) = unit {
            parts.push(unit_shown(quantity, unit));
        }
    }
    if let Some(food) = food {
        let plural = match quantity {
            Some(quantity) if quantity <= 1.0 => false,
            _ => !(quantity.is_some() && unit.is_some()),
        };
        let name = food.name.clone().unwrap_or_default();
        parts.push(if plural {
            food.plural_name
                .clone()
                .filter(|p| !p.is_empty())
                .unwrap_or(name)
        } else {
            name
        });
    }
    if let Some(note) = note.filter(|note| !note.is_empty()) {
        parts.push(note.to_string());
    }
    parts.join(" ").trim().to_string()
}

/// A quantity as Mealie writes it: decimals for a unit that takes no
/// fractions, and otherwise the nearest fraction over at most 32, in
/// superscript and subscript digits, after any whole number.
fn quantity_shown(quantity: f64, unit: Option<&Unit>) -> String {
    if unit.is_some_and(|unit| unit.fraction == Some(false)) {
        let scale = 10f64.powi(DECIMALS);
        let rounded = (quantity * scale).round() / scale;
        return if rounded.fract() == 0.0 {
            format!("{}", rounded as i64)
        } else {
            format!("{rounded}")
        };
    }
    let (mut top, bottom) = nearest_fraction(quantity);
    if bottom == 1 {
        return format!("{}{top}", if quantity < 0.0 { "-" } else { "" });
    }
    let fraction = |top: u128, bottom: u128| {
        let digits = |n: u128, set: &[char; 10]| {
            n.to_string()
                .chars()
                .map(|d| set[d.to_digit(10).unwrap_or(0) as usize])
                .collect::<String>()
        };
        const SUPER: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
        const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
        format!("{}/{}", digits(top, &SUPER), digits(bottom, &SUB))
    };
    if top <= bottom {
        return fraction(top, bottom);
    }
    let mut whole = 0;
    while top > bottom {
        whole += 1;
        top -= bottom;
    }
    format!("{whole} {}", fraction(top, bottom))
}

/// The fraction over at most [`MAX_DENOMINATOR`] nearest a positive
/// quantity, as Python's `Fraction(quantity).limit_denominator(32)` finds it
/// from the float's exact value.
fn nearest_fraction(quantity: f64) -> (u128, u128) {
    let quantity = quantity.abs();
    // The float's exact value is `mantissa / 2^shift`.
    let bits = quantity.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = if exponent == 0 {
        (bits & ((1 << 52) - 1)) << 1
    } else {
        (bits & ((1 << 52) - 1)) | (1 << 52)
    };
    let shift = 1075 - exponent.max(1);
    if shift <= 0 {
        return (quantity as u128, 1);
    }
    if shift > 120 {
        // Far below any quantity a cook writes: nearest to nothing.
        return (0, 1);
    }
    let (mut n, mut d) = (mantissa as u128, 1u128 << shift);
    let common = gcd(n, d);
    (n, d) = (n / common, d / common);
    if d <= MAX_DENOMINATOR {
        return (n, d);
    }
    let whole_d = d;
    let (mut p0, mut q0, mut p1, mut q1) = (0u128, 1u128, 1u128, 0u128);
    loop {
        let a = n / d;
        let q2 = q0 + a * q1;
        if q2 > MAX_DENOMINATOR {
            break;
        }
        (p0, q0, p1, q1) = (p1, q1, p0 + a * p1, q2);
        (n, d) = (d, n - a * d);
    }
    let k = (MAX_DENOMINATOR - q0) / q1;
    // Python compares the two candidates by distance with exact fractions:
    // the second is p1/q1 where 2·d·(q0 + k·q1) <= the original denominator.
    if 2 * d * (q0 + k * q1) <= whole_d {
        (p1, q1)
    } else {
        (p0 + k * p1, q0 + k * q1)
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// A unit as Mealie writes it: its abbreviation where it uses one, and its
/// plural past one.
fn unit_shown(quantity: f64, unit: &Unit) -> String {
    let plural = quantity > 1.0;
    let present = |text: &Option<String>| text.clone().filter(|text| !text.is_empty());
    let mut shown = None;
    if unit.use_abbreviation == Some(true) {
        shown = if plural {
            present(&unit.plural_abbreviation).or_else(|| present(&unit.abbreviation))
        } else {
            present(&unit.abbreviation)
        };
    }
    shown
        .or_else(|| {
            if plural {
                present(&unit.plural_name).or_else(|| present(&unit.name))
            } else {
                present(&unit.name)
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::other_apps::{Photo, Recipe, Unread};

    fn fixture(name: &str) -> Handed {
        Handed::file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/mealie")
                .join(name),
        )
    }

    fn read_all(handed: &Handed) -> (Vec<Result<Recipe, Unread>>, Vec<FileLeftOut>) {
        let export = MealieBackup
            .read(handed)
            .unwrap_or_else(|refusal| panic!("refused: {refusal:?}"))
            .expect("recognised");
        assert_eq!(export.app, "mealie");
        let total = export.total;
        let recipes: Vec<_> = export.recipes.collect();
        assert_eq!(recipes.len(), total);
        (recipes, export.left_out)
    }

    fn titled<'a>(recipes: &'a [Result<Recipe, Unread>], title: &str) -> &'a Recipe {
        recipes
            .iter()
            .flatten()
            .find(|recipe| recipe.candidate["title"] == title)
            .unwrap_or_else(|| panic!("no {title}"))
    }

    #[test]
    fn two_households_are_asked_about_before_anything_is_read() {
        let refused = MealieBackup.read(&fixture("v3-backup.zip")).err();
        assert_eq!(
            refused,
            Some(Refusal::WhichHousehold(vec![
                Household {
                    name: "Family".to_string(),
                    recipes: 3,
                    cooks: vec!["Sam Cook".to_string()],
                },
                Household {
                    name: "Cabin".to_string(),
                    recipes: 1,
                    cooks: vec!["Alex Baker".to_string()],
                },
            ]))
        );
        // A household the backup does not hold is asked about again.
        let refused = MealieBackup
            .read(&fixture("v3-backup.zip").in_household(Some("Garden")))
            .err();
        assert!(matches!(refused, Some(Refusal::WhichHousehold(_))));
    }

    #[test]
    fn a_household_s_recipes_read_as_its_export_does() {
        let (recipes, left_out) = read_all(&fixture("v3-backup.zip").in_household(Some("Family")));
        assert_eq!(recipes.len(), 3);
        assert!(matches!(left_out[..], [FileLeftOut::MealPlans(1)]));

        let export: Vec<_> = crate::mealie::Mealie
            .read(&fixture("v3-export.zip"))
            .unwrap_or_else(|_| panic!("refused"))
            .unwrap()
            .recipes
            .collect();
        for exported in export.iter().flatten() {
            let backed_up = recipes
                .iter()
                .flatten()
                .find(|recipe| recipe.foreign_id == exported.foreign_id)
                .unwrap_or_else(|| panic!("{} is not in the backup", exported.foreign_id));
            // Word for word, the pizza's last Step aside, which changed
            // between the export and the backup.
            assert_eq!(
                backed_up.candidate["ingredients"], exported.candidate["ingredients"],
                "{}",
                exported.candidate["title"]
            );
            assert_eq!(backed_up.candidate["note"], exported.candidate["note"]);
            assert_eq!(backed_up.candidate["yield"], exported.candidate["yield"]);
            assert_eq!(backed_up.tags, exported.tags);
            assert_eq!(backed_up.step_photos.len(), exported.step_photos.len());
            assert_eq!(
                matches!(backed_up.photo, Some(Photo::Bytes(_))),
                matches!(exported.photo, Some(Photo::Bytes(_)))
            );
        }
        let pizza = titled(&recipes, "Margherita pizza");
        assert_eq!(
            pizza.candidate["steps"][5]["text"],
            json!("Bake for 7 minutes, then add the basil and the oil.")
        );
        let [component] = pizza.components.as_slice() else {
            panic!("one Component");
        };
        assert_eq!(component.foreign_id, "dff05ead-5004-460b-8a51-caeb5d945b2a");
        assert!(component.copy.is_some());

        // Each cook's rating and favourite, both comments and the meal plan's
        // entry, if it had written one, stay in Mealie.
        assert!(
            pizza
                .left_out
                .iter()
                .any(|left| matches!(left, LeftOut::Ratings(1)))
        );
        assert!(
            pizza
                .left_out
                .iter()
                .any(|left| matches!(left, LeftOut::Favourites(1)))
        );
        assert!(
            pizza
                .left_out
                .iter()
                .any(|left| matches!(left, LeftOut::Comments(2)))
        );

        // Both cooks made it: Sam with a photo, Alex from the Cabin.
        let cookings: Vec<_> = pizza
            .cookings
            .iter()
            .map(|cooking| {
                (
                    cooking.note.as_str(),
                    cooking.cooked_at.as_str(),
                    cooking.photo.is_some(),
                )
            })
            .collect();
        assert_eq!(
            cookings,
            [
                (
                    "Cooked by Sam Cook in Mealie.\n\nCrisp base, needed more salt.",
                    "2026-10-09T19:30:00",
                    true
                ),
                (
                    "Cooked by Alex Baker in Mealie.\n\nMade it at the cabin.",
                    "2026-10-15T18:00:00",
                    false
                ),
            ]
        );
        assert_eq!(
            pizza.cookings[0].foreign_id,
            "283cc386-79ca-4f4f-8da8-2845932e3228"
        );
        // The recipe's creation is Mealie's bookkeeping, and no cooking.
        assert!(titled(&recipes, "Pizza dough").cookings.is_empty());
    }

    #[test]
    fn the_other_household_reads_alone() {
        let (recipes, left_out) = read_all(&fixture("v3-backup.zip").in_household(Some("Cabin")));
        assert!(left_out.is_empty());
        let [Ok(cake)] = recipes.as_slice() else {
            panic!("the cake alone");
        };
        assert_eq!(cake.candidate["title"], "Lemon drizzle cake");
        assert_eq!(
            cake.candidate["ingredients"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| line["text"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "225 g butter softened",
                "225 g caster sugar",
                "4 eggs",
                "2 lemons zest and juice"
            ]
        );
        assert_eq!(cake.cookings.len(), 1);
        assert!(cake.cookings[0].photo.is_some());
    }

    #[test]
    fn every_era_of_mealies_samples_is_read_or_refused_by_its_revision() {
        let sample = |name: &str| {
            Handed::file(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/mealie/anonymised")
                    .join(name),
            )
        };
        for (name, recipes) in [
            ("backup-version-44e8d670719d-1.zip", 349),
            ("backup-version-44e8d670719d-2.zip", 88),
            ("backup-version-44e8d670719d-3.zip", 69),
            ("backup-version-44e8d670719d-4.zip", 44),
            ("backup-version-bcfdad6b7355-1.zip", 46),
            ("backup-version-ba1e4a6cfe99-1.zip", 218),
            ("backup-version-09aba125b57a-1.zip", 42),
            ("backup-version-86054b40fd06-1.zip", 165),
            ("backup-version-1d9a002d7234-1.zip", 0),
        ] {
            let handed = sample(name);
            let total = match MealieBackup.read(&handed) {
                Ok(Some(export)) => {
                    let read: Vec<_> = export.recipes.collect();
                    assert!(read.iter().all(Result::is_ok), "{name}");
                    read.len()
                }
                Err(Refusal::WhichHousehold(households)) => {
                    households.iter().map(|household| household.recipes).sum()
                }
                other => panic!("{name}: {:?}", other.err()),
            };
            assert_eq!(total, recipes, "{name}");
        }
    }

    /// The real backup with its `database.json` changed by `change`, for the
    /// cases its own two households do not have.
    fn changed(change: impl FnOnce(&mut Value)) -> Handed {
        use std::io::{Read, Write};
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/mealie/v3-backup.zip"),
        )
        .unwrap();
        let mut original = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut zip);
            let mut change = Some(change);
            for index in 0..original.len() {
                let mut entry = original.by_index(index).unwrap();
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                if name == "database.json" {
                    let mut database: Value = serde_json::from_slice(&bytes).unwrap();
                    (change.take().unwrap())(&mut database);
                    bytes = serde_json::to_vec(&database).unwrap();
                }
                writer
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(&bytes).unwrap();
            }
            writer.finish().unwrap();
        }
        Handed::bytes(zip.into_inner())
    }

    #[test]
    fn a_household_name_two_groups_share_is_said_with_its_group() {
        let handed = changed(|database| {
            database["groups"].as_array_mut().unwrap().push(json!({
                "id": "0b9e2f4c5d6a4b7c8d9e0f1a2b3c4d5e", "name": "Weekend", "slug": "weekend",
            }));
            let cabin = &mut database["households"][1];
            cabin["name"] = json!("Family");
            cabin["group_id"] = json!("0b9e2f4c5d6a4b7c8d9e0f1a2b3c4d5e");
        });
        let Some(Refusal::WhichHousehold(households)) = MealieBackup.read(&handed).err() else {
            panic!("asked which");
        };
        let names: Vec<&str> = households.iter().map(|home| home.name.as_str()).collect();
        assert_eq!(names, ["Family (Home)", "Family (Weekend)"]);
        let (recipes, _) = read_all(&handed.in_household(Some("Family (Weekend)")));
        assert_eq!(recipes.len(), 1);
    }

    #[test]
    fn a_meal_plans_timeline_entry_is_counted_and_no_cooking() {
        let handed = changed(|database| {
            database["recipe_timeline_events"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "id": "7d1c0e9b2a3f4e5d8c7b6a5f4e3d2c1b",
                    "recipe_id": "c1acc761fb3d4735b53bb61059d5716a",
                    "user_id": "57760dc4df3c42f5be3328554572d5ca",
                    "subject": "Sam Cook made this for dinner",
                    "message": null, "event_type": "info", "image": "does not have image",
                    "timestamp": "2026-10-23T23:45:00",
                }));
        });
        let (recipes, _) = read_all(&handed.in_household(Some("Family")));
        let pizza = titled(&recipes, "Margherita pizza");
        assert!(
            pizza
                .left_out
                .iter()
                .any(|left| matches!(left, LeftOut::MealPlanTimeline(1)))
        );
        assert_eq!(pizza.cookings.len(), 2);
    }

    #[test]
    fn a_recipe_of_another_household_pointed_at_comes_with_its_photo() {
        let handed = changed(|database| {
            for line in database["recipes_ingredients"].as_array_mut().unwrap() {
                if !line["referenced_recipe_id"].is_null() {
                    line["referenced_recipe_id"] = json!("ed5f3c7f3f1c4b6c9aaeddba6b39a03b");
                }
            }
        });
        let (recipes, _) = read_all(&handed.in_household(Some("Family")));
        let pizza = titled(&recipes, "Margherita pizza");
        let copy = pizza.components[0].copy.as_ref().expect("the cake's copy");
        assert_eq!(copy.candidate["title"], "Lemon drizzle cake");
        assert!(matches!(&copy.photo, Some(Photo::Bytes(bytes)) if bytes.starts_with(b"RIFF")));
    }

    #[test]
    fn a_line_that_hid_its_amount_before_mealies_fix_reads_without_it() {
        let sample = Handed::file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/mealie/anonymised/backup-version-44e8d670719d-1.zip"),
        );
        // Its recipes are three groups', each read in turn.
        let Some(Refusal::WhichHousehold(households)) = MealieBackup.read(&sample).err() else {
            panic!("asked which");
        };
        let mut file = sample.open().unwrap();
        let mut bytes = Vec::new();
        {
            use std::io::Read;
            let mut archive = zip::ZipArchive::new(&mut file).unwrap();
            archive
                .by_name("database.json")
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
        }
        let database: Database = serde_json::from_slice(&bytes).unwrap();
        let hiding: std::collections::HashSet<String> = database
            .recipe_settings
            .iter()
            .filter(|settings| settings.disable_amount == Some(true))
            .map(|settings| settings.recipe_id.clone())
            .collect();
        let ones = |recipe: &other_apps::Recipe| {
            recipe.candidate["ingredients"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|line| line["text"].as_str().unwrap().starts_with("1 "))
                .count()
        };
        let (mut hidden, mut shown) = (0, 0);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mealie/anonymised/backup-version-44e8d670719d-1.zip");
        let recipes = households.iter().flat_map(|home| {
            let handed = Handed::file(path.clone()).in_household(Some(&home.name));
            let Ok(Some(export)) = MealieBackup.read(&handed) else {
                panic!("{} was not read", home.name);
            };
            export.recipes.flatten().collect::<Vec<_>>()
        });
        for recipe in recipes {
            if hiding.contains(&recipe.foreign_id) {
                hidden += ones(&recipe);
            } else {
                shown += ones(&recipe);
            }
        }
        assert_eq!(hidden, 0, "a recipe hiding its amounts shows a 1");
        assert!(shown > 0, "a recipe showing its amounts keeps its 1s");
    }

    #[test]
    fn a_revision_no_test_file_has_is_refused_with_the_update_reason() {
        let mut zip = std::io::Cursor::new(Vec::new());
        {
            use std::io::Write;
            let mut writer = zip::ZipWriter::new(&mut zip);
            writer
                .start_file("database.json", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer
                .write_all(
                    br#"{"alembic_version": [{"version_num": "feecc8ffb956"}], "recipes": []}"#,
                )
                .unwrap();
            writer.finish().unwrap();
        }
        let refused = MealieBackup.read(&Handed::bytes(zip.into_inner())).err();
        assert_eq!(refused, Some(Refusal::UntestedShape { app: "Mealie" }));
    }

    #[test]
    fn an_export_is_not_a_backup() {
        assert!(
            MealieBackup
                .read(&fixture("v3-export.zip"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_line_reads_as_mealie_writes_it() {
        let unit =
            |name: &str, plural: &str, abbreviation: &str, use_it: bool, fraction: bool| Unit {
                name: Some(name.to_string()),
                plural_name: Some(plural.to_string()),
                abbreviation: Some(abbreviation.to_string()),
                plural_abbreviation: Some(abbreviation.to_string()),
                use_abbreviation: Some(use_it),
                fraction: Some(fraction),
                ..Unit::default()
            };
        let food = |name: &str, plural: Option<&str>| Food {
            name: Some(name.to_string()),
            plural_name: plural.map(str::to_string),
            ..Food::default()
        };
        let gram = unit("gram", "grams", "g", true, false);
        let cup = unit("cup", "cups", "", false, true);
        let egg = food("egg", Some("eggs"));
        assert_eq!(
            display(
                Some(500.0),
                Some(&gram),
                Some(&food("flour", None)),
                Some("")
            ),
            "500 g flour"
        );
        assert_eq!(
            display(Some(1.5), Some(&cup), Some(&egg), None),
            "1 ¹/₂ cups egg"
        );
        assert_eq!(display(Some(4.0), None, Some(&egg), None), "4 eggs");
        assert_eq!(display(Some(1.0), None, Some(&egg), None), "1 egg");
        // No quantity: the food's plural, as Mealie's fallback words it.
        assert_eq!(
            display(None, Some(&cup), Some(&egg), Some("to glaze")),
            "eggs to glaze"
        );
        assert_eq!(display(Some(0.333), Some(&cup), None, None), "¹/₃ cup");
        assert_eq!(display(Some(0.3333), Some(&gram), None, None), "0.333 g");
        assert_eq!(display(Some(2.75), Some(&cup), None, None), "2 ³/₄ cups");
        assert_eq!(
            display(Some(0.0), None, None, Some("A few basil leaves")),
            "A few basil leaves"
        );
    }

    #[test]
    fn a_fraction_is_the_nearest_python_finds() {
        assert_eq!(nearest_fraction(0.5), (1, 2));
        assert_eq!(nearest_fraction(0.333), (1, 3));
        assert_eq!(nearest_fraction(0.1), (1, 10));
        assert_eq!(nearest_fraction(0.714), (5, 7));
        assert_eq!(nearest_fraction(0.01), (0, 1));
        assert_eq!(nearest_fraction(0.02), (1, 32));
        assert_eq!(nearest_fraction(7.0), (7, 1));
    }

    #[test]
    fn a_dashless_id_is_written_as_the_export_writes_it() {
        assert_eq!(
            uuid(&json!("C1ACC761FB3D4735B53BB61059D5716A")).as_deref(),
            Some("c1acc761-fb3d-4735-b53b-b61059d5716a")
        );
        assert_eq!(uuid(&json!(12)).as_deref(), Some("12"));
        assert_eq!(uuid(&json!(" ")), None);
    }
}
