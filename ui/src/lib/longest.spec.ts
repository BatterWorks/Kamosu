import { describe, expect, it } from 'vitest';
import { longest } from './longest.js';

describe('longest', () => {
	it('reads the limit each text box is held to from the Catalogue', () => {
		expect(longest('add_loose_item', 'text')).toBe(2000);
		expect(longest('plan_words', 'words')).toBe(2000);
		expect(longest('set_planned_words', 'words')).toBe(2000);
		expect(longest('edit_attempt', 'note')).toBe(10000);
	});

	it('says nothing of a field with no limit', () => {
		expect(longest('add_loose_item', 'item_id')).toBeUndefined();
	});
});
