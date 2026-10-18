//! The Runner (ADR 0048, #274): Kamosu's own program started in another mode,
//! `kamosu runner`, beside an agent that reads what Kamosu cannot.
//!
//! It asks one Kamosu for work and is never called: it opens every connection
//! itself and listens on no port. For each Job it makes a fresh folder, writes
//! down there what the Job is and its Job Key, and starts the agent command in
//! it. While the command runs it renews its hold. When the command ends it
//! reports how, with the report the agent left in the folder, and removes the
//! folder. It knows nothing of the kinds of Job: what Kamosu says about one
//! is passed on as Kamosu said it.
//!
//! It opens no database and needs no data directory. Its one Secret is the
//! Runner Key, taken from the environment and kept out of the agent's reach:
//! it is in no file of the folder, not in the command's environment, and
//! blanked in the Runner's own, so an agent a page has tricked holds a key to
//! its one Job and nothing more.
//!
//! What the agent is handed and what it leaves are a contract with whoever
//! writes an agent's instructions, set down on #274:
//!
//! - `job.json`, written before the command starts: `kamosu` is the address
//!   the Runner was given, `mcp` is Kamosu's MCP door there, and `job` is the
//!   Job as `ask_for_work` answered it, `job_key` included, less the two
//!   fields about the Runner's own hold.
//! - `report.json`, left by the agent: `outcome` is `completed`,
//!   `could_not_be_read` or `failed`, `reason` is one sentence for the last
//!   two, and `report` is the agent's own report as `report_on_job` takes it.
//!
//! An agent in a container of its own is handed its Job through a Unix socket
//! in place of a command (#317), so that the Runner Key is in another container
//! altogether. The agent listens and the Runner calls it, one call for each
//! Job, and that call is the whole of what the two say to each other:
//!
//! 1. The agent greets the Runner with one line, `{"kamosu_agent":1}`. The
//!    Runner holds the call from then on, and asks Kamosu for work only while
//!    an agent waits on it.
//! 2. The Runner sends one line, what `job.json` holds.
//! 3. The agent sends what `report.json` holds, and hangs up.
//!
//! The Runner hanging up first is how it stops the agent: past the time
//! limit, when Kamosu ends the hold, and when the Runner itself is stopped.
//! The agent is expected to take one call in its life and end with it, so that
//! a container restarted for the next Job holds nothing of the last. The
//! Runner waits `agent_rest` before calling again, which leaves an agent's
//! container time to end with everything the Job started.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::process::Command;
use url::Url;

/// The environment variable the Runner Key is read from. Never a command-line
/// argument: every process on the machine can read another's arguments.
pub const RUNNER_KEY_VAR: &str = "KAMOSU_RUNNER_KEY";

/// The variable Kamosu's address may be read from, in place of `--kamosu`.
/// Not `KAMOSU_ADDRESS`, which is the address a server listens on (#348).
pub const INSTANCE_VAR: &str = "KAMOSU_RUNNER_INSTANCE";

/// The variable the agent's socket may be read from, in place of
/// `--agent-socket`.
pub const AGENT_SOCKET_VAR: &str = "KAMOSU_RUNNER_AGENT_SOCKET";

/// What the Runner writes down for the agent, in the Job's folder.
pub const JOB_FILE: &str = "job.json";

/// What the agent leaves behind for the Runner, in the same folder.
pub const REPORT_FILE: &str = "report.json";

/// The line an agent at a socket greets the Runner with. The number is the
/// version of what the two say to each other.
pub const AGENT_GREETING: &str = r#"{"kamosu_agent":1}"#;

/// Whom the Runner hands each Job to.
#[derive(Clone, Debug)]
pub enum Agent {
    /// A command it starts in a fresh folder, with `job.json` there.
    Command(Vec<String>),
    /// An agent listening on this Unix socket, in a container of its own.
    Socket(PathBuf),
}

/// Every pace the Runner keeps, in one place.
#[derive(Clone, Debug)]
pub struct Pace {
    /// How long to wait before asking again when no Job waits.
    pub ask_every: Duration,
    /// How long to wait the first time Kamosu is out of reach. Each time after
    /// that waits twice as long, up to `out_of_reach_most`.
    pub out_of_reach_first: Duration,
    pub out_of_reach_most: Duration,
    /// How long an agent command may run before it is stopped, with
    /// everything it started, and its Job reported failed.
    ///
    /// Thirty minutes, chosen before any Job was timed (#274). The ADR expects
    /// an agent's Import to take minutes: a video adds a download, frames cut
    /// and read, and a check of the saved recipe against its source. Half an
    /// hour leaves several times that, and still stops an agent that is stuck
    /// in a loop before it runs up an evening's bill. #281 sets it again from
    /// measured runs.
    pub time_limit: Duration,
    /// How long a stopped command has to end by itself before it is killed.
    pub stop_grace: Duration,
    /// How long one call to Kamosu may take before it counts as out of reach.
    pub call_timeout: Duration,
    /// How long to wait before calling an agent's socket again when no agent
    /// answers there.
    pub agent_every: Duration,
    /// How long to wait after a Job before calling an agent's socket again,
    /// for the agent's container to end and start afresh.
    pub agent_rest: Duration,
}

impl Pace {
    pub const STANDARD: Pace = Pace {
        ask_every: Duration::from_secs(10),
        out_of_reach_first: Duration::from_secs(15),
        out_of_reach_most: Duration::from_secs(120),
        time_limit: Duration::from_secs(30 * 60),
        stop_grace: Duration::from_secs(10),
        call_timeout: Duration::from_secs(30),
        agent_every: Duration::from_secs(1),
        agent_rest: Duration::from_secs(2),
    };

    /// The pace of this build. A `test-jobs` build lets a test shorten the
    /// waits, since a test cannot wait half an hour for a time limit.
    fn of_this_build() -> Pace {
        #[allow(unused_mut)]
        let mut pace = Pace::STANDARD;
        #[cfg(feature = "test-jobs")]
        {
            let millis = |name: &str| {
                std::env::var(name)
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .map(Duration::from_millis)
            };
            if let Some(every) = millis("KAMOSU_RUNNER_TEST_ASK_EVERY_MS") {
                pace.ask_every = every;
                pace.out_of_reach_first = every;
                pace.out_of_reach_most = every * 4;
                pace.agent_every = every;
                pace.agent_rest = every;
            }
            if let Some(limit) = millis("KAMOSU_RUNNER_TEST_TIME_LIMIT_MS") {
                pace.time_limit = limit;
            }
            if let Some(grace) = millis("KAMOSU_RUNNER_TEST_STOP_GRACE_MS") {
                pace.stop_grace = grace;
            }
        }
        pace
    }
}

/// The version the Runner states when it asks. It is the program's own, and
/// a `test-jobs` build can be made to state another, so a test can see a
/// Kamosu refuse it.
fn stated_version() -> String {
    #[cfg(feature = "test-jobs")]
    if let Ok(version) = std::env::var("KAMOSU_RUNNER_TEST_VERSION") {
        return version;
    }
    env!("CARGO_PKG_VERSION").to_string()
}

/// What the Runner was told on its command line. A flag left out is read from
/// its variable, and an agent command is only ever a command line.
pub struct Told {
    pub kamosu: Option<String>,
    pub agent_socket: Option<PathBuf>,
    pub command: Vec<String>,
}

/// A variable's value, or nothing when it is unset or holds only spaces, so
/// an empty line in a settings file counts as no setting. Not clap's `env`,
/// which hands an empty value on as a value given.
fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Kamosu's address and the agent, each from its flag or else its variable.
fn settings(told: Told) -> Result<(String, Agent), String> {
    let kamosu = told
        .kamosu
        .or_else(|| setting(INSTANCE_VAR))
        .ok_or(format!(
            "Give Kamosu's address, as --kamosu or in {INSTANCE_VAR}."
        ))?;
    let socket = told
        .agent_socket
        .or_else(|| setting(AGENT_SOCKET_VAR).map(PathBuf::from));
    let agent = match (socket, told.command.is_empty()) {
        (Some(_), false) => {
            return Err(format!(
                "{AGENT_SOCKET_VAR} names an agent's socket, and an agent command is \
                 given after `--`. Give one of the two."
            ));
        }
        (Some(socket), true) => Agent::Socket(socket),
        (None, false) => Agent::Command(told.command),
        (None, true) => {
            return Err(format!(
                "Name the agent: a command to start for each Job, after `--`, or the \
                 socket it listens on, as --agent-socket or in {AGENT_SOCKET_VAR}."
            ));
        }
    };
    Ok((kamosu, agent))
}

/// Run the Runner until it is stopped, or until Kamosu refuses it in a way
/// asking again cannot mend. Answers the exit code.
pub fn run(told: Told) -> i32 {
    let (kamosu, agent) = match settings(told) {
        Ok(settings) => settings,
        Err(why) => {
            eprintln!("{why}");
            return 2;
        }
    };
    let kamosu = match kamosu_address(&kamosu) {
        Ok(kamosu) => kamosu,
        Err(why) => {
            eprintln!("{why}");
            return 2;
        }
    };
    let runner_key = match std::env::var(RUNNER_KEY_VAR) {
        Ok(key) if !key.trim().is_empty() => key.trim().to_string(),
        _ => {
            eprintln!(
                "Set {RUNNER_KEY_VAR} to the Runner Key an Operator minted. The Runner \
                 takes it from the environment, never from the command line, where \
                 every process on the machine can read it."
            );
            return 2;
        }
    };
    scrub_from_environment(RUNNER_KEY_VAR);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        let runner = Runner::new(kamosu, runner_key, agent, Pace::of_this_build());
        tokio::select! {
            code = runner.run() => code,
            _ = stop_signal() => {
                runner.stop_the_job_being_worked().await;
                eprintln!(
                    "Runner stopped. A Job it was working waits again once its hold lapses."
                );
                0
            }
        }
    })
}

/// The address the Runner was given, as the base every call is made under.
fn kamosu_address(given: &str) -> Result<Url, String> {
    let mut address = Url::parse(given.trim())
        .map_err(|e| format!("`{given}` is not an address the Runner can ask: {e}"))?;
    if !address.username().is_empty() || address.password().is_some() {
        // The agent is handed this address, so it must hold no Secret. Nor
        // does the refusal, which lands in the Runner's log.
        let _ = address.set_username("");
        let _ = address.set_password(None);
        return Err(format!(
            "`{address}` was given with a name or a password. Give Kamosu's address \
             without one: the Runner Key is how the Runner signs in"
        ));
    }
    if !matches!(address.scheme(), "http" | "https") || address.host().is_none() {
        return Err(format!(
            "`{given}` is not an address the Runner can ask. Give Kamosu's own address, \
             such as https://recipes.kamosu.example"
        ));
    }
    if !address.path().ends_with('/') {
        let path = format!("{}/", address.path());
        address.set_path(&path);
    }
    address.set_query(None);
    address.set_fragment(None);
    Ok(address)
}

/// Overwrite a variable's value where this process's environment holds it.
///
/// Leaving it out of the agent command's environment is not enough. Linux
/// shows a process's environment as it started, at /proc/<pid>/environ, to
/// every process of the same user, and the agent runs as the Runner's user. So
/// the Runner overwrites the value in place, once it has read it, and the agent
/// reads an empty one there. Call it before any other thread starts: nothing
/// may read the environment while it is being written.
fn scrub_from_environment(name: &str) {
    unsafe extern "C" {
        // POSIX's own, in glibc and musl alike.
        static mut environ: *mut *mut libc::c_char;
    }
    let prefix = format!("{name}=");
    // SAFETY: `environ` is the null-terminated list of NUL-terminated strings
    // the process started with, and only this thread runs yet. Each byte
    // written lies inside a string the loop has just measured, and the value
    // keeps its length, ending at the NUL it already had.
    unsafe {
        let mut entry = environ;
        while !entry.is_null() && !(*entry).is_null() {
            let text = std::ffi::CStr::from_ptr(*entry).to_bytes();
            if text.starts_with(prefix.as_bytes()) {
                let value = (*entry).add(prefix.len());
                std::ptr::write_bytes(value, 0, text.len() - prefix.len());
            }
            entry = entry.add(1);
        }
    }
}

async fn stop_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = term.recv() => {},
    }
}

/// How one call to Kamosu went.
enum Answer {
    /// Kamosu did it, and this is its result.
    Done(Value),
    /// Kamosu refused it: asking the same again gets the same answer.
    Refused {
        status: u16,
        message: String,
        reason: Option<String>,
    },
    /// Kamosu answered somewhere else. The Runner follows no redirect.
    Redirected(String),
    /// No answer from Kamosu: it is down, or something in front of it is.
    OutOfReach(String),
}

/// How the Runner's hold on a Job ended, with the agent's own end inside.
enum Held<T> {
    /// The agent ended by itself, this way.
    Ended(T),
    /// It ran past the time limit.
    TimedOut,
    /// Kamosu ended the hold: the Job was cancelled, or the hold lapsed.
    Released,
}

/// How an agent command's time with its Job ended.
enum Ended {
    /// It ended by itself.
    Exited(std::process::ExitStatus),
    /// It ran past the time limit and was stopped.
    TimedOut,
    /// Kamosu ended the hold: the Job was cancelled, or the hold lapsed.
    Released,
    /// The Runner could not wait on it.
    Lost(String),
}

/// The Job being worked, so that stopping the Runner stops its agent too.
struct Working {
    group: i32,
    folder: PathBuf,
}

struct Runner {
    kamosu: Url,
    runner_key: String,
    agent: Agent,
    pace: Pace,
    /// The Runner's own client. It speaks to the one address the Runner was
    /// given, follows no redirect and goes through no proxy. It is kept apart
    /// from the guarded client every other fetch in Kamosu uses (ADR 0033),
    /// which would refuse the private address a Runner is often given, and
    /// nothing outside this file can reach it.
    client: reqwest::Client,
    working: Arc<Mutex<Option<Working>>>,
}

impl Runner {
    fn new(kamosu: Url, runner_key: String, agent: Agent, pace: Pace) -> Runner {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(pace.call_timeout)
            .build()
            .expect("the Runner's client");
        Runner {
            kamosu,
            runner_key,
            agent,
            pace,
            client,
            working: Arc::new(Mutex::new(None)),
        }
    }

    /// Ask for work, do it, and ask again.
    async fn run(&self) -> i32 {
        let version = stated_version();
        eprintln!(
            "Kamosu Runner {version} asking {} for work.",
            self.kamosu.as_str()
        );
        let mut ask = fresh_mark();
        let mut away = Away::default();
        // The call an agent at a socket waits on. A Job is asked for only
        // while one does, so none is taken that no agent can work.
        let mut call: Option<AgentCall> = None;
        loop {
            if let Agent::Socket(socket) = &self.agent
                && !call.as_mut().is_some_and(AgentCall::still_there)
            {
                call = Some(self.reach_agent(socket).await);
            }
            let asked = self
                .call(
                    "ask_for_work",
                    json!({ "runner_version": version, "ask": ask }),
                )
                .await;
            match asked {
                Answer::Done(answer) => {
                    away.back(&self.kamosu);
                    // The next ask is a new one. An ask whose reply never
                    // arrived is repeated under the same mark, below.
                    ask = fresh_mark();
                    match answer.get("job").filter(|job| !job.is_null()) {
                        Some(job) => {
                            self.work(job, call.take()).await;
                            if matches!(self.agent, Agent::Socket(_)) {
                                tokio::time::sleep(self.pace.agent_rest).await;
                            }
                        }
                        None => tokio::time::sleep(self.pace.ask_every).await,
                    }
                }
                Answer::OutOfReach(why) => {
                    let wait = away.wait(&self.pace);
                    eprintln!(
                        "Kamosu at {} is out of reach: {why}. Asking again in {} seconds.",
                        self.kamosu.as_str(),
                        wait.as_secs()
                    );
                    tokio::time::sleep(wait).await;
                }
                Answer::Refused {
                    status: 401,
                    message,
                    ..
                } => {
                    eprintln!(
                        "Kamosu does not know this Runner Key: it was revoked, or it was \
                         never minted there ({message}). An Operator mints a new one."
                    );
                    return 1;
                }
                Answer::Refused {
                    reason, message, ..
                } if reason.as_deref() == Some("another_version") => {
                    eprintln!("Kamosu refused this Runner for its version: {message}.");
                    return 1;
                }
                Answer::Refused { message, .. } => {
                    eprintln!("Kamosu refused to hand this Runner work: {message}.");
                    return 1;
                }
                Answer::Redirected(to) => {
                    eprintln!(
                        "Kamosu at {} answered with a redirect to {to}. The Runner speaks \
                         only to the address it was given and follows no redirect: give it \
                         Kamosu's own address.",
                        self.kamosu.as_str()
                    );
                    return 1;
                }
            }
        }
    }

    /// Do one Job: hand it to the agent waiting on `line`, or else to the
    /// agent command in a fresh folder, and report how it ended.
    async fn work(&self, job: &Value, call: Option<AgentCall>) {
        let Some(job_id) = job["job_id"].as_str() else {
            eprintln!("Kamosu handed out a Job with no id: {job}");
            return;
        };
        eprintln!("Working Job {job_id}.");
        let report = match (call, &self.agent) {
            (Some(call), _) => self.hand_over_by_socket(job_id, job, call).await,
            (None, Agent::Command(command)) => match fresh_folder() {
                Ok(folder) => {
                    let report = self.hand_over(job_id, job, command, &folder).await;
                    remove_folder(&folder);
                    report
                }
                Err(e) => Some(Report::Failed(format!(
                    "the Runner could not make a folder for the agent: {e}"
                ))),
            },
            (None, Agent::Socket(_)) => Some(Report::Failed(
                "the Runner had no agent to hand the Job to".to_string(),
            )),
        };
        if let Some(report) = report {
            self.report(job_id, report).await;
        }
    }

    /// Start the agent command in `folder` with the Job written down there,
    /// keep the hold while it runs, and answer what to report. None when
    /// Kamosu ended the hold, which leaves the Runner nothing to report.
    async fn hand_over(
        &self,
        job_id: &str,
        job: &Value,
        command: &[String],
        folder: &Path,
    ) -> Option<Report> {
        if let Err(e) = write_job_file(folder, &self.kamosu, job) {
            return Some(Report::Failed(format!(
                "the Runner could not write the Job down for the agent: {e}"
            )));
        }
        let mut child = match Command::new(&command[0])
            .args(&command[1..])
            .current_dir(folder)
            .env_remove(RUNNER_KEY_VAR)
            .stdin(std::process::Stdio::null())
            // Its own process group, so that stopping it stops everything
            // it started.
            .process_group(0)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                return Some(Report::Failed(format!(
                    "the agent command could not be started: {e}"
                )));
            }
        };
        let group = child.id().map(|pid| pid as i32).unwrap_or_default();
        *self.working.lock().unwrap() = Some(Working {
            group,
            folder: folder.to_path_buf(),
        });

        let ended = match self.hold_while(job_id, job, child.wait()).await {
            Held::Ended(Ok(status)) => Ended::Exited(status),
            Held::Ended(Err(e)) => Ended::Lost(e.to_string()),
            Held::TimedOut => {
                stop_group(group, &mut child, self.pace.stop_grace).await;
                Ended::TimedOut
            }
            Held::Released => {
                stop_group(group, &mut child, self.pace.stop_grace).await;
                Ended::Released
            }
        };
        // Whatever the command left running is stopped with it, before its
        // report is read, so nothing it started can still be writing there.
        kill_group(group, libc::SIGKILL);
        *self.working.lock().unwrap() = None;

        match ended {
            Ended::Exited(status) if !status.success() => Some(Report::Failed(format!(
                "the agent command ended in an error ({status})"
            ))),
            Ended::Exited(_) => Some(read_report(folder)),
            Ended::TimedOut => Some(self.past_the_time_limit()),
            Ended::Lost(why) => Some(Report::Failed(format!(
                "the Runner lost track of the agent command: {why}"
            ))),
            Ended::Released => None,
        }
    }

    /// Hand the Job to the agent waiting on `line`, keep the hold until the
    /// agent's report arrives, and answer what to report. None when Kamosu
    /// ended the hold. However the Job ends, the Runner hangs up, which is the
    /// agent's sign to stop.
    async fn hand_over_by_socket(
        &self,
        job_id: &str,
        job: &Value,
        mut call: AgentCall,
    ) -> Option<Report> {
        let mut sent = serde_json::to_vec(&handed_job(&self.kamosu, job)).expect("JSON");
        sent.push(b'\n');
        let written =
            tokio::time::timeout(self.pace.call_timeout, call.stream.write_all(&sent)).await;
        if let Some(why) = match written {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e.to_string()),
            Err(_) => Some("it took nothing in".to_string()),
        } {
            return Some(Report::Failed(format!(
                "the Runner could not hand the Job to the agent: {why}"
            )));
        }
        let held = self
            .hold_while(job_id, job, report_from_call(&mut call.stream))
            .await;
        drop(call);
        match held {
            Held::Ended(report) => Some(report),
            Held::TimedOut => Some(self.past_the_time_limit()),
            Held::Released => None,
        }
    }

    fn past_the_time_limit(&self) -> Report {
        Report::Failed(format!(
            "the agent ran past the Runner's time limit of {} and was stopped",
            spoken(self.pace.time_limit)
        ))
    }

    /// Wait for `ending`, the agent's end, renewing the hold on the Job
    /// meanwhile, until it ends, runs past the time limit or Kamosu ends the
    /// hold.
    async fn hold_while<T>(
        &self,
        job_id: &str,
        job: &Value,
        ending: impl std::future::Future<Output = T>,
    ) -> Held<T> {
        tokio::pin!(ending);
        let time_limit = tokio::time::Instant::now() + self.pace.time_limit;
        let mut renew_every = renew_within(job);
        let mut next_renewal = tokio::time::Instant::now() + renew_every;
        loop {
            tokio::select! {
                ended = &mut ending => return Held::Ended(ended),
                _ = tokio::time::sleep_until(time_limit) => return Held::TimedOut,
                _ = tokio::time::sleep_until(next_renewal) => {
                    // A renewal may take half the time between two, so that
                    // one that hangs leaves time for more before the hold
                    // lapses. Kamosu gives a hold three of those times.
                    let renewed = self
                        .call_within("renew_hold", json!({ "job_id": job_id }), renew_every / 2)
                        .await;
                    next_renewal = match renewed {
                        Answer::Done(renewed) => {
                            renew_every = renew_within(&renewed);
                            tokio::time::Instant::now() + renew_every
                        }
                        Answer::Refused { message, .. } => {
                            eprintln!(
                                "Kamosu ended this Runner's hold on Job {job_id}: {message}. \
                                 Stopping the agent."
                            );
                            return Held::Released;
                        }
                        Answer::OutOfReach(why) | Answer::Redirected(why) => {
                            eprintln!(
                                "Could not renew the hold on Job {job_id}: {why}. The agent \
                                 keeps working, and the Runner tries again soon."
                            );
                            // Soon, not a whole period later: two misses in
                            // a row would otherwise lose the hold.
                            tokio::time::Instant::now() + renew_every / 4
                        }
                    };
                }
            }
        }
    }

    /// Call the agent at `socket` until one answers with its greeting, and
    /// answer the call it waits on. Says once why none does, each time the
    /// reason changes.
    async fn reach_agent(&self, socket: &Path) -> AgentCall {
        let mut said: Option<String> = None;
        loop {
            let why = match UnixStream::connect(socket).await {
                Ok(stream) => match AgentCall::greeted(stream, self.pace.call_timeout).await {
                    Ok(call) => {
                        if said.is_some() {
                            eprintln!("The agent at {} answers.", socket.display());
                        }
                        return call;
                    }
                    Err(why) => why,
                },
                Err(e) => e.to_string(),
            };
            if said.as_deref() != Some(&why) {
                eprintln!(
                    "Waiting for the agent at {}: {why}. Kamosu is asked for no Job \
                     until it answers.",
                    socket.display()
                );
                said = Some(why);
            }
            tokio::time::sleep(self.pace.agent_every).await;
        }
    }

    /// Say how a Job ended. Should Kamosu refuse what the agent's own report
    /// says, the Job is reported failed instead, with Kamosu's reason.
    async fn report(&self, job_id: &str, report: Report) {
        let from_the_agent = matches!(report, Report::FromTheAgent(_));
        let mut sent = match report {
            Report::FromTheAgent(report) => report,
            Report::Failed(reason) => json!({ "outcome": "failed", "reason": reason }),
        };
        sent["job_id"] = json!(job_id);
        let outcome = sent["outcome"].as_str().unwrap_or("?").to_string();
        let mut away = Away::default();
        loop {
            match self.call("report_on_job", sent.clone()).await {
                Answer::Done(_) => {
                    eprintln!("Reported Job {job_id} as {outcome}.");
                    return;
                }
                Answer::Refused {
                    status: 400,
                    message,
                    ..
                } if from_the_agent => {
                    // Kamosu's sentence can quote what the agent wrote, so it
                    // is cut short to stay a reason Kamosu takes.
                    let message: String = message.chars().take(REASON_MOST_CHARS).collect();
                    let reason = format!("Kamosu refused the agent's report: {message}");
                    eprintln!("Job {job_id}: {reason}.");
                    return Box::pin(self.report(job_id, Report::Failed(reason))).await;
                }
                Answer::Refused { message, .. } => {
                    eprintln!("Kamosu refused the report on Job {job_id}: {message}.");
                    return;
                }
                Answer::OutOfReach(why) | Answer::Redirected(why) => {
                    let wait = away.wait(&self.pace);
                    eprintln!(
                        "Could not report on Job {job_id}: {why}. Trying again in {} seconds.",
                        wait.as_secs()
                    );
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    /// Stop the agent of the Job being worked, if one is, and remove its
    /// folder. For when the Runner itself is stopped.
    async fn stop_the_job_being_worked(&self) {
        // An agent at a socket is stopped by the Runner hanging up, which
        // its ending does.
        let Some(working) = self.working.lock().unwrap().take() else {
            return;
        };
        kill_group(working.group, libc::SIGTERM);
        tokio::time::sleep(self.pace.stop_grace.min(Duration::from_secs(2))).await;
        kill_group(working.group, libc::SIGKILL);
        remove_folder(&working.folder);
    }

    /// Call one Operation at Kamosu's web door with the Runner Key.
    async fn call(&self, operation: &str, input: Value) -> Answer {
        self.call_within(operation, input, self.pace.call_timeout)
            .await
    }

    /// The same, giving up after `timeout`.
    async fn call_within(&self, operation: &str, input: Value, timeout: Duration) -> Answer {
        let address = self
            .kamosu
            .join(&format!("api/op/{operation}"))
            .expect("an Operation's address");
        let sent = self
            .client
            .post(address)
            .bearer_auth(&self.runner_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(input.to_string())
            .timeout(timeout)
            .send()
            .await;
        let response = match sent {
            Ok(response) => response,
            Err(e) => return Answer::OutOfReach(plain_error(&e)),
        };
        let status = response.status();
        if status.is_redirection() {
            let to = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|to| to.to_str().ok())
                .unwrap_or("an address it did not say")
                .to_string();
            return Answer::Redirected(to);
        }
        let body: Value = match response.text().await {
            Ok(text) => serde_json::from_str(&text).unwrap_or(Value::Null),
            Err(e) => return Answer::OutOfReach(plain_error(&e)),
        };
        if status.is_success() {
            return match body.get("result") {
                Some(result) => Answer::Done(result.clone()),
                None => Answer::OutOfReach(format!(
                    "the answer at {} was not Kamosu's",
                    self.kamosu.as_str()
                )),
            };
        }
        let message = body["error"]["message"]
            .as_str()
            .map(|m| m.trim_end_matches('.').to_string())
            .unwrap_or_else(|| format!("it answered {status}"));
        // Too many requests is a proxy in front of Kamosu asking for patience.
        let refused = status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS;
        if refused && body["error"].is_object() {
            Answer::Refused {
                status: status.as_u16(),
                message,
                reason: body["error"]["reason"].as_str().map(str::to_string),
            }
        } else {
            Answer::OutOfReach(message)
        }
    }
}

/// How long Kamosu said to leave before renewing, with a floor so that a
/// strange answer cannot spin the Runner.
fn renew_within(answer: &Value) -> Duration {
    let seconds = answer["renew_within_seconds"].as_f64().unwrap_or(30.0);
    Duration::from_secs_f64(seconds.max(0.05))
}

/// Waiting longer each time Kamosu stays out of reach.
#[derive(Default)]
struct Away {
    last: Option<Duration>,
}

impl Away {
    fn wait(&mut self, pace: &Pace) -> Duration {
        let wait = match self.last {
            None => pace.out_of_reach_first,
            Some(last) => (last * 2).min(pace.out_of_reach_most),
        };
        self.last = Some(wait);
        wait
    }

    /// Kamosu answered again. Say so if it had been away.
    fn back(&mut self, kamosu: &Url) {
        if self.last.take().is_some() {
            eprintln!("Kamosu at {} answers again.", kamosu.as_str());
        }
    }
}

/// How a Job is reported.
enum Report {
    /// What the agent left in `report.json`, passed on as it wrote it.
    FromTheAgent(Value),
    /// Failed, for a reason the Runner gives.
    Failed(String),
}

/// The most of Kamosu's refusal a reason quotes. Kamosu takes a reason of up
/// to 2,000 characters.
const REASON_MOST_CHARS: usize = 500;

/// The most of `report.json` the Runner reads. Kamosu's own limits on a
/// report, 100 entries of 2,000 characters in each of four lists, come to less.
const REPORT_MOST_BYTES: u64 = 4 * 1024 * 1024;

/// What the agent left in the folder. No report, or one that is not a JSON
/// object in a plain file, fails the Job. The agent can leave anything there,
/// so a link, a pipe or a file too big is never read.
fn read_report(folder: &Path) -> Report {
    let path = folder.join(REPORT_FILE);
    let unreadable =
        |e: std::io::Error| Report::Failed(format!("the agent's report could not be read: {e}"));
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Report::Failed("the agent left no report".to_string());
        }
        Err(e) => return unreadable(e),
        Ok(found) if !found.file_type().is_file() => {
            return Report::Failed("the agent's report is not a plain file".to_string());
        }
        Ok(found) if found.len() > REPORT_MOST_BYTES => {
            return Report::Failed(format!(
                "the agent's report is larger than {} MB",
                REPORT_MOST_BYTES / 1024 / 1024
            ));
        }
        Ok(_) => {}
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => report_from_text(&text),
        Err(e) => unreadable(e),
    }
}

/// The report an agent at a socket sends before it hangs up. Nothing sent
/// fails the Job, as no report.json does, and so does more than a report holds.
async fn report_from_call(stream: &mut UnixStream) -> Report {
    let mut sent = Vec::new();
    if let Err(e) = (&mut *stream)
        .take(REPORT_MOST_BYTES + 1)
        .read_to_end(&mut sent)
        .await
    {
        return Report::Failed(format!("the agent's report could not be read: {e}"));
    }
    if sent.len() as u64 > REPORT_MOST_BYTES {
        return Report::Failed(format!(
            "the agent's report is larger than {} MB",
            REPORT_MOST_BYTES / 1024 / 1024
        ));
    }
    if sent.iter().all(u8::is_ascii_whitespace) {
        return Report::Failed("the agent left no report".to_string());
    }
    match String::from_utf8(sent) {
        Ok(text) => report_from_text(&text),
        Err(e) => Report::Failed(format!("the agent's report is not JSON: {e}")),
    }
}

/// A report as the agent wrote it, which must be a JSON object.
fn report_from_text(text: &str) -> Report {
    match serde_json::from_str::<Value>(text) {
        Ok(report) if report.is_object() => Report::FromTheAgent(report),
        Ok(_) => Report::Failed("the agent's report is not a JSON object".to_string()),
        Err(e) => Report::Failed(format!("the agent's report is not JSON: {e}")),
    }
}

/// A call to an agent at a socket, from its greeting until its Job ends.
struct AgentCall {
    stream: UnixStream,
}

impl AgentCall {
    /// The call, once the agent has greeted the Runner as one. Anything
    /// else at the socket is not an agent the Runner hands a Job to.
    async fn greeted(mut stream: UnixStream, within: Duration) -> Result<AgentCall, String> {
        let mut greeting = Vec::new();
        let read = tokio::time::timeout(within, async {
            loop {
                let byte = stream.read_u8().await?;
                if byte == b'\n' || greeting.len() >= 256 {
                    return Ok::<(), std::io::Error>(());
                }
                greeting.push(byte);
            }
        })
        .await;
        match read {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(format!("it hung up without a greeting ({e})")),
            Err(_) => return Err("it gave no greeting".to_string()),
        }
        let expected: Value = serde_json::from_str(AGENT_GREETING).expect("the greeting");
        match serde_json::from_slice::<Value>(&greeting) {
            Ok(greeting) if greeting == expected => Ok(AgentCall { stream }),
            _ => Err(format!(
                "it greeted the Runner with something other than {AGENT_GREETING}"
            )),
        }
    }

    /// Whether the agent still waits on the call. It sends nothing between
    /// its greeting and its report, so anything to read means it hung up or
    /// is no agent the Runner should trust with a Job.
    fn still_there(&mut self) -> bool {
        let mut byte = [0u8; 1];
        matches!(
            self.stream.try_read(&mut byte),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
        )
    }
}

/// A mark for one ask, made up fresh for each.
fn fresh_mark() -> String {
    hex::encode(crate::core::random_bytes(16))
}

/// A new folder for one Job, readable by this user alone.
fn fresh_folder() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let folder = std::env::temp_dir().join(format!("kamosu-job-{}", fresh_mark()));
    std::fs::DirBuilder::new().mode(0o700).create(&folder)?;
    Ok(folder)
}

/// Write down what the Job is for the agent command.
fn write_job_file(folder: &Path, kamosu: &Url, job: &Value) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(&handed_job(kamosu, job)).expect("JSON");
    std::fs::write(folder.join(JOB_FILE), text)
}

/// What `job.json` holds, which an agent at a socket is sent as one line.
fn handed_job(kamosu: &Url, job: &Value) -> Value {
    let mut job = job.clone();
    if let Some(job) = job.as_object_mut() {
        job.remove("held_until");
        job.remove("renew_within_seconds");
    }
    json!({
        "kamosu": kamosu.as_str(),
        "mcp": kamosu.join("mcp").expect("the MCP door's address").as_str(),
        "job": job,
    })
}

fn remove_folder(folder: &Path) {
    if let Err(e) = std::fs::remove_dir_all(folder) {
        eprintln!(
            "Could not remove the Job's folder {}: {e}",
            folder.display()
        );
    }
}

/// Stop a command and everything it started: ask, give it `grace` to end,
/// then kill what is left.
async fn stop_group(group: i32, child: &mut tokio::process::Child, grace: Duration) {
    kill_group(group, libc::SIGTERM);
    let _ = tokio::time::timeout(grace, child.wait()).await;
    kill_group(group, libc::SIGKILL);
    let _ = child.wait().await;
}

/// Send a signal to every process in a group. A group already gone is fine.
fn kill_group(group: i32, signal: i32) {
    if group > 0 {
        // SAFETY: kill(2) takes plain integers and touches no memory of ours.
        // A negative pid names the process group the agent command leads.
        unsafe {
            libc::kill(-group, signal);
        }
    }
}

/// A duration as a person says it: "30 minutes", or seconds below one.
fn spoken(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds >= 60.0 {
        let minutes = (seconds / 60.0).round() as u64;
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{seconds:.1} seconds")
    }
}

/// A connection error in a sentence, with its cause: reqwest's own words for
/// one stop at "error sending request".
fn plain_error(error: &reqwest::Error) -> String {
    use std::error::Error;
    let mut words = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        words.push_str(": ");
        words.push_str(&inner.to_string());
        cause = inner.source();
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_given_is_the_base_of_every_call() {
        let kamosu = kamosu_address("https://recipes.kamosu.example").unwrap();
        assert_eq!(
            kamosu.join("api/op/ask_for_work").unwrap().as_str(),
            "https://recipes.kamosu.example/api/op/ask_for_work"
        );
        let below = kamosu_address("http://192.0.2.10:5266/kamosu?x=1").unwrap();
        assert_eq!(
            below.join("mcp").unwrap().as_str(),
            "http://192.0.2.10:5266/kamosu/mcp"
        );
        assert!(kamosu_address("ftp://192.0.2.10").is_err());
        assert!(kamosu_address("recipes.kamosu.example").is_err());
        assert!(kamosu_address("https://cook:secret@recipes.kamosu.example").is_err());
    }

    #[test]
    fn out_of_reach_waits_longer_each_time_up_to_a_most() {
        let mut away = Away::default();
        let pace = Pace::STANDARD;
        let waits: Vec<u64> = (0..5).map(|_| away.wait(&pace).as_secs()).collect();
        assert_eq!(waits, [15, 30, 60, 120, 120]);
    }

    #[test]
    fn the_time_limit_is_spoken_in_minutes() {
        assert_eq!(spoken(Pace::STANDARD.time_limit), "30 minutes");
        assert_eq!(spoken(Duration::from_millis(1500)), "1.5 seconds");
    }
}
