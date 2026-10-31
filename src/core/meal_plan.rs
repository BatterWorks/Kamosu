//! The Meal Plan: each Person's own days ahead, and the recipes and written
//! words planned on them, on a day or on a week with no day (#486, #487,
//! ADR 0058).

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

    /// The Planned Meals from one day to another, both included, and those on
    /// any day of a week that overlaps them. They come a week at a time, by
    /// its Monday; in each, the week's meals with no day first, then each
    /// day's, and in each list in its order.
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
                    "SELECT id FROM planned_meals WHERE person_id = ?1 \
                        AND (day BETWEEN ?2 AND ?3 OR (day IS NULL AND week BETWEEN ?4 AND ?3)) \
                      ORDER BY week, day IS NOT NULL, day, position, planned_at, id",
                )
                .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
            let ids: Vec<String> = statement
                .query_map(
                    params![person_id, from, to, monday_of(conn, &from)?],
                    |row| row.get(0),
                )
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

    /// Plan a Branch on a day or a week, after whatever that list already
    /// holds, at a Yield or as written. `planned_meal_id` is how a meal planned
    /// with no network arrives (#77): the phone names it, so sending it twice
    /// plans it once.
    pub fn plan_recipe(
        &self,
        person_id: &str,
        branch_id: &str,
        place: Place<'_>,
        planned_yield: Option<&Value>,
        planned_meal_id: Option<&str>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        let planned_yield = stored_yield(planned_yield)?;
        self.plan_one(person_id, place, planned_meal_id, written_at, |conn| {
            let head = branch_head(conn, branch_id)?;
            ensure_sees_or_absent(conn, &head.cookbook_id, person_id, no_such_branch)?;
            Ok(Planned::Recipe {
                branch_id,
                known_as: branch_title(conn, &head.head_version_id)?,
                planned_yield,
            })
        })
    }

    /// Put written words on a day or a week (#487): "leftovers", "eating
    /// out". They are kept exactly as written, the ends trimmed, and never
    /// read, as a Loose Item is.
    pub fn plan_words(
        &self,
        person_id: &str,
        words: &str,
        place: Place<'_>,
        planned_meal_id: Option<&str>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        let words = required_text(words, "words")?;
        self.plan_one(person_id, place, planned_meal_id, written_at, |_| {
            Ok(Planned::Words(words))
        })
    }

    /// What `plan_recipe` and `plan_words` share: the id, sending twice, the
    /// plan's clock, and the place at the end of its list.
    fn plan_one<'a>(
        &self,
        person_id: &str,
        place: Place<'_>,
        planned_meal_id: Option<&str>,
        written_at: Option<&str>,
        what: impl FnOnce(&Connection) -> Result<Planned<'a>, OpError>,
    ) -> Result<Value, OpError> {
        if let Some(id) = planned_meal_id
            && !is_minted_meal_id(id)
        {
            return Err(OpError::bad_request(
                "planned_meal_id must be pm_ followed by sixteen lower-case hex digits",
            ));
        }
        self.db().with_conn(|conn| {
            let planned = what(conn)?;
            let (week, day) = place.resolve(conn)?;
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
            let (branch_id, words, planned_yield, known_as) = match planned {
                Planned::Recipe {
                    branch_id,
                    known_as,
                    planned_yield,
                } => (Some(branch_id), None, planned_yield, Some(known_as)),
                Planned::Words(words) => (None, Some(words), None, None),
            };
            conn.execute(
                "INSERT INTO planned_meals \
                     (id, person_id, branch_id, words, week, day, position, planned_yield, \
                      known_as, planned_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, \
                     (SELECT COALESCE(MAX(position) + 1, 0) FROM planned_meals \
                       WHERE person_id = ?2 AND week = ?5 AND day IS ?6), \
                     ?7, ?8, ?9)",
                params![
                    id,
                    person_id,
                    branch_id,
                    words,
                    week,
                    day,
                    planned_yield,
                    known_as,
                    moment
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot plan a meal: {e}")))?;
            answer(conn, person_id, &id)
        })
    }

    /// Change the written words of a Planned Meal (#487). A recipe's are its
    /// title, changed on the recipe.
    pub fn set_planned_words(
        &self,
        person_id: &str,
        planned_meal_id: &str,
        words: &str,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        let words = required_text(words, "words")?;
        self.db().with_conn(|conn| {
            let Some(meal) = planned_meal(conn, person_id, planned_meal_id)? else {
                return Err(no_such_meal());
            };
            if meal["words"].is_null() {
                return Err(OpError::bad_request(
                    "this Planned Meal is a recipe, not written words",
                ));
            }
            if plan_moment(conn, person_id, written_at)?.is_none() {
                return answer(conn, person_id, planned_meal_id);
            }
            conn.execute(
                "UPDATE planned_meals SET words = ?3 WHERE id = ?1 AND person_id = ?2",
                params![planned_meal_id, person_id, words],
            )
            .map_err(|e| OpError::internal(format!("cannot change the words: {e}")))?;
            answer(conn, person_id, planned_meal_id)
        })
    }

    /// Move a Planned Meal to another day or week, or to another place in
    /// its list (#487): `position` counts from 0 among the others in the list
    /// it goes to, and none puts it last. The list it lands in is numbered
    /// again from 0.
    pub fn move_planned_meal(
        &self,
        person_id: &str,
        planned_meal_id: &str,
        place: Place<'_>,
        position: Option<i64>,
        written_at: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let Some(meal) = planned_meal(conn, person_id, planned_meal_id)? else {
                return Err(no_such_meal());
            };
            let (week, day) = place.resolve(conn)?;
            if plan_moment(conn, person_id, written_at)?.is_none() {
                return answer(conn, person_id, planned_meal_id);
            }
            let mut list = list_of(conn, person_id, &week, day.as_deref())?;
            list.retain(|id| id != planned_meal_id);
            let at = position.map_or(list.len(), |position| {
                usize::try_from(position.max(0)).map_or(list.len(), |p| p.min(list.len()))
            });
            list.insert(at, planned_meal_id.to_string());
            conn.execute(
                "UPDATE planned_meals SET week = ?3, day = ?4 WHERE id = ?1 AND person_id = ?2",
                params![planned_meal_id, person_id, week, day],
            )
            .map_err(|e| OpError::internal(format!("cannot move a Planned Meal: {e}")))?;
            number(conn, person_id, &list)?;
            // The list it left is numbered again too, so a list's positions
            // always count from 0 with no gap.
            let left = (meal["week"].as_str(), meal["day"].as_str());
            if left != (Some(week.as_str()), day.as_deref()) {
                let behind = list_of(conn, person_id, left.0.unwrap_or_default(), left.1)?;
                number(conn, person_id, &behind)?;
            }
            answer(conn, person_id, planned_meal_id)
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
            if !meal["words"].is_null() {
                return Err(OpError::bad_request(
                    "written words have no Yield to change",
                ));
            }
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

/// The ids of one list of a Person's plan, a day's or a week's with no day,
/// in its order.
fn list_of(
    conn: &Connection,
    person_id: &str,
    week: &str,
    day: Option<&str>,
) -> Result<Vec<String>, OpError> {
    let mut statement = conn
        .prepare(
            "SELECT id FROM planned_meals WHERE person_id = ?1 AND week = ?2 AND day IS ?3 \
              ORDER BY position, planned_at, id",
        )
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
    statement
        .query_map(params![person_id, week, day], |row| row.get(0))
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))
}

/// Number a list from 0 in the order given.
fn number(conn: &Connection, person_id: &str, list: &[String]) -> Result<(), OpError> {
    for (position, id) in list.iter().enumerate() {
        conn.execute(
            "UPDATE planned_meals SET position = ?3 WHERE id = ?1 AND person_id = ?2",
            params![id, person_id, position as i64],
        )
        .map_err(|e| OpError::internal(format!("cannot number a list of the plan: {e}")))?;
    }
    Ok(())
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
/// was known by where it cannot, or its written words. `None` where this
/// Person has no such meal, which is also the answer for another Person's.
fn planned_meal(conn: &Connection, person_id: &str, id: &str) -> Result<Option<Value>, OpError> {
    type Row = (
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        i64,
        Option<String>,
        Option<String>,
    );
    let stored: Option<Row> = conn
        .query_row(
            "SELECT branch_id, words, week, day, position, planned_yield, known_as \
               FROM planned_meals WHERE id = ?1 AND person_id = ?2",
            params![id, person_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the plan: {e}")))?;
    let Some((branch_id, words, week, day, position, planned_yield, known_as)) = stored else {
        return Ok(None);
    };
    let Some(branch_id) = branch_id else {
        // Written words: nothing to read, open or scale.
        return Ok(Some(json!({
            "planned_meal_id": id,
            "week": week,
            "day": day,
            "position": position,
            "branch_id": Value::Null,
            "words": words,
            "title": Value::Null,
            "gone": false,
            "planned_yield": Value::Null,
            "written_yield": Value::Null,
            "main_photo": Value::Null,
        })));
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
        "week": week,
        "day": day,
        "position": position,
        "branch_id": branch_id,
        "words": Value::Null,
        "title": content
            .as_ref()
            .and_then(|content| content["title"].as_str())
            .or(known_as.as_deref()),
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

/// **Where a Planned Meal stands** (#487): on a day, or on any day of a week,
/// the week named by its Monday.
#[derive(Clone, Copy)]
pub enum Place<'a> {
    Day(&'a str),
    Week(&'a str),
}

impl<'a> Place<'a> {
    /// A day or a week, exactly one of the two.
    pub fn from_input(day: Option<&'a str>, week: Option<&'a str>) -> Result<Self, OpError> {
        match (day, week) {
            (Some(day), None) => Ok(Place::Day(day)),
            (None, Some(week)) => Ok(Place::Week(week)),
            _ => Err(OpError::bad_request(
                "send a day or a week: one of them, not both",
            )),
        }
    }

    /// The week (its Monday) and the day, or no day, after checking both are
    /// real dates and a week is named by its Monday.
    fn resolve(self, conn: &Connection) -> Result<(String, Option<String>), OpError> {
        match self {
            Place::Day(day) => {
                let day = calendar_day(conn, day, "day")?;
                Ok((monday_of(conn, &day)?, Some(day)))
            }
            Place::Week(week) => {
                let week = calendar_day(conn, week, "week")?;
                if monday_of(conn, &week)? != week {
                    return Err(OpError::bad_request(
                        "week must be the Monday it starts on, such as 2026-11-02",
                    ));
                }
                Ok((week, None))
            }
        }
    }
}

/// What is being planned: a recipe, or written words.
enum Planned<'a> {
    Recipe {
        branch_id: &'a str,
        known_as: String,
        planned_yield: Option<String>,
    },
    Words(&'a str),
}

/// The Monday of the week a calendar day is in.
fn monday_of(conn: &Connection, day: &str) -> Result<String, OpError> {
    conn.query_row(
        "SELECT date(?1, '-' || ((CAST(strftime('%w', ?1) AS INTEGER) + 6) % 7) || ' days')",
        params![day],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot find a week's Monday: {e}")))
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
