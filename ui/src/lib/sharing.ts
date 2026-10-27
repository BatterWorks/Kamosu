/**
 * Sharing a recipe in from a phone (#452, ADR 0055): what the Settings page
 * that sets a phone up needs to know, written once.
 *
 * An iPhone shares through an Apple Shortcut, which the maintainer builds on
 * an iPhone from `docs/sharing-from-an-iphone.md` and publishes as an iCloud
 * link. An Android phone shares into Kamosu itself, once it is installed.
 */

/**
 * The published Shortcut's iCloud link, or nothing until it is published.
 * A release that changes the Shortcut changes this link, and the version
 * `add_this` holds (`SHORTCUT_VERSION` in `src/core/sharing.rs`).
 */
export const SHORTCUT_URL: string =
	'https://www.icloud.com/shortcuts/193a2777774d45bcb45944faec9f59d7';

/**
 * The one code the Shortcut asks for when it is installed: the format's
 * name, `kamosu1`, the address this phone reaches Kamosu at, and the Add-only
 * key, one space between each. The Shortcut splits it at the spaces, so no
 * part may hold one. The format is written down beside the Shortcut's steps,
 * in `docs/sharing-from-an-iphone.md`, and a new one would be `kamosu2`.
 */
export function setupCode(address: string, key: string): string {
	return `kamosu1 ${address.replace(/\/+$/, '')} ${key}`;
}

/** Which kind of phone this is, as far as sharing goes. */
export type Phone = 'apple' | 'android' | 'other';

export function whichPhone(userAgent: string, apple: boolean): Phone {
	if (apple) return 'apple';
	return /Android/i.test(userAgent) ? 'android' : 'other';
}

/** What the key a phone's Shortcut carries is named, in the key list. */
export function phoneName(userAgent: string): string {
	if (/iPad/.test(userAgent)) return 'iPad';
	return 'iPhone';
}
