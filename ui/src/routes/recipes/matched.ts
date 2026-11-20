/**
 * How a result says what matched (#62, ADR 0027).
 *
 * A result that cannot explain itself is noise: search *chocolate*, get
 * *Chilli con carne*, and without the 50 g of dark chocolate on the card the
 * search looks broken. The Operation answers where it matched and the line
 * itself; this turns the where into the short label set above the quote.
 *
 * Every result says why, a title match included (#650): when only some did,
 * the ones that said nothing read as matching for no reason (the UX audit's
 * R6). A title match says so in a label alone, with no quote: the title is
 * already on the tile's band, and quoting it back would say it twice.
 *
 * A **meaning** match is labelled as one, and only as one. Its quoted line does
 * not contain the words that were typed — that is the whole point of it — so
 * without saying *close in meaning* the card reads as a search gone wrong. That
 * one label says everything the where-label would have said and the thing the
 * where-label could not, which is why it replaces it rather than joining it.
 */

import { m } from '#lib/paraglide/messages.js';
import { languageName } from '#lib/language.js';
import type { SearchRecipesOutput } from '#lib/api/catalogue.js';

export type Entry = SearchRecipesOutput['recipes'][number];
export type Matched = NonNullable<Entry['matched']>;

/** The label above a quoted line, or above nothing for a title, or null where nothing matched. */
export function matchedLabel(matched: Matched | null): string | null {
	if (!matched) return null;
	// A meaning match always quotes a real line of the recipe — never its
	// title, which the tile's band already carries — so there is no case here
	// where the quote would repeat itself.
	if (matched.by === 'meaning') return m.recipes_matched_meaning();
	switch (matched.where) {
		case 'title':
			return m.recipes_matched_title();
		case 'tag':
			return m.recipes_matched_tag();
		case 'ingredient':
			return m.recipes_matched_ingredient();
		case 'step':
			// The Operation counts a Step the way the recipe page does, Sections
			// taking no number, so this only has to print it. A Step that
			// somehow arrived without one is still a real match: it keeps the
			// quote and loses only the number.
			return matched.step_number === null
				? m.recipes_matched_step_unnumbered()
				: m.recipes_matched_step({ number: matched.step_number });
		case 'section':
			return m.recipes_matched_section();
		case 'note':
			return m.recipes_matched_note();
		case 'attempt':
			return m.recipes_matched_attempt();
	}
}

/**
 * The mark on a recipe being read in a Language its reader did not ask for,
 * written out in full — "In French". The card shows only the Language's code,
 * which is all that fits; this is what it means, for anyone listening to the
 * screen rather than looking at it.
 *
 * Null when there is nothing to mark — a preference never hides a recipe from
 * its owner, and the card says which Language it fell back to (ADR 0006).
 *
 * Takes the two fields it reads rather than a whole shelf `Entry`, so the
 * Related Recipes strip marks a fallback in the same words as the shelf does
 * (#105). A second copy of this mapping is how "In French" becomes "In fr" on
 * one screen and nobody notices — which is why the mapping itself moved to
 * `#lib/language.js` when the recipe screen needed it too (#106), and why this
 * reads it rather than keeping a copy.
 */
export function fallbackLanguage(entry: {
	language: string;
	language_fallback: boolean;
}): string | null {
	if (!entry.language_fallback) return null;
	return m.recipes_language_fallback({ language: languageName(entry.language) });
}

/** A piece of a quoted line, set bold where it is a word that was typed. */
export interface Part {
	text: string;
	typed: boolean;
}

/**
 * A quoted line cut where it holds a word that was searched for, so the card
 * can set those words bold and a reader sees why it matched (#650). Only a
 * word match is marked: a meaning match's line does not hold the words typed,
 * which is its point. Words of one letter are not marked, since they would
 * light up half the line. Case is ignored, as the search ignores it.
 */
export function parts(line: string, query: string | null, by: Matched['by']): Part[] {
	const words =
		by === 'words' && query
			? query
					.toLocaleLowerCase()
					.split(/\s+/)
					.filter((word) => word.length > 1)
			: [];
	if (words.length === 0) return [{ text: line, typed: false }];
	const lower = line.toLocaleLowerCase();
	// Lower-casing can change a string's length (İ, ẞ); marking then falls back to none.
	if (lower.length !== line.length) return [{ text: line, typed: false }];
	const out: Part[] = [];
	let at = 0;
	while (at < line.length) {
		let found = -1;
		let word = '';
		for (const each of words) {
			const index = lower.indexOf(each, at);
			if (
				index !== -1 &&
				(found === -1 || index < found || (index === found && each.length > word.length))
			) {
				found = index;
				word = each;
			}
		}
		if (found === -1) {
			out.push({ text: line.slice(at), typed: false });
			break;
		}
		if (found > at) out.push({ text: line.slice(at, found), typed: false });
		out.push({ text: line.slice(found, found + word.length), typed: true });
		at = found + word.length;
	}
	return out;
}
