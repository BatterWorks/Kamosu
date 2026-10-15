//! Originals: pasted text, a PDF or a picture handed to an agent, kept beside
//! the database until 30 days after its Job ended, and fetched by that Job's
//! Job Key alone (ADR 0048, #269, #270). A Job that reads a kept Original
//! again keeps it as its own, under its own id (#273).
//!
//! An Original is a file under `originals/` in the data directory, named by
//! its Job's id. It is no Photograph: no recipe names it, and the Photograph
//! sweep never looks in its directory. It is in no Backup, since a Backup
//! carries the database and the Photographs and nothing else, and it is no
//! part of a Version. The Job's own record says what kind of thing it is and
//! holds nothing of it, because a Job's record is in every Backup.

use super::*;
use crate::jobs::OriginalKind;

/// What a Person handed over for an agent to read: text they pasted, or the
/// bytes of a file, which may be a PDF or a picture.
pub enum Handed {
    Text(String),
    File(Vec<u8>),
}

/// Where a Job's Original is fetched, by its Job Key.
pub fn original_address(job_id: &str) -> String {
    format!("/api/originals/{job_id}")
}

/// The most bytes a file handed to an agent may be read as before it is known
/// to be a PDF or a picture: the larger of the two limits. Each is then held
/// to its own.
pub const LARGEST_HANDED_FILE: u64 = {
    let picture = photographs::MAX_PICTURE_BYTES as u64;
    if crate::pdf::LARGEST_PDF > picture {
        crate::pdf::LARGEST_PDF
    } else {
        picture
    }
};

impl Core {
    /// Hand pasted text, a PDF or a picture to an agent (#269). It makes a
    /// Job that waits for a Runner with no recipe yet: the agent creates the
    /// recipe, in the caller's own Cookbook under their Hand, as every Import
    /// lands. What was handed over is kept as the Job's Original.
    ///
    /// The answer is the Job's id. Its result is an Import Report from the
    /// moment it is asked for, read as the Job stands, so the recipe is listed
    /// as arrived once the agent has created it.
    ///
    /// A PDF or a picture handed over before is known by its contents (#271).
    /// While the caller's Job on it waits or is held, the answer is that Job.
    /// Once its agent has created the recipe, the answer is that recipe, as
    /// `branch_id` and `title`, and no Job is made. Pasted text is not matched.
    pub fn import_by_agent(
        &self,
        caller: &Caller,
        handed: Handed,
        into: Option<&str>,
    ) -> Result<Value, OpError> {
        self.ensure_has_runner()?;
        // As handed over, since a picture is remade before it is kept.
        let digest = match &handed {
            Handed::File(bytes) => Some(hex::encode(sha2::Sha256::digest(bytes))),
            Handed::Text(_) => None,
        };
        let (kind, bytes) = kept_as(handed)?;
        self.hand_to_agent(
            caller,
            kind,
            digest.as_deref(),
            Keeping::Bytes(&bytes),
            None,
            into,
        )
    }

    /// Hand an agent again the kept Original of a Job of the caller's that
    /// failed before its agent made a recipe (#273), so the cook need not find
    /// the file. It is handed over as the file was, a Job of its own matched
    /// as the same file is (#271), and the new Job keeps the Original as its
    /// own, counting its 30 days from itself.
    ///
    /// Only the Person who asked for the Job may, and to anybody else it is
    /// not there (ADR 0040). A Job that made a recipe, or has not failed, is
    /// refused with reason `not_failed_on_a_file`, and one whose Original
    /// was deleted with reason `original_gone`.
    pub fn read_job_again(&self, caller: &Caller, job_id: &str) -> Result<Value, OpError> {
        self.ensure_has_runner()?;
        let job = self
            .db()
            .with_conn(|conn| jobs::job_on_an_original(conn, job_id))?
            .filter(|job| job.asker.as_deref() == Some(caller.person_id.as_str()))
            .ok_or_else(|| jobs::no_such_job(job_id))?;
        if job.status != jobs::JobStatus::Failed || job.branch_id.is_some() {
            return Err(OpError::refused_because(
                "not_failed_on_a_file",
                "only a Job that failed before its agent made a recipe is asked again from \
                 its Original. A recipe the agent made is read again with read_recipe_again",
            ));
        }
        if !original_is_kept(&self.data_dir(), job_id, job.deleted) {
            return Err(original_gone());
        }
        // Where the failed Job was asked to land, an Operator's naming
        // included (#302).
        let into = self
            .db()
            .with_conn(|conn| jobs::cookbook_asked_for(conn, job_id))?;
        // Asked again as the failed Job was: as the Operator, where it went
        // to somebody else's Cookbook, while the asker still is one.
        let caller = &Caller {
            as_operator: caller.as_operator || into.is_some(),
            ..caller.clone()
        };
        self.hand_to_agent(
            caller,
            job.kind,
            job.digest.as_deref(),
            Keeping::From(job_id),
            Some(job_id),
            into.as_deref(),
        )
    }

    /// Make the Job that hands an agent an Original, and keep the Original as
    /// that Job's (#269). A file known by its `digest` answers the caller's
    /// Job standing on it, or the recipe its agent made, instead (#271), and
    /// so does the failed Job `again_of` names, asked again already (#273).
    fn hand_to_agent(
        &self,
        caller: &Caller,
        kind: OriginalKind,
        digest: Option<&str>,
        keeping: Keeping,
        again_of: Option<&str>,
        into: Option<&str>,
    ) -> Result<Value, OpError> {
        let dir = originals_dir(&self.data_dir());
        std::fs::create_dir_all(&dir)
            .map_err(|e| OpError::internal(format!("cannot make a place for Originals: {e}")))?;
        let failed = |e| OpError::internal(format!("cannot record the agent's Job: {e}"));
        self.db().with_conn(|conn| {
            let cookbook_id = lands_in(conn, caller, into)?;
            let import_id = find_or_create_import(conn, &cookbook_id, kind.as_str())?;
            // The agent's create lands where this one does, which is the
            // Job's to say when it is not the asker's own (#302).
            let elsewhere = (cookbook_id != cookbook_of_person(conn, &caller.person_id)?)
                .then_some(cookbook_id.as_str());
            if let Some(again_of) = again_of
                && let Some(job_id) = jobs::standing_job_again_of(conn, again_of)?
            {
                return Ok(json!({ "job_id": job_id, "source_kind": kind.as_str() }));
            }
            if let Some(digest) = digest {
                if let Some(job_id) =
                    jobs::standing_job_on_file(conn, &caller.person_id, &import_id, digest)?
                {
                    return Ok(json!({ "job_id": job_id, "source_kind": kind.as_str() }));
                }
                if let Some((branch_id, title)) = recipe_in_ledger(conn, &import_id, digest)? {
                    return Ok(json!({
                        "source_kind": kind.as_str(), "branch_id": branch_id, "title": title,
                    }));
                }
            }
            // The Job and its Original appear together or not at all: no
            // Runner is handed a Job whose file is not there yet.
            let tx = conn.unchecked_transaction().map_err(failed)?;
            let job_id =
                jobs::wait_for_agent_on_original(&tx, caller, kind, digest, again_of, elsewhere)?;
            jobs::set_result(
                &tx,
                &job_id,
                &json!({
                    "import_id": import_id,
                    "cookbook_id": cookbook_id,
                    "source_kind": kind.as_str(),
                    "arrived": [],
                    "offered": [],
                    "unreadable": [],
                    "left_out": [],
                    "related_candidates": [],
                    "agent_jobs": [{ "job_id": job_id, "branch_id": null }],
                }),
            )?;
            keep_original(&dir, &job_id, keeping)?;
            if let Err(e) = tx.commit() {
                let _ = std::fs::remove_file(dir.join(&job_id));
                return Err(failed(e));
            }
            Ok(json!({ "job_id": job_id, "source_kind": kind.as_str() }))
        })
    }

    /// Refused, with reason `no_runner`, where no agent would read what is
    /// handed over. Asked before a staged upload is read, which uses it up,
    /// so that the cook need not send the file again once there is a Runner.
    pub fn ensure_has_runner(&self) -> Result<(), OpError> {
        if self.has_runner()? {
            return Ok(());
        }
        Err(OpError::refused_because(
            "no_runner",
            "this Kamosu has no Runner, so no agent would read what you handed over. An \
             Operator mints a Runner Key and runs a Runner beside Kamosu first",
        ))
    }

    /// A Job's Original, for `GET /api/originals/<job_id>`: what kind it is and
    /// its bytes. Only that Job's own Job Key fetches it, while the Job waits
    /// or is being worked. An Original is read and never shown, so anyone else
    /// who names a Person is answered as if nothing were there, the Person who
    /// asked included (ADR 0040). A Runner Key is refused as on every route
    /// that is not the Runner's.
    pub fn read_original(
        &self,
        secret: Option<&str>,
        job_id: &str,
    ) -> Result<(OriginalKind, Vec<u8>), OpError> {
        let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
            OpError::unauthorized("this route requires the Job Key of the Job it names")
        })?;
        let caller = self.resolve_person(secret)?;
        if caller.job_id.as_deref() != Some(job_id) {
            return Err(nothing_here_for_a_job_key());
        }
        let kind = self
            .db()
            .with_conn(|conn| jobs::original_of(conn, job_id))?
            .ok_or_else(nothing_here_for_a_job_key)?;
        let bytes = std::fs::read(originals_dir(&self.data_dir()).join(job_id))
            .map_err(|_| nothing_here_for_a_job_key())?;
        Ok((kind, bytes))
    }
}

/// How many days an Original is kept once its Job has ended (ADR 0048).
pub const ORIGINAL_KEPT_DAYS: u32 = 30;

impl Core {
    /// Delete every Original whose Job ended more than
    /// [`ORIGINAL_KEPT_DAYS`] ago, on the daily sweep (#270). A Job that
    /// waits or is held keeps its Original however old it is, and a Job
    /// ends only once, so its last change is when it ended.
    ///
    /// The Job keeps its row and what kind of thing it read, and records
    /// when its Original went. The file goes first and the record second, so
    /// a sweep stopped between the two finishes the record next time. The
    /// answer is how many Originals were deleted.
    pub fn sweep_originals(&self) -> Result<usize, OpError> {
        let failed = |e| OpError::internal(format!("cannot sweep the Originals: {e}"));
        let due: Vec<String> = self.db().with_conn(|conn| {
            conn.prepare(
                "SELECT agent_jobs.job_id
                   FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
                  WHERE agent_jobs.original IS NOT NULL
                    AND agent_jobs.original_deleted_at IS NULL
                    AND jobs.status IN ('completed', 'failed', 'cancelled')
                    AND jobs.updated_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
            )
            .map_err(failed)?
            .query_map(params![format!("-{ORIGINAL_KEPT_DAYS} days")], |row| {
                row.get(0)
            })
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)
        })?;

        let dir = originals_dir(&self.data_dir());
        let mut swept = 0;
        for job_id in due {
            match std::fs::remove_file(dir.join(&job_id)) {
                Ok(()) => swept += 1,
                // Already gone, as on an instance restored from a Backup,
                // which holds no Original.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    tracing::warn!(target: "kamosu::sweep", %job_id, %e, "cannot delete an Original");
                    continue;
                }
            }
            self.db().with_conn(|conn| {
                conn.execute(
                    "UPDATE agent_jobs
                        SET original_deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                      WHERE job_id = ?1",
                    params![job_id],
                )
                .map_err(failed)
            })?;
        }
        Ok(swept)
    }
}

/// The recipe an Import's ledger names by a file's digest, and its name now.
fn recipe_in_ledger(
    conn: &Connection,
    import_id: &str,
    digest: &str,
) -> Result<Option<(String, String)>, OpError> {
    conn.query_row(
        "SELECT branches.id, json_extract(versions.content, '$.title')
           FROM import_ledger
           JOIN branches ON branches.id = import_ledger.branch_id
           JOIN versions ON versions.id = branches.head_version_id
          WHERE import_ledger.import_id = ?1 AND import_ledger.foreign_id = ?2",
        params![import_id, digest],
        |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            ))
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read the Import ledger: {e}")))
}

/// What a Job's Original is kept from: bytes handed over now, or the kept
/// Original of an earlier Job, which the new Job keeps as its own (#273).
pub(super) enum Keeping<'a> {
    Bytes(&'a [u8]),
    From(&'a str),
}

/// Keep `job_id`'s Original in `dir`. One kept already is linked rather than
/// copied, so the bytes are on the disk once while each Job's 30 days are
/// its own, and copied where the disk cannot link.
pub(super) fn keep_original(
    dir: &std::path::Path,
    job_id: &str,
    keeping: Keeping,
) -> Result<(), OpError> {
    let path = dir.join(job_id);
    let kept = match keeping {
        Keeping::Bytes(bytes) => std::fs::write(&path, bytes),
        Keeping::From(earlier) => {
            let from = dir.join(earlier);
            std::fs::hard_link(&from, &path).or_else(|_| std::fs::copy(&from, &path).map(|_| ()))
        }
    };
    kept.map_err(|e| OpError::internal(format!("cannot keep the Original: {e}")))
}

/// Whether a Job's Original is still on the disk: not deleted by the sweep,
/// and there, which on an instance restored from a Backup it is not.
pub(super) fn original_is_kept(data_dir: &std::path::Path, job_id: &str, deleted: bool) -> bool {
    !deleted && originals_dir(data_dir).join(job_id).is_file()
}

/// Refused, with reason `original_gone`, where an Original is asked for that
/// was deleted 30 days after its Job ended.
pub(super) fn original_gone() -> OpError {
    OpError::refused_because(
        "original_gone",
        "Kamosu deleted what was handed over 30 days after its Job ended, so there is \
         nothing kept to read again. Hand the file over again, or paste the text again",
    )
}

/// Where Originals are kept, under the one data directory and beside the
/// Photographs rather than among them.
pub fn originals_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("originals")
}

/// What is kept of what was handed over, and as what kind. Each kind is held
/// to the limit Kamosu already sets for it, and a file that is neither a PDF
/// nor a picture is refused.
pub(super) fn kept_as(handed: Handed) -> Result<(OriginalKind, Vec<u8>), OpError> {
    match handed {
        Handed::Text(text) => {
            if text.trim().is_empty() {
                return Err(OpError::bad_request(
                    "text holds nothing to read. Paste the recipe itself",
                ));
            }
            // `read_pasted_recipe`'s limit, in bytes as it is there.
            if text.len() > crate::pasting::LONGEST_PASTE {
                return Err(OpError::bad_request(format!(
                    "pasted text is at most {} bytes, and that is {}",
                    crate::pasting::LONGEST_PASTE,
                    text.len()
                )));
            }
            Ok((OriginalKind::PastedText, text.into_bytes()))
        }
        Handed::File(bytes) if crate::pdf::looks_like_a_pdf(&bytes) => {
            if bytes.len() as u64 > crate::pdf::LARGEST_PDF {
                return Err(OpError::bad_request(format!(
                    "a PDF is at most {} bytes here, and this one is larger",
                    crate::pdf::LARGEST_PDF
                )));
            }
            // A scan is taken: reading it is the agent's work, not Kamosu's.
            Ok((OriginalKind::Pdf, bytes))
        }
        Handed::File(bytes) if photographs::looks_like_a_picture(&bytes) => {
            // Remade, which strips where it was taken, as a Photograph's is.
            Ok((OriginalKind::Picture, photographs::remake_original(&bytes)?))
        }
        Handed::File(_) => Err(OpError::bad_request(
            "an agent is handed pasted text, a PDF or a picture, and this file is none of \
             them",
        )),
    }
}
