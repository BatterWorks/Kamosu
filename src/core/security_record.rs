//! The Security Record (#371): what happened at the doors of an instance,
//! kept where a Person can read it, and written out as one log line each.
//!
//! A Person reads their own: signing in, wrong passwords at their name, their
//! passkeys, password, Access Keys and Sessions. Every Operator reads the
//! instance's: the House Notes and the public address, who administers and
//! whose account ended, Runner Keys, Backups fetched, and a crowd at the
//! sign-in.
//!
//! Each event is also one line on standard output, in the fixed format
//! `docs/security-record.md` describes. Kamosu sends it nowhere: what to do
//! with the lines is the host's business. A line names Sessions and keys,
//! never an address, since Kamosu trusts no proxy header (ADR 0033), and
//! never a Secret, a password or a name somebody typed at the door.
//!
//! The record is kept [`SECURITY_RECORD_DAYS`] and swept daily with the other things
//! Kamosu keeps for a while. It is not truth the library depends on: losing it
//! loses no recipe and no account.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::*;

/// How long an event stays in the record.
pub const SECURITY_RECORD_DAYS: i64 = 90;

/// How often a crowd at the sign-in is written down while it lasts. The
/// first refusal of a crowd is written at once; the ones after it are
/// counted in memory and written at most this often, so a crowd causes no
/// write per request it sends (ADR 0032).
const CROWD_WRITTEN_EVERY: Duration = Duration::from_secs(60);

/// A run of wrong passwords at one name, or one crowd at the sign-in, is one
/// row while it lasts. It ends an hour after its last, the hour a count of
/// wrong passwords is kept (#373).
const RUN_ENDS_AFTER_SECONDS: i64 = 60 * 60;

/// Every event the record holds, by the name the log line and both Doors use.
pub const SECURITY_EVENTS: &[&str] = &[
    // A Person's own.
    "signed_in",
    "sign_in_failed",
    "passkey_added",
    "passkey_removed",
    "password_changed",
    "access_key_minted",
    "access_key_ended",
    "session_ended",
    // Every Operator's.
    "house_notes_changed",
    "public_address_changed",
    "operator_raised",
    "operator_stood_down",
    "account_disabled",
    "account_deleted",
    "operator_key_minted",
    "sign_in_crowd",
    "runner_key_minted",
    "runner_key_revoked",
    "backup_fetched",
];

/// Who reads an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Readers {
    /// The Person whose account it happened to.
    ThePerson,
    /// Every Operator.
    Operators,
}

/// The Credential an event names: the one that acted, or the one it was
/// about, such as the Session a sign-in made or the key that was ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Named {
    /// `session`, `access_key`, `operator_key`, `job_key`, `runner_key` or
    /// `passkey`.
    pub kind: &'static str,
    pub name: String,
}

/// One event, as it is noted.
#[derive(Clone, Debug)]
pub(super) struct Event {
    pub event: &'static str,
    pub readers: Readers,
    /// The Person whose own record this is, for [`Readers::ThePerson`].
    pub account_id: Option<String>,
    /// The Person who acted, where one did.
    pub by_id: Option<String>,
    /// The Person acted on: raised, stood down, disabled or deleted.
    pub subject_id: Option<String>,
    pub credential: Option<Named>,
    /// What else the row says, by event. `None` holds nothing.
    pub detail: Option<Value>,
}

impl Event {
    /// An event in a Person's own record, which they did themselves or which
    /// was done to their account.
    pub fn theirs(event: &'static str, person_id: &str) -> Event {
        Event {
            event,
            readers: Readers::ThePerson,
            account_id: Some(person_id.to_string()),
            by_id: Some(person_id.to_string()),
            subject_id: None,
            credential: None,
            detail: None,
        }
    }

    /// An event every Operator reads, done through this Credential.
    pub fn instance(event: &'static str, by: Option<&Caller>) -> Event {
        Event {
            event,
            readers: Readers::Operators,
            account_id: None,
            by_id: by.map(|caller| caller.person_id.clone()),
            subject_id: None,
            credential: None,
            detail: None,
        }
    }

    pub fn naming(mut self, credential: Option<Named>) -> Event {
        self.credential = credential;
        self
    }

    pub fn about(mut self, subject_id: &str) -> Event {
        self.subject_id = Some(subject_id.to_string());
        self
    }

    pub fn saying(mut self, detail: Value) -> Event {
        self.detail = Some(detail);
        self
    }
}

/// What the record holds in memory: a crowd at the sign-in counted since it
/// was last written down.
#[derive(Default)]
pub(super) struct Watch {
    crowd: Mutex<Crowd>,
}

#[derive(Default)]
struct Crowd {
    /// Refusals not yet written down.
    unwritten: i64,
    /// When the last of them was refused, as the record writes a time.
    last_refused_at: Option<String>,
    /// When the crowd was last written down.
    written_at: Option<Instant>,
}

impl Core {
    /// Note one event: a row in the record and a line on standard output. A
    /// fault writing the row is logged and let go, since the act it records
    /// has already happened; the line is written either way.
    pub(super) fn note(&self, event: Event) {
        let at = self.note_row(&event);
        self.say(&event, &at, 1);
    }

    /// Say an event in the log, as every one the Security Record notes is,
    /// unless this Core keeps quiet (#404).
    fn say(&self, event: &Event, at: &str, count: i64) {
        if self.says_security_lines {
            log_line(event, at, count);
        }
    }

    /// Write an event's row alone, and answer the time it was written at.
    fn note_row(&self, event: &Event) -> String {
        let at = now_text();
        let written = self.db().with_conn(|conn| write(conn, event, &at));
        if let Err(error) = written {
            tracing::warn!(target: "kamosu::security", %error, event = event.event, "cannot write to the security record");
        }
        at
    }

    /// Note an Access Key minted: in its Person's own record, and in every
    /// Operator's as well when it is an Operator key, naming the Credential
    /// `by` that minted it. One act, so one line.
    pub(super) fn note_key_minted(&self, by: Option<&Caller>, person_id: &str, key: &AccessKey) {
        let named = Named {
            kind: if key.operator {
                "operator_key"
            } else {
                "access_key"
            },
            name: key.name.clone(),
        };
        let detail = json!({ "read_only": key.read_only, "operator": key.operator });
        self.note(
            Event::theirs("access_key_minted", person_id)
                .naming(Some(named.clone()))
                .saying(detail.clone()),
        );
        if key.operator {
            let mut event = Event::instance("operator_key_minted", by)
                .naming(by.and_then(|caller| self.named_credential(caller)))
                .saying(json!({ "key": key.name, "read_only": key.read_only }));
            event.by_id = Some(person_id.to_string());
            self.note_row(&event);
        }
    }

    /// One more wrong password at a Person's name: the run of them grows by
    /// one, or a new one starts. `in_a_row` is how many there have been since
    /// the name's last right password, so one means a new run.
    pub(super) fn note_wrong_password(&self, person_id: &str, in_a_row: i64) {
        let at = now_text();
        let event = Event::theirs("sign_in_failed", person_id);
        let counted = self.db().with_conn(|conn| {
            let grown = (in_a_row > 1)
                .then(|| grow_run(conn, "sign_in_failed", Some(person_id), &at, 1))
                .transpose()?
                .flatten();
            match grown {
                Some(count) => Ok(count),
                None => write(conn, &event, &at).map(|()| 1),
            }
        });
        let count = counted.unwrap_or_else(|error| {
            tracing::warn!(target: "kamosu::security", %error, "cannot write to the security record");
            in_a_row
        });
        self.say(&event, &at, count);
    }

    /// A sign-in refused because the line for password work was full: a
    /// crowd of tries, most likely at made-up names. Written down at once
    /// when a crowd starts, then at most every [`CROWD_WRITTEN_EVERY`]: by
    /// the next refusal, or by the watch that writes what is owed.
    pub(super) fn note_crowd(&self) {
        let mut crowd = self
            .security
            .crowd
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        crowd.unwritten += 1;
        crowd.last_refused_at = Some(now_text());
        if crowd
            .written_at
            .is_some_and(|written| written.elapsed() < CROWD_WRITTEN_EVERY)
        {
            return;
        }
        drop(crowd);
        self.write_crowd_owed(Duration::ZERO);
    }

    /// Write down the refusals a crowd has not yet been written down for,
    /// once `every` has passed since it last was. The watch asks every few
    /// seconds, so the tail of a crowd is written within a minute of its
    /// last refusal, at the time of that refusal. A restart loses what was
    /// still owed.
    fn write_crowd_owed(&self, every: Duration) {
        let (refusals, at) = {
            let mut crowd = self
                .security
                .crowd
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if crowd.unwritten == 0
                || crowd
                    .written_at
                    .is_some_and(|written| written.elapsed() < every)
            {
                return;
            }
            crowd.written_at = Some(Instant::now());
            (
                std::mem::take(&mut crowd.unwritten),
                crowd.last_refused_at.take().unwrap_or_else(now_text),
            )
        };
        let event = Event::instance("sign_in_crowd", None);
        let counted = self.db().with_conn(|conn| {
            match grow_run(conn, "sign_in_crowd", None, &at, refusals)? {
                Some(count) => Ok(count),
                None => {
                    write(conn, &event, &at)?;
                    if refusals > 1 {
                        grow_run(conn, "sign_in_crowd", None, &at, refusals - 1)?;
                    }
                    Ok(refusals)
                }
            }
        });
        let count = counted.unwrap_or_else(|error| {
            tracing::warn!(target: "kamosu::security", %error, "cannot write to the security record");
            refusals
        });
        self.say(&event, &at, count);
    }

    /// A password-work refusal is a crowd at the sign-in when the line was
    /// full. Anything else passes through untouched.
    pub(super) fn noting_a_crowd<T>(&self, result: Result<T, OpError>) -> Result<T, OpError> {
        if let Err(error) = &result
            && error.kind == ErrorKind::Busy
            && error.retry_after_seconds.is_none()
        {
            self.note_crowd();
        }
        result
    }

    /// The Credential a Caller presented, by kind and the name it goes by.
    pub(super) fn named_credential(&self, caller: &Caller) -> Option<Named> {
        self.db()
            .with_conn(|conn| credential_of(conn, caller))
            .ok()
            .flatten()
    }

    /// A Person's own record, newest first.
    pub fn security_record_of(&self, person_id: &str) -> Result<Value, OpError> {
        self.db()
            .with_conn(|conn| read(conn, Readers::ThePerson, Some(person_id)))
    }

    /// The instance's record, every Operator's, newest first.
    pub fn instance_security_record(&self) -> Result<Value, OpError> {
        self.db()
            .with_conn(|conn| read(conn, Readers::Operators, None))
    }

    /// Forget what is older than [`SECURITY_RECORD_DAYS`]. Run daily, beside the
    /// sweep of Originals; answers how many events went.
    pub fn sweep_security_record(&self) -> Result<usize, OpError> {
        self.db().with_conn(|conn| {
            conn.execute(
                "DELETE FROM security_record WHERE at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
                params![format!("-{SECURITY_RECORD_DAYS} days")],
            )
            .map_err(|e| OpError::internal(format!("cannot sweep the security record: {e}")))
        })
    }
}

/// Keep writing down what a crowd at the sign-in is owed, for as long as
/// the instance serves (#371).
pub(super) fn spawn_crowd_watch(core: Arc<Core>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            let core = core.clone();
            let _ = tokio::task::spawn_blocking(move || {
                core.write_crowd_owed(CROWD_WRITTEN_EVERY);
            })
            .await;
        }
    });
}

/// Write one event's row.
fn write(conn: &Connection, event: &Event, at: &str) -> Result<(), OpError> {
    let detail = event.detail.as_ref().map(|detail| detail.to_string());
    conn.execute(
        "INSERT INTO security_record \
           (event, for_operators, account_id, by_id, subject_id, credential_kind, \
            credential_name, detail, first_at, at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            event.event,
            (event.readers == Readers::Operators) as i64,
            event.account_id,
            event.by_id,
            event.subject_id,
            event.credential.as_ref().map(|named| named.kind),
            event.credential.as_ref().map(|named| named.name.as_str()),
            detail,
            at,
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot write to the security record: {e}")))?;
    Ok(())
}

/// Add `more` to the newest row of a run that has not ended, and answer its
/// count; nothing when there is none to add to. A run ends an hour after its
/// last, and a run of wrong passwords ends too at its Person's next sign-in,
/// whether by password or not.
fn grow_run(
    conn: &Connection,
    event: &str,
    account_id: Option<&str>,
    at: &str,
    more: i64,
) -> Result<Option<i64>, OpError> {
    conn.query_row(
        "UPDATE security_record SET count = count + ?4, at = ?3 \
          WHERE id = (SELECT id FROM security_record \
                       WHERE event = ?1 AND account_id IS ?2 \
                         AND at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?5) \
                         AND NOT EXISTS (SELECT 1 FROM security_record AS later \
                                          WHERE later.account_id = ?2 \
                                            AND later.event = 'signed_in' \
                                            AND later.id > security_record.id) \
                       ORDER BY id DESC LIMIT 1) \
         RETURNING count",
        params![
            event,
            account_id,
            at,
            more,
            format!("-{RUN_ENDS_AFTER_SECONDS} seconds")
        ],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot write to the security record: {e}")))
}

/// The kind and the name of the Credential a Caller presented, as it is
/// called now.
fn credential_of(conn: &Connection, caller: &Caller) -> Result<Option<Named>, OpError> {
    let name_of = |sql: &str, id: &str| -> Result<Option<String>, OpError> {
        conn.query_row(sql, params![id], |row| row.get(0))
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read a Credential's name: {e}")))
    };
    if let Some(job_id) = &caller.job_id {
        return Ok(Some(Named {
            kind: "job_key",
            name: job_id.clone(),
        }));
    }
    if let Some(session_id) = &caller.session_id {
        return Ok(
            name_of("SELECT name FROM sessions WHERE id = ?1", session_id)?.map(|name| Named {
                kind: "session",
                name,
            }),
        );
    }
    if let Some(key_id) = &caller.access_key_id {
        return Ok(
            name_of("SELECT name FROM access_keys WHERE id = ?1", key_id)?.map(|name| Named {
                kind: if caller.is_operator {
                    "operator_key"
                } else {
                    "access_key"
                },
                name,
            }),
        );
    }
    Ok(None)
}

/// One record, newest first, within [`SECURITY_RECORD_DAYS`].
fn read(conn: &Connection, readers: Readers, account_id: Option<&str>) -> Result<Value, OpError> {
    let fail =
        |e: rusqlite::Error| OpError::internal(format!("cannot read the security record: {e}"));
    let mut statement = conn
        .prepare(
            "SELECT security_record.id, event, first_at, at, count, \
                    by_person.name, subject.name, credential_kind, credential_name, detail \
               FROM security_record \
               LEFT JOIN people AS by_person ON by_person.id = security_record.by_id \
               LEFT JOIN people AS subject ON subject.id = security_record.subject_id \
              WHERE for_operators = ?1 AND (?2 IS NULL OR account_id = ?2) \
                AND at >= strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?3) \
              ORDER BY at DESC, security_record.id DESC",
        )
        .map_err(fail)?;
    let events = statement
        .query_map(
            params![
                (readers == Readers::Operators) as i64,
                account_id,
                format!("-{SECURITY_RECORD_DAYS} days")
            ],
            |row| {
                let kind: Option<String> = row.get(7)?;
                let name: Option<String> = row.get(8)?;
                let detail: Option<String> = row.get(9)?;
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "event": row.get::<_, String>(1)?,
                    "first_at": row.get::<_, String>(2)?,
                    "at": row.get::<_, String>(3)?,
                    "count": row.get::<_, i64>(4)?,
                    "by": row.get::<_, Option<String>>(5)?,
                    "about": row.get::<_, Option<String>>(6)?,
                    "credential": kind.map(|kind| json!({ "kind": kind, "name": name })),
                    "detail": detail
                        .and_then(|detail| serde_json::from_str::<Value>(&detail).ok())
                        .unwrap_or_else(|| json!({})),
                }))
            },
        )
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    Ok(json!({ "events": events, "kept_for_days": SECURITY_RECORD_DAYS }))
}

/// The line an event writes on standard output. The format is fixed and
/// documented in `docs/security-record.md`: every field, always, in this
/// order, `-` where it has nothing. A name is written as a JSON string, so
/// a name holding a space, a quote or a line break stays one field of one
/// line.
pub(super) fn line_for(event: &Event, at: &str, count: i64) -> String {
    let or_dash = |value: Option<&str>| value.unwrap_or("-").to_string();
    let person = event.account_id.as_deref().or(event.by_id.as_deref());
    format!(
        "kamosu-security event={} at={} person={} subject={} credential={} credential_name={} count={}",
        event.event,
        at,
        or_dash(person),
        or_dash(event.subject_id.as_deref()),
        or_dash(event.credential.as_ref().map(|named| named.kind)),
        event
            .credential
            .as_ref()
            .map(|named| Value::String(named.name.clone()).to_string())
            .unwrap_or_else(|| "-".to_string()),
        count,
    )
}

fn log_line(event: &Event, at: &str, count: i64) {
    println!("{}", line_for(event, at, count));
}

/// Now, as the database writes a time: `2026-10-20T09:15:02.123Z`, in UTC.
fn now_text() -> String {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = since.as_secs() as i64;
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60,
        since.subsec_millis(),
    )
}

/// The calendar date `days` after 1970-01-01, by Howard Hinnant's
/// `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_core() -> (tempfile::TempDir, Core) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(crate::db::Db::open(dir.path()).unwrap());
        (dir, Core::open(db))
    }

    fn crowds(core: &Core) -> Vec<i64> {
        core.instance_security_record().unwrap()["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["count"].as_i64().unwrap())
            .collect()
    }

    fn written(core: &Core) -> Vec<i64> {
        core.db()
            .with_conn(|conn| {
                let mut statement = conn
                    .prepare("SELECT count FROM security_record ORDER BY id")
                    .unwrap();
                Ok(statement
                    .query_map([], |row| row.get(0))
                    .unwrap()
                    .collect::<Result<Vec<i64>, _>>()
                    .unwrap())
            })
            .unwrap()
    }

    #[test]
    fn a_crowd_is_written_down_at_its_start_and_its_tail_by_the_watch() {
        let (_dir, core) = a_core();
        core.note_crowd();
        assert_eq!(written(&core), vec![1]);
        for _ in 0..40 {
            core.note_crowd();
        }
        // Counted in memory, and written no more often than once a minute:
        // reading the record writes nothing.
        assert_eq!(crowds(&core), vec![1]);
        core.write_crowd_owed(CROWD_WRITTEN_EVERY);
        assert_eq!(written(&core), vec![1]);
        // The watch writes the tail once the minute is up, onto the same run.
        core.write_crowd_owed(Duration::ZERO);
        assert_eq!(written(&core), vec![41]);
        core.write_crowd_owed(Duration::ZERO);
        assert_eq!(written(&core), vec![41]);
    }

    #[test]
    fn a_sign_in_of_any_kind_ends_a_run_of_wrong_passwords() {
        let (_dir, core) = a_core();
        let person = core.create_person("Camille").unwrap();
        core.note_wrong_password(&person, 1);
        core.note_wrong_password(&person, 2);
        core.note(Event::theirs("signed_in", &person).saying(json!({ "how": "passkey" })));
        // The name's count was not cleared by the passkey, and still the run
        // that was going is over.
        core.note_wrong_password(&person, 3);
        assert_eq!(written(&core), vec![2, 1, 1]);
    }

    #[test]
    fn only_a_full_line_is_a_crowd() {
        let (_dir, core) = a_core();
        let _ = core.noting_a_crowd::<()>(Err(OpError::bad_request("no")));
        assert!(crowds(&core).is_empty());
        let _ = core.noting_a_crowd::<()>(Err(OpError::busy()));
        assert_eq!(crowds(&core), vec![1]);
    }

    #[test]
    fn a_day_count_is_the_calendar_date() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_746), (2026, 10, 20));
    }

    #[test]
    fn now_is_written_as_the_database_writes_it() {
        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Db::open(db_dir.path()).unwrap();
        let sqlite: String = db
            .with_conn(|conn| {
                conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
                    row.get(0)
                })
                .map_err(|e| OpError::internal(e.to_string()))
            })
            .unwrap();
        let ours = now_text();
        assert_eq!(ours.len(), sqlite.len(), "{ours} against {sqlite}");
        // The same shape, `YYYY-MM-DDTHH:MM:SS.mmmZ`, on the same day.
        let shape = |text: &str| {
            text.chars()
                .map(|c| if c.is_ascii_digit() { '9' } else { c })
                .collect::<String>()
        };
        assert_eq!(shape(&ours), shape(&sqlite));
        assert_eq!(ours[..10], sqlite[..10], "{ours} against {sqlite}");
    }

    #[test]
    fn a_line_is_every_field_in_order_and_a_name_cannot_break_it() {
        let event = Event::theirs("signed_in", "p_1").naming(Some(Named {
            kind: "session",
            name: "Firefox\nkamosu-security event=forged".to_string(),
        }));
        let line = line_for(&event, "2026-10-20T09:15:02.123Z", 1);
        assert_eq!(
            line,
            "kamosu-security event=signed_in at=2026-10-20T09:15:02.123Z person=p_1 \
             subject=- credential=session \
             credential_name=\"Firefox\\nkamosu-security event=forged\" count=1"
        );
        assert_eq!(line.lines().count(), 1);
    }

    #[test]
    fn a_line_with_nothing_to_name_says_so_with_dashes() {
        let line = line_for(
            &Event::instance("sign_in_crowd", None),
            "2026-10-20T09:15:02.123Z",
            40,
        );
        assert_eq!(
            line,
            "kamosu-security event=sign_in_crowd at=2026-10-20T09:15:02.123Z person=- subject=- \
             credential=- credential_name=- count=40"
        );
    }
}
