//! The calendar file a Calendar Link answers (#491): a cook's Meal Plan as
//! iCalendar text (RFC 5545), which a phone's calendar app subscribes to.
//!
//! **This route is not an Operation**, as the Share Link page is not
//! (`share_page.rs`). A calendar app sends no Credential and reads one fixed
//! format, so the Secret in the address is the whole of the permission, and
//! whether it is live is answered by the Core (`Core::calendar_file`). The
//! Operations that make, read and end a link are in the Catalogue, at both
//! Doors.
//!
//! The text is written here rather than by a library: an all-day event needs
//! a handful of lines, and what the format asks of them (escaping, folding
//! long lines) is a few dozen more. How Google Calendar and Apple Calendar
//! read a feed is in `docs/research/meal-planning-in-other-apps.md`.

use std::sync::Arc;

use axum::Router;
use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::core::Core;

/// One Planned Meal as the calendar shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarEvent {
    /// Unchanged for the meal's life, moves included, so a calendar app moves
    /// the event rather than showing it twice.
    pub uid: String,
    /// The recipe's title or the written words, as they are.
    pub title: String,
    pub when: When,
    /// The recipe at the instance's public address, where there is one.
    pub link: Option<String>,
}

/// Where a Planned Meal stands, each as a calendar date such as 2026-11-05.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    Day(String),
    /// A week with no day, named by its Monday.
    Week(String),
}

/// How often a calendar app is asked to fetch the file again. Only a hint
/// (RFC 7986 §5.7): neither Google nor Apple says it obeys one.
const REFRESH: &str = "PT1H";

/// The whole file. `stamp` is now, as the Security Record writes a time.
pub fn write(name: &str, events: &[CalendarEvent], stamp: &str) -> String {
    let stamp = basic_time(stamp);
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//Kamosu//Meal Plan//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
        format!("NAME:{}", text(name)),
        format!("X-WR-CALNAME:{}", text(name)),
        format!("REFRESH-INTERVAL;VALUE=DURATION:{REFRESH}"),
        format!("X-PUBLISHED-TTL:{REFRESH}"),
    ];
    for event in events {
        let (first, after) = match &event.when {
            When::Day(day) => (day.as_str(), days_after(day, 1)),
            When::Week(monday) => (monday.as_str(), days_after(monday, 7)),
        };
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}", text(&event.uid)));
        lines.push(format!("DTSTAMP:{stamp}"));
        lines.push(format!("DTSTART;VALUE=DATE:{}", basic_date(first)));
        // The day after the last, which the format does not count (RFC 5545
        // §3.6.1).
        lines.push(format!("DTEND;VALUE=DATE:{}", basic_date(&after)));
        lines.push(format!("SUMMARY:{}", text(&event.title)));
        if let Some(link) = &event.link {
            lines.push(format!("URL:{}", link));
            // Google Calendar shows no URL property, so the description
            // carries the link too.
            lines.push(format!("DESCRIPTION:{}", text(link)));
        }
        // A dinner does not make its cook busy (RFC 5545 §3.8.2.7).
        lines.push("TRANSP:TRANSPARENT".to_string());
        lines.push("END:VEVENT".to_string());
    }
    lines.push("END:VCALENDAR".to_string());
    lines.iter().map(|line| fold(line)).collect::<String>()
}

/// A value of type TEXT, escaped as RFC 5545 §3.3.11 asks: a backslash,
/// semicolon and comma each behind a backslash, and a line break as `\n`.
pub fn text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ';' => escaped.push_str("\\;"),
            ',' => escaped.push_str("\\,"),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            other => escaped.push(other),
        }
    }
    escaped
}

/// One content line, ended by CRLF and folded so no line is longer than 75
/// octets (RFC 5545 §3.1): each further piece starts with a space, and a
/// character is never split.
pub fn fold(line: &str) -> String {
    let mut folded = String::with_capacity(line.len() + 8);
    let mut width = 0;
    for character in line.chars() {
        let size = character.len_utf8();
        // The first line has 75 octets; a later one 74 after its space.
        if width + size > 75 {
            folded.push_str("\r\n ");
            width = 1;
        }
        folded.push(character);
        width += size;
    }
    folded.push_str("\r\n");
    folded
}

/// 2026-11-05 as 20261105.
fn basic_date(day: &str) -> String {
    day.replace('-', "")
}

/// 2026-11-01T09:30:00.000Z as 20261101T093000Z.
fn basic_time(at: &str) -> String {
    let at = at.split('.').next().unwrap_or(at).trim_end_matches('Z');
    format!("{}Z", at.replace(['-', ':'], ""))
}

/// The calendar date `days` after `day`, or `day` itself where it is no date.
fn days_after(day: &str, days: i64) -> String {
    day.parse::<jiff::civil::Date>()
        .ok()
        .and_then(|date| date.checked_add(jiff::Span::new().days(days)).ok())
        .map(|date| date.to_string())
        .unwrap_or_else(|| day.to_string())
}

/// The route a Calendar Link names. Merged beside the Doors at startup, as the
/// Share Link page is; it enters no Catalogue.
pub fn router(core: Arc<Core>) -> Router {
    Router::new().route(
        "/calendar/{file}",
        get(move |Path(file): Path<String>| {
            let core = core.clone();
            async move { calendar(&core, &file) }
        }),
    )
}

/// The file for a live Secret, and nothing for any other: an ended link and
/// one never made are the same 404, since a calendar app shows neither.
fn calendar(core: &Core, file: &str) -> Response {
    let Some(secret) = file.strip_suffix(".ics") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match core.calendar_file(secret) {
        Ok(Some(body)) => (
            [
                (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            body,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::warn!(%error.message, "cannot write a Calendar Link's file");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_with_a_comma_a_semicolon_and_a_break_are_escaped() {
        assert_eq!(
            text("Pasta, salad; then\\cake\nlater"),
            "Pasta\\, salad\\; then\\\\cake\\nlater"
        );
    }

    #[test]
    fn a_long_line_folds_at_75_octets_without_splitting_a_character() {
        let line = format!("SUMMARY:{}", "é".repeat(60));
        let folded = fold(&line);
        for piece in folded.split("\r\n").filter(|piece| !piece.is_empty()) {
            assert!(piece.len() <= 75, "{piece:?} is {} octets", piece.len());
        }
        let unfolded = folded.trim_end_matches("\r\n").replace("\r\n ", "");
        assert_eq!(unfolded, line);
    }

    #[test]
    fn a_day_lasts_one_day_and_a_week_seven() {
        let file = write(
            "Kamosu · Camille",
            &[
                CalendarEvent {
                    uid: "pm_1@kamosu".into(),
                    title: "Chili".into(),
                    when: When::Day("2026-12-31".into()),
                    link: None,
                },
                CalendarEvent {
                    uid: "pm_2@kamosu".into(),
                    title: "Leftovers".into(),
                    when: When::Week("2026-11-02".into()),
                    link: None,
                },
            ],
            "2026-11-01T09:30:00.000Z",
        );
        assert!(file.contains("DTSTART;VALUE=DATE:20261231\r\nDTEND;VALUE=DATE:20270101\r\n"));
        assert!(file.contains("DTSTART;VALUE=DATE:20261102\r\nDTEND;VALUE=DATE:20261109\r\n"));
        assert!(file.contains("DTSTAMP:20261101T093000Z\r\n"));
        assert!(file.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(file.ends_with("END:VCALENDAR\r\n"));
    }
}
