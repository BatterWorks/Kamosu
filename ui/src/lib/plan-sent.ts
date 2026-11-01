/**
 * What a Planned Meal sent to the Shopping List, in words (#488).
 *
 * The server keeps how much of the recipe as written each meal last sent, as
 * a factor (`sent_factor`: 2 is twice it over). What it is planned for now is
 * a factor too, worked out as the Shopping List works out a Yield
 * (`yieldScale`). The mark, the meal's sheet and the answer to a send are
 * all said from those two numbers, so they never disagree.
 */
import { plainNumber, yieldScale } from '$lib/offline/shopping';
import { writtenCount, type Wanted, type Written } from '$lib/how-much';

/** The tick a sent meal is marked with, as an SVG path. */
export const TICK = 'M5 12.5l4.2 4.2L19 7';

/** The two factors are the same below this: one is read back from a REAL column. */
const SAME = 1e-9;

interface Planned {
	planned_yield: Wanted;
	written_yield: Written;
	sent_factor: number | null;
	words: string | null;
	gone: boolean;
}

/** How much of the recipe as written a meal is planned for now. */
export const factorOf = (meal: Planned): number =>
	yieldScale(meal.planned_yield, meal.written_yield);

/**
 * A factor of a recipe in its own words: `6 servings` where its Yield counts,
 * and `×2` where it does not or where it states none.
 */
export function amountOf(written: Written, factor: number): string {
	const count = writtenCount(written);
	return written && count !== null
		? `${plainNumber(factor * count)} ${written.noun}`
		: `×${plainNumber(factor)}`;
}

/** A factor as the mark counts it, with no noun: `4`, or `×2`. */
export function countOf(written: Written, factor: number): string {
	const count = writtenCount(written);
	return count !== null ? plainNumber(factor * count) : `×${plainNumber(factor)}`;
}

/**
 * What a meal's mark says: nothing for one never sent (or written words, or
 * one that can no longer be read), `sent` for one sent as it is planned now,
 * `more` for one planned for more since, and `fewer` for one planned for less.
 */
export type Sent =
	| { kind: 'none' }
	| { kind: 'sent' }
	| { kind: 'more'; sent: number; more: number }
	| { kind: 'fewer'; sent: number };

export function sentOf(meal: Planned): Sent {
	if (meal.words !== null || meal.gone || meal.sent_factor === null) return { kind: 'none' };
	const now = factorOf(meal);
	const sent = meal.sent_factor;
	if (now > sent + SAME) return { kind: 'more', sent, more: now - sent };
	if (now < sent - SAME) return { kind: 'fewer', sent };
	return { kind: 'sent' };
}
