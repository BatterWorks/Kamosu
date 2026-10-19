//! Passkeys and changing your own password (#370), through the real Doors.
//!
//! The device below makes and presents passkeys as a phone does, signing for
//! real, so each test runs the whole WebAuthn exchange against the instance's
//! own checks rather than a stand-in for them.

mod support;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const ADDRESS: &str = "https://recipes.kamosu.example";
const PASSWORD: &str = "the right password";

/// A phone holding at most one passkey, which it offers unasked, as a
/// keychain does. It answers in the JSON a browser's `toJSON()` writes, with
/// the attestation "none" every passkey Kamosu asks for carries, and it signs
/// with ES256. Written here rather than taken from a library: the one that
/// does this turns on `serde_json`'s `preserve_order` for the whole build, and
/// the tests would then run Kamosu with its JSON written in another order
/// from the one it ships with.
struct Device {
    key: SigningKey,
    passkey: Option<Held>,
}

struct Held {
    credential_id: Vec<u8>,
    user_handle: Vec<u8>,
    rp_id: String,
    count: u32,
}

fn a_device() -> Device {
    use std::sync::atomic::{AtomicU64, Ordering};
    static MADE: AtomicU64 = AtomicU64::new(0);
    let seed = Sha256::digest(format!("device {}", MADE.fetch_add(1, Ordering::Relaxed)));
    Device {
        key: SigningKey::from_slice(&seed).expect("a key"),
        passkey: None,
    }
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(text: &Value) -> Vec<u8> {
    URL_SAFE_NO_PAD
        .decode(text.as_str().expect("base64url text"))
        .expect("base64url")
}

/// CBOR, as much of it as a passkey needs.
fn cbor_head(major: u8, n: u64) -> Vec<u8> {
    let major = major << 5;
    match n {
        0..=23 => vec![major | n as u8],
        24..=0xff => vec![major | 24, n as u8],
        0x100..=0xffff => [vec![major | 25], (n as u16).to_be_bytes().to_vec()].concat(),
        _ => [vec![major | 26], (n as u32).to_be_bytes().to_vec()].concat(),
    }
}
fn cbor_int(n: i64) -> Vec<u8> {
    if n >= 0 {
        cbor_head(0, n as u64)
    } else {
        cbor_head(1, (-1 - n) as u64)
    }
}
fn cbor_bytes(bytes: &[u8]) -> Vec<u8> {
    [cbor_head(2, bytes.len() as u64), bytes.to_vec()].concat()
}
fn cbor_text(text: &str) -> Vec<u8> {
    [cbor_head(3, text.len() as u64), text.as_bytes().to_vec()].concat()
}

fn client_data(kind: &str, challenge: &Value, origin: &str) -> Vec<u8> {
    json!({ "type": kind, "challenge": challenge, "origin": origin, "crossOrigin": false })
        .to_string()
        .into_bytes()
}

/// User present and user verified, as a face or fingerprint says.
const PRESENT_AND_VERIFIED: u8 = 0x01 | 0x04;
const WITH_A_NEW_CREDENTIAL: u8 = 0x40;

impl Device {
    /// Make a passkey from the options the instance sent, at `origin`.
    fn make(&mut self, origin: &str, options: &Value) -> String {
        let rp_id = options["rp"]["id"].as_str().expect("an rp id").to_string();
        let credential_id = Sha256::digest(format!("{origin} {rp_id} credential")).to_vec();
        let point = self.key.verifying_key().to_sec1_bytes();
        assert_eq!(point.len(), 65, "an uncompressed P-256 point");
        let public_key = [
            cbor_head(5, 5),
            cbor_int(1),
            cbor_int(2), // kty: EC2
            cbor_int(3),
            cbor_int(-7), // alg: ES256
            cbor_int(-1),
            cbor_int(1), // crv: P-256
            cbor_int(-2),
            cbor_bytes(&point[1..33]),
            cbor_int(-3),
            cbor_bytes(&point[33..65]),
        ]
        .concat();
        let auth_data = [
            Sha256::digest(rp_id.as_bytes()).to_vec(),
            vec![PRESENT_AND_VERIFIED | WITH_A_NEW_CREDENTIAL],
            0u32.to_be_bytes().to_vec(),
            vec![0; 16],
            (credential_id.len() as u16).to_be_bytes().to_vec(),
            credential_id.clone(),
            public_key,
        ]
        .concat();
        let attestation = [
            cbor_head(5, 3),
            cbor_text("fmt"),
            cbor_text("none"),
            cbor_text("attStmt"),
            cbor_head(5, 0),
            cbor_text("authData"),
            cbor_bytes(&auth_data),
        ]
        .concat();
        let client_data = client_data("webauthn.create", &options["challenge"], origin);
        self.passkey = Some(Held {
            credential_id: credential_id.clone(),
            user_handle: unb64(&options["user"]["id"]),
            rp_id,
            count: 0,
        });
        json!({
            "id": b64(&credential_id),
            "rawId": b64(&credential_id),
            "type": "public-key",
            "response": {
                "clientDataJSON": b64(&client_data),
                "attestationObject": b64(&attestation),
                "transports": ["internal"],
            },
            "clientExtensionResults": {},
        })
        .to_string()
    }

    /// Answer a sign-in with the passkey it holds, made for whichever
    /// address it was made at, signed as at `origin`.
    fn present(&mut self, origin: &str, options: &Value) -> String {
        let held = self.passkey.as_mut().expect("the device holds a passkey");
        held.count += 1;
        let auth_data = [
            Sha256::digest(held.rp_id.as_bytes()).to_vec(),
            vec![PRESENT_AND_VERIFIED],
            held.count.to_be_bytes().to_vec(),
        ]
        .concat();
        let client_data = client_data("webauthn.get", &options["challenge"], origin);
        let signed: Signature = self
            .key
            .sign(&[auth_data.clone(), Sha256::digest(&client_data).to_vec()].concat());
        json!({
            "id": b64(&held.credential_id),
            "rawId": b64(&held.credential_id),
            "type": "public-key",
            "response": {
                "clientDataJSON": b64(&client_data),
                "authenticatorData": b64(&auth_data),
                "signature": b64(signed.to_der().as_bytes()),
                "userHandle": b64(&held.user_handle),
            },
            "clientExtensionResults": {},
        })
        .to_string()
    }
}

/// The first Person, signed in, on an instance at `ADDRESS`.
fn an_instance(app: &support::TestApp) -> String {
    let create = json!({ "name": "Stéphane", "password": PASSWORD, "session_name": "laptop" });
    let made = app.post_auth_response("/auth/first-person", &create.to_string());
    assert_eq!(made.status, 200, "{}", made.text());
    let secret = support::session_cookie_secret(&made);
    let (status, set) = app.post_op(
        "set_public_address",
        Some(&secret),
        &json!({ "public_address": ADDRESS }).to_string(),
    );
    assert_eq!(status, 200, "{set}");
    secret
}

fn add_a_passkey(app: &support::TestApp, secret: &str, device: &mut Device) -> Value {
    let (status, begun) = app.post_op(
        "start_adding_passkey",
        Some(secret),
        &json!({ "password": PASSWORD }).to_string(),
    );
    assert_eq!(status, 200, "{begun}");
    // Sign-in asks for no name, so the device must offer the passkey unasked.
    let selection = &begun["result"]["options"]["authenticatorSelection"];
    assert_eq!(selection["residentKey"], "required", "{selection}");
    assert_eq!(selection["userVerification"], "required", "{selection}");
    let credential = device.make(ADDRESS, &begun["result"]["options"]);
    let (status, added) = app.post_op(
        "finish_adding_passkey",
        Some(secret),
        &json!({ "ceremony_id": begun["result"]["ceremony_id"], "credential": credential,
                 "name": "iPhone · Safari" })
        .to_string(),
    );
    assert_eq!(status, 200, "{added}");
    added["result"].clone()
}

/// Sign in with whatever passkey the device offers; the status and the body.
fn sign_in(app: &support::TestApp, device: &mut Device, origin: &str) -> (u16, String) {
    let (status, begun) = app.post_auth("/auth/passkey/start", "{}");
    assert_eq!(status, 200, "{begun}");
    let credential = device.present(origin, &begun["result"]["options"]);
    let finished = app.post_auth_response(
        "/auth/passkey/finish",
        &json!({ "ceremony_id": begun["result"]["ceremony_id"], "credential": credential,
                 "session_name": "phone" })
        .to_string(),
    );
    (finished.status, finished.text())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_passkey_added_in_settings_signs_in_with_no_name_or_password() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let mut device = a_device();

    let added = add_a_passkey(&app, &secret, &mut device);
    assert_eq!(added["name"], "iPhone · Safari");
    assert_eq!(added["made_for"], "recipes.kamosu.example");
    assert_eq!(added["works_here"], true);

    let begun = app.post_auth("/auth/passkey/start", "{}").1;
    // Asked from a button, not offered in the name field as autofill (#370):
    // the library marks its options for autofill, and none of that is sent.
    assert!(!begun.to_string().contains("mediation"), "{begun}");
    let credential = device.present(ADDRESS, &begun["result"]["options"]);
    let signed_in = app.post_auth_response(
        "/auth/passkey/finish",
        &json!({ "ceremony_id": begun["result"]["ceremony_id"], "credential": credential,
                 "session_name": "phone" })
        .to_string(),
    );
    assert_eq!(signed_in.status, 200, "{}", signed_in.text());
    let phone = support::session_cookie_secret(&signed_in);
    let (status, sessions) = app.post_op("list_sessions", Some(&phone), "{}");
    assert_eq!(status, 200, "{sessions}");
    assert!(
        sessions.to_string().contains("\"phone\""),
        "the new Session is the phone's: {sessions}"
    );

    let (_, listed) = app.post_op("list_passkeys", Some(&secret), "{}");
    let listed = &listed["result"];
    assert_eq!(listed["available"], true);
    assert_eq!(listed["origin"], ADDRESS);
    assert!(
        listed["passkeys"][0]["last_used_at"].is_string(),
        "{listed}"
    );

    // The same exchange cannot be finished twice.
    let again = app.post_auth_response(
        "/auth/passkey/finish",
        &json!({ "ceremony_id": begun["result"]["ceremony_id"], "credential": credential,
                 "session_name": "phone" })
        .to_string(),
    );
    assert_eq!(again.status, 400, "{}", again.text());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn instance_status_says_where_a_passkey_signs_in() {
    let app = support::spawn_app();
    let (_, before) = app.post_op("instance_status", None, "{}");
    assert_eq!(before["result"]["passkey_origin"], Value::Null);
    an_instance(&app);
    let (_, after) = app.post_op("instance_status", None, "{}");
    assert_eq!(after["result"]["passkey_origin"], ADDRESS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_a_passkey_asks_for_the_password_and_counts_a_wrong_one() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let (status, refused) = app.post_op(
        "start_adding_passkey",
        Some(&secret),
        &json!({ "password": "not the password at all" }).to_string(),
    );
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"]["reason"], "wrong_password");
    // Signed in still: a wrong password here is not a Session ending.
    assert_eq!(app.post_op("list_passkeys", Some(&secret), "{}").0, 200);
    // And it counted against the name, as at the door.
    app.post_op(
        "start_adding_passkey",
        Some(&secret),
        &json!({ "password": "not the password either" }).to_string(),
    );
    let (status, waiting) = app.post_op(
        "start_adding_passkey",
        Some(&secret),
        &json!({ "password": PASSWORD }).to_string(),
    );
    assert_eq!(status, 503, "{waiting}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_access_key_may_list_and_remove_passkeys_but_never_add_one() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let mut device = a_device();
    let added = add_a_passkey(&app, &secret, &mut device);
    let (_, minted) = app.post_op(
        "mint_access_key",
        Some(&secret),
        &json!({ "name": "agent" }).to_string(),
    );
    let key = minted["result"]["secret"]
        .as_str()
        .expect("a key")
        .to_string();

    let (status, refused) = app.post_op(
        "start_adding_passkey",
        Some(&key),
        &json!({ "password": PASSWORD }).to_string(),
    );
    assert_ne!(status, 200, "{refused}");
    let (status, listed) = app.post_op("list_passkeys", Some(&key), "{}");
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["result"]["passkeys"].as_array().unwrap().len(), 1);

    let (status, renamed) = app.post_op(
        "rename_passkey",
        Some(&key),
        &json!({ "passkey_id": added["id"], "name": "Old phone" }).to_string(),
    );
    assert_eq!(status, 200, "{renamed}");
    let (status, removed) = app.post_op(
        "remove_passkey",
        Some(&key),
        &json!({ "passkey_id": added["id"] }).to_string(),
    );
    assert_eq!(status, 200, "{removed}");
    let (status, body) = sign_in(&app, &mut device, ADDRESS);
    assert_eq!(status, 401, "a removed passkey signs in to nothing: {body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_passkey_works_at_the_address_it_was_made_for_and_no_other() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let mut device = a_device();
    add_a_passkey(&app, &secret, &mut device);
    let (_, address) = app.post_op("get_public_address", Some(&secret), "{}");
    assert_eq!(address["result"]["passkeys"], 1, "{address}");

    let moved = "https://cooking.kamosu.example";
    app.post_op(
        "set_public_address",
        Some(&secret),
        &json!({ "public_address": moved }).to_string(),
    );
    let (_, listed) = app.post_op("list_passkeys", Some(&secret), "{}");
    let passkey = &listed["result"]["passkeys"][0];
    assert_eq!(passkey["works_here"], false, "{listed}");
    assert_eq!(passkey["made_for"], "recipes.kamosu.example");
    let (_, address) = app.post_op("get_public_address", Some(&secret), "{}");
    assert_eq!(address["result"]["passkeys"], 0, "{address}");

    // Presented at the new address, it is refused: it proves only the old one.
    let (status, body) = sign_in(&app, &mut device, moved);
    assert_eq!(status, 401, "{body}");

    // Back at the old address, it works again.
    app.post_op(
        "set_public_address",
        Some(&secret),
        &json!({ "public_address": ADDRESS }).to_string(),
    );
    let (status, body) = sign_in(&app, &mut device, ADDRESS);
    assert_eq!(status, 200, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_passkey_presented_from_another_page_is_refused() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let mut device = a_device();
    add_a_passkey(&app, &secret, &mut device);
    // A look-alike page on a sibling address the device would still answer
    // for: the instance checks where the browser was.
    let (status, body) = sign_in(&app, &mut device, "https://evil.recipes.kamosu.example");
    assert_eq!(status, 401, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_instance_with_no_https_name_offers_no_passkeys() {
    for (address, because) in [
        (None, "no_public_address"),
        (Some("http://recipes.kamosu.example"), "not_https"),
        (Some("https://192.0.2.10:5266"), "ip_address"),
    ] {
        let app = support::spawn_app();
        let create = json!({ "name": "Stéphane", "password": PASSWORD, "session_name": "laptop" });
        let secret = support::session_cookie_secret(
            &app.post_auth_response("/auth/first-person", &create.to_string()),
        );
        if let Some(address) = address {
            app.post_op(
                "set_public_address",
                Some(&secret),
                &json!({ "public_address": address }).to_string(),
            );
        }
        let (_, listed) = app.post_op("list_passkeys", Some(&secret), "{}");
        assert_eq!(listed["result"]["available"], false, "{address:?}");
        assert_eq!(listed["result"]["unavailable_because"], because);
        let (status, refused) = app.post_auth("/auth/passkey/start", "{}");
        assert_eq!(status, 400, "{refused}");
        assert_eq!(refused["error"]["reason"], "passkeys_unavailable");
        let (status, refused) = app.post_op(
            "start_adding_passkey",
            Some(&secret),
            &json!({ "password": PASSWORD }).to_string(),
        );
        assert_eq!(status, 400, "{refused}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_disabled_persons_passkey_signs_in_to_nothing() {
    let app = support::spawn_app();
    let operator = an_instance(&app);
    let (_, minted) = app.post_op("mint_invite", Some(&operator), "{}");
    let joined = app.post_auth_response(
        "/auth/invite",
        &json!({ "link": minted["result"]["link"], "name": "Camille",
                 "password": PASSWORD, "session_name": "their phone" })
        .to_string(),
    );
    let camille = support::session_cookie_secret(&joined);
    let mut device = a_device();
    add_a_passkey(&app, &camille, &mut device);
    let (status, ended) = app.post_op(
        "disable_account",
        Some(&operator),
        &json!({ "name": "Camille" }).to_string(),
    );
    assert_eq!(status, 200, "{ended}");
    let (status, body) = sign_in(&app, &mut device, ADDRESS);
    assert_eq!(status, 401, "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changing_your_password_ends_what_you_choose_and_keeps_this_device() {
    let app = support::spawn_app();
    let secret = an_instance(&app);
    let mut device = a_device();
    add_a_passkey(&app, &secret, &mut device);
    let login = json!({ "name": "Stéphane", "password": PASSWORD, "session_name": "phone" });
    let phone =
        support::session_cookie_secret(&app.post_auth_response("/auth/login", &login.to_string()));
    let (_, minted) = app.post_op(
        "mint_access_key",
        Some(&secret),
        &json!({ "name": "agent" }).to_string(),
    );
    let key = minted["result"]["secret"].as_str().unwrap().to_string();

    let change = |current: &str, new: &str, sign_out: bool, keys: bool| {
        app.post_op(
            "change_password",
            Some(&secret),
            &json!({ "current_password": current, "new_password": new,
                     "sign_out_other_devices": sign_out, "end_access_keys": keys })
            .to_string(),
        )
    };
    let (status, short) = change(PASSWORD, "too short", true, false);
    assert_eq!(status, 400, "{short}");
    let (status, wrong) = change(
        "not my password at all",
        "a brand new passphrase",
        true,
        false,
    );
    assert_eq!(status, 400, "{wrong}");
    assert_eq!(wrong["error"]["reason"], "wrong_password");

    let (status, changed) = change(PASSWORD, "a brand new passphrase", true, false);
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["result"]["ended_sessions"], 1);
    assert_eq!(changed["result"]["ended_access_keys"], 0);
    assert_eq!(
        app.post_op("list_jobs", Some(&secret), "{}").0,
        200,
        "this device stays"
    );
    assert_eq!(
        app.post_op("list_jobs", Some(&phone), "{}").0,
        401,
        "the phone is signed out"
    );
    assert_eq!(
        app.post_op("list_jobs", Some(&key), "{}").0,
        200,
        "the key was kept"
    );

    assert_eq!(app.post_auth("/auth/login", &login.to_string()).0, 401);
    let login = json!({ "name": "Stéphane", "password": "a brand new passphrase", "session_name": "phone" });
    assert_eq!(app.post_auth("/auth/login", &login.to_string()).0, 200);
    let (status, body) = sign_in(&app, &mut device, ADDRESS);
    assert_eq!(status, 200, "the passkey keeps working: {body}");

    let (status, changed) = app.post_op(
        "change_password",
        Some(&secret),
        &json!({ "current_password": "a brand new passphrase", "new_password": "and another passphrase",
                 "sign_out_other_devices": false, "end_access_keys": true })
        .to_string(),
    );
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["result"]["ended_sessions"], 0);
    assert_eq!(changed["result"]["ended_access_keys"], 1);
    assert_eq!(
        app.post_op("list_jobs", Some(&key), "{}").0,
        401,
        "the key ended"
    );

    // An Access Key never changes the password.
    let (_, minted) = app.post_op(
        "mint_access_key",
        Some(&secret),
        &json!({ "name": "agent two" }).to_string(),
    );
    let key = minted["result"]["secret"].as_str().unwrap().to_string();
    let (status, refused) = app.post_op(
        "change_password",
        Some(&key),
        &json!({ "current_password": "and another passphrase", "new_password": "an agent's passphrase",
                 "sign_out_other_devices": false, "end_access_keys": false })
        .to_string(),
    );
    assert_ne!(status, 200, "{refused}");
}
