/**
 * A paste held on its way to the writing screen of the recipe it made (#175).
 */

import { describe, expect, it } from 'vitest';
import { forgetPaste, holdPaste, takePaste, type PastedDraft } from './pasted.svelte';

const draft: PastedDraft = {
	title: 'Lemon loaf',
	note: null,
	source: null,
	ingredients: [{ kind: 'ingredient', text: '1 lemon' }],
	steps: [{ kind: 'step', text: 'Bake it.' }],
};

describe('a held paste', () => {
	it('is taken once, by the recipe it was held for', () => {
		holdPaste('b_new', draft);
		expect(takePaste('b_other')).toBeUndefined();
		expect(takePaste('b_new')).toEqual(draft);
		expect(takePaste('b_new')).toBeUndefined();
	});

	it('is not taken once forgotten (#509)', () => {
		holdPaste('b_new', draft);
		forgetPaste();
		expect(takePaste('b_new')).toBeUndefined();
	});
});
