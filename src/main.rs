//! One executable, many roles (ADR 0028): `kamosu` with no arguments serves;
//! terminal commands are arguments to the same binary, because a distroless image
//! has no shell — `docker exec kamosu /app/kamosu status`, never a pipeline.

use std::net::SocketAddr;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use serde_json::json;

use kamosu::config::Config;
use kamosu::core::Core;
use kamosu::{db, http_min};

#[derive(Parser)]
#[command(
    name = "kamosu",
    about = "A self-hosted cookbook where people and agents are equal users.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve Kamosu on the configured address and port (the default).
    Serve,
    /// Print this instance's status as JSON: the version and whether setup has happened.
    Status,
    /// Check that this Kamosu answers its own Operations. Exits 0 when healthy.
    /// Run by the container's HEALTHCHECK — a distroless image has no curl to do it.
    HealthCheck,
    /// Print the Catalogue as JSON: every Operation's name, permission, kind and
    /// schemas. The interface's typed client is generated from this (`just client`).
    Catalogue,
    /// Mint a one-use recovery link for a named Person. This never reads, prints,
    /// or sets a password; the link holder chooses their own.
    Recover { name: String },
    /// Run beside an agent: ask the Kamosu at `--kamosu` for Jobs and hand
    /// each, with its Job Key, to the agent command given after `--`, in a
    /// fresh folder, or to the agent listening at `--agent-socket`. Either
    /// flag may be left to KAMOSU_RUNNER_INSTANCE or KAMOSU_RUNNER_AGENT_SOCKET,
    /// and the Runner Key is read from KAMOSU_RUNNER_KEY. Listens on no port and opens no
    /// database.
    Runner {
        /// Kamosu's address, such as https://recipes.kamosu.example. Read
        /// from KAMOSU_RUNNER_INSTANCE when left out.
        #[arg(long)]
        kamosu: Option<String>,
        /// The Unix socket an agent in a container of its own listens on, in
        /// place of an agent command (#317). Read from
        /// KAMOSU_RUNNER_AGENT_SOCKET when left out.
        #[arg(long, conflicts_with = "command")]
        agent_socket: Option<std::path::PathBuf>,
        /// The agent command and its arguments, after `--`.
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Build a Demo's Sample Library into a folder of its own, as a Demo does
    /// when it starts, and print what it holds and how long it took. Fails if
    /// any step of the story does. The folder is emptied first.
    SampleLibrary { folder: std::path::PathBuf },
    /// Read the PDF on standard input and write its text to standard output.
    /// Kamosu starts this itself to read a PDF apart from everything else
    /// (#373); it is no command for a person.
    #[command(hide = true)]
    ReadPdf,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::from_env();

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(config),
        Command::Status => status(&config),
        Command::Catalogue => {
            println!(
                "{}",
                serde_json::to_string_pretty(&kamosu::catalogue::declarations())
                    .expect("the Catalogue serialises")
            );
        }
        Command::HealthCheck => {
            let addr: SocketAddr = format!("127.0.0.1:{}", config.port)
                .parse()
                .expect("valid address");
            std::process::exit(health_check(addr));
        }
        Command::Recover { name } => recover(&config, &name),
        Command::SampleLibrary { folder } => std::process::exit(sample_library(&folder)),
        Command::ReadPdf => std::process::exit(kamosu::pdf::read_here()),
        Command::Runner {
            kamosu,
            agent_socket,
            command,
        } => std::process::exit(kamosu::runner::run(kamosu::runner::Told {
            kamosu,
            agent_socket,
            command,
        })),
    }
}

fn sample_library(folder: &std::path::Path) -> i32 {
    let started = std::time::Instant::now();
    let built = match kamosu::sample_library::build(folder) {
        Ok(built) => built,
        Err(err) => {
            eprintln!("{}", err.to_sentence());
            return 1;
        }
    };
    match kamosu::sample_library::summary(&built) {
        Ok(mut summary) => {
            summary["seconds"] = json!(started.elapsed().as_secs_f64());
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("a summary serialises")
            );
            0
        }
        Err(err) => {
            eprintln!("{}", err.to_sentence());
            1
        }
    }
}

fn serve(config: Config) {
    let filter = tracing_subscriber::EnvFilter::try_new(&config.log_filter)
        .expect("KAMOSU_LOG must be a valid tracing filter, e.g. 'info'");
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        // Core::start opens the database and starts the Job lanes: slow work is
        // carried from the moment the instance serves. A Demo has its Sample
        // Library poured in first, so no visitor arrives before Camille does.
        let db = Arc::new(
            db::Db::open(&config.data_dir).unwrap_or_else(|e| panic!("cannot open /data: {e}")),
        );
        let analytics = config.analytics.clone().unwrap_or_else(|why| {
            eprintln!("This Demo counts no visits: {why}");
            None
        });
        let core = if config.demo {
            kamosu::core::start_demo(db, analytics.clone())
                .unwrap_or_else(|e| panic!("cannot start the Demo with its Sample Library: {e}"))
        } else {
            Core::start(db)
        };
        let app = kamosu::app(core);

        let listener = tokio::net::TcpListener::bind(config.bind_address())
            .await
            .unwrap_or_else(|e| panic!("cannot bind {}: {e}", config.bind_address()));
        println!(
            "Kamosu {} serving on http://{} (data in {})",
            env!("CARGO_PKG_VERSION"),
            config.bind_address(),
            config.data_dir.display()
        );
        if config.demo {
            println!(
                "This is a Demo ({}): anyone arrives signed in, and only cooking and \
                 shopping save",
                kamosu::config::DEMO
            );
            if let Some(analytics) = &analytics {
                println!(
                    "It counts its visits with the Umami at {}",
                    analytics.origin
                );
            }
        }

        kamosu::admission::serve(
            listener,
            app,
            kamosu::admission::Limits::KAMOSU.header,
            shutdown_signal(),
        )
        .await;
    });
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let ctrl_c = tokio::signal::ctrl_c();
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = ctrl_c => {},
        _ = term.recv() => {},
    }
}

/// The terminal form of `instance_status`: the same truth, no Door involved.
fn status(config: &Config) {
    let core = Core::open(Arc::new(db::Db::open(&config.data_dir).expect("database")));
    let setup = core.setup_complete().expect("setup flag");
    println!(
        "{}",
        json!({ "version": env!("CARGO_PKG_VERSION"), "setup_complete": setup })
    );
}

/// Terminal recovery mints one Secret-bearing link and nothing password-shaped.
fn recover(config: &Config, name: &str) {
    let core = Core::open(Arc::new(db::Db::open(&config.data_dir).expect("database")));
    match core.mint_recovery_link(name) {
        Ok(link) => println!("{link}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

/// Ask this Kamosu whether it can carry out its own Operations. One request to
/// one Operation through the real web door; anything but HTTP 200 fails the check.
fn health_check(addr: SocketAddr) -> i32 {
    match http_min::post_json(addr, "/api/op/instance_status", None, "{}") {
        Ok(response) if response.status == 200 => {
            println!("healthy");
            0
        }
        Ok(response) => {
            eprintln!("unhealthy: instance_status answered {}", response.status);
            1
        }
        Err(e) => {
            eprintln!("unhealthy: cannot reach {addr}: {e}");
            1
        }
    }
}
