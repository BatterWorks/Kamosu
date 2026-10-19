# A passkey signs in beside the password

A Person may log in with a passkey, the WebAuthn credential their device keeps behind its face, fingerprint or screen lock, and may change their own password from Settings ([#370](https://github.com/BatterWorks/Kamosu/issues/370)). Neither takes anything away: the password, its throttle, recovery links, Invites and `kamosu recover` all work as before, and nobody is made to add a passkey or change a password.

The throttle of [ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md) stops guessing. It does nothing about a password reused on a site that leaks, or typed into a page that only looks like Kamosu. A passkey is bound to the instance's own address, so neither can steal it, and since a Session lasts until it is ended, a person meets the sign-in screen about once per device and a passkey costs them almost nothing.

## What was decided

- **Beside the password, never instead of it.** The sign-in form stays, with *Log in with a passkey* under it after an *or*. Every app surveyed does the same (`docs/research/passkeys-in-self-hosted-apps.md`).
- **No name is typed.** The device offers the passkeys it holds for this address and says whose the chosen one is. Every passkey is therefore made discoverable (`residentKey: required`) and asks for the person, not just their presence (`userVerification: required`). The prompt opens from the button, never as autofill in the name field.
- **A passkey works at the public address's host and nowhere else.** That host is the relying party. An instance with no public address, an `http://` one or an IP address offers no passkeys and says why; the WebAuthn standard refuses an IP address as a relying party, so a LAN-only instance never can. Kamosu opened at another address than its public one offers none either, and points at the public one.
- **Changing the public address strands the passkeys made at the old one, and does not delete them.** Each passkey row keeps the host it was made for. The Operator's address setting says how many work there now. After a move they stay in each person's list, marked as made for the old address, so moving back revives them and Remove clears them.
- **Adding a passkey asks for the password first.** A Session never expires, so without it whoever holds an unlocked phone could add a passkey of their own, on another device, that ending the phone's Session would not reach. A wrong password there counts against the name as at the sign-in screen.
- **Changing the password asks for the current one, under the rules of a new one.** Signing out the person's other Sessions is offered ticked; ending their Access Keys is offered unticked, since that stops every agent using one and a key never knew the password. The Session asking stays, and passkeys are left alone.
- **Agents keep parity where it is safe.** `list_passkeys`, `rename_passkey` and `remove_passkey` are ordinary Operations at both Doors. `start_adding_passkey`, `finish_adding_passkey` and `change_password` are `session_only`, as minting a Key is: an Access Key cannot turn a leak into a way in that outlives the key. Signing in with a passkey mints a Session, so it is an `/auth/` route beside `/auth/login`, outside the Catalogue.
- **The challenge is kept in memory, never in the browser.** Between a passkey exchange's start and finish, Kamosu holds what it asked for, at most 256 at once and none older than ten minutes; past the cap the oldest is dropped. A stranger may start a sign-in, so what one costs is bounded ([ADR 0032](./0032-a-stranger-may-cause-work-never-work-that-scales-with-them.md)). A restart drops them, and the browser asks again.
- **Kamosu keeps only the public half.** A passkey row holds the public key and its counter, which sign nobody in, so a passkey is not a Secret and its row is not hashed.
- **`webauthn-rs` 0.6, pinned to its prerelease.** Its stable 0.5 links OpenSSL, which the distroless image does not carry and Kamosu otherwise avoids; 0.6 does the same work on RustCrypto. Move to the stable 0.6 when it ships.

## Why

- **The password stays because it is how a person gets back in.** A lost phone takes its passkey with it. Making a passkey the only way in would make a recovery link the only way back, which hands the Operator a job every time a phone is replaced.
- **The relying party is the public address because a passkey has to name one place.** The request's own `Host` would let a passkey be made at the LAN address and fail at the public one, which is the address a phone uses away from home. One fixed place, said on the screen, is what Bitwarden, Vaultwarden and Gitea do.
- **The password before adding, because a Session never expires.** Immich and Gitea add a passkey without asking again; Bitwarden and Nextcloud ask. Kamosu's Sessions last until ended, which makes the borrowed unlocked phone a longer-lived risk here than in an app whose sessions time out.

## Considered options

- **Autofill in the name field** (conditional mediation), as authentik does. Not taken: four of the five apps surveyed ask from a button, and it is one more way for the page and the device to disagree.
- **The request's `Host` as the relying party**, as Nextcloud does. Rejected above.
- **Deleting a passkey when the address changes.** Rejected: moving back would then cost everyone their passkeys for nothing.
- **`webauthn-rs` 0.5 with OpenSSL vendored.** Rejected: a C library in an image built to carry none, for a few months until 0.6 is stable.
- **Keeping the challenge in a signed cookie.** Rejected by the library's own warning: a challenge the browser holds can be replayed.

## Consequences

- **A LAN-only instance has no passkeys.** It says so in Settings, and its sign-in screen is unchanged.
- **A passkey made on one address cannot be used on another.** After the public address moves, people log in with their password and add new passkeys.
- **Removing a passkey here leaves it on the device**, which goes on offering it and is refused. Settings says so when it asks.
- **A stranger who keeps starting passkey sign-ins can make a real one ask again.** Past 256 the oldest exchange is dropped, so someone sending starts in a loop makes a person's passkey answer "try again" while it lasts. It touches no password and no device already signed in, the same bargain as the line of password checks in ADR 0031.
- **The sign-in screen asks for its challenge before the button is pressed**, and again after each press, so the press asks the device at once: Safari opens the device's prompt only close to a tap. Adding a passkey still checks the password first, a tenth of a second, before the device is asked.
- **A browser older than Safari 18.4, Chrome 129 or Firefox 119 is offered no passkey.** Kamosu uses the browser's own JSON conversion, which those versions brought.
