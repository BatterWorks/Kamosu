/**
 * A word too long for its line breaks at the edge of its box (#385).
 *
 * A web address has no space in it, and a browser breaks a line only at a
 * space or a hyphen unless the stylesheet lets it break inside a word. Before
 * #385 nothing did, so an address in a recipe pushed the phone's page
 * sideways. The page is drawn here as the tests draw it and then measured in
 * Chrome at the phone's size (`src/testing/chrome.ts`).
 */

import type { Browser } from 'playwright';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { standIn } from '#lib/api/stand-in.js';
import RecipeTestHarness from './RecipeTestHarness.svelte';
import { MY_KITCHENS, recipeAnswer, threadBranch } from '../../../testing/recipes';
import { CHROME_HOOK_MS, drawn, startChrome } from '../../../testing/chrome';
import { SIZES } from '../../../../tools/sizes.mjs';

/**
 * An address as a recipe quotes one, with a stretch between its breaks longer
 * than a phone's line. The browser breaks one at a hyphen or after a `?`, and
 * neither comes soon enough.
 */
const ADDRESS =
	'https://example.com/blogs/news/vanilla-cheesecake-brownie-recipe?utm_source=newsletter&utm_medium=email&utm_campaign=autumn_bakes';

const BOXES = {
	'Ingredient Line': `250 g cream cheese, as ${ADDRESS} says`,
	Step: `Bake for 25 minutes, as ${ADDRESS} says.`,
	note: `Quantities from ${ADDRESS}`,
};

function recipeWithTheAddress() {
	const answer = recipeAnswer({
		note: BOXES.note,
		ingredients: [{ kind: 'ingredient', text: BOXES['Ingredient Line'] }],
		steps: [{ kind: 'step', text: BOXES.Step, photo: null }],
	});
	const [version] = answer.versions;
	version.readings = [null];
	version.measured = { ingredients: [null], steps: [null] };
	return answer;
}

/** The recipe page as the tests draw it, the markup a browser would hold. */
async function drawnPage(): Promise<string> {
	const kamosu = standIn({
		get_recipe: recipeWithTheAddress(),
		get_thread: {
			lineage_id: 'l_1',
			branches: [threadBranch('b_1', { hand_id: 'h_1', head_version_id: 'v_1' })],
			versions: [],
			attempts: [],
		},
		list_attempts: { attempts: [] },
		get_recipe_job: { job: null },
		can_read_recipe_again: {
			can_read_again: false,
			from: null,
			link: null,
			original: null,
			reason: 'no_runner',
		},
		list_kitchens: MY_KITCHENS,
		note_recipe_opened: { lineage_id: 'l_1', opened_at: '2026-09-22T00:00:00Z' },
		shopping_basis: { branch_id: 'b_1', title: 'Miso Soup', written_yield: null, lines: [] },
		get_shopping_list: { chosen: [], rows: [] },
	});
	render(RecipeTestHarness, { props: { client: kamosu.client, branchId: 'b_1' } });
	await screen.findByText(BOXES.note);
	return document.body.innerHTML;
}

let browser: Browser;
beforeAll(async () => {
	browser = await startChrome();
}, CHROME_HOOK_MS);
afterAll(async () => {
	await browser?.close();
}, CHROME_HOOK_MS);

/**
 * How far each box's text runs past the right edge of its box, or of the
 * screen, in CSS pixels. The box is the nearest element around the text that
 * is not itself a run of text.
 */
async function overhangs(markup: string): Promise<Record<string, number>> {
	const page = await drawn(browser, markup, { size: SIZES.phone });
	try {
		return await page.evaluate((boxes) => {
			const found: Record<string, number> = {};
			for (const [name, text] of Object.entries(boxes)) {
				const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
				let node: Node | null;
				while ((node = walker.nextNode()) && !node.textContent?.includes(text));
				if (!node?.parentElement) continue;
				const range = document.createRange();
				range.selectNodeContents(node);
				let box: Element = node.parentElement;
				while (getComputedStyle(box).display === 'inline' && box.parentElement) {
					box = box.parentElement;
				}
				const edge = Math.min(box.getBoundingClientRect().right, window.innerWidth);
				found[name] = Math.max(0, range.getBoundingClientRect().right - edge);
			}
			return found;
		}, BOXES);
	} finally {
		await page.close();
	}
}

describe('a word too long for its line', () => {
	it('breaks at the edge of its box on the phone, in an Ingredient Line, a Step and the note', async () => {
		const measured = await overhangs(await drawnPage());
		expect(measured).toEqual({ 'Ingredient Line': 0, Step: 0, note: 0 });
	});
});
