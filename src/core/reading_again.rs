//! Reading a recipe again: a Co-author asks for an agent to read a recipe's
//! Source link once more, whichever way the recipe arrived (ADR 0048, #272).
//!
//! The Job is the one a link landing bare makes (#261), on a recipe that is
//! already there. It is its own Import, so the cook reads what the agent did
//! in an Import Report, where the recipe is listed as an arrival.

use super::*;

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
    /// Its Source holds no link an agent may be sent to: none, or one that
    /// is no http or https address on the internet.
    NothingToRead,
}

impl CannotReadAgain {
    fn reason(self) -> &'static str {
        match self {
            CannotReadAgain::NotACoAuthor => "not_a_co_author",
            CannotReadAgain::WouldStartACopy => "would_start_a_copy",
            CannotReadAgain::NoRunner => "no_runner",
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
            CannotReadAgain::NothingToRead => OpError::refused_because(
                reason,
                "this recipe's Source holds no link an agent can be sent to read, which is an \
                 http or https address on the internet, so there is nothing to read it again \
                 from",
            ),
        }
    }
}

impl Core {
    /// Whether `caller` may have a recipe read again, and from what (#272).
    /// A recipe they do not see is not there (ADR 0040).
    pub fn can_read_recipe_again(
        &self,
        caller: &Caller,
        branch_id: &str,
    ) -> Result<Value, OpError> {
        let has_runner = self.has_runner()?;
        self.db().with_conn(|conn| {
            let (link, cannot) = what_to_read_again(conn, caller, branch_id, has_runner)?;
            Ok(json!({
                "can_read_again": cannot.is_none(),
                "link": link,
                "reason": cannot.map(CannotReadAgain::reason),
            }))
        })
    }

    /// Have an agent read a recipe again from its Source link (#272). Only a
    /// Co-author of the Cookbook that holds it may ask, on an instance with a
    /// Runner. The answer is the Job's id. Where a Job waits or is being
    /// worked on the recipe already, that Job is the answer and no second is
    /// made.
    ///
    /// The Job's result is an Import Report from the moment it is asked for,
    /// under the source `read_again`, with the recipe listed as an arrival.
    /// What the agent saves is one new Version under the asker's Hand (#265).
    pub fn read_recipe_again(&self, caller: &Caller, branch_id: &str) -> Result<Value, OpError> {
        let has_runner = self.has_runner()?;
        let failed = |e| OpError::internal(format!("cannot record the agent's Job: {e}"));
        self.db().with_conn(|conn| {
            let (link, cannot) = what_to_read_again(conn, caller, branch_id, has_runner)?;
            if let Some(cannot) = cannot {
                return Err(cannot.refusal());
            }
            let link = link.ok_or_else(|| CannotReadAgain::NothingToRead.refusal())?;
            if let Some(job_id) = jobs::standing_agent_job(conn, branch_id)? {
                return Ok(json!({ "job_id": job_id, "branch_id": branch_id }));
            }

            let cookbook_id = branch_cookbook(conn, branch_id)?;
            let tx = conn.unchecked_transaction().map_err(failed)?;
            let import_id = find_or_create_import(&tx, &cookbook_id, jobs::READ_AGAIN)?;
            // Listed with the status that names the ask, spelt as its source.
            let arrival = imports::arrival_as_it_reads(&tx, branch_id, &link, "read_again")?
                .ok_or_else(no_such_branch)?;
            let job_id = jobs::wait_for_agent_to_read_again(&tx, caller, branch_id, &link)?;
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
            tx.commit().map_err(failed)?;
            Ok(json!({ "job_id": job_id, "branch_id": branch_id }))
        })
    }
}

/// The link a recipe would be read again from, and why `caller` cannot have
/// it read, if they cannot. Asked in the order a cook would put it right:
/// who they are first, then the instance, then the recipe.
fn what_to_read_again(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
    has_runner: bool,
) -> Result<(Option<String>, Option<CannotReadAgain>), OpError> {
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

    let cannot = if !writes_in(conn, &cookbook_id, &caller.person_id)? {
        Some(CannotReadAgain::NotACoAuthor)
    } else if !cookbook_writes_branch(conn, &cookbook_id, branch_id)? {
        Some(CannotReadAgain::WouldStartACopy)
    } else if !has_runner {
        Some(CannotReadAgain::NoRunner)
    } else if link.is_none() {
        Some(CannotReadAgain::NothingToRead)
    } else {
        None
    };
    Ok((link, cannot))
}
