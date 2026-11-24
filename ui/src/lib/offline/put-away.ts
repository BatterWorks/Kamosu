/**
 * The cards a person has put away on this device (#76). Remembered here and
 * not on the account, because what they describe — this connection, this
 * phone's Home Screen — is about the device.
 */

export type PutAway = 'insecure' | 'install';

const KEY = 'kamosu.put-away';

function read(): Partial<Record<PutAway, true>> {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Record<PutAway, true>>;
	} catch {
		return {};
	}
}

export function wasPutAway(card: PutAway): boolean {
	return read()[card] === true;
}

export function putAway(card: PutAway): void {
	try {
		localStorage.setItem(KEY, JSON.stringify({ ...read(), [card]: true }));
	} catch {
		// Storage refused: the card is put away for this visit only.
	}
}

/**
 * Left by the language page when Kamosu's own words change, which reloads the
 * page (#653, the audit's G9). The load that follows carries the line saying
 * what changed, and no card arrives on top of it: `Notices.svelte` reads this
 * once, takes it away, and offers the install card from the next page on.
 */
export const WORDS_CHANGED = 'kamosu.words-changed';

/** Whether this load follows a change of Kamosu's words, asked once and forgotten. */
export function justChangedWords(): boolean {
	try {
		const left = sessionStorage.getItem(WORDS_CHANGED) !== null;
		sessionStorage.removeItem(WORDS_CHANGED);
		return left;
	} catch {
		return false;
	}
}
