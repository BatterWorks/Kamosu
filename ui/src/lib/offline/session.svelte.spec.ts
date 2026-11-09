/**
 * What a sign-in does to what this device holds (#77, #537): another
 * Person's held changes go, the same Person's stay to be sent, and a recipe
 * the server says is gone stops being counted as on the phone.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { KamosuClient } from '#lib/api/catalogue.js';
import { Library, sessionBegan, signedOutHere } from './library.svelte';
import { recipeGone } from './device.svelte';

const HELD = JSON.stringify({ queue: [{ id: 'w_1', operation: 'add_loose_item' }] });

describe('a sign-in', () => {
	beforeEach(() => localStorage.clear());

	it('keeps what the same Person held while their Session was ended elsewhere', () => {
		sessionBegan('p_carl');
		localStorage.setItem('kamosu.outbox', HELD);
		sessionBegan('p_carl');
		expect(localStorage.getItem('kamosu.outbox')).toBe(HELD);
	});

	it('never sends what another Person held as this one', () => {
		sessionBegan('p_carl');
		localStorage.setItem('kamosu.outbox', HELD);
		sessionBegan('p_olga');
		expect(localStorage.getItem('kamosu.outbox')).toBeNull();
	});

	it('never sends what was given up by signing out, the same Person or not', () => {
		sessionBegan('p_carl');
		localStorage.setItem('kamosu.outbox', HELD);
		signedOutHere();
		expect(localStorage.getItem('kamosu.outbox')).toBeNull();
		localStorage.setItem('kamosu.outbox', HELD);
		sessionBegan('p_carl');
		expect(localStorage.getItem('kamosu.outbox')).toBeNull();
	});

	it('lets go of what nobody can say whose it is', () => {
		localStorage.setItem('kamosu.outbox', HELD);
		sessionBegan('p_carl');
		expect(localStorage.getItem('kamosu.outbox')).toBeNull();
	});
});

describe('a device signed in before it kept who', () => {
	beforeEach(() => {
		localStorage.clear();
		// A page a service worker controls, as on a phone.
		vi.stubGlobal('isSecureContext', true);
		Object.defineProperty(navigator, 'serviceWorker', {
			value: { controller: {}, addEventListener() {}, removeEventListener() {} },
			configurable: true,
		});
	});
	afterEach(() => {
		vi.unstubAllGlobals();
		Reflect.deleteProperty(navigator, 'serviceWorker');
	});

	it('learns it from the server when the library starts, and keeps their held changes', async () => {
		const library = new Library({
			searchRecipes: async () => ({ recipes: [] }),
			getPerson: async () => ({ person_id: 'p_carl', name: 'Carl', is_operator: false }),
		} as unknown as KamosuClient);
		await library.start(false);
		await new Promise((resolve) => setTimeout(resolve, 0));
		localStorage.setItem('kamosu.outbox', HELD);
		sessionBegan('p_carl');
		expect(localStorage.getItem('kamosu.outbox')).toBe(HELD);
	});

	it('never notes the Person an answer was asked for once another has signed in', async () => {
		let answer: (value: unknown) => void = () => {};
		const library = new Library({
			searchRecipes: async () => ({ recipes: [] }),
			getPerson: () => new Promise((resolve) => (answer = resolve)),
		} as unknown as KamosuClient);
		await library.start(false);
		sessionBegan('p_olga');
		answer({ person_id: 'p_carl', name: 'Carl', is_operator: false });
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(localStorage.getItem('kamosu.signed-in-as')).toBe('p_olga');
	});
});

describe('a recipe the server says is gone', () => {
	beforeEach(() => localStorage.clear());

	it('is no longer counted as on the phone', () => {
		localStorage.setItem(
			'kamosu.library',
			JSON.stringify({ filledAt: 1, held: ['b_stew', 'b_tart'] }),
		);
		const library = new Library({} as KamosuClient);
		expect(library.held.size).toBe(2);
		for (const told of recipeGone) told('b_stew');
		expect([...library.held]).toEqual(['b_tart']);
		expect(JSON.parse(localStorage.getItem('kamosu.library') ?? '{}').held).toEqual(['b_tart']);
	});
});
