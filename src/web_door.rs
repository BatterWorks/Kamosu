//! The web door: an Axum router built by walking the Catalogue at startup.
//!
//! The API is deliberately RPC-shaped (`POST /api/op/<name>`), not REST-shaped.
//! "Fixing" that would destroy the parity guarantee (ADR 0001) — there is no
//! hand-written route here, only the Catalogue walked once.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::admission;
use crate::catalogue;
use crate::core::{Core, ErrorKind, OpError};
use crate::photographs::DisplaySize;

/// Build the whole web door from the Catalogue. One route per Operation; nothing
/// else. A permission check here would be a bug — authorisation lives in the Core.
pub fn router(core: Arc<Core>) -> Router {
    // The unknown-Operation answer is scoped to `/api/op/…`, not to the whole
    // router: every other path belongs to the interface, which is merged after
    // the Doors and owns the fallback. `/api/op/anything-else` is still a
    // Catalogue question, and still answered as one.
    let first_core = core.clone();
    let login_core = core.clone();
    let invite_core = core.clone();
    let recovery_core = core.clone();
    let passkey_start_core = core.clone();
    let passkey_finish_core = core.clone();
    // Credential minting is deliberately outside the Catalogue: it precedes
    // every Operation, then the cookie becomes the Credential the Door carries.
    // Whoever signs in has no Credential yet, so every one of these is read as
    // a stranger's request is (`admission::meet`, #368).
    let signing_in = Router::new()
        .route(
            "/auth/first-person",
            post(move |body: Option<Json<Value>>| {
                with_a_password(first_core.clone(), move |core| {
                    authenticate_first_person(core, body)
                })
            }),
        )
        .route(
            "/auth/login",
            post(move |body: Option<Json<Value>>| {
                with_a_password(login_core.clone(), move |core| {
                    authenticate_login(core, body)
                })
            }),
        )
        .route(
            "/auth/invite",
            post(move |body: Option<Json<Value>>| {
                with_a_password(invite_core.clone(), move |core| {
                    authenticate_invite(core, body)
                })
            }),
        )
        .route(
            "/auth/recover",
            post(move |body: Option<Json<Value>>| {
                with_a_password(recovery_core.clone(), move |core| {
                    authenticate_recovery(core, body)
                })
            }),
        )
        // A passkey signs in with no password, so neither route waits on
        // password work (#370). The challenge is kept by the Core between them.
        .route(
            "/auth/passkey/start",
            post(move || {
                let core = passkey_start_core.clone();
                async move { respond(core.start_passkey_sign_in()) }
            }),
        )
        .route(
            "/auth/passkey/finish",
            post(move |body: Option<Json<Value>>| {
                let core = passkey_finish_core.clone();
                async move { respond(authenticate_passkey(&core, body)) }
            }),
        )
        .route_layer(middleware::from_fn_with_state(
            core.clone(),
            admit_from_our_own_origin,
        ));
    let mut router = Router::new()
        .route("/api/op/{*name}", any(unknown_operation))
        .merge(signing_in);
    // Photographs travel out of band, authenticated with the same Credential
    // as any Operation (ADR 0001) — raw bytes rather than a JSON envelope, so
    // they never pass through the Catalogue's dispatch. `upload_photograph`
    // is the base64 fallback for a Door that cannot carry raw bytes at all.
    let upload_core = core.clone();
    let photograph_core = core.clone();
    let display_core = core.clone();
    router = router
        .route(
            "/api/photographs",
            post(move |headers: HeaderMap, body: Bytes| {
                let core = upload_core.clone();
                async move { upload_photograph(&core, &headers, &body) }
            })
            .route_layer(middleware::from_fn_with_state(
                core.clone(),
                admit_from_our_own_origin,
            )),
        )
        .route(
            "/api/photographs/{hash}",
            get(move |Path(hash): Path<String>, headers: HeaderMap| {
                let core = photograph_core.clone();
                async move { get_photograph(&core, &headers, &hash) }
            }),
        )
        .route(
            "/api/photographs/{hash}/{size}",
            get(
                move |Path((hash, size)): Path<(String, String)>, headers: HeaderMap| {
                    let core = display_core.clone();
                    async move { get_display_copy(&core, &headers, &hash, &size) }
                },
            ),
        );
    // A file too large for any envelope — a whole Crouton library (#69) —
    // is staged the same way, streamed to disk rather than held in memory,
    // and named afterwards by the Operation that reads it. The Door's body
    // limit is lifted here only because the stream enforces its own.
    let staging_core = core.clone();
    router = router.route(
        "/api/uploads",
        post(move |headers: HeaderMap, body: Body| {
            let core = staging_core.clone();
            async move { stage_upload(&core, &headers, body).await }
        })
        .layer(axum::extract::DefaultBodyLimit::disable())
        .route_layer(middleware::from_fn_with_state(
            core.clone(),
            admit_from_our_own_origin,
        )),
    );
    // A Backup travels the same way and for the same reason (#78): it is one
    // archive of the whole instance, far past anything a JSON envelope should
    // carry, so it is fetched as its own bytes under the same Credential.
    // `list_backups` names what there is to fetch, and it is an ordinary
    // Operation, so both Doors list Backups even though only this one can hand
    // the bytes over.
    let backup_core = core.clone();
    router = router.route(
        "/api/backups/{name}",
        get(move |Path(name): Path<String>, headers: HeaderMap| {
            let core = backup_core.clone();
            async move { get_backup(&core, &headers, &name).await }
        }),
    );
    // A Bundle travels as its own bytes too (#66): `export_bundle` says what it
    // holds at both Doors, and this hands it over under the same Credential.
    let bundle_core = core.clone();
    router = router.route(
        "/api/bundles/{branch_id}",
        get(move |Path(branch_id): Path<String>, headers: HeaderMap| {
            let core = bundle_core.clone();
            async move { get_bundle(core, headers, branch_id).await }
        }),
    );
    // A Sheet is a PDF (#75), made as a Job and fetched here as its own bytes
    // under the same Credential the Job was asked with — or none, when a
    // stranger asked through a Share Link.
    let sheet_core = core.clone();
    router = router.route(
        "/api/sheets/{job_id}",
        get(move |Path(job_id): Path<String>, headers: HeaderMap| {
            let core = sheet_core.clone();
            async move { get_sheet(core, headers, job_id).await }
        }),
    );
    // An Original travels as its own bytes too (#269), to the one Credential
    // that may read it: its Job's own Job Key, which the Core checks.
    let original_core = core.clone();
    router = router.route(
        "/api/originals/{job_id}",
        get(move |Path(job_id): Path<String>, headers: HeaderMap| {
            let core = original_core.clone();
            async move { get_original(core, headers, job_id).await }
        }),
    );
    for op in catalogue::OPERATIONS.iter() {
        let origin_core = core.clone();
        let core = core.clone();
        router = router.route(
            &format!("/api/op/{}", op.name),
            post(
                move |headers: HeaderMap, body: Result<Json<Value>, JsonRejection>| async move {
                    let input = match body {
                        Ok(Json(input)) => input,
                        Err(rejection) => return refused_input(rejection),
                    };
                    let answered = call_operation(&core, op.name, &headers, input);
                    respond_to(&headers, answered)
                },
            )
            .route_layer(middleware::from_fn_with_state(
                origin_core,
                admit_from_our_own_origin,
            )),
        );
    }
    router
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(())
}

/// Axum's own default body limit is 2 MB — plenty for a recipe, nowhere near
/// enough for a camera-original photograph carried as base64 (a third larger
/// again than its own bytes) or raw. 40 MB comfortably covers a modern phone's
/// photo either way, with room to spare. It is a member's: a request with no
/// Credential carries `admission::STRANGER_BODY_BYTES` (#368).
const MAX_BODY_BYTES: usize = 40 * 1024 * 1024;

/// Every route that changes something meets its `Origin` first (#368). A
/// stranger's body was read small before it got here, and a Secret naming
/// nobody was answered (`admission::meet`).
async fn admit_from_our_own_origin(
    State(core): State<Arc<Core>>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(refused) = refuse_another_origin(&core, request.headers()) {
        return refused;
    }
    next.run(request).await
}

/// A page on another site is refused before a member's body is read (#368).
/// The browser sends its Session cookie along with a request from a sibling
/// subdomain, which counts as the same site, so `SameSite` alone does not
/// keep that page out. Refused as a bad request rather than as unauthorized,
/// which the interface reads as a Session that has ended and signs out.
fn refuse_another_origin(core: &Core, headers: &HeaderMap) -> Option<Response> {
    if admission::from_our_own_origin(headers, core.public_address().as_deref()) {
        return None;
    }
    Some(refusal_at(
        StatusCode::FORBIDDEN,
        OpError::bad_request(
            "this request came from a page on another address, and Kamosu answers only its own \
             pages. If a proxy stands in front of Kamosu, it must pass the Host header on \
             unchanged, or the public address must be set",
        ),
    ))
}

/// An Operation's input that could not be read as JSON. Sending it with any
/// `Content-Type` but `application/json` is refused too, rather than run with
/// an empty input (#368): a page on another site can send a body with no
/// type at all, and that must not reach an Operation that takes none.
fn refused_input(rejection: JsonRejection) -> Response {
    let status = rejection.status();
    let sentence = match rejection {
        JsonRejection::MissingJsonContentType(_) => {
            "an Operation's input is JSON, sent with `Content-Type: application/json`".to_string()
        }
        other => other.body_text(),
    };
    refusal_at(status, OpError::bad_request(sentence))
}

/// Reached only when no exact route claimed the path, so the name is one the
/// Catalogue does not declare. Naming it back is the whole of the answer.
async fn unknown_operation(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    respond(Err(OpError::unknown_operation(&name)))
}

/// Run an `/auth/…` route off the request workers (#138). Every one hashes or
/// checks a password, ~20 MiB of Argon2 each, and may wait for one of the few
/// threads allowed to; on a Tokio worker either would stall every other
/// request that worker carries. The blocking pool is where that belongs.
async fn with_a_password(
    core: Arc<Core>,
    route: impl FnOnce(&Core) -> Result<Value, OpError> + Send + 'static,
) -> Response {
    let answer = tokio::task::spawn_blocking(move || route(&core))
        .await
        .unwrap_or_else(|e| Err(OpError::internal(format!("the password work stopped: {e}"))));
    respond(answer)
}

fn authenticate_first_person(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let name = input.get("name").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("account creation takes { name, password, session_name }")
    })?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("account creation takes { name, password, session_name }")
        })?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("account creation takes { name, password, session_name }")
        })?;
    core.create_first_person(name, password, session_name)
}

fn authenticate_login(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("login takes { name, password, session_name }"))?;
    core.log_in(name, password, session_name)
}

fn authenticate_invite(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let link = input.get("link").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
    })?;
    let name = input.get("name").and_then(Value::as_str).ok_or_else(|| {
        OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
    })?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
        })?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OpError::bad_request("Invite redemption takes { link, name, password, session_name }")
        })?;
    core.redeem_invite(link, name, password, session_name)
}

fn authenticate_recovery(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let link = input
        .get("link")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    let password = input
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request("recovery takes { link, password, session_name }"))?;
    core.redeem_recovery(link, password, session_name)
}

fn authenticate_passkey(core: &Core, body: Option<Json<Value>>) -> Result<Value, OpError> {
    const TAKES: &str = "a passkey sign-in takes { ceremony_id, credential, session_name }";
    let input = body.map(|Json(value)| value).unwrap_or_default();
    let ceremony_id = input
        .get("ceremony_id")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(TAKES))?;
    let credential = input
        .get("credential")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(TAKES))?;
    let session_name = input
        .get("session_name")
        .and_then(Value::as_str)
        .ok_or_else(|| OpError::bad_request(TAKES))?;
    core.finish_passkey_sign_in(ceremony_id, credential, session_name)
}

fn call_operation(
    core: &Core,
    operation_name: &str,
    headers: &HeaderMap,
    input: Value,
) -> Result<Value, OpError> {
    // Transport concern only: carry the Secret to the Core. Deciding anything
    // about it here would be a permission check inside a Door.
    let secret = bearer_from_headers(headers);
    core.execute(secret.as_deref(), operation_name, input)
}

/// `POST /api/photographs`: the raw bytes are the whole body — one picture
/// per request, so no multipart envelope is needed. Answers the same
/// `{ "ok": true, "result": { "photograph_id": ... } }` envelope as an
/// Operation, since this is one in every way but how its bytes travel.
fn upload_photograph(core: &Core, headers: &HeaderMap, body: &[u8]) -> Response {
    let secret = bearer_from_headers(headers);
    match core
        .authenticate_to_store_photograph(secret.as_deref())
        .and_then(|caller| core.upload_photograph(&caller, body))
    {
        Ok(result) => respond_to(headers, Ok(result)),
        Err(err) => respond_to(headers, Err(err)),
    }
}

/// The largest file `POST /api/uploads` stages. A real 86-recipe library
/// is 114 MB, almost all of it photographs; this leaves room for a library
/// many times that size without letting one request fill the disk.
const MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// `POST /api/uploads`: the raw bytes are the whole body, written to a staged
/// file under the sender and answered with its id — the envelope every
/// Operation answers, since the Operation that reads the file comes next.
async fn stage_upload(core: &Core, headers: &HeaderMap, body: Body) -> Response {
    let secret = bearer_from_headers(headers);
    let staged = match core
        .authenticate_for_write(secret.as_deref())
        .and_then(|caller| core.begin_upload(&caller.person_id))
    {
        Ok(staged) => staged,
        Err(err) => return respond_to(headers, Err(err)),
    };
    let (upload_id, path) = staged;
    match write_upload(body, &path).await {
        Ok(()) => respond_to(headers, Ok(json!({ "upload_id": upload_id }))),
        Err(err) => {
            let _ = tokio::fs::remove_file(&path).await;
            respond_to(headers, Err(err))
        }
    }
}

async fn write_upload(body: Body, path: &std::path::Path) -> Result<(), OpError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))?;
    let mut stream = body.into_data_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| OpError::bad_request(format!("the upload stopped partway: {e}")))?;
        written += chunk.len() as u64;
        if written > MAX_UPLOAD_BYTES {
            return Err(OpError::bad_request("the file is larger than 2 GB"));
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))?;
    }
    if written == 0 {
        return Err(OpError::bad_request("the upload is empty"));
    }
    file.flush()
        .await
        .map_err(|e| OpError::internal(format!("cannot stage the upload: {e}")))
}

/// `GET /api/photographs/{hash}`: the Photograph's own bytes, WebP.
///
/// The Caller goes through to the Core, which decides whether they may see
/// the picture (#99). Nothing here asks: a check in a Door is a bug (ADR 0001).
fn get_photograph(core: &Core, headers: &HeaderMap, hash: &str) -> Response {
    let secret = bearer_from_headers(headers);
    match core
        .authenticate(secret.as_deref())
        .and_then(|caller| core.read_photograph(&caller, hash))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond_to(headers, Err(err)),
    }
}

/// `GET /api/photographs/{hash}/{size}`: a Display Copy — `card`, `page` or
/// `print` (GLOSSARY.md, "Display Copy") — generated and cached on first ask.
fn get_display_copy(core: &Core, headers: &HeaderMap, hash: &str, size: &str) -> Response {
    let secret = bearer_from_headers(headers);
    let Some(size) = DisplaySize::parse(size) else {
        return respond_to(
            headers,
            Err(OpError::bad_request(
                "a Display Copy is one of 'card', 'page' or 'print'",
            )),
        );
    };
    match core
        .authenticate(secret.as_deref())
        .and_then(|caller| core.read_display_copy(&caller, hash, size))
    {
        Ok(bytes) => image_response(bytes),
        Err(err) => respond_to(headers, Err(err)),
    }
}

/// `GET /api/backups/{name}`: one Backup's archive, zip.
///
/// Sent straight off the disk in chunks rather than read whole into memory
/// first: an archive of a real library is hundreds of megabytes, and buffering
/// one would make fetching a Backup the largest thing the instance ever does
/// to its own memory.
async fn get_backup(core: &Core, headers: &HeaderMap, name: &str) -> Response {
    let secret = bearer_from_headers(headers);
    let path = match core
        .authenticate_for_operator(secret.as_deref())
        .and_then(|_| core.backup_path(name))
    {
        Ok(path) => path,
        Err(err) => return respond_to(headers, Err(err)),
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => return respond_to(headers, Err(OpError::not_found("no such Backup"))),
    };
    // The length is declared rather than left to chunked framing, because the
    // thing on the other end is usually copying half a gigabyte and wants to
    // know how far along it is. An archive is never rewritten in place — a new
    // one gets a new name and the old one is unlinked — so the size read here
    // is the size that arrives.
    let length = match file.metadata().await {
        Ok(metadata) => metadata.len(),
        Err(_) => return respond_to(headers, Err(OpError::not_found("no such Backup"))),
    };
    let stream = tokio_util::io::ReaderStream::new(file);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_LENGTH, length.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

/// `GET /api/bundles/{branch_id}`: one recipe's Bundle, zip (#66).
///
/// Built when asked, on the blocking pool: a Bundle carries its Photographs,
/// and reading and zipping them is file work the runtime answering every other
/// request should not wait behind. The Credential is checked in the Core, as
/// for any Operation — the same circle that may read the Branch may take it.
async fn get_bundle(core: Arc<Core>, headers: HeaderMap, branch_id: String) -> Response {
    let secret = bearer_from_headers(&headers);
    let built = tokio::task::spawn_blocking(move || {
        let caller = core.authenticate(secret.as_deref())?;
        core.bundle(&caller.person_id, &branch_id)
    })
    .await;
    let written = match built {
        Ok(Ok(written)) => written,
        Ok(Err(err)) => return respond_to(&headers, Err(err)),
        Err(e) => {
            return respond_to(
                &headers,
                Err(OpError::internal(format!(
                    "the Bundle could not be built: {e}"
                ))),
            );
        }
    };
    zip_response(&written.file_name, written.bytes)
}

async fn get_sheet(core: Arc<Core>, headers: HeaderMap, job_id: String) -> Response {
    let secret = bearer_from_headers(&headers);
    let read =
        tokio::task::spawn_blocking(move || core.read_sheet(secret.as_deref(), &job_id)).await;
    match read {
        Ok(Ok((name, bytes))) => pdf_response(&name, bytes),
        Ok(Err(err)) => respond_to(&headers, Err(err)),
        Err(e) => respond_to(
            &headers,
            Err(OpError::internal(format!(
                "the Sheet could not be read: {e}"
            ))),
        ),
    }
}

/// `GET /api/originals/{job_id}`: what an agent's Job reads, for its Job Key.
/// Read off the disk on the blocking pool, as a Sheet is.
async fn get_original(core: Arc<Core>, headers: HeaderMap, job_id: String) -> Response {
    let secret = bearer_from_headers(&headers);
    let read =
        tokio::task::spawn_blocking(move || core.read_original(secret.as_deref(), &job_id)).await;
    match read {
        Ok(Ok((kind, bytes))) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, kind.content_type().to_string()),
                (header::CONTENT_LENGTH, bytes.len().to_string()),
                // Read by an agent and never shown, so never opened in place.
                (header::CONTENT_DISPOSITION, "attachment".to_string()),
            ],
            bytes,
        )
            .into_response(),
        Ok(Err(err)) => respond_to(&headers, Err(err)),
        Err(e) => respond_to(
            &headers,
            Err(OpError::internal(format!(
                "the Original could not be read: {e}"
            ))),
        ),
    }
}

/// A Sheet as the browser should get it: shown rather than saved, so printing
/// is the browser's own Print and Kamosu has no dialog of its own (ADR 0023).
pub fn pdf_response(name: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_LENGTH, bytes.len().to_string()),
            (header::CONTENT_DISPOSITION, disposition("inline", name)),
        ],
        bytes,
    )
        .into_response()
}

/// A `Content-Disposition` saying a download is to be saved, naming a file
/// that may not be ASCII.
fn attachment(name: &str) -> String {
    disposition("attachment", name)
}

/// One Bundle, as a download. Shared with the Share Link page, which hands the
/// same zip to a stranger holding the token (#65).
pub fn zip_response(name: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_LENGTH, bytes.len().to_string()),
            (header::CONTENT_DISPOSITION, attachment(name)),
        ],
        bytes,
    )
        .into_response()
}

/// A `Content-Disposition` naming a file that may not be ASCII.
///
/// `filename*` carries the real name, percent-encoded as UTF-8 (RFC 6266), and
/// `filename` an ASCII stand-in for a client too old to read it: a title like
/// `Bœuf bourguignon` is ordinary, and a raw non-ASCII header is not.
fn disposition(kind: &str, name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|ch| {
            if (ch.is_ascii_graphic() && ch != '"' && ch != '\\') || ch == ' ' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("{kind}; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

fn image_response(bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/webp")],
        bytes,
    )
        .into_response()
}

/// Everything about the cookie but its value, written once. Setting and
/// expiring share it because a browser keeps the original unless the expiring
/// one matches it on name and Path — two spellings that drifted apart would
/// leave the fault in place while looking fixed.
const SESSION_COOKIE_ATTRIBUTES: &str = "Path=/; HttpOnly; Secure; SameSite=Lax";

/// The Session cookie's name. The `__Host-` prefix makes a browser refuse it
/// unless it is Secure, set for the whole of this host and for no domain
/// around it (#368), so a page on a sibling subdomain can neither set one
/// nor shadow it with its own. It was `kamosu_session` until then, and the
/// rename signed every browser out once.
const SESSION_COOKIE: &str = "__Host-kamosu_session";

/// How long a browser keeps the Session cookie: 400 days, the most RFC 6265bis
/// lets a cookie ask for and the cap Chrome enforces. Without it the cookie
/// lasts only as long as the browser's own session, which on an iPhone ends
/// whenever iOS closes the app (#137). Kept out of `SESSION_COOKIE_ATTRIBUTES`
/// so the expiring cookie never carries two `Max-Age`s.
const SESSION_COOKIE_MAX_AGE: &str = "Max-Age=34560000";

/// The `Set-Cookie` that hands a browser its Session, or hands it back renewed.
fn issued_session_cookie(secret: &str) -> HeaderValue {
    let cookie =
        format!("{SESSION_COOKIE}={secret}; {SESSION_COOKIE_ATTRIBUTES}; {SESSION_COOKIE_MAX_AGE}");
    HeaderValue::from_str(&cookie).expect("safe session cookie")
}

/// The `Set-Cookie` that takes a Session cookie back (#91).
fn expired_session_cookie() -> HeaderValue {
    let expired = format!("{SESSION_COOKIE}=; {SESSION_COOKIE_ATTRIBUTES}; Max-Age=0");
    HeaderValue::from_str(&expired).expect("safe expiring session cookie")
}

/// Answer a request that carries no Credential of this Door's making: the
/// `/auth/…` routes, which mint a Credential rather than present one, and the
/// reply to an Operation name the Catalogue does not declare. The empty headers
/// say what is true of both — there is no cookie here to take back.
fn respond(result: Result<Value, OpError>) -> Response {
    respond_to(&HeaderMap::new(), result)
}

/// A refusal in this Door's envelope at a status of its own, for an answer
/// given before any Door has read the request: busy, too slow, too large, or
/// from another site (#368). No cookie is taken back by one.
pub fn refusal_at(status: StatusCode, err: OpError) -> Response {
    let mut response = respond(Err(err));
    *response.status_mut() = status;
    response
}

/// The Core's refusal of a request's Secret, answered before its body is
/// read (#368), taking back a Session cookie that names nobody as any
/// refused Operation does (#91).
pub fn refusal_of_secret(headers: &HeaderMap, err: OpError) -> Response {
    respond_to(headers, Err(err))
}

/// Answer one request, taking back its Session cookie where the Core says that
/// cookie names nobody (#91).
fn respond_to(headers: &HeaderMap, result: Result<Value, OpError>) -> Response {
    match result {
        Ok(mut value) => {
            // A browser Session is delivered in an HttpOnly Secure cookie, never
            // exposed to the app's JavaScript. The private field is Core plumbing,
            // removed before the declared Operation result crosses this Door.
            let session_secret = value
                .as_object_mut()
                .and_then(|object| object.remove("_session_secret"))
                .and_then(|secret| secret.as_str().map(str::to_owned));
            // Otherwise, a cookie the Core just accepted is set again with a
            // fresh lifetime, so a browser opened at least once every 400 days
            // is never signed out (#137). The Secret is the one it sent. Only
            // the cookie is renewed: a bearer token wins over it when both
            // arrive, and a bearer token is not this Door's to touch. Signing
            // out renews too, harmlessly — the next request is refused and the
            // refusal below takes the cookie back.
            let session_secret = session_secret.or_else(|| {
                secret_came_from_cookie(headers)
                    .then(|| session_cookie(headers))
                    .flatten()
            });
            let mut response =
                (StatusCode::OK, Json(json!({ "ok": true, "result": value }))).into_response();
            if let Some(secret) = session_secret {
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, issued_session_cookie(&secret));
            }
            response
        }
        Err(err) => {
            let mut response = (status_for(err.kind), Json(error_body(&err))).into_response();
            if let Some(seconds) = err.retry_after_seconds {
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
            }
            // A Session that has ended leaves the browser still holding its
            // cookie, and holding it is what makes even a Public Operation come
            // back refused — so without this the browser waits on the sign-in
            // screen forever (#91). The Core has already decided the Secret
            // names nobody; the Door merely takes back a cookie it set, which is
            // housekeeping rather than a permission check.
            //
            // Only when that cookie is the Secret the Core was actually handed.
            // A bearer token wins over the cookie when both arrive, and a bearer
            // token is not this Door's to take back — so a refused one leaves
            // even a cookie sitting beside it exactly as it was.
            if err.credential_names_nobody && secret_came_from_cookie(headers) {
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, expired_session_cookie());
            }
            response
        }
    }
}

fn error_body(err: &OpError) -> Value {
    let mut error = json!({
        "kind": kind_name(err.kind),
        "message": err.to_sentence(),
    });
    if let Some(seconds) = err.retry_after_seconds {
        error["retry_after_seconds"] = json!(seconds);
    }
    if let Some(reason) = err.reason {
        error["reason"] = json!(reason);
    }
    json!({ "ok": false, "error": error })
}

fn status_for(kind: ErrorKind) -> StatusCode {
    match kind {
        ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorKind::UnknownOperation => StatusCode::NOT_FOUND,
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        // Refused, not failed: a full lane answers busy rather than growing
        // (ADR 0032). The caller is invited back in a moment.
        ErrorKind::Busy => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
        ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn kind_name(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Unauthorized => "unauthorized",
        ErrorKind::UnknownOperation => "unknown_operation",
        ErrorKind::NotFound => "not_found",
        ErrorKind::Busy => "busy",
        ErrorKind::BadRequest => "bad_request",
        ErrorKind::Internal => "internal",
    }
}

/// Pull `Authorization: Bearer <secret>` off a request. Shared by both Doors as a
/// transport detail; what the Secret means is decided in the Core alone.
pub fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    bearer_token(headers).or_else(|| session_cookie(headers))
}

/// Whether the Secret the Core was handed came from the cookie this Door set,
/// which is the only Secret a refusal may ever take back. It reads the same two
/// headers in the same order `bearer_from_headers` does, so the question "whose
/// Secret was refused" can never be answered about a different one.
fn secret_came_from_cookie(headers: &HeaderMap) -> bool {
    bearer_token(headers).is_none() && session_cookie(headers).is_some()
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?;
    let rest = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    Some(rest.trim().to_string())
}

/// The Secret this Door itself put in its Session cookie, if the request
/// carried one back. Kept apart from a bearer token because the two are the
/// Door's business to different degrees: this cookie is Kamosu's own to set and
/// to take back, while a bearer token belongs to whoever sent it and the Door
/// touches nothing of it.
fn session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())?
        .split(';')
        .map(str::trim)
        .find_map(|part| {
            part.strip_prefix(SESSION_COOKIE)
                .and_then(|rest| rest.strip_prefix('='))
                .map(str::to_owned)
        })
}
