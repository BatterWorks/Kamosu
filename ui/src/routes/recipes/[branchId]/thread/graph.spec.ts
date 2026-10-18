import { describe, expect, it } from 'vitest';
import { foldedLines, graphOf, linesBelow, runsOf, type GraphRow } from './graph';
import type { ThreadVersion } from './tree';

let clock = 0;
function v(
	branch_id: string,
	sequence: number,
	version_id: string,
	created_at: string,
	extra: Partial<ThreadVersion> = {},
): ThreadVersion {
	clock++;
	return {
		branch_id,
		sequence,
		version_id,
		parent_version_id: null,
		hand_id: 'h_me',
		hand_name: 'Me',
		name: null,
		change_note: `note ${clock}`,
		created_at,
		translates_version_id: null,
		language: 'en',
		...extra,
	};
}

type VersionRow = Extract<GraphRow, { kind: 'version' }>;

/**
 * Each Version's row as `id@lane`, with `→[lanes]` for the lines that come
 * out of its dot, and a Branch with nothing of its own as `(branch)@lane`.
 */
function shape(rows: GraphRow[]): string[] {
	return rows.map((row) => {
		if (row.kind === 'unchanged') return `(${row.branchId})@${row.lane}`;
		const sprouts = row.sprouts.length
			? `→[${row.sprouts.map((each) => each.lane).join(',')}]`
			: '';
		return `${row.version.version_id}@${row.lane}${sprouts}`;
	});
}

const versionRow = (rows: GraphRow[], id: string): VersionRow => {
	const row = rows.find((each) => each.kind === 'version' && each.version.version_id === id);
	if (row?.kind !== 'version') throw new Error(`no row for ${id}`);
	return row;
};

/** A Version of a Translation: it names the Version it translates. */
const t = (
	branch_id: string,
	sequence: number,
	id: string,
	created_at: string,
	translates: string,
) => v(branch_id, sequence, id, created_at, { translates_version_id: translates, language: 'fr' });

describe('the Thread as a graph (#115)', () => {
	it('draws one Branch as one line in its own colour, starting and ending at a dot', () => {
		const graph = graphOf(
			['b_1'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				v('b_1', 3, 'c', '2026-01-03'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0', 'c@0']);
		expect(graph.lanes).toBe(1);
		const [first, middle, last] = graph.rows as VersionRow[];
		expect(first.lines).toEqual([{ lane: 0, tone: 0, reach: 'below' }]);
		expect(middle.lines).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
		expect(last.lines).toEqual([{ lane: 0, tone: 0, reach: 'above' }]);
		// A Thread that never splits labels no row with a Branch.
		expect(first.labelled).toBe(false);
	});

	it('greys what several Branches share, sends each Copy out of the dot they part at, and interleaves them by date', () => {
		const graph = graphOf(
			['b_papi', 'b_camille', 'b_chalet'],
			[
				v('b_papi', 1, 'root', '2026-08-02'),
				v('b_camille', 1, 'root', '2026-08-02'),
				v('b_chalet', 1, 'root', '2026-08-02'),
				v('b_papi', 2, 'p1', '2026-08-30'),
				v('b_camille', 2, 'c1', '2026-09-05'),
				v('b_papi', 3, 'p2', '2026-09-12'),
				v('b_chalet', 2, 'f1', '2026-09-15'),
				v('b_camille', 3, 'c2', '2026-09-21'),
			],
		);
		// No row of its own says the recipe splits (#250): the lines leave root's dot.
		expect(shape(graph.rows)).toEqual(['root@0→[1,2]', 'p1@0', 'c1@1', 'p2@0', 'f1@2', 'c2@1']);
		expect(graph.lanes).toBe(3);

		const root = versionRow(graph.rows, 'root');
		expect(root.tone).toBe('shared');
		// The shared Version is one row, standing for its occurrence on every Branch.
		expect(root.occurrences).toEqual([
			{ branch_id: 'b_papi', sequence: 1, hand_id: 'h_me' },
			{ branch_id: 'b_camille', sequence: 1, hand_id: 'h_me' },
			{ branch_id: 'b_chalet', sequence: 1, hand_id: 'h_me' },
		]);
		expect(root.labelled).toBe(false);
		// The first Branch carries on down the same lane; the others come out of the dot.
		expect(root.lines).toEqual([{ lane: 0, tone: 'shared', reach: 'below' }]);
		expect(root.sprouts).toEqual([
			{ lane: 1, tone: 1 },
			{ lane: 2, tone: 2 },
		]);

		// Camille's first Version: her line runs through, Papi's carries on
		// past it, and the Chalet's has not had a Version yet but is already drawn.
		const c1 = versionRow(graph.rows, 'c1');
		expect(c1.tone).toBe(1);
		expect(c1.labelled).toBe(true);
		expect(c1.lines).toEqual([
			{ lane: 0, tone: 0, reach: 'full' },
			{ lane: 1, tone: 1, reach: 'full' },
			{ lane: 2, tone: 2, reach: 'full' },
		]);
		// Papi's last Version ends his line at its dot; the others carry on.
		expect(versionRow(graph.rows, 'p2').lines).toEqual([
			{ lane: 0, tone: 0, reach: 'above' },
			{ lane: 1, tone: 1, reach: 'full' },
			{ lane: 2, tone: 2, reach: 'full' },
		]);
		expect(versionRow(graph.rows, 'c2').lines).toEqual([{ lane: 1, tone: 1, reach: 'above' }]);
	});

	it('never puts a Branch’s Version above the Version it parts at, whatever its date says', () => {
		// A Copy's own first save can carry a clock earlier than the Version it
		// was copied from (another device, a wrong clock). That Version still comes first.
		const graph = graphOf(
			['b_1', 'b_2'],
			[
				v('b_1', 1, 'root', '2026-05-01'),
				v('b_2', 1, 'root', '2026-05-01'),
				v('b_1', 2, 'x', '2026-04-01'),
				v('b_2', 2, 'y', '2026-06-01'),
			],
		);
		expect(shape(graph.rows)).toEqual(['root@0→[1]', 'x@0', 'y@1']);
	});

	it('parts again off a Branch, reusing a lane another Branch has finished with', () => {
		const graph = graphOf(
			['b_mine', 'b_marc', 'b_camille'],
			[
				v('b_mine', 1, 'v1', '2026-03-03'),
				v('b_marc', 1, 'v1', '2026-03-03'),
				v('b_camille', 1, 'v1', '2026-03-03'),
				v('b_mine', 2, 'v2', '2026-04-01'),
				v('b_marc', 2, 'm1', '2026-06-02'),
				v('b_camille', 2, 'm1', '2026-06-02'),
				v('b_marc', 3, 'm2', '2026-08-10'),
				v('b_camille', 3, 'c1', '2026-08-12'),
			],
		);
		expect(shape(graph.rows)).toEqual(['v1@0→[1]', 'v2@0', 'm1@1→[0]', 'm2@1', 'c1@0']);
		// Marc and Camille still share m1, so it is grey and names them both.
		const m1 = versionRow(graph.rows, 'm1');
		expect(m1.tone).toBe('shared');
		expect(m1.branchIds).toEqual(['b_marc', 'b_camille']);
		expect(m1.labelled).toBe(true);
		expect(graph.lanes).toBe(2);
	});

	it('folds four or more quiet saves in a row on one line, and leaves a shorter run alone', () => {
		const quiet = (id: string, seq: number, day: number) =>
			v('b_1', seq, id, `2026-01-0${day}`, { change_note: null });
		const graph = graphOf(
			['b_1'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				quiet('q1', 2, 2),
				quiet('q2', 3, 3),
				quiet('q3', 4, 4),
				quiet('q4', 5, 5),
				v('b_1', 6, 'b', '2026-01-06'),
				quiet('q5', 7, 7),
			],
		);
		const items = runsOf(graph.rows, (row) => !row.version.change_note && !row.version.name);
		expect(
			items.map((item) =>
				item.kind === 'run'
					? `run ${item.rows.map((row) => row.version.version_id).join(',')}`
					: item.row.kind === 'version'
						? item.row.version.version_id
						: 'unchanged',
			),
		).toEqual(['a', 'run q1,q2,q3,q4', 'b', 'q5']);

		// Folded, the run is one row on a line that carries on through it.
		const run = items[1];
		if (run.kind !== 'run') throw new Error('expected a run');
		expect(foldedLines(run.rows)).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
		// The last Version ends the line; nothing carries on below it.
		const last = graph.rows[graph.rows.length - 1];
		if (last.kind !== 'version') throw new Error('expected a Version');
		expect(linesBelow(last)).toEqual([]);
	});
});

describe('a Translation in the Thread (#250)', () => {
	const recipe = [
		v('b_1', 1, 'a', '2026-01-01'),
		v('b_1', 2, 'b', '2026-01-02'),
		v('b_1', 3, 'c', '2026-01-03'),
		v('b_1', 4, 'd', '2026-01-04'),
	];

	it('sits under the Version it translates, however late it was written, on a line out of that dot', () => {
		const graph = graphOf(['b_1', 'b_fr'], [...recipe, t('b_fr', 1, 'fa', '2026-02-01', 'a')]);
		// Written last, drawn second: nothing splits before the first Version.
		expect(shape(graph.rows)).toEqual(['a@0→[1]', 'fa@1', 'b@0', 'c@0', 'd@0']);
		expect(graph.lanes).toBe(2);

		const fa = versionRow(graph.rows, 'fa');
		expect(fa.tone).toBe(1);
		// The recipe's line passes it, and its own stops at its dot.
		expect(fa.lines).toEqual([
			{ lane: 0, tone: 0, reach: 'full' },
			{ lane: 1, tone: 1, reach: 'above' },
		]);
		expect(fa.adrift).toBe(false);
		// Rows name their Branch from the second line on, and not before.
		expect(versionRow(graph.rows, 'a').labelled).toBe(false);
		expect(fa.labelled).toBe(true);
		expect(versionRow(graph.rows, 'b').labelled).toBe(true);
		// Nothing of the Translation's reaches the rows below it.
		expect(versionRow(graph.rows, 'b').lines).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
	});

	it('hangs each Version off the one it translates, and joins only two that translate the same one', () => {
		const graph = graphOf(
			['b_1', 'b_fr'],
			[
				...recipe,
				t('b_fr', 1, 'f1', '2026-02-01', 'b'),
				// A word corrected in the French: still a rendering of b.
				t('b_fr', 2, 'f2', '2026-02-02', 'b'),
				// Brought up to date: a rendering of d, which is where it comes out.
				t('b_fr', 3, 'f3', '2026-02-03', 'd'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0→[1]', 'f1@1', 'f2@1', 'c@0', 'd@0→[1]', 'f3@1']);
		expect(versionRow(graph.rows, 'f1').lines).toContainEqual({ lane: 1, tone: 1, reach: 'full' });
		expect(versionRow(graph.rows, 'f2').lines).toContainEqual({ lane: 1, tone: 1, reach: 'above' });
		// The French line does not run from f2 down to f3.
		expect(versionRow(graph.rows, 'c').lines).toEqual([{ lane: 0, tone: 0, reach: 'full' }]);
		expect(versionRow(graph.rows, 'f3').lines).toEqual([{ lane: 1, tone: 1, reach: 'above' }]);
	});

	it('gives two Languages of one Version a line each out of the same dot', () => {
		const graph = graphOf(
			['b_1', 'b_fr', 'b_es'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				t('b_fr', 1, 'fa', '2026-02-01', 'a'),
				t('b_es', 1, 'ea', '2026-01-15', 'a'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0→[1,2]', 'fa@1', 'ea@2', 'b@0']);
		// The Spanish line passes the French row on its way down.
		expect(versionRow(graph.rows, 'fa').lines).toEqual([
			{ lane: 0, tone: 0, reach: 'full' },
			{ lane: 1, tone: 1, reach: 'above' },
			{ lane: 2, tone: 2, reach: 'full' },
		]);
	});

	it('hangs a Translation of a Copy under the Copy’s Version', () => {
		const graph = graphOf(
			['b_1', 'b_vegan', 'b_fr'],
			[
				v('b_1', 1, 'root', '2026-01-01'),
				v('b_vegan', 1, 'root', '2026-01-01'),
				v('b_1', 2, 'x', '2026-01-02'),
				v('b_vegan', 2, 'y', '2026-01-03'),
				v('b_1', 3, 'z', '2026-01-04'),
				t('b_fr', 1, 'fy', '2026-02-01', 'y'),
			],
		);
		expect(shape(graph.rows)).toEqual(['root@0→[1]', 'x@0', 'y@1→[2]', 'fy@2', 'z@0']);
		expect(versionRow(graph.rows, 'fy').tone).toBe(2);
	});

	it('hangs a Translation of a Translation under the Translation', () => {
		const graph = graphOf(
			['b_1', 'b_fr', 'b_es'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				t('b_fr', 1, 'fa', '2026-02-01', 'a'),
				t('b_es', 1, 'ea', '2026-03-01', 'fa'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0→[1]', 'fa@1→[2]', 'ea@2', 'b@0']);
	});

	it('draws a Translation whose source is not in the Thread at the foot, on a line that starts there', () => {
		const graph = graphOf(
			['b_1', 'b_fr'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-03-01'),
				t('b_fr', 1, 'f1', '2026-02-01', 'held_elsewhere'),
				t('b_fr', 2, 'f2', '2026-02-02', 'held_elsewhere_later'),
			],
		);
		// Nothing comes out of any dot, and nothing is drawn above the first Version.
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0', 'f1@1', 'f2@1']);
		const f1 = versionRow(graph.rows, 'f1');
		// Only the first row has to say it: the second is plainly the same line.
		expect(f1.adrift).toBe(true);
		expect(versionRow(graph.rows, 'f2').adrift).toBe(false);
		expect(f1.lines).toEqual([{ lane: 1, tone: 1, reach: 'below' }]);
		expect(versionRow(graph.rows, 'f2').lines).toEqual([{ lane: 1, tone: 1, reach: 'above' }]);
		// No dot says where the second line began, so every row names its Branch.
		expect(f1.labelled).toBe(true);
		expect(versionRow(graph.rows, 'a').labelled).toBe(true);
	});

	it('draws a Translation that is the only Branch here as a line of its own', () => {
		const graph = graphOf(
			['b_fr'],
			[
				t('b_fr', 1, 'f1', '2026-02-01', 'elsewhere'),
				t('b_fr', 2, 'f2', '2026-02-02', 'elsewhere'),
			],
		);
		expect(shape(graph.rows)).toEqual(['f1@0', 'f2@0']);
		expect(versionRow(graph.rows, 'f1').labelled).toBe(false);
	});

	it('sends a Copy of a Translation out of the Translation’s Version, not the recipe’s', () => {
		const graph = graphOf(
			['b_1', 'b_fr', 'b_c1', 'b_c2'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				t('b_fr', 1, 'f', '2026-02-01', 'a'),
				t('b_c1', 1, 'f', '2026-02-01', 'a'),
				t('b_c2', 1, 'f', '2026-02-01', 'a'),
				t('b_fr', 2, 'e', '2026-02-02', 'a'),
				t('b_c1', 2, 'g', '2026-02-03', 'a'),
				t('b_c2', 2, 'h', '2026-02-04', 'a'),
			],
		);
		// One line out of the recipe's dot. The Copies part at f, which all three hold.
		expect(shape(graph.rows)).toEqual(['a@0→[1]', 'f@1→[2,3]', 'g@2', 'h@3', 'e@1']);
		// f is grey and the Translation's own line below it is not.
		expect(versionRow(graph.rows, 'f').lines).toContainEqual({
			lane: 1,
			tone: 'shared',
			reach: 'full',
		});
		expect(versionRow(graph.rows, 'g').lines).toContainEqual({ lane: 1, tone: 1, reach: 'full' });
	});

	it('hangs under the occurrence it was made from, where a Version occurs twice', () => {
		// Saying a recipe's Language appends its content again (ADR 0006): one id, two rows.
		const twice = [
			v('b_1', 1, 'a', '2026-01-01'),
			v('b_1', 2, 'a', '2026-01-05'),
			v('b_1', 3, 'b', '2026-01-09'),
		];
		const early = graphOf(['b_1', 'b_fr'], [...twice, t('b_fr', 1, 'fa', '2026-01-03', 'a')]);
		expect(shape(early.rows)).toEqual(['a@0→[1]', 'fa@1', 'a@0', 'b@0']);
		const late = graphOf(['b_1', 'b_fr'], [...twice, t('b_fr', 1, 'fa', '2026-01-07', 'a')]);
		expect(shape(late.rows)).toEqual(['a@0', 'a@0→[1]', 'fa@1', 'b@0']);
	});

	it('never folds away a Version something comes out of', () => {
		const quiet = (id: string, seq: number, day: number) =>
			v('b_1', seq, id, `2026-01-0${day}`, { change_note: null });
		const graph = graphOf(
			['b_1', 'b_fr'],
			[
				quiet('q1', 1, 1),
				quiet('q2', 2, 2),
				quiet('q3', 3, 3),
				quiet('q4', 4, 4),
				quiet('q5', 5, 5),
				t('b_fr', 1, 'f1', '2026-02-01', 'q5'),
			],
		);
		const items = runsOf(graph.rows, (row) => !row.version.change_note && !row.version.name);
		expect(
			items.map((item) => (item.kind === 'run' ? `run of ${item.rows.length}` : item.row.key)),
		).toEqual(['run of 4', 'b_1:5', 'b_fr:1']);
	});
});

describe('a Branch with no Version of its own, in the Thread (#250)', () => {
	it('gets a row under the Version it starts from, beside a Branch that carried on', () => {
		const graph = graphOf(
			['b_1', 'b_idle'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_idle', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				v('b_idle', 2, 'b', '2026-01-02'),
				v('b_1', 3, 'c', '2026-01-03'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0→[1]', '(b_idle)@1', 'c@0']);
		const idle = graph.rows[2];
		if (idle.kind !== 'unchanged') throw new Error('expected a Branch with nothing of its own');
		expect(idle.tone).toBe(1);
		expect(idle.lines).toEqual([
			// The recipe's line passes it in the recipe's own colour: below b it is the recipe's alone.
			{ lane: 0, tone: 0, reach: 'full' },
			{ lane: 1, tone: 1, reach: 'above' },
		]);
		// What both hold is grey; what only the recipe holds is its own colour, and named.
		expect(versionRow(graph.rows, 'b').tone).toBe('shared');
		expect(versionRow(graph.rows, 'c').tone).toBe(0);
		expect(versionRow(graph.rows, 'c').labelled).toBe(true);
	});

	it('gives two of them a row each, beside a Copy that did change', () => {
		const graph = graphOf(
			['b_1', 'b_vegan', 'b_idle', 'b_arrived'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_vegan', 1, 'a', '2026-01-01'),
				v('b_idle', 1, 'a', '2026-01-01'),
				v('b_arrived', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				v('b_vegan', 2, 'y', '2026-01-03'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0→[1,2,3]', '(b_idle)@2', '(b_arrived)@3', 'b@0', 'y@1']);
		expect(graph.lanes).toBe(4);
	});

	it('takes the oldest Branch for the recipe where every Branch stops at the same Version', () => {
		const graph = graphOf(
			['b_1', 'b_2'],
			[v('b_1', 1, 'a', '2026-01-01'), v('b_2', 1, 'a', '2026-01-01')],
		);
		expect(shape(graph.rows)).toEqual(['a@0→[1]', '(b_2)@1']);
	});

	it('keeps the recipe on its own line where it stopped and a variation carried on', () => {
		const graph = graphOf(
			['b_1', 'b_vegan'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_vegan', 1, 'a', '2026-01-01'),
				v('b_1', 2, 'b', '2026-01-02'),
				v('b_vegan', 2, 'b', '2026-01-02'),
				v('b_vegan', 3, 'y', '2026-01-03'),
				v('b_vegan', 4, 'z', '2026-01-04'),
			],
		);
		// The recipe did not start at b: it was written down to b and left there.
		expect(shape(graph.rows)).toEqual(['a@0', 'b@0→[1]', '(b_1)@0', 'y@1', 'z@1']);
		const rested = graph.rows[2];
		if (rested.kind !== 'unchanged') throw new Error('expected the recipe, at rest');
		expect(rested.stopped).toBe(true);
		expect(rested.lines).toEqual([
			{ lane: 0, tone: 0, reach: 'above' },
			{ lane: 1, tone: 1, reach: 'full' },
		]);
		// The variation's line is its own from the dot it comes out of.
		expect(versionRow(graph.rows, 'y').lines).toEqual([{ lane: 1, tone: 1, reach: 'full' }]);
	});

	it('says a Branch starts where it does, beside the recipe that stopped there', () => {
		const graph = graphOf(
			['b_1', 'b_vegan', 'b_idle'],
			[
				v('b_1', 1, 'a', '2026-01-01'),
				v('b_vegan', 1, 'a', '2026-01-01'),
				v('b_idle', 1, 'a', '2026-01-01'),
				v('b_vegan', 2, 'y', '2026-01-03'),
			],
		);
		expect(shape(graph.rows)).toEqual(['a@0→[1,2]', '(b_idle)@2', '(b_1)@0', 'y@1']);
		expect(graph.rows.flatMap((row) => (row.kind === 'unchanged' ? [row.stopped] : []))).toEqual([
			false,
			true,
		]);
	});
});
