//! Runners: the Runner Keys an Operator mints, a Runner asking for work, and
//! whether this instance has a Runner at all (ADR 0048).

use super::*;

/// The answer to minting a Runner Key: the raw Secret, shown once and never
/// stored, alongside what the Key is known by afterwards.
pub struct RunnerKey {
    pub id: String,
    pub secret: String,
    pub name: String,
}

impl Core {
    /// How often lapsed holds are looked for when no Runner calls.
    const HOLD_WATCH_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

    /// Mint a Runner Key. It belongs to the instance and to no Person: that
    /// an Operator asked is the Catalogue's declaration, checked in
    /// `execute`. `by` is who did, for the Security Record (#371) and for a
    /// recovery of their account to end it (#378), and nobody for plumbing
    /// that mints one directly.
    pub fn mint_runner_key(
        &self,
        by: Option<&Caller>,
        key_name: &str,
    ) -> Result<RunnerKey, OpError> {
        let key_name = required_text(key_name, "name")?.to_string();
        let secret = generate_secret();
        let hash = hash_secret(&secret);
        let id = format!("rk_{}", hex::encode(random_bytes(8)));
        let minted_by = by.map(|caller| caller.person_id.as_str());
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO runner_keys (id, secret_hash, name, minted_by) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, hash, key_name, minted_by],
            )
            .map_err(|e| OpError::internal(format!("cannot mint Runner Key: {e}")))
        })?;
        self.note_runner_key("runner_key_minted", by, &key_name);
        Ok(RunnerKey {
            id,
            secret,
            name: key_name,
        })
    }

    /// Every Runner Key by its name and last use, newest first. The Secret
    /// appears nowhere in it.
    pub fn runner_keys(&self) -> Result<Vec<Value>, OpError> {
        self.db().with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT id, name, created_at, last_used_at, revoked FROM runner_keys
                      ORDER BY created_at DESC, id",
                )
                .map_err(|e| OpError::internal(format!("cannot list Runner Keys: {e}")))?;
            statement
                .query_map([], |row| {
                    let revoked = row.get::<_, i64>(4)? != 0;
                    Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "created_at": row.get::<_, String>(2)?,
                        "last_used_at": self.heard_in_a_demo(!revoked, row.get(3)?),
                        "revoked": revoked,
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot read Runner Keys: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Runner Keys: {e}")))
        })
    }

    /// Revoke a Runner Key. The Job it holds goes back to waiting, and the
    /// Job Keys it was handed stop working with it (#266).
    pub fn revoke_runner_key(&self, by: Option<&Caller>, key_id: &str) -> Result<(), OpError> {
        let failed = |e| OpError::internal(format!("cannot revoke Runner Key: {e}"));
        let name = self.db().with_conn(|conn| {
            // The Key and what it held go together or not at all.
            let tx = conn.unchecked_transaction().map_err(failed)?;
            let name: Option<String> = tx
                .query_row(
                    "UPDATE runner_keys SET revoked = 1 WHERE id = ?1 AND revoked = 0 \
                     RETURNING name",
                    params![key_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(failed)?;
            let Some(name) = name else {
                return Err(OpError::not_found("no live Runner Key has that id"));
            };
            // Nothing can renew its hold or report on its Job from here on.
            jobs::release_for_revoked_runner(&tx, key_id)?;
            tx.commit().map_err(failed)?;
            Ok(name)
        })?;
        self.note_runner_key("runner_key_revoked", by, &name);
        Ok(())
    }

    /// The Runner Keys a recovery link ends with the old password (#378):
    /// every live one `person_id` minted, and when they are an Operator,
    /// every live one minted before Kamosu wrote down who mints them. Each
    /// is revoked as by hand, inside the recovery's own transaction, and
    /// answered by name for the Security Record.
    pub(super) fn end_runner_keys_of_recovered(
        conn: &Connection,
        person_id: &str,
    ) -> Result<Vec<String>, OpError> {
        let failed = |e| OpError::internal(format!("cannot end the Runner Keys: {e}"));
        let ended: Vec<(String, String)> = conn
            .prepare(
                "UPDATE runner_keys SET revoked = 1
                  WHERE revoked = 0
                    AND (minted_by = ?1
                         OR (minted_by IS NULL
                             AND (SELECT is_operator FROM people WHERE id = ?1) = 1))
                 RETURNING id, name",
            )
            .and_then(|mut statement| {
                statement
                    .query_map(params![person_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .map_err(failed)?;
        for (key_id, _) in &ended {
            jobs::release_for_revoked_runner(conn, key_id)?;
        }
        Ok(ended.into_iter().map(|(_, name)| name).collect())
    }

    /// Note the Runner Keys a recovery link ended in every Operator's record,
    /// each as revoked by the recovered Person through the Session the link
    /// opened, which no Caller holds yet.
    pub(super) fn note_runner_keys_recovery_ended(
        &self,
        person_id: &str,
        session_name: &str,
        names: &[String],
    ) {
        for name in names {
            let mut event = Event::instance("runner_key_revoked", None)
                .naming(Some(Named {
                    kind: "session",
                    name: session_name.to_string(),
                }))
                .saying(json!({ "runner_key": name }));
            event.by_id = Some(person_id.to_string());
            self.note(event);
        }
    }

    /// Note a Runner Key minted or revoked in every Operator's record, with
    /// the Credential that did it and the key's own name in `detail`.
    fn note_runner_key(&self, event: &'static str, by: Option<&Caller>, name: &str) {
        self.note(
            Event::instance(event, by)
                .naming(by.and_then(|caller| self.named_credential(caller)))
                .saying(json!({ "runner_key": name })),
        );
    }

    /// The Runner Key a Secret's hash names, if it names a live one. Reads
    /// only: `present` is what records the use.
    pub(super) fn lookup_runner_key(&self, hash: &str) -> Result<Option<String>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT id FROM runner_keys WHERE secret_hash = ?1 AND revoked = 0",
                params![hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// The Person a Job Key's hash names and the Job it is the key of, until
    /// that Job ends: `(person_id, is_operator, job_id)`. A Job that has
    /// ended, however it ended, and an asker who was disabled or deleted both
    /// leave the Job Key naming nobody (ADR 0048). A hold that lapsed does
    /// not: the Job waits again and has not ended, and no Secret dies on a
    /// clock (ADR 0031, #266).
    pub(super) fn lookup_job_key(
        &self,
        hash: &str,
    ) -> Result<Option<(String, bool, String)>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT jobs.person_id, people.is_operator, jobs.id
                   FROM job_keys
                   JOIN jobs ON jobs.id = job_keys.job_id
                   JOIN people ON people.id = jobs.person_id
                  WHERE job_keys.secret_hash = ?1 AND jobs.status IN ('queued', 'running')
                    AND people.disabled = 0 AND people.deleted = 0",
                params![hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot resolve Credential: {e}")))
        })
    }

    /// What a Job Key's write is held to before its Operation runs (ADR 0048,
    /// #264). `mark` is how the Catalogue marks the Operation and `input` is
    /// what the call sent. These are things no Operation asks, since none
    /// knows of Jobs:
    ///
    /// - **It reaches the Job's recipe and no other.** An Operation marked to
    ///   write the recipe names it in `branch_id`. Any other is answered as a
    ///   recipe that is not there, the recipe of another waiting Job included.
    /// - **It never makes a Copy.** Where the asker's own change to the Job's
    ///   recipe would start one, the write is refused and the Job fails with
    ///   the reason. That spends the Job Key, so the Job writes nothing more,
    ///   which is why this is asked of storing a Photograph too. What the Job
    ///   saved before stays. The question is the one `save_onto_branch` asks
    ///   before it copies.
    /// - **It makes no Translation.** A save that names the Version it
    ///   translates is refused.
    /// - **It corrects no Reading another recipe holds too**, which is
    ///   `JobKey::CorrectsItsReadings`.
    /// - **It creates one recipe, and only for a Job that has none** (#269),
    ///   which is `JobKey::CreatesItsRecipe`. A Job reading an Original
    ///   waits with no recipe, and until its agent creates one nothing else
    ///   it writes names a recipe. `execute` then makes the new recipe the
    ///   Job's own, under a lock this check is taken inside.
    ///
    /// This can end the Job, so it writes as well as checks.
    pub(super) fn hold_job_key_write(
        &self,
        caller: &Caller,
        job_id: &str,
        mark: JobKey,
        input: &Value,
    ) -> Result<(), OpError> {
        let names_its_recipe = match mark {
            JobKey::Never | JobKey::Reads => return Ok(()),
            JobKey::WritesItsRecipe | JobKey::CorrectsItsReadings => true,
            JobKey::StoresPhotographs | JobKey::CreatesItsRecipe => false,
        };
        self.db().with_conn(|conn| {
            // The Job ended between the Job Key being presented and here.
            let Some(its_recipe) = jobs::standing_recipe_of(conn, job_id)? else {
                return Err(OpError::credential_names_nobody());
            };
            if mark == JobKey::CreatesItsRecipe {
                // Where the recipe lands is the Job's to say and never the
                // agent's (#302).
                if input
                    .get("cookbook_id")
                    .is_some_and(|named| !named.is_null())
                {
                    return Err(OpError::bad_request(
                        "an agent's Job creates its recipe where the Job was asked for, so \
                         its create names no cookbook_id. Leave the field out",
                    ));
                }
                // The recipe is made the Job's own just after it is created,
                // in a second write. Should that one not have landed, the
                // recipe is found by the first Version the Job wrote.
                let its_recipe = match its_recipe {
                    None => jobs::recipe_created_by(conn, job_id)?,
                    named => named,
                };
                return match its_recipe {
                    None => Ok(()),
                    Some(its_recipe) => Err(OpError::bad_request(format!(
                        "this Job's recipe exists already, as {its_recipe}. A Job creates one \
                         recipe: write that one with edit_recipe or save_recipe_version"
                    ))),
                };
            }
            let Some(its_recipe) = its_recipe else {
                if names_its_recipe {
                    return Err(OpError::bad_request(
                        "this Job has no recipe yet. Create it with create_recipe, then \
                         write it",
                    ));
                }
                // A Photograph stored before the recipe exists has no recipe
                // a Copy could start from.
                return Ok(());
            };
            if names_its_recipe && input["branch_id"].as_str() != Some(its_recipe.as_str()) {
                return Err(no_such_branch());
            }
            let writing_for = changes_for(conn, caller, &its_recipe)?;
            if !cookbook_writes_branch(conn, &writing_for, &its_recipe)? {
                jobs::end_where_a_write_was_refused(conn, job_id, jobs::WOULD_START_A_COPY)?;
                return Err(OpError::unauthorized(jobs::WOULD_START_A_COPY));
            }
            if mark == JobKey::WritesItsRecipe
                && input
                    .get("translates_version_id")
                    .is_some_and(|named| !named.is_null())
            {
                return Err(OpError::bad_request(
                    "an agent's Job makes no Translation, so its save names no \
                     translates_version_id. Leave the field out",
                ));
            }
            if mark == JobKey::CorrectsItsReadings {
                let shared: bool = conn
                    .query_row(
                        "SELECT EXISTS (
                            SELECT 1 FROM branch_versions
                              JOIN branches ON branches.head_version_id = branch_versions.version_id
                             WHERE branches.id = ?1 AND branch_versions.branch_id <> ?1)",
                        params![its_recipe],
                        |row| row.get(0),
                    )
                    .map_err(|e| {
                        OpError::internal(format!("cannot look for the Version elsewhere: {e}"))
                    })?;
                if shared {
                    return Err(OpError::bad_request(
                        "another recipe reads word for word as this one does, so the two \
                         hold this Version and its Readings between them, and correcting \
                         one here would correct the other's. Save a change to this recipe \
                         first, or leave the Reading and say so in your report",
                    ));
                }
            }
            Ok(())
        })
    }

    /// A Runner asking for a Job to do: the oldest that waits, with what to
    /// read and which recipe it is for, or none (#262). The Job is running
    /// from here until the Runner reports on it or stops renewing its hold
    /// (#266). `ask` is the mark of this ask, the same when it is repeated.
    ///
    /// The Runner is Kamosu's own program started in another mode, so one of
    /// another version is an install half upgraded. It is refused before the
    /// asking is recorded: a Runner that cannot work has not asked.
    pub fn ask_for_work(
        &self,
        runner: &RunnerCaller,
        runner_version: &str,
        ask: &str,
    ) -> Result<Value, OpError> {
        let ask = required_text(ask, "ask")?;
        if ask.chars().count() > ASK_CHARS {
            return Err(OpError::bad_request(format!(
                "ask is longer than {ASK_CHARS} characters"
            )));
        }
        let kamosu_version = env!("CARGO_PKG_VERSION");
        if runner_version != kamosu_version {
            return Err(OpError::refused_because(
                "another_version",
                format!(
                    "this Runner is version {runner_version} and this Kamosu is version \
                     {kamosu_version}. Run the Runner of the same version as the Kamosu it asks"
                ),
            ));
        }
        let (work, house_notes) = self.db().with_conn(|conn| {
            heard_from(conn, &runner.runner_key_id)?;
            let work = jobs::take_for_runner(conn, &runner.runner_key_id, ask, self.hold())?;
            Ok((work, current_house_notes(conn)?))
        })?;
        Ok(json!({
            "job": work.map(|work| json!({
                // Several pictures carry `pages` too (ADR 0055): every page
                // in order, the first being `address`. One file is handed
                // over as it always was.
                "original": work.original.map(|kind| {
                    let mut original = json!({
                        "kind": kind.as_str(),
                        "address": original_address(&work.job_id),
                    });
                    if work.original_pages > 1 {
                        original["pages"] = json!(page_addresses(&work.job_id, work.original_pages));
                    }
                    original
                }),
                "job_id": work.job_id,
                "branch_id": work.branch_id,
                "link": work.link,
                "job_key": work.job_key,
                "house_notes": house_notes,
                "held_until": work.held_until,
                "renew_within_seconds": self.renew_within_seconds(),
            })),
        }))
    }

    /// A Runner renewing its hold on the Job it is working, which moves the
    /// deadline on (#266). The answer says when to renew next.
    pub fn renew_hold(&self, runner: &RunnerCaller, job_id: &str) -> Result<Value, OpError> {
        let held_until = self.db().with_conn(|conn| {
            // A Runner working on a long Job asks for no other, so its
            // renewals say it is there as its asks do (#277).
            heard_from(conn, &runner.runner_key_id)?;
            jobs::renew_hold(conn, &runner.runner_key_id, job_id, self.hold())
        })?;
        Ok(json!({
            "held_until": held_until,
            "renew_within_seconds": self.renew_within_seconds(),
        }))
    }

    /// How long a hold lasts unless it is renewed.
    fn hold(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.hold_millis.load(Ordering::Relaxed))
    }

    /// How long a Runner is told to leave between renewals: a third of the
    /// hold, so that two renewals can go missing and the third still lands.
    fn renew_within_seconds(&self) -> f64 {
        self.hold().as_secs_f64() / 3.0
    }

    /// Hold Jobs for `hold` in place of `jobs::HOLD`. Compiled only under
    /// `test-jobs`: a test that waits for a renewal cannot wait two minutes.
    #[cfg(feature = "test-jobs")]
    pub fn hold_jobs_for(&self, hold: std::time::Duration) {
        self.hold_millis
            .store(hold.as_millis() as u64, Ordering::Relaxed);
    }

    /// Send the Jobs whose hold lapsed back to waiting. Every call of a
    /// Runner's does this first, and `spawn_hold_watch` does it when no
    /// Runner calls.
    pub fn release_lapsed_holds(&self) -> Result<(), OpError> {
        self.db().with_conn(jobs::release_lapsed_holds)
    }

    /// A Runner saying how the Job it holds ended, and passing on the agent's
    /// own report (#262). `reason` is required of a Job that did not
    /// complete, and is the sentence the cook reads.
    pub fn report_on_job(
        &self,
        runner: &RunnerCaller,
        job_id: &str,
        outcome: jobs::AgentOutcome,
        reason: Option<&str>,
        report: Option<&Value>,
    ) -> Result<(), OpError> {
        let reason = match (outcome, reason) {
            (jobs::AgentOutcome::Completed, None) => None,
            (jobs::AgentOutcome::Completed, Some(_)) => {
                return Err(OpError::bad_request(
                    "a completed Job takes no reason. What the agent left undone goes in report",
                ));
            }
            (_, reason) => Some(agent_entry(reason.unwrap_or_default(), "reason")?),
        };
        let report = agent_report(report)?;
        self.db().with_conn(|conn| {
            jobs::end_as_reported(
                conn,
                &runner.runner_key_id,
                job_id,
                outcome,
                reason.as_deref(),
                &report,
            )
        })
    }

    /// Whether a Runner Key stands, which is what sends a link Kamosu cannot
    /// read to an agent (ADR 0048).
    pub fn has_runner(&self) -> Result<bool, OpError> {
        Ok(self.runner_status()?["has_runner"] == json!(true))
    }

    /// Who may read a Job (#268). One of Kamosu's own is read by the Person
    /// who asked for it, or by anyone when no Person did. An agent's is read
    /// by the Person who asked and by the Co-authors of the Cookbook its
    /// recipe is in, as that Cookbook stands now (`reads_agent_job`). Anyone
    /// else is refused with `absent`, the answer a Job id naming nothing
    /// gets (ADR 0040).
    ///
    /// Once the recipe is deleted it is in no Cookbook, so the Person who
    /// asked is the one reader left.
    ///
    /// Who cancels is `jobs::ensure_asker`, and this opens none of it.
    pub fn ensure_reads_job(
        &self,
        record: &JobRecord,
        caller: &Option<Caller>,
        absent: impl FnOnce() -> OpError,
    ) -> Result<(), OpError> {
        if jobs::asked_by(record, caller) {
            return Ok(());
        }
        let Some(caller) = caller else {
            return Err(absent());
        };
        let reads =
            self.db()
                .with_conn(|conn| match jobs::recipe_of_agent_job(conn, &record.id)? {
                    Some(branch_id) => {
                        reads_agent_job(conn, record.person_id.as_deref(), &branch_id, caller)
                    }
                    None => Ok(false),
                })?;
        if reads { Ok(()) } else { Err(absent()) }
    }

    /// The agent's Job that waits or is being worked on a recipe, for
    /// `get_recipe_job` (#268): its id and which of the two it is, or null.
    ///
    /// It is named to those who may read that Job (`reads_agent_job`).
    /// Anyone else who sees the recipe is answered as a recipe with no Job
    /// on it answers. Someone who does not see the recipe is told it is not
    /// there (ADR 0040), the Person who asked included: they read their Job
    /// by its id.
    pub fn recipe_job(&self, caller: &Caller, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, &caller.person_id)?;
            let named = match jobs::agent_job_standing_on(conn, branch_id)? {
                Some(standing)
                    if reads_agent_job(conn, standing.asker.as_deref(), branch_id, caller)? =>
                {
                    let yours = standing.asker.as_deref() == Some(caller.person_id.as_str());
                    json!({ "id": standing.id, "status": standing.status, "yours": yours })
                }
                _ => Value::Null,
            };
            Ok(json!({ "job": named }))
        })
    }

    /// Whether this instance has a Runner, which is to say a Runner Key that
    /// is not revoked, and when one was last heard from: asking for work, or
    /// renewing its hold on the Job it works on (#277). The time is read
    /// across every Key, revoked ones included, so replacing a Key does not
    /// erase when the Runner was last heard from. A Demo's has just asked
    /// (#406).
    pub fn runner_status(&self) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT COALESCE(SUM(revoked = 0), 0) > 0, MAX(last_asked_at) FROM runner_keys",
                [],
                |row| {
                    let has_runner = row.get::<_, bool>(0)?;
                    Ok(json!({
                        "has_runner": has_runner,
                        "last_asked_at": self.heard_in_a_demo(has_runner, row.get(1)?),
                    }))
                },
            )
            .map_err(|e| OpError::internal(format!("cannot read the Runner's state: {e}")))
        })
    }

    /// The agent's house notes, with who saved them last and when (#344).
    /// The name is the one that Person goes by now.
    pub fn house_notes(&self) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let found = conn
                .query_row(
                    "SELECT agent_house_notes, people.name, house_notes_saved_at
                       FROM instance_setup
                       LEFT JOIN people ON people.id = instance_setup.house_notes_saved_by
                      WHERE singleton = 1",
                    [],
                    |row| {
                        Ok(json!({
                            "notes": row.get::<_, Option<String>>(0)?,
                            "saved_by": row.get::<_, Option<String>>(1)?,
                            "saved_at": row.get::<_, Option<String>>(2)?,
                        }))
                    },
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the house notes: {e}")))?;
            Ok(found
                .unwrap_or_else(|| json!({ "notes": null, "saved_by": null, "saved_at": null })))
        })
    }

    /// Save the agent's house notes, in place of any there were. Text that
    /// is only blank takes them away, and the agent follows its own
    /// instructions alone again.
    pub fn set_house_notes(&self, by: &Caller, notes: &str) -> Result<Value, OpError> {
        let notes = notes.trim();
        if notes.chars().count() > HOUSE_NOTES_CHARS {
            return Err(OpError::bad_request(format!(
                "the house notes are longer than {HOUSE_NOTES_CHARS} characters"
            )));
        }
        let notes = (!notes.is_empty()).then_some(notes);
        self.db().with_conn(|conn| {
            conn.execute(
                "INSERT INTO instance_setup
                        (singleton, agent_house_notes, house_notes_saved_by, house_notes_saved_at)
                 VALUES (1, ?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
                 ON CONFLICT(singleton) DO UPDATE SET
                        agent_house_notes = excluded.agent_house_notes,
                        house_notes_saved_by = excluded.house_notes_saved_by,
                        house_notes_saved_at = excluded.house_notes_saved_at",
                params![notes, by.person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot save the house notes: {e}")))
        })?;
        // What they now say is read from the House Notes themselves, which
        // keep who saved them last; the record says that they changed.
        self.note(
            Event::instance("house_notes_changed", Some(by))
                .naming(self.named_credential(by))
                .saying(json!({ "taken_away": notes.is_none() })),
        );
        self.house_notes()
    }
}

/// The longest the agent's house notes may be: a page of house rules,
/// where the instructions they add to run to some three hundred lines.
pub const HOUSE_NOTES_CHARS: usize = 4000;

/// The house notes as a Job carries them, read when a Runner takes it.
fn current_house_notes(conn: &Connection) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT agent_house_notes FROM instance_setup WHERE singleton = 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map(Option::flatten)
    .map_err(|e| OpError::internal(format!("cannot read the house notes: {e}")))
}

/// Record that a Runner was heard from, which `runner_status` reads.
fn heard_from(conn: &Connection, runner_key_id: &str) -> Result<(), OpError> {
    conn.execute(
        "UPDATE runner_keys SET last_asked_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
          WHERE id = ?1",
        params![runner_key_id],
    )
    .map_err(|e| OpError::internal(format!("cannot record hearing from the Runner: {e}")))?;
    Ok(())
}

/// Whether `caller` may read the agent's Job on a recipe: they asked for it,
/// or they are a Co-author of the Cookbook the recipe is in now. Both
/// `get_job` and `get_recipe_job` ask it here, so the two cannot disagree.
fn reads_agent_job(
    conn: &Connection,
    asker: Option<&str>,
    branch_id: &str,
    caller: &Caller,
) -> Result<bool, OpError> {
    Ok(asker == Some(caller.person_id.as_str())
        || writes_cookbook_of_branch(conn, branch_id, &caller.person_id)?)
}

/// The most characters the mark of an ask holds. A Runner makes one up for
/// each ask, and nothing longer than an id is any use as one.
const ASK_CHARS: usize = 100;

/// Release lapsed holds for as long as the instance serves (#266), so a Job
/// whose Runner went quiet reads as waiting when no other Runner asks.
pub(super) fn spawn_hold_watch(core: Arc<Core>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Core::HOLD_WATCH_EVERY);
        loop {
            ticker.tick().await;
            let core = core.clone();
            // Blocking SQLite: never on the runtime the Doors answer on.
            let released = tokio::task::spawn_blocking(move || core.release_lapsed_holds()).await;
            if let Ok(Err(error)) = released {
                tracing::warn!("cannot release the holds that lapsed: {error}");
            }
        }
    });
}

/// The four lists of an agent's own report, in the order a Report shows them.
/// Each is a list of sentences, but for `likely_duplicates`, which is a list
/// of Branch ids, and `unused_foods`, which is a list of Food names.
const AGENT_REPORT_LISTS: [&str; 4] =
    ["left_undone", "unsure", "likely_duplicates", "unused_foods"];

/// The most entries one list of an agent's report holds, and the most
/// characters one entry holds. A Report is read on a phone, and an agent
/// that a page has tricked must not be able to fill a database through it.
const AGENT_REPORT_ENTRIES: usize = 100;
const AGENT_REPORT_ENTRY_CHARS: usize = 2000;

/// One entry of an agent's own words, trimmed: a sentence, a Branch id or a
/// Food's name. Not blank, and no longer than a Report shows.
fn agent_entry(text: &str, field: &str) -> Result<String, OpError> {
    let text = required_text(text, field)?;
    if text.chars().count() > AGENT_REPORT_ENTRY_CHARS {
        return Err(OpError::bad_request(format!(
            "{field} holds an entry longer than {AGENT_REPORT_ENTRY_CHARS} characters"
        )));
    }
    Ok(text.to_string())
}

/// An agent's own report as Kamosu keeps it: all four lists, each empty
/// where the agent had nothing to say. The Catalogue's declaration has
/// already checked its shape, so what is checked here is its size.
fn agent_report(report: Option<&Value>) -> Result<Value, OpError> {
    let mut kept = serde_json::Map::new();
    for list in AGENT_REPORT_LISTS {
        let entries = report
            .and_then(|report| report.get(list))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if entries.len() > AGENT_REPORT_ENTRIES {
            return Err(OpError::bad_request(format!(
                "report.{list} holds more than {AGENT_REPORT_ENTRIES} entries"
            )));
        }
        let entries = entries
            .iter()
            .map(|entry| {
                agent_entry(
                    entry.as_str().unwrap_or_default(),
                    &format!("report.{list}"),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        kept.insert(list.to_string(), json!(entries));
    }
    Ok(Value::Object(kept))
}
