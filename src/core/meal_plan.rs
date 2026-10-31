//! The Meal Plan: each Person's own days ahead, and the recipes planned on
//! them (#486, ADR 0058).

use super::shopping_list::stored_yield;
use super::*;

/// The longest stretch of days one read of the plan answers: a year, so a
/// screen asking for a week, a month or a year back is answered and a typo in
/// a year is refused rather than read across centuries.
const LONGEST_READ: i64 = 366;

impl Core {
    // ── The Meal Plan (#486, ADR 0058) ───────────────────────────────────────
    //
    // **The plan is a Person's own**, as the Shopping List is (ADR 0024):
    // every query below is keyed on the asker, so nobody else reads it, and no
    // Operation here accepts `as_operator`. Its writes carry `written_at` and
    // the last one to reach the plan wins (`plan_moment`).

    /// The Planned Meals from one day to another, both included, in the order
    /// of their days and then in the order each day's were put there.
    pub fn meal_plan(&self, person_id: &str, from: &str, to: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let from = calendar_day(conn, from, "from")?;
            let to = calendar_day(conn, to, "to")?;
            let days: i64 = conn
                .query_row(
                    "SELECT julianday(?2) - julianday(?1)",
                    params![from, to],
                    |row| row.get::<_, f64>(0),
                )
                .map(|days| days as i64)
                .map_err(|e| OpError::internal(format!("cannot count the days: {e}")))?;
            if days < 0 {
                return Err(OpError::bad_request(
                    "to must be the same day as from or after it",
                ));
            }
            if days >= LONGEST_READ {
                return Err(OpError::bad_request(format!(
                    "one read of the plan covers at most {LONGEST_READ} days"
                )));
            }
            let mut statement = conn
                .prepare(
                    "SELECT id FROM planned_meals WHERE person_id = ?1 AND day BETWEEN ?2 AND ?3 \
                      ORDER BY day, position, planned_at, id",
                )
                .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
            let ids: Vec<String> = statement
                .query_map(params![person_id, from, to], |row| row.get(0))
                .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
            let meals = ids
                .iter()
                .map(|id| planned_meal(conn, person_id, id))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({ "planned_meals": meals.into_iter().flatten().collect::<Vec<_>>() }))
        })
    }

    /// Plan a Branch on a day, after whatever that day already holds, at a
    /// Yield or as written. `planned_meal_id` is how a meal planned with no
    /// network arrives (#77): the phone names it, so sending it twice plans it
    /// once.
    pub fn plan_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        day: &str,
        planned_yield: Option<&Value>,
        planned_meal_id: Option<&str>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        if let Some(id) = planned_meal_id
            && !is_minted_meal_id(id)
        {
            return Err(OpError::bad_request(
                "planned_meal_id must be pm_ followed by sixteen lower-case hex digits",
            ));
        }
        let planned_yield = stored_yield(planned_yield)?;
        self.db().with_conn(|conn| {
            let head = branch_head(conn, branch_id)?;
            ensure_sees_or_absent(conn, &head.cookbook_id, person_id, no_such_branch)?;
            let day = calendar_day(conn, day, "day")?;
            let id = planned_meal_id
                .map(str::to_string)
                .unwrap_or_else(|| format!("pm_{}", hex::encode(random_bytes(8))));
            let owner: Option<String> = conn
                .query_row(
                    "SELECT person_id FROM planned_meals WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
            match owner {
                // Sent twice: it is planned already, as the first send left it.
                Some(owner) if owner == person_id => return answer(conn, person_id, &id),
                Some(_) => return Err(OpError::bad_request("that planned_meal_id is taken")),
                None => {}
            }
            let Some(moment) = plan_moment(conn, person_id, written_at)? else {
                return answer(conn, person_id, &id);
            };
            conn.execute(
                "INSERT INTO planned_meals \
                     (id, person_id, branch_id, day, position, planned_yield, known_as, planned_at) \
                 VALUES (?1, ?2, ?3, ?4, \
                     (SELECT COALESCE(MAX(position) + 1, 0) FROM planned_meals \
                       WHERE person_id = ?2 AND day = ?4), \
                     ?5, ?6, ?7)",
                params![
                    id,
                    person_id,
                    branch_id,
                    day,
                    planned_yield,
                    branch_title(conn, &head.head_version_id)?,
                    moment
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot plan a recipe: {e}")))?;
            answer(conn, person_id, &id)
        })
    }

    /// Say how much of a Planned Meal is to be cooked: a Yield, a multiplier,
    /// or `None` for the recipe as written.
    pub fn set_planned_yield(
        &self,
        person_id: &str,
        planned_meal_id: &str,
        planned_yield: Option<&Value>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        let planned_yield = stored_yield(planned_yield)?;
        self.db().with_conn(|conn| {
            let Some(meal) = planned_meal(conn, person_id, planned_meal_id)? else {
                return Err(no_such_meal());
            };
            if meal["gone"] == true {
                return Err(OpError::bad_request(
                    "this recipe can no longer be read, so it has no Yield to change",
                ));
            }
            if plan_moment(conn, person_id, written_at)?.is_none() {
                return answer(conn, person_id, planned_meal_id);
            }
            // The name it is known by moves with it, as a Shopping List
            // entry's does, so a recipe renamed and later deleted keeps the
            // name it was last seen under.
            conn.execute(
                "UPDATE planned_meals SET planned_yield = ?3, known_as = ?4 \
                  WHERE id = ?1 AND person_id = ?2",
                params![
                    planned_meal_id,
                    person_id,
                    planned_yield,
                    meal["title"].as_str()
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot set the Yield: {e}")))?;
            answer(conn, person_id, planned_meal_id)
        })
    }

    /// Take a Planned Meal off the plan. Taking one that is already gone is not
    /// an error, for the reason `remove_from_shopping_list` gives: a removal
    /// replayed from a phone that had no network must not fail because it had
    /// already worked.
    pub fn remove_planned_meal(
        &self,
        person_id: &str,
        planned_meal_id: &str,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            if plan_moment(conn, person_id, written_at)?.is_none() {
                return answer(conn, person_id, planned_meal_id);
            }
            conn.execute(
                "DELETE FROM planned_meals WHERE id = ?1 AND person_id = ?2",
                params![planned_meal_id, person_id],
            )
            .map_err(|e| {
                OpError::internal(format!("cannot take a Planned Meal off the plan: {e}"))
            })?;
            Ok(json!({ "planned_meal": Value::Null }))
        })
    }
}

/// What every write answers: the Planned Meal as it now stands, or null where
/// there is none.
fn answer(conn: &Connection, person_id: &str, id: &str) -> Result<Value, OpError> {
    Ok(json!({ "planned_meal": planned_meal(conn, person_id, id)? }))
}

fn no_such_meal() -> OpError {
    OpError::not_found("there is no such Planned Meal on your plan")
}

/// One Planned Meal of this Person's, as every answer shows it: its recipe's
/// live title and Yield where the recipe can still be read, and the name it
/// was known by where it cannot. `None` where this Person has no such meal,
/// which is also the answer for another Person's.
fn planned_meal(conn: &Connection, person_id: &str, id: &str) -> Result<Option<Value>, OpError> {
    let stored: Option<(String, String, i64, Option<String>, String)> = conn
        .query_row(
            "SELECT branch_id, day, position, planned_yield, known_as FROM planned_meals \
              WHERE id = ?1 AND person_id = ?2",
            params![id, person_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
    let Some((branch_id, day, position, planned_yield, known_as)) = stored else {
        return Ok(None);
    };
    // Always the Branch's newest Version, as the Shopping List reads it: a
    // correction made on Monday is what Thursday shows.
    let head = branch_head(conn, &branch_id).ok();
    let readable = match &head {
        Some(head) => sees_cookbook(conn, &head.cookbook_id, person_id)?,
        None => false,
    };
    let content = match (&head, readable) {
        (Some(head), true) => Some(version_content(conn, &head.head_version_id)?),
        _ => None,
    };
    Ok(Some(json!({
        "planned_meal_id": id,
        "day": day,
        "position": position,
        "branch_id": branch_id,
        "title": content
            .as_ref()
            .and_then(|content| content["title"].as_str())
            .unwrap_or(&known_as),
        "gone": !readable,
        "planned_yield": planned_yield
            .as_deref()
            .and_then(|stored| serde_json::from_str::<Value>(stored).ok())
            .unwrap_or(Value::Null),
        "written_yield": content
            .as_ref()
            .map(|content| content["yield"].clone())
            .unwrap_or(Value::Null),
        // Its picture, for the day it is drawn on; none once it cannot be read.
        "main_photo": content
            .as_ref()
            .map(|content| content["main_photo"].clone())
            .unwrap_or(Value::Null),
    })))
}

/// A calendar date as the plan holds one, YYYY-MM-DD, refused unless it is a
/// real day: SQLite reads `2026-02-30` as a day in March, so the date it
/// answers must be the date sent.
fn calendar_day(conn: &Connection, day: &str, field: &str) -> Result<String, OpError> {
    let read: Option<String> = conn
        .query_row("SELECT date(?1)", params![day], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot read a date: {e}")))?;
    match read {
        Some(read) if read == day && day.len() == 10 => Ok(read),
        _ => Err(OpError::bad_request(format!(
            "{field} must be a calendar date, such as 2026-11-05"
        ))),
    }
}

/// **The last device to write the plan wins**, as the Shopping List's does
/// (`shopping_moment`, ADR 0024): the moment this write was made, or nothing
/// where the plan has since been written later and this write, sent late by
/// a phone that had no signal, is older. It is then dropped whole.
fn plan_moment(
    conn: &Connection,
    person_id: &str,
    written_at: Option<&str>,
) -> Result<Option<String>, OpError> {
    let moment = written_moment(conn, written_at)?;
    let last: Option<String> = conn
        .query_row(
            "SELECT plan_written_at FROM people WHERE id = ?1",
            params![person_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?
        .flatten();
    if last.as_deref().is_some_and(|last| moment.as_str() < last) {
        return Ok(None);
    }
    conn.execute(
        "UPDATE people SET plan_written_at = ?2 WHERE id = ?1",
        params![person_id, moment],
    )
    .map_err(|e| OpError::internal(format!("cannot write the plan: {e}")))?;
    Ok(Some(moment))
}

/// Whether a Planned Meal id is one a phone may mint: the shape the server
/// mints its own in.
fn is_minted_meal_id(id: &str) -> bool {
    id.strip_prefix("pm_").is_some_and(is_sixteen_hex)
}

/// **A Planned Meal follows its recipe to the Branch kept** (#486), in the
/// same step that moves the Shopping List's choosing: when two Cookbooks join
/// and one Branch folds into another, and when a Co-author leaves with a Copy.
/// [`Whose`] says which Persons' plans the move reaches.
pub(super) fn repoint_planned_meals(
    conn: &Connection,
    from: &str,
    to: &str,
    whose: Whose<'_>,
) -> Result<(), OpError> {
    let moved = match whose {
        Whose::Everyone => conn.execute(
            "UPDATE planned_meals SET branch_id = ?2 WHERE branch_id = ?1",
            params![from, to],
        ),
        Whose::Only(person_id) => conn.execute(
            "UPDATE planned_meals SET branch_id = ?2 WHERE branch_id = ?1 AND person_id = ?3",
            params![from, to, person_id],
        ),
        Whose::AllBut(person_id) => conn.execute(
            "UPDATE planned_meals SET branch_id = ?2 WHERE branch_id = ?1 AND person_id <> ?3",
            params![from, to, person_id],
        ),
    };
    moved.map_err(|e| OpError::internal(format!("cannot move a Planned Meal: {e}")))?;
    Ok(())
}

/// Whose Planned Meals a move reaches.
#[derive(Clone, Copy)]
pub(super) enum Whose<'a> {
    Everyone,
    Only(&'a str),
    AllBut(&'a str),
}
