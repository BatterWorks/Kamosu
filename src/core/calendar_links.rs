//! A Calendar Link (#491): the private address a cook gives their phone's
//! calendar app, which then reads their Meal Plan with it and no sign-in.
//!
//! It is a key of its own kind, as an Add-only key is (ADR 0055): one per
//! Person, its Secret kept by its hash alone and shown once (ADR 0031), ended
//! at any time, and good for one thing, the calendar file. Making and ending
//! one each write a line in the Security Record.

use super::*;
use crate::calendar_file::{self, CalendarEvent, When};

/// How far back the file reaches: the past 90 days, as Mealie and Tandoor
/// carry, so last week's dinners stay on the phone and the file stays small.
pub const DAYS_BACK: i64 = 90;

impl Core {
    /// Make the caller's Calendar Link, ending the one before, and answer its
    /// Secret this once. `url` is the whole address where the instance has a
    /// public address, and null where it has none: the screen builds it from
    /// the address it was opened at.
    pub fn make_calendar_link(&self, caller: &Caller) -> Result<Value, OpError> {
        let person_id = &caller.person_id;
        let secret = generate_secret();
        let id = format!("cl_{}", hex::encode(random_bytes(8)));
        let (made_at, ended_before) = self.db().with_conn(|conn| {
            let ended_before = end_live(conn, person_id)?;
            conn.execute(
                "INSERT INTO calendar_links (id, person_id, secret_hash) VALUES (?1, ?2, ?3)",
                params![id, person_id, hash_secret(&secret)],
            )
            .map_err(|e| OpError::internal(format!("cannot make the Calendar Link: {e}")))?;
            let made_at: String = conn
                .query_row(
                    "SELECT made_at FROM calendar_links WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read the Calendar Link: {e}")))?;
            Ok((made_at, ended_before))
        })?;
        let mut made =
            Event::theirs("calendar_link_made", person_id).naming(self.named_credential(caller));
        if ended_before {
            made = made.saying(json!({ "replaced": true }));
        }
        self.note(made);
        let path = calendar_path(&secret);
        let url = self
            .public_address()
            .map(|address| format!("{}{path}", address.trim_end_matches('/')));
        Ok(json!({ "secret": secret, "path": path, "url": url, "made_at": made_at }))
    }

    /// Whether the caller has a live Calendar Link, when it was made and when
    /// a calendar app last read it. Never its Secret, which nothing keeps.
    pub fn get_calendar_link(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let live: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT made_at, last_read_at FROM calendar_links \
                      WHERE person_id = ?1 AND ended_at IS NULL",
                    params![person_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Calendar Link: {e}")))?;
            Ok(json!({
                "calendar_link": live.map(|(made_at, last_read_at)| json!({
                    "made_at": made_at,
                    "last_read_at": last_read_at,
                })),
            }))
        })
    }

    /// End the caller's Calendar Link. Permanent: the calendar app that holds
    /// it is answered nothing from now on, and a new one is made afresh.
    pub fn end_calendar_link(&self, caller: &Caller) -> Result<Value, OpError> {
        let person_id = &caller.person_id;
        let ended = self.db().with_conn(|conn| end_live(conn, person_id))?;
        if ended {
            self.note(
                Event::theirs("calendar_link_ended", person_id)
                    .naming(self.named_credential(caller)),
            );
        }
        Ok(json!({ "ended": ended }))
    }

    /// The calendar file a live Calendar Link reads: its Person's Planned
    /// Meals from `DAYS_BACK` days ago on, as iCalendar text. `None` for a
    /// Secret that names no live link, or a Person who can no longer sign in,
    /// which the route answers as nothing there.
    ///
    /// Holding the Secret is the whole of the permission, as it is for a
    /// Share Link, and the check is here rather than in the route.
    pub fn calendar_file(&self, secret: &str) -> Result<Option<String>, OpError> {
        let public_address = self.public_address();
        self.db().with_conn(|conn| {
            let found: Option<(String, String, String)> = conn
                .query_row(
                    "SELECT calendar_links.id, people.id, people.name \
                       FROM calendar_links JOIN people ON people.id = calendar_links.person_id \
                      WHERE calendar_links.secret_hash = ?1 AND calendar_links.ended_at IS NULL \
                        AND people.disabled = 0 AND people.deleted = 0",
                    params![hash_secret(secret)],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the Calendar Link: {e}")))?;
            let Some((link_id, person_id, name)) = found else {
                return Ok(None);
            };
            conn.execute(
                "UPDATE calendar_links \
                    SET last_read_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
                params![link_id],
            )
            .map_err(|e| OpError::internal(format!("cannot note the Calendar Link read: {e}")))?;
            let events = meal_plan::meals_since(conn, &person_id, DAYS_BACK)?
                .iter()
                .map(|meal| event_of(meal, public_address.as_deref()))
                .collect::<Vec<_>>();
            Ok(Some(calendar_file::write(
                &format!("Kamosu · {name}"),
                &events,
                &security_record::now_text(),
            )))
        })
    }
}

/// Where a Calendar Link's file is, under the instance's own address.
pub fn calendar_path(secret: &str) -> String {
    format!("/calendar/{secret}.ics")
}

/// End a Person's live link, if there is one, and say whether there was.
fn end_live(conn: &Connection, person_id: &str) -> Result<bool, OpError> {
    conn.execute(
        "UPDATE calendar_links SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
          WHERE person_id = ?1 AND ended_at IS NULL",
        params![person_id],
    )
    .map(|ended| ended > 0)
    .map_err(|e| OpError::internal(format!("cannot end the Calendar Link: {e}")))
}

/// One Planned Meal as a calendar event: named by its own id, which a move
/// keeps, so a calendar app moves the event rather than adding a second.
fn event_of(meal: &Value, public_address: Option<&str>) -> CalendarEvent {
    let id = meal["planned_meal_id"].as_str().unwrap_or_default();
    let title = meal["words"]
        .as_str()
        .or(meal["title"].as_str())
        .unwrap_or_default()
        .to_string();
    let link = match (
        meal["branch_id"].as_str(),
        meal["gone"].as_bool(),
        public_address,
    ) {
        (Some(branch_id), Some(false), Some(address)) => Some(format!(
            "{}/recipes/{branch_id}",
            address.trim_end_matches('/')
        )),
        _ => None,
    };
    let when = match meal["day"].as_str() {
        Some(day) => When::Day(day.to_string()),
        None => When::Week(meal["week"].as_str().unwrap_or_default().to_string()),
    };
    CalendarEvent {
        uid: format!("{id}@kamosu"),
        title,
        when,
        link,
    }
}
