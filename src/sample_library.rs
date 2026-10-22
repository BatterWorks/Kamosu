//! The Sample Library a Demo holds (ADR 0052): a story of what its cooks did,
//! each step some days before the day it is told, played through the real
//! Operations into a library of its own.
//!
//! The story is `sample-library/story.json`, carried inside the binary with
//! the photographs beside it, so it ships with every release and a change to
//! an Operation it calls breaks its build. Building it is the Demo's own
//! business: an ordinary Core over a database in a folder of its own, with no
//! Door, so the Demo's refusals never meet it and no visitor can reach what
//! lets it past them. Every Operation stamps the present; each step is moved
//! back to its own moment once it has run.
//!
//! What is built is poured into the Demo by `Core::put_back_sample_library`,
//! at startup and every day after, with its dates moved forward.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, params};
use rust_embed::Embed;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::catalogue::{self, Kind};
use crate::core::{Core, DEMO_ACCESS_KEY, DEMO_VISITOR, OpError};
use crate::db::Db;

#[derive(Embed)]
#[folder = "sample-library/"]
struct Files;

/// What every Session the story's cooks hold is named. None outlives the
/// build.
const SESSION_NAME: &str = "the Sample Library";

/// The file that marks a folder as one the Sample Library is built in, and
/// so one a build may empty.
const MARK: &str = "SAMPLE-LIBRARY";

/// Where in `meta` a built library says which day it was built on, which is
/// the day every load moves its dates forward from.
pub(crate) const BUILT_ON: &str = "sample_library_built_on";

/// A full time as Kamosu writes one, `2026-10-20T09:15:02.123Z`, to SQLite's
/// GLOB. Every time stored in Kamosu is a whole column value of this shape,
/// so this finds every one and nothing else.
pub(crate) const A_TIME_GLOB: &str =
    "[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]*Z";

/// The story: what happened, in the order it happened.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Story {
    steps: Vec<Step>,
}

/// One thing a cook did, on one day at one time. Exactly one of
/// `first_person`, `person`, `op`, `photograph` and `published_key` says
/// what.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    /// How many days before the day the library is built this happened: one
    /// at least, so nothing in the story is later than its telling.
    days_ago: u32,
    /// The time of day, in UTC, as `HH:MM`.
    at: String,
    /// Who acts: a cook's name, or `{ "$ref": "…" }` naming a Secret an
    /// earlier step kept, a Runner Key's or a Job Key's.
    #[serde(rename = "as")]
    actor: Option<Value>,
    /// The first Person, the Operator, who sets the instance up.
    first_person: Option<String>,
    /// Another Person, arriving by an Invite the `as` cook mints.
    person: Option<String>,
    /// An Operation, called with `input`, in which `{ "$ref": "…" }` stands
    /// for a value an earlier step kept.
    op: Option<String>,
    input: Option<Value>,
    /// A picture under `sample-library/`, stored as an upload would be.
    photograph: Option<String>,
    /// The one Access Key the Demo publishes, by its name: read-only, the
    /// `as` cook's, and its Secret always [`DEMO_ACCESS_KEY`].
    published_key: Option<String>,
    /// The name later steps find this step's answer by.
    keep: Option<String>,
}

/// A Sample Library, built in a folder of its own: its database, and the
/// Photographs and Originals beside it.
pub struct Built {
    dir: PathBuf,
}

impl Built {
    /// The built library's database, which a load reads from.
    pub fn database_path(&self) -> PathBuf {
        self.dir.join(crate::db::DATABASE_FILE)
    }

    /// The folder it was built in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Build the Sample Library into `dir`, emptied first: every step of the
/// story played as its cook, then moved back to its own moment.
pub fn build(dir: &Path) -> Result<Built, OpError> {
    let story = story()?;
    empty_for_building(dir)?;
    let db = Arc::new(Db::open(dir)?);
    let core = Core::open_to_build_sample_library(db.clone());
    let built_on: String = db.with_conn(|conn| {
        conn.query_row("SELECT date('now')", [], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read today's date: {e}")))
    })?;
    let stamped = db.with_conn(stamped_columns)?;

    let mut told = Told::default();
    // What a step may name that no step kept: this Kamosu, whose version a
    // Runner in the story says it is.
    told.kept.insert(
        "kamosu".to_string(),
        json!({ "version": env!("CARGO_PKG_VERSION") }),
    );
    let mut previous: Option<String> = None;
    for (number, step) in story.steps.iter().enumerate() {
        let failed = |error: OpError| {
            OpError::internal(format!(
                "the Sample Library's step {} ({}) failed: {}",
                number + 1,
                step.what(),
                error.message
            ))
        };
        let moment = db
            .with_conn(|conn| step.moment(conn, &built_on))
            .map_err(failed)?;
        if previous.as_ref().is_some_and(|before| *before > moment) {
            return Err(failed(OpError::bad_request(
                "it comes before the step above it; the story is told in order",
            )));
        }
        let began = db.with_conn(now)?;
        let answered = told.play(&core, step).map_err(failed)?;
        let ended = db.with_conn(now)?;
        db.with_conn(|conn| move_back(conn, &stamped, (&began, &ended), &began, &moment))?;
        if let Some(name) = &step.keep {
            told.kept.insert(name.clone(), answered);
        }
        previous = Some(moment);
    }
    // A deadline a step set ran in the build's own time, so that the steps
    // after it found it still ahead. Once the story is told, every one left
    // ahead moves back with the last step.
    if let Some(last) = &previous {
        let ended = db.with_conn(now)?;
        db.with_conn(|conn| move_back(conn, &stamped, (&ended, "9999"), &ended, last))?;
    }
    if !told.cooks.contains_key(DEMO_VISITOR) {
        return Err(OpError::internal(format!(
            "the Sample Library has nobody named {DEMO_VISITOR} for a visitor to arrive as"
        )));
    }

    db.with_conn(|conn| refuse_what_its_build_day_left(conn, &stamped, &built_on))?;

    // What the build signed in with is the build's alone: no visitor holds
    // its Sessions, and the Security Record a Demo keeps starts empty.
    db.with_conn(|conn| {
        conn.execute_batch(
            "DELETE FROM sessions; DELETE FROM security_record; DELETE FROM login_failures;",
        )
        .map_err(|e| OpError::internal(format!("cannot clear the build's sign-ins: {e}")))?;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![BUILT_ON, built_on],
        )
        .map_err(|e| OpError::internal(format!("cannot date the Sample Library: {e}")))?;
        Ok(())
    })?;
    Ok(Built {
        dir: dir.to_path_buf(),
    })
}

/// What a built library holds, counted, and where it was built: what
/// `kamosu sample-library` prints.
pub fn summary(built: &Built) -> Result<Value, OpError> {
    let db = Db::open(built.dir())?;
    db.with_conn(|conn| {
        let count = |sql: &str| -> Result<i64, OpError> {
            conn.query_row(sql, [], |row| row.get(0))
                .map_err(|e| OpError::internal(format!("cannot count the Sample Library: {e}")))
        };
        let built_on: String = conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [BUILT_ON], |row| {
                row.get(0)
            })
            .map_err(|e| OpError::internal(format!("cannot read when it was built: {e}")))?;
        let first_version_at: Option<String> = conn
            .query_row("SELECT min(created_at) FROM versions", [], |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot read the first Version: {e}")))?;
        Ok(json!({
            "folder": built.dir().display().to_string(),
            "built_on": built_on,
            "people": count("SELECT COUNT(*) FROM people")?,
            "kitchens": count("SELECT COUNT(*) FROM kitchens")?,
            "recipes": count("SELECT COUNT(*) FROM branches")?,
            "versions": count("SELECT COUNT(*) FROM versions")?,
            "attempts": count("SELECT COUNT(*) FROM attempts")?,
            "photographs": count("SELECT COUNT(*) FROM photographs")?,
            "share_links": count("SELECT COUNT(*) FROM share_links")?,
            "agent_jobs": count("SELECT COUNT(*) FROM agent_jobs")?,
            "first_version_at": first_version_at,
        }))
    })
}

/// Empty `dir` for a build, and mark it as the Sample Library's. A folder
/// that holds anything and carries no mark is somebody else's, `/data`
/// itself perhaps, and is refused rather than emptied.
fn empty_for_building(dir: &Path) -> Result<(), OpError> {
    let failed = |e: std::io::Error| {
        OpError::internal(format!(
            "cannot empty {} for the Sample Library: {e}",
            dir.display()
        ))
    };
    let marked = dir.join(MARK);
    let holds_anything = std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some());
    if holds_anything {
        if !marked.is_file() {
            return Err(OpError::bad_request(format!(
                "{} holds files that are not a Sample Library's; build it in an empty folder",
                dir.display()
            )));
        }
        std::fs::remove_dir_all(dir).map_err(failed)?;
    }
    std::fs::create_dir_all(dir).map_err(failed)?;
    std::fs::write(
        &marked,
        "A Sample Library is built here, and emptied at each build.\n",
    )
    .map_err(failed)
}

/// The story as this binary carries it.
fn story() -> Result<Story, OpError> {
    let file = Files::get("story.json")
        .ok_or_else(|| OpError::internal("the Sample Library has no story.json"))?;
    serde_json::from_slice(&file.data).map_err(|e| {
        OpError::internal(format!("the Sample Library's story.json is not right: {e}"))
    })
}

/// What the story has told so far: each cook's Secret, and the answers kept.
#[derive(Default)]
struct Told {
    cooks: HashMap<String, String>,
    kept: HashMap<String, Value>,
}

impl Told {
    fn play(&mut self, core: &Core, step: &Step) -> Result<Value, OpError> {
        let acts = [
            step.first_person.is_some(),
            step.person.is_some(),
            step.op.is_some(),
            step.photograph.is_some(),
            step.published_key.is_some(),
        ];
        if acts.iter().filter(|says| **says).count() != 1 {
            return Err(OpError::bad_request(
                "a step says exactly one of first_person, person, op, photograph and \
                 published_key",
            ));
        }
        if step.input.is_some() && step.op.is_none() {
            return Err(OpError::bad_request("input is for an op"));
        }
        // A password nobody types: a Demo refuses every sign-in.
        let password = crate::core::generate_secret();

        if let Some(name) = &step.first_person {
            let answered = core.create_first_person(name, &password, SESSION_NAME)?;
            return Ok(self.signed_in(name, answered));
        }
        let actor = self.actor(step)?;
        if let Some(name) = &step.person {
            let invite = core.execute(Some(&actor), "mint_invite", json!({}))?;
            let link = invite["link"]
                .as_str()
                .ok_or_else(|| OpError::internal("an Invite came back with no link"))?;
            let answered = core.redeem_invite(link, name, &password, SESSION_NAME)?;
            return Ok(self.signed_in(name, answered));
        }
        if let Some(path) = &step.photograph {
            let bytes = carried(path)?;
            let caller = core.authenticate_to_store_photograph(Some(&actor))?;
            return core.upload_photograph(&caller, &bytes);
        }
        if let Some(name) = &step.published_key {
            let caller = core.authenticate(Some(&actor))?;
            let key = core.mint_access_key(&caller.person_id, name, true)?;
            core.db().with_conn(|conn| {
                conn.execute(
                    "UPDATE access_keys SET secret_hash = ?1 WHERE id = ?2",
                    params![crate::core::hash_secret(DEMO_ACCESS_KEY), key.id],
                )
                .map_err(|e| OpError::internal(format!("cannot publish the Demo's key: {e}")))
            })?;
            return Ok(json!({ "id": key.id, "name": key.name }));
        }
        let op = step.op.as_deref().unwrap_or_default();
        // A Job's work is carried by a lane, and the build has none: a story
        // that needs one says so here rather than as a busy Kamosu.
        if catalogue::find(op).is_some_and(|declared| declared.kind == Kind::Job) {
            return Err(OpError::bad_request(format!(
                "{op} is a Job, and the Sample Library is built without Job lanes"
            )));
        }
        let input = self.resolved(step.input.clone().unwrap_or_else(|| json!({})))?;
        core.execute(Some(&actor), op, input)
    }

    /// A cook signed in: their Secret held for the steps that follow, and
    /// the answer kept without it.
    fn signed_in(&mut self, name: &str, mut answered: Value) -> Value {
        if let Some(secret) = answered
            .as_object_mut()
            .and_then(|fields| fields.remove("_session_secret"))
            .and_then(|secret| secret.as_str().map(str::to_string))
        {
            self.cooks.insert(name.to_string(), secret);
        }
        answered
    }

    /// The Secret a step acts with.
    fn actor(&self, step: &Step) -> Result<String, OpError> {
        match &step.actor {
            Some(Value::String(name)) => {
                self.cooks.get(name).cloned().ok_or_else(|| {
                    OpError::bad_request(format!("nobody named {name} is in it yet"))
                })
            }
            Some(reference @ Value::Object(_)) => match self.resolved(reference.clone())? {
                Value::String(secret) => Ok(secret),
                other => Err(OpError::bad_request(format!(
                    "as names {other}, which is no Secret"
                ))),
            },
            _ => Err(OpError::bad_request(
                "as says who acts: a cook's name or a $ref",
            )),
        }
    }

    /// `input` with every `{ "$ref": "name.path" }` replaced by what it names,
    /// every `{ "$file": "path" }` by that file of `sample-library/` in
    /// base64, as a Door that sends only JSON hands one over, and every
    /// `{ "$text": "path" }` by that file's words.
    fn resolved(&self, input: Value) -> Result<Value, OpError> {
        match input {
            Value::Object(fields) if fields.len() == 1 && fields.contains_key("$ref") => {
                let reference = fields["$ref"]
                    .as_str()
                    .ok_or_else(|| OpError::bad_request("a $ref is a string"))?;
                self.find(reference)
            }
            Value::Object(fields) if fields.len() == 1 && fields.contains_key("$file") => {
                use base64::Engine;
                let bytes = carried(named(&fields["$file"])?)?;
                Ok(Value::String(
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                ))
            }
            Value::Object(fields) if fields.len() == 1 && fields.contains_key("$text") => {
                let bytes = carried(named(&fields["$text"])?)?;
                String::from_utf8(bytes.to_vec())
                    .map(Value::String)
                    .map_err(|_| OpError::bad_request("a $text names a file that is not UTF-8"))
            }
            Value::Object(fields) => Ok(Value::Object(
                fields
                    .into_iter()
                    .map(|(key, value)| Ok((key, self.resolved(value)?)))
                    .collect::<Result<_, OpError>>()?,
            )),
            Value::Array(items) => Ok(Value::Array(
                items
                    .into_iter()
                    .map(|item| self.resolved(item))
                    .collect::<Result<_, _>>()?,
            )),
            other => Ok(other),
        }
    }

    /// What `name.path.to.field` names: a kept answer, then a field or an
    /// item of it at each dot.
    fn find(&self, reference: &str) -> Result<Value, OpError> {
        let mut parts = reference.split('.');
        let name = parts.next().unwrap_or_default();
        let mut found = self
            .kept
            .get(name)
            .ok_or_else(|| OpError::bad_request(format!("no step kept {name}")))?;
        for part in parts {
            found = match part.parse::<usize>() {
                Ok(index) => found.get(index),
                Err(_) => found.get(part),
            }
            .ok_or_else(|| OpError::bad_request(format!("{reference} names nothing")))?;
        }
        Ok(found.clone())
    }
}

/// A file of `sample-library/`, as a `photograph` step, a `$file` or a
/// `$text` names it.
fn carried(path: &str) -> Result<std::borrow::Cow<'static, [u8]>, OpError> {
    Files::get(path)
        .map(|file| file.data)
        .ok_or_else(|| OpError::bad_request(format!("sample-library/{path} is not there")))
}

/// The path a `$file` or a `$text` names.
fn named(path: &Value) -> Result<&str, OpError> {
    path.as_str()
        .ok_or_else(|| OpError::bad_request("a $file or $text names a path, as a string"))
}

impl Step {
    /// The step in a few words, for a build that fails on it.
    fn what(&self) -> String {
        let act = [
            self.first_person
                .as_ref()
                .map(|name| format!("first_person {name}")),
            self.person.as_ref().map(|name| format!("person {name}")),
            self.op.as_ref().map(|op| format!("op {op}")),
            self.photograph
                .as_ref()
                .map(|path| format!("photograph {path}")),
            self.published_key
                .as_ref()
                .map(|name| format!("published_key {name}")),
        ];
        let act = act.into_iter().flatten().next().unwrap_or_default();
        format!("{act}, {} days ago at {}", self.days_ago, self.at)
    }

    /// When it happened, on a library built on `built_on`.
    fn moment(&self, conn: &Connection, built_on: &str) -> Result<String, OpError> {
        if self.days_ago == 0 {
            return Err(OpError::bad_request(
                "days_ago is one at least: nothing in the story is later than its telling",
            ));
        }
        let at = self.at.as_bytes();
        let fits = at.len() == 5
            && at[2] == b':'
            && self.at[..2].parse::<u8>().is_ok_and(|hour| hour < 24)
            && self.at[3..].parse::<u8>().is_ok_and(|minute| minute < 60);
        if !fits {
            return Err(OpError::bad_request("at is a time of day, HH:MM in UTC"));
        }
        conn.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', ?1, ?2, ?3, ?4)",
            params![
                built_on,
                format!("-{} days", self.days_ago),
                format!("+{} hours", &self.at[..2]),
                format!("+{} minutes", &self.at[3..]),
            ],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot place a step in time: {e}")))
    }
}

/// Every column of every table a time could be stamped in: all of them but
/// the ledger's.
fn stamped_columns(conn: &Connection) -> Result<Vec<(String, String)>, OpError> {
    let read = |e: rusqlite::Error| OpError::internal(format!("cannot read the schema: {e}"));
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name <> 'meta' ORDER BY name",
        )
        .and_then(|mut stmt| stmt.query_map([], |row| row.get(0))?.collect())
        .map_err(read)?;
    let mut columns = Vec::new();
    for table in tables {
        let names: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .and_then(|mut stmt| stmt.query_map([&table], |row| row.get(0))?.collect())
            .map_err(read)?;
        columns.extend(names.into_iter().map(|name| (table.clone(), name)));
    }
    Ok(columns)
}

/// Refuse a library that kept anything of the day it was built: a time
/// nothing moved back, or a Version no longer named for its content.
fn refuse_what_its_build_day_left(
    conn: &Connection,
    stamped: &[(String, String)],
    built_on: &str,
) -> Result<(), OpError> {
    let count = |sql: &str, value: &[&dyn rusqlite::ToSql]| -> Result<i64, OpError> {
        conn.query_row(sql, value, |row| row.get(0))
            .map_err(|e| OpError::internal(format!("cannot look over the Sample Library: {e}")))
    };
    for (table, column) in stamped {
        let left = count(
            &format!(
                "SELECT COUNT(*) FROM \"{table}\" WHERE typeof(\"{column}\") = 'text' \
                 AND \"{column}\" GLOB '{A_TIME_GLOB}' AND \"{column}\" >= ?1"
            ),
            &[&built_on],
        )?;
        if left > 0 {
            return Err(OpError::internal(format!(
                "the Sample Library holds {left} time(s) in {table}.{column} from the day it \
                 was built, which no step moved back"
            )));
        }
    }
    let misnamed = count(
        "SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content)",
        &[],
    )?;
    if misnamed > 0 {
        return Err(OpError::internal(format!(
            "the Sample Library holds {misnamed} Version(s) under an id that is not the \
             fingerprint of its content"
        )));
    }
    Ok(())
}

/// The time now, as Kamosu writes one.
fn now(conn: &Connection) -> Result<String, OpError> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
        row.get(0)
    })
    .map_err(|e| OpError::internal(format!("cannot read the time: {e}")))
}

/// Move every time stamped `within` these two, inclusive, by the distance
/// from `from` to `to`, keeping how far apart they were: what one step wrote
/// now, it wrote on its own day. A time a step set ahead of itself, a hold's
/// deadline, lies past the step's end and is left to run.
fn move_back(
    conn: &Connection,
    stamped: &[(String, String)],
    within: (&str, &str),
    from: &str,
    to: &str,
) -> Result<(), OpError> {
    let by: f64 = conn
        .query_row(
            "SELECT (julianday(?1) - julianday(?2)) * 86400.0",
            params![to, from],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot measure a step's distance: {e}")))?;
    let by = format!("{by:+.3} seconds");
    for (table, column) in stamped {
        conn.prepare_cached(&format!(
            "UPDATE \"{table}\" SET \"{column}\" = strftime('%Y-%m-%dT%H:%M:%fZ', \"{column}\", ?1) \
             WHERE typeof(\"{column}\") = 'text' AND \"{column}\" GLOB '{A_TIME_GLOB}' \
               AND \"{column}\" BETWEEN ?2 AND ?3"
        ))
        .and_then(|mut stmt| stmt.execute(params![by, within.0, within.1]))
        .map_err(|e| OpError::internal(format!("cannot move {table}.{column} back: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_named_in_the_story_is_its_bytes_in_base64() {
        use base64::Engine;
        let told = Told::default();
        let input = json!({ "data": { "$file": "photographs/tarte-tatin/photo.jpg" } });
        let resolved = told.resolved(input).unwrap();
        let bytes = Files::get("photographs/tarte-tatin/photo.jpg")
            .unwrap()
            .data;
        assert_eq!(
            resolved["data"],
            json!(base64::engine::general_purpose::STANDARD.encode(bytes))
        );
    }

    #[test]
    fn a_text_named_in_the_story_is_its_words() {
        let told = Told::default();
        let resolved = told
            .resolved(json!({ "text": { "$text": "originals/sunny-side-shakshuka.txt" } }))
            .unwrap();
        let words = Files::get("originals/sunny-side-shakshuka.txt")
            .unwrap()
            .data;
        assert_eq!(
            resolved["text"],
            json!(String::from_utf8(words.to_vec()).unwrap())
        );
    }

    #[test]
    fn a_file_the_story_does_not_carry_fails_the_step() {
        let told = Told::default();
        let refused = told
            .resolved(json!({ "data": { "$file": "originals/nowhere.jpg" } }))
            .unwrap_err();
        assert!(
            refused
                .message
                .contains("sample-library/originals/nowhere.jpg"),
            "{}",
            refused.message
        );
    }
}
