//! What every answer tells the browser the page may do (#139, #368).
//!
//! The headers are stamped on every response by one layer over the whole app,
//! so no route can be added without them. ADR 0033 holds that Kamosu on its
//! bare port is as safe as Kamosu behind a proxy, which puts these in Kamosu
//! rather than in a proxy config each Operator would have to write.
//!
//! Each is the second line behind a defence that already holds: the
//! Content-Security-Policy behind "the app never inserts raw HTML", the frame
//! ban behind the `SameSite` Session cookie, `nosniff` behind re-encoded
//! Photographs, and the Referrer-Policy behind the browser's own default,
//! which already keeps a Share Link's Secret off other sites. The four #368
//! added: Strict-Transport-Security behind the proxy's own redirect to HTTPS,
//! the Permissions-Policy behind an app that never asks for what it denies,
//! and the two Cross-Origin policies behind the browser keeping one site's
//! pages and answers from another's.

use std::sync::{Arc, OnceLock};

use axum::extract::{Request, State};
use axum::http::{HeaderValue, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::admission;
use crate::core::Core;
use crate::interface;

/// The layer's whole work: the same headers on whatever was answered, and
/// two that depend on the request.
pub async fn stamp(State(core): State<Arc<Core>>, request: Request, next: Next) -> Response {
    let over_https =
        admission::came_over_https(request.headers(), core.public_address().as_deref());
    // The Share Link page and everything under it: its picture, its Sheet,
    // and the recipe file another Kamosu fetches (ADR 0018, #169).
    let shared = request.uri().path().starts_with("/s/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_SECURITY_POLICY, content_security_policy());
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // A link followed from a Share Link, an Invite or a recovery page carries
    // nothing of the address to another site, the Secret in its path included.
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    // A year, and this host alone: a homelab's other names may still be
    // plain HTTP, so `includeSubDomains` would break them. Only on an answer
    // that came over HTTPS; a browser ignores it over plain HTTP anyway.
    if over_https {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    headers.insert(
        axum::http::HeaderName::from_static("permissions-policy"),
        permissions_policy(),
    );
    // A window Kamosu opens keeps its hold on Kamosu's own pages only, and a
    // page on another site that opened Kamosu cannot reach into it.
    headers.insert(
        axum::http::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    // Another site's page cannot load Kamosu's answers as its own pictures
    // or scripts, except what a Share Link hands to anyone holding it.
    headers.insert(
        axum::http::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static(if shared {
            "cross-origin"
        } else {
            "same-origin"
        }),
    );
    response
}

/// What a page may never ask the browser for, because Kamosu never does.
///
/// Left out on purpose, so allowed to Kamosu's own pages as the browser's
/// default: the camera, which the cooking screen's photograph input opens;
/// keeping the screen awake while cooking; copying to the clipboard; and
/// sharing a file or a link. Only names every current browser knows are
/// listed, since Chrome reports an unknown one as an error on every page.
fn permissions_policy() -> HeaderValue {
    HeaderValue::from_static(
        "accelerometer=(), browsing-topics=(), display-capture=(), geolocation=(), \
         gyroscope=(), hid=(), magnetometer=(), microphone=(), midi=(), payment=(), \
         serial=(), usb=()",
    )
}

/// One policy for every answer. It names the shell's inline bootstrap by hash
/// rather than allowing inline script at all, so a script that got into a page
/// any other way does not run.
///
/// A release build embeds the shell, so its hash is worked out once. A debug
/// build reads `ui/build` from disk and `just ui-build` can replace it under a
/// running server, so there it is worked out on every answer.
fn content_security_policy() -> HeaderValue {
    static POLICY: OnceLock<HeaderValue> = OnceLock::new();
    if cfg!(debug_assertions) {
        policy_for(&interface::shell_script_hashes())
    } else {
        POLICY
            .get_or_init(|| policy_for(&interface::shell_script_hashes()))
            .clone()
    }
}

fn policy_for(script_hashes: &[String]) -> HeaderValue {
    let mut scripts = String::from("'self'");
    for hash in script_hashes {
        scripts.push(' ');
        scripts.push_str(hash);
    }
    let policy = [
        "default-src 'self'".to_string(),
        format!("script-src {scripts}"),
        // Inline style is allowed, and taken on purpose: the shell wraps the
        // app in `<div style="display: contents">`, a dozen Svelte templates
        // set `style=` (the Cover, the tiles, the hero), and so does the Share
        // Link page's Cover. A style cannot run code; script is what this
        // policy is for.
        "style-src 'self' 'unsafe-inline'".to_string(),
        // `blob:` is a Photograph taken offline, shown from the outbox before
        // it reaches the server. `data:` is a Crouton import's favicon.
        "img-src 'self' blob: data:".to_string(),
        "object-src 'none'".to_string(),
        "base-uri 'self'".to_string(),
        "form-action 'self'".to_string(),
        // Only a header can say this; a `<meta>` policy ignores it.
        "frame-ancestors 'none'".to_string(),
    ]
    .join("; ");
    HeaderValue::from_str(&policy).expect("a policy is plain ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_script_is_allowed_only_by_hash() {
        let policy = policy_for(&["'sha256-abc='".to_string()]);
        let policy = policy.to_str().unwrap();
        assert!(
            policy.contains("script-src 'self' 'sha256-abc='"),
            "{policy}"
        );
        let scripts = policy
            .split(';')
            .find(|d| d.trim().starts_with("script-src"))
            .unwrap();
        assert!(!scripts.contains("unsafe"), "{policy}");
    }
}
