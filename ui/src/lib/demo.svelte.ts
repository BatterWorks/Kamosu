/**
 * A Demo saying it is one (ADR 0052, #407): the welcome card set into the top
 * of Home, the strip at the foot of every screen, and the way to installing
 * both lead to.
 *
 * Whether this instance is a Demo is the Core's answer to `instance_status`,
 * heard by whichever of the shell or Home asks first. It is kept on the
 * device as well, as a hint for drawing only: a phone that cannot reach the
 * server just now still draws the strip it drew a minute ago.
 */

import type { KamosuClient } from './api/catalogue';
import { OperationError } from './api/refusal';

/** Where "Install your own" leads: the README's Quick start, on GitHub. */
export const INSTALL_KAMOSU = 'https://github.com/BatterWorks/Kamosu#quick-start';

/** The key a Demo publishes, kept while this instance is one. */
const DEMO_KEY = 'kamosu.demo-key';
/** Where the welcome card stands on this device: put away, or brought back. */
const WELCOME = 'kamosu.demo-welcome';

function remembered(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		// No storage, or none while the app is prerendered: nothing is known.
		return null;
	}
}

function remember(key: string, value: string | undefined): void {
	try {
		if (value === undefined) localStorage.removeItem(key);
		else localStorage.setItem(key, value);
	} catch {
		// Private browsing, or storage refused: known for this visit only.
	}
}

/**
 * The welcome card on this device: `first`, on the first arrival, before
 * anything else is drawn; `closed`, put away with Got it; `again`, brought
 * back by the strip, which then stays drawn beneath it as the prototype chosen
 * on 23 October 2026 showed.
 */
export type Welcome = 'first' | 'closed' | 'again';

function rememberedWelcome(): Welcome {
	const kept = remembered(WELCOME);
	return kept === 'closed' || kept === 'again' ? kept : 'first';
}

/**
 * What this device knows of the Demo: whether this instance is one, and the
 * read-only Access Key it publishes for a visitor's own agent.
 */
export const demo = $state<{ is: boolean; key: string | undefined; welcome: Welcome }>({
	is: remembered(DEMO_KEY) !== null,
	key: remembered(DEMO_KEY) ?? undefined,
	welcome: rememberedWelcome(),
});

/**
 * Ask the Core whether this is a Demo. An instance that cannot be asked just
 * now leaves what the device last heard standing.
 */
export async function learnWhetherADemo(kamosu: KamosuClient): Promise<void> {
	try {
		const status = await kamosu.instanceStatus();
		demo.is = status.demo;
		demo.key = status.demo_access_key;
		remember(DEMO_KEY, status.demo ? (status.demo_access_key ?? '') : undefined);
	} catch (error) {
		if (!(error instanceof OperationError)) throw error;
	}
}

/**
 * What a screen says of a refusal it otherwise words itself: a Demo's own
 * sentence where a Demo refused, and the screen's words for anything else.
 */
export function demoRefusalOr(error: OperationError, own: string): string {
	return error.reason === 'demo' ? error.message : own;
}

/** Got it: the card is put away on this device. */
export function closeWelcome(): void {
	demo.welcome = 'closed';
	remember(WELCOME, 'closed');
}

/** The strip's "A Kamosu demo": the card comes back at the top of Home. */
export function openWelcomeAgain(): void {
	demo.welcome = 'again';
	remember(WELCOME, 'again');
}

/**
 * The cleaning's hour, 04:00 UTC (`CLEANING_HOUR` in `src/core/demo.rs`), as
 * the reader's own clock reads it today, summer time and all.
 */
export function cleaningInLocalTime(locale: string, timeZone?: string, now = new Date()): string {
	const cleaning = new Date(
		Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate(), 4, 0),
	);
	return new Intl.DateTimeFormat(locale, {
		hour: '2-digit',
		minute: '2-digit',
		hourCycle: 'h23',
		timeZone,
	}).format(cleaning);
}
