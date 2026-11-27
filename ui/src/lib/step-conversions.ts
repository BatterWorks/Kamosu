/**
 * A Step's text, cut where each conversion goes (#150).
 *
 * The Core answers each conversion a Step offers as `{ written, measured }`:
 * the oven temperature, or the amount and Unit, exactly as the text has them,
 * and what they come to for this reader. Each addition goes straight after
 * what it converts, "1 lb. (about 455 g) ground chicken" and "preheat to 425°
 * (about 220 °C)": option A, chosen on 25 September 2026 over one
 * line of figures beneath the Step.
 *
 * Found by searching forward from the last one, so two `2 Tbsp.` in one Step
 * land on the first and the second in turn. One not found (the cook rewrote
 * the words) is left out rather than put somewhere it does not belong.
 * The text itself is never changed: joining each piece's `text` and `written`
 * gives it back.
 */
import type { GetRecipeOutput } from '#lib/api/catalogue.js';

/**
 * A Step's conversions as the Catalogue declares them — its oven and its
 * amounts, in the order written — so a change there breaks the build here.
 */
export type StepConversions = NonNullable<
	GetRecipeOutput['versions'][number]['measured']['steps'][number]
>;

export type StepPiece = { text: string; written: string | null; measured: string | null };

export function stepPieces(text: string, conversions: StepConversions): StepPiece[] {
	const pieces: StepPiece[] = [];
	let from = 0;
	// A phone keeps its answers across an update of Kamosu (#76), so the first
	// read after this one shipped can be an older answer, where a Step's slot
	// was a string. It draws nothing until the refresh behind it arrives.
	for (const { written, measured } of Array.isArray(conversions) ? conversions : []) {
		const at = text.indexOf(written, from);
		if (at < 0) continue;
		pieces.push({ text: text.slice(from, at), written, measured });
		from = at + written.length;
	}
	pieces.push({ text: text.slice(from), written: null, measured: null });
	return pieces;
}

/**
 * Tonight's figure for a scaled Step, to stand in the written one's place
 * (#681), without saying its first word twice: "each weighing about 280g"
 * at twice the recipe is "each weighing about 10 oz", where the figure the
 * Core wrote, "about 10 oz", came after the Step's own "about" (round 3 of
 * #681's climb). Whatever the Language: the word is the one the figure
 * starts with, found at the end of the words before it.
 */
export function inPlace(before: string, measured: string): string {
	const first = measured.split(/\s+/, 1)[0] ?? '';
	const words = before.trimEnd().split(/\s+/);
	const last = words[words.length - 1] ?? '';
	if (first === '' || first === measured) return measured;
	return last.toLocaleLowerCase() === first.toLocaleLowerCase()
		? measured.slice(first.length).trimStart()
		: measured;
}

/**
 * A Step's words with its conversions written in, as one line of text, for
 * where a Step is read small and in one colour: the next Step's preview on a
 * tablet (round 5 of #681's climb). Drawn as `StepWords` draws them, the
 * written figure with tonight's or the cook's after it, or tonight's in its
 * place when `lead`.
 */
export function stepText(text: string, conversions: StepConversions, lead = false): string {
	return stepPieces(text, conversions)
		.map(({ text: before, written, measured }) => {
			if (written === null) return before;
			if (lead) return before + inPlace(before, measured ?? '');
			return `${before}${written} (${measured})`;
		})
		.join('');
}
