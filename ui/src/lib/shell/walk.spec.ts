/**
 * Opening what an arrow led to (#199, #200).
 *
 * The one thing tested here is the thing no screen test can see. An arrow
 * pressed before the last one finished opening ends that navigation and
 * SvelteKit rejects its promise, which in a browser reached the net under the
 * app and drew the *Kamosu went wrong* card for a key held down. jsdom never
 * fires `unhandledrejection`, so the promise itself is what is asked.
 */

import { describe, expect, it, vi } from 'vitest';
import { went } from '../../testing/navigation';
import { openInPlace, toTheTopAfterAClick } from './walk';

describe('opening in place', () => {
	it('replaces the entry, and keeps the caret and the scroll', async () => {
		await openInPlace('/imports/j_1');

		expect(went).toHaveBeenLastCalledWith('/imports/j_1', {
			replace: true,
			reset: false,
		});
	});

	it('is not a Mistake when the next arrow overtakes it', async () => {
		went.mockRejectedValueOnce(new Error('navigation aborted'));

		await expect(openInPlace('/imports/j_1')).resolves.toBeUndefined();
	});
});

/**
 * A clicked row keeps the caret and goes to the top, as SvelteKit 2's
 * `data-sveltekit-keepfocus` did; SvelteKit 3 keeps the scroll with the caret
 * (#525).
 */
describe('a clicked row', () => {
	it('opens at the top of the page', () => {
		const scrollTo = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});

		toTheTopAfterAClick({ type: 'link' });

		expect(scrollTo).toHaveBeenCalledWith(0, 0);
		scrollTo.mockRestore();
	});

	it('leaves the scroll where an arrow left it', () => {
		const scrollTo = vi.spyOn(window, 'scrollTo').mockImplementation(() => {});

		toTheTopAfterAClick({ type: 'goto' });

		expect(scrollTo).not.toHaveBeenCalled();
		scrollTo.mockRestore();
	});
});
