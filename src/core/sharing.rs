//! Adding a recipe from a phone's Share menu (ADR 0055): `add_this`, the one
//! Operation an iPhone's Shortcut and Android's share target hand everything
//! to. Kamosu decides here what the Shortcut never could: whether a link goes
//! to Kamosu's own reader or to the agent, what is true on an instance with no
//! agent, whether the link is already on a shelf, and the words the phone
//! shows, in the asker's Reading Language.
//!
//! The Shortcut is the one piece of Kamosu on someone else's phone that a
//! release cannot update, so what `add_this` takes and answers changes only in
//! ways an older Shortcut survives.

use super::*;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// What was shared, as `add_this` takes it.
pub enum Shared {
    Link(String),
    Text(String),
    /// One PDF, or one or more pictures in order.
    Files(Vec<Vec<u8>>),
}

/// The Shortcut's version Kamosu publishes now, and the oldest it still
/// serves. A release that changes the Shortcut raises the first, and one
/// that can no longer answer an older Shortcut raises the second.
pub const SHORTCUT_VERSION: u64 = 1;
pub const OLDEST_SHORTCUT_SERVED: u64 = 1;

/// How long a link waits for its page to be read before `add_this` answers
/// that it was sent (ADR 0055).
pub const LINK_WAIT: Duration = Duration::from_secs(10);

/// How long a Runner may go unheard before the agent is not answering: the
/// agent tracker's rule (`LATELY_MS` in `ui/src/lib/agent/agent.ts`).
const AGENT_LATELY: Duration = Duration::from_secs(5 * 60);

/// The Shortcut versions and the link's wait, which a test may change.
pub struct Sharing {
    current: AtomicU64,
    oldest: AtomicU64,
    wait_millis: AtomicU64,
}

impl Default for Sharing {
    fn default() -> Self {
        Sharing {
            current: AtomicU64::new(SHORTCUT_VERSION),
            oldest: AtomicU64::new(OLDEST_SHORTCUT_SERVED),
            wait_millis: AtomicU64::new(LINK_WAIT.as_millis() as u64),
        }
    }
}

/// How it went, beside the words: what Android's page draws its mark and its
/// button from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Added,
    Sent,
    Already,
    NotAdded,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Added => "added",
            Outcome::Sent => "sent",
            Outcome::Already => "already",
            Outcome::NotAdded => "not_added",
        }
    }
}

/// One answer before its words are chosen.
struct Answer {
    outcome: Outcome,
    said: Said,
    job_id: Option<String>,
    branch_id: Option<String>,
}

/// What `add_this` says, before it is said in a Language.
enum Said {
    Added(String),
    SentToAgent,
    AgentAway,
    StillReading,
    ReadInKamosu,
    NoAgent,
    PageRefused,
    AlreadyOnShelf(String),
    BeingAdded,
    Failed(String),
    TooOld,
}

impl Answer {
    fn new(outcome: Outcome, said: Said) -> Self {
        Answer {
            outcome,
            said,
            job_id: None,
            branch_id: None,
        }
    }
    fn job(mut self, job_id: Option<&str>) -> Self {
        self.job_id = job_id.map(str::to_string);
        self
    }
    fn recipe(mut self, branch_id: Option<&str>) -> Self {
        self.branch_id = branch_id.map(str::to_string);
        self
    }
}

impl Core {
    /// Add what a phone shared (ADR 0055). Text that is only a link is that
    /// link. A link goes to Kamosu's own reader, unless the Credential in
    /// hand sends everything to the agent, and the answer waits for its page
    /// to be read, [`LINK_WAIT`] at most. Text, pictures and a PDF go to the
    /// agent and are answered at once. Every answer carries the words the
    /// phone shows, in the asker's Reading Language.
    pub fn add_this(
        &self,
        caller: &Caller,
        shared: Shared,
        shortcut_version: Option<u64>,
    ) -> Result<Value, OpError> {
        let language = self
            .db()
            .with_conn(|conn| reading_language_of(conn, &caller.person_id))?;
        let (current, oldest) = (
            self.sharing.current.load(Ordering::Relaxed),
            self.sharing.oldest.load(Ordering::Relaxed),
        );
        let answer = if shortcut_version.is_some_and(|version| version < oldest) {
            Answer::new(Outcome::NotAdded, Said::TooOld)
        } else {
            self.add_shared(caller, shared)?
        };
        let mut message = words(&language).say(&answer.said);
        // Served, and one behind: the phone hears a newer one is there.
        if shortcut_version.is_some_and(|version| version >= oldest && version < current) {
            message = format!("{message} {}", words(&language).newer_shortcut);
        }
        let mut answered = json!({
            "outcome": answer.outcome.as_str(),
            "message": message,
            "shortcut_version": current,
        });
        if let Some(job_id) = answer.job_id {
            answered["job_id"] = json!(job_id);
        }
        if let Some(branch_id) = answer.branch_id {
            answered["branch_id"] = json!(branch_id);
        }
        if let Said::Added(title) | Said::AlreadyOnShelf(title) = &answer.said {
            answered["title"] = json!(title);
        }
        Ok(answered)
    }

    fn add_shared(&self, caller: &Caller, shared: Shared) -> Result<Answer, OpError> {
        match shared {
            Shared::Link(link) => self.add_link(caller, link.trim()),
            Shared::Text(text) => match only_a_link(&text) {
                Some(link) => self.add_link(caller, link),
                None => self.to_the_agent(caller, Handed::Text(text)),
            },
            Shared::Files(mut files) => {
                // Text sent as a file is text: a Shortcut sends whatever is not
                // a link as a file, and leaves telling them apart to Kamosu.
                if let [file] = files.as_slice()
                    && !crate::pdf::looks_like_a_pdf(file)
                    && !crate::photographs::looks_like_a_picture(file)
                    && let Ok(text) = std::str::from_utf8(file)
                {
                    return self.add_shared(caller, Shared::Text(text.to_string()));
                }
                let handed = match files.len() {
                    1 => Handed::File(files.remove(0)),
                    _ if files.iter().any(|file| crate::pdf::looks_like_a_pdf(file)) => {
                        return Err(OpError::bad_request(
                            "a PDF is shared on its own: several files shared together are \
                             pictures, the pages of one recipe",
                        ));
                    }
                    _ => Handed::Pages(files),
                };
                self.to_the_agent(caller, handed)
            }
        }
    }

    /// Text, pictures or a PDF, handed to the agent, or the refusal of an
    /// instance with none.
    fn to_the_agent(&self, caller: &Caller, handed: Handed) -> Result<Answer, OpError> {
        if !self.has_runner()? {
            return Ok(Answer::new(Outcome::NotAdded, Said::NoAgent));
        }
        let answered = self.import_by_agent(caller, handed, None)?;
        Ok(self.handed_to_agent(&answered))
    }

    /// What `import_by_agent` and `import_link_by_agent` answered, as the
    /// phone hears it: the recipe an agent made from the same file before,
    /// or the Job now reading it.
    fn handed_to_agent(&self, answered: &Value) -> Answer {
        if let Some(branch_id) = answered["branch_id"].as_str() {
            let title = answered["title"].as_str().unwrap_or_default().to_string();
            return Answer::new(Outcome::Already, Said::AlreadyOnShelf(title))
                .recipe(Some(branch_id));
        }
        self.sent_to_agent().job(answered["job_id"].as_str())
    }

    fn add_link(&self, caller: &Caller, link: &str) -> Result<Answer, OpError> {
        if link.len() > crate::operations::LONGEST_LINK
            || !crate::web_import::is_readable_link(link)
        {
            return Err(OpError::bad_request(format!(
                "that is not a web address Kamosu can read: an http:// or https:// link of at \
                 most {} bytes, to no address on the local network",
                crate::operations::LONGEST_LINK
            )));
        }
        let link = &crate::video_link::without_tracking(link);
        if let Some((branch_id, title)) = self.on_a_shelf(caller, link)? {
            return Ok(
                Answer::new(Outcome::Already, Said::AlreadyOnShelf(title)).recipe(Some(&branch_id))
            );
        }
        if let Some(job_id) = self.being_added(caller, link)? {
            return Ok(Answer::new(Outcome::Already, Said::BeingAdded).job(Some(&job_id)));
        }
        let has_runner = self.has_runner()?;
        if has_runner && self.everything_to_agent(caller)? {
            let answered = self.import_link_by_agent(caller, link, None)?;
            return Ok(self.handed_to_agent(&answered));
        }
        // Kamosu's own reader, as `import_web_link` is asked from the browser,
        // but landing nothing it cannot read alone: that is for Kamosu itself,
        // where it can be checked.
        let invocation = Invocation {
            asker: Asker::Person(caller.clone()),
            job: None,
        };
        let asked = self.ask_job(
            "import_web_link",
            JobLane::AlwaysSingle,
            &invocation,
            json!({ "url": link, "refuse_unread": true }),
        )?;
        let job_id = asked["job_id"]
            .as_str()
            .ok_or_else(|| OpError::internal("a Job came back with no id"))?
            .to_string();
        let Some(record) = self.wait_for(&job_id)? else {
            return Ok(Answer::new(Outcome::Sent, Said::StillReading).job(Some(&job_id)));
        };
        Ok(self.link_read(record, has_runner).job(Some(&job_id)))
    }

    /// How a link's Job ended, as the phone hears it.
    fn link_read(&self, record: JobRecord, has_runner: bool) -> Answer {
        if record.status != jobs::JobStatus::Completed {
            // With no agent to try where Kamosu could not, a page that would
            // not open, however it failed, is one to copy the text of.
            return match record.error_reason.as_deref() {
                Some(NOT_READ_ALONE) => Answer::new(Outcome::NotAdded, Said::ReadInKamosu),
                _ if !has_runner => Answer::new(Outcome::NotAdded, Said::PageRefused),
                _ => Answer::new(
                    Outcome::NotAdded,
                    Said::Failed(record.error.unwrap_or_default()),
                ),
            };
        }
        let report = record.result.unwrap_or_default();
        // A Job that has ended already read this link: the recipe is held,
        // and nothing was sent.
        let reading = report["agent_jobs"].as_array().is_some_and(|named| {
            named.iter().any(|job| {
                job["job_id"]
                    .as_str()
                    .and_then(|job_id| jobs::read(self, job_id).ok().flatten())
                    .is_some_and(|job| !has_ended(&job))
            })
        });
        if reading {
            return self.sent_to_agent();
        }
        let arrived = report["arrived"].as_array().and_then(|rows| {
            rows.iter()
                .find(|row| row["subject"] == json!(true))
                .or(rows.first())
        });
        // The ledger knows a recipe by its page's own address, after
        // redirects, and a video by its post: a link that only led there is that recipe, which arrives
        // unchanged, or is offered where it has changed since.
        let known = report["offered"].as_array().and_then(|rows| rows.first());
        let named = |row: &Value| row["title"].as_str().unwrap_or_default().to_string();
        match (arrived, known) {
            (Some(row), _) if row["status"] != json!("unchanged") => {
                Answer::new(Outcome::Added, Said::Added(named(row)))
                    .recipe(row["branch_id"].as_str())
            }
            (Some(row), _) | (None, Some(row)) => {
                Answer::new(Outcome::Already, Said::AlreadyOnShelf(named(row)))
                    .recipe(row["branch_id"].as_str())
            }
            (None, None) => {
                let reason = report["unreadable"][0]["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                Answer::new(Outcome::NotAdded, Said::Failed(reason))
            }
        }
    }

    /// Wait for a Job to end, as long as a link waits, and answer it as it
    /// ended, or nothing when the wait ran out. Off the request workers, since
    /// a page can take seconds (`slow_work`).
    fn wait_for(&self, job_id: &str) -> Result<Option<JobRecord>, OpError> {
        let wait = Duration::from_millis(self.sharing.wait_millis.load(Ordering::Relaxed));
        crate::slow_work::off_the_request_workers(|| {
            let until = Instant::now() + wait;
            loop {
                let record = jobs::read(self, job_id)?
                    .ok_or_else(|| OpError::internal("the link's Job went missing"))?;
                if has_ended(&record) {
                    return Ok(Some(record));
                }
                if Instant::now() >= until {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    }

    /// A recipe the caller can see that came from this link, or from another
    /// link to the same video, and its name: its Source says so, or the ledger
    /// of links does, for a recipe whose Source has changed or names none.
    fn on_a_shelf(&self, caller: &Caller, link: &str) -> Result<Option<(String, String)>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                &format!(
                    "SELECT branches.id, COALESCE(json_extract(versions.content, '$.title'), '')
                       FROM branches JOIN versions ON versions.id = branches.head_version_id
                      WHERE (link_identity(json_extract(versions.content, '$.source.link')) = ?1
                             OR branches.id IN (
                                SELECT import_ledger.branch_id
                                  FROM import_ledger
                                  JOIN imports ON imports.id = import_ledger.import_id
                                 WHERE imports.source_kind = ?3
                                   AND link_identity(import_ledger.foreign_id) = ?1))
                        AND {}
                      ORDER BY {}
                      LIMIT 1",
                    visible_to("branches", "?2"),
                    own_first("branches", "?2"),
                ),
                params![
                    crate::video_link::link_identity(link),
                    caller.person_id,
                    jobs::LINK_SOURCE
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot look for the link on a shelf: {e}")))
        })
    }

    /// One of the caller's own imports of this link, or of another link to
    /// the same video, that waits or runs: Kamosu reading its page, or an
    /// agent reading it.
    fn being_added(&self, caller: &Caller, link: &str) -> Result<Option<String>, OpError> {
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT jobs.id FROM jobs
                   LEFT JOIN agent_jobs ON agent_jobs.job_id = jobs.id
                  WHERE jobs.person_id = ?2 AND jobs.status IN ('queued', 'running')
                    AND ((jobs.operation = 'import_web_link'
                          AND link_identity(json_extract(jobs.input, '$.url')) = ?1)
                         OR link_identity(agent_jobs.link) = ?1)
                  ORDER BY jobs.created_at
                  LIMIT 1",
                params![crate::video_link::link_identity(link), caller.person_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot look for the link being added: {e}")))
        })
    }

    /// Sent to the agent, which is reading it, or is not answering now.
    fn sent_to_agent(&self) -> Answer {
        let said = if self.agent_answering() {
            Said::SentToAgent
        } else {
            Said::AgentAway
        };
        Answer::new(Outcome::Sent, said)
    }

    /// Whether an agent is answering: a Runner Key stands whose Runner was
    /// heard from lately, by the agent tracker's rule.
    fn agent_answering(&self) -> bool {
        self.db()
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT EXISTS (SELECT 1 FROM runner_keys
                      WHERE revoked = 0
                        AND last_asked_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1))",
                    params![format!("-{} seconds", AGENT_LATELY.as_secs())],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read the Runner's state: {e}")))
            })
            .unwrap_or(false)
    }

    /// The Shortcut versions and a link's wait in place of the published
    /// ones. Compiled only under `test-jobs`: a test of an old Shortcut needs
    /// a version older than the one Kamosu publishes, and a test of the wait
    /// cannot wait ten seconds.
    #[cfg(feature = "test-jobs")]
    pub fn share_as_if(&self, current: u64, oldest: u64, wait: Duration) {
        self.sharing.current.store(current, Ordering::Relaxed);
        self.sharing.oldest.store(oldest, Ordering::Relaxed);
        self.sharing
            .wait_millis
            .store(wait.as_millis() as u64, Ordering::Relaxed);
    }
}

/// The refusal `import_web_link` asked with `refuse_unread` fails with where
/// a page refused Kamosu and there is no Runner.
pub const PAGE_REFUSED: &str = "page_refused";
/// The refusal `import_web_link` asked with `refuse_unread` fails with where
/// a page holds no recipe data and there is no Runner to read it.
pub const NOT_READ_ALONE: &str = "not_read_alone";

fn has_ended(job: &JobRecord) -> bool {
    matches!(
        job.status,
        jobs::JobStatus::Completed | jobs::JobStatus::Failed | jobs::JobStatus::Cancelled
    )
}

/// Text that is a link and nothing else, as an app that shares links as text
/// sends one.
fn only_a_link(text: &str) -> Option<&str> {
    let text = text.trim();
    let link = !text.is_empty()
        && !text.contains(char::is_whitespace)
        && (text.starts_with("https://") || text.starts_with("http://"));
    link.then_some(text)
}

/// What `add_this` says, in one Language.
struct Words {
    added: &'static str,
    sent_to_agent: &'static str,
    agent_away: &'static str,
    still_reading: &'static str,
    read_in_kamosu: &'static str,
    no_agent: &'static str,
    page_refused: &'static str,
    already_on_shelf: &'static str,
    being_added: &'static str,
    failed: &'static str,
    too_old: &'static str,
    newer_shortcut: &'static str,
}

impl Words {
    fn say(&self, said: &Said) -> String {
        match said {
            Said::Added(title) => self.added.replace("{title}", title),
            Said::SentToAgent => self.sent_to_agent.to_string(),
            Said::AgentAway => self.agent_away.to_string(),
            Said::StillReading => self.still_reading.to_string(),
            Said::ReadInKamosu => self.read_in_kamosu.to_string(),
            Said::NoAgent => self.no_agent.to_string(),
            Said::PageRefused => self.page_refused.to_string(),
            Said::AlreadyOnShelf(title) => self.already_on_shelf.replace("{title}", title),
            Said::BeingAdded => self.being_added.to_string(),
            Said::Failed(reason) => self.failed.replace("{reason}", reason),
            Said::TooOld => self.too_old.to_string(),
        }
    }
}

// The reader's Language, not the recipe's: this is said to the person who
// shared, who has told Kamosu what they read in.
const EN: Words = Words {
    added: "Added to your shelf: {title}",
    sent_to_agent: "Sent. Kamosu's AI agent is reading it, and the recipe will be on your \
                    shelf in a few minutes.",
    agent_away: "Sent. Kamosu's AI agent isn't answering right now, and will read it when \
                 it's back.",
    still_reading: "Sent. Kamosu is still reading it, and the recipe will be on your shelf \
                    once it has.",
    read_in_kamosu: "This Kamosu can't read that page on its own. Open Kamosu and add the \
                     link there, where you can check what it reads.",
    no_agent: "This Kamosu has no AI agent to read photos, text or PDFs. Add the recipe in \
               Kamosu by hand.",
    page_refused: "Kamosu couldn't open that page. Copy the recipe's text and share that \
                   instead.",
    already_on_shelf: "Already on your shelf: {title}",
    being_added: "Already being added. It will be on your shelf once it is read.",
    failed: "Kamosu couldn't add that: {reason}",
    too_old: "This Shortcut is too old for this Kamosu. Install the new one from Settings, \
              under Share from your phone.",
    newer_shortcut: "A newer Shortcut is in Settings, under Share from your phone.",
};

const FR: Words = Words {
    added: "Ajoutée à votre étagère : {title}",
    sent_to_agent: "Envoyé. L'agent IA de Kamosu la lit, et la recette sera sur votre \
                    étagère dans quelques minutes.",
    agent_away: "Envoyé. L'agent IA de Kamosu ne répond pas pour le moment, et la lira à \
                 son retour.",
    still_reading: "Envoyé. Kamosu la lit encore, et la recette sera sur votre étagère une \
                    fois lue.",
    read_in_kamosu: "Ce Kamosu ne sait pas lire cette page seul. Ouvrez Kamosu et ajoutez-y \
                     le lien, pour vérifier ce qu'il en lit.",
    no_agent: "Ce Kamosu n'a pas d'agent IA pour lire des photos, du texte ou des PDF. \
               Ajoutez la recette à la main dans Kamosu.",
    page_refused: "Kamosu n'a pas pu ouvrir cette page. Copiez le texte de la recette et \
                   partagez-le à la place.",
    already_on_shelf: "Déjà sur votre étagère : {title}",
    being_added: "Déjà en cours d'ajout. Elle sera sur votre étagère une fois lue.",
    failed: "Kamosu n'a pas pu l'ajouter : {reason}",
    too_old: "Ce raccourci est trop ancien pour ce Kamosu. Installez le nouveau depuis les \
              Réglages, sous Partager depuis votre téléphone.",
    newer_shortcut: "Un raccourci plus récent est dans les Réglages, sous Partager depuis \
                     votre téléphone.",
};

const ES: Words = Words {
    added: "Añadida a tu estante: {title}",
    sent_to_agent: "Enviado. El agente de IA de Kamosu la está leyendo, y la receta estará \
                    en tu estante en unos minutos.",
    agent_away: "Enviado. El agente de IA de Kamosu no responde ahora mismo, y la leerá \
                 cuando vuelva.",
    still_reading: "Enviado. Kamosu aún la está leyendo, y la receta estará en tu estante \
                    cuando termine.",
    read_in_kamosu: "Este Kamosu no puede leer esa página por sí solo. Abre Kamosu y añade \
                     ahí el enlace, donde puedes revisar lo que lee.",
    no_agent: "Este Kamosu no tiene un agente de IA para leer fotos, texto o PDF. Añade la \
               receta a mano en Kamosu.",
    page_refused: "Kamosu no pudo abrir esa página. Copia el texto de la receta y compártelo \
                   en su lugar.",
    already_on_shelf: "Ya está en tu estante: {title}",
    being_added: "Ya se está añadiendo. Estará en tu estante cuando se haya leído.",
    failed: "Kamosu no pudo añadirla: {reason}",
    too_old: "Este atajo es demasiado antiguo para este Kamosu. Instala el nuevo desde \
              Ajustes, en Compartir desde tu teléfono.",
    newer_shortcut: "Hay un atajo más nuevo en Ajustes, en Compartir desde tu teléfono.",
};

fn words(language: &str) -> &'static Words {
    match language {
        "fr" => &FR,
        "es" => &ES,
        _ => &EN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_that_is_only_a_link_is_that_link() {
        assert_eq!(
            only_a_link("  https://example.com/tart \n"),
            Some("https://example.com/tart")
        );
        assert_eq!(only_a_link("see https://example.com/tart"), None);
        assert_eq!(only_a_link("Lemon tart\n200 g flour"), None);
        assert_eq!(only_a_link("ftp://example.com"), None);
    }

    /// Every Language says every answer, and says the title it is given.
    #[test]
    fn every_message_is_in_every_language() {
        for language in ["en", "fr", "es"] {
            let said = words(language);
            assert!(said.say(&Said::Added("Tarte".into())).contains("Tarte"));
            assert!(
                said.say(&Said::AlreadyOnShelf("Tarte".into()))
                    .contains("Tarte")
            );
            assert!(said.say(&Said::Failed("nope".into())).contains("nope"));
        }
        assert_ne!(words("fr").no_agent, words("en").no_agent);
        assert_ne!(words("es").no_agent, words("en").no_agent);
    }
}
