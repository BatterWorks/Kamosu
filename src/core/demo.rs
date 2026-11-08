//! A Demo (ADR 0052): an instance anyone may open without signing in, where
//! every visitor is the Sample Library's Camille and only cooking, planning
//! and shopping save.
//!
//! What a Demo refuses of the Catalogue is each declaration's `in_demo` mark,
//! met in `Core::execute`. What lives here is the rest: the way in, the
//! refusal every route outside the Catalogue gives, and the Sample Library
//! poured in when the Demo starts and every day at 04:00 UTC (#404).

use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::*;
use crate::sample_library;

/// Whether this Core serves a Demo. Read once from the environment before
/// serving (`config::DEMO`), and never changed while it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Ordinary,
    Demo,
}

/// The Sample Library's cook every visitor arrives as: an Operator, so every
/// screen opens, and refused everything an Operator could change (#405).
pub const DEMO_VISITOR: &str = "Camille";

/// The Secret of the one Access Key a Demo publishes, read-only and
/// Camille's, so a visitor's own agent can read the Sample Library at the MCP
/// door. It is the one Secret Kamosu does not mint at random, and it names
/// nothing on an instance that is no Demo: only the Sample Library's build
/// gives a key this Secret (#404).
pub const DEMO_ACCESS_KEY: &str =
    "657de3ec20f4c523e9a990805d934780227c455645e97181e00d64dcad496cce";

/// Where under the data directory a Demo builds its Sample Library, built
/// again at every start so an upgrade brings its own.
const SAMPLE_LIBRARY_DIR: &str = "sample-library";

/// What an instance keeps of its own when its library is put back: the
/// ledger, and Meaning Search's terms and index. The index is derived and
/// brings itself level with whatever library it finds; one put back the
/// next day has the same ids, so only what changed is embedded again.
const THE_INSTANCES_OWN: [&str; 4] = [
    "meta",
    "meaning_search",
    "meaning_vectors",
    "meaning_nothing_to_embed",
];

/// The hour of the day, in UTC, a Demo puts its Sample Library back.
const CLEANING_HOUR: u64 = 4;

/// How many visitors may arrive in a minute, across the whole instance. Each
/// arrival is a Session and a line in the Security Record until the cleaning
/// (#404), and nothing else stops a script from arriving forever. Five a
/// second is more people than a Demo has ever been sent at once.
const ARRIVALS_A_MINUTE: usize = 300;

/// When the last visitors arrived, the oldest first, no more than a minute's.
#[derive(Default)]
pub(super) struct Arrivals(std::sync::Mutex<VecDeque<Instant>>);

impl Arrivals {
    /// Count one more arrival, or say how many seconds until there is room.
    fn admit(&self, now: Instant) -> Result<(), u64> {
        let minute = Duration::from_secs(60);
        let mut arrived = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while arrived
            .front()
            .is_some_and(|first| now.duration_since(*first) >= minute)
        {
            arrived.pop_front();
        }
        if arrived.len() >= ARRIVALS_A_MINUTE {
            let first = *arrived.front().expect("a full minute has a first arrival");
            let wait = minute.saturating_sub(now.duration_since(first));
            return Err(wait.as_secs().max(1));
        }
        arrived.push_back(now);
        Ok(())
    }
}

/// What a Demo answers whatever it refuses, at both Doors and on every route
/// outside the Catalogue. The interface says it in its own words (#407).
pub fn refused_in_demo() -> OpError {
    OpError::refused_because(
        "demo",
        "This is a demo of Kamosu, where only cooking, planning and shopping are saved. \
         Install your own Kamosu to keep everything else.",
    )
}

/// Fail every Job still waiting from before this instance became a Demo
/// whose Operation a Demo refuses, before the waiting Jobs are sent on at
/// startup: a Backup or a fetched link asked for the minute before demo mode
/// went on would otherwise run inside it. An agent's Job is left waiting, as
/// no Runner is answered in a Demo.
pub(super) fn fail_waiting_jobs_a_demo_refuses(db: &Db) -> Result<usize, OpError> {
    db.with_conn(|conn| {
        let waiting: Vec<(String, String)> = conn
            .prepare(
                "SELECT id, operation FROM jobs
                 WHERE status = 'queued' AND id NOT IN (SELECT job_id FROM agent_jobs)",
            )
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .map_err(|e| OpError::internal(format!("cannot list waiting Jobs: {e}")))?;
        let mut failed = 0;
        for (id, operation) in waiting {
            let refused =
                catalogue::find(&operation).is_none_or(|op| op.in_demo != InDemo::GoesThrough);
            if !refused {
                continue;
            }
            failed += conn
                .execute(
                    "UPDATE jobs SET status = 'failed', error = ?2, error_code = -32602,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                     WHERE id = ?1 AND status = 'queued'",
                    params![id, refused_in_demo().message],
                )
                .map_err(|e| OpError::internal(format!("cannot fail a waiting Job: {e}")))?;
        }
        Ok(failed)
    })
}

/// Start a Demo. Its Sample Library is built, the Core started, and the
/// library poured in before the Demo answers anyone, then put back every day
/// at 04:00 UTC (#404). A Demo whose library cannot be built does not start.
///
/// `public_address` is the one `KAMOSU_DEMO_PUBLIC_ADDRESS` gives, stored
/// over whatever was there at every start (#468). With none, the Demo keeps
/// the address it held.
pub fn start_demo(
    db: Arc<Db>,
    analytics: Option<crate::config::Analytics>,
    public_address: Option<&str>,
) -> Result<Arc<Core>, OpError> {
    let built = sample_library::build(&db.data_dir().join(SAMPLE_LIBRARY_DIR))?;
    let core = Core::start_as(db, Mode::Demo);
    let _ = core.sample_library.set(built);
    if let Some(analytics) = analytics {
        let _ = core.analytics.set(analytics);
    }
    core.open_with_sample_library(public_address)?;
    spawn_daily_cleaning(core.clone());
    Ok(core)
}

/// The address a Demo started as the release checklist starts one is given:
/// a stand-in, since the check opens no link.
const CHECKED_ADDRESS: &str = "https://demo.kamosu.example";

/// The release checklist's look at a built Sample Library as a Demo shows it
/// (#468): poured into a Demo's database in `dir`, emptied first, opened as
/// `start_demo` opens one with an address given, and every live Share Link
/// read as the share screen reads it. Answers how many there are, and fails
/// on one with no link to copy, naming its recipe's Branch.
pub fn links_a_demo_shows(
    built: &sample_library::Built,
    dir: &std::path::Path,
) -> Result<usize, OpError> {
    let failed = |e: std::io::Error| {
        OpError::internal(format!("cannot open a Demo in {}: {e}", dir.display()))
    };
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(failed)?;
    }
    std::fs::create_dir_all(dir).map_err(failed)?;
    let core = Core::open_to_build_sample_library(Arc::new(Db::open(dir)?));
    let _ = core.sample_library.set(built.clone());
    core.open_with_sample_library(Some(CHECKED_ADDRESS))?;
    let live: Vec<(String, String)> = core.db().with_conn(|conn| {
        conn.prepare("SELECT branch_id, shared_by FROM share_links WHERE ended_at IS NULL")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .map_err(|e| OpError::internal(format!("cannot list the Share Links: {e}")))
    })?;
    for (branch_id, shared_by) in &live {
        let shown = core.get_share_link(shared_by, branch_id)?;
        if shown["url"].is_null() {
            return Err(OpError::internal(format!(
                "a Demo started at {CHECKED_ADDRESS} shows no link to copy for the recipe \
                 on Branch {branch_id}: {shown}"
            )));
        }
    }
    Ok(live.len())
}

/// The cleaning, which waits for the next 04:00 UTC and puts the library
/// back. One that fails leaves yesterday's library standing and says so.
///
/// It looks at the clock at least once a minute rather than sleeping the
/// whole wait at once, since a sleep does not count the hours a host spends
/// suspended. A clock that jumps forward past 04:00 has the cleaning run as
/// soon as it is seen; one set back counts the wait again from the new time.
fn spawn_daily_cleaning(core: Arc<Core>) {
    std::thread::spawn(move || {
        let mut next: Option<SystemTime> = None;
        loop {
            let now = SystemTime::now();
            let soonest = now + until_the_cleaning(now);
            let due = next.map_or(soonest, |due| due.min(soonest));
            next = Some(due);
            if let Ok(wait) = due.duration_since(now)
                && !wait.is_zero()
            {
                std::thread::sleep(wait.min(Duration::from_secs(60)));
                continue;
            }
            match core.put_back_sample_library() {
                Ok(()) => tracing::info!("the Demo's Sample Library is back as it was built"),
                Err(err) => tracing::error!("cannot put the Demo's Sample Library back: {err}"),
            }
            next = Some(soonest);
        }
    });
}

/// How long from `now` until the next cleaning.
fn until_the_cleaning(now: SystemTime) -> Duration {
    let of_day = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() % 86_400;
    let at = CLEANING_HOUR * 3600;
    Duration::from_secs(if of_day < at {
        at - of_day
    } else {
        86_400 - of_day + at
    })
}

/// Pour the built library attached as `sample` into this one, inside the
/// transaction the caller opened. Every table but the instance's own is
/// emptied and filled again, every time in it moved forward `by` days, and
/// the Meal Plan with them from `built_on` to `today`. The pour
/// keeps the Sessions used in the last day whose Person is still there, the
/// public address, and, closed, any Person the instance's own state names.
fn pour(conn: &Connection, by: i64, built_on: &str, today: &str) -> Result<(), OpError> {
    let failed =
        |e: rusqlite::Error| OpError::internal(format!("cannot put the Sample Library back: {e}"));
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM main.sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .and_then(|mut stmt| stmt.query_map([], |row| row.get(0))?.collect())
        .map_err(failed)?;
    let tables: Vec<String> = tables
        .into_iter()
        .filter(|table| !THE_INSTANCES_OWN.contains(&table.as_str()))
        .collect();
    let public_address: Option<String> = conn
        .query_row(
            "SELECT public_address FROM main.instance_setup WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(failed)?
        .flatten();
    // A visitor holding a page at 04:00 notices nothing: their Session goes
    // on, since the library has the ids it had (#404). The rest end.
    //
    // Whoever accepted Meaning Search's terms before demo mode went on is
    // kept so the acceptance still names somebody, and closed as a deleted
    // account is, with no password, no Session and a place in no list. It is
    // closed before it goes back, since an open account of the same name as
    // one of the story's cooks could not stand beside it.
    conn.execute_batch(
        "CREATE TEMP TABLE kept_sessions AS SELECT * FROM main.sessions \
           WHERE revoked = 0 \
             AND coalesce(last_used_at, created_at) \
                 >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-1 day') \
             AND person_id IN (SELECT id FROM sample.people); \
         CREATE TEMP TABLE kept_people AS SELECT * FROM main.people \
           WHERE id IN (SELECT accepted_by FROM main.meaning_search \
                        UNION SELECT declined_by FROM main.meaning_search) \
             AND id NOT IN (SELECT id FROM sample.people); \
         UPDATE temp.kept_people SET disabled = 1, deleted = 1, password_hash = NULL;",
    )
    .map_err(failed)?;
    for table in &tables {
        conn.execute(&format!("DELETE FROM main.\"{table}\""), [])
            .map_err(failed)?;
    }
    for table in &tables {
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM main.pragma_table_info(?1)")
            .and_then(|mut stmt| stmt.query_map([table], |row| row.get(0))?.collect())
            .map_err(failed)?;
        let names = columns
            .iter()
            .map(|column| format!("\"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let moved = columns
            .iter()
            .map(|column| {
                format!(
                    "CASE WHEN typeof(\"{column}\") = 'text' AND \"{column}\" GLOB '{}' \
                     THEN strftime('%Y-%m-%dT%H:%M:%fZ', \"{column}\", ?1) ELSE \"{column}\" END",
                    sample_library::A_TIME_GLOB
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        conn.execute(
            &format!(
                "INSERT INTO main.\"{table}\" ({names}) SELECT {moved} FROM sample.\"{table}\""
            ),
            params![format!("{by:+} days")],
        )
        .map_err(failed)?;
    }
    meal_plan::move_poured_plan(conn, built_on, today).map_err(failed)?;
    conn.execute_batch(
        "INSERT INTO main.sessions SELECT * FROM temp.kept_sessions; \
         INSERT INTO main.people SELECT * FROM temp.kept_people; \
         DROP TABLE temp.kept_sessions; DROP TABLE temp.kept_people;",
    )
    .map_err(failed)?;
    // The address is the instance's own, never the story's: it is which
    // Origin a browser's request must come from and whether one came over
    // HTTPS (#368), so a Demo is given it, by `KAMOSU_DEMO_PUBLIC_ADDRESS`
    // or by its Operator before demo mode went on.
    conn.execute(
        "UPDATE main.instance_setup SET public_address = ?1 WHERE singleton = 1",
        params![public_address],
    )
    .map_err(failed)?;
    Ok(())
}

/// A file a Demo's reset found no row naming, and the row that would keep
/// it: a Photograph's or Display Copy's hash, or an Original's Job.
enum Unnamed {
    Photograph {
        hash: String,
        path: std::path::PathBuf,
    },
    Original {
        job_id: String,
        path: std::path::PathBuf,
    },
}

/// Every file in `dir` whose row is not among `named`, each as `make`
/// writes it from that row's key. `key_of` reads the key out of a file's
/// name; a name it reads none from is no file of Kamosu's, and goes too.
fn unnamed_in(
    dir: &std::path::Path,
    key_of: impl Fn(&str) -> Option<&str>,
    named: &HashSet<String>,
    make: impl Fn(String, std::path::PathBuf) -> Unnamed,
    into: &mut Vec<Unnamed>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let key = key_of(&name);
        if !key.is_some_and(|key| named.contains(key)) {
            into.push(make(key.unwrap_or(&name).to_owned(), entry.path()));
        }
    }
}

/// Copy every file in `from` that `to` does not hold yet. Photographs and
/// Originals are named by what they are, so one already there is the same.
fn copy_missing(from: &std::path::Path, to: &std::path::Path) -> Result<(), OpError> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    let failed = |e: std::io::Error| {
        OpError::internal(format!(
            "cannot copy the Sample Library's files into {}: {e}",
            to.display()
        ))
    };
    std::fs::create_dir_all(to).map_err(failed)?;
    for entry in entries {
        let entry = entry.map_err(failed)?;
        let into = to.join(entry.file_name());
        if entry.file_type().map_err(failed)?.is_file() && !into.exists() {
            std::fs::copy(entry.path(), &into).map_err(failed)?;
        }
    }
    Ok(())
}

impl Core {
    /// The Core that builds a Demo's Sample Library (#404): an ordinary one,
    /// with no Job lanes and no Door, so nothing a Demo refuses stops it and
    /// no visitor can reach it. It keeps the Security Record's lines out of
    /// the log, since its cooks' sign-ins are the story's.
    pub(crate) fn open_to_build_sample_library(db: Arc<Db>) -> Core {
        Core {
            says_security_lines: false,
            ..Core::open(db)
        }
    }

    /// What a Demo does before it answers anyone: store the address it was
    /// given, if any, and pour its Sample Library in, which keeps it.
    fn open_with_sample_library(&self, public_address: Option<&str>) -> Result<(), OpError> {
        if let Some(address) = public_address {
            self.keep_public_address_given_at_start(address)?;
        }
        self.put_back_sample_library()
    }

    /// Put the Sample Library back as it was built, its dates moved forward
    /// to today: what a Demo does at startup and every day at 04:00 UTC.
    pub fn put_back_sample_library(&self) -> Result<(), OpError> {
        let today: String = self.db().with_conn(|conn| {
            conn.query_row("SELECT date('now')", [], |row| row.get(0))
                .map_err(|e| OpError::internal(format!("cannot read today's date: {e}")))
        })?;
        self.put_back_sample_library_on(&today)
    }

    /// The same, as if today were `today`, a `YYYY-MM-DD` date: every time in
    /// the library moves forward by the whole days since it was built, so a
    /// dinner cooked at seven is still cooked at seven.
    pub fn put_back_sample_library_on(&self, today: &str) -> Result<(), OpError> {
        let built = self
            .sample_library
            .get()
            .ok_or_else(|| OpError::internal("this Core holds no Sample Library"))?;
        // The files first, so that no row ever names one not there yet.
        let data_dir = self.data_dir();
        copy_missing(
            &photographs::photographs_dir(built.dir()),
            &photographs::photographs_dir(&data_dir),
        )?;
        copy_missing(&originals_dir(built.dir()), &originals_dir(&data_dir))?;
        let path = built.database_path();
        self.db().with_conn(|conn| {
            let failed = |e: rusqlite::Error| {
                OpError::internal(format!("cannot put the Sample Library back: {e}"))
            };
            conn.execute(
                "ATTACH DATABASE ?1 AS sample",
                [path.to_string_lossy().as_ref()],
            )
            .map_err(failed)?;
            let poured = (|| {
                let (built_on, by): (String, i64) = conn
                    .query_row(
                        "SELECT value, CAST(round(julianday(?1) - julianday(value)) AS INTEGER) \
                         FROM sample.meta WHERE key = ?2",
                        params![today, sample_library::BUILT_ON],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(failed)?;
                conn.execute_batch("BEGIN IMMEDIATE; PRAGMA defer_foreign_keys = ON;")
                    .map_err(failed)?;
                // A COMMIT refused, by a foreign key the pour left broken,
                // leaves the transaction open on the one connection every
                // request shares, so it is rolled back like any failure.
                let committed = pour(conn, by, &built_on, today)
                    .and_then(|()| conn.execute_batch("COMMIT").map_err(failed));
                if committed.is_err() {
                    let _ = conn.execute_batch("ROLLBACK");
                }
                committed
            })();
            let _ = conn.execute_batch("DETACH DATABASE sample");
            poured
        })?;
        self.forget_files_no_row_names()
    }

    /// Delete the Photographs, Display Copies and Originals no row names
    /// any more: the library a Demo held before it was built again, or the
    /// ordinary library it replaced. The sweeps delete only what a row names.
    fn forget_files_no_row_names(&self) -> Result<(), OpError> {
        let unnamed = self.files_no_row_names()?;
        self.forget_unnamed_files(&unnamed)
    }

    /// The files no row names, read from a list of what the rows name taken
    /// under the database lock. A visitor's upload may record a row as soon
    /// as the lock is let go, so this is what to ask about, never what to
    /// delete (#556).
    fn files_no_row_names(&self) -> Result<Vec<Unnamed>, OpError> {
        let (photographs, originals) = self.db().with_conn(|conn| {
            let names = |sql: &str| -> Result<HashSet<String>, OpError> {
                conn.prepare(sql)
                    .and_then(|mut stmt| stmt.query_map([], |row| row.get(0))?.collect())
                    .map_err(|e| OpError::internal(format!("cannot list what is kept: {e}")))
            };
            Ok((
                names("SELECT hash FROM photographs")?,
                names("SELECT job_id FROM agent_jobs")?,
            ))
        })?;
        let data_dir = self.data_dir();
        let mut unnamed = Vec::new();
        let photograph = |hash, path| Unnamed::Photograph { hash, path };
        unnamed_in(
            &photographs::photographs_dir(&data_dir),
            |file| file.strip_suffix(".webp"),
            &photographs,
            photograph,
            &mut unnamed,
        );
        unnamed_in(
            &photographs::display_dir(&data_dir),
            |file| file.split_once('-').map(|(hash, _)| hash),
            &photographs,
            photograph,
            &mut unnamed,
        );
        // A Job's later pages and what Settings shows of it are named after
        // the Job too, before a dot (#463).
        unnamed_in(
            &originals_dir(&data_dir),
            |file| Some(file.split_once('.').map_or(file, |(job_id, _)| job_id)),
            &originals,
            |job_id, path| Unnamed::Original { job_id, path },
            &mut unnamed,
        );
        Ok(unnamed)
    }

    /// Delete each of `unnamed` that no row names now, under the database
    /// lock (#556). `record_photograph` writes a Photograph's file and its
    /// row under that lock, and an Original is kept in the transaction that
    /// records its Job, so a file whose row came after the list was taken
    /// is met here with its row and kept. Asked outside the lock, a picture
    /// a visitor uploaded at 04:00 would keep its row and lose its bytes.
    fn forget_unnamed_files(&self, unnamed: &[Unnamed]) -> Result<(), OpError> {
        if unnamed.is_empty() {
            return Ok(());
        }
        self.db().with_conn(|conn| {
            let named = |sql: &str, key: &str| -> Result<bool, OpError> {
                conn.query_row(sql, [key], |_| Ok(()))
                    .optional()
                    .map(|row| row.is_some())
                    .map_err(|e| OpError::internal(format!("cannot ask what is kept: {e}")))
            };
            for file in unnamed {
                let (named_now, path) = match file {
                    Unnamed::Photograph { hash, path } => (
                        named("SELECT 1 FROM photographs WHERE hash = ?1", hash)?,
                        path,
                    ),
                    Unnamed::Original { job_id, path } => (
                        named("SELECT 1 FROM agent_jobs WHERE job_id = ?1", job_id)?,
                        path,
                    ),
                };
                if !named_now {
                    let _ = std::fs::remove_file(path);
                }
            }
            Ok(())
        })
    }

    /// The examples a visitor replays the agent's work on (#406): the
    /// Sample Library's, and none on an instance that is no Demo.
    pub fn replays(&self) -> &[sample_library::Replay] {
        self.sample_library
            .get()
            .map_or(&[], sample_library::Built::replays)
    }

    /// When a Runner was last heard from, as a Demo says it: no agent runs
    /// in one, so a Runner that stands has just asked for work, and no Job a
    /// visitor replays says none is answering (#406). Anywhere else, `heard`.
    pub(super) fn heard_in_a_demo(&self, stands: bool, heard: Option<String>) -> Option<String> {
        if self.is_demo() && stands {
            Some(security_record::now_text())
        } else {
            heard
        }
    }

    /// Whether this instance is a Demo.
    pub fn is_demo(&self) -> bool {
        self.mode == Mode::Demo
    }

    /// The Umami this Demo counts its visits with, if it was given one
    /// (#408). Nothing on an instance that is no Demo.
    pub fn analytics(&self) -> Option<&crate::config::Analytics> {
        self.analytics.get()
    }

    /// Refuse, in a Demo, what a route outside the Catalogue would do: sign
    /// in, store a file, hand over a Backup. Asked first, before any work.
    pub(super) fn refuse_in_a_demo(&self) -> Result<(), OpError> {
        if self.is_demo() {
            return Err(refused_in_demo());
        }
        Ok(())
    }

    /// A visitor arrives in a Demo: a new Session, as Camille, with no
    /// password asked. On an instance that is no Demo there is nothing at
    /// this address, so it opens no way in anywhere else. The web door is its
    /// only caller, and hands the Secret to the browser as a cookie no script
    /// reads.
    pub fn arrive_in_demo(&self, session_name: &str) -> Result<Value, OpError> {
        if !self.is_demo() {
            return Err(OpError::not_found("nothing is at this address"));
        }
        let session_name = required_text(session_name, "session_name")?;
        let visitor: Option<String> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM people WHERE name = ?1 AND disabled = 0 AND deleted = 0",
                params![DEMO_VISITOR],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read the Demo's visitor: {e}")))
        })?;
        let Some(person_id) = visitor else {
            return Err(OpError::bad_request(format!(
                "This demo has nobody named {DEMO_VISITOR} to let you in as: its Sample \
                 Library is not loaded"
            )));
        };
        if let Err(seconds) = self.arrivals.admit(Instant::now()) {
            return Err(OpError::wait(
                seconds,
                "Many people are arriving at this demo at once. Try again in a moment.",
            ));
        }
        let session = self.mint_session(&person_id, session_name, "demo")?;
        Ok(json!({ "session_id": session.id, "_session_secret": session.secret }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_kamosu() -> (tempfile::TempDir, Core) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(crate::db::Db::open(dir.path()).unwrap());
        (dir, Core::open(db))
    }

    /// A Demo's reset lists what the rows name, lets the lock go, and comes
    /// back for the files (#556). A picture a visitor uploads in between
    /// keeps its bytes. Run in the order the race runs it, one step at a time.
    #[test]
    fn a_picture_recorded_while_a_demo_resets_keeps_its_file() {
        let (_dir, core) = a_kamosu();
        let bytes = b"a picture a visitor adds at four in the morning";
        let hash = photographs::hash_bytes(bytes);
        let file = photographs::photograph_path(&core.data_dir(), &hash);
        let display =
            photographs::display_path(&core.data_dir(), &hash, photographs::DisplaySize::Card);
        std::fs::create_dir_all(photographs::photographs_dir(&core.data_dir())).unwrap();
        std::fs::create_dir_all(photographs::display_dir(&core.data_dir())).unwrap();
        std::fs::write(&file, bytes).unwrap();
        std::fs::write(&display, bytes).unwrap();

        // The reset has listed what it keeps: no row names this picture yet.
        let unnamed = core.files_no_row_names().unwrap();
        assert_eq!(unnamed.len(), 2);
        // The visitor's upload lands.
        core.record_photograph(&hash, bytes).unwrap();
        // The reset comes back for the files.
        core.forget_unnamed_files(&unnamed).unwrap();
        assert!(file.exists(), "a recorded picture lost its bytes");
        assert!(display.exists(), "a recorded picture lost its Display Copy");
    }

    /// The same for an Original: one kept with its Job after the reset
    /// listed what it keeps stays on the disk (#556).
    #[test]
    fn an_original_recorded_while_a_demo_resets_keeps_its_file() {
        let (_dir, core) = a_kamosu();
        let dir = originals_dir(&core.data_dir());
        std::fs::create_dir_all(&dir).unwrap();
        let job_id = "a-job-a-visitor-sent";
        let file = dir.join(job_id);
        let page = dir.join(format!("{job_id}.2"));
        std::fs::write(&file, b"a pasted recipe").unwrap();
        std::fs::write(&page, b"its second page").unwrap();
        let leftover = dir.join("a-job-the-old-library-had");
        std::fs::write(&leftover, b"from before the reset").unwrap();

        let unnamed = core.files_no_row_names().unwrap();
        assert_eq!(unnamed.len(), 3);
        core.db()
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO jobs (id, operation, input) VALUES (?1, ?2, '{}')",
                    params![job_id, jobs::AGENT_OPERATION],
                )
                .and_then(|_| {
                    conn.execute(
                        "INSERT INTO agent_jobs (job_id, original, original_pages)
                         VALUES (?1, 'text', 2)",
                        params![job_id],
                    )
                })
                .map_err(|e| OpError::internal(e.to_string()))
            })
            .unwrap();
        core.forget_unnamed_files(&unnamed).unwrap();
        assert!(file.exists(), "a recorded Original lost its first page");
        assert!(page.exists(), "a recorded Original lost its second page");
        assert!(!leftover.exists(), "an Original no row names was kept");
    }

    #[test]
    fn arrivals_past_a_minutes_worth_wait_until_the_oldest_is_a_minute_old() {
        let arrivals = Arrivals::default();
        let start = Instant::now();
        for _ in 0..ARRIVALS_A_MINUTE {
            assert_eq!(arrivals.admit(start), Ok(()));
        }
        assert_eq!(arrivals.admit(start + Duration::from_secs(20)), Err(40));
        assert_eq!(arrivals.admit(start + Duration::from_secs(60)), Ok(()));
    }

    #[test]
    fn the_cleaning_is_at_the_next_four_in_the_morning_utc() {
        let at = |seconds: u64| UNIX_EPOCH + Duration::from_secs(seconds);
        let day = 20_000 * 86_400;
        assert_eq!(until_the_cleaning(at(day)), Duration::from_secs(4 * 3600));
        assert_eq!(
            until_the_cleaning(at(day + 4 * 3600)),
            Duration::from_secs(86_400)
        );
        assert_eq!(
            until_the_cleaning(at(day + 23 * 3600)),
            Duration::from_secs(5 * 3600)
        );
    }
}
