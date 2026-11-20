/**
 * What a rating reads as (#59).
 *
 * A rating is the cook's decision about next time — again, tweak it, no —
 * rather than a score. That is why this is a lookup of three phrases and not
 * arithmetic on a number: there is nothing here to sum, and so nothing anyone
 * can average, which is ADR 0015's refusal made structural rather than left as
 * a rule somebody has to remember.
 */

import { m } from '#lib/paraglide/messages.js';
import type { GetRecipeOutput } from '#lib/api/catalogue.js';

/**
 * The three verdicts — read off the generated client rather than written out
 * again here. A fourth verdict added to the Catalogue makes `ratingLabel`'s
 * switch non-exhaustive and fails the build, which is the point: a second
 * hand-kept copy of this union would simply drift.
 */
export type Rating = GetRecipeOutput['cooked']['ratings'][number]['rating'];

/**
 * One verdict as the cooking log says it beside a cook's name (#649): whole,
 * "Would cook again", where the button the cook pressed says "Again". Beside
 * a name a single word read as a label, a rating or a button (P13).
 */
export function ratingVerdict(rating: Rating): string {
	switch (rating) {
		case 'again':
			return m.cooked_verdict_again();
		case 'tweak':
			return m.cooked_verdict_tweak();
		case 'no':
			return m.cooked_verdict_no();
	}
}

/** One verdict in the reader's language. */
export function ratingLabel(rating: Rating): string {
	switch (rating) {
		case 'again':
			return m.rating_again();
		case 'tweak':
			return m.rating_tweak();
		case 'no':
			return m.rating_no();
	}
}
