<!--
	The Thread drawn as a graph (#115, choice "G1", replacing #53's
	side-by-side lanes; redrawn by #250). One row per Version; a dot per
	Version on a line down the left edge, grey while several Branches share
	it and a colour per Branch from where it parts. Every line comes out of
	the dot of the Version it started from, and no row says a line begins.
	`graph.ts` decides the shape, which rows go where included; this only
	draws it.

	NEWEST FIRST, as chosen on #652 (`history-phone.html?ck=A`, 21 November
	2026): the graph is turned upside down, so a line that leaves a dot runs
	up from it. The newest Version of the recipe the page was opened from
	says "The recipe now", and the newest of every other line says it too,
	by that line's name. Each Version is one row that opens, with a › at its
	end, and who cooked it sits in the row as words (cookings "A"), never as
	an unexplained chip.

	A Version you saved can be named or renamed in place, under its row
	(choice "A"). Its *what changed* line is shown beside the field
	and never offered for editing: it records why an edit was made, at the
	moment that was true, and nothing can write it afterwards (#83).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { aDate } from '#lib/dates.js';
	import Refusal from '#lib/Refusal.svelte';
	import { demoRefusalOr } from '#lib/demo.svelte.js';
	import { ratingVerdict } from '#lib/rating.js';
	import { isUnknown, languageName } from '#lib/language.js';
	import { OperationError } from '#lib/api/client.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { branchLabel, yoursAmong } from '#lib/cookbook.js';
	import {
		foldedLines,
		graphOf,
		linesBelow,
		newestFirst,
		rowKey,
		runsOf,
		type Lane,
		type Line,
		type Tone,
		type VersionRow,
	} from './graph';
	import ActIcon from '../ActIcon.svelte';
	import type { ThreadBranch, ThreadVersion } from './tree';
	import type { GetThreadOutput } from '#lib/api/catalogue.js';

	type Attempt = GetThreadOutput['attempts'][number];

	interface Props {
		thread: GetThreadOutput;
		/** The recipe the History was opened from, whose newest Version is "the recipe now". */
		branchId: string;
		/** The reader's own Hand: only the Versions it saved offer a rename. */
		me: string | undefined;
		attemptsByVersion: Map<string, Attempt[]>;
		/** Which rows said what Language this recipe is in (#106). */
		languageSaid: Map<string, string>;
		onOpenVersion: (version: ThreadVersion) => void;
		/** One badge was pressed: the cookings it stands for, oldest first. */
		onOpenCookings: (cookings: Attempt[]) => void;
		/** A rename landed: read the Thread again. */
		onRenamed: () => Promise<void>;
	}

	let {
		thread,
		branchId,
		me,
		attemptsByVersion,
		languageSaid,
		onOpenVersion,
		onOpenCookings,
		onRenamed,
	}: Props = $props();

	const kamosu = useKamosu();

	const branches = $derived(new Map(thread.branches.map((branch) => [branch.branch_id, branch])));
	/** Your own version, which the switch labels *your Cookbook* (#136). */
	const yoursId = $derived(yoursAmong(thread.branches)?.branch_id);
	const graph = $derived(
		newestFirst(
			graphOf(
				thread.branches.map((branch) => branch.branch_id),
				thread.versions,
			),
		),
	);

	/** The newest Version of each Branch, by its place in that Branch's chain. */
	const newest = $derived.by(() => {
		const at = new Map<string, number>();
		for (const version of thread.versions) {
			at.set(version.branch_id, Math.max(at.get(version.branch_id) ?? 0, version.sequence));
		}
		return at;
	});

	/**
	 * Which occurrence a row opens: the recipe the page was opened from where
	 * the row holds it, so a row that says "The recipe now" opens that recipe,
	 * and otherwise the one the row is drawn from.
	 */
	function opening(row: VersionRow): ThreadVersion {
		const here = row.occurrences.find((occurrence) => occurrence.branch_id === branchId);
		if (!here) return row.version;
		return (
			thread.versions.find(
				(version) => version.branch_id === here.branch_id && version.sequence === here.sequence,
			) ?? row.version
		);
	}

	/**
	 * What a row says when it is a recipe as it stands (#652): "The recipe
	 * now" for the one the page was opened from, wherever that row stands,
	 * and the line's own name on the newest row of any other line it alone
	 * holds. A row several lines share is the newest of another only where
	 * that line has saved nothing since, and its own row below says so.
	 */
	function nowLabel(row: VersionRow): string | undefined {
		const heads = row.occurrences.filter(
			(occurrence) => newest.get(occurrence.branch_id) === occurrence.sequence,
		);
		if (heads.some((occurrence) => occurrence.branch_id === branchId)) return m.thread_now();
		if (heads.length === 1 && row.occurrences.length === 1) {
			return m.thread_now_other({ name: branchesNamed([heads[0].branch_id]) });
		}
		return undefined;
	}

	/**
	 * A save with nothing written down, which runs of get folded away. Saying
	 * a Language is never quiet (#106): nothing was typed, but it is the whole
	 * reason that row exists.
	 */
	const isQuiet = (row: VersionRow) =>
		!row.version.name &&
		!row.version.change_note &&
		!languageSaid.has(rowKey(row.version)) &&
		// A recipe as it stands is never folded away under a count (#652).
		nowLabel(row) === undefined;

	let expanded = $state(new Set<string>());
	const items = $derived(runsOf(graph.rows, isQuiet));

	function toggle(key: string) {
		const next = new Set(expanded);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expanded = next;
	}

	// ---- drawing ------------------------------------------------------------

	const LANE_W = 18;
	const PAD = 9;
	/** How far down a row its dot's centre sits, in px. */
	const DOT_Y = 17;
	const gutter = $derived(PAD * 2 + (graph.lanes - 1) * LANE_W);
	const x = (lane: number) => PAD + lane * LANE_W;

	/**
	 * The rail colours, in the order #53 handed them out, and the two #250
	 * added so that five Branches are told apart. A sixth takes the first
	 * again: every row names its Branch in words as well.
	 */
	const TONES = [
		'var(--color-accent)',
		'var(--color-support)',
		'var(--color-support-2)',
		'var(--color-support-3)',
		'var(--color-support-4)',
	];
	const colour = (tone: Tone) =>
		tone === 'shared' ? 'var(--color-ink-2)' : TONES[tone % TONES.length];

	function lineStyle(line: Line): string {
		const spans = {
			full: 'top:0;bottom:0',
			above: `top:0;height:${DOT_Y}px`,
			below: `top:${DOT_Y}px;bottom:0`,
			none: 'display:none',
		};
		return `left:${x(line.lane) - 1}px;width:2px;${spans[line.reach]};background:${colour(line.tone)}`;
	}
	/**
	 * Round, the one exception to the near-square direction in app.css:
	 * The Thread was asked for as "coloured balls, like git", and
	 * the round dots of prototype G1 were chosen (#115).
	 */
	const dotStyle = (row: Pick<VersionRow, 'lane' | 'tone'>, hollow = false) =>
		`left:${x(row.lane) - 6}px;top:${DOT_Y - 6}px;width:12px;height:12px;border-radius:50%;` +
		(hollow
			? `background:var(--color-ground);border:2px solid ${colour(row.tone)}`
			: `background:${colour(row.tone)};box-shadow:0 0 0 2px var(--color-ground)`);

	/**
	 * A line coming out of a row's dot (#250): a curve from the dot to the
	 * lane it runs up (#652), drawn in a box 100 tall that is stretched over
	 * the part of the row above the dot.
	 */
	const sproutPath = (from: number, to: number) =>
		`M${x(from)} 100 C ${x(from)} 40, ${x(to)} 60, ${x(to)} 0`;

	/**
	 * What names a Branch, or a line several still share: the name the
	 * version switch gives it (#131), so *Yours*, a variation's own name, or
	 * whose Cookbook it is, and where it is worth saying the Language it
	 * stands in. Not the Hand: every version in one Cookbook shares one, so a
	 * variation and the recipe it came from would read as the same line.
	 * Never a Language code, and never a mark on a Branch whose Language is
	 * Unknown (#106, ADR 0006); English is unmarked, as on the shelf.
	 */
	function branchesNamed(branchIds: string[]): string {
		return branchIds
			.map((id) => branches.get(id))
			.filter((branch): branch is ThreadBranch => !!branch)
			.map((branch) =>
				branch.language === 'en' || isUnknown(branch.language)
					? branchLabel(branch, branch.branch_id === yoursId).name
					: `${branchLabel(branch, branch.branch_id === yoursId).name} (${languageName(branch.language)})`,
			)
			.join(', ');
	}

	/**
	 * Who a Hand is, in words (#113, ADR 0015), named live by the server. A
	 * Hand nothing here has a name for is said to be one, never printed as an id.
	 */
	function handName(holder: { hand_id: string; hand_name: string | null }): string {
		if (holder.hand_id === me) return m.thread_you();
		return holder.hand_name ?? m.thread_unnamed_hand();
	}

	// ---- renaming -----------------------------------------------------------

	/** The row being named, by key. */
	let editing = $state<string | undefined>(undefined);
	let draft = $state('');
	let saving = $state(false);
	let renameError = $state<string | undefined>(undefined);
	/**
	 * The last rating a cook gave a Version (#249). A later cooking with no
	 * rating leaves it standing: it is the newest thing said about next time.
	 */
	const lastRating = (cookings: Attempt[]) =>
		cookings.filter((cooking) => cooking.rating).at(-1)?.rating;

	/**
	 * Your own cookings of a Version in words (#652, V4): that you cooked it,
	 * how often where more than once, and the last verdict you gave.
	 */
	function yourCookings(cookings: Attempt[]): string {
		const rating = lastRating(cookings);
		const count = cookings.length;
		if (rating) {
			const verdict = ratingVerdict(rating);
			return count > 1
				? m.thread_you_cooked_times({ count, verdict })
				: m.thread_you_cooked({ verdict });
		}
		return count > 1 ? m.thread_you_cooked_times_plain({ count }) : m.thread_you_cooked_plain();
	}

	/**
	 * Who else cooked a Version, in the order they first did, with a count
	 * beside whoever cooked it more than once: `Sam ×2 · Camille`.
	 */
	function cooksOf(cookings: Attempt[]): string {
		const cooks = new Map<string, { name: string; count: number }>();
		for (const cooking of cookings) {
			const cook = cooks.get(cooking.person_id) ?? { name: cooking.person_name, count: 0 };
			cook.count += 1;
			cooks.set(cooking.person_id, cook);
		}
		return [...cooks.values()]
			.map((cook) => (cook.count > 1 ? `${cook.name} ×${cook.count}` : cook.name))
			.join(' · ');
	}

	/** What the last rename did, said under the row it did it to. */
	let lastRename = $state<{ key: string; cleared: boolean } | undefined>(undefined);

	/**
	 * A Version is yours to name when your Hand saved it (#115) — on every
	 * Branch the row stands for, since naming it names each of them. The
	 * server refuses anyone else; the screen does not offer it.
	 */
	const yours = (row: VersionRow) =>
		me !== undefined && row.occurrences.every((occurrence) => occurrence.hand_id === me);

	function startNaming(row: VersionRow) {
		editing = row.key;
		draft = row.version.name ?? '';
		renameError = undefined;
		lastRename = undefined;
	}

	/**
	 * Name every occurrence the row stands for. A Version several Branches
	 * still share is one row on this screen, so it takes one name; each
	 * Branch holds its own occurrence, and `rename_version` names them one by one.
	 * An empty name clears it, which is an answer and not a mistake.
	 */
	async function rename(row: VersionRow, name: string | null) {
		saving = true;
		renameError = undefined;
		try {
			for (const { branch_id, sequence } of row.occurrences) {
				await kamosu.renameVersion({ branch_id, sequence, name });
			}
			editing = undefined;
			lastRename = { key: row.key, cleared: !name?.trim() };
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			renameError = demoRefusalOr(error, m.thread_rename_failed());
		} finally {
			saving = false;
		}
		// Read back what landed either way: a shared row that failed part of
		// the way has named some Branches already, and the screen should say so.
		await onRenamed().catch((error: unknown) => {
			if (!(error instanceof OperationError)) throw error;
		});
	}

	/** What a row says it was: what was written down, or what the save itself did. */
	function whatItWas(version: ThreadVersion): string {
		const said = languageSaid.get(rowKey(version));
		if (said !== undefined) return m.thread_said_language({ language: languageName(said) });
		if (version.change_note) return version.change_note;
		return version.parent_version_id === null && !version.translates_version_id
			? m.thread_first_save()
			: m.thread_quiet_save();
	}

	/** Whether what a row says is a note somebody wrote, read at full size (V7). */
	const written = (version: ThreadVersion) =>
		!!version.change_note || languageSaid.has(rowKey(version));
</script>

{#snippet gutterOf(
	row: Pick<VersionRow, 'lane' | 'tone' | 'lines'>,
	sprouts: Lane[],
	hollow = false,
)}
	<span class="relative shrink-0" style={`width:${gutter}px`} aria-hidden="true">
		{#each row.lines as line (line.lane)}
			<span class="absolute" style={lineStyle(line)}></span>
		{/each}
		{#if sprouts.length}
			<svg
				class="absolute top-0 left-0"
				style={`width:${gutter}px;height:${DOT_Y}px`}
				viewBox={`0 0 ${gutter} 100`}
				preserveAspectRatio="none"
			>
				{#each sprouts as sprout (sprout.lane)}
					<path
						d={sproutPath(row.lane, sprout.lane)}
						stroke={colour(sprout.tone)}
						stroke-width="2"
						fill="none"
						vector-effect="non-scaling-stroke"
					/>
				{/each}
			</svg>
		{/if}
		<span class="absolute" style={dotStyle(row, hollow)}></span>
	</span>
{/snippet}

{#snippet when(row: VersionRow)}
	<!-- After the first split, a row says which Branch it is on, in its colour. -->
	{#if row.labelled}
		<span class="font-semibold" style={`color:${colour(row.tone)}`}
			>{branchesNamed(row.branchIds)}</span
		> ·
	{/if}
	{aDate(row.version.created_at)} · {handName(row.version)}
{/snippet}

{#snippet versionRow(row: VersionRow)}
	{@const version = row.version}
	{@const now = nowLabel(row)}
	{@const cooked = attemptsByVersion.get(version.version_id) ?? []}
	{@const mine = cooked.filter((cooking) => cooking.mine)}
	{@const theirs = cooked.filter((cooking) => !cooking.mine)}
	<li class="flex">
		{@render gutterOf(row, row.sprouts)}
		<div class="min-w-0 flex-1 border-b border-rule pt-2 pb-3 pl-2">
			<!--
				The whole Version is the thing to open, with a › at its end (#652,
				V2): an older one as a page of its own, the newest as the recipe.
			-->
			<button
				type="button"
				class="flex min-h-12 w-full items-start gap-2 text-left"
				onclick={() => onOpenVersion(opening(row))}
			>
				<span class="grid min-w-0 flex-1 gap-[2px]">
					<span class="text-read text-ink-2">{@render when(row)}</span>
					{#if now}
						<span class="w-fit rounded-sm bg-accent px-2 text-read font-semibold text-on-accent">
							{now}
						</span>
					{/if}
					{#if version.name}
						<span class="font-display text-body font-semibold text-ink">“{version.name}”</span>
					{/if}
					<span class={written(version) ? 'text-body text-ink' : 'text-read text-ink-2'}>
						{whatItWas(version)}
					</span>
					{#if row.adrift}
						<!-- Nothing in this History to sit under, so the row says it (#250). -->
						<span class="text-read text-ink">{m.thread_translates_absent()}</span>
					{/if}
				</span>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[22px] w-[22px] shrink-0 self-center text-accent"
					fill="none"
					stroke="currentColor"
					stroke-width="1.75"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="m9 5 7 7-7 7" />
				</svg>
			</button>
			<!--
				Who cooked it, in words (#652, cookings "A"): your own cookings
				with the last verdict you gave, and everyone else's together, by
				name. Each opens what it stands for (#249).
			-->
			{#if cooked.length || (yours(row) && editing !== row.key)}
				<div class="mt-2 flex flex-wrap gap-3">
					{#if mine.length}
						<button
							type="button"
							class="quiet-button gap-[6px] text-ink"
							onclick={() => onOpenCookings(mine)}
						>
							<ActIcon name="cook" class="h-[18px] w-[18px] shrink-0" />{yourCookings(mine)}
						</button>
					{/if}
					{#if theirs.length}
						<button
							type="button"
							class="quiet-button gap-[6px] text-ink"
							onclick={() => onOpenCookings(theirs)}
						>
							<ActIcon name="cook" class="h-[18px] w-[18px] shrink-0" />{m.thread_cooked_by({
								names: cooksOf(theirs),
							})}
						</button>
					{/if}
					{#if yours(row) && editing !== row.key}
						<button
							type="button"
							class="quiet-button gap-[6px] text-ink"
							onclick={() => startNaming(row)}
						>
							<ActIcon name="name" class="h-[18px] w-[18px] shrink-0" />
							{version.name ? m.thread_rename() : m.thread_name_it()}
						</button>
					{/if}
				</div>
			{/if}
			{#if editing === row.key}
				<form
					class="mt-3 grid gap-3"
					onsubmit={(event) => {
						event.preventDefault();
						void rename(row, draft);
					}}
				>
					<label class="grid gap-1 text-body text-ink">
						{m.thread_name_label()}
						<input
							class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
							bind:value={draft}
							placeholder={m.thread_name_placeholder()}
							{@attach (node) => node.focus()}
						/>
					</label>
					{#if renameError}
						<p class="text-body text-accent" role="alert"><Refusal words={renameError} /></p>
					{/if}
					<div class="flex flex-wrap gap-3">
						<button class="main-button" disabled={saving}>{m.thread_name_save()}</button>
						<button
							type="button"
							class="second-button"
							disabled={saving}
							onclick={() => (editing = undefined)}
						>
							{m.thread_name_keep()}
						</button>
						{#if version.name}
							<button
								type="button"
								class="second-button loses"
								disabled={saving}
								onclick={() => rename(row, null)}
							>
								{m.thread_name_remove()}
							</button>
						{/if}
					</div>
					<!--
						Shown, never a field: the why of an edit is written when it is
						true, and the caption says the fixing is deliberate (#83).
					-->
					<div>
						<p class="text-label text-ink-2 uppercase">{m.thread_what_changed()}</p>
						<p class="text-read text-ink">{whatItWas(version)}</p>
						<p class="text-label text-ink-2">{m.thread_what_changed_fixed()}</p>
					</div>
				</form>
			{/if}
			{#if lastRename?.key === row.key}
				<p role="status" class="mt-2 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
					{lastRename.cleared ? m.thread_name_removed() : m.thread_named()}
				</p>
			{/if}
		</div>
	</li>
{/snippet}

<ul>
	{#each items as item (item.kind === 'run' ? item.key : item.row.key)}
		{#if item.kind === 'run' && !expanded.has(item.key)}
			{@const first = item.rows[0]}
			{@const last = item.rows[item.rows.length - 1]}
			<li class="flex">
				{@render gutterOf({ ...first, lines: foldedLines(item.rows) }, [], true)}
				<div class="flex min-h-12 min-w-0 flex-1 items-center gap-3 border-b border-rule py-2 pl-2">
					<!-- Newest first, so the run reads from its oldest date to its newest. -->
					<span class="min-w-0 flex-1 text-read text-ink-2">
						{m.thread_quiet_run({
							count: item.rows.length,
							from: aDate(last.version.created_at),
							to: aDate(first.version.created_at),
						})}
					</span>
					<button type="button" class="quiet-button shrink-0" onclick={() => toggle(item.key)}>
						{m.thread_show()}
					</button>
				</div>
			</li>
		{:else if item.kind === 'run'}
			{#each item.rows as row (row.key)}
				{@render versionRow(row)}
			{/each}
			<li class="flex">
				<span class="relative shrink-0" style={`width:${gutter}px`} aria-hidden="true">
					{#each linesBelow(item.rows[item.rows.length - 1]) as line (line.lane)}
						<span class="absolute" style={lineStyle(line)}></span>
					{/each}
				</span>
				<div class="py-2 pl-2">
					<button type="button" class="quiet-button" onclick={() => toggle(item.key)}>
						{m.thread_collapse()}
					</button>
				</div>
			</li>
		{:else if item.row.kind === 'version'}
			{@render versionRow(item.row)}
		{:else}
			<!--
				A Branch with no Version past this point (#250): its name, in its
				colour, above the last Version it holds, the History reading newest
				first (#652). Either it starts there and
				nothing was saved on it, or it is the recipe, left as it was while a
				variation carried on. The dot is hollow, as it is for anything on
				this line that is not one saved Version.
			-->
			{@const starting = item.row}
			<li class="flex">
				{@render gutterOf(starting, [], true)}
				<div class="min-w-0 flex-1 border-b border-rule py-2 pl-2">
					<p class="text-read font-semibold" style={`color:${colour(starting.tone)}`}>
						{branchesNamed([starting.branchId])}
					</p>
					<p class="text-read text-ink-2">
						{starting.stopped ? m.thread_nothing_since() : m.thread_starts_here()}
					</p>
				</div>
			</li>
		{/if}
	{/each}
</ul>
