/**
 * The browser's half of a passkey (#370): handing the device what Kamosu
 * asked for, and handing Kamosu back what the device made or presented.
 *
 * Kamosu sends the options as JSON and takes the credential back as the JSON
 * text the browser writes of it. The browser converts both ways itself
 * (`parseCreationOptionsFromJSON`, `toJSON`), which every engine has had since
 * Safari 18.4 (MDN: Baseline 2025). A browser without them is offered no
 * passkey, as one at the wrong address is.
 */

/** What the device did when asked. */
export type DeviceAnswer =
	/** It made or presented a passkey: the browser's JSON of it, as text. */
	| { credential: string }
	/** The person closed the device's own prompt, or it timed out. */
	| { declined: true }
	/** Adding: this device already holds one of this person's passkeys here. */
	| { already: true };

/**
 * Whether a passkey can be used on this page: the instance offers passkeys,
 * at this very address, and the browser can ask its device for one. A
 * passkey proves the address it was made at and nothing else, so Kamosu
 * opened at its LAN address offers none, though its public address does.
 */
export function passkeysWorkHere(origin: string | null | undefined): boolean {
	return (
		!!origin &&
		typeof window !== 'undefined' &&
		window.location.origin === origin &&
		browserHasPasskeys()
	);
}

/** A browser that can ask its device at all, whatever the address. */
export function browserHasPasskeys(): boolean {
	return (
		typeof PublicKeyCredential === 'function' &&
		typeof PublicKeyCredential.parseCreationOptionsFromJSON === 'function' &&
		typeof PublicKeyCredential.parseRequestOptionsFromJSON === 'function'
	);
}

/** Ask the device to make a passkey from Kamosu's options. */
export async function makePasskey(options: object): Promise<DeviceAnswer> {
	try {
		const made = await navigator.credentials.create({
			publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(
				options as PublicKeyCredentialCreationOptionsJSON,
			),
		});
		return answered(made);
	} catch (error) {
		// The device found one of this person's passkeys already on it, which
		// Kamosu listed so as not to make a second.
		if (error instanceof DOMException && error.name === 'InvalidStateError') {
			return { already: true };
		}
		return declinedOr(error);
	}
}

/** Ask the device for one of the passkeys it holds for this address. */
export async function presentPasskey(options: object): Promise<DeviceAnswer> {
	try {
		const presented = await navigator.credentials.get({
			publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(
				options as PublicKeyCredentialRequestOptionsJSON,
			),
		});
		return answered(presented);
	} catch (error) {
		return declinedOr(error);
	}
}

function answered(credential: Credential | null): DeviceAnswer {
	if (!(credential instanceof PublicKeyCredential)) return { declined: true };
	return { credential: JSON.stringify(credential.toJSON()) };
}

/**
 * A prompt the person closed, or let time out, is their answer and nothing
 * went wrong; the browser says both as `NotAllowedError`. Anything else is
 * a fault, and goes where faults go.
 */
function declinedOr(error: unknown): DeviceAnswer {
	if (
		error instanceof DOMException &&
		(error.name === 'NotAllowedError' || error.name === 'AbortError')
	) {
		return { declined: true };
	}
	throw error;
}
