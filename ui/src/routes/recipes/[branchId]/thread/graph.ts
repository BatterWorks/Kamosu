/**
 * The Thread drawn as a graph (#115, redrawn by #250): one row per Version,
 * a dot per Version on a line down the left edge, grey while several
 * Branches still share those Versions and a colour of its own for each Branch
 * from where it parts.
 *
 * **Every line comes out of the dot of the Version it started from** (#250).
 * No row says a line begins.
 *
 * - **A Copy or a variation** leaves the last Version it shares with the
 *   recipe. Its rows are in date order across every Branch and joined to each
 *   other, as `git log --graph` reads.
 * - **A Translation** is a Branch with a line of its own, but when it was
 *   written says nothing about the recipe. Each of its Versions sits directly
 *   under the Version it translates and hangs off that Version's dot. Nothing
 *   joins one of its rows to the next, unless both translate the same Version.
 *   One whose source Version is not in the Thread goes at the foot.
 * - **A Branch with no Version of its own yet** gets a row under the Version
 *   it starts from, with nothing in it but its name.
 *
 * This file only decides the shape: which row goes where, which line a
 * Version sits on, which lines pass each row, which lines leave its dot. It
 * knows nothing about pixels or colours, so the drawing can change without
 * the shape moving, and the shape can be tested without a screen.
 *
 * It is built on `buildGroup` (tree.ts), which knows where Branches that
 * share a chain part: by comparing each Branch's own chain position by
 * position, never by Version id alone (ADR 0004).
 */

import type { RenameVersionInput } from '$lib/api/catalogue';
import { buildGroup, chainsByBranch, type ForkGroup, type ThreadVersion } from './tree';

/**
 * Which colour a line is drawn in: `'shared'` while several Branches share it
 * (grey), otherwise the Branch's place in the Thread's list of Branches — the
 * screen cycles its colours over that number.
 */
export type Tone = 'shared' | number;

/** A lane and the colour it is drawn in at that moment. */
export interface Lane {
	lane: number;
	tone: Tone;
}

/**
 * One line passing through a row. `reach` says which part of the row it
 * covers: all of it, only down to the dot (the line ends here), or only from
 * the dot down (the line starts here).
 */
export interface Line extends Lane {
	reach: 'full' | 'above' | 'below' | 'none';
}

/**
 * Where one Version occurs — what `rename_version` targets (#115) — and whose
 * Hand saved that occurrence, since only that Hand may rename it.
 */
export type Occurrence = Pick<RenameVersionInput, 'branch_id' | 'sequence'> & { hand_id: string };

/** How a row is known: one occurrence on one Branch. */
export const rowKey = (version: Pick<ThreadVersion, 'branch_id' | 'sequence'>) =>
	`${version.branch_id}:${version.sequence}`;

export type GraphRow =
	| {
			kind: 'version';
			key: string;
			/** The occurrence the screen reads from — the first of `occurrences`. */
			version: ThreadVersion;
			/**
			 * Every Branch this row stands for holds the Version at the same place
			 * in its chain, each as an occurrence of its own. A shared Version is
			 * drawn once, so renaming it renames it on each of them.
			 */
			occurrences: Occurrence[];
			branchIds: string[];
			lane: number;
			tone: Tone;
			lines: Line[];
			/**
			 * The lines that come out of this row's dot and run down from it
			 * (#250): a Copy parting here, a Translation of this Version, a
			 * Branch that starts here and has changed nothing.
			 */
			sprouts: Lane[];
			/** Whether the row says which Branch it is on: every row once a second line is drawn. */
			labelled: boolean;
			/**
			 * A Translation's Version whose source is not in the Thread, on the
			 * first row of its line: the row says so in words, having no Version
			 * to sit under.
			 */
			adrift: boolean;
	  }
	| {
			/**
			 * A Branch with no Version past this point, under the last Version it
			 * holds: one that starts there and has changed nothing, or the recipe
			 * itself, left as it was while a variation carried on.
			 */
			kind: 'unchanged';
			key: string;
			branchId: string;
			/** Whether it is the recipe that stopped here, and not a Branch that starts here. */
			stopped: boolean;
			lane: number;
			tone: Tone;
			lines: Line[];
	  };

export interface Graph {
	rows: GraphRow[];
	/** How many lanes the widest moment needs — the gutter's width. */
	lanes: number;
}

export type VersionRow = Extract<GraphRow, { kind: 'version' }>;

/** Which part of a row a line covers, from whether it reaches the top and the bottom. */
const reaches = (top: boolean, bottom: boolean): Line['reach'] =>
	top ? (bottom ? 'full' : 'above') : bottom ? 'below' : 'none';
const hasTop = (reach: Line['reach']) => reach === 'full' || reach === 'above';
const hasBottom = (reach: Line['reach']) => reach === 'full' || reach === 'below';
const byLane = (a: Lane, b: Lane) => a.lane - b.lane;
const laneOf = ({ lane, tone }: Lane): Lane => ({ lane, tone });

interface Segment {
	branchIds: string[];
	trunk: ThreadVersion[];
	children: ForkGroup['children'];
	unchanged: string[];
	stopped?: string;
	tone: Tone;
	lane: number;
	/** How many of `trunk` have been drawn. */
	next: number;
	/** Whether a line reaches this segment's first row from above. */
	fromAbove: boolean;
	/** The order segments were met in — the tie-break between equal dates. */
	order: number;
	/** The last row drawn for it, which what it turns into comes out of. */
	last?: VersionRow;
}

/** One Version of a Translation, before it has a place on the page. */
interface Rendering {
	row: VersionRow;
	/** The Version before it on its Branch. */
	parent?: Rendering;
	translates: string | null;
	/** Which Translation it belongs to, counting those that share no chain. */
	family: number;
}

/** A Translation's rows that sit together: one Version, or several that translate the same one. */
type Block = Rendering[];

/**
 * Lay the Thread out. `branchIds` is the Thread's own order of Branches
 * (oldest first), which is also the order colours are handed out in.
 */
export function graphOf(branchIds: string[], versions: ThreadVersion[]): Graph {
	const chains = chainsByBranch(versions);
	const held = branchIds.filter((id) => chains.has(id));
	if (held.length === 0) return { rows: [], lanes: 1 };
	const root = buildGroup(held, chains, 0);
	// Branches that share no first Version are apart from the start: the
	// recipe on one side, each Translation of it on another.
	const families = root.trunk.length ? [{ branchIds: held, group: root }] : root.children;
	const isTranslation = (ids: string[]) =>
		ids.some((id) => chains.get(id)?.[0]?.translates_version_id != null);
	const toneOf = (ids: string[]): Tone => (ids.length === 1 ? branchIds.indexOf(ids[0]) : 'shared');

	const rowOf = (version: ThreadVersion, ids: string[], position: number): VersionRow => ({
		kind: 'version',
		key: rowKey(version),
		version,
		occurrences: ids.flatMap((id) => {
			const occurrence = chains.get(id)?.[position];
			return occurrence
				? [{ branch_id: id, sequence: occurrence.sequence, hand_id: occurrence.hand_id }]
				: [];
		}),
		branchIds: ids,
		lane: -1,
		tone: toneOf(ids),
		lines: [],
		sprouts: [],
		labelled: false,
		adrift: false,
	});

	/** The Branches with nothing of their own that start at a row, by that row's key. */
	const unchangedAt = new Map<string, string[]>();
	/** The Branch that holds nothing past a row its Versions were written down to, by that row's key. */
	const stoppedAt = new Map<string, string>();

	// ---- the recipe and its Copies: lines through time ---------------------

	const main: VersionRow[] = [];
	/** The colour a row's own line takes below it, where that is not the row's own. */
	const carriesOn = new Map<VersionRow, Tone>();
	const lanes: (Segment | null)[] = [];
	const ready: Segment[] = [];
	let order = 0;

	const firstFree = () => {
		const at = lanes.indexOf(null);
		return at === -1 ? lanes.length : at;
	};
	const place = (segment: Segment, lane = firstFree()) => {
		segment.lane = lane;
		lanes[lane] = segment;
	};
	const segmentOf = (group: ForkGroup, ids: string[], fromAbove: boolean): Segment => ({
		branchIds: ids,
		trunk: group.trunk,
		children: group.children,
		unchanged: group.unchanged,
		stopped: group.stopped,
		tone: toneOf(ids),
		lane: -1,
		next: 0,
		fromAbove,
		order: order++,
	});
	const liveLanes = (): Lane[] =>
		lanes.flatMap((segment) => (segment ? [{ lane: segment.lane, tone: segment.tone }] : []));

	/** Draw what a segment turns into once its own Versions are all drawn. */
	const finish = (segment: Segment) => {
		if (segment.last && segment.unchanged.length) {
			unchangedAt.set(segment.last.key, segment.unchanged);
		}
		const continuing = segment.children.map((child) =>
			segmentOf(child.group, child.branchIds, true),
		);
		// The first continuation carries on in the same lane, unless the lane
		// is the recipe's and the recipe stops here: then it keeps its lane for
		// the row that says so. Every other one comes out of the dot of the
		// last Version they all share.
		const keeps = segment.last !== undefined && segment.stopped !== undefined;
		const stays = keeps ? undefined : continuing.at(0);
		const leave = keeps ? continuing : continuing.slice(1);
		if (stays) {
			place(stays, segment.lane);
			if (segment.last) carriesOn.set(segment.last, stays.tone);
		}
		if (segment.last && segment.stopped) {
			stoppedAt.set(segment.last.key, segment.stopped);
			carriesOn.set(segment.last, toneOf([segment.stopped]));
		}
		for (const next of leave) {
			place(next);
			segment.last?.sprouts.push({ lane: next.lane, tone: next.tone });
		}
		if (!stays) {
			lanes[segment.lane] = null;
			while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop();
		}
		ready.push(...continuing);
	};

	for (const family of families) {
		if (isTranslation(family.branchIds)) continue;
		const first = segmentOf(family.group, family.branchIds, false);
		place(first);
		ready.push(first);
	}

	while (ready.length) {
		// The segment whose next Version is the oldest; equal dates go in the
		// order the segments were met, so a split's first Branch comes first.
		ready.sort(
			(a, b) =>
				a.trunk[a.next].created_at.localeCompare(b.trunk[b.next].created_at) || a.order - b.order,
		);
		const segment = ready[0];
		const version = segment.trunk[segment.next];
		const isFirst = segment.next === 0;
		const isLast = segment.next === segment.trunk.length - 1;
		const ends = isLast && segment.children.length === 0;
		const own = reaches(!isFirst || segment.fromAbove, !ends);

		const position = chains.get(segment.branchIds[0])?.indexOf(version) ?? -1;
		const row = rowOf(version, segment.branchIds, position);
		row.lane = segment.lane;
		row.lines = liveLanes().map(({ lane, tone }) => ({
			lane,
			tone,
			reach: lane === segment.lane ? own : 'full',
		}));
		main.push(row);
		segment.last = row;

		segment.next++;
		if (isLast) {
			ready.shift();
			finish(segment);
		}
	}

	// ---- each Translation: its Versions, and what each one translates ------

	const renderings: Rendering[] = [];
	const read = (
		group: ForkGroup,
		ids: string[],
		from: number,
		family: number,
		parent?: Rendering,
	) => {
		let last = parent;
		group.trunk.forEach((version, index) => {
			const rendering: Rendering = {
				row: rowOf(version, ids, from + index),
				parent: last,
				translates: version.translates_version_id,
				family,
			};
			renderings.push(rendering);
			last = rendering;
		});
		if (last && group.unchanged.length) unchangedAt.set(last.row.key, group.unchanged);
		if (last && group.stopped) stoppedAt.set(last.row.key, group.stopped);
		for (const child of group.children) {
			read(child.group, child.branchIds, from + group.trunk.length, family, last);
		}
	};
	families.forEach((family, index) => {
		if (isTranslation(family.branchIds)) read(family.group, family.branchIds, 0, index);
	});

	// A save that translates the same Version as the one before it sits with
	// it. Anything else is a rendering of another Version, and sits there.
	const blocks: Block[] = [];
	const blockOf = new Map<Rendering, Block>();
	for (const rendering of renderings) {
		const before = rendering.parent && blockOf.get(rendering.parent);
		const block =
			before &&
			before.at(-1) === rendering.parent &&
			rendering.parent?.translates === rendering.translates
				? before
				: [];
		if (block.length === 0) blocks.push(block);
		block.push(rendering);
		blockOf.set(rendering, block);
	}

	// Which row holds the Version a block translates. One id can occur more
	// than once (ADR 0004): the occurrence it was made from is the newest one
	// saved by then, and failing that the oldest there is.
	const holding = new Map<string, { row: VersionRow; family: number }[]>();
	const holds = (row: VersionRow, family: number) => {
		const list = holding.get(row.version.version_id) ?? [];
		list.push({ row, family });
		holding.set(row.version.version_id, list);
	};
	for (const row of main) holds(row, -1);
	for (const rendering of renderings) holds(rendering.row, rendering.family);

	const hung = new Map<string, Block[]>();
	const adrift: Block[] = [];
	for (const block of blocks) {
		const [head] = block;
		const sources = (head.translates ? (holding.get(head.translates) ?? []) : [])
			.filter((each) => each.family !== head.family)
			.map((each) => each.row)
			.sort((a, b) => a.version.created_at.localeCompare(b.version.created_at));
		// A second Copy of a Translation, parting at one of its Versions,
		// renders what that Version does. It comes out of that Version, as a
		// Copy of the recipe would, and not out of the recipe.
		const shared =
			head.parent && head.parent.translates === head.translates ? head.parent.row : undefined;
		const source =
			shared ??
			sources.findLast((row) => row.version.created_at <= head.row.version.created_at) ??
			sources[0];
		if (!source) {
			// With no Version to sit under, a Translation's rows keep each other's company.
			const before = adrift.at(-1);
			if (before && before.at(-1) === head.parent) before.push(...block);
			else adrift.push([...block]);
			continue;
		}
		const list = hung.get(source.key) ?? [];
		list.push(block);
		hung.set(source.key, list);
	}

	// ---- the page: every row, with what hangs under it right below it ------

	const rows: GraphRow[] = [];
	const several = held.length > 1;
	/**
	 * Whether a second line has been drawn yet, from where rows name their
	 * Branch. A Translation standing apart at the foot left no dot, so with
	 * one of those every row names its Branch from the top.
	 */
	let severalLines = several && adrift.length > 0;
	const drawn = new Set<Block>();

	/**
	 * Put a row on the page, then everything that comes out of its dot.
	 * `passing` is every line that runs on below the row, and so through
	 * whatever is hung under it.
	 */
	const draw = (row: VersionRow, below: Lane[]) => {
		rows.push(row);
		const translations = (hung.get(row.key) ?? []).filter((block) => !drawn.has(block));
		const starting = unchangedAt.get(row.key) ?? [];
		const stopped = stoppedAt.get(row.key);
		// The recipe that stops here keeps this row's lane down to a row of
		// its own, drawn last, so its line passes whatever else hangs here.
		const resting: Lane | undefined = stopped
			? { lane: row.lane, tone: toneOf([stopped]) }
			: undefined;
		const passing = resting ? [...below.filter((line) => line.lane !== row.lane), resting] : below;
		if (resting) {
			row.lines = row.lines.map((line) =>
				line.lane === row.lane ? { ...line, reach: reaches(hasTop(line.reach), true) } : line,
			);
		}
		const rest = () => {
			if (!stopped || !resting) return;
			rows.push({
				kind: 'unchanged',
				key: `unchanged:${stopped}`,
				branchId: stopped,
				stopped: true,
				...resting,
				lines: passing
					.map((line): Line => ({ ...line, reach: line.lane === row.lane ? 'above' : 'full' }))
					.sort(byLane),
			});
		};
		if (translations.length + starting.length === 0) return rest();

		const taken = new Set([row.lane, ...passing.map((line) => line.lane)]);
		// To the right of the row it comes out of, so every such line curves the same way.
		const laneFor = (): number => {
			let lane = row.lane + 1;
			while (taken.has(lane)) lane++;
			taken.add(lane);
			return lane;
		};
		const under: (Lane & { block?: Block; branchId?: string })[] = [
			...translations.map((block) => ({ lane: laneFor(), tone: block[0].row.tone, block })),
			...starting.map((branchId) => ({ lane: laneFor(), tone: toneOf([branchId]), branchId })),
		];
		row.sprouts.push(...under.map(laneOf));

		under.forEach((each, index) => {
			// Lines for what is hung further down pass through what is hung above it.
			const through = [...passing, ...under.slice(index + 1).map(laneOf)];
			const whole = through.map((line): Line => ({ ...line, reach: 'full' }));
			const own: Lane = { lane: each.lane, tone: each.tone };
			const ending: Line = { ...own, reach: 'above' };
			if (each.branchId) {
				rows.push({
					kind: 'unchanged',
					key: `unchanged:${each.branchId}`,
					branchId: each.branchId,
					stopped: false,
					...own,
					lines: [...whole, ending].sort(byLane),
				});
				return;
			}
			const block = each.block ?? [];
			drawn.add(block);
			block.forEach((rendering, at) => {
				// Each row in its own colour: a Version two Copies of a
				// Translation share is grey, and what follows it is not.
				const next = block.at(at + 1);
				const line: Line = {
					lane: each.lane,
					tone: rendering.row.tone,
					reach: next ? 'full' : 'above',
				};
				rendering.row.lane = each.lane;
				rendering.row.lines = [...whole, line].sort(byLane);
				rendering.row.labelled = true;
				draw(
					rendering.row,
					next ? [...through, { lane: each.lane, tone: next.row.tone }] : through,
				);
			});
		});
		rest();
	};

	for (const row of main) {
		row.labelled = severalLines;
		// Below a row its own line is whatever carries on from it: the one
		// Branch that keeps the lane, where several shared it down to here.
		const passing = [
			...row.lines
				.filter((line) => hasBottom(line.reach))
				.map((line) =>
					line.lane === row.lane
						? { lane: line.lane, tone: carriesOn.get(row) ?? line.tone }
						: laneOf(line),
				),
			...row.sprouts,
		];
		draw(row, passing);
		if (row.sprouts.length) severalLines = true;
	}

	// A Translation whose source is not here, and any left over by a pointer
	// that leads nowhere drawable: at the foot, on a line that starts at its
	// own first row.
	// Beside the recipe's line where there is one, though it has ended by
	// then: the first lane is the recipe's, and this is not the recipe.
	const footLane = main.length ? 1 : 0;
	const atTheFoot = (block: Block) => {
		drawn.add(block);
		block.forEach((rendering, at) => {
			const more = at < block.length - 1;
			const own: Lane = { lane: footLane, tone: rendering.row.tone };
			rendering.row.lane = footLane;
			rendering.row.lines = [{ ...own, reach: reaches(at > 0, more) }];
			rendering.row.labelled = several;
			rendering.row.adrift = at === 0;
			draw(rendering.row, more ? [own] : []);
		});
	};
	const footed = new Set(adrift.flat());
	for (const block of adrift) atTheFoot(block);
	for (const block of blocks) {
		if (!drawn.has(block) && !footed.has(block[0])) atTheFoot(block);
	}

	const widest = Math.max(
		0,
		...rows.flatMap((row) => [
			...row.lines.map((line) => line.lane),
			...(row.kind === 'version' ? row.sprouts.map((line) => line.lane) : []),
		]),
	);
	return { rows, lanes: widest + 1 };
}

/** A row as the screen shows it: on its own, or folded into a run of quiet saves. */
export type Item =
	{ kind: 'row'; row: GraphRow } | { kind: 'run'; key: string; rows: VersionRow[] };

const RUN_THRESHOLD = 4;

/**
 * Fold runs of quiet saves — four or more in a row, on the same line, with
 * nothing else drawn between them — into one row the cook can open. What
 * counts as quiet is the screen's to say.
 */
export function runsOf(rows: GraphRow[], isQuiet: (row: VersionRow) => boolean): Item[] {
	const items: Item[] = [];
	let run: VersionRow[] = [];
	const flush = () => {
		if (run.length >= RUN_THRESHOLD) {
			items.push({ kind: 'run', key: `run:${run[0].key}`, rows: run });
		} else {
			for (const row of run) items.push({ kind: 'row', row });
		}
		run = [];
	};
	for (const row of rows) {
		// A row something comes out of, or one that has to say where it stands,
		// is never folded away (#250).
		if (row.kind === 'version' && !row.sprouts.length && !row.adrift && isQuiet(row)) {
			if (run.length && run[run.length - 1].lane !== row.lane) flush();
			run.push(row);
			continue;
		}
		flush();
		items.push({ kind: 'row', row });
	}
	flush();
	return items;
}

/**
 * The lines a folded run draws as one row: its own line reaches as far up as
 * the run's first row does and as far down as its last does. Nothing else
 * can start or stop inside a run: a line starts at a row something comes out
 * of, which is never folded, and stops on a row of its own, which would have
 * broken the run.
 */
export function foldedLines(rows: VersionRow[]): Line[] {
	const first = rows[0];
	const last = rows[rows.length - 1];
	const lastOwn = last.lines.find((line) => line.lane === last.lane);
	return first.lines.map((line) =>
		line.lane === first.lane
			? { ...line, reach: reaches(hasTop(line.reach), hasBottom(lastOwn?.reach ?? 'none')) }
			: line,
	);
}

/** The lines that carry on below a row, drawn the whole height of a row with no dot. */
export function linesBelow(row: VersionRow): Line[] {
	return row.lines
		.filter((line) => hasBottom(line.reach))
		.map((line) => ({ ...line, reach: 'full' }));
}
