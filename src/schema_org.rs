//! Reading recipe files in schema.org's Recipe format (#416, the spec is
//! #411), the format web pages carry, which several apps write as their
//! export: Nextcloud Cookbook, RecipeSage, Recipya, CookCLI and Tandoor's
//! "Recipe Sage" export. Bar Assistant's JSON-LD export is one too, and its
//! own reader ([`crate::bar_assistant`]) hands each recipe here.
//!
//! **This is the reader and nothing else** ([`crate::other_apps`]). Every rule
//! below comes from the files described in `tests/fixtures/schema-org/README.md`
//! and `tests/fixtures/tandoor/README.md`.
//!
//! - **The app is told by the file's shape**, and each lands under its own
//!   source kind. A zip of folders each holding a `recipe.json` is Nextcloud
//!   Cookbook's, or Recipya's where the recipe's dates are bare days, which
//!   only Recipya writes. A zip of loose `.json` files, or one alone, is
//!   CookCLI's. `{"recipes": […]}` is RecipeSage's, and a bare array of
//!   recipes with RecipeSage's fields and none of its ids is Tandoor's.
//! - **Every recipe is read by the web-link import's own mapping**
//!   ([`crate::web_import::from_recipe_node`]): its name, lines, steps and
//!   their Sections, Yield, times, photo, Source and calories. What a file
//!   holds beyond a page is read here alone, so a web link imports as it did:
//!   keywords, categories and cuisines become Tags, and the description and
//!   RecipeSage's own notes become the Note.
//! - **Nothing a cook typed is dropped** (#434). A total time no field holds,
//!   a time Kamosu cannot read, the equipment, the method, nutrition beyond
//!   the calories and a video each go into the Note as a line of their own,
//!   worded in the recipe's Language. A total that is only the prep and cook
//!   times added up, as Recipya and CookCLI write one, says nothing more and
//!   is left out. Ratings and reviews are counted on the Report.
//! - **A photo beside the recipe is its Main Photo**: Nextcloud Cookbook's
//!   `full.jpg`, never its thumbnails. Without one, the photo the recipe
//!   names is used: one written into the file as a `data:` address, as a
//!   self-hosted RecipeSage writes them, or one at a web address, downloaded.
//! - **RecipeSage writes a heading as a line in brackets**, `[Dough]`, and it
//!   becomes a Section. **Tandoor writes each line from its parts**, `225.0 g
//!   butter`, with Python's `None` for a part it lacks, and each is written
//!   again as Tandoor's own export reads ([`crate::tandoor`]). Its times are
//!   bare minutes, its waiting time the total less the working time.
//! - **The recipe is known by the app's own id**: RecipeSage's, or the number
//!   of the recipe's file in Nextcloud. Where it has none, it is known by its
//!   link, and by its name where it has neither, so a recipe with neither that
//!   was renamed between two imports arrives twice. Two recipes of one file
//!   known by the same, two from one page, are told apart by their names.
//! - **Each recipe is read twice from a zip**, once for what it is known by
//!   and once as it lands, so a zip of thousands holds one at a time.

use std::collections::HashMap;

use base64::Engine;
use serde_json::{Value, json};

use crate::entities::decode_entities;
use crate::other_apps::{self, Budget, Handed, LeftOut, Reader, Refusal};

/// Recipe files among the readers of another app's export.
pub struct SchemaOrg;

/// The largest recipe file read whole, a RecipeSage export with its photos
/// written into it. A self-hosted RecipeSage keeps a photo at some fifteen
/// kilobytes, so this is thousands of recipes. A file is held whole while it
/// lands, and parsed it takes several times its size, so this is no larger.
const MAX_JSON_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// The largest single file read out of a zip, a recipe or a picture. The
/// largest real one is a 117 KB `full.jpg`, so this refuses only a file that
/// claims to be enormous.
const MAX_ENTRY_BYTES: u64 = 128 * 1024 * 1024;

type Archive = zip::ZipArchive<Box<dyn other_apps::ReadSeek>>;

/// Which app wrote a recipe file, which is the source kind its Import is kept
/// under and decides the little each app does its own way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum App {
    NextcloudCookbook,
    RecipeSage,
    Recipya,
    CookCli,
    BarAssistant,
    Tandoor,
}

impl App {
    /// The source kind.
    pub(crate) fn kind(self) -> &'static str {
        match self {
            App::NextcloudCookbook => "nextcloud_cookbook",
            App::RecipeSage => "recipesage",
            App::Recipya => "recipya",
            App::CookCli => "cookcli",
            App::BarAssistant => "bar_assistant",
            App::Tandoor => "tandoor",
        }
    }
}

impl Reader for SchemaOrg {
    fn names(&self) -> &'static [&'static str] {
        &["Nextcloud Cookbook", "RecipeSage", "Recipya", "CookCLI"]
    }

    fn read(&self, handed: &Handed) -> Result<Option<other_apps::Export>, Refusal> {
        Ok(if handed.is_zip() {
            read_zip(handed)
        } else {
            read_file(handed)
        })
    }
}

/// One recipe the file offers, as far as it was read before landing.
struct Found {
    held: Held,
    /// What the app knows it by, before a second recipe known by the same is
    /// told apart from the first.
    id: String,
    /// Its own name, which tells two known by the same apart.
    title: Option<String>,
    /// What it is called where it has no name of its own or could not be
    /// read: its folder or file.
    name: Option<String>,
    /// The pictures beside it in its folder, in the zip, the one to keep first.
    pictures: Vec<usize>,
}

/// Where a recipe is until it lands.
enum Held {
    /// Read already, from a file handed over alone.
    Node(Value),
    /// A file in the zip, read again when the recipe is pulled, so a zip of
    /// thousands holds one recipe at a time.
    InZip(usize),
    /// Why it could not be read.
    Unread(String),
}

/// A JSON file handed over alone: RecipeSage's export, Tandoor's, or one
/// CookCLI recipe.
fn read_file(handed: &Handed) -> Option<other_apps::Export> {
    let value = handed.json(MAX_JSON_FILE_BYTES)?;
    let (app, items) = match value {
        Value::Object(mut map) if !is_recipe(&map) => {
            let Some(Value::Array(recipes)) = map.remove("recipes") else {
                return None;
            };
            (App::RecipeSage, recipes)
        }
        Value::Object(map) => (App::CookCli, vec![Value::Object(map)]),
        Value::Array(items) if items.iter().any(is_tandoors) => (App::Tandoor, items),
        _ => return None,
    };
    let found: Vec<Found> = items
        .into_iter()
        .map(|item| match crate::web_import::search_for_recipe(&item) {
            Some(node) => Found {
                id: own_id(app, &node),
                title: text(&node, "name"),
                held: Held::Node(node),
                name: None,
                pictures: Vec::new(),
            },
            None => Found {
                id: String::new(),
                title: None,
                held: Held::Unread(not_a_recipe()),
                name: None,
                pictures: Vec::new(),
            },
        })
        .collect();
    if !found
        .iter()
        .any(|found| matches!(found.held, Held::Node(_)))
    {
        return None;
    }
    Some(export(
        app,
        found,
        None,
        Budget::new(other_apps::MOST_UNPACKED),
    ))
}

/// Whether an object is a Recipe node itself.
fn is_recipe(map: &serde_json::Map<String, Value>) -> bool {
    map.get("@type").is_some_and(|kind| match kind {
        Value::String(kind) => kind.eq_ignore_ascii_case("recipe"),
        Value::Array(kinds) => kinds.iter().any(|kind| {
            kind.as_str()
                .is_some_and(|k| k.eq_ignore_ascii_case("recipe"))
        }),
        _ => false,
    })
}

/// Whether a recipe is one Tandoor's "Recipe Sage" export wrote: RecipeSage's
/// own fields, and neither the id nor the date RecipeSage gives every recipe.
fn is_tandoors(recipe: &Value) -> bool {
    recipe.get("creditText").is_some()
        && recipe.get("isBasedOn").is_some()
        && recipe.get("comment").is_some()
        && recipe.get("identifier").is_none()
        && recipe.get("datePublished").is_none()
}

fn not_a_recipe() -> String {
    "the file holds no recipe Kamosu can read: it may be damaged, so export it again \
     and import that"
        .to_string()
}

/// A zip: Nextcloud Cookbook's folders, Recipya's, or CookCLI's files. Each
/// recipe is read once here, for what it is known by, and again when it
/// lands, so no more than one is held at a time.
fn read_zip(handed: &Handed) -> Option<other_apps::Export> {
    let mut archive = Archive::new(handed.open().ok()?).ok()?;
    let paths: Vec<(usize, String)> = (0..archive.len())
        .filter_map(|index| {
            let path = archive.name_for_index(index)?;
            let file = path.rsplit('/').next().unwrap_or(path);
            (!path.starts_with("__MACOSX/") && !file.starts_with("._") && !file.is_empty())
                .then(|| (index, path.to_string()))
        })
        .collect();
    let jsons: Vec<(usize, &str, &str)> = paths
        .iter()
        .filter(|(_, path)| path.to_ascii_lowercase().ends_with(".json"))
        .map(|(index, path)| {
            let (folder, file) = path.rsplit_once('/').unwrap_or(("", path));
            (*index, folder, file)
        })
        .collect();
    // A Bar Assistant export is its own reader's, whichever format it is in.
    if jsons
        .iter()
        .any(|(_, folder, file)| folder.is_empty() && *file == "_meta.json")
    {
        return None;
    }
    // Folders each holding a `recipe.json` are the recipes, and any other
    // JSON beside them is somebody else's. Without them, every JSON is.
    let in_folders = jsons
        .iter()
        .any(|(_, folder, file)| !folder.is_empty() && file.eq_ignore_ascii_case("recipe.json"));
    let jsons: Vec<(usize, &str, &str)> = jsons
        .into_iter()
        .filter(|(_, folder, file)| {
            !in_folders || (!folder.is_empty() && file.eq_ignore_ascii_case("recipe.json"))
        })
        .collect();

    let mut budget = Budget::new(other_apps::MOST_UNPACKED);
    let mut app = None;
    let mut found = Vec::new();
    for (index, folder, file) in jsons {
        let name = match folder.rsplit('/').next().filter(|_| in_folders) {
            Some(folder) => folder.to_string(),
            None => file[..file.len() - ".json".len()].to_string(),
        };
        // Loose files are a cook's own zip of CookCLI's, so no picture in it
        // is known to be any one recipe's.
        let pictures = if in_folders {
            pictures_in(&paths, folder)
        } else {
            Vec::new()
        };
        let (held, id, title) = match node_at(&mut archive, index, &mut budget) {
            Ok(node) => {
                let app = *app.get_or_insert(if !in_folders {
                    App::CookCli
                } else if is_recipyas(&node) {
                    App::Recipya
                } else {
                    App::NextcloudCookbook
                });
                (Held::InZip(index), own_id(app, &node), text(&node, "name"))
            }
            // A JSON that is no recipe in a zip of loose files is somebody
            // else's, and one that is no JSON at all a recipe lost.
            Err(NotRead::NotARecipe) if !in_folders => continue,
            Err(NotRead::NotARecipe) => (Held::Unread(not_a_recipe()), String::new(), None),
            Err(NotRead::Damaged(reason)) => (Held::Unread(reason), String::new(), None),
        };
        found.push(Found {
            held,
            id,
            title,
            name: Some(name),
            pictures,
        });
    }
    let app = app?;
    Some(export(
        app,
        found,
        Some(archive),
        Budget::new(other_apps::MOST_UNPACKED),
    ))
}

/// Why a file in the zip gave no recipe.
enum NotRead {
    /// It is JSON, and holds no Recipe.
    NotARecipe,
    /// It could not be unpacked or is no JSON, in the Report's words.
    Damaged(String),
}

/// The Recipe node in one file of the zip.
fn node_at(archive: &mut Archive, index: usize, budget: &mut Budget) -> Result<Value, NotRead> {
    let bytes = unpack(archive, index, budget).map_err(NotRead::Damaged)?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| NotRead::Damaged(not_a_recipe()))?;
    crate::web_import::search_for_recipe(&value).ok_or(NotRead::NotARecipe)
}

/// Whether a recipe is one Recipya wrote: its dates are bare days, which
/// Nextcloud Cookbook never writes.
fn is_recipyas(recipe: &Value) -> bool {
    let bare_day = |text: &str| {
        text.len() == 10
            && text.bytes().enumerate().all(|(at, byte)| {
                if at == 4 || at == 7 {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            })
    };
    recipe["@context"] == "https://schema.org"
        && recipe["dateCreated"].as_str().is_some_and(bare_day)
}

/// The pictures in `folder`, the one to keep first: Nextcloud Cookbook's
/// `full.jpg` before any other, and never its thumbnails, which are smaller
/// copies of it.
pub(crate) fn pictures_in(paths: &[(usize, String)], folder: &str) -> Vec<usize> {
    let mut pictures: Vec<(bool, &str, usize)> = paths
        .iter()
        .filter_map(|(index, path)| {
            let (parent, file) = path.rsplit_once('/')?;
            let lower = file.to_ascii_lowercase();
            let (stem, extension) = lower.rsplit_once('.')?;
            (parent == folder
                && matches!(extension, "jpg" | "jpeg" | "png" | "webp" | "gif")
                && !stem.starts_with("thumb"))
            .then_some((stem != "full", file, *index))
        })
        .collect();
    pictures.sort();
    pictures.into_iter().map(|(_, _, index)| index).collect()
}

/// The pictures at `indices` unpacked, and how many could not be.
pub(crate) fn unpack_pictures(
    archive: &mut Archive,
    indices: Vec<usize>,
    budget: &mut Budget,
) -> (Vec<Vec<u8>>, usize) {
    let mut pictures = Vec::new();
    let mut unreadable = 0;
    for index in indices {
        match unpack(archive, index, budget) {
            Ok(bytes) => pictures.push(bytes),
            Err(_) => unreadable += 1,
        }
    }
    (pictures, unreadable)
}

/// Unpack one file of the zip through the export's budget. The error is a
/// sentence for the Report.
pub(crate) fn unpack(
    archive: &mut Archive,
    index: usize,
    budget: &mut Budget,
) -> Result<Vec<u8>, String> {
    let damaged = || {
        "the file is damaged and this recipe could not be read; export the recipes \
         again and import that"
            .to_string()
    };
    let mut entry = archive.by_index(index).map_err(|_| damaged())?;
    match budget.unpack(&mut entry, MAX_ENTRY_BYTES) {
        Err(other_apps::TooMuch) => Err(format!(
            "the export unpacks to more than {} GB in all, so this recipe and those \
             after it were not read",
            budget.most() / (1000 * 1000 * 1000)
        )),
        Ok((bytes, _)) if bytes.len() as u64 > MAX_ENTRY_BYTES => {
            Err("a file in it is too large to be one recipe".to_string())
        }
        Ok((_, false)) => Err(damaged()),
        Ok((bytes, true)) => Ok(bytes),
    }
}

/// The export: each recipe read as it is pulled, its pictures unpacked then.
fn export(
    app: App,
    found: Vec<Found>,
    mut archive: Option<Archive>,
    mut budget: Budget,
) -> other_apps::Export {
    let ids = unique_ids(&found);
    let total = found.len();
    let recipes = found.into_iter().zip(ids).map(move |(found, foreign_id)| {
        let unread = |reason: String| other_apps::Unread {
            foreign_id: None,
            name: found.name.clone(),
            reason,
        };
        let node = match (found.held, archive.as_mut()) {
            (Held::Node(node), _) => node,
            (Held::InZip(index), Some(archive)) => match node_at(archive, index, &mut budget) {
                Ok(node) => node,
                Err(NotRead::Damaged(reason)) => return Err(unread(reason)),
                Err(NotRead::NotARecipe) => return Err(unread(not_a_recipe())),
            },
            (Held::InZip(_), None) => return Err(unread(not_a_recipe())),
            (Held::Unread(reason), _) => return Err(unread(reason)),
        };
        let (pictures, unreadable) = match archive.as_mut() {
            Some(archive) => unpack_pictures(archive, found.pictures, &mut budget),
            None => (Vec::new(), 0),
        };
        Ok(recipe(&node, app, foreign_id, pictures, unreadable))
    });
    other_apps::Export {
        app: app.kind(),
        total,
        recipes: Box::new(recipes),
        left_out: Vec::new(),
    }
}

/// What each recipe is known by. Two recipes of one file known by the same,
/// such as two from one page, are told apart by their names, and numbered
/// where their names are the same too, never as any other recipe of the file
/// is already known.
fn unique_ids(found: &[Found]) -> Vec<String> {
    let base: Vec<String> = found
        .iter()
        .map(|found| match found.id.trim() {
            "" => found
                .name
                .clone()
                .unwrap_or_else(|| "Untitled recipe".to_string()),
            id => id.to_string(),
        })
        .collect();
    let mut times: HashMap<&str, usize> = HashMap::new();
    for id in &base {
        *times.entry(id).or_default() += 1;
    }
    let told: Vec<String> = found
        .iter()
        .zip(&base)
        .map(|(found, id)| match &found.title {
            Some(name) if times[id.as_str()] > 1 && name != id => format!("{id} {name}"),
            _ => id.clone(),
        })
        .collect();
    other_apps::numbered(told)
}

/// The app's own id for a recipe, its link, or its name.
fn own_id(app: App, node: &Value) -> String {
    let id = match app {
        App::RecipeSage => text(node, "identifier"),
        // Nextcloud Cookbook numbers a recipe by its file in Nextcloud, and
        // writes 0 for one it has not numbered.
        App::NextcloudCookbook => match &node["id"] {
            Value::Number(id) if id.as_f64() != Some(0.0) => Some(id.to_string()),
            Value::String(id) if !matches!(id.trim(), "" | "0") => Some(id.trim().to_string()),
            _ => None,
        },
        _ => None,
    };
    id.or_else(|| link_of(node))
        .or_else(|| text(node, "name"))
        .unwrap_or_default()
}

/// Whether `link` is an address on the web.
pub(crate) fn is_web_address(link: &str) -> bool {
    link.starts_with("http://") || link.starts_with("https://")
}

/// The recipe's own web page, where the file gives one.
fn link_of(node: &Value) -> Option<String> {
    ["url", "isBasedOn"]
        .into_iter()
        .filter_map(|field| text(node, field))
        .find(|link| is_web_address(link))
}

/// One recipe as Kamosu's content, with its photo: `pictures` are the files
/// beside it in the export, the one to keep first, and `unreadable` how many
/// more could not be unpacked.
pub(crate) fn recipe(
    node: &Value,
    app: App,
    foreign_id: String,
    mut pictures: Vec<Vec<u8>>,
    unreadable: usize,
) -> other_apps::Recipe {
    let link = link_of(node);
    let hostname = link
        .as_deref()
        .and_then(other_apps::host)
        .unwrap_or_default();
    let parsed = crate::web_import::from_recipe_node(node, &hostname);
    let mut left_out = Vec::new();

    let photo = if pictures.is_empty() {
        photo_named(node, parsed.image_url.as_deref(), &mut left_out)
    } else {
        let first = pictures.remove(0);
        if !pictures.is_empty() {
            left_out.push(LeftOut::ExtraPhotos(pictures.len()));
        }
        Some(other_apps::Photo::Bytes(first))
    };
    if unreadable > 0 {
        left_out.push(LeftOut::UnreadablePhotos(unreadable));
    }
    let ratings = node
        .get("aggregateRating")
        .is_some_and(|rating| !is_empty(rating));
    if ratings {
        left_out.push(LeftOut::Ratings(1));
    }
    let reviews = count(&node["review"]) + count(&node["reviews"]) + others_comments(node);
    if reviews > 0 {
        left_out.push(LeftOut::Comments(reviews));
    }

    other_apps::Recipe {
        foreign_id,
        cookings: Vec::new(),
        candidate: content(node, app, &parsed, link),
        tags: tags(node),
        photo,
        step_photos: Vec::new(),
        components: Vec::new(),
        left_out,
    }
}

/// The photo the recipe names, where no file beside it is one: written into
/// the file as a `data:` address, or at a web address to download. A photo
/// named in a way that is neither is the app's, and not in its export.
fn photo_named(
    node: &Value,
    first: Option<&str>,
    left_out: &mut Vec<LeftOut>,
) -> Option<other_apps::Photo> {
    let named = images(&node["image"]);
    if named.len() > 1 {
        left_out.push(LeftOut::ExtraPhotos(named.len() - 1));
    }
    let first = first?;
    if let Some(data) = first.strip_prefix("data:") {
        let bytes = data
            .split_once(',')
            .filter(|(header, _)| header.ends_with(";base64"))
            .and_then(|(_, encoded)| {
                base64::engine::general_purpose::STANDARD
                    .decode(encoded.trim())
                    .ok()
            });
        return match bytes {
            Some(bytes) => Some(other_apps::Photo::Bytes(bytes)),
            None => {
                left_out.push(LeftOut::UnreadablePhotos(1));
                None
            }
        };
    }
    if is_web_address(first) {
        Some(other_apps::Photo::Address(first.to_string()))
    } else {
        Some(other_apps::Photo::NotInExport)
    }
}

/// Every photo a recipe names, however it names them.
fn images(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => vec![text.trim().to_string()],
        Value::Array(items) => items.iter().flat_map(images).collect(),
        Value::Object(_) => ["url", "contentUrl"]
            .into_iter()
            .find_map(|field| text(value, field))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

/// The words a recipe's Language puts around what the file kept, which the
/// KitchenOwl, Mealie and Paprika readers word their own with too.
pub(crate) struct Words {
    pub(crate) servings: &'static str,
    pub(crate) prep: &'static str,
    pub(crate) cook: &'static str,
    pub(crate) total: &'static str,
    /// What leads servings no Yield says whole, and a difficulty, in the
    /// Note (#434).
    pub(crate) servings_label: &'static str,
    pub(crate) difficulty: &'static str,
    equipment: &'static str,
    method: &'static str,
    pub(crate) nutrition: &'static str,
    video: &'static str,
    photo: &'static str,
    hour: (&'static str, &'static str),
    minute: (&'static str, &'static str),
    /// schema.org's name for each figure of its nutrition, as this Language
    /// says it, in the order they are written.
    pub(crate) figures: [(&'static str, &'static str); 13],
}

pub(crate) fn words(language: Option<&str>) -> Words {
    match language {
        Some("fr") => Words {
            servings: "portions",
            prep: "Temps de préparation :",
            cook: "Temps de cuisson :",
            total: "Temps total :",
            servings_label: "Portions :",
            difficulty: "Difficulté :",
            equipment: "Matériel :",
            method: "Méthode :",
            nutrition: "Valeurs nutritionnelles :",
            video: "Vidéo :",
            photo: "Photo :",
            hour: ("heure", "heures"),
            minute: ("minute", "minutes"),
            figures: [
                ("servingSize", "portion"),
                ("calories", "calories"),
                ("fatContent", "matières grasses"),
                ("saturatedFatContent", "acides gras saturés"),
                ("transFatContent", "acides gras trans"),
                ("unsaturatedFatContent", "acides gras insaturés"),
                ("cholesterolContent", "cholestérol"),
                ("sodiumContent", "sodium"),
                ("carbohydrateContent", "glucides"),
                ("fiberContent", "fibres"),
                ("sugarContent", "sucres"),
                ("proteinContent", "protéines"),
                ("description", ""),
            ],
        },
        Some("es") => Words {
            servings: "raciones",
            prep: "Tiempo de preparación:",
            cook: "Tiempo de cocción:",
            total: "Tiempo total:",
            servings_label: "Raciones:",
            difficulty: "Dificultad:",
            equipment: "Utensilios:",
            method: "Método:",
            nutrition: "Nutrición:",
            video: "Vídeo:",
            photo: "Foto:",
            hour: ("hora", "horas"),
            minute: ("minuto", "minutos"),
            figures: [
                ("servingSize", "ración"),
                ("calories", "calorías"),
                ("fatContent", "grasas"),
                ("saturatedFatContent", "grasas saturadas"),
                ("transFatContent", "grasas trans"),
                ("unsaturatedFatContent", "grasas insaturadas"),
                ("cholesterolContent", "colesterol"),
                ("sodiumContent", "sodio"),
                ("carbohydrateContent", "carbohidratos"),
                ("fiberContent", "fibra"),
                ("sugarContent", "azúcares"),
                ("proteinContent", "proteínas"),
                ("description", ""),
            ],
        },
        _ => Words {
            servings: "servings",
            prep: "Prep time:",
            cook: "Cook time:",
            total: "Total time:",
            servings_label: "Servings:",
            difficulty: "Difficulty:",
            equipment: "Equipment:",
            method: "Method:",
            nutrition: "Nutrition:",
            video: "Video:",
            photo: "Photo:",
            hour: ("hour", "hours"),
            minute: ("minute", "minutes"),
            figures: [
                ("servingSize", "serving size"),
                ("calories", "calories"),
                ("fatContent", "fat"),
                ("saturatedFatContent", "saturated fat"),
                ("transFatContent", "trans fat"),
                ("unsaturatedFatContent", "unsaturated fat"),
                ("cholesterolContent", "cholesterol"),
                ("sodiumContent", "sodium"),
                ("carbohydrateContent", "carbohydrates"),
                ("fiberContent", "fibre"),
                ("sugarContent", "sugar"),
                ("proteinContent", "protein"),
                ("description", ""),
            ],
        },
    }
}

/// A time as the file gives it.
#[derive(Debug, PartialEq)]
enum Time {
    None,
    Minutes(i64),
    /// Written in a way Kamosu cannot read, kept as written.
    Unread(String),
}

impl Time {
    fn minutes(&self) -> Option<i64> {
        match self {
            Time::Minutes(minutes) => Some(*minutes),
            _ => None,
        }
    }
}

/// A time field: an ISO 8601 duration, read by the web mapping where it set
/// `read`; bare minutes, which Tandoor writes; or words a Step's timer reads.
/// A time of nothing is none.
fn time(node: &Value, field: &str, read: Option<i64>, language: &str) -> Time {
    if let Some(minutes) = read {
        return if minutes > 0 {
            Time::Minutes(minutes)
        } else {
            Time::None
        };
    }
    let written = match &node[field] {
        Value::Number(minutes) => minutes.to_string(),
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => match items.iter().find_map(Value::as_str) {
            Some(text) => text.trim().to_string(),
            None => return Time::None,
        },
        _ => return Time::None,
    };
    if written.is_empty() {
        return Time::None;
    }
    if let Ok(minutes) = written.parse::<f64>() {
        return if minutes >= 1.0 {
            Time::Minutes(minutes.round() as i64)
        } else {
            Time::None
        };
    }
    if let Ok(duration) = written.parse::<iso8601_duration::Duration>() {
        return match duration.to_std() {
            Some(duration) if duration.as_secs() >= 60 => {
                Time::Minutes((duration.as_secs() / 60) as i64)
            }
            Some(_) => Time::None,
            None => Time::Unread(written),
        };
    }
    match crate::units::step_duration(&written, language) {
        Some(seconds) if seconds >= 60 => Time::Minutes((seconds as f64 / 60.0).round() as i64),
        _ => Time::Unread(written),
    }
}

/// Minutes as a cook says them: `1 hour 10 minutes`.
pub(crate) fn spoken(minutes: i64, words: &Words) -> String {
    let worded = |count: i64, (one, many): (&str, &str)| {
        format!("{count} {}", if count == 1 { one } else { many })
    };
    let (hours, minutes) = (minutes / 60, minutes % 60);
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(worded(hours, words.hour));
    }
    if minutes > 0 {
        parts.push(worded(minutes, words.minute));
    }
    parts.join(" ")
}

/// The recipe content.
fn content(
    node: &Value,
    app: App,
    parsed: &crate::web_import::ParsedRecipe,
    link: Option<String>,
) -> Value {
    let mut ingredients = Vec::new();
    // A line, like any text here, can be blank once its HTML entities are
    // undone, `&nbsp;`, and is then no line.
    for line in parsed
        .ingredients
        .iter()
        .filter(|line| !line.trim().is_empty())
    {
        let heading = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
            .filter(|_| app == App::RecipeSage);
        match heading {
            Some(heading) if !heading.trim().is_empty() => {
                ingredients.push(json!({ "kind": "section", "text": heading.trim() }));
            }
            Some(_) => {}
            None if app == App::Tandoor => {
                if let Some(line) = tandoor_line(line) {
                    ingredients.push(json!({ "kind": "ingredient", "text": line }));
                }
            }
            None => ingredients.push(json!({ "kind": "ingredient", "text": line })),
        }
    }
    // A step with nothing written in it is no step: Nextcloud Cookbook keeps
    // the empty box a cook left.
    let steps: Vec<Value> = parsed
        .steps
        .iter()
        .filter(|step| match step {
            crate::web_import::StepEntry::Section(text)
            | crate::web_import::StepEntry::Step(text) => !text.trim().is_empty(),
        })
        .map(|step| match step {
            crate::web_import::StepEntry::Section(text) => {
                json!({ "kind": "section", "text": text, "photo": null })
            }
            crate::web_import::StepEntry::Step(text) => {
                json!({ "kind": "step", "text": text, "photo": null })
            }
        })
        .collect();
    let decoded = |field: &str| {
        text(node, field)
            .map(|text| decode_entities(&text).trim().to_string())
            .filter(|text| !text.is_empty())
    };
    let title = decoded("name").unwrap_or_else(|| "Untitled recipe".to_string());
    let description = decoded("description");

    // The recipe's own Language words what is added to it, so it is read from
    // what the cook wrote first, none of those words in it.
    let language = crate::language::detect_content(&json!({
        "title": title,
        "note": description,
        "ingredients": ingredients,
        "steps": steps,
    }));
    let words = words(language);

    let written_in = language.unwrap_or(crate::language::UNKNOWN);
    let prep = time(node, "prepTime", parsed.prep_time_minutes, written_in);
    let mut cook = time(node, "cookTime", parsed.cook_time_minutes, written_in);
    let total = time(node, "totalTime", None, written_in);
    // Tandoor writes its working time and the two added up, and the waiting
    // time is what lies between.
    if app == App::Tandoor
        && cook == Time::None
        && let (Some(prep), Some(total)) = (prep.minutes(), total.minutes())
        && total > prep
    {
        cook = Time::Minutes(total - prep);
    }

    let mut paragraphs: Vec<String> = description.into_iter().collect();
    paragraphs.extend(authors_notes(node));
    for (written, lead) in [(&prep, words.prep), (&cook, words.cook)] {
        if let Time::Unread(written) = written {
            paragraphs.push(format!("{lead} {written}"));
        }
    }
    let landed = prep.minutes().unwrap_or(0) + cook.minutes().unwrap_or(0);
    match &total {
        Time::Minutes(total) if *total != landed => {
            paragraphs.push(format!("{} {}", words.total, spoken(*total, &words)));
        }
        Time::Unread(written) => paragraphs.push(format!("{} {written}", words.total)),
        _ => {}
    }
    let tools = named_in(&node["tool"]);
    if !tools.is_empty() {
        paragraphs.push(format!("{} {}.", words.equipment, tools.join(", ")));
    }
    let methods = named_in(&node["cookingMethod"]);
    if !methods.is_empty() {
        paragraphs.push(format!("{} {}", words.method, methods.join(", ")));
    }
    let recipe_yield = recipe_yield(node, parsed, &words);
    if let Some(line) = nutrition_line(node, parsed, &recipe_yield, &words) {
        paragraphs.push(line);
    }
    let videos = videos(&node["video"]);
    if !videos.is_empty() {
        paragraphs.push(format!("{} {}", words.video, videos.join(", ")));
    }
    if let Some(credit) = photo_credit(&node["image"]) {
        paragraphs.push(format!("{} {credit}", words.photo));
    }

    json!({
        "title": title,
        "yield": recipe_yield,
        "prep_time_minutes": prep.minutes(),
        "cook_time_minutes": cook.minutes(),
        "source": source(node, parsed, link),
        "note": if paragraphs.is_empty() { Value::Null } else { json!(paragraphs.join("\n\n")) },
        "nutrition": parsed.nutrition_calories
            .map(|calories| json!({ "calories": calories, "basis": "per_serving" })),
        "ingredients": ingredients,
        "steps": steps,
    })
}

/// A line as Tandoor's schema.org export writes it, `225.0 g butter`, written
/// again as Tandoor's own export reads ([`crate::tandoor`]): the amount as a
/// cook writes it, none where it is zero, and Python's `None` for a missing
/// unit or food left out. `0.0 None None` is a header row, whose name this
/// export does not keep, and is no line.
fn tandoor_line(line: &str) -> Option<String> {
    let Some((amount, rest)) = line.split_once(' ') else {
        return Some(line.to_string());
    };
    let Ok(amount) = amount.parse::<f64>() else {
        return Some(line.to_string());
    };
    let (unit, food) = rest.split_once(' ').unwrap_or((rest, ""));
    let unit = Some(unit).filter(|unit| *unit != "None" && !unit.is_empty());
    let food = Some(food).filter(|food| *food != "None" && !food.is_empty());
    if unit.is_none() && food.is_none() {
        return None;
    }
    let mut parts = Vec::new();
    if amount.is_finite() && amount > 0.0 {
        parts.push(crate::units::amount_for(amount, unit));
    }
    parts.extend(unit.map(str::to_string));
    parts.extend(food.map(str::to_string));
    Some(parts.join(" "))
}

/// The Yield the web mapping read, its noun in the recipe's Language where
/// the file gave a bare number, and none where the number is nothing.
fn recipe_yield(node: &Value, parsed: &crate::web_import::ParsedRecipe, words: &Words) -> Value {
    let Some((amount, noun)) = &parsed.yield_amount_noun else {
        return Value::Null;
    };
    if amount
        .trim()
        .parse::<f64>()
        .is_ok_and(|amount| amount <= 0.0)
    {
        return Value::Null;
    }
    let written = match &node["recipeYield"] {
        Value::Array(items) => items.first().cloned().unwrap_or(Value::Null),
        written => written.clone(),
    };
    let bare = match &written {
        Value::Number(_) => true,
        Value::String(text) => text.trim().parse::<f64>().is_ok(),
        _ => false,
    };
    let noun = if bare { words.servings } else { noun };
    json!({ "amount": amount, "noun": noun })
}

/// The nutrition the Nutrition field does not hold, as one line: each figure
/// named as schema.org names it, in the recipe's Language, as the file wrote
/// it. The calories are left out where they landed in the field, and so is a
/// serving size that is only the Yield's number again, which Recipya writes.
fn nutrition_line(
    node: &Value,
    parsed: &crate::web_import::ParsedRecipe,
    recipe_yield: &Value,
    words: &Words,
) -> Option<String> {
    let nutrition = match &node["nutrition"] {
        Value::Array(items) => items.first()?,
        nutrition => nutrition,
    };
    let as_written = |value: &Value| match value {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    };
    let Value::Object(fields) = nutrition else {
        return as_written(nutrition).map(|text| format!("{} {text}", words.nutrition));
    };
    let mut figures = Vec::new();
    let mut named = |field: &str, name: &str| {
        let Some(figure) = fields.get(field).and_then(as_written) else {
            return;
        };
        if field == "calories" && parsed.nutrition_calories.is_some() {
            return;
        }
        if field == "servingSize" && recipe_yield["amount"].as_str() == Some(figure.as_str()) {
            return;
        }
        figures.push(if name.is_empty() {
            figure
        } else {
            format!("{name} {figure}")
        });
    };
    for (field, name) in words.figures {
        named(field, name);
    }
    // A figure schema.org does not name is written by the app's own name for
    // it, so nothing is dropped.
    for field in fields.keys() {
        if !field.starts_with('@') && !words.figures.iter().any(|(known, _)| known == field) {
            named(field, field);
        }
    }
    (!figures.is_empty()).then(|| format!("{} {}", words.nutrition, figures.join(", ")))
}

/// The Source: the credit RecipeSage writes, the author or the publisher, or
/// the site, and the recipe's own page.
fn source(node: &Value, parsed: &crate::web_import::ParsedRecipe, link: Option<String>) -> Value {
    let credit = text(node, "creditText")
        .or_else(|| Some(parsed.source_text.trim().to_string()).filter(|text| !text.is_empty()));
    let text = credit.or_else(|| link.as_deref().and_then(other_apps::host));
    match text {
        Some(text) => json!({ "text": text, "link": link }),
        None => Value::Null,
    }
}

/// The notes RecipeSage keeps as a comment by the recipe's author, which
/// Tandoor's export writes too.
fn authors_notes(node: &Value) -> Vec<String> {
    comments(node)
        .filter(|(name, _)| name.as_deref() == Some("Author Notes"))
        .map(|(_, text)| text)
        .collect()
}

/// Comments other people wrote, which stay behind.
fn others_comments(node: &Value) -> usize {
    comments(node)
        .filter(|(name, _)| name.as_deref() != Some("Author Notes"))
        .count()
}

/// Every comment with something written in it, and what it is called.
fn comments(node: &Value) -> impl Iterator<Item = (Option<String>, String)> + '_ {
    let items: Vec<&Value> = match &node["comment"] {
        Value::Array(items) => items.iter().collect(),
        Value::Null => Vec::new(),
        comment => vec![comment],
    };
    items.into_iter().filter_map(|comment| match comment {
        Value::String(text) => Some((None, text.trim().to_string())).filter(|(_, t)| !t.is_empty()),
        Value::Object(_) => Some((text(comment, "name"), text(comment, "text")?)),
        _ => None,
    })
}

/// Tags: keywords, categories and cuisines, each once.
fn tags(node: &Value) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for field in ["keywords", "recipeCategory", "recipeCuisine"] {
        let named = match &node[field] {
            Value::String(text) if field == "keywords" => {
                text.split(',').map(str::to_string).collect()
            }
            value => named_in(value),
        };
        for tag in named {
            let tag = decode_entities(tag.trim());
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }
    tags
}

/// The names a field holds: text, an object's `name`, or a list of either.
fn named_in(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string())
            .filter(|text| !text.is_empty())
            .into_iter()
            .collect(),
        Value::Array(items) => items.iter().flat_map(named_in).collect(),
        Value::Object(_) => text(value, "name").into_iter().collect(),
        _ => Vec::new(),
    }
}

/// Where each video is, for a video the file names.
fn videos(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().flat_map(videos).collect(),
        Value::String(link) if !link.trim().is_empty() => vec![link.trim().to_string()],
        Value::Object(_) => ["contentUrl", "embedUrl", "url"]
            .into_iter()
            .find_map(|field| text(value, field))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

/// Who the photo is by, where the file credits it: Bar Assistant writes the
/// credit a cook gave the picture as its author.
fn photo_credit(image: &Value) -> Option<String> {
    let image = match image {
        Value::Array(images) => images.first()?,
        image => image,
    };
    ["author", "creditText", "copyrightHolder"]
        .into_iter()
        .find_map(|field| named_in(&image[field]).into_iter().next())
}

/// How many of something a field holds: one for one, the length of a list.
fn count(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().filter(|item| !is_empty(item)).count(),
        value if is_empty(value) => 0,
        _ => 1,
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
        _ => false,
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
    use crate::other_apps::{Photo, Recipe, Unread};
    use std::io::Write;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    fn export_of(bytes: Vec<u8>) -> other_apps::Export {
        SchemaOrg
            .read(&Handed::bytes(bytes))
            .unwrap_or_else(|_| panic!("refused"))
            .unwrap_or_else(|| panic!("not recognised"))
    }

    fn recipes(name: &str) -> (&'static str, Vec<Result<Recipe, Unread>>) {
        let export = export_of(fixture(name));
        let total = export.total;
        let recipes: Vec<_> = export.recipes.collect();
        assert_eq!(recipes.len(), total, "{name}");
        (export.app, recipes)
    }

    fn named<'a>(recipes: &'a [Result<Recipe, Unread>], title: &str) -> &'a Recipe {
        recipes
            .iter()
            .filter_map(|recipe| recipe.as_ref().ok())
            .find(|recipe| recipe.candidate["title"] == title)
            .unwrap_or_else(|| panic!("no {title}"))
    }

    fn rows(content: &Value, field: &str) -> Vec<String> {
        content[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| match row["kind"].as_str() {
                Some("section") => format!("## {}", row["text"].as_str().unwrap()),
                _ => row["text"].as_str().unwrap().to_string(),
            })
            .collect()
    }

    fn paragraphs(recipe: &Recipe) -> Vec<String> {
        recipe.candidate["note"]
            .as_str()
            .map(|note| note.split("\n\n").map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn zip_of(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            for (name, bytes) in entries {
                zip.start_file(name.as_str(), zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    /// Every file in a zip, by name.
    fn entries(bytes: Vec<u8>) -> Vec<(String, Vec<u8>)> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|index| {
                let mut entry = archive.by_index(index).unwrap();
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
                (entry.name().to_string(), bytes)
            })
            .collect()
    }

    #[test]
    fn each_real_file_is_told_by_its_shape() {
        for (file, app, total) in [
            (
                "schema-org/nextcloud-mealie-sample.zip",
                "nextcloud_cookbook",
                3,
            ),
            (
                "schema-org/nextcloud-flutter-recipes.zip",
                "nextcloud_cookbook",
                19,
            ),
            ("schema-org/recipya-1.2.2.zip", "recipya", 2),
            ("schema-org/cookcli-0.38.1.zip", "cookcli", 3),
            ("schema-org/recipesage-4.0.13.json", "recipesage", 2),
            ("tandoor/v2-schema-org.json", "tandoor", 10),
            ("tandoor/v2.6-schema-org.json", "tandoor", 2),
        ] {
            let (read_as, recipes) = recipes(file);
            assert_eq!((read_as, recipes.len()), (app, total), "{file}");
            assert!(recipes.iter().all(Result::is_ok), "{file}");
        }
        // One CookCLI file alone is one recipe.
        let lemonade = entries(fixture("schema-org/cookcli-0.38.1.zip"))
            .into_iter()
            .find(|(name, _)| name == "Lemonade.json")
            .unwrap()
            .1;
        let export = export_of(lemonade);
        assert_eq!((export.app, export.total), ("cookcli", 1));
    }

    #[test]
    fn what_is_no_recipe_file_is_not_this_readers() {
        for bytes in [
            b"%PDF-1.7 not a recipe".to_vec(),
            br#"{"name": "a settings file"}"#.to_vec(),
            br#"[1, 2, 3]"#.to_vec(),
            zip_of(&[("notes.txt".to_string(), b"hello".to_vec())]),
            fixture("tandoor/v2.6-export.zip"),
            // Bar Assistant's own reader reads its exports, whichever format.
            fixture("schema-org/bar-assistant-6.9.1-json-ld.zip"),
        ] {
            assert!(SchemaOrg.read(&Handed::bytes(bytes)).unwrap().is_none());
        }
    }

    #[test]
    fn a_recipe_is_read_as_a_web_page_is_and_its_file_adds_tags_and_a_note() {
        let (_, recipes) = recipes("schema-org/nextcloud-mealie-sample.zip");
        let shrimp = named(&recipes, "Air Fryer Shrimp");
        let content = &shrimp.candidate;
        assert_eq!(content["prep_time_minutes"], 15);
        assert_eq!(content["cook_time_minutes"], 8);
        assert_eq!(
            content["yield"],
            json!({ "amount": "4", "noun": "servings" })
        );
        assert_eq!(
            content["source"],
            json!({ "text": "Anna", "link": "https://www.crunchycreamysweet.com/air-fryer-shrimp/" })
        );
        assert_eq!(
            content["nutrition"],
            json!({ "calories": 134.0, "basis": "per_serving" })
        );
        assert_eq!(rows(content, "ingredients")[0], "1 lb shrimp");
        assert_eq!(rows(content, "steps").len(), 6);
        assert_eq!(shrimp.tags, ["air fryer shrimp", "Main Course", "American"]);
        // The total is the prep and cook times added up, so it is no line.
        assert_eq!(
            paragraphs(shrimp),
            [
                "These Air Fryer Shrimp are plump, juicy and perfectly seasoned! This healthy \
                 dish is ready in just 8 minutes and requires pantry staples to make it.",
                "Nutrition: serving size 1 serving, fat 4 g, saturated fat 1 g, cholesterol \
                 286 mg, sodium 1172 mg, carbohydrates 1 g, fibre 1 g, sugar 1 g, protein 23 g",
            ]
        );
        // Its own picture, `full.jpg`, and never its thumbnail.
        assert!(matches!(&shrimp.photo, Some(Photo::Bytes(bytes)) if bytes.len() == 40193));
        assert!(matches!(shrimp.left_out[..], [LeftOut::Ratings(1)]));

        // With no picture beside it, the one it names is downloaded.
        let pie = named(&recipes, "Skillet Shepherd's Pie");
        assert!(matches!(
            &pie.photo,
            Some(Photo::Address(address)) if address.starts_with("https://www.jocooks.com/")
        ));
        assert_eq!(pie.foreign_id, "4485");

        let chicken = named(&recipes, "Chicken Parmigiana");
        assert!(matches!(
            chicken.left_out[..],
            [LeftOut::Ratings(1), LeftOut::Comments(10)]
        ));
        assert_eq!(
            paragraphs(chicken).last().unwrap(),
            "Video: https://players.brightcove.net/1033249144001/default_default/index.html?videoId=1653498713001"
        );
    }

    #[test]
    fn a_total_no_field_holds_is_a_line_of_the_note_and_one_that_adds_up_is_not() {
        let (_, recipya) = recipes("schema-org/recipya-1.2.2.zip");
        let cake = named(&recipya, "Orange cake");
        assert_eq!(cake.candidate["prep_time_minutes"], 25);
        assert_eq!(cake.candidate["cook_time_minutes"], 45);
        assert!(!cake.candidate["note"].as_str().unwrap().contains("Total"));

        let (_, flutter) = recipes("schema-org/nextcloud-flutter-recipes.zip");
        let bread = named(&flutter, "Bauernbrot mit Sauerteig");
        assert_eq!(bread.candidate["prep_time_minutes"], 30);
        assert_eq!(bread.candidate["cook_time_minutes"], Value::Null);
        assert_eq!(
            paragraphs(bread).last().unwrap(),
            "Total time: 17 hours 30 minutes"
        );

        // RecipeSage keeps an active time and a total, and no cook time.
        let (_, sage) = recipes("schema-org/recipesage-4.0.13.json");
        let pasta = named(&sage, "Pâtes à la tomate");
        assert_eq!(pasta.candidate["prep_time_minutes"], 5);
        assert_eq!(paragraphs(pasta), ["Temps total : 20 minutes"]);
    }

    #[test]
    fn recipesage_headings_are_sections_its_notes_the_note_and_its_photo_in_the_file() {
        let (_, recipes) = recipes("schema-org/recipesage-4.0.13.json");
        let cake = named(&recipes, "Orange cake");
        assert_eq!(cake.foreign_id, "830d2122-ef52-4212-bca4-9fd7512770f4");
        assert_eq!(
            rows(&cake.candidate, "ingredients"),
            [
                "## Cake",
                "225 g butter, soft",
                "225 g caster sugar",
                "4 eggs",
                "225 g self-raising flour",
                "2 oranges",
                "## Syrup",
                "1/2 cup icing sugar",
            ]
        );
        assert_eq!(rows(&cake.candidate, "steps")[3], "## Syrup");
        assert_eq!(
            cake.candidate["yield"],
            json!({ "amount": "8", "noun": "slices" })
        );
        assert_eq!(
            cake.candidate["source"],
            json!({ "text": "A cook's notebook", "link": "https://example.com/orange-cake" })
        );
        assert_eq!(
            paragraphs(cake),
            [
                "A moist cake soaked in a sticky orange syrup.",
                "Keeps three days in a tin.",
                "Total time: 1 hour 10 minutes",
                "Nutrition: serving size 1 slice, fat 16 g, sugar 30 g, protein 4 g, Calcium: 40 mg",
            ]
        );
        assert_eq!(cake.tags, ["cake", "citrus"]);
        // Written into the file as a `data:` address: a JPEG.
        assert!(
            matches!(&cake.photo, Some(Photo::Bytes(bytes)) if bytes.starts_with(&[0xFF, 0xD8]))
        );
        assert!(matches!(cake.left_out[..], [LeftOut::Ratings(1)]));
    }

    #[test]
    fn tandoors_lines_are_written_again_as_its_own_export_reads_them() {
        assert_eq!(
            tandoor_line("225.0 g butter").as_deref(),
            Some("225 g butter")
        );
        assert_eq!(tandoor_line("4.0 None egg").as_deref(), Some("4 egg"));
        assert_eq!(tandoor_line("0.0 None salt").as_deref(), Some("salt"));
        assert_eq!(
            tandoor_line("0.5 cup icing sugar").as_deref(),
            Some("½ cup icing sugar")
        );
        assert_eq!(
            tandoor_line("1.0 Taza de harina").as_deref(),
            Some("1 Taza de harina")
        );
        assert_eq!(tandoor_line("0.0 None None"), None);

        let (_, recipes) = recipes("tandoor/v2.6-schema-org.json");
        let cake = named(&recipes, "Orange cake");
        assert_eq!(
            rows(&cake.candidate, "ingredients"),
            [
                "225 g butter",
                "225 g caster sugar",
                "4 egg",
                "225 g self-raising flour",
                "salt",
                "2 orange",
                "2 orange",
                "½ cup icing sugar",
            ]
        );
        // Working time 25 of 70 minutes: waiting 45, and no total line.
        assert_eq!(cake.candidate["prep_time_minutes"], 25);
        assert_eq!(cake.candidate["cook_time_minutes"], 45);
        assert_eq!(
            paragraphs(cake),
            ["A moist cake soaked in a sticky orange syrup."]
        );
        assert_eq!(cake.foreign_id, "Orange cake");
        let pasta = named(&recipes, "Pâtes à la tomate");
        assert_eq!(
            pasta.candidate["yield"],
            json!({ "amount": "2", "noun": "portions" })
        );
    }

    #[test]
    fn what_a_page_lacks_is_worded_in_the_recipes_language() {
        let (_, recipes) = recipes("schema-org/cookcli-0.38.1.zip");
        let pasta = named(&recipes, "Pâtes à la tomate");
        assert_eq!(paragraphs(pasta), ["Matériel : poêle."]);
        assert_eq!(pasta.tags, ["pâtes", "rapide"]);
        let cake = named(&recipes, "Orange cake");
        assert_eq!(
            rows(&cake.candidate, "steps")[..2],
            [
                "## Cake",
                "Heat the oven to 180°C. Beat butter and caster sugar in a bowl until pale, \
                 then beat in eggs one at a time."
            ]
        );
        assert_eq!(
            paragraphs(cake),
            [
                "A moist cake soaked in a sticky orange syrup.",
                "Equipment: bowl, 20 cm tin."
            ]
        );
        assert!(matches!(
            &cake.photo,
            Some(Photo::Address(address)) if address.starts_with("https://upload.wikimedia.org/")
        ));
    }

    #[test]
    fn a_recipe_is_known_by_its_id_its_link_or_its_name_and_a_second_is_numbered() {
        let (_, flutter) = recipes("schema-org/nextcloud-flutter-recipes.zip");
        // Nextcloud Cookbook writes 0 for a recipe it has not numbered.
        assert_eq!(
            named(&flutter, "Recipe Without an image").foreign_id,
            "Recipe Without an image"
        );
        assert_eq!(named(&flutter, "Gelber Smoothie").foreign_id, "922169");

        // Two recipes from one page are told apart by their names, and two
        // of one name are numbered, never as a third is already known.
        let mut files = entries(fixture("schema-org/cookcli-0.38.1.zip"));
        let file = |name: &str| files.iter().find(|(n, _)| n == name).unwrap().1.clone();
        let (lemonade, cake) = (file("Lemonade.json"), file("Orange cake.json"));
        let mut renamed: Value = serde_json::from_slice(&lemonade).unwrap();
        renamed["name"] = json!("Lemonade (2)");
        let mut other_cake: Value = serde_json::from_slice(&cake).unwrap();
        other_cake["name"] = json!("Orange drizzle cake");
        files.push(("Lemonade again.json".to_string(), lemonade));
        files.push((
            "Lemonade two.json".to_string(),
            renamed.to_string().into_bytes(),
        ));
        files.push((
            "Drizzle.json".to_string(),
            other_cake.to_string().into_bytes(),
        ));
        let ids: Vec<String> = export_of(zip_of(&files))
            .recipes
            .map(|recipe| recipe.ok().unwrap().foreign_id)
            .collect();
        assert_eq!(
            ids,
            [
                "Lemonade",
                "https://example.com/orange-cake Orange cake",
                "Pâtes à la tomate",
                "Lemonade (3)",
                "Lemonade (2)",
                "https://example.com/orange-cake Orange drizzle cake",
            ]
        );
    }

    #[test]
    fn another_json_beside_the_folders_changes_nothing() {
        let mut files = entries(fixture("schema-org/nextcloud-mealie-sample.zip"));
        files.push((
            "settings.json".to_string(),
            b"{\"theme\": \"dark\"}".to_vec(),
        ));
        files.push(("Air Fryer Shrimp/notes.json".to_string(), b"[]".to_vec()));
        let export = export_of(zip_of(&files));
        assert_eq!((export.app, export.total), ("nextcloud_cookbook", 3));
        let recipes: Vec<_> = export.recipes.collect();
        let shrimp = named(&recipes, "Air Fryer Shrimp");
        assert!(matches!(&shrimp.photo, Some(Photo::Bytes(bytes)) if bytes.len() == 40193));

        // Among CookCLI's loose files, a JSON that is no recipe is somebody
        // else's, and one that is no JSON at all a recipe lost.
        let mut files = entries(fixture("schema-org/cookcli-0.38.1.zip"));
        files.push(("package.json".to_string(), b"{\"name\": \"x\"}".to_vec()));
        files.push(("Broken.json".to_string(), b"{\"@type\": \"Rec".to_vec()));
        let recipes: Vec<_> = export_of(zip_of(&files)).recipes.collect();
        assert_eq!(recipes.len(), 4);
        let unread: Vec<_> = recipes.iter().filter_map(|r| r.as_ref().err()).collect();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].name.as_deref(), Some("Broken"));
    }

    #[test]
    fn a_recipe_json_that_cannot_be_read_is_listed_and_the_rest_arrive() {
        let mut files = entries(fixture("schema-org/nextcloud-mealie-sample.zip"));
        for (name, bytes) in &mut files {
            if name == "Chicken Parmigiana/recipe.json" {
                bytes.truncate(100);
            }
        }
        let recipes: Vec<_> = export_of(zip_of(&files)).recipes.collect();
        assert_eq!(recipes.len(), 3);
        let unread: Vec<&Unread> = recipes.iter().filter_map(|r| r.as_ref().err()).collect();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].name.as_deref(), Some("Chicken Parmigiana"));
    }
}
