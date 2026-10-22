//! A Demo (ADR 0052): an instance anyone may open without signing in, where
//! every visitor is the Sample Library's Camille and only cooking and
//! shopping save.
//!
//! What a Demo refuses of the Catalogue is each declaration's `in_demo` mark,
//! met in `Core::execute`. What lives here is the rest: the way in, and the
//! refusal every route outside the Catalogue gives.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::*;

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
        "This is a demo of Kamosu, where only cooking and shopping are saved. \
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

impl Core {
    /// Whether this instance is a Demo.
    pub fn is_demo(&self) -> bool {
        self.mode == Mode::Demo
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
}
