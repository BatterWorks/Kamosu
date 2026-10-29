//! Everything a Person sent to Kamosu's AI agent, in one list, and how each
//! one ended (#463), as Settings' *Sent to the AI agent* shows it. That is a
//! link, pasted text, a PDF, photos, a recipe read again, and whatever a
//! phone's Share menu sent, which arrives as one of those.
//!
//! Brought in (`list_imports`, #108) lists the same Jobs under the source
//! each came in by, and a row there is a count of recipes. This list is the
//! other question: what went to the agent, and what had problems. So it
//! names what was sent as far as Kamosu holds it, the recipe that came of
//! it, and a failure's reason in the agent's own words, on the row itself.

use super::originals::{first_words_of, has_preview, original_is_kept, preview_address};
use super::reading_again::may_read_again;
use super::*;
use crate::jobs::{JobStatus, OriginalKind};

/// One agent's Job as the database hands it over.
struct SentRow {
    job_id: String,
    status: JobStatus,
    created_at: String,
    updated_at: String,
    error: Option<String>,
    /// The source its own Import is listed under, or none for a Job on a
    /// recipe an Import landed bare, which belongs to that Import.
    source_kind: Option<String>,
    link: Option<String>,
    original: Option<String>,
    pages: i64,
    original_deleted: bool,
    branch_id: Option<String>,
    /// The agent said what it was handed holds no recipe, so `error` is
    /// its own words (#276).
    could_not_be_read: bool,
}

impl Core {
    /// Every agent's Job `caller` asked for, newest first, each with what
    /// was sent, how it stands or ended, and what the cook can do next where
    /// it failed. Only the caller's own, as Brought in lists them: an
    /// Operator's reach does not stretch to other people's Jobs here.
    pub fn list_agent_jobs(&self, caller: &Caller) -> Result<Value, OpError> {
        let has_runner = self.has_runner()?;
        let data_dir = self.data_dir();
        self.db().with_conn(|conn| {
            let failed = |e| OpError::internal(format!("cannot list the agent's Jobs: {e}"));
            let rows: Vec<SentRow> = conn
                .prepare(&format!(
                    "SELECT jobs.id, jobs.status, jobs.created_at, jobs.updated_at, jobs.error,
                            json_extract(jobs.input, '$.source_kind'),
                            agent_jobs.link, agent_jobs.original, agent_jobs.original_pages,
                            agent_jobs.original_deleted_at IS NOT NULL, agent_jobs.branch_id,
                            agent_jobs.outcome IS 'could_not_be_read'
                       FROM jobs JOIN agent_jobs ON agent_jobs.job_id = jobs.id
                      WHERE jobs.person_id = ?1 AND jobs.operation = '{}'
                      ORDER BY jobs.created_at DESC, jobs.rowid DESC",
                    jobs::AGENT_OPERATION
                ))
                .map_err(failed)?
                .query_map(params![caller.person_id], |row| {
                    Ok(SentRow {
                        job_id: row.get(0)?,
                        status: JobStatus::from_column(row.get(1)?),
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        error: row.get(4)?,
                        source_kind: row.get(5)?,
                        link: row.get(6)?,
                        original: row.get(7)?,
                        pages: row.get(8)?,
                        original_deleted: row.get(9)?,
                        branch_id: row.get(10)?,
                        could_not_be_read: row.get(11)?,
                    })
                })
                .map_err(failed)?
                .collect::<Result<_, _>>()
                .map_err(failed)?;

            let mut sent = Vec::with_capacity(rows.len());
            for row in rows {
                sent.push(sent_row(conn, caller, row, has_runner, &data_dir)?);
            }
            Ok(json!({ "jobs": sent }))
        })
    }
}

/// One row of the list: what was sent, how it ended, and what next.
fn sent_row(
    conn: &Connection,
    caller: &Caller,
    row: SentRow,
    has_runner: bool,
    data_dir: &std::path::Path,
) -> Result<Value, OpError> {
    let original = row.original.as_deref().and_then(OriginalKind::named);
    let kept =
        original.is_none_or(|_| original_is_kept(data_dir, &row.job_id, row.original_deleted));
    let recipe = match &row.branch_id {
        Some(branch_id) => recipe_as_it_reads(conn, caller, branch_id)?,
        None => None,
    };
    let ended = matches!(
        row.status,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    );
    let read_again = row.source_kind.as_deref() == Some(jobs::READ_AGAIN);
    let failed = row.status == JobStatus::Failed;
    let in_agents_words = failed && row.could_not_be_read;

    // What the cook can do about a failure, as the Operations that do it
    // would answer them: a recipe that is there is read again, and a link
    // that made nothing is sent again or its text pasted instead. A photo or
    // a PDF the agent said holds no recipe is no use sent again, so another
    // is chosen. Any other file the agent made nothing of is sent again
    // while Kamosu keeps it. With no Runner, a link's text can still be
    // pasted, and nothing else is offered. A recipe read again that the
    // caller no longer sees is offered nothing.
    let mut next = Vec::new();
    if failed {
        match (&recipe, &row.branch_id, original) {
            // Whatever stands in the way of reading it again, the recipe is
            // what is offered, and nothing else.
            (Some(_), Some(branch_id), _) => next.extend(
                may_read_again(conn, caller, branch_id, has_runner, data_dir)
                    .then_some("read_recipe_again"),
            ),
            (None, Some(_), _) if read_again => {}
            (_, _, None) => {
                if has_runner {
                    next.push("send_link_again");
                }
                next.push("paste_instead");
            }
            (_, None, Some(kind)) if has_runner => {
                if in_agents_words && kind != OriginalKind::PastedText {
                    next.push("choose_another");
                } else if kept {
                    next.push("send_again");
                }
            }
            _ => {}
        }
    }

    // A recipe read again was read from what somebody handed over first,
    // who may be another Co-author: the row names the recipe, and shows
    // nothing of what they handed over.
    let shows_what_was_sent = kept && !read_again;
    Ok(json!({
        "job_id": row.job_id,
        "created_at": row.created_at,
        "ended_at": ended.then_some(row.updated_at),
        "status": row.status.as_str(),
        "kind": original.map_or("link", OriginalKind::as_str),
        "pages": row.pages.max(1),
        // A recipe read again from its Source names that link only while
        // the caller sees the recipe (ADR 0040).
        "link": if read_again && recipe.is_none() { None } else { row.link },
        "kept": kept,
        "first_words": match original {
            Some(kind) if shows_what_was_sent => first_words_of(data_dir, &row.job_id, kind),
            _ => None,
        },
        "preview": (original == Some(OriginalKind::Picture)
            && shows_what_was_sent
            && has_preview(data_dir, &row.job_id))
            .then(|| preview_address(&row.job_id)),
        "read_again": read_again,
        "fills_in": row.source_kind.is_none(),
        "recipe": recipe,
        "reason": failed.then_some(row.error).flatten(),
        "in_agents_words": in_agents_words,
        "next": next,
    }))
}

/// The recipe a Job made or worked on, by its name now, where it is there
/// and the caller sees it.
fn recipe_as_it_reads(
    conn: &Connection,
    caller: &Caller,
    branch_id: &str,
) -> Result<Option<Value>, OpError> {
    if ensure_sees_branch(conn, branch_id, &caller.person_id).is_err() {
        return Ok(None);
    }
    let title: Option<Option<String>> = conn
        .query_row(
            "SELECT json_extract(versions.content, '$.title')
               FROM branches JOIN versions ON versions.id = branches.head_version_id
              WHERE branches.id = ?1",
            params![branch_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the agent's recipe: {e}")))?;
    Ok(title.map(|title| json!({ "branch_id": branch_id, "title": title.unwrap_or_default() })))
}
