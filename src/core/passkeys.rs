//! Passkeys (#370): signing in with the device's own lock instead of the
//! password, adding one from Settings, and the list a Person keeps of theirs.
//!
//! A passkey is bound to one address, the public address's host, which the
//! WebAuthn standard calls the relying party. Nothing about a passkey works
//! where that address is not an `https://` name, and a passkey made for one
//! address does nothing at another, so every passkey row remembers the host it
//! was made for.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use webauthn_rs::prelude::{
    DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Url, Uuid, Webauthn, WebauthnBuilder,
};

use super::*;

/// How many passkey exchanges may be half done at once. A sign-in starts one
/// for anybody who asks, a stranger included, so the store is bounded rather
/// than left to grow with them (ADR 0032): past this, the oldest is dropped,
/// and its browser is told to try again.
const CEREMONIES_HELD: usize = 256;

/// How long a half-done exchange is kept. The browser gives up after five
/// minutes (WebAuthn's own timeout), so one older than this can never finish.
const CEREMONY_LASTS: Duration = Duration::from_secs(10 * 60);

/// The passkey exchanges begun and not yet finished: the challenge Kamosu
/// sent, kept here and never in the browser, which would let a browser replay
/// an old one. Kept in memory: a restart drops them, and the browser asks
/// again.
#[derive(Default)]
pub(super) struct Ceremonies {
    held: Mutex<HashMap<String, Ceremony>>,
}

struct Ceremony {
    begun: Instant,
    /// The address it was begun for. A public address changed in between
    /// leaves it unfinishable.
    rp_id: String,
    step: Step,
}

enum Step {
    Adding {
        person_id: String,
        state: PasskeyRegistration,
    },
    SigningIn {
        state: DiscoverableAuthentication,
    },
}

impl Ceremonies {
    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<String, Ceremony>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn begin(&self, rp_id: &str, step: Step) -> String {
        let id = format!("ceremony_{}", hex::encode(random_bytes(16)));
        let mut held = self.held();
        let now = Instant::now();
        held.retain(|_, ceremony| now.duration_since(ceremony.begun) < CEREMONY_LASTS);
        if held.len() >= CEREMONIES_HELD
            && let Some(oldest) = held
                .iter()
                .min_by_key(|(_, ceremony)| ceremony.begun)
                .map(|(id, _)| id.clone())
        {
            held.remove(&oldest);
        }
        held.insert(
            id.clone(),
            Ceremony {
                begun: now,
                rp_id: rp_id.to_string(),
                step,
            },
        );
        id
    }

    /// The exchange by its id, taken out: each finishes once.
    fn finish(&self, id: &str) -> Option<Ceremony> {
        self.held()
            .remove(id)
            .filter(|ceremony| ceremony.begun.elapsed() < CEREMONY_LASTS)
    }
}

/// Why this instance offers no passkeys, said by name so a screen can say it
/// in its own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// No public address has been set.
    NoAddress,
    /// The public address is `http://`.
    NotHttps,
    /// The public address is an IP address, which the standard refuses as a
    /// relying party.
    IpAddress,
}

impl Unavailable {
    pub fn name(self) -> &'static str {
        match self {
            Unavailable::NoAddress => "no_public_address",
            Unavailable::NotHttps => "not_https",
            Unavailable::IpAddress => "ip_address",
        }
    }
}

/// Where passkeys work on this instance: the public address's host, and the
/// origin a browser must be at to use one.
pub struct Place {
    pub rp_id: String,
    pub origin: Url,
}

impl Place {
    /// The origin as a browser writes it: `https://host` or `https://host:port`.
    pub fn origin_text(&self) -> String {
        self.origin.origin().ascii_serialization()
    }

    fn webauthn(&self) -> Result<Webauthn, OpError> {
        WebauthnBuilder::new(&self.rp_id, &self.origin)
            .and_then(|builder| builder.rp_name("Kamosu").build())
            .map_err(|e| OpError::internal(format!("cannot prepare passkeys here: {e}")))
    }
}

/// Where passkeys work, read off a public address, or why they cannot.
pub fn place_of(public_address: Option<&str>) -> Result<Place, Unavailable> {
    let address = public_address.ok_or(Unavailable::NoAddress)?;
    let url = Url::parse(address).map_err(|_| Unavailable::NoAddress)?;
    if url.scheme() != "https" {
        return Err(Unavailable::NotHttps);
    }
    let rp_id = match url.host() {
        Some(url::Host::Domain(domain)) => domain.to_ascii_lowercase(),
        Some(url::Host::Ipv4(_) | url::Host::Ipv6(_)) => return Err(Unavailable::IpAddress),
        None => return Err(Unavailable::NoAddress),
    };
    let origin =
        Url::parse(&url.origin().ascii_serialization()).map_err(|_| Unavailable::NoAddress)?;
    Ok(Place { rp_id, origin })
}

/// The id a Person's passkeys carry for them, which the device hands back at
/// sign-in to say whose passkey it holds. Made from the Person's id rather
/// than stored: the same Person always gets the same one, and it says nothing
/// about them.
fn user_handle(person_id: &str) -> Uuid {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("kamosu passkey user {person_id}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

/// What a refused passkey sign-in says, whichever part of it failed: the
/// screen offers the password and adding the passkey again.
const NOT_KNOWN: &str = "that passkey is not one this Kamosu knows";

impl Core {
    /// Where passkeys work on this instance now, or why they cannot.
    pub fn passkey_place(&self) -> Result<Place, Unavailable> {
        place_of(self.public_address().as_deref())
    }

    /// A Person's passkeys, and whether this instance offers passkeys at all
    /// (#370). Each says whether it was made for the address passkeys work at
    /// now: one made before the public address changed does nothing here.
    pub fn list_passkeys(&self, person_id: &str) -> Result<Value, OpError> {
        let place = self.passkey_place();
        let here = place.as_ref().ok().map(|place| place.rp_id.clone());
        let passkeys = self.db().with_conn(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT id, name, rp_id, created_at, last_used_at FROM passkeys \
                      WHERE person_id = ?1 ORDER BY created_at DESC, id",
                )
                .map_err(|e| OpError::internal(format!("cannot list passkeys: {e}")))?;
            statement
                .query_map(params![person_id], |row| {
                    let rp_id: String = row.get(2)?;
                    Ok(json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "made_for": rp_id,
                        "works_here": here.as_deref() == Some(rp_id.as_str()),
                        "created_at": row.get::<_, String>(3)?,
                        "last_used_at": row.get::<_, Option<String>>(4)?,
                    }))
                })
                .map_err(|e| OpError::internal(format!("cannot read passkeys: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| OpError::internal(format!("cannot read passkeys: {e}")))
        })?;
        Ok(json!({
            "public_address": self.public_address(),
            "available": place.is_ok(),
            "unavailable_because": place.as_ref().err().map(|why| why.name()),
            "origin": place.as_ref().ok().map(|place| place.origin_text()),
            "passkeys": passkeys,
        }))
    }

    /// The passkeys made for the address passkeys work at now, across every
    /// Person: what changing the public address would leave unable to sign in.
    pub fn passkeys_here(&self) -> Result<i64, OpError> {
        let Ok(place) = self.passkey_place() else {
            return Ok(0);
        };
        self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM passkeys WHERE rp_id = ?1",
                params![place.rp_id],
                |row| row.get(0),
            )
            .map_err(|e| OpError::internal(format!("cannot count passkeys: {e}")))
        })
    }

    /// Begin adding a passkey: the password first (#370), so that whoever
    /// holds an unlocked phone cannot add a passkey of their own that ending
    /// the phone's Session would not reach. Answers what the browser hands to
    /// the device.
    pub fn start_adding_passkey(&self, person_id: &str, password: &str) -> Result<Value, OpError> {
        let place = self.passkey_place().map_err(unavailable)?;
        self.check_own_password(person_id, password)?;
        let (name, made): (String, Vec<Vec<u8>>) = self.db().with_conn(|conn| {
            let name = conn
                .query_row(
                    "SELECT name FROM people WHERE id = ?1",
                    params![person_id],
                    |row| row.get(0),
                )
                .map_err(|e| OpError::internal(format!("cannot read this Person: {e}")))?;
            let made = conn
                .prepare("SELECT credential_id FROM passkeys WHERE person_id = ?1 AND rp_id = ?2")
                .and_then(|mut statement| {
                    statement
                        .query_map(params![person_id, place.rp_id], |row| row.get(0))?
                        .collect::<Result<Vec<Vec<u8>>, _>>()
                })
                .map_err(|e| OpError::internal(format!("cannot read passkeys: {e}")))?;
            Ok((name, made))
        })?;
        let (challenge, state) = place
            .webauthn()?
            .start_passkey_registration(
                user_handle(person_id),
                &name,
                &name,
                // A device already holding one of this Person's passkeys says
                // so rather than making a second.
                (!made.is_empty()).then_some(made),
            )
            .map_err(|e| OpError::internal(format!("cannot begin adding a passkey: {e}")))?;
        let mut options = serde_json::to_value(&challenge)
            .map_err(|e| OpError::internal(format!("cannot write the passkey request: {e}")))?;
        // Sign-in asks for no name, so the device must keep the passkey where
        // it can offer it unasked: a discoverable credential. The library asks
        // for one only if convenient, which a security key may decline.
        options["publicKey"]["authenticatorSelection"]["residentKey"] = json!("required");
        options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = json!(true);
        let ceremony = self.ceremonies.begin(
            &place.rp_id,
            Step::Adding {
                person_id: person_id.to_string(),
                state,
            },
        );
        Ok(json!({ "ceremony_id": ceremony, "options": options["publicKey"] }))
    }

    /// Finish adding a passkey with what the device made, under the name it
    /// is given: the device's, as a Session's is (#370). The credential is the
    /// JSON a browser writes of it, as text: what extensions it reports differ
    /// from one browser to the next, so its shape is the library's to check.
    pub fn finish_adding_passkey(
        &self,
        person_id: &str,
        ceremony_id: &str,
        credential: &str,
        name: &str,
    ) -> Result<Value, OpError> {
        let name = required_text(name, "name")?;
        let credential: RegisterPublicKeyCredential =
            serde_json::from_str(credential).map_err(|e| {
                OpError::bad_request(format!("that is not a passkey a device made: {e}"))
            })?;
        let place = self.passkey_place().map_err(unavailable)?;
        let started_over = || {
            OpError::refused_because(
                "start_again",
                "this passkey was begun too long ago, or somewhere else: add it again",
            )
        };
        let ceremony = self
            .ceremonies
            .finish(ceremony_id)
            .ok_or_else(started_over)?;
        let Step::Adding {
            person_id: begun_by,
            state,
        } = ceremony.step
        else {
            return Err(started_over());
        };
        if begun_by != person_id || ceremony.rp_id != place.rp_id {
            return Err(started_over());
        }
        let passkey = place
            .webauthn()?
            .finish_passkey_registration(&credential, &state)
            .map_err(|e| OpError::bad_request(format!("the device's passkey was refused: {e}")))?;
        let stored = serde_json::to_string(&passkey)
            .map_err(|e| OpError::internal(format!("cannot keep the passkey: {e}")))?;
        let id = format!("pk_{}", hex::encode(random_bytes(8)));
        let created_at: String = self.db().with_conn(|conn| {
            conn.query_row(
                "INSERT INTO passkeys (id, person_id, name, rp_id, credential_id, passkey) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING created_at",
                params![
                    id,
                    person_id,
                    name,
                    place.rp_id,
                    passkey.cred_id().clone(),
                    stored
                ],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(failure, _)
                    if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    OpError::bad_request("this device already holds that passkey here")
                }
                e => OpError::internal(format!("cannot keep the passkey: {e}")),
            })
        })?;
        self.note(
            Event::theirs("passkey_added", person_id).naming(Some(Named {
                kind: "passkey",
                name: name.to_string(),
            })),
        );
        Ok(json!({
            "id": id,
            "name": name,
            "made_for": place.rp_id,
            "works_here": true,
            "created_at": created_at,
            "last_used_at": null,
        }))
    }

    pub fn rename_passkey(
        &self,
        person_id: &str,
        passkey_id: &str,
        name: &str,
    ) -> Result<String, OpError> {
        let name = required_text(name, "name")?;
        let changed = self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE passkeys SET name = ?1 WHERE id = ?2 AND person_id = ?3",
                params![name, passkey_id, person_id],
            )
            .map_err(|e| OpError::internal(format!("cannot rename the passkey: {e}")))
        })?;
        if changed == 0 {
            return Err(OpError::not_found("no passkey with that id is yours"));
        }
        Ok(name.to_string())
    }

    /// Forget a passkey here. The device still holds it until it is deleted
    /// there; it signs in to nothing.
    pub fn remove_passkey(&self, person_id: &str, passkey_id: &str) -> Result<(), OpError> {
        let removed: Option<String> = self.db().with_conn(|conn| {
            conn.query_row(
                "DELETE FROM passkeys WHERE id = ?1 AND person_id = ?2 RETURNING name",
                params![passkey_id, person_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot remove the passkey: {e}")))
        })?;
        let Some(name) = removed else {
            return Err(OpError::not_found("no passkey with that id is yours"));
        };
        self.note(
            Event::theirs("passkey_removed", person_id).naming(Some(Named {
                kind: "passkey",
                name,
            })),
        );
        Ok(())
    }

    /// Begin signing in with a passkey. No name is asked: the device offers
    /// the passkeys it holds for this address, and says whose the one chosen
    /// is. Anybody may ask, so what is held for it is bounded (`Ceremonies`).
    pub fn start_passkey_sign_in(&self) -> Result<Value, OpError> {
        let place = self.passkey_place().map_err(unavailable)?;
        let (challenge, state) = place
            .webauthn()?
            .start_discoverable_authentication()
            .map_err(|e| OpError::internal(format!("cannot begin a passkey sign-in: {e}")))?;
        let options = serde_json::to_value(&challenge)
            .map_err(|e| OpError::internal(format!("cannot write the passkey request: {e}")))?;
        let ceremony = self
            .ceremonies
            .begin(&place.rp_id, Step::SigningIn { state });
        // Only the options: the library marks them for autofill in the name
        // field, and Kamosu asks from a button instead (#370).
        Ok(json!({ "ceremony_id": ceremony, "options": options["publicKey"] }))
    }

    /// Finish a passkey sign-in, and mint the Session it unlocks, as a right
    /// password does.
    pub fn finish_passkey_sign_in(
        &self,
        ceremony_id: &str,
        credential: &str,
        session_name: &str,
    ) -> Result<Value, OpError> {
        let session_name = required_text(session_name, "session_name")?;
        let credential: PublicKeyCredential = serde_json::from_str(credential).map_err(|e| {
            OpError::bad_request(format!("that is not a passkey a device sent: {e}"))
        })?;
        let place = self.passkey_place().map_err(unavailable)?;
        let ceremony = self.ceremonies.finish(ceremony_id).ok_or_else(|| {
            OpError::refused_because(
                "start_again",
                "this sign-in was begun too long ago: try the passkey again",
            )
        })?;
        let Step::SigningIn { state } = ceremony.step else {
            return Err(OpError::unauthorized(NOT_KNOWN));
        };
        if ceremony.rp_id != place.rp_id {
            return Err(OpError::unauthorized(NOT_KNOWN));
        }
        let webauthn = place.webauthn()?;
        let (handle, credential_id) = webauthn
            .identify_discoverable_authentication(&credential)
            .map_err(|_| OpError::unauthorized(NOT_KNOWN))?;
        let found: Option<(String, String, String)> = self.db().with_conn(|conn| {
            conn.query_row(
                "SELECT passkeys.id, passkeys.person_id, passkeys.passkey FROM passkeys \
                   JOIN people ON people.id = passkeys.person_id \
                  WHERE passkeys.credential_id = ?1 AND passkeys.rp_id = ?2 \
                    AND people.disabled = 0 AND people.deleted = 0",
                params![credential_id, place.rp_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|e| OpError::internal(format!("cannot read the passkey: {e}")))
        })?;
        let (passkey_id, person_id, stored) =
            found.ok_or_else(|| OpError::unauthorized(NOT_KNOWN))?;
        if user_handle(&person_id) != handle {
            return Err(OpError::unauthorized(NOT_KNOWN));
        }
        let mut passkey: Passkey = serde_json::from_str(&stored)
            .map_err(|e| OpError::internal(format!("cannot read the kept passkey: {e}")))?;
        let result = webauthn
            .finish_discoverable_authentication(
                &credential,
                state,
                &[DiscoverableKey::from(&passkey)],
            )
            .map_err(|_| OpError::unauthorized(NOT_KNOWN))?;
        passkey.update_credential(&result);
        let stored = serde_json::to_string(&passkey)
            .map_err(|e| OpError::internal(format!("cannot keep the passkey: {e}")))?;
        self.db().with_conn(|conn| {
            conn.execute(
                "UPDATE passkeys SET passkey = ?1, \
                        last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
                params![stored, passkey_id],
            )
            .map_err(|e| OpError::internal(format!("cannot note the passkey's use: {e}")))
        })?;
        let session = self.mint_session(&person_id, session_name, "passkey")?;
        Ok(json!({ "session_id": session.id, "_session_secret": session.secret }))
    }
}

fn unavailable(why: Unavailable) -> OpError {
    OpError::refused_because(
        "passkeys_unavailable",
        match why {
            Unavailable::NoAddress => {
                "this instance offers no passkeys: it has no public address yet"
            }
            Unavailable::NotHttps => {
                "this instance offers no passkeys: its public address is not https://"
            }
            Unavailable::IpAddress => {
                "this instance offers no passkeys: its public address is an IP address, \
                 and a passkey needs a name"
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passkey_works_only_at_an_https_name() {
        assert_eq!(place_of(None).err(), Some(Unavailable::NoAddress));
        assert_eq!(
            place_of(Some("http://recipes.kamosu.example")).err(),
            Some(Unavailable::NotHttps)
        );
        assert_eq!(
            place_of(Some("https://192.0.2.10:5266")).err(),
            Some(Unavailable::IpAddress)
        );
        assert_eq!(
            place_of(Some("https://[2001:db8::1]")).err(),
            Some(Unavailable::IpAddress)
        );
        let place = place_of(Some("https://Recipes.Kamosu.example:8443/kamosu")).unwrap();
        assert_eq!(place.rp_id, "recipes.kamosu.example");
        assert_eq!(place.origin_text(), "https://recipes.kamosu.example:8443");
    }

    #[test]
    fn a_person_keeps_one_user_handle() {
        assert_eq!(user_handle("p_1"), user_handle("p_1"));
        assert_ne!(user_handle("p_1"), user_handle("p_2"));
    }

    #[test]
    fn a_crowd_of_sign_ins_holds_no_more_than_the_cap() {
        let ceremonies = Ceremonies::default();
        let place = place_of(Some("https://recipes.kamosu.example")).unwrap();
        let webauthn = place.webauthn().unwrap();
        let mut first = None;
        for _ in 0..CEREMONIES_HELD + 10 {
            let (_, state) = webauthn.start_discoverable_authentication().unwrap();
            let id = ceremonies.begin(&place.rp_id, Step::SigningIn { state });
            first.get_or_insert(id);
        }
        assert_eq!(ceremonies.held().len(), CEREMONIES_HELD);
        assert!(ceremonies.finish(&first.unwrap()).is_none());
    }
}
