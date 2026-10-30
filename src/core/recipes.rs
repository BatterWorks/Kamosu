//! Recipes themselves: creating, saving, renaming, deleting and reading one;
//! its thread, branch points and divergence; unfolding its components; and
//! `parse_recipe_content`, the one reading of what a recipe holds.

use super::*;

impl Core {
    /// Create a Recipe: a Lineage, a Branch of it in the writer's own
    /// Cookbook, and a first Version fingerprinted from its content (ADR 0004,
    /// ADR 0041). A recipe needs only a title — every other field of `input`
    /// (Yield, Prep/Cook Time, Note, Source, Ingredients, Steps) is optional
    /// (#43). It goes into your own Cookbook, or into the one `into` names,
    /// which only an Operator looking after it may name (#302).
    pub fn create_recipe(
        &self,
        caller: &Caller,
        input: &Value,
        language: Option<&str>,
        into: Option<&str>,
    ) -> Result<Value, OpError> {
        let mut content = parse_recipe_content(input)?;
        // A code on a video's link names who shared it, and a new recipe does
        // not pass that on (#464). A save keeps what it is given: a Version's
        // Source is part of its id, and an existing recipe stays as written.
        if let Some(link) = content["source"]["link"].as_str() {
            content["source"]["link"] = json!(crate::video_link::without_tracking(link));
        }
        let (version_id, content_text) = stored_version(&content);
        let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
        let branch_id = format!("b_{}", hex::encode(random_bytes(8)));

        let foods_created = self.db().with_conn(|conn| {
            let cookbook_id = lands_in(conn, caller, into)?;
            let hand_id = cookbook_hand(conn, &cookbook_id)?;
            let language = language_for_new_branch(conn, &caller.person_id, language, &content)?;

            insert_new_lineage_and_branch(
                conn,
                &lineage_id,
                &branch_id,
                &cookbook_id,
                &hand_id,
                &language,
                &version_id,
                &content_text,
                caller,
                &[],
            )
        })?;

        let mut recipe = self.get_recipe(caller, &branch_id, None)?;
        recipe["foods_created"] = foods_created_json(&foods_created);
        Ok(recipe)
    }

    /// Save a new state of a Recipe onto a Branch: an ordinary edit becomes an
    /// append-only Version. A save by the same Hand within the collapse
    /// window collapses into the Version already being shaped, rather than
    /// starting a new one (ADR 0004). Saving content identical to what is
    /// already there mints nothing.
    ///
    /// Changing a recipe your Cookbook did not write is a **Copy**
    /// (GLOSSARY.md, "Copy"; ADR 0041): it happens here, at the moment of the
    /// change, never at the moment of merely reading `branch_id`. Somebody
    /// who is not one of the Branch's Co-authors — a Kitchen-mate, or anyone
    /// saving onto a Branch that arrived in their own Cookbook from elsewhere
    /// — gets a brand new Branch of the same Lineage in their own Cookbook,
    /// carrying the whole chain behind it, starting at the Version being
    /// changed. The Branch being edited is never touched by a Copy.
    ///
    /// **A Copy only starts from a Branch you may see** (#100). Because a Copy
    /// carries the whole chain, a save onto a Branch you could not read would
    /// hand you its every Version. So a caller who may not see `branch_id` is
    /// refused exactly as an id naming nothing is (ADR 0040). Nothing
    /// legitimate is lost: an arrived Bundle and a kept Share Link both put the
    /// Branch in the receiver's own Cookbook first.
    ///
    /// Two things about Language happen here (ADR 0006). The save reads the
    /// new text and, where it disagrees with the Language the Branch carries,
    /// answers `language_offer` — a suggestion for the cook, never a change.
    /// The one Language it writes is a guess's: a Branch that took the
    /// writer's own because its text was too short to read takes the first
    /// Language its text says (#321). And `translates_version_id`
    /// carries forward from the Version being replaced unless this save names
    /// a new one, so editing a Translation's wording never quietly claims it
    /// has caught up with its source. Naming one works even when no word
    /// changes (#183): the newest Version's pointer moves, and no Version is
    /// written.
    #[allow(clippy::too_many_arguments)]
    pub fn save_recipe_version(
        &self,
        caller: &Caller,
        branch_id: &str,
        input: &Value,
        name: Option<&str>,
        change_note: Option<&str>,
        translates_version_id: Option<&str>,
    ) -> Result<Value, OpError> {
        let content = parse_recipe_content(input)?;
        self.save_onto_branch(
            caller,
            branch_id,
            NewContent::Whole(&content),
            name,
            change_note,
            translates_version_id,
        )
    }

    /// Change some fields of a Recipe and save the result onto its Branch
    /// (#164): exactly `save_recipe_version`, except that `changes` names only
    /// the fields that change and every field it leaves out is kept as the
    /// head has it. `null` clears a field and `[]` clears a list; a list is
    /// replaced whole, never edited line by line. The Version is still the
    /// fingerprint of the complete content the merge produces (ADR 0004), so
    /// an edit and a whole save of the same recipe name the same Version.
    #[allow(clippy::too_many_arguments)]
    pub fn edit_recipe(
        &self,
        caller: &Caller,
        branch_id: &str,
        changes: &Value,
        name: Option<&str>,
        change_note: Option<&str>,
        translates_version_id: Option<&str>,
    ) -> Result<Value, OpError> {
        self.save_onto_branch(
            caller,
            branch_id,
            NewContent::Fields(changes),
            name,
            change_note,
            translates_version_id,
        )
    }

    /// What `save_recipe_version` and `edit_recipe` both are, once each has
    /// said the new content its own way.
    #[allow(clippy::too_many_arguments)]
    fn save_onto_branch(
        &self,
        caller: &Caller,
        branch_id: &str,
        content: NewContent<'_>,
        name: Option<&str>,
        change_note: Option<&str>,
        translates_version_id: Option<&str>,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            // Before anything else reads the Branch, the identical-content
            // shortcut below included: it answers with the head's Version id.
            ensure_reaches_branch(conn, branch_id, caller)?;
            let (lineage_id, language, language_guessed): (String, String, bool) = conn
                .query_row(
                    "SELECT lineage_id, language, language_guessed FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
            // The Cookbook a save is made for: the caller's own, which is
            // where a Copy goes when the Branch is not theirs to write, or the
            // one an Operator acting as one looks after (#302).
            let writing_for = changes_for(conn, caller, branch_id)?;

            #[allow(clippy::type_complexity)]
            let (head_sequence, head_version_id, head_hand_id, head_parent_id, within_window, head_content_text, head_translates, head_name, head_change_note, head_job_id): (
                i64,
                String,
                String,
                Option<String>,
                bool,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ) = conn
                .query_row(
                    "SELECT branch_versions.sequence, branch_versions.version_id, \
                            branch_versions.hand_id, branch_versions.parent_version_id, \
                            (julianday('now') - julianday(branch_versions.created_at)) * 86400.0 <= ?2, \
                            versions.content, branch_versions.translates_version_id, \
                            branch_versions.name, branch_versions.change_note, \
                            branch_versions.job_id \
                       FROM branch_versions JOIN versions ON versions.id = branch_versions.version_id \
                      WHERE branch_versions.branch_id = ?1 ORDER BY branch_versions.sequence DESC LIMIT 1",
                    params![branch_id, COLLAPSE_WINDOW_SECONDS as f64],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                            row.get(9)?,
                        ))
                    },
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch head: {e}")))?;
            let head_content: Value = serde_json::from_str(&head_content_text)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;

            // An edit is merged onto the head read just above, under the same
            // lock, so no save can land between the read and the write.
            let content = match content {
                NewContent::Whole(content) => content.clone(),
                NewContent::Fields(changes) => {
                    parse_recipe_content(&merged_onto_head(&head_content, changes))?
                }
            };
            let (version_id, content_text) = stored_version(&content);

            // What this save renders of its source: the Version named here, or
            // — for the ordinary edit that names none — whatever the Version
            // being replaced already pointed at. An ordinary recipe points at
            // nothing and stays pointing at nothing.
            let translates_version_id = match translates_version_id {
                Some(named) => {
                    let named = required_text(named, "translates_version_id")?;
                    Some(translated_source_version(conn, &lineage_id, branch_id, named)?)
                }
                None => head_translates.clone(),
            };

            // A Language that was only the writer's own, taken for a text too
            // short to read, ends with the first save that gives the recipe
            // Ingredient Lines or Steps: it takes the Language their text
            // reads as, or keeps the guess as the cook's own where it reads as
            // none (#321). Nobody said it, so this changes nothing anybody
            // said. A recipe with a Translation keeps it, since a Translation
            // is in another Language than its source (ADR 0006).
            let fills = ["ingredients", "steps"]
                .iter()
                .any(|field| content[field].as_array().is_some_and(|lines| !lines.is_empty()));
            let settles = language_guessed && fills;
            let language = if settles && !has_translations(conn, &lineage_id, branch_id)? {
                crate::language::detect_content(&content).map_or(language, str::to_string)
            } else {
                language
            };
            let language_guessed = language_guessed && !settles;

            // The Language the new text reads as, where it disagrees with the
            // one the Branch carries. Computed before the identical-content
            // shortcut below so that re-saving unchanged text still answers
            // it: the offer is about the recipe, not about this save.
            let offer = language_offer(&language, &content);

            // The half of "still being shaped" the head row answers by itself.
            // A person shapes the Version they saved within the collapse
            // window. An agent's Job shapes the one Version it wrote, however
            // long ago, and never one that was there before it. Nobody else's
            // save folds into a Job's Version, the asker's own included,
            // though the Job saved it under their Hand (ADR 0048, #265).
            let head_is_callers_to_shape = match &caller.job_id {
                Some(job_id) => head_job_id.as_deref() == Some(job_id.as_str()),
                None => {
                    head_job_id.is_none() && within_window && head_hand_id == caller.person_id
                }
            };

            // A Job leaves at most one new Version (ADR 0048, #265), so once
            // it has saved, its next save has that Version to fold into or
            // is refused. Somebody else's save since would be written over by
            // a whole-recipe save. A Copy or a Translation holding the Job's
            // Version closes it as it does anybody's, and appending instead
            // would be a second Version. Either way the Job fails with the
            // reason and keeps what it saved. Asked before anything is
            // written, and of a save that changes nothing too, so that a Job
            // is told the same thing whatever it sent.
            if let Some(job_id) = &caller.job_id {
                let refused = if head_is_callers_to_shape {
                    version_has_left(conn, &lineage_id, branch_id, &head_version_id)?
                        .then_some(crate::jobs::HELD_ELSEWHERE)
                } else {
                    job_has_saved_onto(conn, job_id, branch_id)?
                        .then_some(crate::jobs::CHANGED_WHILE_IT_WORKED)
                };
                if let Some(reason) = refused {
                    crate::jobs::end_where_a_write_was_refused(conn, job_id, reason)?;
                    return Err(OpError::bad_request(reason));
                }
            }

            if version_id == head_version_id {
                // Identical content: the fingerprint already names this state,
                // so there is nothing new to save — and merely reading a
                // recipe you cannot change must never start a Copy.
                //
                // Unless the call brings a name or change note the head does
                // not carry yet (#165). Those belong to the Version still
                // being shaped, the one a content change would collapse into,
                // and are written onto it. Past that point there is no change
                // left for them to describe, and saying so beats dropping them.
                //
                // Or a source Version the head does not name yet (#183): the
                // same words render a newer source as truly as the old one,
                // when the source changed where the Translation needed no
                // change. The pointer is a claim about the words as they
                // stand, not a description of a change, so it moves at any
                // time, being shaped or not. It still moves only on a Branch
                // the caller's Cookbook writes.
                let new_name = name.filter(|n| Some(*n) != head_name.as_deref());
                let new_note = change_note.filter(|n| Some(*n) != head_change_note.as_deref());
                let noted = new_name.is_some() || new_note.is_some();
                let names_new_source = translates_version_id != head_translates;
                // Every refusal before the one write: one call, one outcome.
                if (noted || names_new_source)
                    && !cookbook_writes_branch(conn, &writing_for, branch_id)?
                {
                    return Err(OpError::bad_request(if noted {
                        "nothing in the recipe changed, and it is not yours to change, \
                         so there is no change of yours for this name or change note \
                         to describe"
                    } else {
                        "nothing in the recipe changed, and it is not yours to change, \
                         so it is not yours to say which Version of its source it renders"
                    }));
                }
                if noted
                    && !version_being_shaped(
                        conn,
                        &lineage_id,
                        branch_id,
                        &head_version_id,
                        head_is_callers_to_shape,
                    )?
                {
                    return Err(OpError::bad_request(
                        "nothing in the recipe changed, and its newest Version is no \
                         longer being shaped, so there is no change for this name or \
                         change note to describe",
                    ));
                }
                if noted || names_new_source {
                    conn.execute(
                        "UPDATE branch_versions SET name = COALESCE(?1, name), \
                                change_note = COALESCE(?2, change_note), \
                                translates_version_id = ?3 \
                         WHERE branch_id = ?4 AND sequence = ?5",
                        params![
                            new_name,
                            new_note,
                            translates_version_id,
                            branch_id,
                            head_sequence
                        ],
                    )
                    .map_err(|e| OpError::internal(format!("cannot note Version: {e}")))?;
                }
                return Ok(json!({
                    "branch_id": branch_id,
                    "version_id": version_id,
                    "parent_version_id": head_parent_id,
                    "sequence": head_sequence,
                    "collapsed": noted,
                    "copied": false,
                    "language": language,
                    "language_offer": offer,
                    "translates_version_id": translates_version_id,
                    "foods_created": [],
                }));
            }

            conn.execute(
                "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                params![version_id, content_text],
            )
            .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;

            // A Reading travels with the Version it belongs to and is never
            // recomputed (ADR 0021) — so wherever this save left a line
            // reading exactly as it did before, wherever it now sits (#355),
            // its Reading carries forward onto the new Version rather than
            // being silently lost. A line that actually changed loses its
            // Reading, which is ADR 0002's "re-reading the edited line
            // refreshes the Reading" — and the refresh itself happens
            // directly below. This holds identically for a Copy's first
            // save: it is starting exactly at this head.
            let kept = kept_lines(&head_content, &content);
            carry_forward_readings(conn, &head_version_id, &version_id, &kept)?;
            // So does a person's correction of which lines a Step uses, while
            // the Step and those lines are as they were (#439).
            carry_forward_step_uses(
                conn,
                &head_version_id,
                &version_id,
                &kept_steps(&head_content, &content),
                &kept,
            )?;
            // ADR 0002's other half, which the comment above used to defer:
            // a line this save actually wrote is read now (#71). A line it
            // did not write keeps whatever it had, so a Reading somebody
            // cleared on purpose stays cleared.
            //
            // A save bringing a Translation up to a newer source renders it
            // afresh, so the lines it wrote take the Food of the source line
            // in their place (#356). A plain edit is somebody changing the
            // Translation's own words, and a line it swapped for another
            // ingredient at the same amount would take the wrong Food.
            let renders_anew = translates_version_id
                .as_deref()
                .filter(|_| translates_version_id != head_translates);
            let foods_created = foods_created_json(
                &read_unread_lines(
                    conn,
                    &version_id,
                    &language,
                    &content,
                    Some(&lines_this_save_wrote(&kept)),
                    renders_anew,
                )
                .foods_created,
            );

            if !cookbook_writes_branch(conn, &writing_for, branch_id)? {
                // Copy: your Cookbook did not write this Branch, so the change
                // starts a new one of your own — the source Branch is left
                // exactly as it was.
                let new_branch_id = start_copy(
                    conn,
                    branch_id,
                    &lineage_id,
                    &writing_for,
                    &caller.person_id,
                    &language,
                    &version_id,
                )?;
                // Settled or not, the guess stands on the Copy as this save
                // left it (#321).
                conn.execute(
                    "UPDATE branches SET language_guessed = ?1 WHERE id = ?2",
                    params![language_guessed, new_branch_id],
                )
                .map_err(|e| OpError::internal(format!("cannot start the Copy: {e}")))?;

                conn.execute(
                    "INSERT INTO branch_versions \
                     (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, translates_version_id, language) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        new_branch_id,
                        head_sequence + 1,
                        version_id,
                        head_version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id,
                        translates_version_id,
                        language
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot append Version: {e}")))?;

                return Ok(json!({
                    "branch_id": new_branch_id,
                    "version_id": version_id,
                    "parent_version_id": head_version_id,
                    "sequence": head_sequence + 1,
                    "collapsed": false,
                    "copied": true,
                    "language": language,
                    "language_offer": offer,
                    "translates_version_id": translates_version_id,
                    "foods_created": foods_created,
                }));
            }

            // A collapse *replaces* the Version at the head sequence rather
            // than appending after it — which is exactly right for a rapid
            // re-save nobody else has seen, and exactly wrong once a
            // Translation of this recipe says it renders that Version. The
            // pointer would be left naming text that no longer occurs
            // anywhere, and how far behind the Translation had fallen would
            // stop being answerable at all (ADR 0006). So the collapse window
            // closes the moment somebody translates you: the save appends, and
            // the Translation goes honestly one Version behind instead of
            // silently losing its footing.
            //
            // A Copy closes it for the same reason and one degree more
            // literally (issue #82): a Copy holds the source's chain verbatim,
            // so it names this Version in a row of its own. Rewrite the
            // source's row and the two chains never intersect again — the
            // Branch Point, the Divergence and the fork the Thread draws are
            // all lost at once, silently and permanently. The window exists so
            // that a person still shaping a save does not litter their own
            // history, which is right; the moment another Branch's chain
            // references that Version it has stopped being the Version being
            // shaped and become a shared fact. Appending costs one extra
            // Version, which is honest, because somebody else really is
            // holding the old one.
            let collapse = version_being_shaped(
                conn,
                &lineage_id,
                branch_id,
                &head_version_id,
                head_is_callers_to_shape,
            )?;
            if collapse {
                // A name or change note the call leaves out is the one the
                // Version already carries, not an erasure (#165).
                conn.execute(
                    "UPDATE branch_versions SET version_id = ?1, hand_id = ?2, \
                            name = COALESCE(?3, name), \
                            change_note = COALESCE(?4, change_note), access_key_id = ?5, \
                            translates_version_id = ?8, language = ?9, \
                            created_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                     WHERE branch_id = ?6 AND sequence = ?7",
                    params![
                        version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id,
                        branch_id,
                        head_sequence,
                        translates_version_id,
                        language
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot collapse Version: {e}")))?;
            } else {
                // Which Job wrote it is kept beside which Access Key did, and
                // is as local (#265).
                conn.execute(
                    "INSERT INTO branch_versions \
                     (branch_id, sequence, version_id, parent_version_id, hand_id, name, change_note, access_key_id, translates_version_id, language, job_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        branch_id,
                        head_sequence + 1,
                        version_id,
                        head_version_id,
                        caller.person_id,
                        name,
                        change_note,
                        caller.access_key_id,
                        translates_version_id,
                        language,
                        caller.job_id
                    ],
                )
                .map_err(|e| OpError::internal(format!("cannot append Version: {e}")))?;
            }
            conn.execute(
                "UPDATE branches SET head_version_id = ?1, language = ?3, language_guessed = ?4 \
                  WHERE id = ?2",
                params![version_id, branch_id, language, language_guessed],
            )
            .map_err(|e| OpError::internal(format!("cannot move Branch head: {e}")))?;

            Ok(json!({
                "branch_id": branch_id,
                "version_id": version_id,
                "parent_version_id": if collapse { head_parent_id } else { Some(head_version_id) },
                "sequence": if collapse { head_sequence } else { head_sequence + 1 },
                "collapsed": collapse,
                "copied": false,
                "language": language,
                "language_offer": offer,
                "translates_version_id": translates_version_id,
                "foods_created": foods_created,
            }))
        })
    }

    /// **Put an older Version back** (#354): it becomes the recipe again, as a
    /// new Version on top of the newest, every field as it held it.
    ///
    /// It never collapses into the Version being shaped, whoever saved that
    /// and however recently. A collapse would replace the Version the cook is
    /// stepping away from, and the Thread keeps it.
    ///
    /// Nothing is read or carried: the Version is already written, and its
    /// Readings are keyed on its id, so a Reading corrected on it comes back
    /// corrected, and one cleared on it stays cleared. It keeps the source
    /// Version it said it translates, and the Language is the Branch's own,
    /// settled where it was only guessed, as a save settles it.
    ///
    /// Only for those who write the Branch's Cookbook. Anybody else who sees
    /// it keeps the Version as their own copy instead.
    pub fn put_back_version(
        &self,
        caller: &Caller,
        branch_id: &str,
        version_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, &caller.person_id)?;
            let own = cookbook_of_person(conn, &caller.person_id)?;
            if !cookbook_writes_branch(conn, &own, branch_id)? {
                return Err(OpError::unauthorized(
                    "only the people who write this recipe's Cookbook may put one of its \
                     Versions back; keep it as your own copy instead",
                ));
            }
            let (_, translates_version_id) = occurrence(conn, branch_id, version_id)?;
            #[allow(clippy::type_complexity)]
            let (head_sequence, head_version_id, lineage_id, language, language_guessed): (
                i64,
                String,
                String,
                String,
                bool,
            ) = conn
                .query_row(
                    "SELECT branch_versions.sequence, branches.head_version_id, \
                            branches.lineage_id, branches.language, branches.language_guessed \
                       FROM branches JOIN branch_versions ON branch_versions.branch_id = branches.id \
                      WHERE branches.id = ?1 ORDER BY branch_versions.sequence DESC LIMIT 1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch head: {e}")))?;
            if version_id == head_version_id {
                return Err(OpError::bad_request(
                    "this Version is the recipe as it stands already",
                ));
            }
            let content = version_content(conn, version_id)?;
            // A guessed Language settles as a save of the same content would
            // settle it (#321).
            let fills = ["ingredients", "steps"]
                .iter()
                .any(|field| content[field].as_array().is_some_and(|lines| !lines.is_empty()));
            let settles = language_guessed && fills;
            let language = if settles && !has_translations(conn, &lineage_id, branch_id)? {
                crate::language::detect_content(&content).map_or(language, str::to_string)
            } else {
                language
            };
            conn.execute(
                "INSERT INTO branch_versions \
                 (branch_id, sequence, version_id, parent_version_id, hand_id, access_key_id, translates_version_id, language) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    branch_id,
                    head_sequence + 1,
                    version_id,
                    head_version_id,
                    caller.person_id,
                    caller.access_key_id,
                    translates_version_id,
                    language
                ],
            )
            .map_err(|e| OpError::internal(format!("cannot append Version: {e}")))?;
            conn.execute(
                "UPDATE branches SET head_version_id = ?1, language = ?3, \
                        language_guessed = ?4 WHERE id = ?2",
                params![version_id, branch_id, language, language_guessed && !settles],
            )
            .map_err(|e| OpError::internal(format!("cannot move Branch head: {e}")))?;
            Ok(json!({
                "branch_id": branch_id,
                "version_id": version_id,
                "parent_version_id": head_version_id,
                "sequence": head_sequence + 1,
                "collapsed": false,
                "copied": false,
                "language_offer": language_offer(&language, &content),
                "language": language,
                "translates_version_id": translates_version_id,
            }))
        })
    }

    /// Rename a Version — the one thing about it that can change later
    /// (GLOSSARY.md, "Version"). An absent or empty name clears it. Targeted
    /// by `sequence` rather than `version_id`: the same content can recur
    /// more than once on one Branch (a save reverting to exact earlier
    /// text), and each occurrence carries its own independent name — the
    /// content hash alone cannot tell them apart, but `sequence` always can.
    /// Returns the name as it was actually stored, trimmed and cleared.
    /// Only the Hand that saved that occurrence may rename it (#115).
    pub fn rename_version(
        &self,
        person_id: &str,
        branch_id: &str,
        sequence: i64,
        name: Option<&str>,
    ) -> Result<Option<String>, OpError> {
        let name = name.map(str::trim).filter(|n| !n.is_empty());
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let hand_id: String = conn
                .query_row(
                    "SELECT hand_id FROM branch_versions WHERE branch_id = ?1 AND sequence = ?2",
                    params![branch_id, sequence],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?
                .ok_or_else(|| OpError::not_found("that sequence does not occur on this Branch"))?;
            // A name is its writer's to give (#115). Everyone who sees the
            // recipe reads it; only the Hand that saved the Version changes it.
            if hand_id != person_id {
                return Err(OpError::unauthorized(
                    "only the cook who saved a Version may rename it",
                ));
            }
            conn.execute(
                "UPDATE branch_versions SET name = ?1 WHERE branch_id = ?2 AND sequence = ?3",
                params![name, branch_id, sequence],
            )
            .map_err(|e| OpError::internal(format!("cannot rename Version: {e}")))?;
            Ok(name.map(str::to_string))
        })
    }

    /// Take one Branch off the shelf for good (#120).
    ///
    /// **A Branch, never a Lineage and never a Version.** A translation is an
    /// ordinary Branch (ADR 0006), so deleting the English one leaves the
    /// French one whole; another Cookbook's copy of the same Lineage is not
    /// this Person's to touch and is not touched.
    ///
    /// **Only its Co-authors may, and whoever cooked it keeps it** (ADR 0041,
    /// #131 question 11). Everybody else who could see the recipe and cooked
    /// from it keeps a Branch of it in their own Cookbook first, exactly as
    /// they would had its Cookbook left their Kitchen.
    ///
    /// What goes is everything keyed on this Branch and nothing else. What
    /// stays, and why each one has to:
    ///
    /// - **`versions`.** Never deleted, by anyone, ever (ADR 0004, spec item
    ///   59). A Version is content-addressed and global: another Kitchen that
    ///   reached identical content holds the very same row.
    /// - **`readings`.** Keyed on `version_id`, not on a Branch. Deleting
    ///   this Branch's Readings would take the Reading corrections off
    ///   somebody else's copy of the same recipe.
    /// - **`attempts`.** The cooking history survives the recipe, chosen
    ///   deliberately on #120: losing eleven cooks because you tidied a
    ///   duplicate is a loss nobody asked for. An Attempt is a private diary
    ///   entry deleted on its own terms (ADR 0010).
    /// - **`lineages`.** `attempts.lineage_id` still references the row and
    ///   foreign keys are enforced. A Lineage with no Branch left is an id and
    ///   a date, and costs nothing.
    /// - **`related_recipes`.** It stores both Lineages' names precisely so a
    ///   line stays readable when either leaves the shelf.
    /// - **`recipe_opens`.** Keyed on the Lineage, which a sibling Branch may
    ///   still stand on. A row left naming a Lineage with no Branch is inert:
    ///   Home reads these only as an ordering over recipes already on the
    ///   shelf, so one that is not there cannot be ranked onto it.
    /// - **`shopping_choices`.** The entry stays and comes alive: it keeps the
    ///   name it was known by, contributes no rows, and says it can no longer
    ///   be read (ADR 0024). A thing that quietly disappears from a shopping
    ///   list is a thing that does not get bought. Migration 33 dropped the
    ///   foreign key that would otherwise have refused this whole delete.
    /// - **`foods`.** Untouched, for the reason `delete_food` records: a
    ///   deleted recipe must never quietly discard the fact that a cup of this
    ///   flour is 125 g.
    ///
    /// **Photographs need nothing here.** The orphan sweep recomputes what is
    /// referenced by joining `versions` to `branch_versions` and to `attempts`,
    /// so dropping this Branch's `branch_versions` rows takes its pictures out
    /// of that join on their own, a week later. A picture an Attempt still
    /// names survives, which is the right answer — and that reaches further
    /// than the picture the cook took at the stove. An Attempt names the
    /// Version it was cooked from, and a Version is never rewritten, so a
    /// Main Photo somebody actually cooked from stays too. Neither rule had to
    /// be taught to the sweep; both fall out of asking what is referenced now.
    ///
    /// **One transaction, referencing rows first.** `foreign_keys` is ON, so a
    /// table added later that references a Branch and is not swept here fails
    /// the whole delete loudly rather than silently orphaning a row. That
    /// refusal is a safety net worth keeping.
    pub fn delete_recipe(&self, caller: &Caller, branch_id: &str) -> Result<(), OpError> {
        self.db().with_conn(|conn| {
            // ADR 0040: a Person who may not see the recipe is answered
            // exactly as an id naming nothing is; one who sees it and does not
            // write it is told they may not. An Operator looks after it (#302).
            let cookbook_id = ensure_changes_branch(conn, branch_id, caller, no_such_branch)?;

            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let cooks: Vec<String> = {
                let mut statement = conn
                    .prepare(
                        "SELECT visible_cookbooks.person_id FROM visible_cookbooks \
                          WHERE visible_cookbooks.cookbook_id = ?1 \
                            AND visible_cookbooks.person_id NOT IN \
                                (SELECT person_id FROM cookbook_authors WHERE cookbook_id = ?1) \
                          ORDER BY visible_cookbooks.person_id",
                    )
                    .map_err(|e| OpError::internal(format!("cannot read who cooks this: {e}")))?;
                statement
                    .query_map(params![cookbook_id], |row| row.get(0))
                    .map_err(|e| OpError::internal(format!("cannot read who cooks this: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read who cooks this: {e}")))?
            };
            let mut kept: Vec<(String, String)> = Vec::new();
            for cook in cooks {
                if cooked_from(conn, &cookbook_id, &cook)?
                    .iter()
                    .any(|b| b == branch_id)
                {
                    kept.push((cook, branch_id.to_string()));
                }
            }
            keep_branches(conn, &kept)?;
            delete_branch_rows(conn, branch_id)?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot delete Recipe: {e}")))?;
            Ok(())
        })
    }

    /// Remember that this Person opened this recipe, for Home's *recently
    /// opened* shelf (ADR 0027).
    ///
    /// One fact per Person per Lineage — opening the French Branch and the
    /// English one is opening the same recipe — and opening it again moves the
    /// timestamp rather than adding a row.
    ///
    /// It is its own Operation rather than something [`Self::get_recipe`] does
    /// on the side, because `get_recipe` does not write and must not start: a
    /// read-only Access Key is a Credential that may read every recipe, and
    /// making the reading itself a write would lock it out of the library. The
    /// cost is that a read-only Key's opening is not remembered, which is
    /// exactly the kind of loss ADR 0027 said this fact may take.
    pub fn note_recipe_opened(&self, person_id: &str, branch_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            ensure_sees_branch(conn, branch_id, person_id)?;
            let lineage_id: String = conn
                .query_row(
                    "SELECT lineage_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;

            conn.execute(
                "INSERT INTO recipe_opens (person_id, lineage_id) VALUES (?1, ?2) \
                 ON CONFLICT(person_id, lineage_id) DO UPDATE SET \
                   opened_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
                params![person_id, lineage_id],
            )
            .map_err(|e| OpError::internal(format!("cannot remember the opening: {e}")))?;

            let opened_at: String = conn
                .query_row(
                    "SELECT opened_at FROM recipe_opens WHERE person_id = ?1 AND lineage_id = ?2",
                    params![person_id, lineage_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read the opening back: {e}")))?;
            Ok(json!({ "lineage_id": lineage_id, "opened_at": opened_at }))
        })
    }

    /// Read a Recipe: the Branch as it stands and its whole chain of Versions,
    /// oldest first — the Thread's raw material.
    pub fn get_recipe(
        &self,
        caller: &Caller,
        branch_id: &str,
        wanted: Option<&Value>,
    ) -> Result<Value, OpError> {
        let person_id = caller.person_id.as_str();
        self.db().with_conn(|conn| {
            let cookbook_id = ensure_reaches_branch(conn, branch_id, caller)?;
            let (lineage_id, hand_id, language, origin_address, head_version_id, branch_name, arrived): (
                String,
                String,
                String,
                Option<String>,
                String,
                Option<String>,
                bool,
            ) = conn
                .query_row(
                    "SELECT lineage_id, hand_id, language, origin_address, head_version_id, name, \
                            arrived \
                       FROM branches WHERE id = ?1",
                    params![branch_id],
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
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;

            let mut statement = conn
                .prepare(
                    "SELECT branch_versions.sequence, branch_versions.version_id, \
                            branch_versions.parent_version_id, branch_versions.hand_id, \
                            branch_versions.name, branch_versions.change_note, \
                            branch_versions.created_at, versions.content, \
                            branch_versions.translates_version_id, branch_versions.language \
                       FROM branch_versions JOIN versions ON versions.id = branch_versions.version_id \
                      WHERE branch_versions.branch_id = ?1 ORDER BY branch_versions.sequence ASC",
                )
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;
            let mut versions: Vec<Value> = statement
                .query_map(params![branch_id], |row| {
                    let content: String = row.get(7)?;
                    Ok(json!({
                        "sequence": row.get::<_, i64>(0)?,
                        "version_id": row.get::<_, String>(1)?,
                        "parent_version_id": row.get::<_, Option<String>>(2)?,
                        "hand_id": row.get::<_, String>(3)?,
                        "name": row.get::<_, Option<String>>(4)?,
                        "change_note": row.get::<_, Option<String>>(5)?,
                        "created_at": row.get::<_, String>(6)?,
                        "content": serde_json::from_str::<Value>(&content)
                            .map(content_as_declared)
                            .unwrap_or(Value::Null),
                        // What this Version renders, and the Language it stood
                        // in when it was written — both on the occurrence, so
                        // the history is exact at every point in it (ADR 0006).
                        "translates_version_id": row.get::<_, Option<String>>(8)?,
                        "language": row.get::<_, Option<String>>(9)?,
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;

            // A Reading never sits inside `content` (ADR 0021), so it is
            // fetched separately here and laid alongside it: one slot per
            // Ingredient Line, null wherever no Reading has been recorded.
            //
            // How this Person reads is read once here rather than per Version:
            // it is a fact about the reader, and a long-edited recipe has many
            // Versions.
            let reader = Reader::of(conn, person_id)?;
            let named = wanted.map(parse_named_yield).transpose()?;
            let wanted = wanted_yield(conn, &lineage_id, person_id, named.as_ref())?;
            for version in &mut versions {
                let scale = yield_scale(&wanted, &version["content"]["yield"]);
                // Which Yield this Version's amounts were scaled to, or null
                // where they are as written — so a screen can say which lines
                // did not scale without working out a ratio itself (#109), and
                // can tell an answer scaled to what it now asks for from one
                // it was kept from before (#77).
                version["scaled_to"] = if scale == 1.0 {
                    Value::Null
                } else {
                    wanted.clone()
                };
                let version_id = version["version_id"]
                    .as_str()
                    .expect("version_id is always a string")
                    .to_string();
                let line_count = version["content"]["ingredients"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0);
                version["readings"] = json!(readings_for_version(conn, &version_id, line_count)?);
                // The one subordinate line each of those Readings produces for
                // THIS reader — scaling and conversion in one slot (ADR 0016,
                // #49). A recipe being read rather than cooked is at the Yield
                // as written, which is a scale of one.
                // Read in the Language this occurrence stood in, and the
                // Branch's where an older one recorded none.
                let written_in_language = version["language"]
                    .as_str()
                    .unwrap_or(&language)
                    .to_string();
                version["measured"] = measured_for_version(
                    conn,
                    &version["content"],
                    &version_id,
                    &written_in_language,
                    &reader,
                    scale,
                )?;
                // And what the cooking screen reads out of each Step, from the
                // Readings just laid alongside (ADR 0011). Derived here, on
                // every read, so both Doors say the same thing, and stored
                // only where a person corrected it (ADR 0054).
                let readings = version["readings"].as_array().cloned().unwrap_or_default();
                version["cooking"] = cooking_for_version(
                    &version["content"],
                    &readings,
                    version["language"].as_str(),
                    &step_uses_by_hand(conn, &version_id)?,
                );
                // A Component unfolded (ADR 0008): the inner recipe, already
                // scaled by how much of it this line asks for, to whatever
                // depth the composition goes. Empty for nearly every recipe.
                //
                // The chain starts with this Lineage in it, so a recipe naming
                // itself is a repeat like any other rather than a special case.
                let mut walk = Unfolding {
                    open: vec![lineage_id.clone()],
                    ..Unfolding::default()
                };
                unfold_components(
                    conn,
                    &Unfolds::ForReader { person_id, reader: &reader },
                    &version_id,
                    scale,
                    &mut walk,
                )?;
                version["components"] = json!(walk.found);
            }

            let own_cookbook_id = cookbook_of_person(conn, person_id)?;
            Ok(json!({
                "branch_id": branch_id,
                "lineage_id": lineage_id,
                // Whose recipe this is, and whether the reader writes it: a
                // save by anybody else starts a Branch of their own (ADR 0041),
                // which the writing screens say before the tap. An Operator
                // looking after the Cookbook writes it as its Co-authors do.
                "cookbook": cookbook_label(conn, &cookbook_id)?,
                "name": branch_name,
                "writes": cookbook_writes_branch(conn, &changes_for(conn, caller, branch_id)?, branch_id)?,
                // And why not, where not (#132): in the reader's own Cookbook
                // but sent from elsewhere, or in somebody else's Cookbook. The
                // same two facts a Thread's Branch answers.
                "mine": cookbook_id == own_cookbook_id,
                "arrived": arrived,
                "hand_id": hand_id,
                "language": language,
                "origin_address": origin_address,
                "head_version_id": head_version_id,
                "versions": versions,
                // How this recipe stands as a Translation, or null — which is
                // what the overwhelming majority of recipes are. Computed, not
                // stored: the original is the Branch that translates nothing,
                // and how far behind this one has fallen is arithmetic over
                // the source Branch as it stands right now (ADR 0006).
                "translation": translation_of_branch(conn, &lineage_id, branch_id)?,
                // Beside the Versions rather than inside any of them: a Tag is
                // how this Cookbook files the recipe, not part of what the
                // recipe is, so it belongs to the Branch as it stands now and
                // to no Version's content (ADR 0035).
                "tags": tags_of_branch(conn, branch_id, person_id)?,
                // Related Recipes are shelf notes between Lineages, kept by the
                // Cookbook with its recipes (#131, question 3). They sit beside
                // the Thread just as Tags do, never inside a Version.
                "related_recipes": related_recipes_of_lineage(
                    conn,
                    &cookbook_id,
                    &lineage_id,
                    person_id,
                    looks_after(conn, caller)?.then_some(cookbook_id.as_str()),
                )?,
                // How this dish has been cooked: how many times, when last,
                // and each Person's most recent rating by name (#59). Beside
                // the Versions and never inside one — an Attempt is a private
                // diary entry, not part of what the recipe is, which is the
                // whole of why none of this can reach a Share Link.
                "cooked": cooking_record(conn, &lineage_id, branch_id, &head_version_id, person_id)?,
            }))
        })
    }

    /// Read the Thread: every Version of every Branch of one Lineage this
    /// Person can see, oldest first per Branch, with every Attempt hanging
    /// off it (GLOSSARY.md, "Thread"). `branch_id` is only the entry point —
    /// any Branch of the Lineage answers the same Thread. Forking itself is
    /// left for the caller to read out of `parent_version_id`, or to ask
    /// `branch_point` to compute authoritatively; this never calls it, so
    /// the two stay independently testable.
    pub fn get_thread(&self, caller: &Caller, branch_id: &str) -> Result<Value, OpError> {
        let person_id = caller.person_id.as_str();
        self.db().with_conn(|conn| {
            let entry_cookbook_id = ensure_reaches_branch(conn, branch_id, caller)?;
            // An Operator looking after the Cookbook the Thread was entered
            // from reads that Cookbook's Branches too, as its Co-authors do
            // (#302). An Operator's Job Key reaching its Job's recipe there
            // reads that one Branch, and nobody reads a Branch of a Cookbook
            // they could neither see nor were let into.
            let (looked_after, reached): (Option<String>, Option<&str>) =
                if looks_after(conn, caller)? {
                    (Some(entry_cookbook_id), None)
                } else if sees_cookbook(conn, &entry_cookbook_id, person_id)? {
                    (None, None)
                } else {
                    (None, Some(branch_id))
                };
            let lineage_id: String = conn
                .query_row(
                    "SELECT lineage_id FROM branches WHERE id = ?1",
                    params![branch_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?;
            let own_cookbook_id = cookbook_of_person(conn, person_id)?;

            // Every Branch of this Lineage in a Cookbook this Person may see —
            // never one they may not, the same boundary a single `get_recipe`
            // enforces (ADR 0041).
            let mut statement = conn
                .prepare(&format!(
                    "SELECT branches.id, branches.cookbook_id, branches.hand_id, \
                            branches.language, branches.head_version_id, {}, \
                            branches.name, branches.arrived \
                       FROM branches \
                      WHERE branches.lineage_id = ?1 AND ({} OR branches.cookbook_id = ?3 OR branches.id = ?4) \
                      ORDER BY branches.created_at ASC, branches.id ASC",
                    hand_name_sql("branches.hand_id"),
                    visible_to("branches", "?2"),
                ))
                .map_err(|e| OpError::internal(format!("cannot list Branches: {e}")))?;
            let rows: Vec<(Value, String)> = statement
                .query_map(params![lineage_id, person_id, looked_after, reached], |row| {
                    let cookbook_id: String = row.get(1)?;
                    Ok((
                        json!({
                            "branch_id": row.get::<_, String>(0)?,
                            // A variation's own name, null on every other Branch.
                            "name": row.get::<_, Option<String>>(6)?,
                            // Whether this Branch is in the reader's own
                            // Cookbook, and whether it arrived there from
                            // somebody else (ADR 0020): what the switch labels
                            // it by.
                            "mine": cookbook_id == own_cookbook_id,
                            "arrived": row.get::<_, bool>(7)?,
                            "hand_id": row.get::<_, String>(2)?,
                            "language": row.get::<_, String>(3)?,
                            "head_version_id": row.get::<_, String>(4)?,
                            "hand_name": row.get::<_, Option<String>>(5)?,
                        }),
                        cookbook_id,
                    ))
                })
                .map_err(|e| OpError::internal(format!("cannot list Branches: {e}")))?
                .collect::<Result<_, _>>()
                .map_err(|e| OpError::internal(format!("cannot list Branches: {e}")))?;
            let mut branches: Vec<Value> = Vec::new();
            for (mut branch, cookbook_id) in rows {
                branch["cookbook"] = cookbook_label(conn, &cookbook_id)?;
                branches.push(branch);
            }

            // Which of these Branches are Translations, and how far behind
            // each has fallen — the Thread is where a divergence and a
            // Translation are told apart, and both are Branches (ADR 0006).
            for branch in &mut branches {
                let this_branch_id = branch["branch_id"]
                    .as_str()
                    .expect("branch_id is always a string")
                    .to_string();
                branch["translation"] = translation_of_branch(conn, &lineage_id, &this_branch_id)?;
            }

            let mut versions: Vec<Value> = Vec::new();
            let mut visible_version_ids: HashSet<String> = HashSet::new();
            for branch in &branches {
                let this_branch_id = branch["branch_id"]
                    .as_str()
                    .expect("branch_id is always a string");
                let mut vstmt = conn
                    .prepare(&format!(
                        "SELECT sequence, version_id, parent_version_id, hand_id, name, \
                                change_note, created_at, translates_version_id, language, {} \
                           FROM branch_versions WHERE branch_id = ?1 ORDER BY sequence ASC",
                        hand_name_sql("branch_versions.hand_id"),
                    ))
                    .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;
                let rows: Vec<Value> = vstmt
                    .query_map(params![this_branch_id], |row| {
                        Ok(json!({
                            "branch_id": this_branch_id,
                            "sequence": row.get::<_, i64>(0)?,
                            "version_id": row.get::<_, String>(1)?,
                            "parent_version_id": row.get::<_, Option<String>>(2)?,
                            "hand_id": row.get::<_, String>(3)?,
                            "name": row.get::<_, Option<String>>(4)?,
                            "change_note": row.get::<_, Option<String>>(5)?,
                            "created_at": row.get::<_, String>(6)?,
                            "translates_version_id": row.get::<_, Option<String>>(7)?,
                            "language": row.get::<_, Option<String>>(8)?,
                            "hand_name": row.get::<_, Option<String>>(9)?,
                        }))
                    })
                    .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot read Thread: {e}")))?;
                for row in &rows {
                    visible_version_ids.insert(
                        row["version_id"]
                            .as_str()
                            .expect("version_id is always a string")
                            .to_string(),
                    );
                }
                versions.extend(rows);
            }

            Ok(json!({
                "lineage_id": lineage_id,
                "branches": branches,
                "versions": versions,
                "attempts": attempts_for_lineage(conn, &lineage_id, person_id, &visible_version_ids)?,
            }))
        })
    }

    /// The Branch Point between two Branches: the last Version they share,
    /// found by walking both chains back until they meet (GLOSSARY.md,
    /// "Branch Point") — never declared, always computed. Every valid
    /// Branch's chain is contiguous back to a first Version with no parent;
    /// a chain that is not, or two chains that never converge, is reported
    /// as a damaged Bundle rather than answered with a guess. A Translation
    /// is the one healthy pair with no Branch Point, and is refused as such
    /// (`last_shared_version`).
    pub fn branch_point(
        &self,
        person_id: &str,
        branch_a_id: &str,
        branch_b_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let chain_a = ordered_chain(conn, branch_a_id, person_id)?;
            let chain_b = ordered_chain(conn, branch_b_id, person_id)?;
            let version_id =
                last_shared_version(conn, (branch_a_id, &chain_a), (branch_b_id, &chain_b))?;
            Ok(json!({ "version_id": version_id }))
        })
    }

    /// A Divergence: two Branches of one Lineage, laid over each other so a
    /// screen can show **two whole recipes with a switch between them** rather
    /// than a difference (ADR 0014). There is no "the difference" object here
    /// and no summary — every row carries both sides' own words, so whichever
    /// Branch a cook is standing in they read a complete, cookable recipe with
    /// the handful of unshared lines marked, and the lines only the other side
    /// has shown as Ghosts in the position they hold over there.
    ///
    /// Which line is which is worked out by reading both Branches against their
    /// Branch Point (ADR 0019). No id is stapled to any line, so retyping a line
    /// identically manufactures no divergence, and where the reading is
    /// uncertain Kamosu declines to pair and both lines simply stand.
    ///
    /// A Translation has no Branch Point with its source, so that pair is
    /// refused (`last_shared_version`).
    ///
    /// `mine` throughout names the Branch given as `branch_id` — the one the
    /// caller is standing in. The rows themselves are symmetric: crossing over
    /// is reading the same rows from the other side, not asking again.
    pub fn divergence(
        &self,
        person_id: &str,
        branch_id: &str,
        other_branch_id: &str,
    ) -> Result<Value, OpError> {
        if branch_id == other_branch_id {
            return Err(OpError::bad_request(
                "a Branch does not diverge from itself — name two Branches",
            ));
        }
        self.db().with_conn(|conn| {
            // `ordered_chain` checks membership of each Branch's Kitchen, so
            // both halves of this are authorised before anything is read.
            let chain_mine = ordered_chain(conn, branch_id, person_id)?;
            let chain_theirs = ordered_chain(conn, other_branch_id, person_id)?;

            let mine = branch_head(conn, branch_id)?;
            let theirs = branch_head(conn, other_branch_id)?;
            if mine.lineage_id != theirs.lineage_id {
                return Err(OpError::bad_request(
                    "these two Branches are not of the same Lineage — there is \
                     nothing between them to read",
                ));
            }

            let branch_point_id = last_shared_version(
                conn,
                (branch_id, &chain_mine),
                (other_branch_id, &chain_theirs),
            )?;

            two_whole_recipes(conn, person_id, branch_point_id, &mine, &theirs)
        })
    }

    /// **An older Version read against the newest Version of its Branch**
    /// (#211): the same two whole recipes a Divergence answers (ADR 0014),
    /// where the two are one Branch at two moments. `mine` is the Branch as it
    /// stands and `theirs` is the Branch at the Version named, so a screen
    /// standing in the older Version reads the rows from the `theirs` side.
    ///
    /// The older Version is the point the two share, so it is what both are
    /// read against (ADR 0019). A row only `theirs` has is a line taken out
    /// since; a row only `mine` has is a line written since.
    ///
    /// The newest Version answers too, with every row the same and `newest`
    /// true: it is the recipe, and there is nothing to mark.
    pub fn changed_since(
        &self,
        person_id: &str,
        branch_id: &str,
        version_id: &str,
    ) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            // `ordered_chain` checks the caller may see the Branch.
            let chain = ordered_chain(conn, branch_id, person_id)?;
            if !chain.iter().any(|each| each == version_id) {
                return Err(OpError::bad_request(
                    "version_id is not a Version of this Branch",
                ));
            }

            let now = branch_head(conn, branch_id)?;
            // The same content can recur on one Branch. The last time it was
            // saved is the one the Thread shows nearest the head.
            let (sequence, hand_id, name, change_note, created_at, hand_name): (
                i64,
                String,
                Option<String>,
                Option<String>,
                String,
                Option<String>,
            ) = conn
                .query_row(
                    &format!(
                        "SELECT sequence, hand_id, name, change_note, created_at, {} \
                           FROM branch_versions WHERE branch_id = ?1 AND version_id = ?2 \
                          ORDER BY sequence DESC LIMIT 1",
                        hand_name_sql("branch_versions.hand_id"),
                    ),
                    params![branch_id, version_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?;
            let then = BranchHead {
                branch_id: branch_id.to_string(),
                lineage_id: now.lineage_id.clone(),
                cookbook_id: now.cookbook_id.clone(),
                name: now.name.clone(),
                arrived: now.arrived,
                hand_id: hand_id.clone(),
                language: now.language.clone(),
                head_version_id: version_id.to_string(),
            };

            let mut answer = two_whole_recipes(conn, person_id, version_id, &now, &then)?;
            answer["version"] = json!({
                "version_id": version_id,
                "sequence": sequence,
                "name": name,
                "change_note": change_note,
                "created_at": created_at,
                "hand_id": hand_id,
                "hand_name": hand_name,
                "newest": version_id == now.head_version_id,
            });
            // Whether the reader writes this Branch, as `get_recipe` says it:
            // only then can a line be written back into the recipe.
            let own = cookbook_of_person(conn, person_id)?;
            answer["writes"] = json!(cookbook_writes_branch(conn, &own, branch_id)?);
            // Whether keeping this Version as the reader's own copy asks for a
            // name first (#354).
            answer["keeping_needs_name"] = json!(keeping_needs_name(conn, &own, branch_id)?);
            Ok(answer)
        })
    }

    /// **Correct which Ingredient Lines one Step uses** (#439, ADR 0054), on
    /// a Recipe's current state. `uses` is the whole new list, empty for a
    /// Step that uses none; `None` takes the correction away, so the Step
    /// goes back to what Kamosu reads out of its words. Like a corrected
    /// Reading it mints no Version and appears in no history (ADR 0021), and
    /// a save carries it on while the Step and its lines stay as they are.
    pub fn set_step_uses(
        &self,
        caller: &Caller,
        branch_id: &str,
        step_index: i64,
        uses: Option<Vec<i64>>,
    ) -> Result<Value, OpError> {
        if step_index < 0 {
            return Err(OpError::bad_request("step_index must be zero or more"));
        }
        self.db().with_conn(|conn| {
            // The Language the head stood in, as `get_recipe` reads it, so
            // this answers what that answers.
            let (language, head_version_id, content): (Option<String>, String, String) = conn
                .query_row(
                    "SELECT branch_versions.language, branches.head_version_id, versions.content \
                       FROM branches JOIN versions ON versions.id = branches.head_version_id \
                       LEFT JOIN branch_versions ON branch_versions.branch_id = branches.id \
                        AND branch_versions.version_id = branches.head_version_id \
                      WHERE branches.id = ?1 \
                      ORDER BY branch_versions.sequence DESC LIMIT 1",
                    params![branch_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
                .ok_or_else(no_such_branch)?;
            ensure_reaches_branch(conn, branch_id, caller)?;

            let content: Value = serde_json::from_str(&content)
                .map_err(|e| OpError::internal(format!("cannot read Version content: {e}")))?;
            let step = content["steps"]
                .get(step_index as usize)
                .ok_or_else(|| OpError::bad_request("no Step at that index"))?;
            if step["kind"] != "step" {
                return Err(OpError::bad_request(
                    "a Step's Ingredient Lines belong to a Step, not a section",
                ));
            }

            match uses {
                None => {
                    conn.execute(
                        "DELETE FROM step_uses_by_hand WHERE version_id = ?1 AND step_index = ?2",
                        params![head_version_id, step_index],
                    )
                    .map_err(|e| OpError::internal(format!("cannot clear a Step's lines: {e}")))?;
                }
                Some(mut uses) => {
                    uses.sort_unstable();
                    uses.dedup();
                    for &line in &uses {
                        let row = usize::try_from(line)
                            .ok()
                            .and_then(|line| content["ingredients"].get(line));
                        if row.is_none_or(|row| row["kind"] != "ingredient") {
                            return Err(OpError::bad_request(format!(
                                "line {line} is no Ingredient Line of this recipe"
                            )));
                        }
                    }
                    conn.execute(
                        "INSERT INTO step_uses_by_hand (version_id, step_index, uses) \
                         VALUES (?1, ?2, ?3) \
                         ON CONFLICT (version_id, step_index) DO UPDATE SET \
                            uses = excluded.uses, \
                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
                        params![head_version_id, step_index, json!(uses).to_string()],
                    )
                    .map_err(|e| OpError::internal(format!("cannot save a Step's lines: {e}")))?;
                }
            }

            // What the Step now uses, worked out as `get_recipe` works it out,
            // so a cleared correction answers with what Kamosu reads.
            let line_count = content["ingredients"].as_array().map_or(0, Vec::len);
            let readings = readings_for_version(conn, &head_version_id, line_count)?;
            let by_hand = step_uses_by_hand(conn, &head_version_id)?;
            let cooking = cooking_for_version(&content, &readings, language.as_deref(), &by_hand);
            let now = &cooking["steps"][step_index as usize];
            Ok(json!({
                "step_index": step_index,
                "uses": now["uses"],
                "by_hand": now["by_hand"],
            }))
        })
    }
}

/// The Steps of one Version whose Ingredient Lines a person corrected
/// (#439), by Step.
pub(super) fn step_uses_by_hand(
    conn: &Connection,
    version_id: &str,
) -> Result<Vec<(usize, Vec<usize>)>, OpError> {
    let fail = |e: rusqlite::Error| OpError::internal(format!("cannot read a Step's lines: {e}"));
    let mut statement = conn
        .prepare("SELECT step_index, uses FROM step_uses_by_hand WHERE version_id = ?1")
        .map_err(fail)?;
    let rows = statement
        .query_map(params![version_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    Ok(rows
        .into_iter()
        .filter_map(|(step, uses)| {
            Some((
                usize::try_from(step).ok()?,
                serde_json::from_str::<Vec<usize>>(&uses).ok()?,
            ))
        })
        .collect())
}

/// Carry each corrected Step forward onto a freshly saved Version where the
/// save kept the Step word for word and every line it names, onto wherever
/// they now sit (#439). A correction naming a line the save changed or took
/// away is left behind, since what the Step uses may have changed with it.
fn carry_forward_step_uses(
    conn: &Connection,
    old_version_id: &str,
    new_version_id: &str,
    kept_steps: &[Option<usize>],
    kept_lines: &[Option<usize>],
) -> Result<(), OpError> {
    let corrected = step_uses_by_hand(conn, old_version_id)?;
    for (new_step, old_step) in kept_steps.iter().enumerate() {
        let Some((_, uses)) = corrected.iter().find(|(step, _)| Some(*step) == *old_step) else {
            continue;
        };
        let moved: Option<Vec<usize>> = uses
            .iter()
            .map(|old_line| kept_lines.iter().position(|kept| *kept == Some(*old_line)))
            .collect();
        let Some(mut moved) = moved else {
            continue;
        };
        moved.sort_unstable();
        conn.execute(
            "INSERT OR IGNORE INTO step_uses_by_hand (version_id, step_index, uses) VALUES (?1, ?2, ?3)",
            params![new_version_id, new_step as i64, json!(moved).to_string()],
        )
        .map_err(|e| OpError::internal(format!("cannot carry a Step's lines forward: {e}")))?;
    }
    Ok(())
}

/// **Two whole recipes and the rows between them** (ADR 0014): what a
/// Divergence answers, and what an older Version read against its own Branch
/// answers too (#211). Both sides are read against `shared_version_id`, the
/// last Version the two have in common (ADR 0019). For two Branches that is
/// their Branch Point. For one Branch at two moments it is the older Version
/// itself, and the answer still calls it `branch_point_version_id`, the one
/// name a screen reads either answer by.
fn two_whole_recipes(
    conn: &Connection,
    person_id: &str,
    shared_version_id: &str,
    mine: &BranchHead,
    theirs: &BranchHead,
) -> Result<Value, OpError> {
    let base = version_content(conn, shared_version_id)?;
    let content_mine = version_content(conn, &mine.head_version_id)?;
    let content_theirs = version_content(conn, &theirs.head_version_id)?;

    let rows = |field: &str| -> Vec<Value> {
        crate::pairing::read(
            &crate::pairing::Line::list_from(&base, field),
            &crate::pairing::Line::list_from(&content_mine, field),
            &crate::pairing::Line::list_from(&content_theirs, field),
        )
        .iter()
        .map(crate::pairing::Row::to_json)
        .collect()
    };

    // The marking covers the whole recipe, not only the two lists
    // (ADR 0019). These are single values: same or not, with nothing to
    // pair and nothing to get wrong. Tags are deliberately absent —
    // what one Kitchen means by "quick" is its own business (ADR 0035).
    let field = |name: &str| {
        crate::pairing::compare_field(
            content_mine.get(name).unwrap_or(&Value::Null),
            content_theirs.get(name).unwrap_or(&Value::Null),
        )
    };

    Ok(json!({
        "lineage_id": mine.lineage_id,
        "branch_point_version_id": shared_version_id,
        // Each side scales to its OWN written Yield: the two may disagree
        // about it, and the cook is making one number of servings either
        // way.
        "mine": mine.to_json(
            conn,
            &content_mine,
            person_id,
            cooking_scale(conn, &mine.lineage_id, person_id, &content_mine)?,
        )?,
        "theirs": theirs.to_json(
            conn,
            &content_theirs,
            person_id,
            cooking_scale(conn, &theirs.lineage_id, person_id, &content_theirs)?,
        )?,
        "ingredients": rows("ingredients"),
        "steps": rows("steps"),
        "fields": {
            "title": field("title"),
            "yield": field("yield"),
            "prep_time_minutes": field("prep_time_minutes"),
            "cook_time_minutes": field("cook_time_minutes"),
            "source": field("source"),
            "note": field("note"),
            "nutrition": field("nutrition"),
            "main_photo": field("main_photo"),
        },
    }))
}

/// Which Ingredient Line of the old Version each line of the new one keeps
/// word for word (same kind, same text), wherever it now sits (#355): `None`
/// for a line the save wrote, changed or new. The same words twice are
/// matched in order, the first new one to the first old one, and a copy left
/// over is a new line.
///
/// Position alone, which this replaced, lost the Reading of every line below
/// an insert, the thing ADR 0019 warned of. A line whose words changed at all
/// is not paired with what it was: it is read again (ADR 0002). That is why
/// this is not `pairing.rs`, which lets case and spacing pass so that two
/// pages line up for a cook comparing them. A Reading belongs to the exact
/// words it read.
fn kept_lines(old_content: &Value, new_content: &Value) -> Vec<Option<usize>> {
    kept_rows(old_content, new_content, "ingredients", |old, new| {
        old == new
    })
}

/// Which Step of the old Version each Step of the new one keeps word for
/// word, as [`kept_lines`] pairs Ingredient Lines (#439). Only the words
/// count: a photograph put on a Step leaves what it uses as it was.
fn kept_steps(old_content: &Value, new_content: &Value) -> Vec<Option<usize>> {
    kept_rows(old_content, new_content, "steps", |old, new| {
        old["kind"] == new["kind"] && old["text"] == new["text"]
    })
}

/// The pairing [`kept_lines`] and [`kept_steps`] make, over one list, two
/// rows being kept when `same` says so.
fn kept_rows(
    old_content: &Value,
    new_content: &Value,
    list: &str,
    same: impl Fn(&Value, &Value) -> bool,
) -> Vec<Option<usize>> {
    let no_rows = Vec::new();
    let old_rows = old_content[list].as_array().unwrap_or(&no_rows);
    let new_rows = new_content[list].as_array().unwrap_or(&no_rows);
    let mut taken = vec![false; old_rows.len()];
    new_rows
        .iter()
        .map(|new_row| {
            let old_index = (0..old_rows.len())
                .find(|&index| !taken[index] && same(&old_rows[index], new_row))?;
            taken[old_index] = true;
            Some(old_index)
        })
        .collect()
}

/// Carry a Reading forward onto a freshly saved Version for every Ingredient
/// Line the save kept word for word, onto wherever that line now sits. A
/// Reading whose line changed is left behind rather than guessed at.
fn carry_forward_readings(
    conn: &Connection,
    old_version_id: &str,
    new_version_id: &str,
    kept: &[Option<usize>],
) -> Result<(), OpError> {
    for (new_index, old_index) in kept.iter().enumerate() {
        let Some(old_index) = old_index else {
            continue;
        };
        conn.execute(
            // `lineage_id` travels with the rest of the Reading, and leaving it
            // out is not a smaller bug than losing the amount: it is what makes
            // a line a Component (ADR 0008), so a save would have quietly turned
            // every dough inside every pizza back into an ordinary ingredient —
            // on a line nobody touched. Found building #87, which is the first
            // thing to make a Component from the interface and so the first
            // thing that could notice.
            // So does `by_hand`: a correction carried onto the next Version
            // is still a correction, and a re-read must still leave it (#166).
            "INSERT OR IGNORE INTO readings (version_id, line_index, amount, unit, target, food_id, lineage_id, by_hand) \
             SELECT ?1, ?4, amount, unit, target, food_id, lineage_id, by_hand FROM readings \
              WHERE version_id = ?2 AND line_index = ?3",
            params![
                new_version_id,
                old_version_id,
                *old_index as i64,
                new_index as i64
            ],
        )
        .map_err(|e| OpError::internal(format!("cannot carry Reading forward: {e}")))?;
    }
    Ok(())
}

/// Which Ingredient Lines this save actually wrote — the ones that changed and
/// the ones that are new. Exactly the lines [`read_unread_lines`] may read,
/// and the complement of the ones [`carry_forward_readings`] carries, since
/// both are read off the one [`kept_lines`].
fn lines_this_save_wrote(kept: &[Option<usize>]) -> HashSet<usize> {
    (0..kept.len())
        .filter(|&index| kept[index].is_none())
        .collect()
}

/// **How one unfolding resolves and words what it finds** — the whole of what
/// differs between reading your own recipe and carrying a Passenger, so that
/// everything else about unfolding is written once.
///
/// The two used to be two walks, and the walk is the part with the teeth in it:
/// the same SQL, the same repeat guard, the same shape. A fix to the cycle
/// guard that landed in one and not the other would be the worst kind of bug
/// here, because both sides look right in isolation.
pub(super) enum Unfolds<'a> {
    /// **A Person reading their own recipe.** A Component resolves against every
    /// Cookbook they may see, its sentence is in their Language, and the inner
    /// recipe carries the scaled, converted subordinate lines their measures ask
    /// for (#49).
    ForReader {
        person_id: &'a str,
        reader: &'a Reader,
    },
    /// **A Share Link carrying a Passenger** (ADR 0008). There is no reader to
    /// resolve against — a stranger holding the link holds nothing — so the
    /// dough that travels is the one the sharing **Cookbook** holds, and the
    /// sentence is in the recipe's own Language, which is the rule the whole of
    /// that page follows.
    ///
    /// It carries no subordinate line, because that page carries none at all:
    /// Kamosu converts to a kitchen and a stranger has none.
    AsPassenger {
        cookbook_id: &'a str,
        language: &'a str,
    },
    /// **A Passenger printed on a stranger's Sheet** (ADR 0023). Resolved as a
    /// Share Link resolves one, against the sharing Cookbook — but a Sheet
    /// prints every Component already scaled, so it carries the scaled amounts
    /// in the recipe's own measures, converted to nobody's.
    PrintedPassenger {
        cookbook_id: &'a str,
        reader: &'a Reader,
    },
    /// **A Shopping List unfolding to the bottom** (ADR 0008, #86). Resolved
    /// exactly as the Person's own page resolves it, because it is the same
    /// question — which dough do I hold — asked for a different purpose.
    ///
    /// It carries no subordinate line. A list wants the Food each inner line
    /// was read as, which it reads for itself line by line; the worded amount
    /// a page shows beneath a line would be several hundred conversions
    /// nothing on this road ever prints.
    ForShopping {
        person_id: &'a str,
        reader: &'a Reader,
    },
}

impl Unfolds<'_> {
    /// Which Branch of a Lineage this unfolding can see, if any.
    fn resolve(&self, conn: &Connection, lineage_id: &str) -> Result<Option<Held>, OpError> {
        match self {
            Unfolds::ForReader { person_id, .. } | Unfolds::ForShopping { person_id, .. } => {
                branch_of_lineage_for(conn, lineage_id, person_id)
            }
            Unfolds::AsPassenger { cookbook_id, .. }
            | Unfolds::PrintedPassenger { cookbook_id, .. } => {
                branch_of_lineage_in_cookbook(conn, lineage_id, cookbook_id)
            }
        }
    }

    /// The Language the Component's own line is worded in.
    fn language(&self) -> &str {
        match self {
            Unfolds::ForReader { reader, .. }
            | Unfolds::PrintedPassenger { reader, .. }
            | Unfolds::ForShopping { reader, .. } => &reader.language,
            Unfolds::AsPassenger { language, .. } => language,
        }
    }

    /// The inner recipe's subordinate lines, where this unfolding carries them.
    fn measured(
        &self,
        conn: &Connection,
        content: &Value,
        version_id: &str,
        written_in_language: &str,
        scale: f64,
    ) -> Result<Value, OpError> {
        match self {
            Unfolds::ForReader { reader, .. } | Unfolds::PrintedPassenger { reader, .. } => {
                measured_for_version(
                    conn,
                    content,
                    version_id,
                    written_in_language,
                    reader,
                    scale,
                )
            }
            Unfolds::AsPassenger { .. } | Unfolds::ForShopping { .. } => Ok(Value::Null),
        }
    }
}

/// **A Version's Components, unfolded** (ADR 0008) — an entry for every
/// Ingredient Line whose Reading names a Lineage rather than a Food, and for
/// every such line inside those, to whatever depth the composition goes.
/// Nothing at all for a recipe that composes nothing, which is nearly all of
/// them.
///
/// **Flat, depth first, each entry carrying the `path` of line indexes that
/// reaches it.** A tree would be the obvious shape and cannot be declared: ADR
/// 0008 sets no depth limit beyond the repeat guard, and the Catalogue is what
/// the typed client is generated from, so a shape that cannot be declared is a
/// shape the interface silently stops checking. Depth-first order is also the
/// order the page sets a Component's Steps at its foot.
///
/// Three things happen here that are worth knowing before changing any of it.
///
/// **The pointer resolves to a Branch, late.** A Component names a Lineage and
/// never a Version, so it lands on whatever Branch of that dough is in view
/// *now* — which is why editing the dough reaches every recipe using it without
/// minting a Version of any of them. Where it resolves to nothing, `held` is
/// false and there is nothing else to say: the written line already carries the
/// human meaning (ADR 0002), so the screen leaves a sentence rather than a
/// hole. Deleted, never received and held by nobody in view are one case, on
/// purpose — no cascade, no ceremony, no "3 recipes use this".
///
/// **How much is computed and stored nowhere.** `share` is the Reading's
/// quantity over the inner recipe's Yield, worked out at display time by
/// [`units::how_much_of`], carrying the Yield being cooked — ADR 0008's "scaling
/// the outer recipe rescales the Reading, and the factor follows for free" — and
/// it compounds down the chain, so half of a dough that is itself half a starter
/// is a quarter of the starter. A `null` share means Kamosu could not compare
/// the two, and the inner recipe is then handed over exactly as written.
///
/// **A cycle is never refused; unfolding stops.** `walk.open` is the chain of
/// Lineages already open *above* this point, not everything ever seen: two
/// different lines may name the same dough without that being a loop. Meeting
/// one that is already open sets `stopped` and goes no deeper. It is a
/// display-time guard rather than a save-time check because a loop can be
/// assembled from two halves on two servers and arrive already formed — a check
/// at the door could not have held that line and would only give false
/// confidence.
pub(super) fn unfold_components(
    conn: &Connection,
    how: &Unfolds<'_>,
    version_id: &str,
    scale: f64,
    walk: &mut Unfolding,
) -> Result<(), OpError> {
    let mut statement = conn
        .prepare(
            "SELECT line_index, amount, unit, lineage_id FROM readings \
              WHERE version_id = ?1 AND lineage_id IS NOT NULL ORDER BY line_index",
        )
        .map_err(|e| OpError::internal(format!("cannot read Components: {e}")))?;
    let rows: Vec<(i64, Option<String>, Option<String>, String)> = statement
        .query_map(params![version_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read Components: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Components: {e}")))?;

    for (line_index, amount, unit, lineage_id) in rows {
        walk.path.push(line_index);

        // A repeat, met before anything is read: say so, and go no deeper.
        let stopped = walk.open.contains(&lineage_id);
        let found = if stopped {
            None
        } else {
            how.resolve(conn, &lineage_id)?
        };

        let Some(Held {
            branch_id,
            version_id: inner_version_id,
            content,
        }) = found
        else {
            // A stopped repeat IS held — it is open further up this very page —
            // and simply goes no deeper. A Lineage nothing in view holds is not.
            let title = if stopped {
                how.resolve(conn, &lineage_id)?
                    .map_or(Value::Null, |held| held.content["title"].clone())
            } else {
                Value::Null
            };
            walk.found.push(json!({
                "path": walk.path.clone(),
                "lineage_id": lineage_id,
                "held": stopped,
                "stopped": stopped,
                "branch_id": Value::Null,
                "title": title.clone(),
                "share": Value::Null,
                "said": units::component_line(
                    stopped, stopped, title.as_str(), None, how.language(),
                ),
                "content": Value::Null,
                "readings": Value::Null,
                "measured": Value::Null,
            }));
            walk.path.pop();
            continue;
        };

        let share = units::how_much_of(
            amount.as_deref(),
            unit.as_deref(),
            content["yield"]["amount"].as_str(),
            content["yield"]["noun"].as_str(),
        )
        .map(|of_it| of_it * scale);
        // No factor means the inner recipe AS WRITTEN, which is a scale of one
        // and not the outer scale: Kamosu has just said it could not work out
        // how much, so scaling the dough by the pizza's factor anyway would be
        // the guess it declined to make one line above.
        let inner_scale = share.unwrap_or(1.0);

        let line_count = content["ingredients"].as_array().map(Vec::len).unwrap_or(0);
        let readings = readings_for_version(conn, &inner_version_id, line_count)?;
        // The inner recipe's Steps are read in its own Language (#397).
        let inner_language: String = conn
            .query_row(
                "SELECT language FROM branches WHERE id = ?1",
                params![branch_id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read a Component's Branch: {e}")))?;
        let measured = how.measured(
            conn,
            &content,
            &inner_version_id,
            &inner_language,
            inner_scale,
        )?;

        walk.found.push(json!({
            "path": walk.path.clone(),
            "lineage_id": lineage_id.clone(),
            "held": true,
            "stopped": false,
            "branch_id": branch_id,
            "title": content["title"],
            "share": share,
            "said": units::component_line(
                true, false, content["title"].as_str(), share, how.language(),
            ),
            "content": content,
            "readings": readings,
            "measured": measured,
        }));
        walk.inside
            .push((walk.path.clone(), inner_version_id.clone()));

        walk.open.push(lineage_id);
        unfold_components(conn, how, &inner_version_id, inner_scale, walk)?;
        walk.open.pop();
        walk.path.pop();
    }
    Ok(())
}

/// **Where an unfolding has got to**: the chain of Lineages open above this
/// point, the line indexes that reach it, and what has been unfolded so far.
///
/// The three travel together because they are one walk. `open` is what makes a
/// cycle stop rather than be refused, and it is the chain *above* rather than
/// everything visited: two different lines may name the same dough without that
/// being a loop.
#[derive(Default)]
pub(super) struct Unfolding {
    pub(super) open: Vec<String>,
    pub(super) path: Vec<i64>,
    /// Every Component met so far, depth first, in the order the page meets them.
    pub(super) found: Vec<Value>,
    /// **The Version each Component that opened resolved to**, by the path that
    /// reaches it — everything a caller needs to go and read the inner recipe
    /// for itself, as a Shopping List does (#86).
    ///
    /// Beside `found` rather than in it, because `found` is what the Catalogue
    /// declares a Component to be and a Version id is Kamosu's own bookkeeping:
    /// nothing on a page uses it, so nothing on a page should be handed it.
    /// A Component that did not open is absent, which is what makes finding a
    /// path here the same question as *did this one open*.
    pub(super) inside: Vec<(Vec<i64>, String)>,
}

/// The Branch of one Lineage this reader holds, and its head Version's content.
struct Held {
    branch_id: String,
    version_id: String,
    content: Value,
}

/// **Which Branch of a Lineage this reader sees.** A Component names a Lineage,
/// so it resolves to whatever Branch of it the reader may see — their own
/// Cookbook's where they have one, and otherwise the oldest, so a Component
/// reads the same on every screen rather than following whichever row the
/// database happened to return first.
fn branch_of_lineage_for(
    conn: &Connection,
    lineage_id: &str,
    person_id: &str,
) -> Result<Option<Held>, OpError> {
    held_from(
        conn,
        &format!(
            "SELECT branches.id, branches.head_version_id, versions.content \
               FROM branches \
               JOIN versions ON versions.id = branches.head_version_id \
              WHERE branches.lineage_id = ?1 AND {} \
              ORDER BY {} LIMIT 1",
            visible_to("branches", "?2"),
            own_first("branches", "?2"),
        ),
        lineage_id,
        person_id,
    )
}

/// **Which Branch of a Lineage one Cookbook holds** — how a Passenger is chosen.
///
/// A Share Link has no reader to resolve against: a stranger holding the link
/// holds nothing, and the whole point of a Passenger is that they can read the
/// dough anyway (ADR 0008). So the dough that travels is the one the **sharing
/// Cookbook** holds, fixed when the page is read, which is also the only answer
/// that does not leak — resolving against the reader would be resolving against
/// nobody, and resolving against every Cookbook on the instance would carry a
/// dough its own Cookbook never shared.
fn branch_of_lineage_in_cookbook(
    conn: &Connection,
    lineage_id: &str,
    cookbook_id: &str,
) -> Result<Option<Held>, OpError> {
    held_from(
        conn,
        "SELECT branches.id, branches.head_version_id, versions.content \
           FROM branches JOIN versions ON versions.id = branches.head_version_id \
          WHERE branches.lineage_id = ?1 AND branches.cookbook_id = ?2 \
          ORDER BY branches.name IS NULL DESC, branches.created_at ASC, branches.id ASC LIMIT 1",
        lineage_id,
        cookbook_id,
    )
}

/// The one row-to-[`Held`] step both resolvers share. They differ only in what
/// "holds" means; how a held recipe is read does not.
fn held_from(
    conn: &Connection,
    sql: &str,
    lineage_id: &str,
    against: &str,
) -> Result<Option<Held>, OpError> {
    let found: Option<(String, String, String)> = conn
        .query_row(sql, params![lineage_id, against], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .optional()
        .map_err(|e| OpError::internal(format!("cannot resolve a Component: {e}")))?;
    let Some((branch_id, version_id, content)) = found else {
        return Ok(None);
    };
    let content: Value = serde_json::from_str(&content)
        .map(content_as_declared)
        .map_err(|e| OpError::internal(format!("cannot read a Component's content: {e}")))?;
    Ok(Some(Held {
        branch_id,
        version_id,
        content,
    }))
}

/// **What the cooking screen reads out of each Step** (ADR 0011, GLOSSARY.md
/// "Step"): which Ingredient Lines the Step uses, whether a person corrected
/// that, and the duration it offers as a timer.
///
/// One slot per row of `content.steps`, in the same order — the shape
/// `measured` already takes — and `null` on a Section row, which is neither a
/// Step nor something a cook stands on.
///
/// **Nobody has to type a link.** A Step points at no Ingredient Line and an
/// Ingredient Line has no name of its own (ADR 0019); what joins them is the
/// Reading's target, which is a word, or a part of it the Step shortens it to
/// (`crate::step_uses`, #184). So this is worked out here, on every read,
/// from the two things already written — and a Step on a recipe nothing has
/// been read on simply uses nothing, which is the panel degrading to prose
/// rather than failing. Where a person corrected a Step, `by_hand` holds the
/// lines they gave, and those stand instead (ADR 0054), with what was read
/// beside them as `as_read` (#456).
///
/// It is worked out in the Core rather than on the screen so that both Doors
/// get it: an agent asked to read out the next step names the same amounts the
/// phone on the worktop is showing (ADR 0001, ADR 0010).
fn cooking_for_version(
    content: &Value,
    readings: &[Value],
    language: Option<&str>,
    by_hand: &[(usize, Vec<usize>)],
) -> Value {
    let no_lines = Vec::new();
    let step_lines = content["steps"].as_array().unwrap_or(&no_lines);
    let ingredient_rows = content["ingredients"].as_array().unwrap_or(&no_lines);
    let title = content["title"].as_str().unwrap_or_default();
    let uses = crate::step_uses::step_uses(step_lines, ingredient_rows, readings, language, title);

    let steps: Vec<Value> = step_lines
        .iter()
        .zip(uses)
        .enumerate()
        .map(|(at, (line, uses))| match (uses, line["text"].as_str()) {
            (Some(read), Some(text)) => {
                // A person's correction stands in place of what was read
                // (#439, ADR 0054), and what was read is said beside it: it
                // is what putting the correction back returns to (#456).
                let corrected = by_hand.iter().find(|(step, _)| *step == at);
                let duration = units::step_duration(text);
                match corrected {
                    Some((_, uses)) => json!({
                        "uses": uses,
                        "by_hand": true,
                        "as_read": read,
                        "timer_seconds": duration,
                    }),
                    None => json!({
                        "uses": read,
                        "by_hand": false,
                        "timer_seconds": duration,
                    }),
                }
            }
            _ => Value::Null,
        })
        .collect();

    json!({ "steps": steps })
}

/// Where `folded_text` first names this Food, as a whole word with one
/// trailing `s` forgiven. What joins an amount written in a Step to its
/// Ingredient Line (#150): the Food named soonest after the Unit is the one the
/// amount measures.
///
/// Deliberately stricter than the looser join a Step's `uses` makes
/// (`crate::step_uses`, #184): an amount belongs to the Food its words name
/// in full, and loosening that would move which line a converted amount
/// borrows its Cup Weight from.
pub(super) fn named_at(folded_text: &str, folded_target: &str) -> Option<usize> {
    let singular = folded_target.strip_suffix('s').unwrap_or(folded_target);
    whole_word_position(folded_text, singular)
}

/// Where `haystack` contains `needle` as a whole word — a letter or a digit on
/// neither side — allowing one trailing `s` on the word found, which is the
/// plural tolerance [`named_at`] wants.
///
/// An empty `needle` is never contained. Saying so here rather than trusting
/// the caller is what keeps the byte arithmetic below sound: with nothing to
/// advance past, the walk would step a byte at a time through characters it
/// must not split.
fn whole_word_position(haystack: &str, needle: &str) -> Option<usize> {
    let first = needle.chars().next()?;
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        if whole_word_at(haystack, start, needle) {
            return Some(start);
        }
        from = start + first.len_utf8();
    }
    None
}

/// Whether the `needle` found at `start` is a whole word there — a letter or
/// a digit on neither side — allowing one trailing `s`.
fn whole_word_at(haystack: &str, start: usize, needle: &str) -> bool {
    let mut end = start + needle.len();
    let before = haystack[..start].chars().next_back();
    // The plural tolerance: `egg` is found in `eggs`, and the word still has
    // to end there — `egg` is not found in `eggshell`.
    if haystack[end..].starts_with('s') {
        end += 's'.len_utf8();
    }
    let after = haystack[end..].chars().next();
    !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
}

/// How long a gap between saves on the same Branch, by the same Hand, still
/// collapses into the Version already being shaped rather than starting a
/// new one. Chosen at an hour: a save is a deliberate act, never periodic
/// autosave, so genuinely separate editing sessions land far apart in
/// practice — there is no realistic pattern this window would wrongly merge,
/// while it comfortably absorbs one meandering sitting, pauses included
/// (decided on issue #42).
pub const COLLAPSE_WINDOW_SECONDS: i64 = 3600;

/// How a save says the recipe's new content: whole, already parsed, or as
/// only the fields that change (#164).
enum NewContent<'a> {
    Whole(&'a Value),
    Fields(&'a Value),
}

/// Every field of a Recipe's content, as `parse_recipe_content` reads it.
const CONTENT_FIELDS: [&str; 10] = [
    "title",
    "yield",
    "prep_time_minutes",
    "cook_time_minutes",
    "note",
    "main_photo",
    "source",
    "nutrition",
    "ingredients",
    "steps",
];

/// The head's content with each field `changes` names put in its place, for
/// `parse_recipe_content` to read as a whole recipe. A field `changes` leaves
/// out keeps what the head has. One it sends as `null` is taken out, so the
/// parse gives it the empty value every field starts from: `null`, or `[]`
/// for a list. The title has no empty value, so taking it out is refused.
fn merged_onto_head(head: &Value, changes: &Value) -> Value {
    let mut merged = head.clone();
    if let (Some(merged), Some(changes)) = (merged.as_object_mut(), changes.as_object()) {
        for field in CONTENT_FIELDS {
            match changes.get(field) {
                None => {}
                Some(Value::Null) => {
                    merged.remove(field);
                }
                Some(value) => {
                    merged.insert(field.to_string(), value.clone());
                }
            }
        }
    }
    merged
}

/// Build and validate the stored shape of a Recipe's content out of raw
/// request input (#43): the title, the optional Yield, Prep/Cook Time, Note,
/// Source and Nutrition figure, and the Ingredient Line and Step lists — each a flat, ordered
/// sequence in which a Section is a real entry rather than a faked line
/// (GLOSSARY.md, "Section"). Every field but the title is optional and
/// normalises to `null` or `[]` when absent, so `{ "title": "..." }` alone is
/// a complete, valid Recipe. This is the whole state, never a delta: calling
/// it again with fields left out replaces them, exactly as a fresh save of
/// the title alone already did before this ticket.
pub(super) fn parse_recipe_content(input: &Value) -> Result<Value, OpError> {
    let title = input
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("title is required"))?;
    let title = required_text(title, "title")?.to_string();

    let recipe_yield = match input.get("yield") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => parse_yield(value)?,
    };
    let prep_time_minutes =
        parse_optional_minutes(input.get("prep_time_minutes"), "prep_time_minutes")?;
    let cook_time_minutes =
        parse_optional_minutes(input.get("cook_time_minutes"), "cook_time_minutes")?;
    let note = match input.get("note") {
        None | Some(Value::Null) => Value::Null,
        Some(Value::String(text)) => json!(text),
        Some(_) => return Err(OpError::bad_request("note must be a string or null")),
    };
    // The Main Photo is carried only as a reference to a Photograph already
    // uploaded (#45, ADR 0017) — the same loose, unvalidated pointer a Step's
    // photo already is, and part of the fingerprint below for the same reason.
    let main_photo = match input.get("main_photo") {
        None | Some(Value::Null) => None,
        Some(Value::String(photo)) if !photo.is_empty() => Some(photo.as_str()),
        Some(Value::String(_)) => None,
        Some(_) => return Err(OpError::bad_request("main_photo must be a string or null")),
    };
    let source = match input.get("source") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => parse_source(value)?,
    };
    let nutrition = match input.get("nutrition") {
        None | Some(Value::Null) => Value::Null,
        Some(value) => parse_nutrition(value)?,
    };
    let ingredients = parse_line_list(input.get("ingredients"), "ingredients", "ingredient")?;
    let steps = parse_step_list(input.get("steps"))?;

    Ok(json!({
        "title": title,
        "yield": recipe_yield,
        "prep_time_minutes": prep_time_minutes,
        "cook_time_minutes": cook_time_minutes,
        "note": note,
        "main_photo": main_photo,
        "source": source,
        "nutrition": nutrition,
        "ingredients": ingredients,
        "steps": steps,
    }))
}

/// Nutrition, all of it v1 has: one figure and what that figure counts
/// (GLOSSARY.md, "Nutrition"). Kamosu never works the number out from the
/// Ingredient Lines or the Foods they name — #12 defers CIQUAL and the
/// compute button past v1 — so this is only ever what somebody typed, or
/// what a source page's own structured data stated (#70, ADR 0025).
///
/// The basis is the whole reason the figure is readable at all: 308 means
/// nothing until it says whether it counts a serving or 100 g, and the two
/// are not convertible without a weight the recipe does not carry. So it is
/// required alongside the number rather than defaulted to either — a default
/// would silently label half the library wrong.
fn parse_nutrition(value: &Value) -> Result<Value, OpError> {
    let object = value.as_object().ok_or_else(|| {
        OpError::bad_request("nutrition must be an object with calories and a basis")
    })?;
    let calories = object
        .get("calories")
        .and_then(Value::as_f64)
        .filter(|calories| *calories >= 0.0)
        .ok_or_else(|| OpError::bad_request("nutrition.calories must be a number, zero or more"))?;
    let basis = object
        .get("basis")
        .and_then(Value::as_str)
        .filter(|basis| matches!(*basis, "per_serving" | "per_100g"))
        .ok_or_else(|| {
            OpError::bad_request("nutrition.basis must be 'per_serving' or 'per_100g'")
        })?;
    Ok(json!({ "calories": calories, "basis": basis }))
}

/// A Yield: one amount and what it is an amount of — "4 servings", "24
/// cookies", "1.5 litres" are all the same field (GLOSSARY.md, "Yield"), kept
/// as written rather than parsed into a number and a Unit. The noun is a
/// distinct concept from Unit (grams, cups, spoons — a closed, convertible
/// list), so it is never called `unit` here.
pub(super) fn parse_yield(value: &Value) -> Result<Value, OpError> {
    let object = value
        .as_object()
        .ok_or_else(|| OpError::bad_request("yield must be an object with amount and noun"))?;
    let amount = object
        .get("amount")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("yield.amount is required"))?;
    let amount = required_text(amount, "yield.amount")?.to_string();
    let noun = object
        .get("noun")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("yield.noun is required"))?;
    let noun = required_text(noun, "yield.noun")?.to_string();
    Ok(json!({ "amount": amount, "noun": noun }))
}

/// **How much of a recipe somebody means** (#109): a Yield, or a multiplier —
/// a Yield whose noun is empty, `{"amount": "2", "noun": ""}` for twice the
/// recipe, which is how a recipe that never said what it makes is scaled
/// (`yield_scale`). A multiplier has to be a number above nothing, because a
/// multiplier Kamosu cannot read would scale nothing while claiming to.
pub(super) fn parse_wanted_yield(value: &Value) -> Result<Value, OpError> {
    if !is_multiplier(value) {
        return parse_yield(value);
    }
    let times = value
        .get("amount")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("yield.amount is required"))?;
    match units::parse_amount(times) {
        Some(number) if number > 0.0 => Ok(json!({ "amount": times.trim(), "noun": "" })),
        _ => Err(OpError::bad_request(
            "a Yield with no noun is a multiplier, and its amount must be a number above zero",
        )),
    }
}

/// A Yield a reader names for one read (#109): null for the recipe as
/// written, and otherwise held to the same rule as a cooking's.
pub(super) fn parse_named_yield(value: &Value) -> Result<Value, OpError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    parse_wanted_yield(value)
}

/// Prep Time and Cook Time are whole minutes; Cook Time includes resting,
/// proving, marinating and chilling — one field for however the dish spends
/// unattended time, documented at the Catalogue level for any Door reading
/// it (GLOSSARY.md, "Cook Time").
fn parse_optional_minutes(value: Option<&Value>, field: &str) -> Result<Option<i64>, OpError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_i64()
            .filter(|minutes| *minutes >= 0)
            .map(Some)
            .ok_or_else(|| {
                OpError::bad_request(format!(
                    "{field} must be a whole number of minutes, zero or more"
                ))
            }),
        Some(_) => Err(OpError::bad_request(format!(
            "{field} must be a whole number of minutes, zero or more"
        ))),
    }
}

/// A Source: free text with an optional link — "Mum's ring binder, p.40" is
/// as real an attribution as a URL (GLOSSARY.md, "Source").
fn parse_source(value: &Value) -> Result<Value, OpError> {
    let object = value.as_object().ok_or_else(|| {
        OpError::bad_request("source must be an object with text and an optional link")
    })?;
    let text = object
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("source.text is required"))?;
    let text = required_text(text, "source.text")?.to_string();
    let link = match object.get("link") {
        None | Some(Value::Null) => None,
        Some(Value::String(link)) => {
            let link = link.trim();
            if link.is_empty() { None } else { Some(link) }
        }
        Some(_) => return Err(OpError::bad_request("source.link must be a string or null")),
    };
    Ok(json!({ "text": text, "link": link }))
}

/// The Ingredient list: a flat, ordered sequence of Sections and Ingredient
/// Lines, each kept exactly as typed (ADR 0002) — never trimmed, never
/// rewritten. Absent entirely, this is an empty list rather than an error,
/// since a bare-name recipe with no ingredients yet is a real Recipe.
fn parse_line_list(value: Option<&Value>, field: &str, line_kind: &str) -> Result<Value, OpError> {
    let items = match value {
        None => return Ok(json!([])),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(OpError::bad_request(format!("{field} must be an array"))),
    };
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry must be an object")))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry needs a kind")))?;
        if kind != "section" && kind != line_kind {
            return Err(OpError::bad_request(format!(
                "{field} entries must be 'section' or '{line_kind}'"
            )));
        }
        // The written line itself: preserved verbatim, so only its raw string
        // reaches storage — `required_text`'s trim is used to reject a
        // whitespace-only line, never to alter what is kept.
        let text = object
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request(format!("each {field} entry needs text")))?;
        required_text(text, "text")?;
        parsed.push(json!({ "kind": kind, "text": text }));
    }
    Ok(Value::Array(parsed))
}

/// The Step list: the same flat Section-and-entry shape as the Ingredient
/// list, where a Step carries text and an optional photo — no timer or
/// temperature field to fill in (GLOSSARY.md, "Step"). The photo is carried
/// only as a reference to a Photograph already uploaded through
/// `upload_photograph`, unvalidated here the way any other pointer in this
/// codebase is (ADR 0017).
fn parse_step_list(value: Option<&Value>) -> Result<Value, OpError> {
    let items = match value {
        None => return Ok(json!([])),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(OpError::bad_request("steps must be an array")),
    };
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or_else(|| OpError::bad_request("each step entry must be an object"))?;
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request("each step entry needs a kind"))?;
        if kind != "section" && kind != "step" {
            return Err(OpError::bad_request(
                "step entries must be 'section' or 'step'",
            ));
        }
        let text = object
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| OpError::bad_request("each step entry needs text"))?;
        required_text(text, "text")?;
        let photo = match object.get("photo") {
            None | Some(Value::Null) => None,
            Some(Value::String(photo)) if !photo.is_empty() => Some(photo.as_str()),
            Some(Value::String(_)) => None,
            Some(_) => return Err(OpError::bad_request("step photo must be a string or null")),
        };
        parsed.push(json!({ "kind": kind, "text": text, "photo": photo }));
    }
    Ok(Value::Array(parsed))
}

/// Every Attempt on a Lineage, as the Thread and the recipe screen show them.
///
/// **Somebody else's As Cooked is stripped here.** An Attempt inherits its
/// recipe's visibility and names its cook (ADR 0005), so a Kitchen-mate's
/// cooking is legitimately on this list — but the words they cooked are the
/// private half, and an In Progress one is "visible to its cook alone"
/// (ADR 0010). The count, the date and the rating travel; the recipe they
/// wrote at their own stove does not. Stripped in the Core rather than left to
/// a screen to omit, because a second Door would omit it differently.
///
/// **`mine` and `person_name` say whose cooking it is** (#249), so a screen
/// can keep the reader's own apart and name the rest without a second read.
///
/// **`eaten_version_id` says which Version a cooking counts for** (#209), as
/// `thread_attempt_schema` declares it. Worked out here, before the strip,
/// because a Kitchen-mate's screen is never handed the As Cooked to work it
/// out from. An As Cooked nobody kept is on no Branch, so its fingerprint
/// never leaves this way.
fn attempts_for_lineage(
    conn: &Connection,
    lineage_id: &str,
    viewer_person_id: &str,
    visible_version_ids: &HashSet<String>,
) -> Result<Vec<Value>, OpError> {
    // Only the reader's company's cooking (#131, question 1): a recipe is seen
    // in Kitchens its cook may never have joined, and a cook's diary does not
    // follow the recipe there.
    let mut statement = conn
        .prepare(&format!(
            "SELECT {ATTEMPT_COLUMNS} FROM attempts \
              WHERE lineage_id = ?1 AND person_id IN ({}) ORDER BY created_at ASC",
            company_of("?2"),
        ))
        .map_err(|e| OpError::internal(format!("cannot read Attempts: {e}")))?;
    let rows: Vec<Value> = statement
        .query_map(params![lineage_id, viewer_person_id], attempt_row)
        .map_err(|e| OpError::internal(format!("cannot read Attempts: {e}")))?
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Attempts: {e}")))?;
    // Whose cooking each one is (#249), each cook asked for once.
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for attempt in &rows {
        let Some(person_id) = attempt["person_id"].as_str() else {
            continue;
        };
        if !names.contains_key(person_id) {
            names.insert(person_id.to_string(), person_name(conn, person_id)?);
        }
    }
    Ok(rows
        .into_iter()
        .filter_map(|mut attempt| {
            let on_the_thread = |id: &Value| {
                id.as_str()
                    .filter(|id| visible_version_ids.contains(*id))
                    .map(|id| json!(id))
            };
            // A cooking is on the Thread where the Version it counts for is.
            // Asked of what was eaten first, since a Promotion inside the
            // collapse window folds away the very Version the cooking started
            // from and leaves only the one it was kept as.
            let eaten = on_the_thread(&attempt["as_cooked"]["version_id"])
                .or_else(|| on_the_thread(&attempt["version_id"]))?;
            attempt["eaten_version_id"] = eaten;
            attempt["person_name"] = json!(names[attempt["person_id"].as_str()?]);
            let mine = attempt["person_id"] == json!(viewer_person_id);
            attempt["mine"] = json!(mine);
            if !mine {
                attempt["as_cooked"] = Value::Null;
            }
            Some(attempt)
        })
        .collect())
}

/// How a Lineage has been cooked, as a recipe shows it (#59): how many times,
/// when last, and **each Person's most recent rating with their name** — never
/// an average, a mean or a score of any kind (ADR 0015).
///
/// The refusal is structural rather than a rule somebody must remember. There
/// is no number here to average: a rating is one of three words, and what is
/// returned is one row per Person rather than a distribution. A superseded
/// verdict cannot permanently drag down a recipe that has since been fixed,
/// because only the newest one each Person gave is carried at all — the older
/// ones stay in their own diary and reach this not at all.
///
/// **Scoped by the reader's company, not by the Versions a Branch happens to
/// carry right now** (#131, question 1): the reader, everyone who cooks in a
/// Kitchen with them, and everyone who writes a Cookbook with them. Somebody
/// cooking the same dish whom this reader does not cook with is left out
/// rather than leaked just because it shares a Lineage id — a mother's note on
/// her son's recipe is for the family Kitchen, not his supper club.
///
/// Scoping by *Version* instead — the boundary `attempts_for_lineage` uses for
/// the Thread — reads correctly and is wrong here. A collapsing save repoints
/// `branch_versions` at a fresh Version (ADR 0005's append-only rule applies to
/// what a Branch carries, not to the row store), so every Attempt pinned to the
/// Version it replaced stops being reachable that way. The cook count would
/// then silently fall — the recipe would forget cookings because somebody
/// edited it — which is exactly the systematic wrongness ADR 0010 refuses.
/// Company survives a collapse; a Version id does not.
///
/// An unfinished Attempt counts toward both the count and the date (ADR 0010):
/// a cooking is real from the moment it starts, and the library must not be
/// systematically wrong because nobody filed paperwork.
///
/// **A rating says which Version it was given on** (#229), the quiet mark
/// ADR 0005 asks for: a "no" from before a recipe was fixed must not read as a
/// "no" to the recipe as it stands. That Version is the one that was eaten,
/// worked out as `attempts_for_lineage` does for the Thread, and it is set
/// against the head of the Branch being read: `newest` where it is that head,
/// `older` where it is an earlier state of this Branch, and `another` where
/// only a variation or a Translation holds it. The collapse described above is
/// why the rating is compared and never joined: one whose Version no Branch
/// carries any more is `older`, and it is marked and kept.
fn cooking_record(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    head_version_id: &str,
    person_id: &str,
) -> Result<Value, OpError> {
    let household = company_of("?2");

    let (count, last_cooked_at): (i64, Option<String>) = conn
        .query_row(
            &format!(
                "SELECT COUNT(*), MAX(created_at) FROM attempts \
                  WHERE lineage_id = ?1 AND person_id IN ({household})"
            ),
            params![lineage_id, person_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| OpError::internal(format!("cannot read the cooking record: {e}")))?;

    // Exactly one row per Person: the newest cooking of theirs that carries a
    // rating. A Person whose latest cooking went unrated keeps the last verdict
    // they actually gave — silence is not a retraction.
    //
    // "Newest" is by when the cooking happened, not by when the verdict was
    // typed: this answers *what they thought the last time they cooked it*.
    // Picking one id with `ORDER BY … LIMIT 1` rather than matching on
    // `MAX(created_at)` matters — timestamps are millisecond-precision, two
    // cookings can share one, and matching on the value would return that
    // Person twice and hand the screen a duplicate key. The id breaks the tie.
    let mut statement = conn
        .prepare(&format!(
            "SELECT attempts.person_id, people.name, attempts.rating, attempts.created_at, \
                    attempts.version_id, attempts.as_cooked_version_id \
               FROM attempts JOIN people ON people.id = attempts.person_id \
              WHERE attempts.lineage_id = ?1 AND attempts.person_id IN ({household}) \
                AND attempts.id = ( \
                    SELECT newer.id FROM attempts AS newer \
                     WHERE newer.person_id = attempts.person_id \
                       AND newer.lineage_id = ?1 \
                       AND newer.rating IS NOT NULL \
                     ORDER BY newer.created_at DESC, newer.id DESC LIMIT 1) \
              ORDER BY attempts.created_at DESC, people.name ASC"
        ))
        .map_err(|e| OpError::internal(format!("cannot read ratings: {e}")))?;
    let rows: Vec<(Value, String, Option<String>)> = statement
        .query_map(params![lineage_id, person_id], |row| {
            let rating = json!({
                "person_id": row.get::<_, String>(0)?,
                // The Person's name as it stands now, not as it stood when
                // they cooked: renaming yourself reaches your own history
                // (ADR 0015). An Attempt never leaves the instance, so the
                // Hand's travelling copy of a name has no part here.
                "name": row.get::<_, String>(1)?,
                "rating": row.get::<_, String>(2)?,
                "at": row.get::<_, String>(3)?,
            });
            Ok((rating, row.get(4)?, row.get(5)?))
        })
        .map_err(|e| OpError::internal(format!("cannot read ratings: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read ratings: {e}")))?;

    // Where the reader's Thread holds a Version: on the Branch being read, on
    // another Branch of the Lineage they may see, or on none.
    #[derive(PartialEq)]
    enum Held {
        Here,
        Elsewhere,
        Nowhere,
    }
    let mut holding = conn
        .prepare(&format!(
            "SELECT MAX(branches.id = ?3) \
               FROM branch_versions JOIN branches ON branches.id = branch_versions.branch_id \
              WHERE branch_versions.version_id = ?1 AND branches.lineage_id = ?4 AND {} \
             HAVING COUNT(*) > 0",
            visible_to("branches", "?2"),
        ))
        .map_err(|e| OpError::internal(format!("cannot read ratings: {e}")))?;
    let mut held = |version_id: &str| -> Result<Held, OpError> {
        let here: Option<bool> = holding
            .query_row(
                params![version_id, person_id, branch_id, lineage_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read ratings: {e}")))?;
        Ok(match here {
            Some(true) => Held::Here,
            Some(false) => Held::Elsewhere,
            None => Held::Nowhere,
        })
    };
    let mut ratings: Vec<Value> = Vec::with_capacity(rows.len());
    for (mut rating, started_from, as_cooked) in rows {
        // What was eaten (#209): the As Cooked once somebody kept it as a
        // Version, otherwise the Version the cooking started from. The same
        // rule, in the same order, as the Thread hangs a cooking by.
        let eaten = match as_cooked {
            Some(as_cooked) if held(&as_cooked)? != Held::Nowhere => as_cooked,
            _ => started_from,
        };
        let (given_on, version_id) = if eaten == head_version_id {
            ("newest", Value::Null)
        } else {
            match held(&eaten)? {
                // Named only here, where the older Version's page can open
                // it. An As Cooked nobody kept is on no Branch, so its
                // fingerprint never leaves this way.
                Held::Here => ("older", json!(eaten)),
                // A variation's or a Translation's: no older state of this
                // recipe, so the screen must not call it one.
                Held::Elsewhere => ("another", Value::Null),
                // Folded away by a collapsing save. Still a rating, and still
                // from before the recipe was what it is now.
                Held::Nowhere => ("older", Value::Null),
            }
        };
        rating["given_on"] = json!(given_on);
        rating["version_id"] = version_id;
        ratings.push(rating);
    }

    Ok(json!({
        "count": count,
        "last_cooked_at": last_cooked_at,
        "ratings": ratings,
    }))
}

/// **Whose cooking a reader sees** (#131, question 1), as SQL answering person
/// ids, `reader` being the placeholder holding theirs: the reader themselves,
/// everyone who cooks in a Kitchen with them, and everyone who writes a
/// Cookbook with them.
pub(super) fn company_of(reader: &str) -> String {
    format!(
        "SELECT {reader} \
         UNION SELECT theirs.person_id FROM kitchen_members AS mine \
                 JOIN kitchen_members AS theirs ON theirs.kitchen_id = mine.kitchen_id \
                WHERE mine.person_id = {reader} \
         UNION SELECT theirs.person_id FROM cookbook_authors AS mine \
                 JOIN cookbook_authors AS theirs ON theirs.cookbook_id = mine.cookbook_id \
                WHERE mine.person_id = {reader}"
    )
}

/// One Branch's Versions, oldest first, verified contiguous back to a first
/// Version with no parent. A gap anywhere in that chain — a parent that is
/// not the previous row's own Version — means the Bundle this Branch arrived
/// in was damaged, and `branch_point` needs to say so rather than guess.
fn ordered_chain(
    conn: &Connection,
    branch_id: &str,
    person_id: &str,
) -> Result<Vec<String>, OpError> {
    ensure_sees_branch(conn, branch_id, person_id)?;

    let mut statement = conn
        .prepare(
            "SELECT version_id, parent_version_id FROM branch_versions \
              WHERE branch_id = ?1 ORDER BY sequence ASC",
        )
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;
    let rows: Vec<(String, Option<String>)> = statement
        .query_map(params![branch_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?
        .collect::<Result<_, _>>()
        .map_err(|e| OpError::internal(format!("cannot read Branch chain: {e}")))?;

    let mut previous: Option<&str> = None;
    for (version_id, parent_version_id) in &rows {
        if parent_version_id.as_deref() != previous {
            return Err(OpError::internal(format!(
                "damaged Bundle: Branch {branch_id} does not chain back to a first Version"
            )));
        }
        previous = Some(version_id.as_str());
    }

    Ok(rows.into_iter().map(|(version_id, _)| version_id).collect())
}

/// The last Version two Branches share, each given as its id and its chain
/// oldest first: the Branch Point, as `branch_point` and `divergence` both
/// need it.
///
/// Two Copies of one recipe always share a first Version, so finding none is
/// damage. A Translation is the exception, by design: its chain starts fresh
/// (ADR 0006), so it shares no Version with the recipe it translates, nor
/// with another Translation of it (#251). That pair is a question with no
/// answer, and is refused as one instead of being called damaged.
fn last_shared_version<'a>(
    conn: &Connection,
    (branch_a_id, chain_a): (&str, &[String]),
    (branch_b_id, chain_b): (&str, &'a [String]),
) -> Result<&'a str, OpError> {
    let ancestors_a: HashSet<&str> = chain_a.iter().map(String::as_str).collect();
    if let Some(version_id) = chain_b
        .iter()
        .rev()
        .find(|version_id| ancestors_a.contains(version_id.as_str()))
    {
        return Ok(version_id);
    }

    let mut translations = Vec::new();
    for branch_id in [branch_a_id, branch_b_id] {
        if translation_source(conn, branch_id)?.is_some() {
            translations.push(branch_id);
        }
    }
    if translations.is_empty() {
        return Err(OpError::internal(format!(
            "damaged Bundle: {branch_a_id} and {branch_b_id} share no Version — \
             their chains never converge on the same first Version"
        )));
    }
    let who_translates = match translations[..] {
        [one] => format!("{one} is a Translation"),
        _ => format!("{branch_a_id} and {branch_b_id} are both Translations"),
    };
    Err(OpError::bad_request(format!(
        "{who_translates}. A Translation's chain starts fresh, so it shares no Version with \
         the recipe it translates, nor with another Translation: {branch_a_id} and \
         {branch_b_id} have no Branch Point and nothing to lay over each other. What a \
         Translation translates is held as translates_version_id on each of its Versions, \
         and as translation on its Branch in the answers of get_thread and get_recipe"
    )))
}

/// One Branch as a Divergence needs it: which Cookbook holds it and where its
/// head is.
pub(super) struct BranchHead {
    branch_id: String,
    pub(super) lineage_id: String,
    pub(super) cookbook_id: String,
    name: Option<String>,
    arrived: bool,
    hand_id: String,
    language: String,
    pub(super) head_version_id: String,
}

impl BranchHead {
    /// The Branch as one side of the switch: enough to name whose recipe you
    /// are standing in, and the Readings for the lines it actually has. A Reading
    /// never sits inside content (ADR 0021), so it is fetched and laid
    /// alongside — one slot per Ingredient Line, null wherever none is recorded.
    fn to_json(
        &self,
        conn: &Connection,
        content: &Value,
        person_id: &str,
        scale: f64,
    ) -> Result<Value, OpError> {
        let line_count = content["ingredients"].as_array().map(Vec::len).unwrap_or(0);
        let reader = Reader::of(conn, person_id)?;
        // Its Components, unfolded (#50, ADR 0008). Both sides of the switch
        // carry their own, because ADR 0014's whole shape is two WHOLE recipes
        // with a switch between them — a dough that unfolds in your pizza and
        // not in Marc's would make his the lesser of two recipes the switch
        // exists to hold as equals.
        let mut walk = Unfolding {
            open: vec![self.lineage_id.clone()],
            ..Unfolding::default()
        };
        unfold_components(
            conn,
            &Unfolds::ForReader {
                person_id,
                reader: &reader,
            },
            &self.head_version_id,
            scale,
            &mut walk,
        )?;
        Ok(json!({
            "branch_id": self.branch_id,
            "cookbook": cookbook_label(conn, &self.cookbook_id)?,
            "name": self.name,
            "mine": self.cookbook_id == cookbook_of_person(conn, person_id)?,
            "arrived": self.arrived,
            "hand_id": self.hand_id,
            "hand_name": conn
                .query_row(
                    &format!("SELECT {}", hand_name_sql("?1")),
                    params![self.hand_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(|e| OpError::internal(format!("cannot name a Hand: {e}")))?,
            "language": self.language,
            "head_version_id": self.head_version_id,
            "content": content.clone(),
            "readings": readings_for_version(conn, &self.head_version_id, line_count)?,
            // Both sides of the switch carry the measured line too, so a line
            // read while marking a Divergence says exactly what the same line
            // says on the recipe page.
            "measured": measured_for_version(
                conn,
                content,
                &self.head_version_id,
                &self.language,
                &reader,
                scale,
            )?,
            "components": walk.found,
        }))
    }
}

pub(super) fn branch_head(conn: &Connection, branch_id: &str) -> Result<BranchHead, OpError> {
    conn.query_row(
        "SELECT lineage_id, cookbook_id, name, arrived, hand_id, language, head_version_id \
           FROM branches WHERE id = ?1",
        params![branch_id],
        |row| {
            Ok(BranchHead {
                branch_id: branch_id.to_string(),
                lineage_id: row.get(0)?,
                cookbook_id: row.get(1)?,
                name: row.get(2)?,
                arrived: row.get(3)?,
                hand_id: row.get(4)?,
                language: row.get(5)?,
                head_version_id: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Branch: {e}")))?
    .ok_or_else(no_such_branch)
}

/// A Version's stored content in the shape the Catalogue declares, filling in
/// any field that did not exist when it was written.
///
/// A Version is immutable (ADR 0004), so a field added to the recipe later —
/// `nutrition` was the first, in #72 — is never written into a row already
/// stored. What is stored stays as it was written, and the missing field is
/// supplied here, on the way out.
///
/// **This moves no fingerprint, and that is the whole reason it is safe.**
/// Every field it fills in holds nothing, and a field holding nothing is no
/// part of a Version's id (ADR 0038) — so the recipe that goes out of this
/// function fingerprints to exactly the id the stored row sits under. Before
/// that rule existed the two disagreed, which is what #89 was.
///
/// Without this a recipe written before the field existed answers content the
/// Catalogue says must carry it, and the generated client — the whole point of
/// which is that the frontend and the Core cannot disagree about an
/// Operation's shape (AGENTS.md, "The interface") — is handed a shape its own
/// declaration forbids.
///
/// The empty value per field is the same one `parse_recipe_content` normalises
/// an absent input to, so a recipe read here and saved straight back reads
/// identically either way.
pub(super) fn content_as_declared(mut content: Value) -> Value {
    let Some(object) = content.as_object_mut() else {
        return content;
    };
    for (field, empty) in [
        ("title", json!("")),
        ("yield", Value::Null),
        ("prep_time_minutes", Value::Null),
        ("cook_time_minutes", Value::Null),
        ("note", Value::Null),
        ("main_photo", Value::Null),
        ("source", Value::Null),
        ("nutrition", Value::Null),
        ("ingredients", json!([])),
        ("steps", json!([])),
    ] {
        object.entry(field).or_insert(empty);
    }
    content
}

/// One Version's stored content, as written. A Version is content-addressed and
/// global, so this needs no Branch: two Branches that reached identical content
/// hold the very same row.
pub(super) fn version_content(conn: &Connection, version_id: &str) -> Result<Value, OpError> {
    let content: String = conn
        .query_row(
            "SELECT content FROM versions WHERE id = ?1",
            params![version_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?
        .ok_or_else(|| OpError::not_found("no such Version"))?;
    serde_json::from_str(&content)
        .map(content_as_declared)
        .map_err(|e| {
            OpError::internal(format!(
                "Version {version_id} holds unreadable content: {e}"
            ))
        })
}

/// The Language this save's text reads as, where that disagrees with the
/// Language the Branch already carries — the *offered when it disagrees* half
/// of the rule (ADR 0006). Never written anywhere: it rides out on the save's
/// answer, and only [`Core::set_recipe_language`] can act on it.
///
/// Silent about a Branch whose Language is Unknown, which is the whole of what
/// Unknown buys: a recipe honestly written in two Languages is never nagged,
/// because no detector can be right about it.
fn language_offer(branch_language: &str, content: &Value) -> Option<&'static str> {
    if crate::language::is_stated_unknown(branch_language) {
        return None;
    }
    crate::language::detect_content(content).filter(|detected| *detected != branch_language)
}

/// Whether some other Branch of this Lineage says it renders this exact
/// Version — the question a collapse has to ask before replacing it.
fn version_is_translated(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    version_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS ( \
            SELECT 1 FROM branch_versions \
              JOIN branches ON branches.id = branch_versions.branch_id \
             WHERE branch_versions.translates_version_id = ?1 \
               AND branches.lineage_id = ?2 \
               AND branch_versions.branch_id <> ?3 )",
        params![version_id, lineage_id, branch_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot look for Translations of this Version: {e}")))
}

/// Whether an agent's Job has saved a Version onto this Branch already
/// (#265): what tells its first save, which starts its Version, from a later
/// one that finds somebody else's Version at the head.
fn job_has_saved_onto(conn: &Connection, job_id: &str, branch_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS ( \
            SELECT 1 FROM branch_versions WHERE branch_id = ?1 AND job_id = ?2 )",
        params![branch_id, job_id],
        |row| row.get(0),
    )
    .map_err(|e| OpError::internal(format!("cannot look for the Job's Version: {e}")))
}

/// Whether a Branch's head Version is still the one being shaped: the one a
/// re-save collapses into, and the one a note-only edit writes onto (#165).
/// `head_is_callers_to_shape` is the part the head row already answers: that the
/// caller saved it within the collapse window, or that the agent's Job
/// asking is the one that wrote it (#265). The rest is why the window
/// closes early, spelled out where `save_onto_branch` collapses.
fn version_being_shaped(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    head_version_id: &str,
    head_is_callers_to_shape: bool,
) -> Result<bool, OpError> {
    Ok(
        head_is_callers_to_shape
            && !version_has_left(conn, lineage_id, branch_id, head_version_id)?,
    )
}

/// Where a Version last occurs on a Branch: its sequence there, and the source
/// Version it said it translates. The same content can recur on one Branch,
/// and the last time is the one the Thread shows nearest the head (#354).
pub(super) fn occurrence(
    conn: &Connection,
    branch_id: &str,
    version_id: &str,
) -> Result<(i64, Option<String>), OpError> {
    conn.query_row(
        "SELECT sequence, translates_version_id FROM branch_versions \
          WHERE branch_id = ?1 AND version_id = ?2 ORDER BY sequence DESC LIMIT 1",
        params![branch_id, version_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Version: {e}")))?
    .ok_or_else(|| OpError::bad_request("version_id is not a Version of this Branch"))
}

/// Whether somebody besides this Branch holds its head Version: a
/// Translation rendering it, another Branch naming it in its chain, or a
/// Bundle that carried it away (#328). Any one of them closes the collapse
/// window, since replacing the Version would pull it out from under them.
fn version_has_left(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    version_id: &str,
) -> Result<bool, OpError> {
    Ok(
        version_is_translated(conn, lineage_id, branch_id, version_id)?
            || version_is_held_by_another_branch(conn, lineage_id, branch_id, version_id)?
            || head_was_bundled(conn, branch_id)?,
    )
}

/// Whether a Bundle carried this Branch's head away, through the app or a
/// Share Link (#328). The receiver holds the chain up to it, and a sender
/// who replaced it in place could send them nothing of this recipe again:
/// the next Bundle's chain would disagree with theirs. Asked of the head row
/// itself, since the same Version can stand at two places in one chain, and
/// only the head is rewritten.
fn head_was_bundled(conn: &Connection, branch_id: &str) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS ( \
            SELECT 1 FROM branch_versions \
             WHERE branch_id = ?1 AND bundled = 1 \
               AND sequence = (SELECT MAX(sequence) FROM branch_versions WHERE branch_id = ?1) )",
        params![branch_id],
        |row| row.get(0),
    )
    .map_err(|e| {
        OpError::internal(format!(
            "cannot ask whether a Bundle carried this Version: {e}"
        ))
    })
}

/// Whether another Branch of this Lineage names this exact Version in its own
/// chain — the other question a collapse has to ask before replacing it
/// (issue #82).
///
/// A Copy carries the source's chain across verbatim, so from that moment the
/// Copy holds a row of its own saying *this Version was my sequence n*.
/// Rewriting the source's row in place would leave that Version named nowhere
/// on the source, the two chains would stop intersecting, and the Branch Point
/// walk would fall off the end — permanently, and without a symptom, since
/// each chain stays internally contiguous and `get_thread` goes on answering
/// with two histories that appear never to have shared anything.
///
/// Scoped to the Lineage, exactly as [`version_is_translated`] is, and the
/// scoping is load-bearing rather than tidy. A Version is content-addressed
/// and **global**: two people who each start a recipe called *Soupe* mint the
/// very same `version_id` in two unrelated Lineages. Asked without the scope,
/// this would answer yes for that pair and quietly close the collapse window
/// on both of them, with no Copy anywhere — a rapid re-save littering a
/// history because somebody else, elsewhere, chose the same title. Every
/// cross-Branch reference that the Branch Point actually walks is within one
/// Lineage: a Copy sets the source's `lineage_id` and so does a Translation.
fn version_is_held_by_another_branch(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    version_id: &str,
) -> Result<bool, OpError> {
    conn.query_row(
        "SELECT EXISTS ( \
            SELECT 1 FROM branch_versions \
              JOIN branches ON branches.id = branch_versions.branch_id \
             WHERE branch_versions.version_id = ?1 \
               AND branches.lineage_id = ?2 \
               AND branch_versions.branch_id <> ?3 )",
        params![version_id, lineage_id, branch_id],
        |row| row.get(0),
    )
    .map_err(|e| {
        OpError::internal(format!(
            "cannot look for other Branches holding this Version: {e}"
        ))
    })
}

/// Start a **Copy** (GLOSSARY.md, "Copy"): a new Branch of the same Lineage in
/// `cookbook_id`, under that Cookbook's own Hand, carrying the whole chain of
/// `branch_id` behind it verbatim — same Versions, same Hands, same names and
/// *what changed* lines, nothing truncated (ADR 0018). The caller appends the
/// change that made it; the source Branch is never touched.
///
/// Where the Cookbook already holds a Branch of its own of this recipe in this
/// Language, named or not (#136), the Copy is named after the Cookbook it came
/// from, so the two can be told apart on the switch (#131, question 6).
pub(super) fn start_copy(
    conn: &Connection,
    branch_id: &str,
    lineage_id: &str,
    cookbook_id: &str,
    started_by: &str,
    language: &str,
    head_version_id: &str,
) -> Result<String, OpError> {
    let new_branch_id = format!("b_{}", hex::encode(random_bytes(8)));
    let hand_id = cookbook_hand(conn, cookbook_id)?;
    let name = name_on_arrival(
        conn,
        cookbook_id,
        lineage_id,
        language,
        &whose_branch(conn, branch_id)?,
    )?;
    conn.execute(
        // A guessed Language is the Copy's guess too (#321). A save that
        // settles it says so on the Copy after.
        "INSERT INTO branches (id, lineage_id, cookbook_id, hand_id, language, head_version_id, name, started_by, language_guessed) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, (SELECT language_guessed FROM branches WHERE id = ?9))",
        params![
            new_branch_id,
            lineage_id,
            cookbook_id,
            hand_id,
            language,
            head_version_id,
            name,
            started_by,
            branch_id
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;
    carry_chain(conn, branch_id, &new_branch_id)?;
    Ok(new_branch_id)
}

/// Mint a brand-new Lineage, a Branch of it in `cookbook_id`, and its first
/// Version — the one sequence `create_recipe` and a freshly-seen Import
/// candidate both start from (ADR 0004): the Cookbook's own Hand on the
/// Branch, the writing Person's Hand on this first Version, and the writer
/// as the one who started it. The lines in `leave_unread` are not read: an
/// import gives them their Readings itself once its file has landed (#413).
#[allow(clippy::too_many_arguments)]
pub(super) fn insert_new_lineage_and_branch(
    conn: &Connection,
    lineage_id: &str,
    branch_id: &str,
    cookbook_id: &str,
    cookbook_hand_id: &str,
    language: &NewBranchLanguage,
    version_id: &str,
    content_text: &str,
    writer: &Caller,
    leave_unread: &[usize],
) -> Result<Vec<CreatedFood>, OpError> {
    conn.execute(
        "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
        params![version_id, content_text],
    )
    .map_err(|e| OpError::internal(format!("cannot record Version: {e}")))?;
    conn.execute("INSERT INTO lineages (id) VALUES (?1)", params![lineage_id])
        .map_err(|e| OpError::internal(format!("cannot mint Lineage: {e}")))?;
    conn.execute(
        "INSERT INTO branches (id, lineage_id, cookbook_id, hand_id, language, head_version_id, started_by, language_guessed) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            branch_id,
            lineage_id,
            cookbook_id,
            cookbook_hand_id,
            language.language,
            version_id,
            writer.person_id,
            language.guessed
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot start Branch: {e}")))?;
    // A Job Key's create writes its Job on the first Version, as any save of
    // its does, so the Job's later saves fold into this one (#265, #269).
    conn.execute(
        "INSERT INTO branch_versions \
         (branch_id, sequence, version_id, parent_version_id, hand_id, access_key_id, language, job_id) \
         VALUES (?1, 1, ?2, NULL, ?3, ?4, ?5, ?6)",
        params![
            branch_id,
            version_id,
            writer.person_id,
            writer.access_key_id,
            language.language,
            writer.job_id
        ],
    )
    .map_err(|e| OpError::internal(format!("cannot record first Version: {e}")))?;
    // Every line of a first Version written *here* is Kamosu's to read (#71).
    // An imported recipe is an ordinary recipe (ADR 0025), so a Crouton file
    // and a typed recipe arrive read the same way.
    //
    // **A Bundle is not** (#67): it carries its own Readings, and they are
    // carried, never recomputed (ADR 0003, ADR 0021) — so a received Branch
    // never comes through here. Only a damaged Bundle's words do, as a new
    // recipe of the receiver's own, and those are read like any other.
    //
    // A recipe whose own content will not parse is a recipe left unread, never
    // a failed write. The Foods reading it created are the caller's to name
    // (#358).
    let Ok(content) = serde_json::from_str::<Value>(content_text) else {
        return Ok(Vec::new());
    };
    let lines: Option<HashSet<usize>> = (!leave_unread.is_empty()).then(|| {
        (0..content["ingredients"].as_array().map_or(0, Vec::len))
            .filter(|line| !leave_unread.contains(line))
            .collect()
    });
    Ok(read_unread_lines(
        conn,
        version_id,
        &language.language,
        &content,
        lines.as_ref(),
        None,
    )
    .foods_created)
}

/// The title of one Version, which is what a recipe is called right now.
pub(super) fn branch_title(conn: &Connection, version_id: &str) -> Result<String, OpError> {
    Ok(version_content(conn, version_id)?["title"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

#[cfg(test)]
mod tests {
    use crate::fingerprint::fingerprint_content;
    use serde_json::{Value, json};

    /// `edit_recipe` merges only the fields `CONTENT_FIELDS` names (#164), so
    /// a field added to `parse_recipe_content` and not to that list would be
    /// accepted by the edit and then silently dropped.
    #[test]
    fn an_edit_can_change_every_field_a_recipe_holds() {
        let parsed = super::parse_recipe_content(&json!({ "title": "Soupe" })).unwrap();
        let mut held: Vec<&str> = parsed
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut editable = super::CONTENT_FIELDS.to_vec();
        held.sort_unstable();
        editable.sort_unstable();
        assert_eq!(held, editable);
    }

    /// A Step is kept by its words: a photograph put on it, as promoting a
    /// cooking's photograph does, leaves its corrected lines in place, and a
    /// change to its words does not (#439).
    #[test]
    fn a_step_is_kept_by_its_words_and_not_its_photograph() {
        let before = json!({ "steps": [
            { "kind": "step", "text": "Fry the chicken.", "photo": null },
            { "kind": "step", "text": "Cook the glaze.", "photo": null },
        ] });
        let after = json!({ "steps": [
            { "kind": "step", "text": "Fry the chicken.", "photo": "p_1" },
            { "kind": "step", "text": "Cook the glaze well.", "photo": null },
        ] });
        assert_eq!(super::kept_steps(&before, &after), [Some(0), None]);
    }

    /// **The gate on adding a field to a recipe** (ADR 0038, AGENTS.md).
    ///
    /// A recipe built from nothing but a title must fingerprint as though
    /// nothing but the title were written, however many slots
    /// `parse_recipe_content` fills in around it — because every one of them
    /// holds nothing, and a field holding nothing is no part of a Version's
    /// id. That is what lets a field be added to a recipe without moving the
    /// id of every recipe already saved, which is what #89 was.
    ///
    /// **If this test fails, you gave a new field a non-empty default.** Doing
    /// that re-fingerprints the entire library and needs a migration rewriting
    /// `branches`, `branch_versions`, `readings`, `attempts` and
    /// `meaning_vectors` — migration 29 is the worked example, and the only
    /// time Kamosu has done it. Default the field to `null` or `[]` instead.
    #[test]
    fn a_field_added_to_a_recipe_moves_no_existing_id() {
        let parsed = super::parse_recipe_content(&json!({ "title": "Ratatouille" }))
            .expect("a title alone is a complete recipe");
        assert_eq!(
            fingerprint_content(&parsed),
            fingerprint_content(&json!({ "title": "Ratatouille" })),
            "every field `parse_recipe_content` fills in must hold nothing"
        );
    }

    /// The other half of the same rule: what a Door serves is filled out to
    /// the shape the Catalogue declares, and that must fingerprint to the id
    /// the stored row already sits under. Where these two disagreed, reading a
    /// recipe and hashing what came back answered an id that named nothing —
    /// the measurement #89 opened with.
    #[test]
    fn filling_a_recipe_out_to_its_declared_shape_moves_no_id() {
        // A Version as it was stored before `nutrition` existed (#72).
        let stored = json!({
            "title": "Coq au Vin",
            "note": "Best served hot.",
            "ingredients": [{ "text": "1 poulet" }],
            "steps": [{ "text": "Cuire" }],
        });
        let served = super::content_as_declared(stored.clone());
        assert_eq!(
            served["nutrition"],
            Value::Null,
            "the field must be filled in"
        );
        assert_eq!(
            fingerprint_content(&served),
            fingerprint_content(&stored),
            "filling in the declared shape must move no fingerprint"
        );
    }
}
