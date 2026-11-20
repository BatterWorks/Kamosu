import { describe, expect, it } from 'vitest';
import { firstOfEach, letterOf, nearest } from './letters';

describe('the letter a recipe is filed under (#650)', () => {
	it('is the first letter, in capitals', () => {
		expect(letterOf('shakshuka')).toBe('S');
		expect(letterOf('  Miso soup')).toBe('M');
	});

	it('folds accents away, so Éclair is beside Eggs', () => {
		expect(letterOf('Éclair au café')).toBe('E');
		expect(letterOf('Île flottante')).toBe('I');
	});

	it('files a ligature under #, as the shelf sorts it apart from A to Z', () => {
		expect(letterOf('Œufs à la neige')).toBe('#');
		expect(letterOf('Æbleskiver')).toBe('#');
	});

	it('files a figure or another alphabet under #', () => {
		expect(letterOf('10 minute tofu')).toBe('#');
		expect(letterOf('親子丼')).toBe('#');
		expect(letterOf('')).toBe('#');
	});
});

describe('where each letter starts', () => {
	it('is the first title under it, in the order given', () => {
		const starts = firstOfEach(['10 minute tofu', 'Apple pie', 'Avocado rice', 'Éclair', 'Eggs']);
		expect([...starts]).toEqual([
			['#', 0],
			['A', 1],
			['E', 3],
		]);
	});
});

describe('the letter a touch goes to', () => {
	const held = new Set(['#', 'A', 'E', 'S'] as const);

	it('is the letter touched where the list holds it', () => {
		expect(nearest('E', held)).toBe('E');
	});

	it('is the next one held after an empty letter', () => {
		expect(nearest('B', held)).toBe('E');
	});

	it('is the last one held past the end', () => {
		expect(nearest('Z', held)).toBe('S');
	});

	it('is nothing on an empty list', () => {
		expect(nearest('A', new Set())).toBeNull();
	});
});
