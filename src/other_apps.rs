//! Reading another recipe app's export (#412, the spec is #411): the one shape
//! every app's reader hands over, and the asking of each reader in turn which
//! of them made the file.
//!
//! **A reader reads and nothing else.** It takes the file handed over and
//! answers its recipes one at a time, each as recipe content, the app's own id
//! for it, its Tags, its photo and what it deliberately left out, plus what the
//! whole file held that belongs to no recipe. Landing, the ledger and the
//! database are [`crate::core::Core`]'s, so a reader can be tested from a file
//! to its recipes and nothing more.
//!
//! - **A file is recognised by what is in it, never by its name.** Each reader
//!   looks inside and says whether the file is its app's; the first that says
//!   yes reads it.
//! - **A reader knows the shapes it was tested against.** Most formats carry no
//!   version number, so a reader that knows the app but not this shape of its
//!   export refuses with [`Refusal::UntestedShape`] rather than guessing, and
//!   the screen tells the cook to update the app and export again.
//! - **Unpacking stops at a limit at every level.** Some formats are zips
//!   inside zips, or gzip inside zips, so a reader opens what is nested through
//!   [`Level::inner`], which refuses a file nested deeper than any app writes,
//!   and unpacks every file through one [`Budget`] for the whole export.

use std::io::{Read, Seek};
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

/// The readers, asked in this order. Each later ticket of #411 adds its app.
pub static READERS: &[&dyn Reader] = &[
    &crate::mealie::Mealie,
    &crate::mealie_backup::MealieBackup,
    &crate::paprika::Paprika,
    &crate::tandoor::Tandoor,
    &crate::crouton::Crouton,
    &crate::kitchenowl::KitchenOwl,
    // Bar Assistant's JSON-LD export is recipe files too, so its reader is
    // asked first.
    &crate::bar_assistant::BarAssistant,
    &crate::schema_org::SchemaOrg,
];

/// What a cook handed over, which each reader opens afresh to look inside.
pub struct Handed {
    source: HandedSource,
    /// The Mealie household the cook chose, for a backup holding several
    /// (#418).
    household: Option<String>,
}

enum HandedSource {
    File(PathBuf),
    Bytes(Arc<Vec<u8>>),
}

pub trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

impl Handed {
    /// A staged upload on disk.
    pub fn file(path: PathBuf) -> Self {
        Handed {
            source: HandedSource::File(path),
            household: None,
        }
    }

    /// The file's bytes, sent inline by a Door that can send only JSON.
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Handed {
            source: HandedSource::Bytes(Arc::new(bytes)),
            household: None,
        }
    }

    /// The same file, with the household the cook chose in it (#418).
    pub fn in_household(mut self, household: Option<&str>) -> Self {
        self.household = household.map(str::to_string);
        self
    }

    /// The household the cook chose, where they chose one.
    pub fn household(&self) -> Option<&str> {
        self.household.as_deref()
    }

    /// The file from its first byte.
    pub fn open(&self) -> std::io::Result<Box<dyn ReadSeek>> {
        Ok(match &self.source {
            HandedSource::File(path) => Box::new(std::fs::File::open(path)?),
            HandedSource::Bytes(bytes) => Box::new(std::io::Cursor::new(ArcBytes(bytes.clone()))),
        })
    }

    /// The file as JSON, where it is an object or an array of no more than
    /// `most` bytes. A zip, a PDF or a picture is turned away by its first
    /// bytes, before the rest of it is read.
    pub fn json(&self, most: u64) -> Option<Value> {
        let mut start = Vec::new();
        self.open().ok()?.take(1024).read_to_end(&mut start).ok()?;
        let first = start.iter().find(|byte| !byte.is_ascii_whitespace());
        if !matches!(first, Some(b'{' | b'[')) {
            return None;
        }
        let mut bytes = Vec::new();
        self.open()
            .ok()?
            .take(most + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 > most {
            return None;
        }
        serde_json::from_slice(&bytes).ok()
    }

    /// Whether the file is a zip, by its first four bytes.
    pub fn is_zip(&self) -> bool {
        self.open()
            .ok()
            .and_then(|mut file| {
                let mut magic = [0u8; 4];
                file.read_exact(&mut magic).ok()?;
                Some(magic == *b"PK\x03\x04")
            })
            .unwrap_or(false)
    }
}

/// Shared bytes a `Cursor` can read, so each reader's look inside costs no copy.
struct ArcBytes(Arc<Vec<u8>>);

impl AsRef<[u8]> for ArcBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// How deep a reader has opened: the file handed over is level one, and an
/// archive inside it is level two. No app's export nests deeper than a zip of
/// zips or a zip of gzip files, which is level two; three leaves room for one
/// more, and four is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level(u8);

/// The deepest level a reader may open.
pub const MOST_NESTED: u8 = 3;

impl Level {
    /// The file handed over.
    pub const OUTER: Level = Level(1);

    /// The level of an archive found inside this one, refused past
    /// [`MOST_NESTED`]: a file nested that deep is no app's export, and
    /// unpacking it is how one small file fills a disk.
    pub fn inner(self) -> Result<Level, Refusal> {
        if self.0 >= MOST_NESTED {
            Err(Refusal::NestedTooDeep)
        } else {
            Ok(Level(self.0 + 1))
        }
    }
}

/// The most one export ever unpacks, counted across every file in it at
/// every level (#373): a zip, a zip inside it, a gzip inside that. A ceiling
/// per file alone is no limit, since 128 MiB of zeros packs into about 130 KB
/// and a small zip can name thousands of such files.
///
/// Why 4 GB: the real Crouton export of 86 recipes, of 23 September 2026, is
/// an 83 MB zip that unpacks to 114 MB, 1.37 times its size, because its
/// pictures are already compressed. `POST /api/uploads` takes a file of up to
/// 2 GiB, which at that ratio unpacks to some 2.9 GB, so every export the
/// upload takes fits under this one.
pub const MOST_UNPACKED: u64 = 4 * 1000 * 1000 * 1000;

/// How much more one export may unpack. Every reader unpacks through one, so
/// the ceiling holds at every level of a file however its archives nest.
#[derive(Debug)]
pub struct Budget {
    left: u64,
    most: u64,
}

/// The export unpacks to more than its [`Budget`] allows.
#[derive(Debug, PartialEq, Eq)]
pub struct TooMuch;

impl Budget {
    pub fn new(most: u64) -> Self {
        Budget { left: most, most }
    }

    /// The ceiling this budget started at, which a refusal names.
    pub fn most(&self) -> u64 {
        self.most
    }

    /// Unpack one file of an archive, at most `one` bytes of it: its bytes, and
    /// whether it unpacked cleanly. An archive says how big its files are and
    /// can lie, so every byte unpacked is counted rather than the header
    /// believed. A damaged file counts too: its checksum is checked only once
    /// it has been unpacked to the end. Past the budget the rest of the export
    /// is not read.
    pub fn unpack(&mut self, entry: impl Read, one: u64) -> Result<(Vec<u8>, bool), TooMuch> {
        let room = self.left;
        let mut bytes = Vec::new();
        let unpacked = entry.take(one.min(room) + 1).read_to_end(&mut bytes);
        let read = bytes.len() as u64;
        if read > room {
            self.left = 0;
            return Err(TooMuch);
        }
        self.left -= read;
        Ok((bytes, unpacked.is_ok()))
    }
}

/// One app's export reader.
pub trait Reader: Sync {
    /// The names of the apps it reads, as a cook knows them, which the
    /// refusal of an unknown file lists. Most readers read one app's export;
    /// a format several apps write is read by one reader (#416).
    fn names(&self) -> &'static [&'static str];

    /// Look inside `handed`: `Ok(None)` when it is not this app's export, its
    /// recipes when it is, and a refusal when it is this app's but cannot be
    /// read, such as a shape no test file has.
    fn read(&self, handed: &Handed) -> Result<Option<Export>, Refusal>;
}

/// Hand `handed` to the first of `readers` that knows it.
pub fn read(handed: &Handed, readers: &[&dyn Reader]) -> Result<Export, Refusal> {
    for reader in readers {
        if let Some(export) = reader.read(handed)? {
            return Ok(export);
        }
    }
    Err(Refusal::Unrecognised)
}

/// Why a file was refused whole, with nothing imported. The screen says each
/// in its own words; [`Refusal::sentence`] is the Core's, for the MCP Door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No reader knows this file.
    Unrecognised,
    /// A reader knows the app but not this shape of its export.
    UntestedShape { app: &'static str },
    /// The file nests archives deeper than [`MOST_NESTED`].
    NestedTooDeep,
    /// A Mealie backup holding recipes of several households, none of them
    /// chosen, or one it does not hold (#418). Nothing is refused for good:
    /// the answer names them, and the same file comes in once one is.
    WhichHousehold(Vec<Household>),
}

/// One household of a Mealie backup, as the cook chooses between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Household {
    /// What the cook names it by, and calls again with.
    pub name: String,
    /// How many of its recipes the file holds.
    pub recipes: usize,
    /// The full names of the cooks in it, as Mealie keeps them.
    pub cooks: Vec<String>,
}

impl Household {
    pub fn to_value(&self) -> Value {
        json!({ "name": self.name, "recipes": self.recipes, "cooks": self.cooks })
    }
}

impl Refusal {
    pub fn reason(&self) -> &'static str {
        match self {
            Refusal::Unrecognised => "unrecognised",
            Refusal::UntestedShape { .. } => "untested_shape",
            Refusal::NestedTooDeep => "nested_too_deep",
            Refusal::WhichHousehold(_) => "which_household",
        }
    }

    pub fn sentence(&self) -> String {
        match self {
            Refusal::Unrecognised => format!(
                "Kamosu could not tell which app made this file, so nothing was imported. \
                 It reads exports from {}",
                READERS
                    .iter()
                    .flat_map(|reader| reader.names().iter().copied())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Refusal::UntestedShape { app } => format!(
                "this is an export from {app}, in a shape Kamosu has not been tested with, \
                 so nothing was imported: update {app}, export again and import the new file"
            ),
            Refusal::NestedTooDeep => format!(
                "this file holds archives nested more than {MOST_NESTED} deep, which no app's \
                 export does, so it was not unpacked and nothing was imported"
            ),
            Refusal::WhichHousehold(households) => format!(
                "this Mealie backup holds the recipes of {} households, so nothing was imported \
                 yet: import the same file again, naming one of them as mealie_household",
                households.len()
            ),
        }
    }

    /// What `import_from_another_app` answers: `which_household` alone for a
    /// backup whose household is to be chosen, and `refused` alone otherwise.
    pub fn answer(&self) -> Value {
        match self {
            Refusal::WhichHousehold(households) => json!({ "which_household": {
                "households": households.iter().map(Household::to_value).collect::<Vec<_>>(),
                "sentence": self.sentence(),
            } }),
            _ => json!({ "refused": self.to_value() }),
        }
    }

    /// The answer's `refused`.
    pub fn to_value(&self) -> Value {
        let mut refused = json!({ "reason": self.reason(), "sentence": self.sentence() });
        if let Refusal::UntestedShape { app } = self {
            refused["app"] = json!(app);
        }
        refused
    }
}

/// A recognised export: its recipes, read one at a time as they are pulled,
/// and what the whole file held that belongs to no recipe.
pub struct Export {
    /// The source kind the ledger keeps it under: `crouton`, `mealie` and so on.
    pub app: &'static str,
    /// How many recipes the file offers, the Job's total.
    pub total: usize,
    pub recipes: Box<dyn Iterator<Item = Result<Recipe, Unread>> + Send>,
    pub left_out: Vec<FileLeftOut>,
}

/// One recipe as a reader hands it over.
pub struct Recipe {
    /// What the app calls it, the ledger's key.
    pub foreign_id: String,
    /// Each time somebody cooked it, as the app recorded it (#418).
    pub cookings: Vec<Cooking>,
    /// Recipe content in the shape `create_recipe` accepts, `main_photo` absent:
    /// storing the photo is the Core's work.
    pub candidate: Value,
    /// English words to file it under, beside the content and never in it,
    /// since a Tag is filing (ADR 0035).
    pub tags: Vec<String>,
    /// What becomes its Main Photo.
    pub photo: Option<Photo>,
    /// Pictures the app placed in a Step, each by the Step's place in the
    /// candidate's `steps` (#413).
    pub step_photos: Vec<StepPhoto>,
    /// Lines that point at another of the app's recipes, which become
    /// Components once the whole file has landed (#413).
    pub components: Vec<Component>,
    pub left_out: Vec<LeftOut>,
}

/// A cooking the app recorded, which lands as a finished Attempt of the
/// importing Person (#418, ADR 0053).
pub struct Cooking {
    /// What the app calls the record, so a second import adds none twice.
    pub foreign_id: String,
    /// When it was cooked, as the app wrote it; the Core reads it as a time.
    pub cooked_at: String,
    /// The Attempt's note, which begins by naming who cooked it.
    pub note: String,
    pub photo: Option<Vec<u8>>,
}

/// A picture the app placed in one Step.
pub struct StepPhoto {
    /// The Step's place in the candidate's `steps`, Sections counted.
    pub step: usize,
    pub bytes: Vec<u8>,
}

/// An Ingredient Line that points at another of the app's recipes (#413).
pub struct Component {
    /// The line's place in the candidate's `ingredients`, Sections counted.
    pub line: usize,
    /// The app's own id for the recipe it points at.
    pub foreign_id: String,
    /// How many of that recipe the line asks for, where the app counts in
    /// whole recipes: one is all of it.
    pub times: Option<f64>,
    /// The app's copy of that recipe, which lands in its place when the file
    /// holds it nowhere else and no earlier import brought it in.
    pub copy: Option<Box<Recipe>>,
}

/// A Main Photo as the file holds it.
pub enum Photo {
    /// The picture itself.
    Bytes(Vec<u8>),
    /// Only its web address.
    Address(String),
    /// The app had one, and its export does not carry it.
    NotInExport,
}

/// What each recipe is known by, given `ids` in the file's order: each as it
/// is, and one already given numbered, `Lemonade (2)`, never as another of
/// `ids` is already known.
pub fn numbered(ids: Vec<String>) -> Vec<String> {
    let mut taken: std::collections::HashSet<String> = ids.iter().cloned().collect();
    let mut given = std::collections::HashSet::new();
    ids.into_iter()
        .map(|id| {
            if given.insert(id.clone()) {
                return id;
            }
            let numbered = (2..)
                .map(|n| format!("{id} ({n})"))
                .find(|numbered| !taken.contains(numbered))
                .unwrap_or_default();
            taken.insert(numbered.clone());
            given.insert(numbered.clone());
            numbered
        })
        .collect()
}

/// A Source that is a web page alone: named by the page's site, and linking
/// to it.
pub fn source_of_link(link: &str) -> Value {
    json!({ "text": host(link).unwrap_or_else(|| link.to_string()), "link": link })
}

/// A link's site as a cook names it: its host, `www.` left off.
pub fn host(link: &str) -> Option<String> {
    let rest = link.split_once("://").map_or(link, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_string())
}

/// A recipe a reader could not read at all. `name` is what the file called it
/// where its own id was unreadable too, such as a Crouton file's name.
pub struct Unread {
    pub foreign_id: Option<String>,
    pub name: Option<String>,
    pub reason: String,
}

/// Something a reader read on one recipe and chose not to bring in (#69,
/// #412), named in the Report's `left_out`.
pub enum LeftOut {
    /// The source site's favicon, not a photograph of the dish (ADR 0017).
    /// Its bytes ride into the Report, and only there; it is never stored.
    SiteIcon(Vec<u8>),
    /// Pictures past the first: a recipe keeps one Main Photo, and Kamosu does
    /// not guess which Step another one shows.
    ExtraPhotos(usize),
    /// Pictures that could not be decoded or remade.
    UnreadablePhotos(usize),
    /// Files the app kept beside the recipe that are not its photos.
    AttachedFiles(usize),
    /// Scores the app kept, which no cooking in Kamosu stands behind: one,
    /// or one for each cook who scored it (#418).
    Ratings(usize),
    /// The app's favourite marks, one for each cook who set one.
    Favourites(usize),
    /// Comments people wrote on the recipe in the app.
    Comments(usize),
    /// A Main Photo the file holds only as an address, which was not downloaded.
    PhotoNotDownloaded(String),
    /// Nutrition the app kept as free text, which Kamosu cannot be sure of a
    /// number in (#414).
    Nutrition,
    /// Entries the app wrote on the recipe's timeline from its meal plan,
    /// which say it was planned, not that anyone cooked it (#418).
    MealPlanTimeline(usize),
}

/// Something the whole file held that belongs to no recipe, named in the
/// Report's `left_out_of_file`.
pub enum FileLeftOut {
    MealPlans(usize),
    ShoppingLists(usize),
    /// Recipes the app had a photo for that its export does not carry.
    PhotosNotInExport(usize),
    /// The app's catalogue of things to buy (#417).
    Items(usize),
    /// Money the household recorded spending (#417).
    Expenses(usize),
}

impl FileLeftOut {
    /// How many of it the file held.
    pub fn count(&self) -> usize {
        match self {
            FileLeftOut::MealPlans(count)
            | FileLeftOut::ShoppingLists(count)
            | FileLeftOut::PhotosNotInExport(count)
            | FileLeftOut::Items(count)
            | FileLeftOut::Expenses(count) => *count,
        }
    }

    /// The Report's row for it.
    pub fn row(&self) -> Value {
        let (what, count) = match self {
            FileLeftOut::MealPlans(count) => ("meal_plans", count),
            FileLeftOut::ShoppingLists(count) => ("shopping_lists", count),
            FileLeftOut::PhotosNotInExport(count) => ("photos_not_in_export", count),
            FileLeftOut::Items(count) => ("items", count),
            FileLeftOut::Expenses(count) => ("expenses", count),
        };
        json!({ "what": what, "count": count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            for (name, bytes) in entries {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn empty_export(app: &'static str) -> Export {
        Export {
            app,
            total: 0,
            recipes: Box::new(std::iter::empty()),
            left_out: Vec::new(),
        }
    }

    /// Knows a zip holding `recipes.test`, and reads it only when it says `v1`.
    struct Versioned;

    impl Reader for Versioned {
        fn names(&self) -> &'static [&'static str] {
            &["Testapp"]
        }

        fn read(&self, handed: &Handed) -> Result<Option<Export>, Refusal> {
            let Ok(mut archive) = zip::ZipArchive::new(handed.open().unwrap()) else {
                return Ok(None);
            };
            let Ok(mut entry) = archive.by_name("recipes.test") else {
                return Ok(None);
            };
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            if text.trim() != "v1" {
                return Err(Refusal::UntestedShape { app: "Testapp" });
            }
            Ok(Some(empty_export("testapp")))
        }
    }

    /// Knows a zip of `inner.zip`s, and opens each level through [`Level`].
    struct Nested;

    impl Nested {
        fn open(bytes: Vec<u8>, level: Level) -> Result<Option<()>, Refusal> {
            let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
                return Ok(None);
            };
            let Ok(mut entry) = archive.by_name("inner.zip") else {
                return Ok(Some(()));
            };
            let level = level.inner()?;
            let mut inner = Vec::new();
            entry.read_to_end(&mut inner).unwrap();
            Nested::open(inner, level)
        }
    }

    impl Reader for Nested {
        fn names(&self) -> &'static [&'static str] {
            &["Nested"]
        }

        fn read(&self, handed: &Handed) -> Result<Option<Export>, Refusal> {
            let mut bytes = Vec::new();
            handed.open().unwrap().read_to_end(&mut bytes).unwrap();
            Ok(Nested::open(bytes, Level::OUTER)?.map(|()| empty_export("nested")))
        }
    }

    #[test]
    fn the_first_reader_that_knows_the_file_reads_it() {
        let handed = Handed::bytes(zip_of(&[("recipes.test", b"v1")]));
        let export = read(&handed, &[&Nested, &Versioned]).unwrap_or_else(|_| panic!());
        // `Nested` knows any zip with no `inner.zip` as a level it reached the
        // bottom of; asked first, it answers first.
        assert_eq!(export.app, "nested");
        let export = read(&handed, &[&Versioned, &Nested]).unwrap_or_else(|_| panic!());
        assert_eq!(export.app, "testapp");
    }

    #[test]
    fn a_file_no_reader_knows_is_unrecognised() {
        for bytes in [zip_of(&[("photo.jpg", b"x")]), b"not a zip at all".to_vec()] {
            let refused = read(&Handed::bytes(bytes), &[&Versioned]).err().unwrap();
            assert_eq!(refused, Refusal::Unrecognised);
            assert_eq!(refused.to_value()["reason"], "unrecognised");
        }
    }

    #[test]
    fn a_known_app_in_an_untested_shape_is_refused_with_the_update_reason() {
        let handed = Handed::bytes(zip_of(&[("recipes.test", b"v2")]));
        let refused = read(&handed, &[&Versioned]).err().unwrap();
        assert_eq!(refused, Refusal::UntestedShape { app: "Testapp" });
        let answer = refused.to_value();
        assert_eq!(answer["reason"], "untested_shape");
        assert_eq!(answer["app"], "Testapp");
        assert!(
            answer["sentence"]
                .as_str()
                .unwrap()
                .contains("update Testapp")
        );
    }

    #[test]
    fn a_file_nested_past_the_limit_is_refused_not_unpacked() {
        let bottom = zip_of(&[("recipe.json", b"{}")]);
        let two = zip_of(&[("inner.zip", &bottom)]);
        let three = zip_of(&[("inner.zip", &two)]);
        // Three levels is as deep as a reader may open.
        assert!(read(&Handed::bytes(three.clone()), &[&Nested]).is_ok());
        let four = zip_of(&[("inner.zip", &three)]);
        let refused = read(&Handed::bytes(four), &[&Nested]).err().unwrap();
        assert_eq!(refused, Refusal::NestedTooDeep);
        assert_eq!(refused.to_value()["reason"], "nested_too_deep");
    }

    #[test]
    fn a_budget_counts_every_byte_unpacked_and_stops_past_it() {
        let mut budget = Budget::new(10);
        assert_eq!(
            budget.unpack(&b"123456"[..], 100),
            Ok((b"123456".to_vec(), true))
        );
        // One file is cut at its own ceiling, and only what was read counts.
        assert_eq!(
            budget.unpack(&b"abcdef"[..], 2).map(|(b, _)| b),
            Ok(b"abc".to_vec())
        );
        assert_eq!(budget.unpack(&b"xy"[..], 100), Err(TooMuch));
        // Once spent, it stays spent.
        assert_eq!(budget.unpack(&b""[..], 100), Ok((Vec::new(), true)));
        assert_eq!(budget.unpack(&b"z"[..], 100), Err(TooMuch));
        assert_eq!(budget.most(), 10);
    }

    #[test]
    fn a_handed_file_opens_afresh_for_each_reader() {
        let handed = Handed::bytes(zip_of(&[("recipes.test", b"v1")]));
        assert!(handed.is_zip());
        assert!(handed.is_zip());
        assert!(!Handed::bytes(b"PK".to_vec()).is_zip());
    }
}
