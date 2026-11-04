/**
 * Walking a list with the arrow keys, where the list stays beside what is
 * open (ADR 0044, #199). Foods is the first such list and Brought in the
 * second (#200).
 */

import { afterNavigate, goto, type AfterNavigate } from '$app/navigation';

/**
 * Whether a key was pressed with the caret somewhere that uses it: a field,
 * where the arrows move the caret, or a control that takes them itself.
 */
export function typingInAField(event: KeyboardEvent): boolean {
	const target = event.target;
	if (!(target instanceof HTMLElement)) return false;
	return target.isContentEditable || target.closest('input, textarea, select') !== null;
}

/**
 * Where an arrow leads in a list: the item above or below the open one, the
 * first where none of the list's is open, and the open one again at either
 * end. No answer for any other key, or an arrow held with another key, which
 * is the browser's.
 */
export function stepInList(
	event: KeyboardEvent,
	items: string[],
	open: string | undefined,
): string | undefined {
	const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
	if (step === 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
	if (items.length === 0) return;
	const at = open === undefined ? -1 : items.indexOf(open);
	if (at < 0) return items[0];
	return items[Math.min(items.length - 1, Math.max(0, at + step))];
}

/**
 * Open what an arrow led to, in place of what was open: the history entry is
 * replaced, so going back leaves the list in one step, and the caret and the
 * scroll stay where they are.
 *
 * An arrow pressed again before the last one has finished opening ends that
 * navigation, and SvelteKit rejects its promise. That is a key held down and
 * no Mistake, so it is caught here and never reaches the net under the app
 * (#98), which showed the *Kamosu went wrong* card for it.
 */
export async function openInPlace(address: string): Promise<void> {
	try {
		await goto(address, { replace: true, reset: false });
	} catch {
		// Overtaken by the next arrow.
	}
}

/**
 * A row of such a list, clicked, keeps the caret on itself so the arrows walk
 * on from it, and opens what it leads to at the top of the page. A row's link
 * says `data-sveltekit-reset="false"` for the first half, which in SvelteKit 3
 * also keeps the scroll, so this puts the top back after a click, as SvelteKit
 * 2's `data-sveltekit-keepfocus` did (#525). An arrow, which keeps the scroll
 * on purpose, is a `goto` and is left alone. Called once by each such list
 * while it is drawn.
 */
export function openClickedRowsAtTheTop(): void {
	afterNavigate(toTheTopAfterAClick);
}

/** What `openClickedRowsAtTheTop` does after each navigation. */
export function toTheTopAfterAClick({ type }: Pick<AfterNavigate, 'type'>): void {
	if (type === 'link') window.scrollTo(0, 0);
}
