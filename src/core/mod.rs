//! The Core: where Operations are carried out and where permission is checked.
//! It is reached only through a Door (or the terminal) and knows nothing about
//! how a request arrived.
//!
//! Authorisation lives here, beneath both Doors, keyed on a Credential. A
//! permission check written inside a Door is a bug.
//!
//! This file holds what every caller passes through before any Operation runs:
//! the `Core` itself, startup, `execute` with its permission check, and
//! resolving a Credential. The work behind each Operation lives beside it, one
//! file per area, each adding its methods to `Core` in an `impl` of its own.
//!
//! Four areas are named for what they do rather than what they hold —
//! `carrying_bundles`, `keeping_backups`, `keeping_photographs`,
//! `shopping_list` — because `crate::bundles`, `crate::backups`,
//! `crate::photographs` and `crate::shopping` already exist and the code in
//! those files names them by path.
//!
//! Every area's items are gathered back into this module, so the areas reach
//! one another's helpers through `use super::*` and every `crate::core::X`
//! path still works. How widely each area is re-exported follows what it
//! holds: `pub use` where it has something public, `pub(crate) use` where the
//! widest is crate-wide, and a plain `use` where everything in it is for the
//! other areas only. An area nobody outside it calls into has no line at all.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::{TryRng, rngs::SysRng};
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::backups;
use crate::bundles;
use crate::catalogue::{self, JobKey, JobLane, Kind, Permission};
use crate::crouton;
use crate::db::Db;
use crate::fingerprint::{canonical_json, stored_version};
use crate::jobs::{self, JobProgress, JobRecord};
use crate::language::LANGUAGES;
use crate::photographs;
use crate::sheet;
use crate::shopping;
use crate::units;

mod accounts;
mod carrying_bundles;
mod cookbooks;
mod cooking;
mod imports;
mod keeping_backups;
mod keeping_photographs;
mod kitchens;
mod originals;
mod passkeys;
mod reading_again;
mod readings;
mod recipes;
mod related;
mod runners;
mod search;
mod security_record;
mod share_links;
mod shared;
mod sheets;
mod shelves;
mod shopping_list;
mod tags;
mod translation;

pub use accounts::*;
use cookbooks::*;
pub use cooking::*;
pub use imports::*;
use keeping_backups::*;
use keeping_photographs::*;
use kitchens::*;
pub use originals::*;
pub use readings::*;
use recipes::*;
// What the MCP door's instructions and the save summaries say the window is.
pub use recipes::COLLAPSE_WINDOW_SECONDS;
pub use related::*;
pub use runners::*;
pub use search::*;
pub use security_record::*;
pub(crate) use shared::*;
pub use sheets::*;
use shelves::*;
use tags::*;
use translation::*;

/// What went wrong with an Operation, in words a caller can act on. The Doors map
/// these to their own transports; they never decide them.
#[derive(Debug, Clone)]
pub struct OpError {
    pub kind: ErrorKind,
    pub message: String,
    /// True for exactly one refusal: the Secret presented resolves to nobody,
    /// because it is unknown, revoked or spent. Every other `Unauthorized` is
    /// about what a Person Kamosu *did* recognise may do, and so says nothing
    /// about the Credential itself.
    ///
    /// A Door may act on that distinction — the web door takes back a Session
    /// cookie it set (#91) — but the distinction is drawn here, once, as every
    /// authorisation decision is.
    pub credential_names_nobody: bool,
    /// How many seconds to wait before asking again, on the one refusal that
    /// knows: a sign-in tried while its name is still waiting (#138). The web
    /// door carries it as `retry_after_seconds` and a `Retry-After` header, and
    /// the sign-in screen counts it down.
    pub retry_after_seconds: Option<u64>,
    /// Which refusal this is, for the few a screen says in its own words
    /// rather than the Core's English sentence: a scanned PDF (#176). The web
    /// door carries it as `reason`. Nothing else is keyed on it; the sentence
    /// is still the whole answer at the MCP door.
    pub reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The Credential was presented but does not resolve: unknown, revoked or spent.
    Unauthorized,
    /// No such Operation exists in the Catalogue.
    UnknownOperation,
    /// The input envelope does not fit the Operation's declaration.
    BadRequest,
    /// No such thing — a Job id that names nothing, read by its owner or not at all.
    NotFound,
    /// The work is refused rather than queued: the lane asked for is full.
    /// A stranger may cause work, never work that scales with them (ADR 0032) —
    /// a full line answers *busy* so collapse becomes latency instead.
    Busy,
    /// Kamosu itself failed. Never the caller's fault.
    Internal,
}

impl OpError {
    /// Every refusal but one: it says what went wrong, and nothing about the
    /// Credential that was presented.
    fn of(kind: ErrorKind, message: impl Into<String>) -> Self {
        OpError {
            kind,
            message: message.into(),
            credential_names_nobody: false,
            retry_after_seconds: None,
            reason: None,
        }
    }
    pub fn unauthorized(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::Unauthorized, message)
    }
    /// The one refusal that is about the Secret itself rather than about what
    /// its Person may do: it resolves to nobody. Unknown, revoked and spent all
    /// answer alike, saying nothing about which.
    pub fn credential_names_nobody() -> Self {
        OpError {
            credential_names_nobody: true,
            ..OpError::unauthorized("this Credential does not name anyone")
        }
    }
    pub fn unknown_operation(name: &str) -> Self {
        OpError::of(
            ErrorKind::UnknownOperation,
            format!("no Operation named '{name}' exists in the Catalogue"),
        )
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::BadRequest, message)
    }
    /// A refusal a screen may say in its own words, named by `reason`.
    pub fn refused_because(reason: &'static str, message: impl Into<String>) -> Self {
        OpError {
            reason: Some(reason),
            ..OpError::bad_request(message)
        }
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::NotFound, message)
    }
    pub fn busy() -> Self {
        OpError::of(
            ErrorKind::Busy,
            "Kamosu is busy right now — try again in a moment",
        )
    }
    /// Busy for a known while: this name's sign-in is waiting out a wrong
    /// password, or another try at it is being checked (#138).
    pub fn wait(seconds: u64, message: impl Into<String>) -> Self {
        OpError {
            retry_after_seconds: Some(seconds),
            ..OpError::of(ErrorKind::Busy, message)
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        OpError::of(ErrorKind::Internal, message)
    }

    /// A courteous sentence for the caller; safe to show through either Door.
    pub fn to_sentence(&self) -> String {
        self.message.clone()
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

/// Who is acting: a Person resolved from a Credential. An agent is never a Person
/// — it acts as one, under that Person's Hand.
#[derive(Debug, Clone)]
pub struct Caller {
    pub person_id: String,
    /// True when the Credential came from an Access Key minted read-only.
    /// `Core::execute` refuses every Operation the Catalogue declares `write`
    /// to a caller carrying this (ADR 0031).
    pub read_only: bool,
    /// True when the Credential came from an Access Key or a Job Key rather
    /// than a login Session. `Core::execute` refuses every Operation the
    /// Catalogue declares `session_only` to a caller carrying this, however
    /// unrestricted the Key.
    pub via_access_key: bool,
    /// Whether this Credential acts as an Operator: a Session or a Job Key of
    /// a Person who administers the instance, or that Person's Operator key.
    /// Any other Access Key of theirs acts as an ordinary Person (#369).
    pub is_operator: bool,
    /// The Access Key that resolved this Credential, if any — never set for a
    /// login Session. Local only: a Version records it (GLOSSARY.md, "Version")
    /// so it never travels in a Bundle. No Operation surfaces it back yet —
    /// that is for whichever future ticket reads a Version's full detail.
    pub access_key_id: Option<String>,
    /// The login Session that resolved this Credential, if any — never set
    /// for an Access Key. What lets the Sessions list say which one is in
    /// your hand (#114).
    pub session_id: Option<String>,
    /// The Job this Credential is the Job Key of, if it is one (ADR 0048). The
    /// Person is the one who asked for that Job. `Core::execute` refuses a
    /// caller carrying this every Operation the Catalogue does not mark for a
    /// Job Key, and holds what it writes to that Job's one recipe. Every
    /// route outside the Catalogue refuses it too, but storing a Photograph
    /// and fetching its own Job's Original.
    pub job_id: Option<String>,
    /// True when this one call asked to act as an Operator, with
    /// `as_operator`, and may (#302, ADR 0049). Only then does an Operator
    /// reach a recipe beyond what they see as a Person.
    pub as_operator: bool,
}

/// A Runner, known by the Runner Key it presented. It is no Person and acts as
/// none (ADR 0048), which is why it is no `Caller`: an Operation written for a
/// Person takes a `Caller`, and no Runner Key ever becomes one.
#[derive(Debug, Clone)]
pub struct RunnerCaller {
    pub runner_key_id: String,
}

/// What a Secret came to before a request's body was read (#368): no Secret
/// at all, or one that names somebody. One that names nobody is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Knock {
    Stranger,
    Somebody,
}

/// Who is asking for an Operation: nobody Kamosu knows, a Person, or a Runner.
/// Exactly one of the three, which is what keeps a Runner from ever standing
/// where a Person or a stranger is expected.
#[derive(Debug, Clone)]
pub enum Asker {
    /// No Credential was presented.
    Nobody,
    Person(Caller),
    Runner(RunnerCaller),
}

/// Whether a call asks to act as an Operator (#302).
pub(crate) fn asks_as_operator(input: &Value) -> bool {
    input.get("as_operator").and_then(Value::as_bool) == Some(true)
}

/// What a Job Key is answered on every route outside the Catalogue: as if the
/// thing were not there, so an agent can tell a refusal from a fault.
fn nothing_here_for_a_job_key() -> OpError {
    OpError::not_found("nothing is at this address")
}

/// A Runner Key was presented where only a Person may act.
fn runner_key_out_of_place(what: &str) -> OpError {
    OpError::unauthorized(format!(
        "{what} is not for a Runner Key, which calls the Runner's own Operations and nothing else"
    ))
}

#[derive(Clone)]
struct Session {
    id: String,
    secret: String,
}

/// One row `lookup_credential` can find: `(person_id, read_only, is_session,
/// is_operator, access_key_id, session_id)`.
type CredentialLookup = (String, bool, bool, bool, Option<String>, Option<String>);

/// One asking of an Operation: who is acting, and (when the Operation runs as a
/// Job) the handle through which the running work reports its progress.
pub struct Invocation {
    pub asker: Asker,
    /// Set only while an Operation declared `Kind::Job` is being carried out.
    pub job: Option<JobProgress>,
}

impl Invocation {
    /// The Person acting, if one is. A Runner is none, and so is nobody.
    pub fn caller(&self) -> Option<&Caller> {
        match &self.asker {
            Asker::Person(caller) => Some(caller),
            Asker::Nobody | Asker::Runner(_) => None,
        }
    }

    /// The Runner asking, if one is: on an Operation declared
    /// `Permission::Runner` and on no other.
    pub fn runner(&self) -> Option<&RunnerCaller> {
        match &self.asker {
            Asker::Runner(runner) => Some(runner),
            Asker::Nobody | Asker::Person(_) => None,
        }
    }
}

/// The Core. Holds the database — the truth (ADR 0003) — and the Job lanes that
/// carry slow work (ADR 0032): one for members, one depth-one lane for strangers.
pub struct Core {
    db: Arc<Db>,
    lanes: jobs::Lanes,
    /// Meaning Search, which on most instances is an empty slot and stays one
    /// (ADR 0029). Nothing branches on that outside the two places that must:
    /// the search itself, and the Operations that turn it on.
    meaning: Arc<crate::meaning::MeaningSearch>,
    /// Whose login is waiting, and the few threads that may hash or check a
    /// password at once (#138).
    passwords: accounts::Passwords,
    /// Passkey exchanges begun and not yet finished (#370).
    ceremonies: passkeys::Ceremonies,
    /// A crowd at the sign-in, counted between the times it is written down
    /// in the Security Record (#371).
    security: security_record::Watch,
    /// The PDFs being read now, each in a program of its own (#373).
    pdf_readers: crate::pdf::Readers,
    /// How long a Runner's hold on a Job lasts unless renewed, in
    /// milliseconds: `jobs::HOLD`, until a test shortens it (#266).
    hold_millis: AtomicU64,
    /// Held while a Job Key creates its Job's recipe (#269), from the check
    /// that the Job has none to the moment the new one is made its own, so
    /// that two creates at once cannot both find it has none.
    job_key_creates: std::sync::Mutex<()>,
    /// The public address as last read from the database, or nothing until
    /// it is first asked for (#368). Every answer asks, to know whether it
    /// came over HTTPS and which `Origin` is the instance's own, and taking
    /// the database lock for each picture and script would put a page load
    /// behind whatever write holds it. Forgotten whenever the address is
    /// stored, so the next ask reads it again.
    public_address: std::sync::RwLock<Option<Option<String>>>,
    /// The seed Home shuffles *never cooked* by, once a test has pinned one
    /// (#151). Compiled only under `test-jobs`: a real instance draws a fresh
    /// seed each time Home is asked for, and carries nothing to pin it with.
    #[cfg(feature = "test-jobs")]
    shelf_seed: std::sync::Mutex<Option<u64>>,
}

impl Core {
    pub fn open(db: Arc<Db>) -> Core {
        // Terminal commands open a Core without Job workers; they never ask for
        // slow work. A Job asked for here would be refused as busy.
        let (lanes, _) = jobs::lanes();
        let meaning = Arc::new(crate::meaning::MeaningSearch::new(
            db.data_dir().to_path_buf(),
        ));
        Core {
            db,
            lanes,
            meaning,
            passwords: accounts::Passwords::default(),
            ceremonies: Default::default(),
            security: Default::default(),
            pdf_readers: Default::default(),
            hold_millis: AtomicU64::new(jobs::HOLD.as_millis() as u64),
            job_key_creates: Default::default(),
            public_address: Default::default(),
            #[cfg(feature = "test-jobs")]
            shelf_seed: Default::default(),
        }
    }

    /// Open the database *and* start carrying Jobs: both Doors share this entry.
    /// The lanes exist from the moment the instance serves — and anything a
    /// previous process left behind is recovered before the first answer goes out.
    pub fn start(db: Arc<Db>) -> Arc<Core> {
        let (lanes, receivers) = jobs::lanes();
        jobs::recover_at_startup(&db, &lanes);
        let meaning = Arc::new(crate::meaning::MeaningSearch::new(
            db.data_dir().to_path_buf(),
        ));
        let core = Core {
            db,
            lanes,
            meaning,
            passwords: accounts::Passwords::default(),
            ceremonies: Default::default(),
            security: Default::default(),
            pdf_readers: Default::default(),
            hold_millis: AtomicU64::new(jobs::HOLD.as_millis() as u64),
            job_key_creates: Default::default(),
            public_address: Default::default(),
            #[cfg(feature = "test-jobs")]
            shelf_seed: Default::default(),
        };
        core.passwords.prepare();
        let core = Arc::new(core);
        jobs::spawn_workers(core.clone(), receivers);
        spawn_orphan_sweep(core.clone());
        spawn_meaning_search(core.clone());
        spawn_backups(core.clone());
        spawn_hold_watch(core.clone());
        spawn_crowd_watch(core.clone());
        core
    }

    /// Meaning Search as this instance holds it.
    pub fn meaning(&self) -> Arc<crate::meaning::MeaningSearch> {
        self.meaning.clone()
    }

    /// What reads a PDF someone sent, no more than a few at once (#373).
    pub fn pdf_readers(&self) -> &crate::pdf::Readers {
        &self.pdf_readers
    }

    /// The database, shared. An Arc clone keeps Job progress reporting able to
    /// write while the Core itself is borrowed elsewhere.
    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    /// Where Kamosu's truth lives: one SQLite file under the one data directory.
    pub fn database_path(&self) -> std::path::PathBuf {
        self.db.database_path()
    }

    /// Carry out one Operation on behalf of whatever presented itself.
    ///
    /// This is the whole authorisation surface of Kamosu: resolve the Credential,
    /// check the permission the Catalogue declares, then perform. Both Doors call
    /// exactly this and add no checks of their own — that is Parity working.
    pub fn execute(
        &self,
        credential_secret: Option<&str>,
        operation_name: &str,
        input: Value,
    ) -> Result<Value, OpError> {
        let op = catalogue::find(operation_name)
            .ok_or_else(|| OpError::unknown_operation(operation_name))?;

        // Resolving a Credential is not an Operation (it precedes every one), but
        // presenting an unrecognised Secret must fail loudly rather than silently
        // become a stranger.
        let mut asker = match credential_secret.filter(|secret| !secret.is_empty()) {
            None => Asker::Nobody,
            Some(secret) => self.present(secret)?,
        };

        // A Job Key reaches the Operations marked for it and no other, the
        // ones open to anyone included (ADR 0048). It is met first, and the
        // refusal is the one an Operation that does not exist gets. Whether
        // the asker could do it themselves is everything below, which a Job
        // Key passes through as the Person it names.
        if let Asker::Person(caller) = &asker
            && caller.job_id.is_some()
            && op.job_key == JobKey::Never
        {
            return Err(OpError::unknown_operation(op.name));
        }

        // Who is asking, against the level the Operation is declared with. A
        // Runner Key reaches the Runner's own Operations and no other, the
        // ones open to anyone included (ADR 0048), so it is met first.
        let refused = match (&asker, op.permission) {
            (Asker::Runner(_), Permission::Runner) => None,
            (Asker::Runner(_), _) => {
                Some(runner_key_out_of_place(&format!("Operation '{}'", op.name)))
            }
            (_, Permission::Runner) => Some(OpError::unauthorized(format!(
                "Operation '{}' is a Runner's own, and requires a Runner Key",
                op.name
            ))),
            (_, Permission::Public) | (Asker::Person(_), Permission::Person) => None,
            (Asker::Nobody, Permission::Person) => Some(OpError::unauthorized(format!(
                "Operation '{}' requires a Credential naming a Person",
                op.name
            ))),
            (Asker::Person(caller), Permission::Operator) if caller.is_operator => None,
            (_, Permission::Operator) => Some(OpError::unauthorized(format!(
                "Operation '{}' requires a Credential naming an Operator",
                op.name
            ))),
        };
        if let Some(refused) = refused {
            return Err(refused);
        }

        // The read-only filter and the session-only rule both live here, beneath
        // both Doors, alongside the permission check above — a Door adding either
        // check itself would be the same bug as a Door checking permission.
        if let Asker::Person(caller) = &asker {
            if op.write && caller.read_only {
                return Err(OpError::unauthorized(format!(
                    "Operation '{}' writes, and this Credential is a read-only Access Key",
                    op.name
                )));
            }
            if op.session_only && caller.via_access_key {
                return Err(OpError::unauthorized(format!(
                    "Operation '{}' may be performed only by a Person logged in directly, \
                     never by an Access Key",
                    op.name
                )));
            }
        }

        // Shape is checked here, once, against the Operation's own declaration
        // (#85): both Doors inherit it, and a new Operation cannot forget it any
        // more than it can forget to appear at both. It runs after the
        // authorisation checks above so a caller who may not perform an
        // Operation learns nothing about its input, and before any handler, so
        // no handler is ever reached with an envelope the Catalogue does not
        // describe. What it does *not* check is meaning — whether a Branch
        // exists, whether an amount parses — which stays in the handlers.
        crate::schema::validate_input(op.name, &input)?;

        // Acting as an Operator is asked for one call at a time (#302, ADR
        // 0049): an Operator is a Person like any other until a call says
        // `as_operator`. Only an Operator acting themselves may say it, never
        // through a read-only Access Key or a Job Key. Only the Operations
        // that declare it accept it, which the shape check above holds.
        if asks_as_operator(&input) {
            match &mut asker {
                Asker::Person(caller)
                    if caller.is_operator && !caller.read_only && caller.job_id.is_none() =>
                {
                    caller.as_operator = true;
                }
                _ => {
                    return Err(OpError::unauthorized(
                        "as_operator is for an Operator acting themselves, signed in or \
                         through an Access Key that may write",
                    ));
                }
            }
        }

        // What a Job Key writes, it writes on the one recipe its Job is for,
        // and never where that would start a Copy (ADR 0048). Asked here so
        // that no Operation learns about Jobs. It comes after the shape is
        // checked, since it reads what the call sent.
        //
        // A Job Key creating its Job's recipe holds a lock from the check to
        // the moment the new recipe is the Job's own (#269).
        let mut creates_for_job = None;
        let _one_create_at_a_time = match &asker {
            Asker::Person(
                caller @ Caller {
                    job_id: Some(job_id),
                    ..
                },
            ) => {
                let lock = (op.job_key == JobKey::CreatesItsRecipe).then(|| {
                    creates_for_job = Some(job_id.clone());
                    self.job_key_creates
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                });
                self.hold_job_key_write(caller, job_id, op.job_key, &input)?;
                lock
            }
            _ => None,
        };

        let invocation = Invocation { asker, job: None };

        let answered = match op.kind {
            Kind::Immediate => (op.handler)(self, &invocation, input),
            Kind::Job => self.ask_job(op.name, op.job_lane, &invocation, input),
        }?;
        if let Some(job_id) = creates_for_job {
            let branch_id = answered["branch_id"]
                .as_str()
                .ok_or_else(|| OpError::internal("a created recipe came back with no id"))?;
            self.db()
                .with_conn(|conn| jobs::take_its_recipe(conn, &job_id, branch_id))?;
        }
        Ok(answered)
    }

    /// Accept slow work and answer at once with an id. The work itself runs in a
    /// lane: members never wait behind strangers (ADR 0032), except a Job whose
    /// own Catalogue entry declares `JobLane::AlwaysSingle` — the risk there is
    /// the outbound fetch itself, not who asked for it, so it never scales past
    /// one in flight regardless of who is signed in. A full line refuses rather
    /// than grows — the caller is told busy, try again in a moment.
    fn ask_job(
        &self,
        operation_name: &str,
        job_lane: JobLane,
        invocation: &Invocation,
        input: Value,
    ) -> Result<Value, OpError> {
        let job_id = jobs::record(self, operation_name, invocation, input)?;
        let lane = match job_lane {
            // The single depth-one lane, reused rather than duplicated — it
            // is a stranger's lane only in the sense that it was built for
            // "no more than one at a time"; an `AlwaysSingle` Job lands here
            // even with a signed-in Person asking (ADR 0032's #70 exception).
            JobLane::AlwaysSingle => &self.lanes.stranger,
            JobLane::ByCaller if invocation.caller().is_some() => &self.lanes.member,
            JobLane::ByCaller => &self.lanes.stranger,
        };
        if lane.try_send(job_id.clone()).is_err() {
            // Refused outright: the row must not linger as work nobody carries.
            let _ = jobs::forget(self, &job_id);
            return Err(OpError::busy());
        }
        Ok(json!({ "job_id": job_id }))
    }

    /// Read one Job back: its state, progress, and result or failure reason.
    ///
    /// An Import's result is its Report, and one that left a recipe for an
    /// agent is read with that agent's Job as it stands now (#262). Doing it
    /// here is what gives every reader the same Report: `get_job` at both
    /// Doors and the tasks extension.
    pub fn job(&self, job_id: &str) -> Result<Option<JobRecord>, OpError> {
        let Some(mut record) = jobs::read(self, job_id)? else {
            return Ok(None);
        };
        if let Some(report) = record.result.as_mut() {
            self.db()
                .with_conn(|conn| imports::report_with_agent_jobs_as_they_stand(conn, report))?;
        }
        Ok(Some(record))
    }

    /// Every Job one Person has asked for, newest first.
    pub fn jobs_of(&self, person_id: &str) -> Result<Vec<JobRecord>, OpError> {
        jobs::read_of_person(self, person_id)
    }

    /// Cancel a Job, acked either way: one of Kamosu's own while it still
    /// waits in line, an agent's whether it waits or a Runner holds it (#267).
    pub fn cancel_job(&self, job_id: &str) -> Result<bool, OpError> {
        jobs::cancel(self, job_id)
    }

    /// Look up what a Secret's hash names, without marking it used: `(person_id,
    /// read_only, is_session)`, or nothing for an unknown or revoked Secret.
    /// `present` builds on this and additionally records the use;
    /// `operations_listed_to` reads only, so a Door merely listing what a
    /// Key may do never counts as the Key being used.
    fn lookup_credential(&self, hash: &str) -> Result<Option<CredentialLookup>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT access_keys.person_id, access_keys.read_only, 0, people.is_operator AND access_keys.acts_as_operator, access_keys.id, NULL
                   FROM access_keys JOIN people ON people.id = access_keys.person_id
                   WHERE access_keys.secret_hash = ?1 AND access_keys.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 UNION ALL
                 SELECT sessions.person_id, 0, 1, people.is_operator, NULL, sessions.id
                   FROM sessions JOIN people ON people.id = sessions.person_id
                   WHERE sessions.secret_hash = ?1 AND sessions.revoked = 0 AND people.disabled = 0 AND people.deleted = 0
                 LIMIT 1",
                params![hash],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// Whether a Secret names anyone, asked by a Door before it reads a
    /// request's body (#368): somebody gets a body of the size the Door allows
    /// its members, a stranger a small one, and a Secret that names nobody is
    /// answered at once, with the refusal `execute` would give. Nothing is
    /// recorded as used here; the call that follows does that.
    pub fn knock(&self, secret: Option<&str>) -> Result<Knock, OpError> {
        let Some(secret) = secret.filter(|secret| !secret.is_empty()) else {
            return Ok(Knock::Stranger);
        };
        let hash = hash_secret(secret);
        if self.lookup_credential(&hash)?.is_some()
            || self.lookup_runner_key(&hash)?.is_some()
            || self.lookup_job_key(&hash)?.is_some()
        {
            Ok(Knock::Somebody)
        } else {
            Err(OpError::credential_names_nobody())
        }
    }

    /// Resolve a raw Secret to the Person it acts as, for everything that is
    /// not `execute`: the routes outside the Catalogue. A Runner Key acts as no
    /// Person, so it is refused here whatever the route. So is a Job Key,
    /// whose reach is its list in the Catalogue and the two routes opened to
    /// it: `authenticate_to_store_photograph`, and `read_original` for its own
    /// Job's Original.
    fn resolve_credential(&self, secret: &str) -> Result<Caller, OpError> {
        match self.resolve_person(secret)? {
            caller if caller.job_id.is_some() => Err(nothing_here_for_a_job_key()),
            caller => Ok(caller),
        }
    }

    /// The Person a raw Secret acts as on a route outside the Catalogue, a
    /// Job Key's asker included. A Runner Key acts as none.
    fn resolve_person(&self, secret: &str) -> Result<Caller, OpError> {
        match self.present(secret)? {
            Asker::Person(caller) => Ok(caller),
            Asker::Runner(_) => Err(runner_key_out_of_place("this route")),
            Asker::Nobody => Err(OpError::credential_names_nobody()),
        }
    }

    /// Resolve a raw Secret to whoever it names, and record that it was used:
    /// a Person, through a Session, an Access Key or a Job Key, or a Runner,
    /// through its Runner Key. Stored hashed, never in the clear.
    fn present(&self, secret: &str) -> Result<Asker, OpError> {
        let hash = hash_secret(secret);
        match self.lookup_credential(&hash)? {
            Some((person_id, read_only, is_session, is_operator, access_key_id, session_id)) => {
                self.record_use(
                    if is_session {
                        "sessions"
                    } else {
                        "access_keys"
                    },
                    &hash,
                );
                Ok(Asker::Person(Caller {
                    person_id,
                    read_only,
                    via_access_key: !is_session,
                    is_operator,
                    access_key_id,
                    session_id,
                    job_id: None,
                    as_operator: false,
                }))
            }
            None => {
                if let Some(runner_key_id) = self.lookup_runner_key(&hash)? {
                    self.record_use("runner_keys", &hash);
                    return Ok(Asker::Runner(RunnerCaller { runner_key_id }));
                }
                match self.lookup_job_key(&hash)? {
                    // The asker as they stand now, an Operator or not. A Job
                    // Key is no login, so what only a Person logged in
                    // directly may do stays out of its reach whatever its
                    // list comes to hold.
                    Some((person_id, is_operator, job_id)) => Ok(Asker::Person(Caller {
                        person_id,
                        read_only: false,
                        via_access_key: true,
                        is_operator,
                        access_key_id: None,
                        session_id: None,
                        job_id: Some(job_id),
                        as_operator: false,
                    })),
                    // Unknown, revoked or spent: the same answer each way,
                    // saying nothing about which. Marked, so the web door can
                    // take back a Session cookie it set that now names nobody
                    // (#91).
                    None => Err(OpError::credential_names_nobody()),
                }
            }
        }
    }

    /// Write down that the Secret with this hash was used just now, in the
    /// table that holds it. A failure here is let go: the call the Secret was
    /// presented for matters more than the note of it.
    fn record_use(&self, table: &str, hash: &str) {
        let _ = self.db().with_conn(|conn| {
            conn.execute(
                &format!("UPDATE {table} SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE secret_hash = ?1"),
                params![hash],
            )
            .map_err(|e| OpError::internal(format!("cannot record Credential use: {e}")))
        });
    }

    /// The Operations a Door lists to whoever holds this Secret, for the MCP
    /// door's tool listing alone. The listing is not itself an Operation, so it
    /// does not go through `execute` and must not mark a Key used just for
    /// being listed against. What it leaves out is what `execute` would refuse
    /// for the kind of Credential alone:
    ///
    /// - a Runner Key is listed the Runner's own Operations and no other;
    /// - a Job Key is listed the Operations marked for it and no other;
    /// - everyone else is listed every Operation but the Runner's, an Access
    ///   Key none that is `session_only`, and a read-only one none that
    ///   writes either (ADR 0031, #347).
    ///
    /// An unresolvable or absent Secret is listed as a full Credential is: the
    /// listing itself requires none.
    pub fn operations_listed_to(&self, secret: Option<&str>) -> Vec<&'static catalogue::Operation> {
        let hash = secret.filter(|s| !s.is_empty()).map(hash_secret);
        let credential = hash
            .as_deref()
            .and_then(|hash| self.lookup_credential(hash).ok().flatten());
        let read_only = credential
            .as_ref()
            .is_some_and(|(_, read_only, _, _, _, _)| *read_only);
        let is_access_key = credential
            .as_ref()
            .is_some_and(|(_, _, is_session, _, _, _)| !is_session);
        let is_runner = hash
            .as_deref()
            .is_some_and(|hash| matches!(self.lookup_runner_key(hash), Ok(Some(_))));
        let is_job_key = hash
            .as_deref()
            .is_some_and(|hash| matches!(self.lookup_job_key(hash), Ok(Some(_))));
        catalogue::OPERATIONS
            .iter()
            .filter(|op| {
                let left_out = if is_runner {
                    op.permission != Permission::Runner
                } else if is_job_key {
                    op.job_key == JobKey::Never
                } else {
                    op.permission == Permission::Runner
                        || (read_only && op.write)
                        || (is_access_key && op.session_only)
                };
                !left_out
            })
            .collect()
    }

    /// The one data directory everything durable lives under (ADR 0028) — the
    /// database beside it, Photographs and Display Copies below it.
    pub fn data_dir(&self) -> std::path::PathBuf {
        self.db().data_dir().to_path_buf()
    }

    /// Resolve a Credential for a route that carries the same ability as a
    /// `Permission::Person` Operation, without requiring it also write
    /// (ADR 0031's read-only Access Keys may still use it). Exists for the
    /// out-of-band Photograph routes (ADR 0001), which never pass through
    /// [`Core::execute`] and so must repeat its permission check themselves
    /// rather than skip it — a Door deciding this on its own would be the
    /// same bug `execute` exists to prevent.
    pub fn authenticate(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
            OpError::unauthorized("this route requires a Credential naming a Person")
        })?;
        self.resolve_credential(secret)
    }

    /// The same resolution, additionally refusing a read-only Access Key — the
    /// out-of-band equivalent of a Catalogue Operation declared `write: true`.
    pub fn authenticate_for_write(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let caller = self.authenticate(secret)?;
        if caller.read_only {
            return Err(OpError::unauthorized(
                "this route writes, and this Credential is a read-only Access Key",
            ));
        }
        Ok(caller)
    }

    /// Resolve a Credential for `POST /api/photographs`, which is
    /// `upload_photograph` in every way but how its bytes travel. So it
    /// admits whoever that Operation admits, by that Operation's own
    /// declaration: a Person who may write, and a Job Key while the Operation
    /// is marked for one, held to what `execute` holds it to (ADR 0048). It
    /// is one of two routes outside the Catalogue a Job Key may use, the
    /// other being its own Job's Original (`read_original`).
    pub fn authenticate_to_store_photograph(
        &self,
        secret: Option<&str>,
    ) -> Result<Caller, OpError> {
        let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
            OpError::unauthorized("this route requires a Credential naming a Person")
        })?;
        let caller = self.resolve_person(secret)?;
        if let Some(job_id) = &caller.job_id {
            let upload = catalogue::find("upload_photograph")
                .ok_or_else(|| OpError::internal("upload_photograph is not declared"))?;
            if upload.job_key != JobKey::StoresPhotographs {
                return Err(nothing_here_for_a_job_key());
            }
            self.hold_job_key_write(&caller, job_id, upload.job_key, &Value::Null)?;
        }
        if caller.read_only {
            return Err(OpError::unauthorized(
                "this route writes, and this Credential is a read-only Access Key",
            ));
        }
        Ok(caller)
    }

    /// The same resolution, additionally requiring an Operator — the
    /// out-of-band equivalent of a Catalogue Operation declared
    /// `Permission::Operator`. Fetching a Backup is the one route that needs
    /// it: an archive holds every Person's recipes, Attempts and shopping
    /// lists, so carrying one off the machine is on the Operator's exact list
    /// (GLOSSARY.md, "Operator") and nobody else's.
    pub fn authenticate_for_operator(&self, secret: Option<&str>) -> Result<Caller, OpError> {
        let caller = self.authenticate(secret)?;
        if !caller.is_operator {
            return Err(OpError::unauthorized(
                "this route requires a Credential naming an Operator",
            ));
        }
        // A Backup holds every password and Secret hash, which is more than a
        // key minted to read recipes was handed (#369).
        if caller.read_only {
            return Err(OpError::unauthorized(
                "this route requires a Credential naming an Operator, and this one is a \
                 read-only Access Key",
            ));
        }
        Ok(caller)
    }
}
