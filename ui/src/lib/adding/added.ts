/**
 * A recipe just added from a web page, for the page it opens on to say so
 * (#650): the UX audit found an imported recipe opened with no word that it
 * was saved, or where, so a cook might import it twice (L8). And a recipe
 * that *Undo* took back out, for Recipes to say so where the cook lands.
 *
 * Each is a *just happened* fact, held here for one navigation and taken by
 * the screen that shows it, so a reload an hour later greets nobody. The
 * fact a Bundle file brought in is `#lib/arrival.svelte.js`'s, with words of
 * its own about what the file carried.
 */

let added: string | null = null;
let takenOut = false;

/** Said by the act that added it, just before it opens the recipe. */
export function noteAdded(branchId: string): void {
	added = branchId;
}

/** Whether this recipe was just added, asked once by its page. */
export function takeAdded(branchId: string): boolean {
	const was = added === branchId;
	if (was) added = null;
	return was;
}

/** Said by *Undo*, just before it opens Recipes. */
export function noteTakenOut(): void {
	takenOut = true;
}

/** Whether a recipe was just taken back out, asked once by Recipes. */
export function takeTakenOut(): boolean {
	const was = takenOut;
	takenOut = false;
	return was;
}
