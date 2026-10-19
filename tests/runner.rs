//! The Runner, started as the real program against a real Kamosu (#274).
//!
//! Every test here starts `kamosu runner` as its own process, beside a Kamosu
//! the test serves, with its agent command set to a stand-in: a small script
//! that reads the Job it was handed, does what the pasted text tells it, and
//! writes down what it saw for the test to read. No model is called.
//!
//! The stand-in is Python, which `just test` needs on the path beside
//! `sqlite3` (AGENTS.md).
//!
//! An agent in a container of its own is played by a second stand-in, which
//! listens on a Unix socket, serves one call and ends, and is started again
//! as its container would be (#317).
//!
//! It needs a `test-jobs` build, in which a test may shorten the Runner's
//! paces and its hold, so it is compiled in no other.
#![cfg(feature = "test-jobs")]

mod support;

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use url::Url;

/// The stand-in agent. The pasted text it fetches as its Original says which
/// thing it does, so that one Runner can be handed several kinds of Job:
///
/// - `writes`: creates the recipe with its Job Key, after waiting
///   `STAND_IN_SLEEP_S`, and leaves a report saying it completed.
/// - `errors`: ends in an error. `silent`: ends with no report. `pipe`: leaves
///   a pipe as its report, which nothing will ever write to.
/// - `lingers`: starts a child that would sleep for minutes, and sleeps too.
/// - `asks-the-model`: sends one request through the Runner's model relay
///   with the stand-in key it was handed and one with another key, notes what
///   each was answered, and completes.
const STAND_IN: &str = r#"#!/usr/bin/env python3
import json, os, subprocess, sys, time, urllib.error, urllib.parse, urllib.request

handed = json.load(open("job.json"))
job = handed["job"]
key = job["job_key"]
seen = os.path.join(os.environ["STAND_IN_SEEN"], job["job_id"])
os.makedirs(seen, exist_ok=True)

def note(name, text):
    with open(os.path.join(seen, name), "w") as f:
        f.write(text)

def ask(path, body=None):
    request = urllib.request.Request(
        urllib.parse.urljoin(handed["kamosu"], path),
        data=None if body is None else json.dumps(body).encode(),
        headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"},
    )
    return urllib.request.urlopen(request).read().decode()

note("cwd", os.getcwd())
note("pid", str(os.getpid()))
note("environ.json", json.dumps(dict(os.environ)))
for name in os.listdir("."):
    with open(name) as f:
        note("folder." + name, f.read())

mode = ask(job["original"]["address"]).split(":", 1)[1].strip()

if mode == "writes":
    time.sleep(float(os.environ.get("STAND_IN_SLEEP_S", "0")))
    ask("api/op/create_recipe", {"title": "The stand-in's soup"})
    with open("report.json", "w") as f:
        json.dump({"outcome": "completed",
                   "report": {"unsure": ["The stand-in guessed the salt."]}}, f)
    note("done", "")
elif mode == "errors":
    sys.exit(3)
elif mode == "silent":
    note("done", "")
elif mode == "pipe":
    os.mkfifo("report.json")
elif mode == "asks-the-model":
    relay = handed["model_relay"]
    def model(key):
        request = urllib.request.Request(
            relay["address"] + "chat/completions?stream=true", data=b'{"model": "m"}',
            headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(request) as answer:
                first = answer.readline()
                waited = time.monotonic()
                rest = answer.read()
                note("model.waited", str(time.monotonic() - waited))
                return [answer.status, (first + rest).decode()]
        except urllib.error.HTTPError as e:
            return [e.code, e.read().decode()]
    note("model.address", relay["address"])
    note("model.right", json.dumps(model(relay["key"])))
    note("model.wrong", json.dumps(model("kamosu-relay-of-another-job")))
    ask("api/op/create_recipe", {"title": "The model's soup"})
    with open("report.json", "w") as f:
        json.dump({"outcome": "completed"}, f)
elif mode == "lingers":
    child = subprocess.Popen(["sleep", "300"])
    note("child.pid", str(child.pid))
    note("started", "")
    time.sleep(300)
"#;

/// The stand-in for an agent in a container of its own: it listens on the
/// socket it is given, greets the Runner, takes one Job and ends, as the
/// agent's container does. The pasted text says what it does:
///
/// - `writes`: creates the recipe with its Job Key and sends a report.
/// - `silent`: hangs up with no report. `garbage`: sends a report that is
///   not JSON.
/// - `lingers`: waits for the Runner to hang up, and notes that it did.
/// - `asks-the-model`: sends one request through the Runner's model relay
///   socket with the stand-in key, in `x-api-key` as Anthropic's clients do,
///   and one with another key, notes what each was answered, and completes.
///
/// It notes what it was sent, and its pid, under `STAND_IN_SEEN`.
const SOCKET_STAND_IN: &str = r#"#!/usr/bin/env python3
import http.client, json, os, socket, sys, urllib.parse, urllib.request

path = sys.argv[1]
if os.path.exists(path):
    os.unlink(path)
listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind(path)
listener.listen(1)
call, _ = listener.accept()
listener.close()
os.unlink(path)
call.sendall(os.environ.get("STAND_IN_GREETING", '{"kamosu_agent":1}').encode() + b"\n")
line = call.makefile("rb").readline()
if not line:
    sys.exit(0)
handed = json.loads(line)
job = handed["job"]
key = job["job_key"]
seen = os.path.join(os.environ["STAND_IN_SEEN"], job["job_id"])
os.makedirs(seen, exist_ok=True)

def note(name, text):
    with open(os.path.join(seen, name), "w") as f:
        f.write(text)

def ask(path, body=None):
    request = urllib.request.Request(
        urllib.parse.urljoin(handed["kamosu"], path),
        data=None if body is None else json.dumps(body).encode(),
        headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"},
    )
    return urllib.request.urlopen(request).read().decode()

note("pid", str(os.getpid()))
note("sent", line.decode())
mode = ask(job["original"]["address"]).split(":", 1)[1].strip()

if mode == "writes":
    ask("api/op/create_recipe", {"title": "The stand-in's soup"})
    call.sendall(json.dumps({"outcome": "completed",
                             "report": {"unsure": ["The stand-in guessed the salt."]}}).encode())
elif mode == "asks-the-model":
    relay = handed["model_relay"]
    class Relay(http.client.HTTPConnection):
        def connect(self):
            self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self.sock.connect(relay["socket"])
    def model(key):
        connection = Relay("relay")
        connection.request("POST", "/messages", body=b"{}",
                           headers={"x-api-key": key, "anthropic-version": "2023-06-01"})
        answer = connection.getresponse()
        return [answer.status, answer.read().decode()]
    note("model.socket", relay["socket"])
    note("model.right", json.dumps(model(relay["key"])))
    note("model.wrong", json.dumps(model("kamosu-relay-of-another-job")))
    ask("api/op/create_recipe", {"title": "The model's soup"})
    call.sendall(json.dumps({"outcome": "completed"}).encode())
elif mode == "garbage":
    call.sendall(b"not a report")
elif mode == "lingers":
    note("started", "")
    if call.recv(1) == b"":
        note("hung up", "")
call.close()
"#;

/// The socket stand-in, started again each time it ends, as a container
/// with a restart policy is, until the test ends.
struct SocketAgent {
    socket: PathBuf,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _place: tempfile::TempDir,
}

impl SocketAgent {
    fn start(seen: &Path, greeting: Option<&str>) -> SocketAgent {
        let place = tempfile::tempdir().expect("a place for the agent");
        let script = place.path().join("socket-stand-in.py");
        std::fs::write(&script, SOCKET_STAND_IN).unwrap();
        let socket = place.path().join("agent.sock");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (seen, greeting) = (seen.to_path_buf(), greeting.map(str::to_string));
        let (path, stopped) = (socket.clone(), stop.clone());
        std::thread::spawn(move || {
            // The script goes with the test's temporary folder.
            while !stopped.load(std::sync::atomic::Ordering::SeqCst) && script.exists() {
                let mut command = std::process::Command::new("python3");
                command
                    .arg(&script)
                    .arg(&path)
                    .env("STAND_IN_SEEN", &seen)
                    .stdin(Stdio::null());
                if let Some(greeting) = &greeting {
                    command.env("STAND_IN_GREETING", greeting);
                }
                let mut child = command.spawn().expect("the stand-in starts");
                // Checked often, so a test that ends stops its stand-in.
                loop {
                    if child.try_wait().unwrap().is_some() {
                        break;
                    }
                    if stopped.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        });
        SocketAgent {
            socket,
            stop,
            _place: place,
        }
    }
}

impl Drop for SocketAgent {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// A Runner started as its own process, stopped when the test ends.
struct RunningRunner {
    child: Child,
    output: PathBuf,
    /// The Job folders are made under here, and the stand-in writes what it
    /// saw under `seen/`.
    place: tempfile::TempDir,
}

impl RunningRunner {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn seen(&self) -> PathBuf {
        self.place.path().join("seen")
    }

    fn folders(&self) -> PathBuf {
        self.place.path().join("folders")
    }

    /// What the Runner has said so far.
    fn said(&self) -> String {
        std::fs::read_to_string(&self.output).unwrap_or_default()
    }

    /// Wait for the Runner to end by itself, and answer its exit code.
    fn ends(&mut self) -> i32 {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code().unwrap_or(-1);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the Runner never ended. It said:\n{}", self.said());
    }
}

impl Drop for RunningRunner {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// How to start one: where, as what and how it may be tuned.
struct Start<'a> {
    kamosu: String,
    runner_key: &'a str,
    extra_env: Vec<(&'a str, String)>,
    /// The data directory the Runner is pointed at, which it must not touch.
    data_dir: &'a Path,
}

fn start_runner(start: Start) -> RunningRunner {
    start_runner_for(start, None)
}

/// The same, handing its Jobs to the agent at `socket` when one is given.
fn start_runner_for(start: Start, socket: Option<&Path>) -> RunningRunner {
    let kamosu = start.kamosu.clone();
    start_runner_told(start, |command, script| {
        command.args(["--kamosu", &kamosu]);
        match socket {
            Some(socket) => command.arg("--agent-socket").arg(socket),
            None => command.arg("--").arg(script),
        };
    })
}

/// The same, with its command line written by `tell`, which is handed the
/// stand-in agent command. `Start::kamosu` is not passed on by itself.
fn start_runner_told(
    start: Start,
    tell: impl FnOnce(&mut std::process::Command, &Path),
) -> RunningRunner {
    let place = tempfile::tempdir().expect("a place for the Runner");
    let script = place.path().join("stand-in.py");
    std::fs::write(&script, STAND_IN).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::create_dir(place.path().join("seen")).unwrap();
    std::fs::create_dir(place.path().join("folders")).unwrap();
    let output = place.path().join("runner.txt");
    let log = std::fs::File::create(&output).unwrap();

    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_kamosu"));
    command.arg("runner");
    tell(&mut command, &script);
    command
        // Whatever the shell running the tests holds, a test says its own.
        .env_remove("KAMOSU_RUNNER_INSTANCE")
        .env_remove("KAMOSU_RUNNER_AGENT_SOCKET")
        .env_remove("KAMOSU_RUNNER_MODEL_KEY")
        .env_remove("KAMOSU_RUNNER_MODEL_ADDRESS")
        .env_remove("KAMOSU_RUNNER_MODEL_SOCKET")
        .env("KAMOSU_RUNNER_KEY", start.runner_key)
        .env("KAMOSU_DATA_DIR", start.data_dir)
        .env("TMPDIR", place.path().join("folders"))
        .env("STAND_IN_SEEN", place.path().join("seen"))
        .env("KAMOSU_RUNNER_TEST_ASK_EVERY_MS", "100")
        .env("KAMOSU_RUNNER_TEST_STOP_GRACE_MS", "300")
        .stdin(Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log);
    for (name, value) in start.extra_env {
        command.env(name, value);
    }
    RunningRunner {
        child: command.spawn().expect("the Runner starts"),
        output,
        place,
    }
}

/// A cook with an Access Key, and a Runner Key on the instance.
fn cook_and_runner(app: &support::TestApp) -> (String, String) {
    let person = app.core.create_person("Stéphane").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "importer", false)
        .unwrap()
        .secret;
    let runner = app
        .core
        .mint_runner_key("the kitchen's Runner")
        .unwrap()
        .secret;
    (key, runner)
}

/// Pasted text handed to the agent, telling the stand-in what to do. Answers
/// the agent's Job.
fn paste_for_the_agent(app: &support::TestApp, key: &str, mode: &str) -> String {
    let (status, asked) = app.post_op(
        "import_by_agent",
        Some(key),
        &json!({ "text": format!("stand-in: {mode}") }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    asked["result"]["job_id"]
        .as_str()
        .expect("an agent's Job")
        .to_string()
}

fn job(app: &support::TestApp, key: &str, job_id: &str) -> Value {
    let (status, read) = app.post_op(
        "get_job",
        Some(key),
        &json!({ "job_id": job_id }).to_string(),
    );
    assert_eq!(status, 200, "{read}");
    read["result"].clone()
}

/// Wait for a Job to end, with time for a Runner to try it three times.
fn ended(app: &support::TestApp, key: &str, job_id: &str, runner: &RunningRunner) -> Value {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        let record = job(app, key, job_id);
        if ["completed", "failed", "cancelled"].contains(&record["status"].as_str().unwrap_or("")) {
            return record;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "Job {job_id} never ended. The Runner said:\n{}",
        runner.said()
    );
}

fn wait_for(what: &str, runner: &RunningRunner, mut done: impl FnMut() -> bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("{what} never happened. The Runner said:\n{}", runner.said());
}

/// Whether a process is still running: present, and not a zombie.
fn alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat
            .rsplit(')')
            .next()
            .is_some_and(|rest| !rest.trim_start().starts_with('Z')),
        Err(_) => false,
    }
}

fn read_pid(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_hands_a_job_to_its_command_and_reports_what_the_agent_did() {
    let app = support::spawn_app();
    // A hold of 600 ms, so the stand-in's two seconds outlast it three times.
    app.core.hold_jobs_for(Duration::from_millis(600));
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "writes");

    let runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![("STAND_IN_SLEEP_S", "2".to_string())],
        data_dir: data_dir.path(),
    });
    let seen = runner.seen().join(&job_id);
    wait_for("the stand-in starting", &runner, || {
        seen.join("environ.json").exists()
    });

    // While the agent works, the Runner listens on no port and has opened
    // no database: its data directory is as it was.
    assert_eq!(listening_sockets(runner.pid()), 0, "{}", runner.said());
    assert!(
        std::fs::read_dir(data_dir.path()).unwrap().next().is_none(),
        "the Runner wrote into a data directory"
    );
    assert!(
        !open_files(runner.pid())
            .iter()
            .any(|file| file.contains(".db")),
        "{:?}",
        open_files(runner.pid())
    );

    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(
        finished["status"],
        json!("completed"),
        "{finished}\n{}",
        runner.said()
    );

    // The command found the Job and its Job Key in a fresh folder of its
    // own, under the temporary directory, which is gone afterwards.
    let cwd = std::fs::read_to_string(seen.join("cwd")).unwrap();
    assert!(Path::new(&cwd).starts_with(runner.folders()), "{cwd}");
    let handed: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("folder.job.json")).unwrap())
            .unwrap();
    assert_eq!(handed["kamosu"], json!(format!("http://{}/", app.addr)));
    assert_eq!(handed["mcp"], json!(format!("http://{}/mcp", app.addr)));
    assert_eq!(handed["job"]["job_id"], json!(job_id));
    assert!(
        handed["job"]["job_key"]
            .as_str()
            .is_some_and(|k| !k.is_empty())
    );
    assert_eq!(handed["job"]["original"]["kind"], json!("pasted_text"));
    assert_eq!(handed["job"].get("held_until"), None, "{handed}");
    wait_for("the folder going", &runner, || !Path::new(&cwd).exists());
    assert!(
        std::fs::read_dir(runner.folders())
            .unwrap()
            .next()
            .is_none(),
        "a folder was left behind"
    );

    // The Runner Key is in no file of the folder and not in the command's
    // environment.
    for entry in std::fs::read_dir(&seen).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!text.contains(&runner_key), "the agent held the Runner Key");
    }
    let environ: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("environ.json")).unwrap()).unwrap();
    assert_eq!(environ.get("KAMOSU_RUNNER_KEY"), None);
    // Nor can the agent read it from the Runner's own environment, which
    // /proc shows to any process of the same user.
    let runners_environment = std::fs::read(format!("/proc/{}/environ", runner.pid())).unwrap();
    assert!(
        !String::from_utf8_lossy(&runners_environment).contains(&runner_key),
        "the Runner's environment still shows its Runner Key"
    );

    // What the stand-in wrote is on the recipe, and its report reached the
    // Import Report.
    let report = &finished["result"];
    let arrived = &report["arrived"][0];
    let (_, recipe) = app.post_op(
        "get_recipe",
        Some(&key),
        &json!({ "branch_id": arrived["branch_id"] }).to_string(),
    );
    assert_eq!(
        recipe["result"]["versions"][0]["content"]["title"],
        json!("The stand-in's soup"),
        "{recipe}"
    );
    assert!(
        report
            .to_string()
            .contains("The stand-in guessed the salt."),
        "{report}"
    );

    // The Job stayed held from start to end: the Runner renewed, so it was
    // never taken a second time.
    assert_eq!(
        runner
            .said()
            .matches(&format!("Working Job {job_id}"))
            .count(),
        1,
        "{}",
        runner.said()
    );
    assert!(alive(runner.pid()), "{}", runner.said());
    // With no model key, the Runner says where the key belongs.
    assert!(
        runner.said().contains("The Runner holds no model key"),
        "{}",
        runner.said()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_command_that_errs_or_leaves_no_plain_report_fails_saying_why() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let errs = paste_for_the_agent(&app, &key, "errors");
    let silent = paste_for_the_agent(&app, &key, "silent");
    let pipe = paste_for_the_agent(&app, &key, "pipe");

    let runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![],
        data_dir: data_dir.path(),
    });

    let errs = ended(&app, &key, &errs, &runner);
    assert_eq!(errs["status"], json!("failed"), "{errs}");
    assert!(
        errs.to_string()
            .contains("the agent command ended in an error (exit status: 3)"),
        "{errs}"
    );
    let silent = ended(&app, &key, &silent, &runner);
    assert_eq!(silent["status"], json!("failed"), "{silent}");
    assert!(
        silent.to_string().contains("the agent left no report"),
        "{silent}"
    );
    // A report that is not a plain file is never read, so it cannot hold
    // the Runner up.
    let pipe = ended(&app, &key, &pipe, &runner);
    assert_eq!(pipe["status"], json!("failed"), "{pipe}");
    assert!(
        pipe.to_string()
            .contains("the agent's report is not a plain file"),
        "{pipe}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_command_past_the_time_limit_is_stopped_with_all_it_started() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "lingers");

    let runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![("KAMOSU_RUNNER_TEST_TIME_LIMIT_MS", "1500".to_string())],
        data_dir: data_dir.path(),
    });
    let seen = runner.seen().join(&job_id);
    wait_for("the stand-in starting", &runner, || {
        seen.join("started").exists()
    });
    let (agent, its_child) = (
        read_pid(&seen.join("pid")),
        read_pid(&seen.join("child.pid")),
    );

    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(finished["status"], json!("failed"), "{finished}");
    assert!(
        finished
            .to_string()
            .contains("the agent ran past the Runner's time limit of 1.5 seconds and was stopped"),
        "{finished}"
    );
    wait_for("the agent and its child stopping", &runner, || {
        !alive(agent) && !alive(its_child)
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_job_cancelled_mid_work_stops_the_command() {
    let app = support::spawn_app();
    app.core.hold_jobs_for(Duration::from_millis(600));
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "lingers");

    let runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![],
        data_dir: data_dir.path(),
    });
    let seen = runner.seen().join(&job_id);
    wait_for("the stand-in starting", &runner, || {
        seen.join("started").exists()
    });
    let (agent, its_child) = (
        read_pid(&seen.join("pid")),
        read_pid(&seen.join("child.pid")),
    );

    let (status, cancelled) = app.post_op(
        "cancel_job",
        Some(&key),
        &json!({ "job_id": job_id }).to_string(),
    );
    assert_eq!(status, 200, "{cancelled}");

    wait_for("the agent and its child stopping", &runner, || {
        !alive(agent) && !alive(its_child)
    });
    assert_eq!(job(&app, &key, &job_id)["status"], json!("cancelled"));
    wait_for("the Runner saying so", &runner, || {
        runner.said().contains("was cancelled")
    });
    // It reported nothing of its own, and asks for work again.
    assert!(!runner.said().contains("Reported Job"), "{}", runner.said());
    assert!(alive(runner.pid()), "{}", runner.said());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_of_another_version_says_so_and_takes_no_job() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "writes");

    let mut runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![("KAMOSU_RUNNER_TEST_VERSION", "0.0.1-other".to_string())],
        data_dir: data_dir.path(),
    });
    assert_eq!(runner.ends(), 1);
    let said = runner.said();
    assert!(
        said.contains(
            "Kamosu refused this Runner for its version: this Runner is version \
                       0.0.1-other and this Kamosu is version"
        ),
        "{said}"
    );
    assert_eq!(job(&app, &key, &job_id)["status"], json!("queued"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_kamosu_out_of_reach_the_runner_says_so_and_tries_again() {
    // A port that was free a moment ago, and that nothing listens on.
    let nowhere = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    let runner = start_runner(Start {
        kamosu: format!("http://{nowhere}"),
        runner_key: "a key nobody minted",
        extra_env: vec![],
        data_dir: data_dir.path(),
    });
    wait_for("a second try", &runner, || {
        runner.said().matches("is out of reach").count() >= 2
    });
    assert!(alive(runner.pid()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_whose_key_was_revoked_says_so() {
    let app = support::spawn_app();
    let minted = app.core.mint_runner_key("gone").unwrap();
    app.core.revoke_runner_key(&minted.id).unwrap();
    let data_dir = tempfile::tempdir().unwrap();

    let mut runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &minted.secret,
        extra_env: vec![],
        data_dir: data_dir.path(),
    });
    assert_eq!(runner.ends(), 1);
    assert!(
        runner
            .said()
            .contains("Kamosu does not know this Runner Key: it was revoked"),
        "{}",
        runner.said()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_follows_no_redirect() {
    // An address that answers every request with a redirect elsewhere.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = [0u8; 4096];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(
                b"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://192.0.2.10/api/op/ask_for_work\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    let data_dir = tempfile::tempdir().unwrap();
    let mut runner = start_runner(Start {
        kamosu: format!("http://{address}"),
        runner_key: "a key",
        extra_env: vec![],
        data_dir: data_dir.path(),
    });
    assert_eq!(runner.ends(), 1);
    assert!(
        runner.said().contains(
            "answered with a redirect to http://192.0.2.10/api/op/ask_for_work. The Runner \
             speaks only to the address it was given"
        ),
        "{}",
        runner.said()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_hands_each_job_to_a_fresh_agent_at_a_socket() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let first = paste_for_the_agent(&app, &key, "writes");
    let second = paste_for_the_agent(&app, &key, "writes");
    let seen = tempfile::tempdir().unwrap();
    let place = tempfile::tempdir().unwrap();
    let socket = place.path().join("agent.sock");

    // No agent answers yet, so the Runner says so and takes no Job.
    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![],
            data_dir: data_dir.path(),
        },
        Some(&socket),
    );
    wait_for("the Runner waiting for its agent", &runner, || {
        runner.said().contains("Waiting for the agent at")
    });
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(job(&app, &key, &first)["status"], json!("queued"));

    // An agent answers, and works both Jobs: each in a life of its own.
    let agent = SocketAgent::start(seen.path(), None);
    let runner = {
        drop(runner);
        start_runner_for(
            Start {
                kamosu: format!("http://{}", app.addr),
                runner_key: &runner_key,
                extra_env: vec![],
                data_dir: data_dir.path(),
            },
            Some(&agent.socket),
        )
    };
    for job_id in [&first, &second] {
        let finished = ended(&app, &key, job_id, &runner);
        assert_eq!(
            finished["status"],
            json!("completed"),
            "{finished}\n{}",
            runner.said()
        );
        assert!(
            finished
                .to_string()
                .contains("The stand-in guessed the salt."),
            "{finished}"
        );
    }
    let pid = |job_id: &str| read_pid(&seen.path().join(job_id).join("pid"));
    assert_ne!(pid(&first), pid(&second), "one agent worked two Jobs");

    // The agent was sent what job.json holds, and not the Runner Key.
    let sent = std::fs::read_to_string(seen.path().join(&first).join("sent")).unwrap();
    assert!(
        !sent.contains(&runner_key),
        "the agent was sent the Runner Key"
    );
    let sent: Value = serde_json::from_str(&sent).unwrap();
    assert_eq!(sent["kamosu"], json!(format!("http://{}/", app.addr)));
    assert_eq!(sent["mcp"], json!(format!("http://{}/mcp", app.addr)));
    assert_eq!(sent["job"]["job_id"], json!(first));
    assert!(
        sent["job"]["job_key"]
            .as_str()
            .is_some_and(|k| !k.is_empty())
    );
    assert_eq!(sent["job"].get("held_until"), None, "{sent}");
    // Calling a socket is not listening on one.
    assert_eq!(listening_sockets(runner.pid()), 0, "{}", runner.said());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_takes_its_address_and_agent_socket_from_the_environment_alone() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "writes");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), None);

    let runner = start_runner_told(
        Start {
            kamosu: String::new(),
            runner_key: &runner_key,
            extra_env: vec![
                ("KAMOSU_RUNNER_INSTANCE", format!("http://{}", app.addr)),
                (
                    "KAMOSU_RUNNER_AGENT_SOCKET",
                    agent.socket.display().to_string(),
                ),
            ],
            data_dir: data_dir.path(),
        },
        |_, _| {},
    );
    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(
        finished["status"],
        json!("completed"),
        "{finished}\n{}",
        runner.said()
    );
    let sent = std::fs::read_to_string(seen.path().join(&job_id).join("sent")).unwrap();
    let sent: Value = serde_json::from_str(&sent).unwrap();
    assert_eq!(sent["kamosu"], json!(format!("http://{}/", app.addr)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_flag_wins_over_its_variable() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "writes");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), None);
    // A port nothing listens on, and a socket nothing answers at.
    let nowhere = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let place = tempfile::tempdir().unwrap();

    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![
                ("KAMOSU_RUNNER_INSTANCE", format!("http://{nowhere}")),
                (
                    "KAMOSU_RUNNER_AGENT_SOCKET",
                    place.path().join("nobody.sock").display().to_string(),
                ),
            ],
            data_dir: data_dir.path(),
        },
        Some(&agent.socket),
    );
    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(
        finished["status"],
        json!("completed"),
        "{finished}\n{}",
        runner.said()
    );
    assert!(!runner.said().contains("out of reach"), "{}", runner.said());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_refuses_settings_it_cannot_work_with_from_either_place() {
    let data_dir = tempfile::tempdir().unwrap();
    // Each case is given the stand-in as its agent command unless it says not.
    let refused = |extra_env: Vec<(&'static str, String)>, agent_command: bool| {
        let mut runner = start_runner_told(
            Start {
                kamosu: String::new(),
                runner_key: "a key",
                extra_env,
                data_dir: data_dir.path(),
            },
            |command, script| {
                if agent_command {
                    command.arg("--").arg(script);
                }
            },
        );
        assert_eq!(runner.ends(), 2, "{}", runner.said());
        runner.said()
    };

    // No address in either place.
    let said = refused(vec![], true);
    assert!(
        said.contains("Give Kamosu's address, as --kamosu or in KAMOSU_RUNNER_INSTANCE."),
        "{said}"
    );
    // A socket from the variable beside an agent command.
    let said = refused(
        vec![
            ("KAMOSU_RUNNER_INSTANCE", "http://127.0.0.1:9".to_string()),
            ("KAMOSU_RUNNER_AGENT_SOCKET", "/run/agent.sock".to_string()),
        ],
        true,
    );
    assert!(said.contains("Give one of the two."), "{said}");
    // An empty socket variable is no socket, so the address is what is
    // refused: the variable is checked as the flag is.
    let said = refused(
        vec![
            (
                "KAMOSU_RUNNER_INSTANCE",
                "http://cook:secret@127.0.0.1:9".to_string(),
            ),
            ("KAMOSU_RUNNER_AGENT_SOCKET", String::new()),
        ],
        true,
    );
    assert!(said.contains("given with a name or a password"), "{said}");
    assert!(!said.contains("Give one of the two."), "{said}");
    // The refusal lands in the Runner's log, so it repeats no password.
    assert!(!said.contains("secret"), "{said}");
    // No agent in either place, the empty variable counting as none.
    let said = refused(
        vec![
            ("KAMOSU_RUNNER_INSTANCE", "http://127.0.0.1:9".to_string()),
            ("KAMOSU_RUNNER_AGENT_SOCKET", " ".to_string()),
        ],
        false,
    );
    assert!(said.contains("Name the agent:"), "{said}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_at_a_socket_that_sends_no_plain_report_fails_its_job() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let silent = paste_for_the_agent(&app, &key, "silent");
    let garbage = paste_for_the_agent(&app, &key, "garbage");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), None);

    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![],
            data_dir: data_dir.path(),
        },
        Some(&agent.socket),
    );
    let silent = ended(&app, &key, &silent, &runner);
    assert_eq!(silent["status"], json!("failed"), "{silent}");
    assert!(
        silent.to_string().contains("the agent left no report"),
        "{silent}"
    );
    let garbage = ended(&app, &key, &garbage, &runner);
    assert_eq!(garbage["status"], json!("failed"), "{garbage}");
    assert!(
        garbage
            .to_string()
            .contains("the agent's report is not JSON"),
        "{garbage}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_runner_hangs_up_on_an_agent_at_a_socket_past_the_time_limit() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "lingers");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), None);

    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![("KAMOSU_RUNNER_TEST_TIME_LIMIT_MS", "1500".to_string())],
            data_dir: data_dir.path(),
        },
        Some(&agent.socket),
    );
    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(finished["status"], json!("failed"), "{finished}");
    assert!(
        finished
            .to_string()
            .contains("the agent ran past the Runner's time limit of 1.5 seconds and was stopped"),
        "{finished}"
    );
    let hung_up = seen.path().join(&job_id).join("hung up");
    wait_for("the agent seeing the Runner hang up", &runner, || {
        hung_up.exists()
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_socket_that_does_not_greet_as_an_agent_is_handed_no_job() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let job_id = paste_for_the_agent(&app, &key, "writes");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), Some(r#"{"kamosu_agent":2}"#));

    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![],
            data_dir: data_dir.path(),
        },
        Some(&agent.socket),
    );
    wait_for("the Runner saying why it waits", &runner, || {
        runner
            .said()
            .contains(r#"it greeted the Runner with something other than {"kamosu_agent":1}"#)
    });
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(job(&app, &key, &job_id)["status"], json!("queued"));
}

/// The key the tests give the Runner for the model's provider.
const MODEL_KEY: &str = "sk-the-providers-own-key";

/// One request a provider was sent: its path and its headers.
type Sent = (String, Vec<(String, String)>);

/// A model provider that writes down each request it is sent, its path and
/// headers, and answers with a stream of two pieces a second apart, as a
/// model streams.
struct FakeProvider {
    address: String,
    seen: std::sync::Arc<std::sync::Mutex<Vec<Sent>>>,
}

impl FakeProvider {
    fn start() -> FakeProvider {
        use axum::body::Body;
        use axum::extract::Request;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = format!("http://{}/v1", listener.local_addr().unwrap());
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let noted = seen.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let app = axum::Router::new().fallback(move |request: Request| {
                    let noted = noted.clone();
                    async move {
                        let headers = request
                            .headers()
                            .iter()
                            .map(|(name, value)| {
                                (name.to_string(), value.to_str().unwrap_or("").to_string())
                            })
                            .collect();
                        let path = request
                            .uri()
                            .path_and_query()
                            .map(|p| p.to_string())
                            .unwrap_or_default();
                        noted.lock().unwrap().push((path, headers));
                        // The second piece a second after the first, so a
                        // relay that held the answer back shows.
                        let pieces = futures_util::stream::unfold(0, |sent| async move {
                            match sent {
                                0 => Some((Ok::<_, std::io::Error>("data: the model's\n"), 1)),
                                1 => {
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    Some((Ok("answer\n\n"), 2))
                                }
                                _ => None,
                            }
                        });
                        Body::from_stream(pieces)
                    }
                });
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        });
        FakeProvider { address, seen }
    }

    /// What it was sent so far: each request's path and headers.
    fn seen(&self) -> Vec<Sent> {
        self.seen.lock().unwrap().clone()
    }
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

/// Whether any file the stand-in wrote holds `text`.
fn seen_holds(seen: &Path, text: &str) -> bool {
    std::fs::read_dir(seen).unwrap().any(|entry| {
        std::fs::read_to_string(entry.unwrap().path()).is_ok_and(|file| file.contains(text))
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_command_reaches_its_model_through_the_runner_and_never_holds_the_key() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let provider = FakeProvider::start();
    let job_id = paste_for_the_agent(&app, &key, "asks-the-model");

    let runner = start_runner(Start {
        kamosu: format!("http://{}", app.addr),
        runner_key: &runner_key,
        extra_env: vec![
            ("KAMOSU_RUNNER_MODEL_KEY", MODEL_KEY.to_string()),
            ("KAMOSU_RUNNER_MODEL_ADDRESS", provider.address.clone()),
        ],
        data_dir: data_dir.path(),
    });
    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(
        finished["status"],
        json!("completed"),
        "{finished}\n{}",
        runner.said()
    );
    assert!(
        runner.said().contains(&format!(
            "Relaying the agent's model requests to {}/",
            provider.address
        )),
        "{}",
        runner.said()
    );

    // The request with the stand-in reached the provider under the same
    // path, with the Runner's key in its place, and its answer streamed back.
    let seen = runner.seen().join(&job_id);
    let right: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("model.right")).unwrap()).unwrap();
    assert_eq!(right, json!([200, "data: the model's\nanswer\n\n"]));
    let sent = provider.seen();
    // The first piece reached the agent before the provider sent the second.
    let waited: f64 = std::fs::read_to_string(seen.join("model.waited"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(waited > 0.5, "the relay held the answer back: {waited} s");
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "/v1/chat/completions?stream=true");
    assert_eq!(
        header(&sent[0].1, "authorization"),
        Some(format!("Bearer {MODEL_KEY}").as_str())
    );
    assert_eq!(header(&sent[0].1, "content-type"), Some("application/json"));

    // The agent is told where the relay sends, to check its own settings.
    let handed: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("folder.job.json")).unwrap())
            .unwrap();
    assert_eq!(
        handed["model_relay"]["provider"],
        json!(format!("{}/", provider.address))
    );

    // Another key is refused and goes nowhere.
    let wrong: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("model.wrong")).unwrap()).unwrap();
    assert_eq!(wrong[0], json!(401), "{wrong}");

    // The key is in no file of the folder, not in the command's environment,
    // and blanked in the Runner's own.
    assert!(
        !seen_holds(&seen, MODEL_KEY),
        "the agent held the model's key"
    );
    let environ: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("environ.json")).unwrap()).unwrap();
    assert_eq!(environ.get("KAMOSU_RUNNER_MODEL_KEY"), None);
    let runners_environment = std::fs::read(format!("/proc/{}/environ", runner.pid())).unwrap();
    assert!(
        !String::from_utf8_lossy(&runners_environment).contains(MODEL_KEY),
        "the Runner's environment still shows the model's key"
    );

    // The relay was on the loopback for that Job alone: it is closed now,
    // and the Runner listens on nothing between Jobs.
    let address = std::fs::read_to_string(seen.join("model.address")).unwrap();
    let port = Url::parse(&address).unwrap().socket_addrs(|| None).unwrap()[0];
    assert!(port.ip().is_loopback(), "{address}");
    wait_for("the relay closing", &runner, || {
        std::net::TcpStream::connect(port).is_err()
    });
    assert_eq!(listening_sockets(runner.pid()), 0, "{}", runner.said());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_at_a_socket_reaches_its_model_through_the_runners_socket() {
    let app = support::spawn_app();
    let (key, runner_key) = cook_and_runner(&app);
    let data_dir = tempfile::tempdir().unwrap();
    let provider = FakeProvider::start();
    let job_id = paste_for_the_agent(&app, &key, "asks-the-model");
    let seen = tempfile::tempdir().unwrap();
    let agent = SocketAgent::start(seen.path(), None);
    let relay_place = tempfile::tempdir().unwrap();
    let relay_socket = relay_place.path().join("model.sock");

    let runner = start_runner_for(
        Start {
            kamosu: format!("http://{}", app.addr),
            runner_key: &runner_key,
            extra_env: vec![
                ("KAMOSU_RUNNER_MODEL_KEY", MODEL_KEY.to_string()),
                ("KAMOSU_RUNNER_MODEL_ADDRESS", provider.address.clone()),
                (
                    "KAMOSU_RUNNER_MODEL_SOCKET",
                    relay_socket.display().to_string(),
                ),
            ],
            data_dir: data_dir.path(),
        },
        Some(&agent.socket),
    );
    let finished = ended(&app, &key, &job_id, &runner);
    assert_eq!(
        finished["status"],
        json!("completed"),
        "{finished}\n{}",
        runner.said()
    );

    let seen = seen.path().join(&job_id);
    assert_eq!(
        std::fs::read_to_string(seen.join("model.socket")).unwrap(),
        relay_socket.display().to_string()
    );
    let right: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("model.right")).unwrap()).unwrap();
    assert_eq!(right, json!([200, "data: the model's\nanswer\n\n"]));
    let wrong: Value =
        serde_json::from_str(&std::fs::read_to_string(seen.join("model.wrong")).unwrap()).unwrap();
    assert_eq!(wrong[0], json!(401), "{wrong}");

    // The key went where the agent put the stand-in, and nowhere else.
    let sent = provider.seen();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "/v1/messages");
    assert_eq!(header(&sent[0].1, "x-api-key"), Some(MODEL_KEY));
    assert_eq!(header(&sent[0].1, "authorization"), None);
    assert_eq!(header(&sent[0].1, "anthropic-version"), Some("2023-06-01"));
    assert!(
        !seen_holds(&seen, MODEL_KEY),
        "the agent was sent the model's key"
    );

    // The socket was the Job's alone, and is gone with it.
    wait_for("the relay's socket going", &runner, || {
        !relay_socket.exists()
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_refuses_model_settings_it_cannot_relay_with() {
    let data_dir = tempfile::tempdir().unwrap();
    let refused = |extra_env: Vec<(&'static str, String)>, agent_command: bool| {
        let mut runner = start_runner_told(
            Start {
                kamosu: String::new(),
                runner_key: "a key",
                extra_env,
                data_dir: data_dir.path(),
            },
            |command, script| {
                command.args(["--kamosu", "http://127.0.0.1:9"]);
                if agent_command {
                    command.arg("--").arg(script);
                } else {
                    command.args(["--agent-socket", "/run/kamosu-agent/agent.sock"]);
                }
            },
        );
        assert_eq!(runner.ends(), 2, "{}", runner.said());
        runner.said()
    };
    let key = ("KAMOSU_RUNNER_MODEL_KEY", MODEL_KEY.to_string());
    let address = (
        "KAMOSU_RUNNER_MODEL_ADDRESS",
        "https://openrouter.ai/api/v1".to_string(),
    );
    let socket = (
        "KAMOSU_RUNNER_MODEL_SOCKET",
        "/run/kamosu-relay/model.sock".to_string(),
    );

    let said = refused(vec![key.clone()], true);
    assert!(said.contains("names no address to send it to"), "{said}");
    assert!(!said.contains(MODEL_KEY), "{said}");
    let said = refused(vec![address.clone()], true);
    assert!(said.contains("holds no key for it"), "{said}");
    let said = refused(vec![socket.clone()], true);
    assert!(
        said.contains("no KAMOSU_RUNNER_MODEL_KEY is given"),
        "{said}"
    );
    let said = refused(vec![key.clone(), address.clone(), socket.clone()], true);
    assert!(said.contains("is for an agent at a socket"), "{said}");
    let said = refused(vec![key.clone(), address.clone()], false);
    assert!(
        said.contains("name it in KAMOSU_RUNNER_MODEL_SOCKET"),
        "{said}"
    );
    let said = refused(
        vec![
            key.clone(),
            (
                "KAMOSU_RUNNER_MODEL_ADDRESS",
                "https://me:sk-hidden@openrouter.ai/api/v1".to_string(),
            ),
        ],
        true,
    );
    assert!(said.contains("with a name or a password"), "{said}");
    assert!(!said.contains("sk-hidden"), "{said}");
}

/// How many TCP and Unix sockets a process holds that are listening.
fn listening_sockets(pid: u32) -> usize {
    let held: Vec<String> = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .filter_map(|fd| std::fs::read_link(fd.ok()?.path()).ok())
        .filter_map(|target| {
            let target = target.to_string_lossy().into_owned();
            Some(
                target
                    .strip_prefix("socket:[")?
                    .trim_end_matches(']')
                    .to_string(),
            )
        })
        .collect();
    let tcp = ["tcp", "tcp6"]
        .iter()
        .flat_map(|table| {
            std::fs::read_to_string(format!("/proc/{pid}/net/{table}"))
                .unwrap_or_default()
                .lines()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // State 0A is LISTEN; the inode is the tenth field.
            fields.get(3) == Some(&"0A")
                && fields.get(9).is_some_and(|i| held.iter().any(|h| h == i))
        })
        .count();
    let unix = std::fs::read_to_string(format!("/proc/{pid}/net/unix"))
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // Flags 00010000 is a socket accepting calls; the inode is the
            // seventh field.
            fields.get(3) == Some(&"00010000")
                && fields.get(6).is_some_and(|i| held.iter().any(|h| h == i))
        })
        .count();
    tcp + unix
}

/// Every file a process holds open, by path.
fn open_files(pid: u32) -> Vec<String> {
    std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .filter_map(|fd| std::fs::read_link(fd.ok()?.path()).ok())
        .map(|target| target.to_string_lossy().into_owned())
        .collect()
}
