//! The model relay (#372): how the agent beside a Runner reaches its model
//! without holding the provider's key.
//!
//! The agent reads pages strangers wrote and has the open internet, so a page
//! that tricks it can have it send away anything it holds. The Runner holds
//! the key instead, given in `KAMOSU_RUNNER_MODEL_KEY`, with the one address
//! it may be sent to, `KAMOSU_RUNNER_MODEL_ADDRESS`. For each Job it opens a
//! relay for that Job's agent: a port on the loopback for an agent command,
//! or a Unix socket for an agent in a container of its own. The agent is
//! handed the relay's address and a stand-in key made up for that Job.
//!
//! A request that carries the stand-in where a provider reads its key is sent
//! on to the configured address, under the same path, with the real key in
//! the stand-in's place. Providers differ in where that is: `Authorization:
//! Bearer` for OpenAI, OpenRouter and every OpenAI-compatible service,
//! `x-api-key` for Anthropic, `x-goog-api-key` for Google. The relay reads
//! all three and fills in whichever the agent used, so it needs no list of
//! providers. Any other request is refused. When the Job ends the relay
//! closes, with every request still open, and its stand-in is worth nothing.
//!
//! The relay is the Runner's, so its client is its own too, kept apart from
//! the guarded client every other fetch in Kamosu uses (ADR 0033) as the
//! Runner's is. It speaks to the configured address alone, follows no
//! redirect and goes through no proxy.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, UnixListener};
use tokio::task::{JoinHandle, JoinSet};
use url::Url;

/// The variable the provider's key is read from. Like the Runner Key, never a
/// command-line argument.
pub const MODEL_KEY_VAR: &str = "KAMOSU_RUNNER_MODEL_KEY";

/// The variable naming the one address the relay sends requests to.
pub const MODEL_ADDRESS_VAR: &str = "KAMOSU_RUNNER_MODEL_ADDRESS";

/// The variable naming where the relay opens its socket, for an agent in a
/// container of its own.
pub const MODEL_SOCKET_VAR: &str = "KAMOSU_RUNNER_MODEL_SOCKET";

/// The headers a provider reads its key from, lowercase as `http` holds them.
const KEY_HEADERS: [&str; 3] = ["authorization", "x-api-key", "x-goog-api-key"];

/// Headers that describe one connection and are never passed on, with `host`,
/// which names the relay, and `content-length`, which hyper writes again.
const CONNECTION_HEADERS: [&str; 10] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "host",
];

/// The most of one request the relay takes. An agent sends its whole
/// conversation again at each step, pictures included, and providers refuse
/// a request past 32 MB to 50 MB, so this leaves room for any of them.
const REQUEST_MOST_BYTES: usize = 100 * 1024 * 1024;

/// How long the agent has to send a request's headers.
const HEADER_DEADLINE: Duration = Duration::from_secs(20);

/// How long the relay waits for the provider to take a connection.
const CONNECT_DEADLINE: Duration = Duration::from_secs(30);

/// The provider: the one address the relay sends to, and the key it adds.
pub struct Provider {
    address: Url,
    key: String,
    client: reqwest::Client,
}

/// The address the Runner was given for the provider, as the base every
/// relayed path goes under. Its refusal never repeats what was given, which
/// lands in the Runner's log and may be the key set in the wrong place.
pub fn provider_address(given: &str) -> Result<Url, String> {
    let not_an_address = || {
        format!(
            "{MODEL_ADDRESS_VAR} holds no provider's address. Give the address its API \
             is reached at, such as https://openrouter.ai/api/v1, and the key in \
             {MODEL_KEY_VAR}"
        )
    };
    let mut address = Url::parse(given.trim()).map_err(|_| not_an_address())?;
    if !matches!(address.scheme(), "http" | "https") || address.host().is_none() {
        return Err(not_an_address());
    }
    if !address.username().is_empty() || address.password().is_some() {
        return Err(format!(
            "{MODEL_ADDRESS_VAR} holds an address with a name or a password. Give the \
             provider's address alone: the key goes in {MODEL_KEY_VAR}"
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

impl Provider {
    pub fn new(address: Url, key: String) -> Provider {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(CONNECT_DEADLINE)
            .build()
            .expect("the relay's client");
        Provider {
            address,
            key,
            client,
        }
    }

    pub fn address(&self) -> &Url {
        &self.address
    }

    /// Where a request the agent sent to `path_and_query` goes: the same path
    /// under the provider's address, and nowhere outside it.
    fn target(&self, path_and_query: &str) -> Option<Url> {
        if !path_and_query.starts_with('/') {
            return None;
        }
        let base = self.address.as_str().trim_end_matches('/');
        let target = Url::parse(&format!("{base}{path_and_query}")).ok()?;
        (target.origin() == self.address.origin() && target.path().starts_with(self.address.path()))
            .then_some(target)
    }
}

/// Where the relay is opened for a Job's agent.
#[derive(Clone, Debug)]
pub enum Place {
    /// A port on the loopback, for an agent command on the Runner's machine.
    Loopback,
    /// A Unix socket at this path, for an agent in a container of its own.
    Socket(PathBuf),
}

/// One Job's relay. Dropping it closes it, with every request still open.
pub struct Relay {
    handed: Value,
    server: JoinHandle<()>,
    socket: Option<PathBuf>,
}

impl Relay {
    /// Open a relay at `place`, with a stand-in key made up for this Job.
    pub async fn open(provider: Arc<Provider>, place: &Place) -> std::io::Result<Relay> {
        let stand_in = format!(
            "kamosu-relay-{}",
            hex::encode(crate::core::random_bytes(24))
        );
        let sends_to = provider.address.to_string();
        let app = Router::new()
            .fallback(forward)
            .with_state(Arc::new(RelayState {
                provider,
                stand_in: Sha256::digest(stand_in.as_bytes()).to_vec(),
            }));
        match place {
            Place::Loopback => {
                let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
                let address = format!("http://{}/", listener.local_addr()?);
                Ok(Relay {
                    handed: json!({ "address": address, "key": stand_in, "provider": sends_to }),
                    server: tokio::spawn(serve(Listening::Tcp(listener), app)),
                    socket: None,
                })
            }
            Place::Socket(path) => {
                // One left by a Runner stopped in the middle of a Job.
                match std::fs::remove_file(path) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
                    _ => {}
                }
                let listener = UnixListener::bind(path)?;
                // The agent runs as another user in another container, and
                // the socket is in a volume only the two share.
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;
                Ok(Relay {
                    handed: json!({ "socket": path, "key": stand_in, "provider": sends_to }),
                    server: tokio::spawn(serve(Listening::Unix(listener), app)),
                    socket: Some(path.clone()),
                })
            }
        }
    }

    /// What the agent is handed in `job.json`, as `model_relay`: where the
    /// relay is, as `address` or `socket`, the stand-in, as `key`, and the
    /// provider's address it sends to, as `provider`.
    pub fn handed(&self) -> Value {
        self.handed.clone()
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        // The server task holds every connection, so they end with it.
        self.server.abort();
        if let Some(socket) = &self.socket {
            let _ = std::fs::remove_file(socket);
        }
    }
}

struct RelayState {
    provider: Arc<Provider>,
    /// The stand-in's hash. A key presented is hashed and compared to it, so
    /// how long the comparison takes says nothing about the stand-in.
    stand_in: Vec<u8>,
}

enum Listening {
    Tcp(TcpListener),
    Unix(UnixListener),
}

/// Take connections until the relay is dropped, serving each on its own task
/// held here, so that they all end with this one.
async fn serve(listening: Listening, app: Router) {
    let mut connections = JoinSet::new();
    loop {
        let accepted = match &listening {
            Listening::Tcp(listener) => listener
                .accept()
                .await
                .map(|(stream, _)| connect(&mut connections, stream, app.clone())),
            Listening::Unix(listener) => listener
                .accept()
                .await
                .map(|(stream, _)| connect(&mut connections, stream, app.clone())),
        };
        if accepted.is_err() {
            // Out of file descriptors, say: waited out, not retried in a
            // tight loop.
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        while connections.try_join_next().is_some() {}
    }
}

fn connect<S>(connections: &mut JoinSet<()>, stream: S, app: Router)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    connections.spawn(async move {
        let mut builder = hyper::server::conn::http1::Builder::new();
        builder
            .timer(TokioTimer::new())
            .header_read_timeout(HEADER_DEADLINE);
        let _ = builder
            .serve_connection(TokioIo::new(stream), TowerToHyperService::new(app))
            .await;
    });
}

/// Send one request on to the provider with the real key, and its answer
/// back as it arrives, so that a streamed answer streams.
async fn forward(State(shared): State<Arc<RelayState>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let Some(carrier) = carrier(&parts.headers, &shared.stand_in) else {
        return refused(
            StatusCode::UNAUTHORIZED,
            "This relay answers the agent of the Job it was opened for, with the key it was \
             handed in job.json.",
        );
    };
    let path = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    let Some(target) = shared.provider.target(path) else {
        return refused(
            StatusCode::BAD_REQUEST,
            "This relay sends requests under the provider's address alone.",
        );
    };
    let Ok(body) = axum::body::to_bytes(body, REQUEST_MOST_BYTES).await else {
        return refused(
            StatusCode::BAD_REQUEST,
            "The relay could not read the whole request, which may hold up to 100 MB.",
        );
    };
    let mut headers = passed_on(&parts.headers);
    for name in KEY_HEADERS {
        headers.remove(name);
    }
    let key = if carrier == header::AUTHORIZATION {
        format!("Bearer {}", shared.provider.key)
    } else {
        shared.provider.key.clone()
    };
    let Ok(mut key) = HeaderValue::from_str(&key) else {
        return refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The Runner's model key cannot be sent in a header.",
        );
    };
    key.set_sensitive(true);
    headers.insert(carrier, key);

    let sent = shared
        .provider
        .client
        .request(parts.method, target)
        .headers(headers)
        .body(body)
        .send()
        .await;
    let answer = match sent {
        Ok(answer) => answer,
        Err(e) => {
            return refused(
                StatusCode::BAD_GATEWAY,
                &format!(
                    "The relay could not reach the model's provider at {}: {}",
                    shared.provider.address,
                    crate::runner::plain_error(&e)
                ),
            );
        }
    };
    let status = answer.status();
    let headers = passed_on(answer.headers());
    let chunks = futures_util::stream::unfold(Some(answer), |answer| async move {
        let mut answer = answer?;
        match answer.chunk().await {
            Ok(Some(chunk)) => Some((Ok(chunk), Some(answer))),
            Ok(None) => None,
            Err(e) => Some((Err(e), None)),
        }
    });
    let mut response = Response::new(Body::from_stream(chunks));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

/// The header the agent put the stand-in in, or None when it sent no
/// stand-in, or one from another Job.
fn carrier(headers: &HeaderMap, stand_in: &[u8]) -> Option<HeaderName> {
    KEY_HEADERS.into_iter().find_map(|name| {
        let value = headers.get(name)?.to_str().ok()?.trim();
        let presented = if name == "authorization" {
            let (scheme, key) = value.split_once(' ')?;
            if !scheme.eq_ignore_ascii_case("bearer") {
                return None;
            }
            key.trim()
        } else {
            value
        };
        (Sha256::digest(presented.as_bytes()).as_slice() == stand_in)
            .then(|| HeaderName::from_static(name))
    })
}

/// A request's or an answer's headers, less those about one connection.
fn passed_on(headers: &HeaderMap) -> HeaderMap {
    let mut kept = headers.clone();
    for name in CONNECTION_HEADERS {
        kept.remove(name);
    }
    kept.remove(header::CONTENT_LENGTH);
    kept
}

/// The relay's own refusal, in the shape providers answer one, so that the
/// agent shows it.
fn refused(status: StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(json!({ "error": { "message": message, "type": "kamosu_relay" } })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(address: &str) -> Provider {
        Provider::new(provider_address(address).unwrap(), "sk-real".to_string())
    }

    #[test]
    fn a_path_goes_under_the_providers_address_and_nowhere_else() {
        let openrouter = provider("https://openrouter.ai/api/v1");
        assert_eq!(
            openrouter.target("/chat/completions").unwrap().as_str(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            openrouter.target("/models?limit=5").unwrap().as_str(),
            "https://openrouter.ai/api/v1/models?limit=5"
        );
        assert!(openrouter.target("/../v2/chat").is_none());
        assert!(openrouter.target("/%2e%2e/%2e%2e/admin").is_none());
        // A path that reads like another host is still a path at this one.
        assert_eq!(
            openrouter
                .target("//elsewhere.example/v1/chat")
                .unwrap()
                .host_str(),
            Some("openrouter.ai")
        );
        assert!(openrouter.target("http://elsewhere.example/").is_none());
        assert!(openrouter.target("@elsewhere.example/").is_none());
    }

    #[test]
    fn a_providers_address_holds_no_secret() {
        assert!(provider_address("https://user:sk-secret@api.example").is_err());
        let refusal = provider_address("https://user:sk-secret@api.example").unwrap_err();
        assert!(!refusal.contains("sk-secret"), "{refusal}");
        // The key put in the address's place by mistake is not repeated.
        for misplaced in ["sk-or-v1-secret", "user:sk-secret@api.example"] {
            let refusal = provider_address(misplaced).unwrap_err();
            assert!(!refusal.contains("secret"), "{refusal}");
        }
        assert!(provider_address("ftp://api.example").is_err());
        assert!(provider_address("api.example/v1").is_err());
        assert_eq!(
            provider_address("http://192.0.2.10:11434/v1?x=1")
                .unwrap()
                .as_str(),
            "http://192.0.2.10:11434/v1/"
        );
    }

    #[test]
    fn the_stand_in_is_found_where_each_provider_reads_its_key() {
        let stand_in = Sha256::digest(b"kamosu-relay-abc").to_vec();
        let with = |name: &'static str, value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(name, HeaderValue::from_str(value).unwrap());
            carrier(&headers, &stand_in)
        };
        assert_eq!(
            with("authorization", "Bearer kamosu-relay-abc"),
            Some(header::AUTHORIZATION)
        );
        assert_eq!(
            with("authorization", "bearer kamosu-relay-abc"),
            Some(header::AUTHORIZATION)
        );
        assert_eq!(
            with("x-api-key", "kamosu-relay-abc")
                .as_ref()
                .map(|n| n.as_str()),
            Some("x-api-key")
        );
        assert_eq!(
            with("x-goog-api-key", "kamosu-relay-abc")
                .as_ref()
                .map(|n| n.as_str()),
            Some("x-goog-api-key")
        );
        assert_eq!(with("authorization", "Basic kamosu-relay-abc"), None);
        assert_eq!(with("authorization", "Bearer kamosu-relay-abd"), None);
        assert_eq!(with("x-api-key", "sk-real"), None);
        assert_eq!(with("api-key", "kamosu-relay-abc"), None);
    }
}
