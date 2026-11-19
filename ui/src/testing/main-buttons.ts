/**
 * At most two main buttons on a screen (`docs/ux-rules.md`, U6; #632).
 *
 * Every screen test ends on this (`./setup.ts`) with what it drew still in
 * the document, so a screen that grows a third `main-button` fails whichever
 * test drew it, whatever that test was about. jsdom lays nothing out, but it
 * can count.
 * An open sheet (`SheetFrame`, `aria-modal`) is the screen a person is
 * looking at, so with one open its buttons are counted and the page behind
 * it is not. The tokens page's specimens stand inside `data-specimens`, and
 * are not counted.
 */
export const MOST_MAIN_BUTTONS = 2;

/** What is wrong with the main buttons on the screen drawn now, if anything. */
export function tooManyMainButtons(): Error | undefined {
	const sheets = document.querySelectorAll('[aria-modal="true"]');
	const screen = sheets.length > 0 ? sheets[sheets.length - 1] : document;
	const shown = [...screen.querySelectorAll('.main-button')].filter(
		(button) => !button.closest('[data-specimens]'),
	);
	if (shown.length > MOST_MAIN_BUTTONS) {
		const said = shown.map((button) => `"${button.textContent?.trim()}"`).join(', ');
		return new Error(
			`${shown.length} main buttons on one screen, ${said}: at most ${MOST_MAIN_BUTTONS} (docs/ux-rules.md, U6). Draw the others as second-button.`,
		);
	}
}
