/**
 * Buttons are drawn by the shared pieces (#632, `docs/ux-rules.md` U9).
 *
 * A button or a link a screen gives a fill or a border of its own is a
 * button drawn by hand: the audit of November 2026 counted 58 filled indigo
 * controls drawn 13 ways. The pieces are `main-button`, `second-button` and
 * `quiet-button`, with the controls that have one shape of their own
 * (`shelf-arrow`, `step-button`, `tap-out`, `version-chip`, `tag-chip`) and
 * the rows of a list (`list-row`).
 *
 * The files that drew their own when the pieces arrived are listed below.
 * The list only shrinks: each area ticket of #627 moves its files onto the
 * pieces and takes them off it, and a file that no longer draws its own
 * fails here until it is taken off. A file not on the list that draws its
 * own fails here too. Never add one to make this pass.
 *
 * What counts is read off each `<button>`, `<a>` and `NeedsServer` in a
 * `.svelte` file's markup: its class, its `class:` directives, and a
 * `NeedsServer`'s `shapeClass`, `lookClass` and `idleClass`. A name in them
 * that the file sets to a string (`const MATCHA = 'border …'`) is read as
 * that string. A class given any `bg-` colour or any `border` utility, and
 * none of the pieces, is drawn by hand; a reset (`bg-transparent`,
 * `border-0`, `border-none`) is not a style. A class that names a piece
 * anywhere passes, one branch of a condition included. Not read: a class
 * built some other way, a `<svelte:element>`, and a `<label>` drawn as a
 * button.
 */

import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import { parse } from 'svelte/compiler';
import { describe, expect, it } from 'vitest';

const SRC = join(import.meta.dirname, '..');

/**
 * The files #632 found drawing their own, less the share screen it moved, the
 * Kitchens page #648 did, Settings, Foods, Import history and the Operator's
 * pages #653 did, the Meal Plan and its sheets #654 did, and the files no
 * area ticket owned, which #688 did. Of those, three wait for a design
 * choice raised on #688: the pasted lines tapped to split a paste
 * (`PasteCheck`), the cooking pictures to choose from (`PhotoToRecipe`) and
 * the tiles of the examples to watch (`Replays`).
 */
const STILL_HAND_STYLED: string[] = [
	'lib/PasteCheck.svelte',
	'lib/PhotoToRecipe.svelte',
	'lib/adding/Replays.svelte',
	'lib/shell/BackArrow.svelte',
	'lib/shell/PlaceLink.svelte',
	'routes/+layout.svelte',
];

const PIECES = [
	'main-button',
	'second-button',
	'quiet-button',
	'shelf-arrow',
	'step-button',
	'tap-out',
	'list-row',
	// A version's chip stands on the recipe's indigo or beni band, where
	// the pieces drawn for the paper would not read (#666).
	'version-chip',
	// A recipe's Tag, today's outlined word, chosen over the quiet button (#666).
	'tag-chip',
];

const FILL = /(?<![\w-])bg-(?!transparent(?![\w-]))[a-z]/;
const BORDER = /(?<![\w-])border(?!-(?:0|none)(?![\w-]))(?:-[\w-]+)?(?![\w-])/;
const PIECE = new RegExp(`(?<![\\w-])(?:${PIECES.join('|')})(?![\\w-])`);

/** Whether a control's class, as written in the source, draws it by hand. */
export function drawnByHand(classes: string): boolean {
	return !PIECE.test(classes) && (FILL.test(classes) || BORDER.test(classes));
}

type Node = {
	type?: string;
	name?: string;
	start: number;
	end: number;
	attributes?: Node[];
	[key: string]: unknown;
};

/** The lines of a `.svelte` file where a control is drawn by hand. */
export function handStyledLines(source: string): number[] {
	const ast = parse(source, { modern: true });
	// Each name the file sets to a string, read where a class names it.
	const strings = new Map(
		[...source.matchAll(/(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*(['"`])([^'"`]*)\2/g)].map(
			([, name, , text]) => [name, text] as const,
		),
	);
	const withStrings = (classes: string) =>
		[classes, ...(classes.match(/[A-Za-z_$][\w$]*/g) ?? []).map((word) => strings.get(word))]
			.filter((part) => part !== undefined)
			.join(' ');
	const lines: number[] = [];
	const visit = (value: unknown): void => {
		if (Array.isArray(value)) return value.forEach(visit);
		if (value === null || typeof value !== 'object') return;
		const node = value as Node;
		const control =
			(node.type === 'RegularElement' && (node.name === 'button' || node.name === 'a')) ||
			(node.type === 'Component' && node.name === 'NeedsServer');
		if (control) {
			const classes = (node.attributes ?? [])
				.filter(
					(attribute) =>
						(attribute.type === 'Attribute' &&
							['class', 'shapeClass', 'lookClass', 'idleClass'].includes(attribute.name ?? '')) ||
						attribute.type === 'ClassDirective',
				)
				.map((attribute) => source.slice(attribute.start, attribute.end))
				.join(' ');
			if (drawnByHand(withStrings(classes)))
				lines.push(source.slice(0, node.start).split('\n').length);
		}
		for (const [key, child] of Object.entries(node)) {
			if (key !== 'attributes' && key !== 'metadata') visit(child);
		}
	};
	visit(ast.fragment);
	return lines;
}

function svelteFiles(folder: string): string[] {
	return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
		const path = join(folder, entry.name);
		if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : svelteFiles(path);
		return entry.name.endsWith('.svelte') ? [path] : [];
	});
}

const found = new Map(
	svelteFiles(SRC)
		.map((path) => [relative(SRC, path), handStyledLines(readFileSync(path, 'utf8'))] as const)
		.filter(([, lines]) => lines.length > 0),
);

describe('buttons drawn by hand', () => {
	it('are only in the files still allowed to draw their own', () => {
		const newOnes = [...found]
			.filter(([file]) => !STILL_HAND_STYLED.includes(file))
			.map(([file, lines]) => `${file}:${lines.join(',')}`);
		expect(
			newOnes,
			'Draw these with main-button, second-button or quiet-button (docs/ux-rules.md, U9)',
		).toEqual([]);
	});

	it('are still in every file on the list, which only shrinks', () => {
		const moved = STILL_HAND_STYLED.filter((file) => !found.has(file));
		expect(moved, 'These draw no button of their own now: take them off the list').toEqual([]);
	});

	it('are listed once each, in order', () => {
		expect(STILL_HAND_STYLED).toEqual([...new Set(STILL_HAND_STYLED)].sort());
	});
});

describe('what counts as drawn by hand', () => {
	it.each([
		['the fill a screen picked', 'mt-3 block w-full bg-accent p-4 text-on-accent', true],
		['an outline a screen picked', 'block border border-rule p-4 text-accent', true],
		['a conditional fill', "{pressed ? 'bg-accent' : 'bg-card'} min-h-12", true],
		['the main button', 'mt-3 main-button w-full', false],
		['a secondary button that loses', 'second-button loses w-full', false],
		['a quiet button in beni', 'quiet-button text-support', false],
		['a text link', 'text-accent underline', false],
		['a word that only says border in passing', 'text-accent borderless-thing', false],
		['a fill taken away', 'bg-transparent text-on-accent', false],
		['a border taken away', 'border-0 text-accent', false],
		['an outline that is there', 'border-2 border-accent', true],
	])('%s', (_name, classes, byHand) => {
		expect(drawnByHand(classes)).toBe(byHand);
	});

	it('reads a class a file keeps in a string of its own', () => {
		const lines = handStyledLines(
			[
				"<script>const MATCHA = 'rounded-sm border border-support-2 px-2';</script>",
				'<button class={MATCHA}>Change</button>',
				'<button class="{MATCHA} text-accent">Change</button>',
			].join('\n'),
		);
		expect(lines).toEqual([2, 3]);
	});

	it('reads a NeedsServer’s classes and a class: directive', () => {
		const lines = handStyledLines(
			[
				'<NeedsServer label="Save" waiting="" onclick={() => {}} shapeClass="p-3" lookClass="border border-rule" />',
				'<button class:bg-accent={on}>Go</button>',
				'<button class="second-button">Fine</button>',
			].join('\n'),
		);
		expect(lines).toEqual([1, 2]);
	});
});
