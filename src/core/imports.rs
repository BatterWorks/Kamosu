//! Imports: a Crouton export, another app's export (#412), staged uploads,
//! and the ledger that finds every Import again (#108).

use super::*;
use std::collections::VecDeque;

use crate::other_apps::{self, LeftOut, Unread};

/// How long one import from another app's export may spend downloading the
/// photos its file holds only as web addresses, all of them together (#414).
/// Such an import runs on the lane strangers' addresses share (ADR 0032), so
/// this is how long one file can hold that lane on downloads. Ten minutes
/// takes some six hundred photos at a second each; a photo after that lands
/// its recipe without it, and the Report lists its address.
pub(super) const PHOTO_DOWNLOAD_TIME: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// One import's downloads of the photos its file holds only as addresses:
/// the time it has left for them, and the Cookbook and app whose earlier
/// imports may already have brought a photo in.
struct Downloads {
    left: std::cell::Cell<std::time::Duration>,
    cookbook_id: String,
    app: &'static str,
}

impl Downloads {
    /// The photo at `address`, or nothing where it could not be fetched, was
    /// refused by the guarded fetch (ADR 0033), or the time had run out. Only
    /// the time spent fetching counts against what is left.
    fn fetch(&self, address: &str) -> Option<Vec<u8>> {
        let left = self.left.get();
        if left.is_zero() {
            return None;
        }
        let started = std::time::Instant::now();
        let fetched = crate::web_import::fetch_photo_within(address, left).ok();
        self.left.set(left.saturating_sub(started.elapsed()));
        fetched
    }
}

impl Core {
    /// Bring in a Crouton library (#69): every `.crumb` in the export read one
    /// at a time, its first picture remade into a Photograph at the door (#45),
    /// and landed through the ledger every importer shares, so a second run
    /// matches what the first made (ADR 0025). What the reader left out on
    /// purpose — the site's favicon, pictures past the first — is named in the
    /// Report rather than stored.
    pub fn import_crouton(
        &self,
        caller: &Caller,
        export: crouton::Export,
        into: Option<&str>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        self.land_export(caller, export.into_recipes(), into, progress)
    }

    /// Bring in an export from another recipe app (#412): the first reader
    /// that knows the file reads it, and its recipes land as Crouton's do. A
    /// file no reader knows, or one a reader knows but cannot read, lands
    /// nothing, and the answer is `refused` alone. A Mealie backup of several
    /// households lands nothing until one is named, and the answer is
    /// `which_household` alone (#418).
    pub fn import_from_another_app(
        &self,
        caller: &Caller,
        handed: &other_apps::Handed,
        into: Option<&str>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        match other_apps::read(handed, other_apps::READERS) {
            Ok(export) => self.land_export(caller, export, into, progress),
            Err(refusal) => Ok(refusal.answer()),
        }
    }

    /// Land what an app's reader hands over: each recipe's photos stored as
    /// Photographs first, its lines that point at another recipe made
    /// Components once the whole file has landed, and what the whole file
    /// left out added to the Report.
    fn land_export(
        &self,
        caller: &Caller,
        export: other_apps::Export,
        into: Option<&str>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let downloads = Downloads {
            left: std::cell::Cell::new(self.photo_download_time()),
            cookbook_id: self.db().with_conn(|conn| lands_in(conn, caller, into))?,
            app: export.app,
        };
        let mut left_out_of_file = export.left_out;
        let mut pointing = VecDeque::new();
        let mut cooked = Vec::new();
        let mut photos_not_in_export = 0;
        let incoming = export.recipes.map(|recipe| match recipe {
            Ok(recipe) => {
                let foreign_id = recipe.foreign_id.clone();
                let arriving = self.arriving(recipe, &downloads);
                if !arriving.components.is_empty() {
                    pointing.push_back((foreign_id.clone(), arriving.components));
                }
                if !arriving.cookings.is_empty() {
                    cooked.push((foreign_id, arriving.cookings));
                }
                photos_not_in_export += usize::from(arriving.photo_not_in_export);
                arriving.incoming
            }
            Err(unread) => Incoming {
                candidate: Err(unread),
                left_out: Vec::new(),
                tags: Vec::new(),
                component_lines: Vec::new(),
            },
        });
        let mut report =
            self.import_each(caller, export.app, into, export.total, incoming, progress)?;
        photos_not_in_export += self.land_components(caller, &mut report, pointing, &downloads)?;
        self.land_cookings(caller, &report, cooked)?;
        if photos_not_in_export > 0 {
            match left_out_of_file.iter_mut().find_map(|left| match left {
                other_apps::FileLeftOut::PhotosNotInExport(count) => Some(count),
                _ => None,
            }) {
                Some(count) => *count += photos_not_in_export,
                None => left_out_of_file.push(other_apps::FileLeftOut::PhotosNotInExport(
                    photos_not_in_export,
                )),
            }
        }
        report["left_out_of_file"] = json!(
            left_out_of_file
                .iter()
                .map(other_apps::FileLeftOut::row)
                .collect::<Vec<_>>()
        );
        Ok(report)
    }

    /// One recipe a reader handed over, as the landing takes it: its Main
    /// Photo and its Steps' photos stored as Photographs, a Main Photo the
    /// file holds only as an address downloaded first, and the lines that
    /// point at another recipe named so they are left unread, with what they
    /// point at.
    fn arriving(&self, recipe: other_apps::Recipe, downloads: &Downloads) -> Arriving {
        let mut left_out = recipe.left_out;
        // A picture that cannot be remade never fails its recipe (#45): the
        // recipe lands without it, and the Report says so.
        let mut unreadable = 0;
        let mut stored = |bytes: &[u8]| {
            let stored = self
                .store_photograph(bytes)
                .ok()
                .and_then(|stored| stored["photograph_id"].as_str().map(str::to_string));
            if stored.is_none() {
                unreadable += 1;
            }
            stored
        };
        let photo_not_in_export = matches!(recipe.photo, Some(other_apps::Photo::NotInExport));
        let main_photo = match recipe.photo {
            Some(other_apps::Photo::Bytes(bytes)) => stored(&bytes),
            // A download that fails, or is no picture, never fails its
            // recipe either, and the Report names its address (#414).
            // A recipe an earlier import brought in keeps the photo it has,
            // and nothing is downloaded again: a site gone since would
            // otherwise offer a change the cook never made.
            Some(other_apps::Photo::Address(address)) => {
                let stored = self
                    .photo_already_landed(downloads, &recipe.foreign_id)
                    .or_else(|| {
                        let bytes = downloads.fetch(&address)?;
                        let stored = self.store_photograph(&bytes).ok()?;
                        stored["photograph_id"].as_str().map(str::to_string)
                    });
                if stored.is_none() {
                    left_out.push(LeftOut::PhotoNotDownloaded(address));
                }
                stored
            }
            Some(other_apps::Photo::NotInExport) | None => None,
        };
        let mut candidate = recipe.candidate;
        for other_apps::StepPhoto { step, bytes } in recipe.step_photos {
            if let Some(photograph_id) = stored(&bytes)
                && let Some(step) = candidate["steps"].get_mut(step)
            {
                step["photo"] = json!(photograph_id);
            }
        }
        let cookings = recipe
            .cookings
            .into_iter()
            .map(|cooking| StoredCooking {
                photograph: cooking.photo.as_deref().and_then(&mut stored),
                foreign_id: cooking.foreign_id,
                cooked_at: cooking.cooked_at,
                note: cooking.note,
            })
            .collect();
        if unreadable > 0 {
            match left_out.iter_mut().find_map(|left| match left {
                LeftOut::UnreadablePhotos(count) => Some(count),
                _ => None,
            }) {
                Some(count) => *count += unreadable,
                None => left_out.push(LeftOut::UnreadablePhotos(unreadable)),
            }
        }
        candidate["foreign_id"] = json!(recipe.foreign_id);
        candidate["main_photo"] = json!(main_photo);
        Arriving {
            cookings,
            incoming: Incoming {
                candidate: Ok(candidate),
                left_out,
                tags: recipe.tags,
                component_lines: recipe.components.iter().map(|c| c.line).collect(),
            },
            components: recipe.components,
            photo_not_in_export,
        }
    }

    /// Make each line that pointed at another of the app's recipes a
    /// Component (#413), once every recipe of the file has landed, so the
    /// recipe it points at is there whichever came first in the file.
    ///
    /// The line's Reading names that recipe's Lineage, found through the
    /// Import's ledger, and counts as set by hand: an Operator's re-read of
    /// every line keeps it, and so does a later import, which never replaces a
    /// Reading somebody set. A recipe pointed at that no import has brought in
    /// lands first, from the copy the app keeps in the line, and once, since
    /// it is then in the ledger. On a recipe that was offered rather than
    /// landed, the Component goes on the offered Version, ready if the cook
    /// takes it.
    ///
    /// This is a link the cook made in the other app, never a guess, so ADR
    /// 0025's "no Component is guessed at" stands.
    ///
    /// The answer is how many recipes landed from a copy whose photos the
    /// file does not carry, for the Report to count.
    fn land_components(
        &self,
        caller: &Caller,
        report: &mut Value,
        mut pointing: VecDeque<(String, Vec<other_apps::Component>)>,
        downloads: &Downloads,
    ) -> Result<usize, OpError> {
        if pointing.is_empty() {
            return Ok(0);
        }
        let mut photos_not_in_export = 0;
        // A copy that could not land is tried once, and listed once.
        let mut failed: HashSet<String> = HashSet::new();
        let import_id = report["import_id"].as_str().unwrap_or_default().to_string();
        let cookbook_id = report["cookbook_id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let cookbook_hand_id = self
            .db()
            .with_conn(|conn| cookbook_hand(conn, &cookbook_id))?;
        while let Some((foreign_id, components)) = pointing.pop_front() {
            let Some(lines_on) = LinesOn::of(report, &foreign_id) else {
                continue;
            };
            for component in components {
                let mut lineage_id = self
                    .db()
                    .with_conn(|conn| ledger_lineage(conn, &import_id, &component.foreign_id))?;
                if lineage_id.is_none()
                    && !failed.contains(&component.foreign_id)
                    && let Some(copy) = component.copy
                {
                    let arriving = self.arriving(*copy, downloads);
                    let copy_id = component.foreign_id.clone();
                    let landed = self.land_one(
                        caller,
                        report,
                        (&cookbook_id, &cookbook_hand_id, &import_id),
                        &copy_id,
                        arriving.incoming,
                    )?;
                    match landed {
                        Some(_) => {
                            photos_not_in_export += usize::from(arriving.photo_not_in_export);
                            if !arriving.components.is_empty() {
                                pointing.push_back((copy_id, arriving.components));
                            }
                        }
                        None => {
                            failed.insert(copy_id);
                        }
                    }
                    lineage_id = landed;
                }
                self.db().with_conn(|conn| {
                    point_line(
                        conn,
                        &lines_on,
                        component.line,
                        lineage_id.as_deref(),
                        component.times,
                    )
                })?;
            }
        }
        Ok(photos_not_in_export)
    }

    /// Land one recipe through the ledger outside the run of a file, and say
    /// so in the Report: the Lineage it is, or nothing where it could not land.
    fn land_one(
        &self,
        caller: &Caller,
        report: &mut Value,
        (cookbook_id, cookbook_hand_id, import_id): (&str, &str, &str),
        foreign_id: &str,
        incoming: Incoming,
    ) -> Result<Option<String>, OpError> {
        let Ok(candidate) = incoming.candidate else {
            return Ok(None);
        };
        let shape = LandedShape::of(&candidate);
        let landed = self.import_one(
            caller,
            cookbook_id,
            cookbook_hand_id,
            import_id,
            foreign_id,
            &candidate,
            &incoming.tags,
            &incoming.component_lines,
        );
        let (lineage_id, branch_id, title) = match landed {
            Ok(ImportOutcome::Landed {
                lineage_id,
                branch_id,
                title,
                status,
                ..
            }) => {
                push_row(
                    report,
                    "arrived",
                    arrived_row(foreign_id, status, &lineage_id, &branch_id, &title, &shape),
                );
                (lineage_id, branch_id, title)
            }
            Ok(ImportOutcome::Offered {
                lineage_id,
                branch_id,
                title,
                candidate_version_id,
            }) => {
                push_row(
                    report,
                    "offered",
                    offered_row(
                        foreign_id,
                        &lineage_id,
                        &branch_id,
                        &title,
                        &candidate_version_id,
                    ),
                );
                (lineage_id, branch_id, title)
            }
            Err(err) => {
                push_row(
                    report,
                    "unreadable",
                    json!({ "foreign_id": foreign_id, "reason": err.to_sentence() }),
                );
                return Ok(None);
            }
        };
        for left in &incoming.left_out {
            push_row(report, "left_out", left.row(foreign_id, &branch_id, &title));
        }
        Ok(Some(lineage_id))
    }

    /// Write each cooking the app recorded as a finished Attempt of the
    /// importing Person (#418, ADR 0053), once every recipe of the file has
    /// landed: on the Version that arrived, or the one offered where the
    /// recipe was offered, dated as the app dated it. It is written whole and
    /// finished, never started, so a cook's own Attempt In Progress on the
    /// recipe is left as it is. A cooking an earlier import of this Import
    /// brought in is never written twice, even where its Attempt was deleted
    /// since. A recipe that landed nothing has no cookings.
    fn land_cookings(
        &self,
        caller: &Caller,
        report: &Value,
        cooked: Vec<(String, Vec<StoredCooking>)>,
    ) -> Result<(), OpError> {
        let import_id = report["import_id"].as_str().unwrap_or_default();
        for (foreign_id, cookings) in cooked {
            let Some(lines_on) = LinesOn::of(report, &foreign_id) else {
                continue;
            };
            self.db().with_conn(|conn| {
                let failed = |e| OpError::internal(format!("cannot write the cooking: {e}"));
                let (lineage_id, head): (String, String) = conn
                    .query_row(
                        "SELECT lineage_id, head_version_id FROM branches WHERE id = ?1",
                        params![lines_on.branch_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(failed)?;
                let version_id = lines_on.offered.clone().unwrap_or(head);
                let tx = conn.unchecked_transaction().map_err(failed)?;
                for cooking in &cookings {
                    let known: bool = tx
                        .query_row(
                            "SELECT 1 FROM imported_attempts WHERE import_id = ?1 AND foreign_id = ?2",
                            params![import_id, cooking.foreign_id],
                            |_| Ok(()),
                        )
                        .optional()
                        .map_err(failed)?
                        .is_some();
                    if known {
                        continue;
                    }
                    // A time the app wrote that SQLite cannot read is the
                    // moment of the import, as a cooking with no date is.
                    let moment = written_moment(&tx, Some(&cooking.cooked_at))
                        .or_else(|_| written_moment(&tx, None))?;
                    let attempt_id = super::cooking::record_finished_attempt(
                        &tx,
                        super::cooking::FinishedAttempt {
                            lineage_id: &lineage_id,
                            person_id: &caller.person_id,
                            version_id: &version_id,
                            note: &cooking.note,
                            photographs: cooking.photograph.as_slice(),
                            finished_at: &moment,
                        },
                    )?;
                    tx.execute(
                        "INSERT INTO imported_attempts (import_id, foreign_id, branch_id, attempt_id) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![import_id, cooking.foreign_id, lines_on.branch_id, attempt_id],
                    )
                    .map_err(failed)?;
                }
                tx.commit().map_err(failed)
            })?;
        }
        Ok(())
    }

    /// How long one import may spend downloading photos (#414).
    fn photo_download_time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.photo_download_millis.load(Ordering::Relaxed))
    }

    /// Give each import `time` to download photos in place of
    /// `imports::PHOTO_DOWNLOAD_TIME`. Compiled only under `test-jobs`: a test
    /// that runs the time out cannot wait ten minutes.
    #[cfg(feature = "test-jobs")]
    pub fn download_photos_for(&self, time: std::time::Duration) {
        self.photo_download_millis
            .store(time.as_millis() as u64, Ordering::Relaxed);
    }

    /// The Main Photo the recipe an earlier import from the same app landed
    /// as `foreign_id` holds now, where the ledger knows it and it holds one.
    fn photo_already_landed(&self, downloads: &Downloads, foreign_id: &str) -> Option<String> {
        let content: String = self
            .db()
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT versions.content FROM imports \
                       JOIN import_ledger ON import_ledger.import_id = imports.id \
                       JOIN branches ON branches.id = import_ledger.branch_id \
                       JOIN versions ON versions.id = branches.head_version_id \
                      WHERE imports.cookbook_id = ?1 AND imports.source_kind = ?2 \
                        AND import_ledger.foreign_id = ?3",
                    params![downloads.cookbook_id, downloads.app, foreign_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Import ledger: {e}")))
            })
            .ok()??;
        let content: Value = serde_json::from_str(&content).ok()?;
        content["main_photo"].as_str().map(str::to_string)
    }

    /// What this Person has brought in from outside, and what happened each
    /// time (#108).
    ///
    /// The answer is grouped by **source kind** rather than by Import row, and
    /// the distinction is the whole design. An Import is one durable channel
    /// per Kitchen per source, reused by every run after the first (ADR 0025),
    /// so three Imports can sit behind forty-eight arrivals. `forget_import`
    /// deletes the channel; the Jobs it ran never vanish, and each still holds
    /// the Report that `get_job` serves. Keying the answer on the Import row
    /// would therefore hide every past Report the moment a ledger was
    /// forgotten — which is the exact unreachability this Operation exists to
    /// end. So a source is listed while it has *either* a ledger or an
    /// arrival, and `import_id` is null once the ledger is gone.
    ///
    /// **Scoped to the caller's own Cookbook, and that is what an Import is.**
    /// Every importer lands its recipes in the Cookbook of whoever asked
    /// (`import_each`, and the Bundle path alike; ADR 0041), which GLOSSARY.md
    /// states as the definition rather than as an implementation detail.
    /// `imports` is unique per `(cookbook_id, source_kind)`, so one Cookbook
    /// has one channel of each kind, however many Co-authors run it.
    ///
    /// Arrivals are the caller's own Jobs, matching `list_jobs` and the reader
    /// check `get_job` makes: listing a Co-author's Job would offer a link that
    /// then refuses. `remembered` is the Cookbook's, because a ledger belongs
    /// to the Cookbook and not to whoever happened to run the importer.
    pub fn list_imports(&self, person_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            // The ledgers this Person's Cookbook holds, newest channel first.
            let ledgers: Vec<(String, String, String, i64)> = {
                let mut statement = conn
                    .prepare(
                        "SELECT imports.id, imports.source_kind, imports.created_at,
                                (SELECT COUNT(*) FROM import_ledger
                                  WHERE import_ledger.import_id = imports.id)
                           FROM imports
                           JOIN cookbook_authors ON cookbook_authors.cookbook_id = imports.cookbook_id
                          WHERE cookbook_authors.person_id = ?1
                          ORDER BY imports.created_at DESC",
                    )
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?;
                statement
                    .query_map(params![person_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                    })
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list Imports: {e}")))?
            };

            // Every arrival this Person asked for, newest first. Queried on the
            // importer names rather than through `jobs_of`, whose hundred-row
            // cap is about watching recent work and would quietly drop the old
            // arrivals this screen exists to reach.
            //
            // `input` is read with `json_extract` and never whole. A Job's input
            // is stored verbatim (`jobs::record`), and `import_crouton` accepts
            // the export itself as base64 `data` for a Door that can send only
            // JSON — so a row's input can be a 114 MB library rendered as 152 MB
            // of text. All this needs from it is one short string, and only for
            // `import`, the one importer that does not name its source in its
            // own definition.
            //
            // An agent's Job is listed where it is its own Import: one reading
            // an Original, or a recipe read again.
            let arrivals: Vec<ArrivalRow> = {
                let mut statement = conn
                    .prepare(&format!(
                        "SELECT id, operation, json_extract(input, '$.source_kind'),
                                status, result, created_at
                           FROM jobs
                          WHERE person_id = ?1
                            AND (operation IN ('import', 'import_crouton',
                                               'import_from_another_app',
                                               'import_bundle', 'import_web_link')
                                 OR (operation = '{}' AND {}))
                          ORDER BY created_at DESC",
                        jobs::AGENT_OPERATION,
                        jobs::ITS_OWN_IMPORT,
                    ))
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?;
                statement
                    .query_map(params![person_id], |row| {
                        let result: Option<String> = row.get(4)?;
                        Ok(ArrivalRow {
                            job_id: row.get(0)?,
                            operation: row.get(1)?,
                            declared: row.get(2)?,
                            status: row.get(3)?,
                            result: result.and_then(|text| serde_json::from_str(&text).ok()),
                            created_at: row.get(5)?,
                        })
                    })
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?
                    .collect::<Result<_, _>>()
                    .map_err(|e| OpError::internal(format!("cannot list arrivals: {e}")))?
            };

            let mut grouped: Vec<(String, Vec<Value>)> = Vec::new();
            for row in arrivals {
                // A page read from its own text and handed back to be checked
                // brought nothing in (#301): saved, it is a recipe written by
                // hand, and unsaved it is nothing.
                if row.result.as_ref().is_some_and(|result| result.get("to_check").is_some()) {
                    continue;
                }
                let Some(source_kind) = arrival_source_kind(
                    &row.operation,
                    row.declared.as_deref(),
                    row.result.as_ref(),
                ) else {
                    continue;
                };
                // The agent's Job is found from its arrival, which named it
                // in its Report, and read as it stands now: the Report was
                // written when the Import ended and the Job has moved since.
                let mut report = row.result;
                if let Some(report) = report.as_mut() {
                    report_with_agent_jobs_as_they_stand(conn, report)?;
                }
                let mut summary =
                    arrival_summary(&row.job_id, &row.status, &row.created_at, report.as_ref());
                let agent_jobs = arrival_agent_jobs(report.as_ref());
                if !agent_jobs.is_empty() {
                    summary["agent_jobs"] = json!(agent_jobs);
                }
                match grouped.iter_mut().find(|(kind, _)| *kind == source_kind) {
                    Some((_, rows)) => rows.push(summary),
                    None => grouped.push((source_kind, vec![summary])),
                }
            }

            // A source the ledger knows leads; one that only arrivals remember
            // — a forgotten channel — follows, rather than disappearing.
            let mut imports = Vec::new();
            for (import_id, source_kind, created_at, remembered) in ledgers {
                let position = grouped.iter().position(|(kind, _)| *kind == source_kind);
                let rows = position.map(|at| grouped.remove(at).1).unwrap_or_default();
                imports.push(json!({
                    "import_id": import_id,
                    "source_kind": source_kind,
                    "created_at": created_at,
                    "remembered": remembered,
                    "arrivals": rows,
                }));
            }
            for (source_kind, rows) in grouped {
                let created_at = rows
                    .last()
                    .and_then(|row| row["created_at"].as_str())
                    .unwrap_or_default()
                    .to_string();
                imports.push(json!({
                    "import_id": Value::Null,
                    "source_kind": source_kind,
                    "created_at": created_at,
                    "remembered": 0,
                    "arrivals": rows,
                }));
            }

            Ok(json!({ "imports": imports }))
        })
    }

    /// Throw an Import's ledger away whole (ADR 0025): the record of which
    /// foreign id became which recipe, and the Import itself. Every recipe it
    /// made stays exactly as it is — nothing about where a recipe came from was
    /// ever on it. The cost is the ledger's whole point run backwards: an
    /// import of the same file afterwards matches nothing, and brings every
    /// recipe in again.
    pub fn forget_import(&self, person_id: &str, import_id: &str) -> Result<Value, OpError> {
        self.db().with_conn(|conn| {
            let cookbook_id: String = conn
                .query_row(
                    "SELECT cookbook_id FROM imports WHERE id = ?1",
                    params![import_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Import: {e}")))?
                .ok_or_else(no_such_import)?;
            ensure_writes_or_absent(conn, &cookbook_id, person_id, no_such_import)?;
            let transaction = conn
                .unchecked_transaction()
                .map_err(|e| OpError::internal(format!("cannot begin: {e}")))?;
            let forgotten = transaction
                .execute(
                    "DELETE FROM import_ledger WHERE import_id = ?1",
                    params![import_id],
                )
                .map_err(|e| OpError::internal(format!("cannot forget the ledger: {e}")))?;
            transaction
                .execute("DELETE FROM imports WHERE id = ?1", params![import_id])
                .map_err(|e| OpError::internal(format!("cannot forget the Import: {e}")))?;
            transaction
                .commit()
                .map_err(|e| OpError::internal(format!("cannot forget the Import: {e}")))?;
            Ok(json!({ "import_id": import_id, "forgotten": forgotten }))
        })
    }

    /// A file too large for an Operation's JSON envelope, staged on disk under
    /// the Person who sent it until an Operation names it by its id (ADR 0001:
    /// non-JSON payloads travel out of band under the same Credential). A
    /// Crouton library is 114 MB; the envelope, and a Job's stored input,
    /// should carry an id, not the library.
    ///
    /// Staged uploads live beside the database and the Photographs but are
    /// neither: no Backup carries them, and [`Core::sweep_uploads`] clears any
    /// nobody used.
    pub fn begin_upload(&self, person_id: &str) -> Result<(String, std::path::PathBuf), OpError> {
        let dir = self.uploads_dir(person_id)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| OpError::internal(format!("cannot create the uploads directory: {e}")))?;
        let upload_id = format!("u_{}", hex::encode(random_bytes(16)));
        let path = dir.join(&upload_id);
        Ok((upload_id, path))
    }

    /// Where the Person's own staged upload sits. Another Person's upload is
    /// not found rather than refused: its id is theirs, and says nothing here.
    pub fn staged_upload(
        &self,
        person_id: &str,
        upload_id: &str,
    ) -> Result<std::path::PathBuf, OpError> {
        let well_formed = upload_id.len() == 34
            && upload_id.starts_with("u_")
            && upload_id[2..].bytes().all(|b| b.is_ascii_hexdigit());
        let path = self.uploads_dir(person_id)?.join(upload_id);
        if !well_formed || !path.is_file() {
            return Err(OpError::not_found(
                "no such upload: send the file again, then ask with the id it answers",
            ));
        }
        Ok(path)
    }

    fn uploads_dir(&self, person_id: &str) -> Result<std::path::PathBuf, OpError> {
        if person_id.is_empty()
            || !person_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(OpError::internal(
                "a Person id is not a safe directory name",
            ));
        }
        Ok(self.data_dir().join("uploads").join(person_id))
    }

    /// How long a staged upload nobody used is kept. A day covers a person
    /// who sent a file and wandered off before the import started.
    const UPLOAD_GRACE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

    /// Delete staged uploads older than a day. An upload is consumed by the
    /// Operation that names it, so anything this finds is a file somebody
    /// sent and never used — or one whose import was interrupted.
    pub fn sweep_uploads(&self) -> Result<usize, OpError> {
        let root = self.data_dir().join("uploads");
        let Ok(people) = std::fs::read_dir(&root) else {
            return Ok(0);
        };
        let mut swept = 0;
        for person in people.flatten() {
            let Ok(files) = std::fs::read_dir(person.path()) else {
                continue;
            };
            for file in files.flatten() {
                let stale = file
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age > Self::UPLOAD_GRACE);
                if stale && std::fs::remove_file(file.path()).is_ok() {
                    swept += 1;
                }
            }
        }
        Ok(swept)
    }

    /// Import: land a batch of candidates already read from an outside
    /// source into the caller's own Cookbook, matched through that
    /// Kitchen's ledger for this source kind rather than doubled on every
    /// re-run (ADR 0025). Reading the source itself — a `.crumb`, a web
    /// page, a Bundle — is each importer's own job; this is the shared
    /// machinery every importer lands its candidates through.
    ///
    /// Every candidate gets a fate: newly made or found unchanged (both
    /// `arrived`), found changed and `offered` for review rather than
    /// written over, or `unreadable` and named with why. One candidate's
    /// failure never stops the rest — the Report is a ledger, not an error
    /// log that drops what it could not place.
    pub fn import(
        &self,
        caller: &Caller,
        source_kind: &str,
        candidates: &[Value],
        into: Option<&str>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        self.import_each(
            caller,
            source_kind,
            into,
            candidates.len(),
            candidates.iter().cloned().map(Incoming::candidate),
            progress,
        )
    }

    /// The same landing, for an importer that reads its source one recipe at a
    /// time rather than holding the whole of it: a Crouton library is 86 files
    /// and 110 MB of photographs, and none of it needs to be in memory at once
    /// (#69). `incoming` is pulled once per recipe, after the progress for it
    /// is reported, so the slow part of reading — remaking a photograph — is
    /// what the progress counts.
    ///
    /// Beyond what every Import Report says, this one also says what an
    /// importer read and deliberately did not bring in (`left_out`), and which
    /// of the recipes it landed share a name or a web page (`related_candidates`)
    /// — offered as **Related Recipes** to tick, never joined, since importing
    /// never guesses at a Lineage (ADR 0025).
    pub fn import_each(
        &self,
        caller: &Caller,
        source_kind: &str,
        into: Option<&str>,
        total: usize,
        mut incoming: impl Iterator<Item = Incoming>,
        progress: Option<&JobProgress>,
    ) -> Result<Value, OpError> {
        let source_kind = required_text(source_kind, "source_kind")?.to_string();
        // Into the caller's own Cookbook (ADR 0041), or the one an Operator
        // looking after it names (#302).
        let (cookbook_id, cookbook_hand_id) = self.db().with_conn(|conn| {
            let cookbook_id = lands_in(conn, caller, into)?;
            let hand_id = cookbook_hand(conn, &cookbook_id)?;
            Ok((cookbook_id, hand_id))
        })?;

        let import_id = self
            .db()
            .with_conn(|conn| find_or_create_import(conn, &cookbook_id, &source_kind))?;

        // Listed under its source while it runs (#412): an import that
        // recognises its file says only here which app made it, until the
        // whole Report replaces this frame of one.
        if let Some(progress) = progress {
            progress.result_so_far(&json!({
                "import_id": import_id, "cookbook_id": cookbook_id, "source_kind": source_kind,
                "arrived": [], "offered": [], "unreadable": [], "left_out": [],
                "related_candidates": [],
            }));
        }

        let total = total as u64;
        let mut arrived = Vec::new();
        let mut offered = Vec::new();
        let mut unreadable = Vec::new();
        let mut left_out = Vec::new();
        let mut landed = Vec::new();

        let mut done = 0u64;
        loop {
            if let Some(progress) = progress {
                progress.report(done, Some(total), format!("{done} of {total}"));
            }
            let Some(item) = incoming.next() else { break };
            done += 1;

            let candidate = match item.candidate {
                Ok(candidate) => candidate,
                Err(unread) => {
                    let mut row =
                        json!({ "foreign_id": unread.foreign_id, "reason": unread.reason });
                    if let Some(name) = unread.name {
                        row["name"] = json!(name);
                    }
                    unreadable.push(row);
                    continue;
                }
            };

            let foreign_id = match candidate.get("foreign_id").and_then(Value::as_str) {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => {
                    unreadable.push(json!({
                        "foreign_id": candidate.get("foreign_id").cloned().unwrap_or(Value::Null),
                        "reason": "a foreign id is required to keep the ledger",
                    }));
                    continue;
                }
            };
            let mut shape = LandedShape::of(&candidate);

            let (lineage_id, branch_id, title) = match self.import_one(
                caller,
                &cookbook_id,
                &cookbook_hand_id,
                &import_id,
                &foreign_id,
                &candidate,
                &item.tags,
                &item.component_lines,
            ) {
                Ok(ImportOutcome::Landed {
                    lineage_id,
                    branch_id,
                    title,
                    status,
                    held,
                }) => {
                    if let Some(held) = &held {
                        shape = LandedShape::of(held);
                    }
                    arrived.push(arrived_row(
                        &foreign_id,
                        status,
                        &lineage_id,
                        &branch_id,
                        &title,
                        &shape,
                    ));
                    (lineage_id, branch_id, title)
                }
                Ok(ImportOutcome::Offered {
                    lineage_id,
                    branch_id,
                    title,
                    candidate_version_id,
                }) => {
                    offered.push(offered_row(
                        &foreign_id,
                        &lineage_id,
                        &branch_id,
                        &title,
                        &candidate_version_id,
                    ));
                    (lineage_id, branch_id, title)
                }
                Err(err) => {
                    unreadable.push(json!({
                        "foreign_id": foreign_id,
                        "reason": err.to_sentence(),
                    }));
                    continue;
                }
            };

            for left in &item.left_out {
                left_out.push(left.row(&foreign_id, &branch_id, &title));
            }
            landed.push(Relatable {
                lineage_id,
                branch_id,
                title,
                shape,
            });
        }

        if let Some(progress) = progress {
            progress.report(total, Some(total), "finished".to_string());
        }

        let related_candidates = self
            .db()
            .with_conn(|conn| related_candidates(conn, &cookbook_id, &landed))?;

        Ok(json!({
            "import_id": import_id,
            "cookbook_id": cookbook_id,
            "source_kind": source_kind,
            "arrived": arrived,
            "offered": offered,
            "unreadable": unreadable,
            "left_out": left_out,
            "related_candidates": related_candidates,
        }))
    }

    /// Hand a link to an agent with nothing landed first (#349), as pasted
    /// text is handed over (#269): a Job that waits for a Runner with no
    /// recipe, whose agent creates it in the caller's own Cookbook under
    /// their Hand. Kamosu never fetches the link itself, so no bare recipe
    /// lands from the page's title. The same link sent again is not matched,
    /// and makes a Job of its own.
    ///
    /// The answer is the Job's id. Its result is an Import Report from the
    /// moment it is asked for, under the source a link imported is listed
    /// under, read as the Job stands.
    pub fn import_link_by_agent(
        &self,
        caller: &Caller,
        link: &str,
        into: Option<&str>,
    ) -> Result<Value, OpError> {
        self.ensure_has_runner()?;
        let failed = |e| OpError::internal(format!("cannot record the agent's Job: {e}"));
        self.db().with_conn(|conn| {
            let cookbook_id = lands_in(conn, caller, into)?;
            let import_id = find_or_create_import(conn, &cookbook_id, jobs::LINK_SOURCE)?;
            // The agent's create lands where this one does, which is the
            // Job's to say when it is not the asker's own (#302).
            let elsewhere = (cookbook_id != cookbook_of_person(conn, &caller.person_id)?)
                .then_some(cookbook_id.as_str());
            // No Runner is handed a Job whose Report is not written yet.
            let tx = conn.unchecked_transaction().map_err(failed)?;
            let job_id = jobs::wait_for_agent_on_link_alone(&tx, caller, link, elsewhere)?;
            jobs::set_result(
                &tx,
                &job_id,
                &report_before_its_agent(&import_id, &cookbook_id, jobs::LINK_SOURCE, &job_id),
            )?;
            tx.commit().map_err(failed)?;
            Ok(json!({ "job_id": job_id, "source_kind": jobs::LINK_SOURCE }))
        })
    }

    /// Have an agent read the link one Import just landed bare (ADR 0048,
    /// #261), and say so in that Import's Report.
    ///
    /// A recipe that `arrived` is the bare reading or an agent's writing,
    /// newly made or matched. A Job is made on it unless one stands already,
    /// which is named, or an agent has read it, when the Job that did is
    /// named and none is made (#271). A recipe that was `offered` the bare
    /// reading holds more than that: Kamosu read it in full once, or somebody
    /// wrote it since. Sending that to an agent is for a Co-author to ask
    /// (ADR 0048), so only a Job already standing on it is named. A candidate
    /// that landed nothing has no recipe for a Job to be on.
    pub fn hand_link_to_agent(
        &self,
        caller: &Caller,
        mut report: Value,
        link: &str,
    ) -> Result<Value, OpError> {
        let branch_of = |rows: &str| report[rows][0]["branch_id"].as_str().map(str::to_string);
        let job = self.db().with_conn(|conn| {
            if let Some(branch_id) = branch_of("arrived") {
                let job_id = match jobs::agent_job_completed_on(conn, &branch_id)? {
                    // A Job standing on it comes first, and `wait_for_agent`
                    // answers that one.
                    Some(completed) if jobs::standing_agent_job(conn, &branch_id)?.is_none() => {
                        completed
                    }
                    _ => jobs::wait_for_agent(conn, caller, &branch_id, link)?,
                };
                return Ok(Some((job_id, branch_id)));
            }
            let Some(branch_id) = branch_of("offered") else {
                return Ok(None);
            };
            Ok(jobs::standing_agent_job(conn, &branch_id)?.map(|job_id| (job_id, branch_id)))
        })?;
        if let Some((job_id, branch_id)) = job {
            report["agent_jobs"] = json!([{ "job_id": job_id, "branch_id": branch_id }]);
        }
        Ok(report)
    }

    /// One candidate against the ledger: unseen becomes a new Lineage,
    /// Branch and first Version, the importing Person's Hand on it
    /// (GLOSSARY.md, "Hand"; ADR 0025). Seen before and now identical is
    /// `Unchanged`; seen before and now different records the candidate's
    /// content as a Version — content-addressed, so this never collides with
    /// or moves anything already on the Branch — and answers `Offered`
    /// without touching `head_version_id`: the offer is never written over
    /// the Branch on its own. A bare candidate on a recipe an agent has
    /// written is `Unchanged` instead (#271): what the agent wrote is the
    /// reading of that source.
    ///
    /// Whatever the fate, the Branch is then filed under `tags`, English words
    /// in the Cookbook's own list (#128). Filing only adds, so a recipe matched
    /// again gains the tags it lacked and keeps every one given it here; and
    /// since a Tag is no part of a Version (ADR 0035), filing moves no id and
    /// leaves `unchanged` unchanged.
    ///
    /// A new recipe's lines in `component_lines` are left unread: they point
    /// at another recipe, and [`Core::land_components`] reads them (#413).
    #[allow(clippy::too_many_arguments)]
    fn import_one(
        &self,
        caller: &Caller,
        cookbook_id: &str,
        cookbook_hand_id: &str,
        import_id: &str,
        foreign_id: &str,
        candidate: &Value,
        tags: &[String],
        component_lines: &[usize],
    ) -> Result<ImportOutcome, OpError> {
        let content = parse_recipe_content(candidate)?;
        let title = content["title"].as_str().unwrap_or_default().to_string();
        let (version_id, content_text) = stored_version(&content);

        self.db().with_conn(|conn| {
            let existing: Option<(String, String, String)> = conn
                .query_row(
                    "SELECT import_ledger.lineage_id, import_ledger.branch_id, branches.head_version_id \
                       FROM import_ledger JOIN branches ON branches.id = import_ledger.branch_id \
                      WHERE import_ledger.import_id = ?1 AND import_ledger.foreign_id = ?2",
                    params![import_id, foreign_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read Import ledger: {e}")))?;

            let outcome = match existing {
                None => {
                    let lineage_id = format!("l_{}", hex::encode(random_bytes(8)));
                    let branch_id = format!("b_{}", hex::encode(random_bytes(8)));
                    // An imported recipe has no Language to disagree with, so
                    // detection simply sets one (ADR 0025) — the same rule a
                    // recipe typed in by hand follows, through the same code.
                    let language = language_for_new_branch(
                        conn,
                        &caller.person_id,
                        candidate.get("language").and_then(Value::as_str),
                        &content,
                    )?;

                    insert_new_lineage_and_branch(
                        conn,
                        &lineage_id,
                        &branch_id,
                        cookbook_id,
                        cookbook_hand_id,
                        &language,
                        &version_id,
                        &content_text,
                        caller,
                        component_lines,
                    )?;
                    conn.execute(
                        "INSERT INTO import_ledger (import_id, foreign_id, lineage_id, branch_id) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![import_id, foreign_id, lineage_id, branch_id],
                    )
                    .map_err(|e| {
                        OpError::internal(format!("cannot record Import ledger entry: {e}"))
                    })?;
                    Ok(ImportOutcome::Landed {
                        lineage_id,
                        branch_id,
                        title,
                        status: "created",
                        held: None,
                    })
                }
                Some((lineage_id, branch_id, head_version_id)) => {
                    if head_version_id == version_id {
                        Ok(ImportOutcome::Landed {
                            lineage_id,
                            branch_id,
                            title,
                            status: "unchanged",
                            held: None,
                        })
                    } else if LandedShape::of(candidate).bare
                        && agent_has_written(conn, &branch_id)?
                    {
                        // Described as the agent left it, not as the page is.
                        let held: Value = conn
                            .query_row(
                                "SELECT content FROM versions WHERE id = ?1",
                                params![head_version_id],
                                |row| row.get::<_, String>(0),
                            )
                            .map(|content| serde_json::from_str(&content).unwrap_or_default())
                            .map_err(|e| OpError::internal(format!("cannot read the recipe: {e}")))?;
                        Ok(ImportOutcome::Landed {
                            lineage_id,
                            branch_id,
                            title: held["title"].as_str().map_or(title, str::to_string),
                            status: "unchanged",
                            held: Some(held),
                        })
                    } else {
                        conn.execute(
                            "INSERT OR IGNORE INTO versions (id, content) VALUES (?1, ?2)",
                            params![version_id, content_text],
                        )
                        .map_err(|e| {
                            OpError::internal(format!("cannot record candidate Version: {e}"))
                        })?;
                        Ok(ImportOutcome::Offered {
                            lineage_id,
                            branch_id,
                            title,
                            candidate_version_id: version_id,
                        })
                    }
                }
            }?;

            let (ImportOutcome::Landed { branch_id, .. }
            | ImportOutcome::Offered { branch_id, .. }) = &outcome;
            for name in tags {
                if let Some(tag_id) = arriving_tag(conn, cookbook_id, &[("en", name)])? {
                    file_branch_under(conn, branch_id, &tag_id)?;
                }
            }
            Ok(outcome)
        })
    }
}

/// One recipe as an importer hands it to [`Core::import_each`]: the candidate,
/// or why the importer could not read one, and what it read but deliberately
/// did not bring in, named in the Report's `left_out`.
pub struct Incoming {
    pub candidate: Result<Value, Unread>,
    pub left_out: Vec<LeftOut>,
    /// English words to file the recipe under in the Kitchen's own list: a
    /// Crouton library's tags (#128). Beside the candidate, not in it,
    /// because a Tag is filing, not content (ADR 0035).
    pub tags: Vec<String>,
    /// Lines that point at another recipe, left unread on a new recipe until
    /// the whole file has landed (#413).
    pub component_lines: Vec<usize>,
}

impl LeftOut {
    /// The Report's row for it, naming the recipe it was left out of.
    fn row(&self, foreign_id: &str, branch_id: &str, title: &str) -> Value {
        let (what, count) = match self {
            LeftOut::SiteIcon(_) => ("site_icon", 1),
            LeftOut::ExtraPhotos(count) => ("extra_photos", *count),
            LeftOut::UnreadablePhotos(count) => ("unreadable_photos", *count),
            LeftOut::AttachedFiles(count) => ("attached_files", *count),
            LeftOut::Ratings(count) => ("ratings", *count),
            LeftOut::Favourites(count) => ("favourites", *count),
            LeftOut::Comments(count) => ("comments", *count),
            LeftOut::PhotoNotDownloaded(_) => ("photos_not_downloaded", 1),
            LeftOut::Nutrition => ("nutrition", 1),
            LeftOut::MealPlanTimeline(count) => ("meal_plan_timeline", *count),
        };
        let mut row = json!({
            "foreign_id": foreign_id, "branch_id": branch_id, "title": title,
            "what": what, "count": count,
        });
        if let LeftOut::SiteIcon(bytes) = self
            && let Some(icon) = icon_data_uri(bytes)
        {
            row["icon"] = json!(icon);
        }
        if let LeftOut::PhotoNotDownloaded(address) = self {
            row["address"] = json!(address);
        }
        row
    }
}

/// A favicon as a `data:` URI the Report can draw, or nothing when the bytes
/// are not a small picture in a format a browser shows. A site's icon is a
/// couple of kilobytes; anything past 64 KB is not one, and is not carried.
fn icon_data_uri(bytes: &[u8]) -> Option<String> {
    use base64::Engine;
    const MAX_ICON_BYTES: usize = 64 * 1024;
    let mime = match bytes {
        [0xff, 0xd8, 0xff, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'G', b'I', b'F', b'8', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        _ => return None,
    };
    (bytes.len() <= MAX_ICON_BYTES).then(|| {
        format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    })
}

impl Incoming {
    /// A candidate already read, with nothing left out of it.
    pub fn candidate(candidate: Value) -> Self {
        Incoming {
            candidate: Ok(candidate),
            left_out: Vec::new(),
            tags: Vec::new(),
            component_lines: Vec::new(),
        }
    }
}

/// One recipe from a reader, ready to land: what [`Core::import_each`] takes,
/// the lines that point at another recipe, its cookings, and whether the app
/// had a photo of it that the file does not carry.
struct Arriving {
    incoming: Incoming,
    components: Vec<other_apps::Component>,
    cookings: Vec<StoredCooking>,
    photo_not_in_export: bool,
}

/// A cooking a reader handed over, its photo stored as a Photograph.
struct StoredCooking {
    foreign_id: String,
    cooked_at: String,
    note: String,
    photograph: Option<String>,
}

/// A recipe the Report lists as having arrived.
fn arrived_row(
    foreign_id: &str,
    status: &str,
    lineage_id: &str,
    branch_id: &str,
    title: &str,
    shape: &LandedShape,
) -> Value {
    json!({
        "foreign_id": foreign_id, "status": status,
        "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
        "main_photo": shape.main_photo, "bare": shape.bare,
    })
}

/// A recipe the Report lists as changed and offered.
fn offered_row(
    foreign_id: &str,
    lineage_id: &str,
    branch_id: &str,
    title: &str,
    candidate_version_id: &str,
) -> Value {
    json!({
        "foreign_id": foreign_id,
        "lineage_id": lineage_id, "branch_id": branch_id, "title": title,
        "candidate_version_id": candidate_version_id,
    })
}

/// Add a row to one of the Report's lists.
fn push_row(report: &mut Value, list: &str, row: Value) {
    if let Some(rows) = report[list].as_array_mut() {
        rows.push(row);
    }
}

/// The Version whose lines a recipe of the file holds, as the Report says it
/// landed: the Version offered where it was offered, and the recipe as it
/// stands otherwise.
struct LinesOn {
    branch_id: String,
    offered: Option<String>,
}

impl LinesOn {
    fn of(report: &Value, foreign_id: &str) -> Option<LinesOn> {
        let row = |list: &str| {
            report[list]
                .as_array()?
                .iter()
                .find(|row| row["foreign_id"] == foreign_id)
                .cloned()
        };
        if let Some(offered) = row("offered") {
            return Some(LinesOn {
                branch_id: offered["branch_id"].as_str()?.to_string(),
                offered: Some(offered["candidate_version_id"].as_str()?.to_string()),
            });
        }
        let arrived = row("arrived")?;
        Some(LinesOn {
            branch_id: arrived["branch_id"].as_str()?.to_string(),
            offered: None,
        })
    }
}

/// The Lineage an Import's ledger knows a foreign id as.
fn ledger_lineage(
    conn: &Connection,
    import_id: &str,
    foreign_id: &str,
) -> Result<Option<String>, OpError> {
    conn.query_row(
        "SELECT lineage_id FROM import_ledger WHERE import_id = ?1 AND foreign_id = ?2",
        params![import_id, foreign_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| OpError::internal(format!("cannot read Import ledger: {e}")))
}

/// Point one line at the Lineage the app's line pointed at (#413), counted as
/// set by hand. A Reading somebody set already stands.
///
/// How much of it the line asks for is said in the inner recipe's own Yield,
/// so [`units::how_much_of`] works out the share the app meant: the app counts
/// in whole recipes, and a cook's `1 batch` of a dough that makes `2 balls`
/// is both balls. One is the whole recipe and needs no amount at all.
///
/// Where nothing is there to point at, the line on the recipe as it stands is
/// read as any other line is. An offered Version is read when it is taken.
fn point_line(
    conn: &Connection,
    lines_on: &LinesOn,
    line: usize,
    lineage_id: Option<&str>,
    times: Option<f64>,
) -> Result<(), OpError> {
    // A recipe deleted since it landed has no line left to point.
    let Some((head_version_id, language)): Option<(String, String)> = conn
        .query_row(
            "SELECT head_version_id, language FROM branches WHERE id = ?1",
            params![lines_on.branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the recipe: {e}")))?
    else {
        return Ok(());
    };
    let version_id = lines_on.offered.as_deref().unwrap_or(&head_version_id);
    let line_index = line as i64;

    let Some(lineage_id) = lineage_id else {
        if lines_on.offered.is_none() {
            let content = version_content(conn, version_id)?;
            read_unread_lines(
                conn,
                version_id,
                &language,
                &content,
                Some(&HashSet::from([line])),
                None,
            );
        }
        return Ok(());
    };

    let set_by_hand: bool = conn
        .query_row(
            "SELECT by_hand FROM readings WHERE version_id = ?1 AND line_index = ?2",
            params![version_id, line_index],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the Reading: {e}")))?
        .unwrap_or(false);
    if set_by_hand {
        return Ok(());
    }

    let (amount, unit) = match times.filter(|times| (times - 1.0).abs() > f64::EPSILON) {
        None => (None, None),
        Some(times) => {
            let made: Option<Value> = conn
                .query_row(
                    "SELECT versions.content FROM branches
                       JOIN versions ON versions.id = branches.head_version_id
                      WHERE branches.lineage_id = ?1 LIMIT 1",
                    params![lineage_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| OpError::internal(format!("cannot read the recipe: {e}")))?
                .and_then(|content| serde_json::from_str::<Value>(&content).ok())
                .map(|content| content["yield"].clone());
            let made_amount = made
                .as_ref()
                .and_then(|made| made["amount"].as_str())
                .and_then(units::parse_amount)
                .filter(|amount| *amount > 0.0);
            let noun = made.as_ref().and_then(|made| made["noun"].as_str());
            match (made_amount, noun) {
                (Some(made_amount), Some(noun)) => (
                    Some(units::as_decimal(times * made_amount)),
                    Some(noun.to_string()),
                ),
                _ => (Some(units::as_decimal(times)), None),
            }
        }
    };
    conn.execute(
        "INSERT INTO readings (version_id, line_index, amount, unit, target, food_id, lineage_id, by_hand) \
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, 1) \
         ON CONFLICT (version_id, line_index) DO UPDATE SET \
            amount = excluded.amount, unit = excluded.unit, target = NULL, \
            food_id = NULL, lineage_id = excluded.lineage_id, by_hand = 1, \
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![version_id, line_index, amount, unit, lineage_id],
    )
    .map_err(|e| OpError::internal(format!("cannot save the Component: {e}")))?;
    Ok(())
}

/// What the Report says about a landed recipe beyond its name, read off the
/// candidate rather than the stored Version, which is the same content.
struct LandedShape {
    main_photo: Option<String>,
    /// Neither an Ingredient Line nor a Step: a name and, usually, a link —
    /// a real recipe and a real habit, reported as arriving, never as failing
    /// (ADR 0025).
    bare: bool,
    ingredients: usize,
    link: Option<String>,
}

impl LandedShape {
    fn of(candidate: &Value) -> Self {
        let count = |field: &str, kind: &str| {
            candidate[field].as_array().map_or(0, |items| {
                items.iter().filter(|item| item["kind"] == kind).count()
            })
        };
        let ingredients = count("ingredients", "ingredient");
        LandedShape {
            main_photo: candidate["main_photo"]
                .as_str()
                .filter(|photo| !photo.is_empty())
                .map(str::to_string),
            bare: ingredients == 0 && count("steps", "step") == 0,
            ingredients,
            link: candidate["source"]["link"]
                .as_str()
                .map(str::trim)
                .filter(|link| !link.is_empty())
                .map(str::to_string),
        }
    }
}

struct Relatable {
    lineage_id: String,
    branch_id: String,
    title: String,
    shape: LandedShape,
}

/// The name two recipes are compared by when offered as related: case and
/// spacing set aside, and a trailing `(Version 2)` — how a person marks a
/// second go at a dish in an app with nowhere else to put it.
fn relating_name(title: &str) -> String {
    let folded = title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if let Some(open) = folded.rfind(" (version ")
        && folded.ends_with(')')
    {
        let number = &folded[open + " (version ".len()..folded.len() - 1];
        if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) {
            return folded[..open].to_string();
        }
    }
    folded
}

/// Pairs among the recipes one Import landed that share a name or a web page,
/// offered as Related Recipe candidates (GLOSSARY.md, "Related Recipe"). A
/// site's name alone is not enough: nine Instagram recipes share one and have
/// nothing to do with each other. A pair already related is not offered again.
fn related_candidates(
    conn: &Connection,
    cookbook_id: &str,
    landed: &[Relatable],
) -> Result<Vec<Value>, OpError> {
    let mut pairs: Vec<((usize, usize), Vec<&'static str>)> = Vec::new();
    let mut offer = |a: usize, b: usize, shared: &'static str| match pairs
        .iter_mut()
        .find(|(pair, _)| *pair == (a, b))
    {
        Some((_, why)) => {
            if !why.contains(&shared) {
                why.push(shared);
            }
        }
        None => pairs.push(((a, b), vec![shared])),
    };
    let names: Vec<String> = landed.iter().map(|r| relating_name(&r.title)).collect();
    for a in 0..landed.len() {
        for b in a + 1..landed.len() {
            if landed[a].lineage_id == landed[b].lineage_id {
                continue;
            }
            if names[a] == names[b] {
                offer(a, b, "name");
            }
            if landed[a].shape.link.is_some() && landed[a].shape.link == landed[b].shape.link {
                offer(a, b, "page");
            }
        }
    }

    let mut offered = Vec::new();
    for ((a, b), shared) in pairs {
        let (first, second) = (&landed[a], &landed[b]);
        let (low, high) = if first.lineage_id < second.lineage_id {
            (&first.lineage_id, &second.lineage_id)
        } else {
            (&second.lineage_id, &first.lineage_id)
        };
        let already: bool = conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM related_recipes \
                  WHERE cookbook_id = ?1 AND lineage_a_id = ?2 AND lineage_b_id = ?3)",
                params![cookbook_id, low, high],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot read Related Recipes: {e}")))?;
        if already {
            continue;
        }
        let side = |recipe: &Relatable| {
            json!({
                "lineage_id": recipe.lineage_id, "branch_id": recipe.branch_id,
                "title": recipe.title, "main_photo": recipe.shape.main_photo,
                "ingredients": recipe.shape.ingredients,
            })
        };
        offered.push(json!({ "recipes": [side(first), side(second)], "shared": shared }));
    }
    Ok(offered)
}

/// Whether an agent has written a recipe Kamosu's bare reading of its source
/// would otherwise be offered as a change to (#271): an agent's Job on it
/// completed, or the last save on it was an agent's. The bare reading always
/// differs from an agent's writing, so it is no news. A recipe a person wrote
/// after an agent that never completed is offered as before (#261).
fn agent_has_written(conn: &Connection, branch_id: &str) -> Result<bool, OpError> {
    if jobs::agent_job_completed_on(conn, branch_id)?.is_some() {
        return Ok(true);
    }
    conn.query_row(
        "SELECT job_id IS NOT NULL FROM branch_versions
          WHERE branch_id = ?1 ORDER BY sequence DESC LIMIT 1",
        params![branch_id],
        |row| row.get(0),
    )
    .optional()
    .map(Option::unwrap_or_default)
    .map_err(|e| OpError::internal(format!("cannot read the recipe's History: {e}")))
}

/// What became of one Import candidate against the ledger.
enum ImportOutcome {
    /// Newly made (`status: "created"`) or matched and found unchanged
    /// (`status: "unchanged"`) — both `arrived`, told apart only by that tag.
    Landed {
        lineage_id: String,
        branch_id: String,
        title: String,
        status: &'static str,
        /// The recipe as it stands, where the Report describes it rather
        /// than the candidate: a bare reading matched to what an agent
        /// wrote (#271).
        held: Option<Value>,
    },
    Offered {
        lineage_id: String,
        branch_id: String,
        title: String,
        candidate_version_id: String,
    },
}

/// One arrival as the database hands it over, before it is placed under a
/// source (#108). Named rather than a six-wide tuple, because positional
/// fields of which four are `String` are how the wrong two get swapped.
struct ArrivalRow {
    job_id: String,
    operation: String,
    /// The `source_kind` this Job's own input declared, which only the general
    /// `import` carries — read with `json_extract` so a base64 export never
    /// leaves the database.
    declared: Option<String>,
    status: String,
    /// The Import Report, once there is one.
    result: Option<Value>,
    created_at: String,
}

/// Which source an arrival belongs to, answerable the moment the Job row
/// exists rather than only once it has finished (#108).
///
/// The Report states `source_kind` outright, and it is the authority, because
/// it is the very string the ledger row was opened under. But it exists only
/// once the work has finished, and an import still in flight is exactly the one
/// a screen most wants to place. So the Report is read where there is one and
/// the Job's own declaration otherwise: three importers name their source in
/// their own definition, and `import`, the general one, carries it in its input
/// where its schema requires it.
///
/// `Report.svelte` faces the same question and answers it the other way round,
/// preferring the Job's Operation over the Report — which is not a disagreement
/// but the same reasoning applied to a different need. It is deciding what to
/// call a page while the bar is still filling, so the earliest answer wins
/// outright. Here the row being placed is usually long finished, and where it
/// is not, this falls back to exactly what that screen reads.
fn arrival_source_kind(
    operation: &str,
    declared: Option<&str>,
    result: Option<&Value>,
) -> Option<String> {
    if let Some(kind) = result
        .and_then(|report| report.get("source_kind"))
        .and_then(Value::as_str)
    {
        return Some(kind.to_string());
    }
    match operation {
        "import_crouton" => Some("crouton".to_string()),
        "import_bundle" => Some("bundle".to_string()),
        "import_web_link" => Some(jobs::LINK_SOURCE.to_string()),
        "import" => declared.map(str::to_string),
        _ => None,
    }
}

/// One arrival as its row: when it happened, how it ended, and the three
/// counts that let a screen say what happened in a line without reading the
/// whole Report back. The Report itself is untouched and still reached by
/// `get_job` with `job_id` (ADR 0025) — these are a summary of it, never a
/// second copy of it.
fn arrival_summary(job_id: &str, status: &str, created_at: &str, result: Option<&Value>) -> Value {
    let arrived = result
        .and_then(|report| report.get("arrived"))
        .and_then(Value::as_array);
    let count = |key: &str| {
        result
            .and_then(|report| report.get(key))
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    };
    json!({
        "job_id": job_id,
        "status": status,
        "created_at": created_at,
        "arrived": arrived.map_or(0, Vec::len),
        "created": arrived.map_or(0, |rows| {
            rows.iter()
                .filter(|row| row["status"] == json!("created"))
                .count()
        }),
        "offered": count("offered"),
        "unreadable": count("unreadable"),
    })
}

/// The agent's Jobs an arrival's Report names, each with the recipe it is on
/// and its state now (#261): the row `list_imports` shows, cut from a Report
/// already brought up to date.
fn arrival_agent_jobs(report: Option<&Value>) -> Vec<Value> {
    report
        .and_then(|report| report.get("agent_jobs"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("status").is_some())
                .map(|row| {
                    json!({
                        "job_id": row["job_id"], "branch_id": row["branch_id"],
                        "status": row["status"],
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Bring an Import Report up to date with the agent's Jobs it names (#262).
///
/// The Report was written when the Import ended, and the agent's Job has
/// moved since. So the Report is kept as it was written and each reading
/// adds how those Jobs stand now. That keeps the bare recipe and the agent's
/// work one arrival, however many Reports name the Job.
///
/// Each Job gains its `status`. One an agent completed says `made_by_agent`,
/// and one whose Original was deleted says `original_gone` (#270). One whose
/// agent said it holds no recipe says `could_not_be_read` (#276).
/// One a Runner reported on carries the agent's own report. One that failed,
/// whoever ended it, is also listed under `unreadable` with the reason, and
/// the bare recipe stays under `arrived`. A Report naming no such Job is
/// left exactly as it was.
///
/// A Job reading an Original is its own Import, and its Report names it with
/// no recipe, since the agent creates one (#269). Once it has, the recipe is
/// named and listed under `arrived` as it reads now. A recipe read again is
/// its own Import too (#272), and its arrival is listed as it reads now.
pub(super) fn report_with_agent_jobs_as_they_stand(
    conn: &Connection,
    report: &mut Value,
) -> Result<(), OpError> {
    let Some(named) = report.get("agent_jobs").and_then(Value::as_array) else {
        return Ok(());
    };
    let ids: Vec<String> = named
        .iter()
        .filter_map(|row| row["job_id"].as_str().map(str::to_string))
        .collect();
    let states = jobs::agent_jobs_as_they_stand(conn, &ids)?;

    let mut could_not_be_read = Vec::new();
    for state in states {
        let Some(row) = report["agent_jobs"].as_array_mut().and_then(|rows| {
            rows.iter_mut()
                .find(|row| row["job_id"] == json!(state.job_id))
        }) else {
            continue;
        };
        row["status"] = json!(state.status.as_str());
        // The recipe an agent reading an Original, or a link handed to it
        // alone, has created since.
        let created = match &state.branch_id {
            Some(branch_id) if row["branch_id"].is_null() => {
                row["branch_id"] = json!(branch_id);
                Some(branch_id.clone())
            }
            _ => None,
        };
        if state.status == jobs::JobStatus::Completed {
            row["made_by_agent"] = json!(true);
        }
        if let Some(agent_report) = state.report {
            row["agent_report"] = agent_report;
        }
        if state.original_gone {
            row["original_gone"] = json!(true);
        }
        if state.could_not_be_read {
            row["could_not_be_read"] = json!(true);
        }
        if state.status == jobs::JobStatus::Failed {
            could_not_be_read.push((row["branch_id"].clone(), state.error));
        }
        if let Some(branch_id) = created
            && let Some(arrival) = created_by_agent(conn, &state.job_id, &branch_id)?
            && let Some(arrived) = report["arrived"].as_array_mut()
        {
            arrived.push(arrival);
        }
    }

    // A recipe read again was listed as it read when the cook asked, and is
    // read as it reads now, the agent's work included (#272).
    if let Some(arrived) = report["arrived"].as_array_mut() {
        for row in arrived
            .iter_mut()
            .filter(|row| row["status"] == "read_again")
        {
            let (Some(branch_id), Some(foreign_id)) =
                (row["branch_id"].as_str(), row["foreign_id"].as_str())
            else {
                continue;
            };
            if let Some(now) = arrival_as_it_reads(conn, branch_id, foreign_id, "read_again")? {
                *row = now;
            }
        }
    }

    for (branch_id, reason) in could_not_be_read {
        // The recipe the Job was on is listed above, bare, under the id and
        // the name it arrived with.
        let landed = ["arrived", "offered"]
            .iter()
            .filter_map(|rows| report[*rows].as_array())
            .flatten()
            .find(|row| row["branch_id"] == branch_id)
            .cloned()
            .unwrap_or(Value::Null);
        let mut entry = json!({
            "foreign_id": landed["foreign_id"],
            "reason": reason.unwrap_or_else(|| "the agent's Job failed".to_string()),
        });
        // A Job that creates its recipe may fail before it made any.
        if !branch_id.is_null() {
            entry["branch_id"] = branch_id;
        }
        if landed["title"].is_string() {
            entry["name"] = landed["title"].clone();
        }
        if let Some(unreadable) = report["unreadable"].as_array_mut() {
            unreadable.push(entry);
        }
    }
    Ok(())
}

/// The Import Report of an agent's Job that creates its recipe, before it
/// has (#269, #349): nothing arrived yet, and the one Job named with no
/// recipe. It is read as the Job stands from then on.
pub(super) fn report_before_its_agent(
    import_id: &str,
    cookbook_id: &str,
    source_kind: &str,
    job_id: &str,
) -> Value {
    json!({
        "import_id": import_id,
        "cookbook_id": cookbook_id,
        "source_kind": source_kind,
        "arrived": [],
        "offered": [],
        "unreadable": [],
        "left_out": [],
        "related_candidates": [],
        "agent_jobs": [{ "job_id": job_id, "branch_id": null }],
    })
}

/// The recipe an agent created for a Job reading an Original, or a link
/// handed to it alone, as the Report lists an arrival (#269, #349): the Job's
/// id stands for the foreign id, since no ledger names what was handed over.
/// None once the recipe is deleted.
fn created_by_agent(
    conn: &Connection,
    job_id: &str,
    branch_id: &str,
) -> Result<Option<Value>, OpError> {
    arrival_as_it_reads(conn, branch_id, job_id, "created")
}

/// A recipe as the Report lists an arrival, read as it stands now, under
/// `foreign_id` and `status`. None where the recipe is not there.
pub(super) fn arrival_as_it_reads(
    conn: &Connection,
    branch_id: &str,
    foreign_id: &str,
    status: &str,
) -> Result<Option<Value>, OpError> {
    let found: Option<(String, String)> = conn
        .query_row(
            "SELECT branches.lineage_id, versions.content
               FROM branches JOIN versions ON versions.id = branches.head_version_id
              WHERE branches.id = ?1",
            params![branch_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| OpError::internal(format!("cannot read the agent's recipe: {e}")))?;
    let Some((lineage_id, content)) = found else {
        return Ok(None);
    };
    let content: Value = serde_json::from_str(&content).unwrap_or_default();
    let shape = LandedShape::of(&content);
    Ok(Some(json!({
        "foreign_id": foreign_id,
        "status": status,
        "lineage_id": lineage_id,
        "branch_id": branch_id,
        "title": content["title"].as_str().unwrap_or_default(),
        "main_photo": shape.main_photo,
        "bare": shape.bare,
    })))
}

/// An Import id that names nothing here — and, by ADR 0040, an Import of a
/// Cookbook the caller may not see.
fn no_such_import() -> OpError {
    OpError::not_found("no such Import")
}
