/**
 * The letter a recipe is filed under on Recipes, for the letters down the
 * screen's right edge (#650, option A): a cook slides a finger along them to
 * reach the part of the A to Z they want, as a phone's contacts list does.
 *
 * A title's first letter with its accents folded away, so *Éclair* is under E
 * beside *Eggs*, as the shelf's own order puts it (the server sorts on the
 * title folded the same way, `folded_for_search`). A title starting with
 * anything else, a figure, a ligature such as Œ, or a letter from outside the
 * Latin alphabet, is under `#`: the server's fold leaves those apart from A
 * to Z, so filing *Œufs* under O would send the cook where it is not.
 */

/** Every letter the edge offers, in its order. */
export const LETTERS = ['#', ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'] as const;

export type Letter = (typeof LETTERS)[number];

export function letterOf(title: string): Letter {
	const first = title.trim().normalize('NFD').replace(/\p{M}/gu, '').charAt(0).toUpperCase();
	return (LETTERS as readonly string[]).includes(first) && first !== '#' ? (first as Letter) : '#';
}

/**
 * Where each letter starts, as the index of its first title in a list
 * already in the shelf's order. A letter no title starts with is absent.
 */
export function firstOfEach(titles: readonly string[]): Map<Letter, number> {
	const starts = new Map<Letter, number>();
	titles.forEach((title, index) => {
		const letter = letterOf(title);
		if (!starts.has(letter)) starts.set(letter, index);
	});
	return starts;
}

/**
 * The letter to go to when one is touched: that one, or the next the list
 * holds, or the last it holds when nothing comes after. A finger sliding over
 * an empty letter still moves the list, as a contacts list does.
 */
export function nearest(touched: Letter, held: ReadonlySet<Letter>): Letter | null {
	const from = LETTERS.indexOf(touched);
	const after = LETTERS.slice(from).find((letter) => held.has(letter));
	if (after) return after;
	return LETTERS.toReversed().find((letter) => held.has(letter)) ?? null;
}
