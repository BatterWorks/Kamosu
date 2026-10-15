//! Slow work: the Job machinery beneath every Operation declared `Kind::Job`.
//!
//! Asking for a Job records it in the database — the truth about the work, its
//! progress and its result or failure reason (ADR 0003) — and hands it to a
//! **lane**. There are exactly two lanes, and ordinarily which one carries a
//! job depends on who asked, never on what was asked (ADR 0032): members have
//! their own lane, and everything a stranger can cause runs in one depth-one
//! lane behind a short bounded line. When the line is full the ask is refused
//! with *busy*, so the worst a stranger can inflict is latency on other
//! strangers — never collapse.
//!
//! One kind of Job breaks that rule on purpose: the Catalogue may declare a Job
//! `JobLane::AlwaysSingle` when its risk lives in the outbound fetch itself
//! rather than in who asked for it (the web-link importer, #70 — a page's own
//! text can tell an agent to fetch another URL, ADR 0033). Such a Job always
//! takes the single depth-one lane, member or not.
//!
//! An **agent's Job** is the one kind that takes no lane at all (ADR 0048). A
//! Runner beside Kamosu takes it and an agent does the work. The row waits
//! queued until a Runner takes it, is running while that Runner holds it, and
//! ends as the Runner reports. The hold has a deadline the Runner renews, and
//! a Job whose deadline passed waits again (#266). The deadline is stored, so
//! a restart leaves the Job as it was. It can be cancelled in either state,
//! which one of Kamosu's own cannot once a worker carries it (#267).
//!
//! A streaming progress mechanism, if one ever exists, may decorate this but is
//! never instead of it: the row in this table is where watching slow work reads.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::catalogue;
use crate::core::{Asker, Caller, ErrorKind, Invocation, OpError, generate_secret, hash_secret};
use crate::db::Db;

/// How often a client should look back at a running Job.
pub const POLL_INTERVAL_MS: i64 = 5000;

/// The handle through which running work reports its progress. Given to a
/// handler only while it runs as a Job; reporting is best-effort — a failed
/// progress write never fails the work itself, whose truth is its outcome.
#[derive(Clone)]
pub struct JobProgress {
    job_id: String,
    db: Arc<Db>,
}

impl JobProgress {
    pub fn new(job_id: &str, db: Arc<Db>) -> Self {
        JobProgress {
            job_id: job_id.to_string(),
            db,
        }
    }

    /// The Job this is the progress of — what work that leaves a file behind
    /// names the file after, so it can be fetched by the id the caller holds.
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    /// Say how far the work has got: how much done, of what total if it knows,
    /// and one sentence a reader can act on. Read back through `get_job`.
    pub fn report(&self, done: u64, total: Option<u64>, message: impl Into<String>) {
        let _ = self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE jobs SET progress_done = ?2, progress_total = ?3,
                 progress_message = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1",
                params![
                    self.job_id,
                    done as i64,
                    total.map(|t| t as i64),
                    message.into()
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot record progress: {e}")))?;
            Ok(())
        });
    }
}

/// The member lane: two pieces of work at once, a bounded line behind them.
const MEMBER_WORKERS: usize = 2;
const MEMBER_LINE: usize = 16;
/// The stranger lane: one piece of work at once and nowhere to queue beyond a
/// handful. A stranger may cause work, never work that scales with them.
const STRANGER_WORKERS: usize = 1;
const STRANGER_LINE: usize = 4;

/// The five states a Job can be in. Kept an enum rather than bare strings so
/// every comparison and mapping spells the states in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }

    fn from_column(value: String) -> Self {
        match value.as_str() {
            "running" => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Queued,
        }
    }
}

/// One Job as the database holds it: what was asked, by whom, how far it has
/// got, and how it ended. The row is the truth; this is that truth in memory.
#[derive(Debug)]
pub struct JobRecord {
    pub id: String,
    /// The Person who asked, when one did; None for work a stranger caused.
    pub person_id: Option<String>,
    pub read_only: bool,
    pub via_access_key: bool,
    pub operation: String,
    pub input: Value,
    pub status: JobStatus,
    pub progress_done: Option<i64>,
    pub progress_total: Option<i64>,
    pub progress_message: Option<String>,
    /// The eventual output, present once completed.
    pub result: Option<Value>,
    /// Why it failed: one sentence and the JSON-RPC code that carries it.
    pub error: Option<String>,
    pub error_code: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// The sending ends of both lanes, held by the Core.
pub struct Lanes {
    pub member: mpsc::Sender<String>,
    pub stranger: mpsc::Sender<String>,
}

/// The receiving ends, consumed once when the workers are spawned. Each lane's
/// receiver is shared by its workers, so they compete for the next Job.
pub struct LaneReceivers {
    member: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
    stranger: Arc<tokio::sync::Mutex<mpsc::Receiver<String>>>,
}

/// Build both lanes. Called once per Core.
pub fn lanes() -> (Lanes, LaneReceivers) {
    let (member_tx, member_rx) = mpsc::channel(MEMBER_LINE);
    let (stranger_tx, stranger_rx) = mpsc::channel(STRANGER_LINE);
    (
        Lanes {
            member: member_tx,
            stranger: stranger_tx,
        },
        LaneReceivers {
            member: Arc::new(tokio::sync::Mutex::new(member_rx)),
            stranger: Arc::new(tokio::sync::Mutex::new(stranger_rx)),
        },
    )
}

/// Start the workers that carry Jobs from the lanes. Each worker takes work off
/// its own lane only — members never wait behind strangers.
pub fn spawn_workers(core: Arc<crate::core::Core>, receivers: LaneReceivers) {
    for _ in 0..MEMBER_WORKERS {
        let core = core.clone();
        let rx = receivers.member.clone();
        tokio::spawn(async move {
            loop {
                let job_id = rx.lock().await.recv().await;
                match job_id {
                    Some(job_id) => carry(core.clone(), job_id).await,
                    None => return, // the lane is gone; nothing more to carry
                }
            }
        });
    }
    for _ in 0..STRANGER_WORKERS {
        let core = core.clone();
        let rx = receivers.stranger.clone();
        tokio::spawn(async move {
            while let Some(job_id) = rx.lock().await.recv().await {
                carry(core.clone(), job_id).await;
            }
        });
    }
}

async fn carry(core: Arc<crate::core::Core>, job_id: String) {
    // The database is behind a mutex and handlers block on it; keep them off the
    // async worker threads by carrying the work on the blocking pool.
    let _ = tokio::task::spawn_blocking(move || core.run_job(&job_id)).await;
}

/// Called once at startup, before any Door answers: work a previous process
/// accepted must not strand. Running work can't be resumed honestly — the
/// handler died with the process — so it reports why it ended; queued work
/// re-enters its lane and is carried as if just asked for.
///
/// An agent's Job is left as it was. Kamosu was never doing the work, so
/// nothing of it died with the process, and no lane carries it. Its hold is
/// the one thing looked at: a deadline that passed while Kamosu was down has
/// lapsed like any other.
pub fn recover_at_startup(db: &Arc<Db>, lanes: &Lanes) {
    if let Err(err) = db.with_conn(release_lapsed_holds) {
        tracing::warn!("cannot release the holds that lapsed: {err}");
    }
    let _ = db.with_conn(|conn| {
        let interrupted = conn
            .execute(
                "UPDATE jobs SET status = 'failed',
             error = 'this Job was interrupted when Kamosu restarted',
             error_code = -32603,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE status = 'running'
               AND id NOT IN (SELECT job_id FROM agent_jobs)",
                [],
            )
            .map_err(|e| OpError::internal(format!("cannot recover interrupted Jobs: {e}")))?;
        if interrupted > 0 {
            tracing::warn!(
                "{interrupted} Job(s) were mid-flight at shutdown; marked failed with the reason"
            );
        }
        Ok(())
    });

    let waiting: Vec<(String, bool)> = match db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, person_id IS NULL FROM jobs
                 WHERE status = 'queued'
                   AND id NOT IN (SELECT job_id FROM agent_jobs)
                 ORDER BY created_at",
            )
            .map_err(|e| OpError::internal(format!("cannot list waiting Jobs: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
            })
            .map_err(|e| OpError::internal(format!("cannot list waiting Jobs: {e}")))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }) {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!("cannot recover waiting Jobs: {err}");
            return;
        }
    };

    for (job_id, is_stranger_work) in waiting {
        let lane = if is_stranger_work {
            &lanes.stranger
        } else {
            &lanes.member
        };
        if lane.try_send(job_id.clone()).is_err() {
            // The line was already full at boot; refuse honestly rather than
            // leave the row pretending to wait forever.
            let _ = db.with_conn(|conn| {
                conn.execute(
                    "UPDATE jobs SET status = 'failed',
                     error = 'refused when Kamosu restarted: the lane for this Job was full',
                     error_code = -32603,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                     WHERE id = ?1",
                    params![job_id],
                )
                .map_err(|e| OpError::internal(format!("cannot fail recovered Job: {e}")))?;
                Ok(())
            });
        }
    }
}

impl crate::core::Core {
    /// Carry one Job from its row to its end state. Runs on the blocking pool.
    pub(crate) fn run_job(&self, job_id: &str) {
        let record = match self.job(job_id) {
            // A row nobody knows has nothing to carry.
            Ok(Some(record)) => record,
            Ok(None) => return,
            Err(_) => return,
        };
        // Only a queued row is ours to carry: a cancelled one is skipped, and
        // anything else has already ended some other way.
        if record.status != JobStatus::Queued {
            return;
        }

        set_running(self, job_id);

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            match catalogue::find(&record.operation) {
                Some(op) => {
                    let caller = record.person_id.clone().map(|person_id| Caller {
                        person_id,
                        read_only: record.read_only,
                        via_access_key: record.via_access_key,
                        is_operator: false,
                        // A JobRecord does not carry which Access Key asked, only
                        // that one did (`via_access_key`) — nothing built on a Job
                        // needs the specific Key today.
                        access_key_id: None,
                        // Nor which Session: a Job lists no Sessions.
                        session_id: None,
                        // A Job Key asks for no Job (ADR 0048).
                        job_id: None,
                        // As it was asked for. Whether the asker may still
                        // act as an Operator is asked of the account as it
                        // stands now (`looks_after`).
                        as_operator: crate::core::asks_as_operator(&record.input),
                    });
                    let invocation = Invocation {
                        // A Job Kamosu does itself was asked for by a Person
                        // or by nobody, never by a Runner.
                        asker: caller.map_or(Asker::Nobody, Asker::Person),
                        job: Some(JobProgress::new(job_id, self.db().clone())),
                    };
                    (op.handler)(self, &invocation, record.input.clone())
                }
                None => Err(OpError::internal(format!(
                    "the Catalogue no longer knows '{}'",
                    record.operation
                ))),
            }
        }));

        match outcome {
            Ok(Ok(value)) => finish(
                self,
                job_id,
                JobStatus::Completed,
                Some(value.to_string()),
                None,
                None,
            ),
            Ok(Err(err)) => finish(
                self,
                job_id,
                JobStatus::Failed,
                None,
                Some(err.to_sentence()),
                Some(rpc_code(err.kind)),
            ),
            // A panic is a failure like any other: reported through the same
            // Operations rather than vanishing with the request.
            Err(_) => finish(
                self,
                job_id,
                JobStatus::Failed,
                None,
                Some("this Operation failed unexpectedly".to_string()),
                Some(-32603),
            ),
        }
    }
}

/// Map an Operation failure onto the JSON-RPC code the MCP door reports when the
/// failed Job is read back through the long-running-task extension.
fn rpc_code(kind: ErrorKind) -> i64 {
    match kind {
        ErrorKind::BadRequest | ErrorKind::UnknownOperation | ErrorKind::NotFound => -32602,
        ErrorKind::Unauthorized => -32001,
        ErrorKind::Busy | ErrorKind::Internal => -32603,
    }
}

/// Record that slow work was asked for, before any answer goes out: by the time
/// the caller sees the job id, `get_job` must be able to see the Job too.
pub fn record(
    core: &crate::core::Core,
    operation: &str,
    invocation: &Invocation,
    input: Value,
) -> Result<String, OpError> {
    core.db()
        .with_conn(|conn| insert(conn, operation, invocation.caller(), &input))
}

/// Write a Job's row, queued, and answer its id.
fn insert(
    conn: &Connection,
    operation: &str,
    caller: Option<&Caller>,
    input: &Value,
) -> Result<String, OpError> {
    let job_id = new_id();
    // A Job asked for by an Operator acting as one says so, which is what lets
    // its Job Key reach its recipe in somebody else's Cookbook (#302).
    let mut input = input.clone();
    if caller.is_some_and(|caller| caller.as_operator) {
        input["as_operator"] = json!(true);
    }
    let (person_id, read_only, via_access_key) = match caller {
        Some(caller) => (
            Some(caller.person_id.clone()),
            caller.read_only,
            caller.via_access_key,
        ),
        None => (None, false, false),
    };
    conn.execute(
        "INSERT INTO jobs (id, person_id, read_only, via_access_key, operation, input)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            job_id,
            person_id,
            read_only as i64,
            via_access_key as i64,
            operation,
            input.to_string()
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot record Job: {e}")))?;
    Ok(job_id)
}

/// What an agent's Job is recorded as having been asked for. It names no
/// Operation of the Catalogue, because no handler of Kamosu's does the work:
/// a Runner takes the Job and an agent does it (ADR 0048).
pub const AGENT_OPERATION: &str = "read_by_agent";

/// The reason an agent's Job ends with when its recipe goes from under it.
const RECIPE_GONE: &str = "the recipe this Job was for is gone";

/// The agent's Job that waits or is being worked on a recipe, if there is one.
/// A recipe has one at a time.
pub fn standing_agent_job(conn: &Connection, branch_id: &str) -> Result<Option<String>, OpError> {
    Ok(agent_job_standing_on(conn, branch_id)?.map(|standing| standing.id))
}

/// An agent's Job that stands on a recipe: which, whether it waits (`queued`)
/// or is being worked (`running`), and who asked for it.
pub struct StandingAgentJob {
    pub id: String,
    pub status: String,
    pub asker: Option<String>,
}

/// The Job [`standing_agent_job`] answers, with its state and who asked.
pub fn agent_job_standing_on(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<StandingAgentJob>, OpError> {
    conn.query_row(
        "SELECT jobs.id, jobs.status, jobs.person_id
           FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
          WHERE agent_jobs.branch_id = ?1 AND jobs.status IN ('queued', 'running')",
        params![branch_id],
        |row| {
            Ok(StandingAgentJob {
                id: row.get(0)?,
                status: row.get(1)?,
                asker: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the recipe's Job: {e}")))
}

/// The recipe an agent's Job is for, standing or ended. A Job Kamosu does
/// itself is for none, and so is one reading an Original until its agent
/// creates the recipe (#269). The recipe may since have been deleted.
pub fn recipe_of_agent_job(conn: &Connection, job_id: &str) -> Result<Option<String>, OpError> {
    Ok(conn
        .query_row(
            "SELECT branch_id FROM agent_jobs WHERE job_id = ?1",
            params![job_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Job's recipe: {e}")))?
        .flatten())
}

/// Have an agent read `link` into the recipe `branch_id`: a Job that waits
/// for a Runner, in neither lane (ADR 0048, #261). Asked for by `caller`,
/// whose Job it is to read and to cancel.
///
/// Where a Job already stands on the recipe, that Job is the answer and no
/// second is made. Taking the connection is what makes that hold: the looking
/// and the writing are one turn at the database.
pub fn wait_for_agent(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    link: &str,
) -> Result<String, OpError> {
    if let Some(job_id) = standing_agent_job(conn, branch_id)? {
        return Ok(job_id);
    }
    insert_on_link(conn, caller, branch_id, link, &json!({}))
}

/// The `source_kind` of a recipe a Co-author asked to have read again
/// (#272). A Job asked for that way is its own Import, as one reading an
/// Original is, and its input says so.
pub const READ_AGAIN: &str = "read_again";

/// Have an agent read `link` again into the recipe `branch_id`, which a
/// Co-author asked for (ADR 0048, #272). The caller has checked there is no
/// Job standing on it already. Its result is written by the caller: the Job
/// is its own Import.
pub fn wait_for_agent_to_read_again(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    link: &str,
) -> Result<String, OpError> {
    insert_on_link(
        conn,
        caller,
        branch_id,
        link,
        &json!({ "source_kind": READ_AGAIN }),
    )
}

/// Whether the recipe `branch_id` has been read again already, which a
/// recipe is once (#279). A reading counts once an agent finished it, or
/// was stopped while a Runner held it. One that failed, or was stopped while
/// it still waited, gives the turn back: no agent finished it, or none
/// began. A Job stopped while held keeps `held_by` (`cancel`), and one that
/// waits again has it cleared (`end_a_try`), so `held_by` says which.
pub fn read_again_already(conn: &Connection, branch_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
            WHERE agent_jobs.branch_id = ?1
              AND json_extract(jobs.input, '$.source_kind') = ?2
              AND (jobs.status = 'completed'
                   OR (jobs.status = 'cancelled' AND agent_jobs.held_by IS NOT NULL)))",
        params![branch_id, READ_AGAIN],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot read the recipe's readings: {e}")))
}

/// Have an agent read again, into the recipe `branch_id`, an Original of
/// `kind` that the caller keeps under the id this answers (#273). As
/// [`wait_for_agent_to_read_again`], which reads a link: the Job is the
/// recipe's from the start, so its agent writes it and creates none.
pub fn wait_for_agent_to_read_original_again(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    kind: OriginalKind,
) -> Result<String, OpError> {
    let job_id = insert(
        conn,
        AGENT_OPERATION,
        Some(caller),
        &json!({ "source_kind": READ_AGAIN }),
    )?;
    conn.execute(
        "INSERT INTO agent_jobs (job_id, branch_id, original) VALUES (?1, ?2, ?3)",
        params![job_id, branch_id, kind.as_str()],
    )
    .map_err(|e| OpError::internal(format!("cannot record the agent's Job: {e}")))?;
    Ok(job_id)
}

/// The Original a recipe was last read from (#273): the agent's Job that read
/// it, what kind it is, and whether the sweep has deleted it since (#270).
pub struct RecipesOriginal {
    pub job_id: String,
    pub kind: OriginalKind,
    pub deleted: bool,
}

/// The Original the latest agent's Job on a recipe read, where that Job's
/// Import is still there (#273). That a recipe came from a file is the
/// Import's memory and never the recipe's: nothing about it is in a Version,
/// and forgetting the Import forgets it. The Import may be another Cookbook's
/// than the recipe's, as where the Person who asked left with it.
pub fn original_of_recipe(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<RecipesOriginal>, OpError> {
    let found: Option<(String, String, bool)> = conn
        .query_row(
            "SELECT agent_jobs.job_id, agent_jobs.original,
                    agent_jobs.original_deleted_at IS NOT NULL
               FROM agent_jobs
               JOIN jobs ON jobs.id = agent_jobs.job_id
               JOIN imports ON imports.id = json_extract(jobs.result, '$.import_id')
              WHERE agent_jobs.branch_id = ?1 AND agent_jobs.original IS NOT NULL
              ORDER BY jobs.created_at DESC, jobs.rowid DESC
              LIMIT 1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the recipe's Original: {e}")))?;
    Ok(found.and_then(|(job_id, kind, deleted)| {
        OriginalKind::named(&kind).map(|kind| RecipesOriginal {
            job_id,
            kind,
            deleted,
        })
    }))
}

/// The Job that waits or is held reading again the Original of the failed
/// Job `job_id` (#273). Pasted text is known by no digest, so this is how
/// asking twice answers the one Job.
pub fn standing_job_again_of(conn: &Connection, job_id: &str) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT id FROM jobs
          WHERE json_extract(input, '$.again_of') = ?1 AND status IN ('queued', 'running')
          ORDER BY created_at
          LIMIT 1",
        params![job_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the Job asked again: {e}")))
}

/// An agent's Job that read an Original, as asking it again needs it (#273):
/// who asked, how it stands, whether it made a recipe, and its Original.
pub struct JobOnAnOriginal {
    pub asker: Option<String>,
    pub status: JobStatus,
    pub branch_id: Option<String>,
    pub kind: OriginalKind,
    pub digest: Option<String>,
    pub deleted: bool,
}

/// The Job `job_id` names, where it is an agent's Job that read an Original.
pub fn job_on_an_original(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<JobOnAnOriginal>, OpError> {
    type Row = (
        Option<String>,
        String,
        Option<String>,
        String,
        Option<String>,
        bool,
    );
    let found: Option<Row> = conn
        .query_row(
            "SELECT jobs.person_id, jobs.status, agent_jobs.branch_id, agent_jobs.original,
                    agent_jobs.original_digest, agent_jobs.original_deleted_at IS NOT NULL
               FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
              WHERE jobs.id = ?1 AND agent_jobs.original IS NOT NULL",
            params![job_id],
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
        .map_err(|e| OpError::internal(format!("cannot read the Job: {e}")))?;
    Ok(
        found.and_then(|(asker, status, branch_id, kind, digest, deleted)| {
            OriginalKind::named(&kind).map(|kind| JobOnAnOriginal {
                asker,
                status: JobStatus::from_column(status),
                branch_id,
                kind,
                digest,
                deleted,
            })
        }),
    )
}

/// SQL that holds for an agent's Job that is its own Import, on a `jobs`
/// row: one reading an Original (#269) or a recipe read again (#272). Its
/// result is the Import Report it was asked with. A Job on a link an Import
/// landed bare is that Import's, and names no source of its own.
pub const ITS_OWN_IMPORT: &str = "json_extract(jobs.input, '$.source_kind') IS NOT NULL";

fn insert_on_link(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    link: &str,
    input: &Value,
) -> Result<String, OpError> {
    let job_id = insert(conn, AGENT_OPERATION, Some(caller), input)?;
    conn.execute(
        "INSERT INTO agent_jobs (job_id, branch_id, link) VALUES (?1, ?2, ?3)",
        params![job_id, branch_id, link],
    )
    .map_err(|e| OpError::internal(format!("cannot record the agent's Job: {e}")))?;
    Ok(job_id)
}

/// What kind of thing an Original is (#269). Its name is the Import's
/// `source_kind`, and is what `agent_jobs.original` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalKind {
    PastedText,
    Pdf,
    Picture,
}

impl OriginalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginalKind::PastedText => "pasted_text",
            OriginalKind::Pdf => "pdf",
            OriginalKind::Picture => "picture",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "pasted_text" => Some(OriginalKind::PastedText),
            "pdf" => Some(OriginalKind::Pdf),
            "picture" => Some(OriginalKind::Picture),
            _ => None,
        }
    }

    /// What the bytes are, for whoever fetches them.
    pub fn content_type(self) -> &'static str {
        match self {
            OriginalKind::PastedText => "text/plain; charset=utf-8",
            OriginalKind::Pdf => "application/pdf",
            // Remade as a Photograph is (`photographs::remake_original`).
            OriginalKind::Picture => "image/webp",
        }
    }
}

/// Have an agent read an Original into a recipe it creates (#269): a Job that
/// waits for a Runner, in neither lane, with no recipe yet. Its record says
/// what kind of thing it reads and holds nothing of it: the Original is a
/// file the caller keeps beside the database, named by the id this answers.
/// `digest` is a file's, by which the same file is known again (#271).
/// `again_of` is the failed Job whose Original this one reads again (#273).
/// `cookbook_id` is a Cookbook an Operator named for its recipe, other than
/// their own (#302).
pub fn wait_for_agent_on_original(
    conn: &Connection,
    caller: &Caller,
    kind: OriginalKind,
    digest: Option<&str>,
    again_of: Option<&str>,
    cookbook_id: Option<&str>,
) -> Result<String, OpError> {
    let mut input = json!({ "source_kind": kind.as_str() });
    if let Some(again_of) = again_of {
        input["again_of"] = json!(again_of);
    }
    if let Some(cookbook_id) = cookbook_id {
        input["cookbook_id"] = json!(cookbook_id);
    }
    let job_id = insert(conn, AGENT_OPERATION, Some(caller), &input)?;
    conn.execute(
        "INSERT INTO agent_jobs (job_id, original, original_digest) VALUES (?1, ?2, ?3)",
        params![job_id, kind.as_str(), digest],
    )
    .map_err(|e| OpError::internal(format!("cannot record the agent's Job: {e}")))?;
    Ok(job_id)
}

/// The caller's own Job that waits or is held on the file `digest` names,
/// for the Import `import_id` (#271). A Job of an Import since forgotten is
/// not it, and neither is a Co-author's, which the caller could not read.
pub fn standing_job_on_file(
    conn: &Connection,
    person_id: &str,
    import_id: &str,
    digest: &str,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT jobs.id
           FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
          WHERE agent_jobs.original_digest = ?1
            AND jobs.status IN ('queued', 'running')
            AND jobs.person_id = ?2
            AND json_extract(jobs.result, '$.import_id') = ?3
          ORDER BY jobs.created_at
          LIMIT 1",
        params![digest, person_id, import_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the file's Job: {e}")))
}

/// A Cookbook's Import of one kind was joined to another's (`move_imports`):
/// the agent's Jobs that are their own Import are now the one kept's. The same
/// file still finds a Job reading it, and its recipe still goes into its
/// ledger (#271), and a recipe read again names an Import that is there (#272).
pub fn move_to_kept_import(
    conn: &Connection,
    from: &str,
    into: &str,
    cookbook_id: &str,
) -> Result<(), OpError> {
    conn.execute(
        &format!(
            "UPDATE jobs
                SET result = json_set(result, '$.import_id', ?2, '$.cookbook_id', ?3)
              WHERE json_extract(result, '$.import_id') = ?1
                AND operation = '{AGENT_OPERATION}' AND {ITS_OWN_IMPORT}"
        ),
        params![from, into, cookbook_id],
    )
    .map_err(|e| OpError::internal(format!("cannot move the Import's Jobs: {e}")))?;
    Ok(())
}

/// The latest agent's Job that completed on a recipe, which says an agent
/// has read it (#271).
pub fn agent_job_completed_on(
    conn: &Connection,
    branch_id: &str,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT jobs.id
           FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
          WHERE agent_jobs.branch_id = ?1 AND jobs.status = 'completed'
          ORDER BY jobs.updated_at DESC
          LIMIT 1",
        params![branch_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the recipe's Jobs: {e}")))
}

/// Write a Job's result before it has one: the Import Report of a Job that
/// reads an Original, which is read as it stands from the moment it is asked
/// for (#269).
pub fn set_result(conn: &Connection, job_id: &str, result: &Value) -> Result<(), OpError> {
    conn.execute(
        "UPDATE jobs SET result = ?2 WHERE id = ?1",
        params![job_id, result.to_string()],
    )
    .map_err(|e| OpError::internal(format!("cannot write the Job's Report: {e}")))?;
    Ok(())
}

/// The kind of Original an agent's Job reads, or none where it reads a link
/// or is no agent's Job.
pub fn original_of(conn: &Connection, job_id: &str) -> Result<Option<OriginalKind>, OpError> {
    Ok(conn
        .query_row(
            "SELECT original FROM agent_jobs WHERE job_id = ?1",
            params![job_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Job's Original: {e}")))?
        .flatten()
        .and_then(|name| OriginalKind::named(&name)))
}

/// Whether an agent's Job waits or is being worked, and the recipe it is for
/// if it has one yet: `None` once it has ended or where it is no agent's Job.
pub fn standing_recipe_of(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<Option<String>>, OpError> {
    conn.query_row(
        "SELECT agent_jobs.branch_id
           FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
          WHERE jobs.id = ?1 AND jobs.status IN ('queued', 'running')",
        params![job_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the Job's recipe: {e}")))
}

/// Whether a Job was asked for by an Operator acting as one (#302).
pub fn asked_as_operator(conn: &Connection, job_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT json_extract(input, '$.as_operator') IS 1 FROM jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .optional()
    .map(|asked| asked.unwrap_or(false))
    .map_err(|e| OpError::internal(format!("cannot read the Job: {e}")))
}

/// The Cookbook an Operator named for an agent's Job reading an Original,
/// other than their own (#302). `None` for every other Job. Read off the
/// Import Report, which a join of two Cookbooks moves with the Import
/// (`move_to_kept_import`), and not off the ask, which nothing moves.
pub fn cookbook_asked_for(conn: &Connection, job_id: &str) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT CASE WHEN json_extract(input, '$.cookbook_id') IS NOT NULL
                     THEN json_extract(result, '$.cookbook_id') END
           FROM jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(|e| OpError::internal(format!("cannot read the Job's Cookbook: {e}")))
}

/// The agent created its Job's recipe (#269): from here the Job is for that
/// recipe, as one that began on a bare recipe is. Set once, and never moved
/// by a second create, which the Core refuses before it runs.
///
/// A Job reading a file also has its Import's ledger name the recipe by the
/// file's digest, so the same file handed over again finds it (#271). Not
/// where that Import was forgotten while the Job waited, and not where the
/// ledger names something else by that digest already. Nor where the recipe
/// is in another Cookbook than the Import, as when the asker left the
/// Cookbook while the Job waited: that Import's other Co-authors could not
/// open it.
pub fn take_its_recipe(conn: &Connection, job_id: &str, branch_id: &str) -> Result<(), OpError> {
    let failed = |e| OpError::internal(format!("cannot give the Job its recipe: {e}"));
    let taken = conn
        .execute(
            "UPDATE agent_jobs SET branch_id = ?2 WHERE job_id = ?1 AND branch_id IS NULL",
            params![job_id, branch_id],
        )
        .map_err(failed)?;
    if taken == 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT OR IGNORE INTO import_ledger (import_id, foreign_id, lineage_id, branch_id)
         SELECT imports.id, agent_jobs.original_digest, branches.lineage_id, branches.id
           FROM agent_jobs
           JOIN jobs ON jobs.id = agent_jobs.job_id
           JOIN imports ON imports.id = json_extract(jobs.result, '$.import_id')
           JOIN branches ON branches.id = agent_jobs.branch_id
                        AND branches.cookbook_id = imports.cookbook_id
          WHERE agent_jobs.job_id = ?1 AND agent_jobs.original_digest IS NOT NULL",
        params![job_id],
    )
    .map_err(failed)?;
    Ok(())
}

/// The recipe a Job's Job Key created, read off the first Version it wrote,
/// and made the Job's own where it was not yet (#269).
pub fn recipe_created_by(conn: &Connection, job_id: &str) -> Result<Option<String>, OpError> {
    let created: Option<String> = conn
        .query_row(
            "SELECT branch_id FROM branch_versions WHERE job_id = ?1 AND sequence = 1",
            params![job_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Job's recipe: {e}")))?;
    if let Some(branch_id) = &created {
        take_its_recipe(conn, job_id, branch_id)?;
    }
    Ok(created)
}

/// A recipe held twice was folded into one (`fold_branch`): the Job on the
/// holding that goes carries on, on the one kept. Where the kept one has a
/// Job standing already, this one stays behind and ends with its Branch.
pub fn move_to_kept_recipe(conn: &Connection, drop: &str, keep: &str) -> Result<(), OpError> {
    if standing_agent_job(conn, keep)?.is_some() {
        return Ok(());
    }
    conn.execute(
        "UPDATE agent_jobs SET branch_id = ?2 WHERE branch_id = ?1",
        params![drop, keep],
    )
    .map_err(|e| OpError::internal(format!("cannot move the recipe's Job: {e}")))?;
    Ok(())
}

/// End the agent's Job on a recipe that is being deleted, waiting or being
/// worked, and say why. A Job with no recipe to write could only fail later.
pub fn end_for_deleted_recipe(conn: &Connection, branch_id: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE jobs SET status = 'failed', error = ?2, error_code = -32602,
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE status IN ('queued', 'running')
           AND id IN (SELECT job_id FROM agent_jobs WHERE branch_id = ?1)",
        params![branch_id, RECIPE_GONE],
    )
    .map_err(|e| OpError::internal(format!("cannot end the recipe's Job: {e}")))?;
    Ok(())
}

/// The reason an agent's Job ends with when the Person who asked is deleted.
const ASKER_GONE: &str = "the person who asked for this Job is gone";

/// End the agent's Jobs a Person asked for, waiting or being worked, because
/// that Person was deleted (#266). No Runner is handed a Job whose asker is
/// gone, and a deleted Person does not come back, so the Job would stand on
/// its recipe for ever and keep any other from being asked for.
pub fn end_for_deleted_asker(conn: &Connection, person_id: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE jobs SET status = 'failed', error = ?2, error_code = -32602,
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE status IN ('queued', 'running') AND person_id = ?1
           AND id IN (SELECT job_id FROM agent_jobs)",
        params![person_id, ASKER_GONE],
    )
    .map_err(|e| OpError::internal(format!("cannot end the Person's Jobs: {e}")))?;
    Ok(())
}

/// The reason an agent's Job ends with when a write of its would have started
/// a Copy (ADR 0048). The cook reads it in the Import Report.
pub const WOULD_START_A_COPY: &str = "the person who asked cannot change this recipe: a change of \
     theirs would start a copy in their own Cookbook, and an agent's Job never starts one";

/// The reason an agent's Job ends with when somebody saved the recipe between
/// two of its saves (#265). The cook reads it in the Import Report.
pub const CHANGED_WHILE_IT_WORKED: &str = "the recipe was changed while the agent worked on it, \
     so the agent stopped rather than write over that change. What it had saved before is kept";

/// The reason an agent's Job ends with when another Branch or a Translation
/// holds the Version it was still shaping (#265): a Copy or a variation made
/// since, a Translation of it, or a Branch that already read the same.
pub const HELD_ELSEWHERE: &str = "a copy, a variation or a translation of the recipe holds what \
     the agent had saved, so the agent stopped rather than change it under that one. What it \
     had saved before is kept";

/// End a Job because a write of its was refused, with one of the reasons
/// above (#264, #265). Failed, which spends its Job Keys. The Job is being
/// worked, or waits again while the Job Key of a hold that lapsed still
/// writes (#266).
/// What it saved before stays. Marked as the thing itself at fault, as a
/// recipe gone from under a Job is: trying again would meet the same refusal.
pub fn end_where_a_write_was_refused(
    conn: &Connection,
    job_id: &str,
    reason: &str,
) -> Result<(), OpError> {
    conn.execute(
        "UPDATE jobs SET status = 'failed', error = ?2, error_code = -32602,
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1 AND status IN ('queued', 'running')",
        params![job_id, reason],
    )
    .map_err(|e| OpError::internal(format!("cannot end the Job: {e}")))?;
    Ok(())
}

/// An agent's Job as it stands now: its state, the reason it failed where it
/// did, and the agent's own report once a Runner has passed one on.
pub struct AgentJobState {
    pub job_id: String,
    /// The recipe the Job is for, which one reading an Original has only
    /// once its agent created it (#269).
    pub branch_id: Option<String>,
    pub status: JobStatus,
    pub error: Option<String>,
    pub report: Option<Value>,
    /// Its Original was deleted, 30 days after it ended (#270).
    pub original_gone: bool,
    /// Its agent said what it was handed holds no recipe it can read, so
    /// `error` is the agent's own words (#276).
    pub could_not_be_read: bool,
}

/// How each of these Jobs stands now, for the ones that are an agent's. An id
/// that names no agent's Job is left out.
pub fn agent_jobs_as_they_stand(
    conn: &Connection,
    job_ids: &[String],
) -> Result<Vec<AgentJobState>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT jobs.status, jobs.error, agent_jobs.report, agent_jobs.branch_id,
                    agent_jobs.original_deleted_at IS NOT NULL,
                    agent_jobs.outcome IS 'could_not_be_read'
               FROM jobs JOIN agent_jobs ON agent_jobs.job_id = jobs.id
              WHERE jobs.id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read an agent's Job: {e}")))?;
    let mut states = Vec::new();
    for job_id in job_ids {
        let state = statement
            .query_row(params![job_id], |row| {
                Ok(AgentJobState {
                    job_id: job_id.clone(),
                    branch_id: row.get(3)?,
                    status: JobStatus::from_column(row.get(0)?),
                    error: row.get(1)?,
                    report: row
                        .get::<_, Option<String>>(2)?
                        .and_then(|text| serde_json::from_str(&text).ok()),
                    original_gone: row.get(4)?,
                    could_not_be_read: row.get(5)?,
                })
            })
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read an agent's Job: {e}")))?;
        states.extend(state);
    }
    Ok(states)
}

/// How long a Runner's hold on a Job lasts unless it is renewed (#266). A
/// Job whose Runner crashed waits again within this long. Two minutes is short
/// beside the minutes an agent takes, and long enough that a Runner renewing
/// every third of it can miss twice and keep its Job.
pub const HOLD: std::time::Duration = std::time::Duration::from_secs(120);

/// How many times a Job is tried (#266). A try that does not finish is a hold
/// that lapsed or a Runner's report that it failed, and the Job waits again
/// after each but the last. Three, as a retry is where a page that timed out
/// once is read, and a Job that failed three times is not a fourth away from
/// working. This is the one place the number is written.
pub const TRIES: i64 = 3;

/// What a try that ended by its hold lapsing is said to have ended with.
const RUNNER_WENT_QUIET: &str = "the Runner working on it went quiet.";

/// The reason a Job ends with when its last try did not finish. `trouble` is
/// what that try ended with: the Runner's own reason, or that it went quiet.
fn tried_too_often(trouble: &str) -> String {
    format!(
        "Kamosu tried this Job {TRIES} times, which is as often as it tries one, and it \
         never finished. The last time: {trouble}"
    )
}

/// A deadline `hold` from now, as SQLite's `strftime` takes it.
fn from_now(hold: std::time::Duration) -> String {
    format!("+{:.3} seconds", hold.as_secs_f64())
}

/// What a Runner is handed with a Job: what to read, which recipe it is for
/// (#262), the Job Key its agent presents (#263), and the deadline of its
/// hold (#266). What to read is a link or an Original, never both (#269). A
/// Job reading an Original has no recipe until its agent creates one, unless
/// it reads a recipe again (#273).
pub struct AgentWork {
    pub job_id: String,
    pub branch_id: Option<String>,
    pub link: Option<String>,
    pub original: Option<OriginalKind>,
    /// The Job Key's raw Secret. Only its hash is kept, so this is the one
    /// time it is known.
    pub job_key: String,
    pub held_until: String,
}

/// A Job a Runner may be handed, as read: its id, its recipe, its link and
/// the kind of its Original.
type WaitingJob = (String, Option<String>, Option<String>, Option<String>);

/// Hand the Runner holding `runner_key_id` a Job to do (ADR 0048, #262).
///
/// A Runner holds one Job at a time, so one that already holds a Job is
/// answered that Job again. Otherwise it takes the oldest that waits, which
/// is running from here on, with a deadline `hold` away. A Job whose asker is
/// disabled waits and is handed to nobody. Taking the connection is
/// what keeps two Runners off one Job: the looking and the taking are one turn
/// at the database.
///
/// Every answer carries a Job Key minted for it. `ask` is the mark of this
/// ask, which the Runner makes up and sends again when it repeats the ask
/// because the reply was lost (#266). The Job Key of that reply is the one
/// nobody received, so the repeat replaces it. An ask under a new mark spends
/// nothing: a Job Key is spent when its Job ends and by no clock, so the one
/// from a hold that lapsed works beside the next Runner's until then.
pub fn take_for_runner(
    conn: &Connection,
    runner_key_id: &str,
    ask: &str,
    hold: std::time::Duration,
) -> Result<Option<AgentWork>, OpError> {
    let failed = |e| OpError::internal(format!("cannot hand over a Job: {e}"));
    release_lapsed_holds(conn)?;
    // The Job this Runner holds comes first, and after it the waiting ones,
    // oldest first.
    let found: Option<WaitingJob> = conn
        .query_row(
            "SELECT jobs.id, agent_jobs.branch_id, agent_jobs.link, agent_jobs.original
               FROM agent_jobs
               JOIN jobs ON jobs.id = agent_jobs.job_id
               JOIN people ON people.id = jobs.person_id
              WHERE (jobs.status = 'queued' AND people.disabled = 0 AND people.deleted = 0)
                 OR (jobs.status = 'running' AND agent_jobs.held_by = ?1)
              ORDER BY jobs.status = 'running' DESC, jobs.created_at, jobs.rowid
              LIMIT 1",
            params![runner_key_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(failed)?;
    let Some((job_id, branch_id, link, original)) = found else {
        return Ok(None);
    };
    let job_key = generate_secret();
    // Every row moves or none does: a Job is held only while it is running,
    // names its Runner and has a deadline.
    let tx = conn.unchecked_transaction().map_err(failed)?;
    tx.execute(
        "DELETE FROM job_keys WHERE job_id = ?1 AND runner_key_id = ?2 AND ask = ?3",
        params![job_id, runner_key_id, ask],
    )
    .map_err(failed)?;
    tx.execute(
        "INSERT INTO job_keys (secret_hash, job_id, runner_key_id, ask) VALUES (?1, ?2, ?3, ?4)",
        params![hash_secret(&job_key), job_id, runner_key_id, ask],
    )
    .map_err(failed)?;
    let held_until = tx
        .query_row(
            "UPDATE agent_jobs SET held_by = ?2,
             held_until = strftime('%Y-%m-%dT%H:%M:%fZ','now', ?3)
             WHERE job_id = ?1 RETURNING held_until",
            params![job_id, runner_key_id, from_now(hold)],
            |row| row.get(0),
        )
        .map_err(failed)?;
    tx.execute(
        "UPDATE jobs SET status = 'running',
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1 AND status = 'queued'",
        params![job_id],
    )
    .map_err(failed)?;
    tx.commit().map_err(failed)?;
    Ok(Some(AgentWork {
        job_id,
        branch_id,
        link,
        original: original.as_deref().and_then(OriginalKind::named),
        job_key,
        held_until,
    }))
}

/// Move the deadline of the hold `runner_key_id` has on a Job to `hold` from
/// now, and answer the new deadline (#266).
pub fn renew_hold(
    conn: &Connection,
    runner_key_id: &str,
    job_id: &str,
    hold: std::time::Duration,
) -> Result<String, OpError> {
    release_lapsed_holds(conn)?;
    let held_until = conn
        .query_row(
            "UPDATE agent_jobs SET held_until = strftime('%Y-%m-%dT%H:%M:%fZ','now', ?3)
              WHERE job_id = ?1 AND held_by = ?2
                AND job_id IN (SELECT id FROM jobs WHERE status = 'running')
             RETURNING held_until",
            params![job_id, runner_key_id, from_now(hold)],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot renew the hold: {e}")))?;
    match held_until {
        Some(held_until) => Ok(held_until),
        None => Err(not_held(conn, runner_key_id, job_id)?),
    }
}

/// What a Runner is answered about a Job it does not hold. One it was handed
/// before is told its hold is gone, which tells it nothing it did not know,
/// or that the Job was cancelled, which is how a Runner learns of a cancel
/// (#267): in the reply to a call it was already making. Any other Job is
/// answered as one that is not there, whether another Runner holds it or it
/// never was (ADR 0040).
fn not_held(conn: &Connection, runner_key_id: &str, job_id: &str) -> Result<OpError, OpError> {
    let failed = |e| OpError::internal(format!("cannot read the Job's holds: {e}"));
    let was_handed_it: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM job_keys WHERE job_id = ?1 AND runner_key_id = ?2)",
            params![job_id, runner_key_id],
            |row| row.get(0),
        )
        .map_err(failed)?;
    if !was_handed_it {
        return Ok(no_such_job(job_id));
    }
    let was_cancelled: bool = conn
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM jobs WHERE id = ?1 AND status = 'cancelled')",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(failed)?;
    Ok(OpError::not_found(if was_cancelled {
        format!(
            "Job '{job_id}' was cancelled. Stop the agent working on it, then ask for work again"
        )
    } else {
        format!(
            "this Runner no longer holds Job '{job_id}'. The hold is gone: its deadline passed \
             before it was renewed, or the Job has ended. Stop the agent working on it, then \
             ask for work again"
        )
    }))
}

/// Send every Job whose hold has lapsed back to waiting, or end it where that
/// was its last try (#266). Run before a Runner is answered anything, on a
/// short tick and at startup, so a Job nobody is working never reads as one
/// that is.
pub fn release_lapsed_holds(conn: &Connection) -> Result<(), OpError> {
    let failed = |e| OpError::internal(format!("cannot release a lapsed hold: {e}"));
    let lapsed: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT agent_jobs.job_id
                   FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
                  WHERE jobs.status = 'running'
                    AND agent_jobs.held_until <= strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            )
            .map_err(failed)?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)?
    };
    for job_id in lapsed {
        let tx = conn.unchecked_transaction().map_err(failed)?;
        end_a_try(&tx, &job_id, RUNNER_WENT_QUIET)?;
        tx.commit().map_err(failed)?;
    }
    Ok(())
}

/// A Runner Key was revoked: the Job it held waits again, and the Job Keys it
/// was handed are revoked with it (#266). Revoking is an Operator's act and no
/// fault of the Job's, so it counts as no try.
pub fn release_for_revoked_runner(conn: &Connection, runner_key_id: &str) -> Result<(), OpError> {
    let failed = |e| OpError::internal(format!("cannot release the Runner's Job: {e}"));
    conn.execute(
        "UPDATE jobs SET status = 'queued',
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE status = 'running'
           AND id IN (SELECT job_id FROM agent_jobs WHERE held_by = ?1)",
        params![runner_key_id],
    )
    .map_err(failed)?;
    conn.execute(
        "UPDATE agent_jobs SET held_by = NULL, held_until = NULL
          WHERE held_by = ?1 AND job_id IN (SELECT id FROM jobs WHERE status = 'queued')",
        params![runner_key_id],
    )
    .map_err(failed)?;
    conn.execute(
        "DELETE FROM job_keys WHERE runner_key_id = ?1",
        params![runner_key_id],
    )
    .map_err(failed)?;
    Ok(())
}

/// One try at a Job that is being worked ended without finishing it (#266).
/// The Job waits again with the recipe as the try left it, or ends as failed
/// where this was the last of its tries. `trouble` is what the try ended
/// with, which the cook reads if it was the last. Answers whether the Job
/// ended.
fn end_a_try(conn: &Connection, job_id: &str, trouble: &str) -> Result<bool, OpError> {
    let failed = |e| OpError::internal(format!("cannot end the try: {e}"));
    let tries: i64 = conn
        .query_row(
            "UPDATE agent_jobs SET tries = tries + 1, held_until = NULL
              WHERE job_id = ?1 RETURNING tries",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(failed)?;
    if tries >= TRIES {
        conn.execute(
            "UPDATE jobs SET status = 'failed', error = ?2, error_code = -32603,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
            params![job_id, tried_too_often(trouble)],
        )
        .map_err(failed)?;
        return Ok(true);
    }
    // Nothing of the try stays on a Job that waits again but its count and
    // what the agent saved.
    conn.execute(
        "UPDATE agent_jobs SET held_by = NULL WHERE job_id = ?1",
        params![job_id],
    )
    .map_err(failed)?;
    conn.execute(
        "UPDATE jobs SET status = 'queued',
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ?1",
        params![job_id],
    )
    .map_err(failed)?;
    Ok(false)
}

/// How a Runner says a Job ended (#262). The last two are kept apart because
/// one is worth trying again and the other is not (#266).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentOutcome {
    Completed,
    /// Something went wrong on the way. The Job is tried again, up to `TRIES`.
    Failed,
    /// The agent says the thing holds no recipe it can read. That ends the
    /// Job at once: another try would read the same thing.
    CouldNotBeRead,
}

impl AgentOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentOutcome::Completed => "completed",
            AgentOutcome::Failed => "failed",
            AgentOutcome::CouldNotBeRead => "could_not_be_read",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "completed" => Some(AgentOutcome::Completed),
            "failed" => Some(AgentOutcome::Failed),
            "could_not_be_read" => Some(AgentOutcome::CouldNotBeRead),
            _ => None,
        }
    }
}

/// End the try `runner_key_id` holds, as that Runner reports it (#262).
/// `reason` is the sentence a failed Job carries, and `report` is the agent's
/// own, already checked. A Job reported failed waits to be tried again unless
/// that was its last try (#266). Any other report ends it.
///
/// Only the Runner Key holding a Job reports on it. One whose hold is gone is
/// told so. A Job it never held, and one that never was, are answered alike,
/// as a Job that is not there (ADR 0040).
pub fn end_as_reported(
    conn: &Connection,
    runner_key_id: &str,
    job_id: &str,
    outcome: AgentOutcome,
    reason: Option<&str>,
    report: &Value,
) -> Result<(), OpError> {
    let failed = |e| OpError::internal(format!("cannot end the Job: {e}"));
    release_lapsed_holds(conn)?;
    let held: Option<(Option<String>, bool)> = conn
        .query_row(
            &format!(
                "SELECT agent_jobs.branch_id, {ITS_OWN_IMPORT}
                   FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
                  WHERE jobs.id = ?1 AND jobs.status = 'running' AND agent_jobs.held_by = ?2"
            ),
            params![job_id, runner_key_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(failed)?;
    let Some((branch_id, its_own_import)) = held else {
        return Err(not_held(conn, runner_key_id, job_id)?);
    };
    // A Job reading an Original makes its recipe, so one that made none did
    // not complete, whatever its agent says (#269).
    if outcome == AgentOutcome::Completed && branch_id.is_none() {
        return Err(OpError::bad_request(
            "the agent created no recipe, so this Job did not complete. Report it failed or \
             could_not_be_read, with the reason",
        ));
    }
    let tx = conn.unchecked_transaction().map_err(failed)?;
    let ended = match outcome {
        AgentOutcome::Failed => end_a_try(&tx, job_id, reason.unwrap_or_default())?,
        AgentOutcome::Completed | AgentOutcome::CouldNotBeRead => {
            let (status, result, error_code) = if outcome == AgentOutcome::Completed {
                let result = json!({ "branch_id": branch_id, "agent_report": report });
                (JobStatus::Completed, Some(result.to_string()), None)
            } else {
                // The thing itself is at fault, as a bad request's input is.
                (JobStatus::Failed, None, Some(-32602))
            };
            // A Job reading an Original or reading a recipe again is its own
            // Import, so its result is the Import Report it was asked with,
            // kept as it is (#269, #272).
            tx.execute(
                "UPDATE jobs SET status = ?2, result = CASE WHEN ?6 THEN result ELSE ?3 END,
                 error = ?4, error_code = ?5,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1",
                params![
                    job_id,
                    status.as_str(),
                    result,
                    reason,
                    error_code,
                    its_own_import
                ],
            )
            .map_err(failed)?;
            true
        }
    };
    // What the agent said is kept with the Job it ended. A try that is
    // followed by another leaves its report to that one.
    if ended {
        tx.execute(
            "UPDATE agent_jobs SET outcome = ?2, report = ?3, held_until = NULL
              WHERE job_id = ?1",
            params![job_id, outcome.as_str(), report.to_string()],
        )
        .map_err(failed)?;
    }
    tx.commit().map_err(failed)
}

/// Remove the row of work that was refused before anyone saw its id.
pub fn forget(core: &crate::core::Core, job_id: &str) -> Result<(), OpError> {
    core.db().with_conn(|conn| {
        conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])
            .map_err(|e| OpError::internal(format!("cannot forget refused Job: {e}")))?;
        Ok(())
    })
}

/// Read one Job back from the database.
pub fn read(core: &crate::core::Core, job_id: &str) -> Result<Option<JobRecord>, OpError> {
    core.db().with_conn(|conn| {
        conn.prepare(
            "SELECT id, person_id, read_only, via_access_key, operation, input, status,
                        progress_done, progress_total, progress_message,
                        result, error, error_code, created_at, updated_at
                 FROM jobs WHERE id = ?1",
        )
        .map_err(|e| OpError::internal(format!("cannot read Job: {e}")))?
        .query_row(params![job_id], map_record)
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Job: {e}")))
    })
}

/// Every Job one Person has asked for, newest first, most recent hundred.
pub fn read_of_person(
    core: &crate::core::Core,
    person_id: &str,
) -> Result<Vec<JobRecord>, OpError> {
    core.db().with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, person_id, read_only, via_access_key, operation, input, status,
                        progress_done, progress_total, progress_message,
                        result, error, error_code, created_at, updated_at
                 FROM jobs WHERE person_id = ?1
                 ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| OpError::internal(format!("cannot list Jobs: {e}")))?;
        let rows = stmt
            .query_map(params![person_id], map_record)
            .map_err(|e| OpError::internal(format!("cannot list Jobs: {e}")))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    })
}

fn map_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?,
        person_id: row.get(1)?,
        read_only: row.get::<_, i64>(2)? != 0,
        via_access_key: row.get::<_, i64>(3)? != 0,
        operation: row.get(4)?,
        input: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(Value::Null),
        status: JobStatus::from_column(row.get(6)?),
        progress_done: row.get(7)?,
        progress_total: row.get(8)?,
        progress_message: row.get(9)?,
        result: row
            .get::<_, Option<String>>(10)?
            .map(|t| serde_json::from_str(&t).unwrap_or(Value::Null)),
        error: row.get(11)?,
        error_code: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

/// Cancel a Job, and answer whether it was cancelled. Acked either way.
///
/// A Job Kamosu does itself is cancelled while it still waits in line. Once
/// a worker carries it, the handler is running and nothing stops it.
///
/// An agent's Job is cancelled whether it waits or a Runner holds it (#267,
/// ADR 0048), because Kamosu is not the one doing the work. Cancelled is an
/// end like any other, so every Job Key of the Job is spent from here on
/// (`lookup_job_key`), and what the agent saved before stays. Kamosu never
/// calls a Runner: the one holding the Job is refused its next renewal and
/// told why (`not_held`), and stops its agent then. A cancel is no try.
pub fn cancel(core: &crate::core::Core, job_id: &str) -> Result<bool, OpError> {
    let failed = |e| OpError::internal(format!("cannot cancel Job: {e}"));
    core.db().with_conn(|conn| {
        // The Job and its hold go together or not at all.
        let tx = conn.unchecked_transaction().map_err(failed)?;
        let changed = tx
            .execute(
                "UPDATE jobs SET status = 'cancelled',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                 WHERE id = ?1
                   AND (status = 'queued'
                        OR (status = 'running' AND id IN (SELECT job_id FROM agent_jobs)))",
                params![job_id],
            )
            .map_err(failed)?;
        if changed > 0 {
            // A Job that has ended has no deadline. Who held it stays, as on
            // a Job that ended any other way.
            tx.execute(
                "UPDATE agent_jobs SET held_until = NULL WHERE job_id = ?1",
                params![job_id],
            )
            .map_err(failed)?;
        }
        tx.commit().map_err(failed)?;
        Ok(changed > 0)
    })
}

/// Who may read a Job Kamosu does itself: the Person who asked for it, or
/// anyone when no Person did. Work caused anonymously (a Sheet rendered for a
/// Share Link, say) carries nothing that was not already public, so hiding its
/// state would hide nothing. An agent's Job is read by more People, which
/// `Core::ensure_reads_job` asks (#268).
///
/// Anyone else is refused with `absent` — the refusal the id they named would
/// have got had it named nothing, so asking tells the caller nothing about other
/// People's work (ADR 0040). The caller passes it, because which id they named
/// decides which not-found is the honest one: `/api/sheets/<id>` resolved a
/// **Sheet** id and must answer about Sheets, while `get_job` resolved a Job id
/// and must answer about Jobs. Whose work it is is no part of either answer.
pub fn ensure_reader(
    record: &JobRecord,
    caller: &Option<Caller>,
    absent: impl FnOnce() -> OpError,
) -> Result<(), OpError> {
    if asked_by(record, caller) {
        Ok(())
    } else {
        Err(absent())
    }
}

/// Who may cancel a Job: the Person who asked for it and nobody else (#267),
/// or anyone when no Person did, as with reading.
///
/// This is asked apart from who reads on purpose. The Co-authors of the
/// Cookbook an agent's Job writes in read that Job (#268), and reading it
/// lets none of them cancel it. Anyone else is refused with `absent`, for the
/// reason given at `ensure_reader` (ADR 0040).
pub fn ensure_asker(
    record: &JobRecord,
    caller: &Option<Caller>,
    absent: impl FnOnce() -> OpError,
) -> Result<(), OpError> {
    if asked_by(record, caller) {
        Ok(())
    } else {
        Err(absent())
    }
}

/// Whether `caller` is the Person who asked for a Job. A Job no Person asked
/// for is anyone's.
pub fn asked_by(record: &JobRecord, caller: &Option<Caller>) -> bool {
    match (&record.person_id, caller) {
        (None, _) => true,
        (Some(asker), Some(c)) => asker.as_str() == c.person_id,
        (Some(_), None) => false,
    }
}

/// A Job id that names nothing here — and, by ADR 0040, a Job belonging to
/// someone else.
pub fn no_such_job(job_id: &str) -> OpError {
    OpError::not_found(format!("no Job with id '{job_id}'"))
}

fn set_running(core: &crate::core::Core, job_id: &str) {
    let _ = core.db().with_conn(|conn| {
        conn.execute(
            "UPDATE jobs SET status = 'running',
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
            params![job_id],
        )
        .map_err(|e| OpError::internal(format!("cannot start Job: {e}")))?;
        Ok(())
    });
}

/// End a Job. A completed Job that declared a total reads all of it done,
/// so no work has to remember to say so itself: a Backup did forget, and read
/// `0 of 1` for ever after (#155). A failed Job keeps its count, since how far
/// it got is worth knowing, and a Job that never knew its total is not handed
/// one.
fn finish(
    core: &crate::core::Core,
    job_id: &str,
    status: JobStatus,
    result: Option<String>,
    error: Option<String>,
    error_code: Option<i64>,
) {
    let _ = core.db().with_conn(|conn| {
        conn.execute(
            "UPDATE jobs SET status = ?2, result = ?3, error = ?4, error_code = ?5,
             progress_done = CASE WHEN ?6 AND progress_total IS NOT NULL
                             THEN progress_total ELSE progress_done END,
             progress_message = CASE WHEN ?6 AND progress_total IS NOT NULL
                                THEN 'finished' ELSE progress_message END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
            params![
                job_id,
                status.as_str(),
                result,
                error,
                error_code,
                status == JobStatus::Completed
            ],
        )
        .map_err(|e| OpError::internal(format!("cannot finish Job: {e}")))?;
        Ok(())
    });
}

/// Render one Job as the ordinary Operation output both Doors serve.
pub fn to_value(record: &JobRecord) -> Value {
    json!({
        "id": record.id,
        "operation": record.operation,
        "status": record.status.as_str(),
        "progress": {
            "done": record.progress_done,
            "total": record.progress_total,
            "message": record.progress_message,
        },
        "result": record.result,
        "error": record.error,
        "errorCode": record.error_code,
        "created_at": record.created_at,
        "updated_at": record.updated_at,
    })
}

/// Render one Job as a summary for `list_jobs`.
pub fn to_summary(record: &JobRecord) -> Value {
    json!({
        "id": record.id,
        "operation": record.operation,
        "status": record.status.as_str(),
        "created_at": record.created_at,
    })
}

fn new_id() -> String {
    // A Job id names work; it grants nothing, so plain randomness suffices and
    // the leading letter keeps it legibly not a Secret.
    let mut bytes = [0u8; 12];
    rand::fill(&mut bytes);
    format!("j_{}", hex::encode(bytes))
}
