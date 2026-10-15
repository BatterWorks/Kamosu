//! Reading a recipe again: a Co-author asks for an agent to read a recipe
//! once more (ADR 0048). A recipe that came from an Original is read from it
//! while it is kept, and from a file handed over with the ask once it is gone
//! (#273). Any other is read from its Source link, whichever way it arrived
//! (#272).
//!
//! The Job is the one a link landing bare makes (#261), or a file handed to
//! an agent (#269), on a recipe that is already there. It is its own Import,
//! so the cook reads what the agent did in an Import Report, where the
//! recipe is listed as an arrival.

use super::originals::{Keeping, keep_original, kept_as, original_is_kept};
use super::*;
use crate::jobs::OriginalKind;

/// Why a recipe cannot be read again by the Person asking. `can_read_recipe_again`
/// answers it, and `read_recipe_again` refuses with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CannotReadAgain {
    /// They see the recipe and do not write its Cookbook.
    NotACoAuthor,
    /// Their Cookbook holds it and did not write it, as a recipe received in
    /// a Bundle: their change would start a Copy, and a Job never makes one.
    WouldStartACopy,
    /// No agent would read it.
    NoRunner,
    /// It came from an Original, which was deleted 30 days after its Job
    /// ended (#270): the file is handed over again with the ask.
    NeedsTheFile,
    /// It came from no Original, and its Source holds no link an agent may
    /// be sent to: none, or one that is no http or https address on the
    /// internet.
    NothingToRead,
}

impl CannotReadAgain {
    fn reason(self) -> &'static str {
        match self {
            CannotReadAgain::NotACoAuthor => "not_a_co_author",
            CannotReadAgain::WouldStartACopy => "would_start_a_copy",
            CannotReadAgain::NoRunner => "no_runner",
            CannotReadAgain::NeedsTheFile => "needs_the_file",
            CannotReadAgain::NothingToRead => "nothing_to_read",
        }
    }

    fn refusal(self) -> OpError {
        let reason = self.reason();
        match self {
            CannotReadAgain::NotACoAuthor => OpError {
                reason: Some(reason),
                ..OpError::unauthorized(
                    "only a Co-author of the Cookbook that holds this recipe may ask for it to \
                     be read again",
                )
            },
            CannotReadAgain::WouldStartACopy => OpError::refused_because(
                reason,
                "your Cookbook received this recipe and did not write it, so a change of yours \
                 would start a Copy of it, and an agent's Job never starts one. Change it \
                 yourself first: that makes it your own, and that one can be read again",
            ),
            CannotReadAgain::NoRunner => OpError::refused_because(
                reason,
                "this Kamosu has no Runner, so no agent would read the recipe again. An \
                 Operator mints a Runner Key and runs a Runner beside Kamosu first",
            ),
            CannotReadAgain::NeedsTheFile => OpError::refused_because(
                reason,
                "this recipe was read from a file or from pasted text, which Kamosu deleted 30 \
                 days after its Job ended. Hand it over again with the ask, as text, upload_id \
                 or data",
            ),
            CannotReadAgain::NothingToRead => OpError::refused_because(
                reason,
                "this recipe was read from no file, and its Source holds no link an agent can \
                 be sent to read, which is an http or https address on the internet, so there \
                 is nothing to read it again from",
            ),
        }
    }
}

/// What a recipe would be read again from.
enum ReadFrom {
    /// The link in its Source (#272).
    Link(String),
    /// The Original it came from, kept under the Job that read it (#273).
    KeptOriginal { job_id: String, kind: OriginalKind },
}

/// Whether a recipe can be read again by the Person asking, and from what.
struct WhatToReadAgain {
    /// The link in its Source, where an agent may be sent to it.
    link: Option<String>,
    /// The kind of Original it came from, kept or not.
    original: Option<OriginalKind>,
    from: Option<ReadFrom>,
    cannot: Option<CannotReadAgain>,
}

impl Core {
    /// Whether `caller` may have a recipe read again, and from what (#272,
    /// #273). A recipe they do not see is not there (ADR 0040).
    pub fn can_read_recipe_again(
        &self,
        caller: &Caller,
        branch_id: &str,
    ) -> Result<Value, OpError> {
        let has_runner = self.has_runner()?;
        let data_dir = self.data_dir();
        self.db().with_conn(|conn| {
            let what = what_to_read_again(conn, caller, branch_id, has_runner, &data_dir)?;
            // Whatever stands in the way: what it would be read from, kept
            // or to be handed over again.
            let from = if what.original.is_some() {
                Some("original")
            } else {
                what.link.as_ref().map(|_| "link")
            };
            Ok(json!({
                "can_read_again": what.cannot.is_none(),
                "from": from,
                "link": what.link,
                "original": what.original.map(OriginalKind::as_str),
                "reason": what.cannot.map(CannotReadAgain::reason),
            }))
        })
    }

    /// Refused where a file handed over to read a recipe again would be,
    /// asked before the file is read: reading a staged upload uses it up,
    /// and the cook should not have to send it twice over a refusal.
    pub fn ensure_may_hand_over_to_read_again(
        &self,
        caller: &Caller,
        branch_id: &str,
    ) -> Result<(), OpError> {
        let has_runner = self.has_runner()?;
        let data_dir = self.data_dir();
        self.db().with_conn(|conn| {
            let what = what_to_read_again(conn, caller, branch_id, has_runner, &data_dir)?;
            may_hand_over(&what)
        })
    }

    /// Have an agent read a recipe again (#272, #273). Only a Co-author of
    /// the Cookbook that holds it may ask, on an instance with a Runner. The
    /// answer is the Job's id. Where a Job waits or is being worked on the
    /// recipe already, that Job is the answer and no second is made.
    ///
    /// A recipe that came from an Original is read from it while it is kept,
    /// and the new Job keeps it as its own, so its 30 days run from that Job.
    /// Once it is gone the ask is refused with reason `needs_the_file`, and
    /// `handed` is what to read: it is kept as the new Job's Original. A file
    /// handed over always makes a Job, whatever was handed over before, but
    /// where a Job already stands on the recipe that Job is the answer and
    /// nothing handed over is kept. Any other recipe is read from its Source
    /// link, and takes no file.
    ///
    /// The Job's result is an Import Report from the moment it is asked for,
    /// under the source `read_again`, with the recipe listed as an arrival.
    /// What the agent saves is one new Version under the asker's Hand (#265).
    pub fn read_recipe_again(
        &self,
        caller: &Caller,
        branch_id: &str,
        handed: Option<Handed>,
    ) -> Result<Value, OpError> {
        let has_runner = self.has_runner()?;
        let data_dir = self.data_dir();
        let handed = handed.map(kept_as).transpose()?;
        let dir = originals_dir(&data_dir);
        let failed = |e| OpError::internal(format!("cannot record the agent's Job: {e}"));
        self.db().with_conn(|conn| {
            let what = what_to_read_again(conn, caller, branch_id, has_runner, &data_dir)?;
            if handed.is_some() {
                may_hand_over(&what)?;
            } else if let Some(cannot) = what.cannot {
                return Err(cannot.refusal());
            }
            if let Some(job_id) = jobs::standing_agent_job(conn, branch_id)? {
                return Ok(json!({ "job_id": job_id, "branch_id": branch_id }));
            }

            let cookbook_id = branch_cookbook(conn, branch_id)?;
            if handed.is_some() || matches!(what.from, Some(ReadFrom::KeptOriginal { .. })) {
                std::fs::create_dir_all(&dir).map_err(|e| {
                    OpError::internal(format!("cannot make a place for Originals: {e}"))
                })?;
            }
            let tx = conn.unchecked_transaction().map_err(failed)?;
            let import_id = find_or_create_import(&tx, &cookbook_id, jobs::READ_AGAIN)?;
            // The Job, what its arrival is known by, and the Original it keeps:
            // the link it reads, or, for an Original, which has no name, the
            // Job itself.
            let original = match (&handed, &what.from) {
                (Some((kind, bytes)), _) => Some((*kind, Keeping::Bytes(bytes))),
                (None, Some(ReadFrom::KeptOriginal { job_id, kind })) => {
                    Some((*kind, Keeping::From(job_id)))
                }
                _ => None,
            };
            let (job_id, foreign_id, keeping) = match (original, &what.from) {
                (Some((kind, keeping)), _) => {
                    let job_id =
                        jobs::wait_for_agent_to_read_original_again(&tx, caller, branch_id, kind)?;
                    (job_id.clone(), job_id, Some(keeping))
                }
                (None, Some(ReadFrom::Link(link))) => {
                    let job_id = jobs::wait_for_agent_to_read_again(&tx, caller, branch_id, link)?;
                    (job_id, link.clone(), None)
                }
                (None, _) => return Err(CannotReadAgain::NothingToRead.refusal()),
            };
            // Listed with the status that names the ask, spelt as its source.
            let arrival = imports::arrival_as_it_reads(&tx, branch_id, &foreign_id, "read_again")?
                .ok_or_else(no_such_branch)?;
            jobs::set_result(
                &tx,
                &job_id,
                &json!({
                    "import_id": import_id,
                    "cookbook_id": cookbook_id,
                    "source_kind": jobs::READ_AGAIN,
                    "arrived": [arrival],
                    "offered": [],
                    "unreadable": [],
                    "left_out": [],
                    "related_candidates": [],
                    "agent_jobs": [{ "job_id": job_id, "branch_id": branch_id }],
                }),
            )?;
            // Kept last, so that nothing above can fail and leave a file no
            // Job names, which no sweep would ever find.
            let kept = keeping.is_some();
            if let Some(keeping) = keeping {
                keep_original(&dir, &job_id, keeping)?;
            }
            if let Err(e) = tx.commit() {
                if kept {
                    let _ = std::fs::remove_file(dir.join(&job_id));
                }
                return Err(failed(e));
            }
            Ok(json!({ "job_id": job_id, "branch_id": branch_id }))
        })
    }
}

/// Refused unless a file may be handed over to read this recipe again: the
/// asker may ask, and the recipe came from an Original, kept or not. A recipe
/// read from its link is read from it, and takes no file.
fn may_hand_over(what: &WhatToReadAgain) -> Result<(), OpError> {
    match what.cannot {
        None | Some(CannotReadAgain::NeedsTheFile) if what.original.is_some() => Ok(()),
        None | Some(CannotReadAgain::NothingToRead) => Err(OpError::bad_request(
            "a file or text is handed over only to read again a recipe that was read from \
             one, and this one was not. Ask without it",
        )),
        Some(cannot) => Err(cannot.refusal()),
    }
}

/// What a recipe would be read again from, and why `caller` cannot have it
/// read, if they cannot. Asked in the order a cook would put it right: who
/// they are first, then the instance, then the recipe. A recipe that came
/// from an Original is read from it, even where its Source has a link (#273):
/// that is what it was read from.
fn what_to_read_again(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    has_runner: bool,
    data_dir: &std::path::Path,
) -> Result<WhatToReadAgain, OpError> {
    let cookbook_id = ensure_sees_branch(conn, branch_id, &caller.person_id)?;
    let link: Option<String> = conn
        .query_row(
            "SELECT json_extract(versions.content, '$.source.link')
               FROM branches JOIN versions ON versions.id = branches.head_version_id
              WHERE branches.id = ?1",
            params![branch_id],
            |row| row.get(0),
        )
        .map_err(|e| OpError::internal(format!("cannot read the recipe's Source: {e}")))?;
    // Kamosu never fetched this link, so it is held to the guard every fetch
    // of Kamosu's is held to before an agent is sent to it (ADR 0033).
    let link = link.filter(|link| crate::web_import::is_readable_link(link));
    let original = jobs::original_of_recipe(conn, branch_id)?;

    let from = match &original {
        Some(original) if original_is_kept(data_dir, &original.job_id, original.deleted) => {
            Some(ReadFrom::KeptOriginal {
                job_id: original.job_id.clone(),
                kind: original.kind,
            })
        }
        Some(_) => None,
        None => link.clone().map(ReadFrom::Link),
    };
    let cannot = if !writes_in(conn, &cookbook_id, &caller.person_id)? {
        Some(CannotReadAgain::NotACoAuthor)
    } else if !cookbook_writes_branch(conn, &cookbook_id, branch_id)? {
        Some(CannotReadAgain::WouldStartACopy)
    } else if !has_runner {
        Some(CannotReadAgain::NoRunner)
    } else if from.is_some() {
        None
    } else if original.is_some() {
        Some(CannotReadAgain::NeedsTheFile)
    } else {
        Some(CannotReadAgain::NothingToRead)
    };
    Ok(WhatToReadAgain {
        link,
        original: original.map(|original| original.kind),
        from,
        cannot,
    })
}
