/**
 * Whether a page draws its own back arrow on its photograph (#649).
 *
 * On the phone the shell draws the back arrow in the page's flow, sticking to
 * the top of the window (#219). Over a recipe that arrow covered a Step's
 * first words as the page scrolled under it (P12 of the UX audit of 19
 * November 2026). A recipe and an older version of one draw the arrow on
 * their photograph instead, where it scrolls away with it, and say so here,
 * so the shell leaves its own out. Counted rather than a flag, so a page
 * leaving as the next arrives never takes the next one's arrow away.
 */
import { untrack } from 'svelte';

export const arrowOnPhoto = $state({ held: 0 });

/**
 * Say the page draws its own arrow, for as long as the returned call is not
 * made. Called from an effect, so the count is read untracked: an effect that
 * read what it then wrote would run again for its own write, without end.
 */
export function holdArrowOnPhoto(): () => void {
	untrack(() => (arrowOnPhoto.held += 1));
	return () => {
		untrack(() => (arrowOnPhoto.held -= 1));
	};
}
