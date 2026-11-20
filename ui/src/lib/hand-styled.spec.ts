/**
 * Buttons are drawn by the shared pieces (#632, `docs/ux-rules.md` U9).
 *
 * A button or a link a screen gives a fill or a border of its own is a
 * button drawn by hand: the audit of November 2026 counted 58 filled indigo
 * controls drawn 13 ways. The pieces are `main-button`, `second-button` and
 * `quiet-button`, with the controls that have one shape of their own
 * (`shelf-arrow`, `step-button`, `tap-out`) and the rows of a
 * list (`list-row`).
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

/** The files #632 found drawing their own, less the share screen it moved and the Kitchens page #648 did. */
const STILL_HAND_STYLED: string[] = [
	'lib/AddOrImport.svelte',
	'lib/Confirm.svelte',
	'lib/CookbookJoins.svelte',
	'lib/HowMuch.svelte',
	'lib/OneTimeSecret.svelte',
	'lib/PasteCheck.svelte',
	'lib/PhotoToRecipe.svelte',
	'lib/PlanPicker.svelte',
	'lib/RefusedExport.svelte',
	'lib/WhichHousehold.svelte',
	'lib/adding/AddressForm.svelte',
	'lib/adding/PasteForm.svelte',
	'lib/adding/PasteSheet.svelte',
	'lib/adding/Replays.svelte',
	'lib/adding/SendSheet.svelte',
	'lib/adding/TitleForm.svelte',
	'lib/adding/VideoForm.svelte',
	'lib/adding/WatchExamples.svelte',
	'lib/agent/AgentWork.svelte',
	'lib/offline/InstallSteps.svelte',
	'lib/offline/Notice.svelte',
	'lib/operator/HouseNotes.svelte',
	'lib/operator/RunnerKeySecret.svelte',
	'lib/shell/BackArrow.svelte',
	'lib/shell/PlaceLink.svelte',
	'routes/+layout.svelte',
	'routes/Account.svelte',
	'routes/DemoWelcome.svelte',
	'routes/cook/[branchId]/Cooking.svelte',
	'routes/cookbook-invite/[secret]/+page.svelte',
	'routes/cooked/Diary.svelte',
	'routes/cooked/KeepCooking.svelte',
	'routes/foods/FoodList.svelte',
	'routes/foods/[foodId]/Food.svelte',
	'routes/import/ImportShared.svelte',
	'routes/imports/Imports.svelte',
	'routes/imports/[jobId]/Report.svelte',
	'routes/imports/[jobId]/SendAgain.svelte',
	'routes/kitchen-invite/[secret]/+page.svelte',
	'routes/plan/ChangeSheet.svelte',
	'routes/plan/MoveSheet.svelte',
	'routes/plan/Plan.svelte',
	'routes/plan/SendToShopping.svelte',
	'routes/plan/WriteSheet.svelte',
	'routes/recipes/FilterList.svelte',
	'routes/recipes/MeaningOffer.svelte',
	'routes/recipes/Plus.svelte',
	'routes/recipes/Shelf.svelte',
	'routes/recipes/[branchId]/Carrying.svelte',
	'routes/recipes/[branchId]/ComponentPicker.svelte',
	'routes/recipes/[branchId]/Correcting.svelte',
	'routes/recipes/[branchId]/LanguageOffer.svelte',
	'routes/recipes/[branchId]/LanguageSheet.svelte',
	'routes/recipes/[branchId]/MarkedRow.svelte',
	'routes/recipes/[branchId]/PlanThis.svelte',
	'routes/recipes/[branchId]/Promotion.svelte',
	'routes/recipes/[branchId]/ReadAgain.svelte',
	'routes/recipes/[branchId]/RelatedSheet.svelte',
	'routes/recipes/[branchId]/RenameSheet.svelte',
	'routes/recipes/[branchId]/StepUsesSheet.svelte',
	'routes/recipes/[branchId]/TagSheet.svelte',
	'routes/recipes/[branchId]/Tags.svelte',
	'routes/recipes/[branchId]/VersionStrip.svelte',
	'routes/recipes/[branchId]/Writing.svelte',
	'routes/recipes/[branchId]/thread/ThreadGraph.svelte',
	'routes/recipes/[branchId]/versions/OlderVersion.svelte',
	'routes/recipes/[branchId]/versions/UseVersion.svelte',
	'routes/settings/CalendarLink.svelte',
	'routes/settings/ChangePassword.svelte',
	'routes/settings/Passkeys.svelte',
	'routes/settings/SentToAgent.svelte',
	'routes/settings/SettingsMenu.svelte',
	'routes/settings/ShareFromYourPhone.svelte',
	'routes/settings/SignOut.svelte',
	'routes/settings/SigningIn.svelte',
	'routes/settings/about/+page.svelte',
	'routes/settings/address/+page.svelte',
	'routes/settings/backups/+page.svelte',
	'routes/settings/device/+page.svelte',
	'routes/settings/keys/+page.svelte',
	'routes/settings/language/+page.svelte',
	'routes/settings/name/+page.svelte',
	'routes/settings/people/+page.svelte',
	'routes/settings/runner/+page.svelte',
	'routes/settings/upkeep/+page.svelte',
	'routes/shared/Shared.svelte',
];

const PIECES = [
	'main-button',
	'second-button',
	'quiet-button',
	'shelf-arrow',
	'step-button',
	'tap-out',
	'list-row',
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
