//! What a request must meet before a Door reads it (#368).
//!
//! ADR 0033 holds that Kamosu on its bare port, with no proxy in front, is as
//! safe as Kamosu behind one. So the limits a proxy would otherwise set are
//! Kamosu's own, written here with the reason for each number:
//!
//! - how long a connection may take to send a request's headers, and how long
//!   it may sit idle between two requests;
//! - how long a whole request may take, once its headers are in;
//! - how large a body a request with no Credential may carry, and how long it
//!   may take to arrive;
//! - how many requests are answered at once;
//! - which `Origin` a request that changes something may come from.
//!
//! Whether a request carries a Credential is the Core's to say (`Core::knock`);
//! [`meet`] asks it before a body is read, and reads a stranger's small and
//! apart from everyone else's requests. Nothing here decides what anybody may
//! do.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::graceful::GracefulShutdown;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::core::{Core, Knock, OpError};
use crate::{mcp_door, web_door};

/// The numbers, held together so a test can shorten the ones it waits on.
/// A running instance uses [`Limits::KAMOSU`] and nothing else.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// How long a connection has to send a request's headers, counted from
    /// when Kamosu starts waiting for them, so an idle connection kept open
    /// between two requests is let go after the same time. Headers are a few
    /// hundred bytes; 20 seconds is long enough for a phone on one bar of
    /// signal, and a slow sender holds a connection no longer than that.
    pub header: Duration,
    /// How long a request with no Credential has to send its body, which is
    /// at most [`STRANGER_BODY_BYTES`]. 64 KB in 30 seconds is about 17
    /// kilobits a second, slower than any connection a person still uses.
    pub stranger_body: Duration,
    /// How long any request may take from its headers to its answer, the
    /// body included, except a staged upload (below). Ten minutes carries the
    /// largest body a member may send, 40 MB, over an uplink of half a
    /// megabit. An answer streamed after that, a Backup, is not counted.
    pub request: Duration,
    /// How many requests are answered at once. One browser asks for six at
    /// a time over HTTP/1.1 and a household has a few, so 256 is far past
    /// what a kitchen does. Past it, Kamosu answers busy at once rather than
    /// queueing without end. A stranger's body is read before it counts
    /// here, so strangers sending slowly cannot take these places from the
    /// members of the kitchen.
    pub in_flight: usize,
    /// How many strangers' bodies are read at once, apart from the requests
    /// counted above. Past it a stranger is answered busy, and nobody else
    /// is: a crowd of slow strangers holds up only the next stranger. 64
    /// bodies of 64 KB is the most a stranger can make Kamosu hold, 4 MB.
    pub strangers: usize,
}

impl Limits {
    pub const KAMOSU: Limits = Limits {
        header: Duration::from_secs(20),
        stranger_body: Duration::from_secs(30),
        request: Duration::from_secs(10 * 60),
        in_flight: 256,
        strangers: 64,
    };
}

/// The largest body a request with no Credential may carry: 64 KB. The
/// largest such request Kamosu answers is a sign-in or an Invite redeemed,
/// a few hundred bytes of name, password and link; a stranger's Operations
/// (`instance_status`, a Share Link read) carry less. 64 KB leaves room for a
/// long password and a long link with plenty over.
pub const STRANGER_BODY_BYTES: usize = 64 * 1024;

/// The MCP door's one path, whose refusals are JSON-RPC's.
const MCP: &str = "/mcp";

/// The one path the whole-request deadline does not cover. A staged upload is
/// a whole Crouton library streamed to disk, up to 2 GB under its own cap,
/// and it takes a Credential before a byte is written (`stage_upload`).
const STAGED_UPLOADS: &str = "/api/uploads";

/// Serve `app` on `listener` until `shutdown` resolves.
///
/// Kamosu's own accept loop rather than `axum::serve`, because that one gives
/// hyper no timer, and without a timer hyper has no header deadline. HTTP/1.1
/// only: a browser never speaks HTTP/2 without TLS, which Kamosu leaves to the
/// proxy in front, and the header deadline is HTTP/1.1's, so HTTP/2 spoken in
/// the clear would be the one way around it.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    header_deadline: Duration,
    shutdown: impl Future<Output = ()>,
) {
    let graceful = GracefulShutdown::new();
    let mut shutdown = std::pin::pin!(shutdown);
    loop {
        let stream = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(error) => {
                    wait_after_failed_accept(error).await;
                    continue;
                }
            },
            () = &mut shutdown => break,
        };
        let mut builder = hyper::server::conn::http1::Builder::new();
        builder
            .timer(TokioTimer::new())
            .header_read_timeout(header_deadline);
        let connection = graceful.watch(
            builder.serve_connection(TokioIo::new(stream), TowerToHyperService::new(app.clone())),
        );
        tokio::spawn(async move {
            let _ = connection.await;
        });
    }
    drop(listener);
    graceful.shutdown().await;
}

/// A connection that went away before it was accepted is nothing to wait
/// for. Anything else, such as running out of file descriptors, is waited
/// out for a second rather than retried in a tight loop, as `axum::serve`
/// does.
async fn wait_after_failed_accept(error: std::io::Error) {
    use std::io::ErrorKind;
    if matches!(
        error.kind(),
        ErrorKind::ConnectionRefused | ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
    ) {
        return;
    }
    tracing::error!("cannot accept a connection: {error}");
    tokio::time::sleep(Duration::from_secs(1)).await;
}

/// Put the limits around the assembled app, outermost first: a stranger's
/// body read small ([`meet`]), then the cap on requests in flight, then the
/// whole-request deadline. A request turned away busy never started its
/// clock, and a stranger still sending its body holds no place a member
/// needs.
pub fn hold(app: Router, core: Arc<Core>, limits: Limits) -> Router {
    app.layer(middleware::from_fn_with_state(limits, within_deadline))
        .layer(middleware::from_fn_with_state(
            Arc::new(Semaphore::new(limits.in_flight)),
            one_of_so_many,
        ))
        .layer(middleware::from_fn_with_state(
            Gate {
                core,
                limits,
                strangers: Arc::new(Semaphore::new(limits.strangers)),
            },
            meet,
        ))
}

#[derive(Clone)]
struct Gate {
    core: Arc<Core>,
    limits: Limits,
    strangers: Arc<Semaphore>,
}

/// A request that carries a body, met before it is read. A Secret naming
/// somebody goes on to the member's limits. One naming nobody is answered at
/// once with the refusal `execute` would give. No Secret, or a sign-in, which
/// a browser with a dead cookie must still reach, has its body read here as a
/// stranger's. So does the MCP door's, which answers a Secret naming nobody in
/// JSON-RPC once it has read the request. A GET carries no body to read.
async fn meet(State(gate): State<Gate>, request: Request, next: Next) -> Response {
    if matches!(*request.method(), Method::GET | Method::HEAD) {
        return next.run(request).await;
    }
    let path = request.uri().path();
    let mcp = path == MCP;
    let knock = if path.starts_with("/auth/") {
        Ok(Knock::Stranger)
    } else {
        gate.core
            .knock(web_door::bearer_from_headers(request.headers()).as_deref())
    };
    match knock {
        Ok(Knock::Somebody) => return next.run(request).await,
        Err(refused) if !mcp => return web_door::refusal_of_secret(request.headers(), refused),
        Ok(Knock::Stranger) | Err(_) => {}
    }
    let Ok(place) = gate.strangers.clone().try_acquire_owned() else {
        return busy();
    };
    let read = read_stranger_body(request, gate.limits).await;
    drop(place);
    match read {
        Ok(request) => next.run(request).await,
        Err(turned) => {
            let (status, sentence) = turned.status_and_sentence(gate.limits);
            if mcp {
                mcp_door::refusal_before_reading(status, &sentence)
            } else {
                web_door::refusal_at(status, OpError::bad_request(sentence))
            }
        }
    }
}

async fn within_deadline(State(limits): State<Limits>, request: Request, next: Next) -> Response {
    if request.uri().path() == STAGED_UPLOADS {
        return next.run(request).await;
    }
    match tokio::time::timeout(limits.request, next.run(request)).await {
        Ok(response) => response,
        Err(_) => web_door::refusal_at(
            StatusCode::REQUEST_TIMEOUT,
            OpError::bad_request(format!(
                "the request took longer than {} minutes, and was let go",
                limits.request.as_secs() / 60
            )),
        ),
    }
}

async fn one_of_so_many(
    State(slots): State<Arc<Semaphore>>,
    request: Request,
    next: Next,
) -> Response {
    let Ok(_slot) = slots.try_acquire_owned() else {
        return busy();
    };
    next.run(request).await
}

fn busy() -> Response {
    web_door::refusal_at(
        StatusCode::SERVICE_UNAVAILABLE,
        OpError::wait(
            1,
            "Kamosu is answering as many requests as it takes at once — try again in a moment",
        ),
    )
}

/// Why a stranger's body was turned away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turned {
    TooLarge,
    TooSlow,
    Broken,
}

impl Turned {
    /// The status and the sentence, the same at both Doors.
    pub fn status_and_sentence(self, limits: Limits) -> (StatusCode, String) {
        match self {
            Turned::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                format!(
                    "a request with no Credential carries at most {} KB",
                    STRANGER_BODY_BYTES / 1024
                ),
            ),
            Turned::TooSlow => (
                StatusCode::REQUEST_TIMEOUT,
                format!(
                    "a request with no Credential has {} seconds to arrive, and this one took longer",
                    limits.stranger_body.as_secs()
                ),
            ),
            Turned::Broken => (
                StatusCode::BAD_REQUEST,
                "the request stopped partway".to_string(),
            ),
        }
    }
}

/// Read a stranger's body before any Door sees it, at most
/// [`STRANGER_BODY_BYTES`] of it within `limits.stranger_body`, and hand the
/// request on carrying what was read. A body declared too large is refused
/// before a byte of it is read.
pub async fn read_stranger_body(request: Request, limits: Limits) -> Result<Request, Turned> {
    let (parts, body) = request.into_parts();
    let declared = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if declared.is_some_and(|length| length > STRANGER_BODY_BYTES as u64) {
        return Err(Turned::TooLarge);
    }
    let read = tokio::time::timeout(limits.stranger_body, read_at_most(body))
        .await
        .map_err(|_| Turned::TooSlow)??;
    Ok(Request::from_parts(parts, Body::from(read)))
}

async fn read_at_most(body: Body) -> Result<Bytes, Turned> {
    use futures_util::StreamExt;

    let mut stream = body.into_data_stream();
    let mut read = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| Turned::Broken)?;
        if read.len() + chunk.len() > STRANGER_BODY_BYTES {
            return Err(Turned::TooLarge);
        }
        read.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(read))
}

/// Whether a request came from a page of this instance's own, or from no page
/// at all. A browser sends `Origin` on every request a page makes that could
/// change something, a page on another site included; an agent, curl and the
/// Runner send none, and are let through. A page's own requests carry the
/// address it was loaded from, which is the address the request was sent to,
/// so its authority is the request's `Host`.
///
/// The public address is the instance's own as well, for a proxy that hands
/// Kamosu a `Host` of its own making: nginx, unless told to pass it on.
pub fn from_our_own_origin(headers: &HeaderMap, public_address: Option<&str>) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    if public_address
        .and_then(origin_of)
        .is_some_and(|ours| ours.eq_ignore_ascii_case(origin))
    {
        return true;
    }
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
    else {
        return false;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
}

/// Whether a request was sent to the public address over HTTPS (#368).
/// Kamosu never sees TLS, which the proxy ends, and reads no header a proxy
/// adds (ADR 0033). What it has is the public address the Operator gave and
/// the `Host` the request was sent to: an `https://` address whose host is
/// this request's is a request that came over HTTPS.
pub fn came_over_https(headers: &HeaderMap, public_address: Option<&str>) -> bool {
    let Some(address) = public_address.filter(|address| address.starts_with("https://")) else {
        return false;
    };
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
    else {
        return false;
    };
    origin_of(address)
        .and_then(|origin| origin.split_once("://"))
        .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
}

/// `https://recipes.example` out of `https://recipes.example/kitchen`: the
/// scheme and the authority, which is all an `Origin` holds.
fn origin_of(address: &str) -> Option<&str> {
    let (scheme, rest) = address.split_once("://")?;
    let authority_ends = rest.find('/').unwrap_or(rest.len());
    Some(&address[..scheme.len() + 3 + authority_ends])
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(name.clone(), HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn no_origin_is_let_through() {
        assert!(from_our_own_origin(
            &headers(&[(header::HOST, "recipes.example")]),
            None
        ));
    }

    #[test]
    fn an_origin_is_our_own_when_it_names_the_host_it_was_sent_to() {
        let sent = headers(&[
            (header::HOST, "192.0.2.10:5266"),
            (header::ORIGIN, "http://192.0.2.10:5266"),
        ]);
        assert!(from_our_own_origin(&sent, None));
        let sent = headers(&[
            (header::HOST, "Recipes.Example"),
            (header::ORIGIN, "https://recipes.example"),
        ]);
        assert!(from_our_own_origin(&sent, None));
    }

    #[test]
    fn a_sibling_subdomain_is_not_our_own() {
        let sent = headers(&[
            (header::HOST, "recipes.example"),
            (header::ORIGIN, "https://evil.example"),
        ]);
        assert!(!from_our_own_origin(&sent, None));
        let sent = headers(&[
            (header::HOST, "recipes.home.example"),
            (header::ORIGIN, "https://photos.home.example"),
        ]);
        assert!(!from_our_own_origin(
            &sent,
            Some("https://recipes.home.example")
        ));
        let sent = headers(&[(header::HOST, "recipes.example"), (header::ORIGIN, "null")]);
        assert!(!from_our_own_origin(&sent, None));
    }

    #[test]
    fn the_public_address_is_our_own_whatever_host_a_proxy_wrote() {
        let sent = headers(&[
            (header::HOST, "127.0.0.1:5266"),
            (header::ORIGIN, "https://recipes.example"),
        ]);
        assert!(from_our_own_origin(
            &sent,
            Some("https://recipes.example/kitchen")
        ));
        assert!(!from_our_own_origin(&sent, None));
    }

    fn stranger(body: Body) -> Request {
        Request::builder().body(body).unwrap()
    }

    #[tokio::test]
    async fn a_strangers_body_is_read_up_to_the_cap_and_no_further() {
        let at_the_cap = vec![b'a'; STRANGER_BODY_BYTES];
        let read = read_stranger_body(stranger(Body::from(at_the_cap)), Limits::KAMOSU)
            .await
            .expect("a body at the cap is read");
        let bytes = axum::body::to_bytes(read.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(bytes.len(), STRANGER_BODY_BYTES);

        // Sent with no length declared, as a chunked body is.
        let over = futures_util::stream::iter(
            [vec![b'a'; STRANGER_BODY_BYTES], vec![b'a'; 1]]
                .map(|chunk| Ok::<_, std::io::Error>(Bytes::from(chunk))),
        );
        assert_eq!(
            read_stranger_body(stranger(Body::from_stream(over)), Limits::KAMOSU)
                .await
                .err(),
            Some(Turned::TooLarge)
        );
    }

    #[test]
    fn https_is_judged_from_the_public_address() {
        let sent = headers(&[(header::HOST, "recipes.example")]);
        assert!(came_over_https(&sent, Some("https://recipes.example")));
        assert!(!came_over_https(&sent, Some("http://recipes.example")));
        assert!(!came_over_https(&sent, None));
        let by_its_address = headers(&[(header::HOST, "192.0.2.10:5266")]);
        assert!(!came_over_https(
            &by_its_address,
            Some("https://recipes.example")
        ));
    }
}
