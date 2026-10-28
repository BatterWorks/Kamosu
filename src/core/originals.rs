//! Originals: pasted text, a PDF or a picture handed to an agent, kept beside
//! the database until 30 days after its Job ended, and fetched by that Job's
//! Job Key alone (ADR 0048, #269, #270). A Job that reads a kept Original
//! again keeps it as its own, under its own id (#273).
//!
//! An Original is a file under `originals/` in the data directory, named by
//! its Job's id. Several pictures handed over together are the pages of one
//! recipe (ADR 0055), kept in order as `<job_id>`, `<job_id>.2`, `<job_id>.3`. It is no Photograph: no recipe names it, and the Photograph
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
    /// Several pictures, the pages of one recipe in order (ADR 0055).
    Pages(Vec<Vec<u8>>),
}

/// The most pictures one Original may be: a recipe across many pages is a
/// PDF's work, and a phone's Share menu sends one file at a time.
pub const MOST_PAGES: usize = 10;

/// Where a Job's Original is fetched, by its Job Key: its first page, and
/// every page in order.
pub fn original_address(job_id: &str) -> String {
    format!("/api/originals/{job_id}")
}

/// Where each page of a Job's Original is fetched, in order. The first is
/// [`original_address`] itself, which an agent written before several pages
/// still fetches.
pub fn page_addresses(job_id: &str, pages: usize) -> Vec<String> {
    (1..=pages.max(1))
        .map(|page| match page {
            1 => original_address(job_id),
            page => format!("/api/originals/{job_id}/{page}"),
        })
        .collect()
}

/// The file one page of a Job's Original is kept in.
fn page_path(dir: &std::path::Path, job_id: &str, page: usize) -> std::path::PathBuf {
    match page {
        1 => dir.join(job_id),
        page => dir.join(format!("{job_id}.{page}")),
    }
}

/// What a file handed over is known by again (#271): the digest of its bytes
/// as handed over, or, for several pictures, of their digests in order, so
/// the same pages in another order are another Original. One picture is
/// known as it always was.
fn digest_of(handed: &Handed) -> Option<String> {
    let one = |bytes: &[u8]| hex::encode(sha2::Sha256::digest(bytes));
    match handed {
        Handed::Text(_) => None,
        Handed::File(bytes) => Some(one(bytes)),
        Handed::Pages(pages) if pages.len() == 1 => Some(one(&pages[0])),
        Handed::Pages(pages) => {
            let listed: String = pages.iter().map(|page| one(page) + "\n").collect();
            Some(one(format!("pages\n{listed}").as_bytes()))
        }
    }
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
        let digest = digest_of(&handed);
        let (kind, pages) = kept_as(handed)?;
        // Made before the database is held: a picture made smaller and a PDF
        // read are slow beside a write.
        let glimpse = self.glimpse_of(kind, &pages);
        self.hand_to_agent(
            caller,
            kind,
            digest.as_deref(),
            Keeping::Bytes(&pages),
            Some(&glimpse),
            None,
            into,
        )
    }

    /// What Settings' *Sent to the AI agent* shows of what was handed over
    /// (#463): a small copy of a picture's first page, or the first words of
    /// a PDF. A PDF that holds no text, or a reader that is busy or fails,
    /// leaves it without words, which costs the list a quote and nothing
    /// else. Pasted text needs neither: its first words are the Original's
    /// own.
    fn glimpse_of(&self, kind: OriginalKind, pages: &[Vec<u8>]) -> Glimpse {
        let Some(first) = pages.first() else {
            return Glimpse::default();
        };
        match kind {
            OriginalKind::Picture => Glimpse {
                preview: photographs::display_copy(first, PREVIEW_LONG_EDGE).ok(),
                words: None,
            },
            OriginalKind::Pdf => Glimpse {
                preview: None,
                words: self
                    .pdf_readers()
                    .text_within(first, GLIMPSE_READ)
                    .ok()
                    .map(|text| first_words(&text)),
            },
            OriginalKind::PastedText => Glimpse::default(),
        }
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
            Keeping::From(job_id, job.pages),
            None,
            Some(job_id),
            into.as_deref(),
        )
    }

    /// Make the Job that hands an agent an Original, and keep the Original as
    /// that Job's (#269). A file known by its `digest` answers the caller's
    /// Job standing on it, or the recipe its agent made, instead (#271), and
    /// so does the failed Job `again_of` names, asked again already (#273).
    /// `glimpse` is kept beside it (#463); one kept from an earlier Job
    /// comes with its Original.
    #[allow(clippy::too_many_arguments)]
    fn hand_to_agent(
        &self,
        caller: &Caller,
        kind: OriginalKind,
        digest: Option<&str>,
        keeping: Keeping,
        glimpse: Option<&Glimpse>,
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
            let job_id = jobs::wait_for_agent_on_original(
                &tx,
                caller,
                kind,
                keeping.pages(),
                digest,
                again_of,
                elsewhere,
            )?;
            jobs::set_result(
                &tx,
                &job_id,
                &imports::report_before_its_agent(&import_id, &cookbook_id, kind.as_str(), &job_id),
            )?;
            let pages = keeping.pages();
            keep_original(&dir, &job_id, keeping)?;
            if let Some(glimpse) = glimpse {
                keep_glimpse(&dir, &job_id, glimpse);
            }
            // A failed Job of the asker's own, sent again, shows what it
            // showed. A recipe read again from somebody else's never does.
            if let Some(again_of) = again_of {
                keep_glimpse_of(&dir, again_of, &job_id);
            }
            if let Err(e) = tx.commit() {
                remove_original(&dir, &job_id, pages);
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

    /// One page of a Job's Original, for `GET /api/originals/<job_id>` (the
    /// first) and `/api/originals/<job_id>/<page>`: what kind it is and its
    /// bytes. A page past the last is not there. Only that Job's own Job Key fetches it, while the Job waits
    /// or is being worked. An Original is read and never shown, so anyone else
    /// who names a Person is answered as if nothing were there, the Person who
    /// asked included (ADR 0040). A Runner Key is refused as on every route
    /// that is not the Runner's.
    pub fn read_original(
        &self,
        secret: Option<&str>,
        job_id: &str,
        page: usize,
    ) -> Result<(OriginalKind, Vec<u8>), OpError> {
        let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
            OpError::unauthorized("this route requires the Job Key of the Job it names")
        })?;
        let caller = self.resolve_person(secret)?;
        if caller.job_id.as_deref() != Some(job_id) {
            return Err(nothing_here_for_a_job_key());
        }
        let (kind, pages) = self
            .db()
            .with_conn(|conn| jobs::original_of(conn, job_id))?
            .ok_or_else(nothing_here_for_a_job_key)?;
        if page == 0 || page > pages {
            return Err(nothing_here_for_a_job_key());
        }
        let bytes = std::fs::read(page_path(&originals_dir(&self.data_dir()), job_id, page))
            .map_err(|_| nothing_here_for_a_job_key())?;
        Ok((kind, bytes))
    }
}

impl Core {
    /// `GET /api/originals/<job_id>/preview`: the small copy of a picture
    /// handed to an agent, which Settings' *Sent to the AI agent* shows
    /// beside it (#463). An Original is otherwise read and never shown. This
    /// one copy of one is, and to the Person who handed it over alone: to
    /// anybody else it is not there (ADR 0040). A Job Key and an Add-only key
    /// are refused, as on every route that is not theirs.
    pub fn read_original_preview(
        &self,
        secret: Option<&str>,
        job_id: &str,
    ) -> Result<Vec<u8>, OpError> {
        let caller = self.authenticate(secret)?;
        let nothing_here = || OpError::not_found("nothing is at this address");
        self.db()
            .with_conn(|conn| jobs::job_on_an_original(conn, job_id))?
            .filter(|job| job.asker.as_deref() == Some(caller.person_id.as_str()))
            .ok_or_else(nothing_here)?;
        std::fs::read(preview_path(&originals_dir(&self.data_dir()), job_id))
            .map_err(|_| nothing_here())
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
        let due: Vec<(String, i64)> = self.db().with_conn(|conn| {
            conn.prepare(
                "SELECT agent_jobs.job_id, agent_jobs.original_pages
                   FROM agent_jobs JOIN jobs ON jobs.id = agent_jobs.job_id
                  WHERE agent_jobs.original IS NOT NULL
                    AND agent_jobs.original_deleted_at IS NULL
                    AND jobs.status IN ('completed', 'failed', 'cancelled')
                    AND jobs.updated_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
            )
            .map_err(failed)?
            .query_map(params![format!("-{ORIGINAL_KEPT_DAYS} days")], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(failed)?
            .collect::<Result<_, _>>()
            .map_err(failed)
        })?;

        let dir = originals_dir(&self.data_dir());
        let mut swept = 0;
        for (job_id, pages) in due {
            // The later pages first, so that a sweep stopped partway leaves
            // the first, which is what says the Original is still there.
            for page in (2..=pages as usize).rev() {
                match std::fs::remove_file(page_path(&dir, &job_id, page)) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                        tracing::warn!(target: "kamosu::sweep", %job_id, %e, "cannot delete an Original");
                    }
                    _ => {}
                }
            }
            // What Settings' *Sent to the AI agent* showed of it goes with
            // it (#463).
            remove_glimpse(&dir, &job_id);
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

/// What a Job's Original is kept from: the pages handed over now, or the
/// kept Original of an earlier Job and how many pages it is, which the new
/// Job keeps as its own (#273).
pub(super) enum Keeping<'a> {
    Bytes(&'a [Vec<u8>]),
    From(&'a str, usize),
}

impl Keeping<'_> {
    /// How many files the Original is.
    pub(super) fn pages(&self) -> usize {
        match self {
            Keeping::Bytes(pages) => pages.len(),
            Keeping::From(_, pages) => *pages,
        }
    }
}

/// Keep `job_id`'s Original in `dir`, every page of it. One kept already is
/// linked rather than copied, so the bytes are on the disk once while each
/// Job's 30 days are its own, and copied where the disk cannot link. Where a
/// page cannot be kept, none is.
pub(super) fn keep_original(
    dir: &std::path::Path,
    job_id: &str,
    keeping: Keeping,
) -> Result<(), OpError> {
    let pages = keeping.pages();
    for page in 1..=pages {
        let path = page_path(dir, job_id, page);
        let kept = match &keeping {
            Keeping::Bytes(bytes) => std::fs::write(&path, &bytes[page - 1]),
            Keeping::From(earlier, _) => {
                let from = page_path(dir, earlier, page);
                std::fs::hard_link(&from, &path)
                    .or_else(|_| std::fs::copy(&from, &path).map(|_| ()))
            }
        };
        if let Err(e) = kept {
            remove_original(dir, job_id, pages);
            return Err(OpError::internal(format!("cannot keep the Original: {e}")));
        }
    }
    Ok(())
}

/// Take every page of `job_id`'s Original off the disk, as far as it is
/// there: a Job that could not be recorded keeps none.
pub(super) fn remove_original(dir: &std::path::Path, job_id: &str, pages: usize) {
    for page in 1..=pages {
        let _ = std::fs::remove_file(page_path(dir, job_id, page));
    }
    remove_glimpse(dir, job_id);
}

/// What Settings' *Sent to the AI agent* shows of an Original beside its kind
/// (#463), kept as files beside it under `originals/` rather than in the
/// database, so that it is in no Backup and goes when the Original goes.
/// Either part may be missing: an Original handed over before #463 has
/// neither.
#[derive(Default)]
pub(super) struct Glimpse {
    /// A picture's first page, made small.
    pub preview: Option<Vec<u8>>,
    /// A PDF's first words.
    pub words: Option<String>,
}

/// The long edge of a picture's small copy: a 48-pixel square on a screen
/// that draws three pixels for one, and a little to spare.
const PREVIEW_LONG_EDGE: u32 = 192;

/// How long a PDF handed over is read for its first words. Far shorter than
/// a PDF read for its recipe, since a quote is worth less than the wait, and
/// the read holds one of the few places a PDF is read in, which a cook's
/// own PDF may be waiting for.
const GLIMPSE_READ: std::time::Duration = std::time::Duration::from_secs(5);

/// The most characters of a text Settings' *Sent to the AI agent* quotes.
const MOST_FIRST_WORDS: usize = 200;

/// The start of a text as one line: its runs of spaces and line ends made
/// one space, and cut at a word near [`MOST_FIRST_WORDS`] characters.
pub(super) fn first_words(text: &str) -> String {
    let mut words = String::new();
    for word in text.split_whitespace() {
        if words.chars().count() + word.chars().count() > MOST_FIRST_WORDS && !words.is_empty() {
            break;
        }
        if !words.is_empty() {
            words.push(' ');
        }
        words.extend(word.chars().take(MOST_FIRST_WORDS));
    }
    words
}

fn preview_path(dir: &std::path::Path, job_id: &str) -> std::path::PathBuf {
    dir.join(format!("{job_id}.preview"))
}

fn words_path(dir: &std::path::Path, job_id: &str) -> std::path::PathBuf {
    dir.join(format!("{job_id}.words"))
}

/// Keep what Settings' *Sent to the AI agent* shows of `job_id`'s Original. A
/// part that cannot be written is left out: the list then says what kind of
/// thing it was, as it does once the Original is gone.
fn keep_glimpse(dir: &std::path::Path, job_id: &str, glimpse: &Glimpse) {
    if let Some(preview) = &glimpse.preview {
        let _ = std::fs::write(preview_path(dir, job_id), preview);
    }
    if let Some(words) = &glimpse.words {
        let _ = std::fs::write(words_path(dir, job_id), words);
    }
}

/// What the list shows of `earlier`'s Original, kept as `job_id`'s too,
/// linked rather than copied where the disk allows, as the Original is.
fn keep_glimpse_of(dir: &std::path::Path, earlier: &str, job_id: &str) {
    for path in [preview_path, words_path] {
        let (from, to) = (path(dir, earlier), path(dir, job_id));
        if from.is_file() {
            let _ =
                std::fs::hard_link(&from, &to).or_else(|_| std::fs::copy(&from, &to).map(|_| ()));
        }
    }
}

fn remove_glimpse(dir: &std::path::Path, job_id: &str) {
    let _ = std::fs::remove_file(preview_path(dir, job_id));
    let _ = std::fs::remove_file(words_path(dir, job_id));
}

/// The first words of what `job_id` was handed, while it is kept (#463):
/// pasted text's own, read from the start of the file, or what a PDF's
/// reading kept. None for a picture, or once the Original is gone.
pub(super) fn first_words_of(
    data_dir: &std::path::Path,
    job_id: &str,
    kind: OriginalKind,
) -> Option<String> {
    let dir = originals_dir(data_dir);
    match kind {
        OriginalKind::PastedText => {
            use std::io::Read;
            // A few times what is quoted, since a paste can open on blank
            // lines and a character can be four bytes.
            let mut start = Vec::new();
            std::fs::File::open(page_path(&dir, job_id, 1))
                .ok()?
                .take(MOST_FIRST_WORDS as u64 * 8)
                .read_to_end(&mut start)
                .ok()?;
            let words = first_words(&String::from_utf8_lossy(&start));
            (!words.is_empty()).then_some(words)
        }
        OriginalKind::Pdf => std::fs::read_to_string(words_path(&dir, job_id)).ok(),
        OriginalKind::Picture => None,
    }
}

/// Whether a small copy of `job_id`'s picture is kept (#463).
pub(super) fn has_preview(data_dir: &std::path::Path, job_id: &str) -> bool {
    preview_path(&originals_dir(data_dir), job_id).is_file()
}

/// Where the small copy of a Job's picture is fetched, by the Person who
/// handed it over (#463).
pub fn preview_address(job_id: &str) -> String {
    format!("/api/originals/{job_id}/preview")
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
pub(super) fn kept_as(handed: Handed) -> Result<(OriginalKind, Vec<Vec<u8>>), OpError> {
    let (kind, bytes) = match handed {
        Handed::Pages(mut pages) if pages.len() == 1 => kept_one(Handed::File(pages.remove(0)))?,
        Handed::Pages(pages) => return kept_pages(pages),
        one => kept_one(one)?,
    };
    Ok((kind, vec![bytes]))
}

/// Several pictures, kept as the pages of one recipe in order (ADR 0055).
/// Each is held to what one picture is held to, and nothing but pictures
/// goes together: a PDF has pages of its own.
fn kept_pages(pages: Vec<Vec<u8>>) -> Result<(OriginalKind, Vec<Vec<u8>>), OpError> {
    if pages.is_empty() {
        return Err(OpError::bad_request("pictures holds no picture"));
    }
    if pages.len() > MOST_PAGES {
        return Err(OpError::bad_request(format!(
            "one recipe is at most {MOST_PAGES} pictures, and that is {}",
            pages.len()
        )));
    }
    pages
        .into_iter()
        .enumerate()
        .map(|(at, bytes)| {
            if !photographs::looks_like_a_picture(&bytes) {
                return Err(OpError::bad_request(format!(
                    "several files handed over together are pictures, the pages of one \
                     recipe, and file {} is not one",
                    at + 1
                )));
            }
            photographs::remake_original(&bytes)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|kept| (OriginalKind::Picture, kept))
}

fn kept_one(handed: Handed) -> Result<(OriginalKind, Vec<u8>), OpError> {
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
        Handed::Pages(_) => unreachable!("kept_as keeps several pictures itself"),
    }
}
