<!--
	The Import Report for a Crouton library (#69): option A, "one page, three
	parts", as chosen on 2026-09-19 (the decision is a comment on #69).

	It owes three different kinds of thing at once, and each is drawn as what it
	is rather than flattened into one list: what is **waiting for a tap** (work,
	on a card with the one indigo button), what **could not be read** (a loss,
	with the red edge and the file named), and what **arrived** (a fact — a
	count, then the list behind a disclosure you open if you want to).

	The Report is the Job's own result, read through `get_job` (ADR 0025,
	ADR 0032), so it is still here after the screen closes and an agent asking
	how the import went reads the same thing. While the Job runs, the page is
	already here: the bar fills and the count climbs. The pairs arrive only at
	the end, because a pair needs both of its halves in.

	Relating is ordinary `set_related_recipe`, one call per pair whose answer
	changed — a pair unticked after "Change" is unlinked again, so the card
	never says less was linked than was. Keeping a pair apart is not an
	Operation — it is simply the absence of a link — so *that the card was
	answered* is remembered in this browser, and on another device the pairs
	are offered again, harmlessly.

	**On the wide layout it is a card beside the list of imports** (#200, ADR
	0044), as an open Food is beside Foods, and what is a white card on the
	phone's page is set on the page's ground there, since white on a white card
	is no card. Nothing else about it changes: the long list of what arrived
	stays behind its disclosure. A Report that watches its import end says so
	to that list, whose row for it was read while it ran.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { untrack } from 'svelte';
	import { useKamosu } from '$lib/kamosu';
	import { useRoom } from '$lib/room.svelte';
	import { OperationError } from '$lib/api/client';
	import { waitForJob } from '$lib/api/job';
	import type { GetJobOutput, ImportCroutonOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { useEnded } from '../ended.svelte';
	import AgentWork from '$lib/agent/AgentWork.svelte';
	import SendAgain from './SendAgain.svelte';
	import { AGENTS_OWN, working } from '$lib/agent/agent';

	interface Props {
		jobId: string;
		/** How long to wait between reads of a running Job. Tests shorten it. */
		every?: number;
	}

	let { jobId, every = 700 }: Props = $props();

	const kamosu = useKamosu();
	const room = useRoom();
	const ended = useEnded();

	/** A card, where the list of imports is beside this page: on the wide layout. */
	const card = $derived(room.wide);
	/** What the phone draws as a card on its page, which inside a card is the page's ground. */
	const panel = $derived(card ? 'bg-ground' : 'bg-card');

	type Report = ImportCroutonOutput;
	type Pair = Report['related_candidates'][number];

	let job = $state<GetJobOutput | undefined>(undefined);
	let unreachable = $state<string | undefined>(undefined);
	/** Moved when an agent's Job this Report names ends, to read the Report again (#277). */
	let again = $state(0);

	// Read the Job until it ends — its row is the truth about it (ADR 0032).
	// Leaving the page stops the reading, never the import.
	$effect(() => {
		const id = jobId;
		void again;
		let current = true;
		let waited = false;
		waitForJob(kamosu, id, {
			every,
			giveUpAfter: 6 * 60 * 60 * 1000,
			whileWaiting: (read) => {
				waited = true;
				if (current) job = read;
			},
			stopped: () => !current,
			answersWhenFailed: holdsAReport,
			answersWhenCancelled: (read) => read.operation === AGENTS_OWN,
		})
			.then((read) => {
				if (!current) return;
				job = read;
			})
			.catch((error: unknown) => {
				if (!(error instanceof Error)) throw error;
				if (!current) return;
				unreachable = error.message;
			})
			.finally(() => {
				// The list beside this read the import while it ran.
				if (current && waited) ended?.say();
			});
		return () => {
			current = false;
		};
	});

	/** A failed Job that is an agent's own Import still holds its Report (#269). */
	function holdsAReport(read: GetJobOutput) {
		return Boolean((read.result as Report | null)?.agent_jobs?.length);
	}

	/**
	 * The Report, once there is one. A Job an agent read a file in is its own
	 * Import (#269), so it can end as failed and still hold a Report naming
	 * what could not be read; that is drawn as a Report, not as a stop.
	 */
	const report = $derived.by(() => {
		const result = job?.result as Report | undefined;
		if (job?.status === 'completed') return result;
		if (job?.status === 'failed' && holdsAReport(job)) return result;
		return undefined;
	});
	const running = $derived(job !== undefined && ['queued', 'running'].includes(job.status));
	/**
	 * This Report is an agent's own Import (#269, #272). Its Job carries the
	 * Report from the start, naming what was handed over, so the page is
	 * titled for it while the agent waits or reads.
	 */
	const agentsOwn = $derived(job?.operation === AGENTS_OWN);
	/** What is drawn from: the Report once there is one, and an agent's own before. */
	const drawn = $derived(report ?? (agentsOwn ? (job?.result as Report | null) : undefined));
	const done = $derived(job?.progress.done ?? 0);
	const total = $derived(job?.progress.total ?? 0);

	const arrived = $derived(drawn?.arrived ?? []);
	const pairs = $derived(report?.related_candidates ?? []);
	const photos = $derived(arrived.filter((row) => row.main_photo).length);
	/** Landed bare and left so: an agent's recipe or one kept as it was is said apart. */
	const bare = $derived(
		arrived.filter(
			(row) =>
				row.bare &&
				!madeBranches.includes(row.branch_id) &&
				!keptAsItWas.includes(row) &&
				!atWorkOn.includes(row.branch_id),
		),
	);
	const siteIcons = $derived(
		(report?.left_out ?? [])
			.filter((row) => row.what === 'site_icon')
			.reduce((sum, row) => sum + row.count, 0),
	);
	/** Each distinct site icon once: nine Instagram recipes carry one icon. */
	const icons = $derived([
		...new Set(
			(report?.left_out ?? [])
				.map((row) => (row.what === 'site_icon' ? row.icon : undefined))
				.filter((icon): icon is string => Boolean(icon)),
		),
	]);
	const extraPhotos = $derived(
		(report?.left_out ?? []).filter((row) => row.what === 'extra_photos'),
	);
	const brokenPhotos = $derived(
		(report?.left_out ?? []).filter((row) => row.what === 'unreadable_photos'),
	);

	// --- The pairs -------------------------------------------------------------

	const pairKey = (pair: Pair) => pair.recipes.map((side) => side.branch_id).join('+');
	const memory = $derived(`kamosu.import-report.${jobId}`);

	/** The pairs this page has linked, as far as it knows, and what is ticked now. */
	let linked = $state<string[]>([]);
	let ticked = $state<string[]>([]);
	let settled = $state(false);
	let relating = $state(false);
	let relateError = $state<string | undefined>(undefined);

	$effect(() => {
		try {
			const kept = JSON.parse(localStorage.getItem(memory) ?? 'null') as {
				related: string[];
			} | null;
			if (kept) {
				linked = kept.related;
				ticked = kept.related;
				settled = true;
			}
		} catch {
			// A private window or blocked storage: the pairs are simply offered again.
		}
	});

	function remember() {
		try {
			localStorage.setItem(memory, JSON.stringify({ related: linked }));
		} catch {
			// As above: forgetting that the card was answered costs a second look.
		}
	}

	function tick(pair: Pair, on: boolean) {
		const key = pairKey(pair);
		ticked = on ? [...ticked, key] : ticked.filter((k) => k !== key);
	}

	/** Make the links match the ticks: link what was ticked, unlink what no longer is. */
	async function answer(wanted: string[]) {
		relating = true;
		relateError = undefined;
		try {
			for (const pair of pairs) {
				const key = pairKey(pair);
				const want = wanted.includes(key);
				if (want === linked.includes(key)) continue;
				await kamosu.setRelatedRecipe({
					branch_id: pair.recipes[0].branch_id,
					related_branch_id: pair.recipes[1].branch_id,
					related: want,
				});
				linked = want ? [...linked, key] : linked.filter((k) => k !== key);
			}
			ticked = wanted;
			settled = true;
			remember();
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			relateError = error.message;
		} finally {
			relating = false;
		}
	}

	const linkedPairs = $derived(pairs.filter((pair) => linked.includes(pairKey(pair))));

	/** A pair is named by its shorter title: "Pandebonos", not "(Version 2)". */
	const pairName = (pair: Pair) =>
		pair.recipes
			.map((side) => side.title)
			.reduce((shorter, title) => (title.length < shorter.length ? title : shorter));

	const why = (pair: Pair) =>
		pair.shared
			.map((shared) => (shared === 'name' ? m.report_pair_same_name() : m.report_pair_same_page()))
			.join(' · ');

	const thumb = (photo: string | null | undefined) =>
		photo ? `/api/photographs/${photo}/card` : undefined;

	/**
	 * Whose import this is, and so what to call it.
	 *
	 * The Report's `source_kind` says, once the work has finished. Until then
	 * the Job's Operation does, because the Job says it the moment it is first
	 * read and this page is already on screen while the bar fills. A recipe
	 * file reaches this page too, since #93 (one recipe file ends on the
	 * recipe, and this is where *How it went* leads); before that it was only
	 * ever a Crouton library, and calling a friend's recipe *your Crouton
	 * library* was a plain lie about what you were looking at. A web link no
	 * agent read was told the same lie until #308, and the generic `import`
	 * can name `web` too, which only the Report shows.
	 *
	 * Everything else here reads correctly for any source. The one phrase that
	 * names Crouton, `report_changed`, belongs to an `offered` row, and a
	 * Bundle never produces one.
	 */
	const SOURCE_OF: Record<string, string> = { import_bundle: 'bundle', import_web_link: 'web' };
	const source = $derived(drawn?.source_kind ?? (job ? SOURCE_OF[job.operation] : undefined));
	const TITLED: Record<string, () => string> = {
		bundle: () => m.report_title_file(),
		web: () => m.report_title_link(),
	};

	// --- An agent's work (#276), option B as chosen on 2026-10-15 ---------------
	//
	// What an agent did with an arrival is read off the Report's `agent_jobs`,
	// which the server brings up to date on every reading (#262). It is said
	// here and never on the recipe. What the cook should look at goes under
	// Needs you; Arrived stays the record of what came in, with the bare
	// landing and the agent's work as one row. A Report naming no agent's Job
	// is drawn exactly as before.

	type AgentJob = NonNullable<Report['agent_jobs']>[number];

	const agentJobs = $derived(drawn?.agent_jobs ?? []);

	// --- An agent's Job that waits or is read (#277), option B ------------------
	//
	// The tracker for each, at the top of the Report. A Job is watched from the
	// first reading that finds it waiting or read, and stays watched after it
	// ends, so a tracker that has just been stopped goes on saying so.

	const atWork = $derived(agentJobs.filter((agent) => working(agent.status)));
	let watched = $state<string[]>([]);
	/**
	 * Whose each watched Job is. A link a Co-author brought in may name the
	 * Job the other Co-author's link already waits on, and only the Person
	 * who asked stops it (#267), so the recipe is asked, as its page asks. A
	 * Job with no recipe yet is an agent's own Import, which only the Person
	 * who asked reads.
	 */
	let yours = $state<Record<string, boolean>>({});
	$effect(() => {
		const before = untrack(() => watched);
		const fresh = atWork.filter((agent) => !before.includes(agent.job_id));
		if (!fresh.length) return;
		watched = [...before, ...fresh.map((agent) => agent.job_id)];
		for (const agent of fresh) {
			const branch = agent.branch_id;
			if (branch === null) {
				yours[agent.job_id] = true;
				continue;
			}
			kamosu
				.getRecipeJob({ branch_id: branch })
				.then((read) => {
					yours[agent.job_id] = read.job?.id === agent.job_id && read.job.yours;
				})
				.catch((error: unknown) => {
					if (!(error instanceof OperationError)) throw error;
				});
		}
	});
	/** The recipes an agent waits or reads on: they have not come as bare, they are waited on. */
	const atWorkOn = $derived(atWork.map((agent) => agent.branch_id));
	const waitedOn = $derived(arrived.filter((row) => atWorkOn.includes(row.branch_id)));
	const hasRecipe = (id: string) =>
		Boolean(agentJobs.find((agent) => agent.job_id === id)?.branch_id);

	/** An agent's own Import ends with the Job this page reads, which is followed already. */
	function agentEnded(id: string) {
		if (id !== jobId) again += 1;
	}
	/** The Jobs whose agent made its recipe. Only these have notes to look over. */
	const made = $derived(agentJobs.filter((agent) => agent.made_by_agent && agent.branch_id));
	const madeBranches = $derived(made.map((agent) => agent.branch_id));
	const failed = $derived(agentJobs.filter((agent) => agent.status === 'failed'));

	/**
	 * The recipe an agent's Job is on, by its name. A link brought in again
	 * names the Job already standing on a recipe it was only offered (#261).
	 */
	const titleOf = (branchId: string | null) =>
		[...arrived, ...(report?.offered ?? [])].find((row) => row.branch_id === branchId)?.title ??
		m.report_unreadable_unnamed();

	/** What the agent was handed, which names a Report that has one. */
	const handed = $derived(agentJobs.length ? drawn?.source_kind : undefined);

	/** Each thing an agent is handed: what its Report is called, and what a failed one is. */
	const HANDED: Record<string, { title: () => string; unnamed: () => string }> = {
		web: { title: () => m.report_title_link(), unnamed: () => m.report_unreadable_unnamed() },
		picture: {
			title: () => m.report_title_picture(),
			unnamed: () => m.report_unreadable_picture(),
		},
		pdf: { title: () => m.report_title_pdf(), unnamed: () => m.report_unreadable_pdf() },
		pasted_text: {
			title: () => m.report_title_pasted_text(),
			unnamed: () => m.report_unreadable_pasted_text(),
		},
		read_again: {
			title: () => m.report_title_again({ title: titleOf(agentJobs[0]?.branch_id ?? null) }),
			unnamed: () => m.report_unreadable_unnamed(),
		},
	};
	const kind = $derived(handed ? HANDED[handed] : undefined);

	const title = $derived(
		kind ? kind.title() : ((source && TITLED[source]?.()) ?? m.report_title()),
	);

	/** A recipe read again whose agent failed is left as it was before. */
	const keptAsItWas = $derived(
		arrived.filter(
			(row) =>
				row.status === 'read_again' && failed.some((agent) => agent.branch_id === row.branch_id),
		),
	);

	/**
	 * The recipes an agent thinks may be the same dish, by their names now.
	 * One that cannot be opened (deleted, or in a Cookbook the cook has left)
	 * is left out, since there is nothing to do with it.
	 */
	let duplicates = $state<{ branch_id: string; title: string }[]>([]);
	/** Whether they have been read, so "Nothing needs you" is not said before. */
	let duplicatesRead = $state(false);
	$effect(() => {
		const ids = [...new Set(made.flatMap((agent) => agent.agent_report?.likely_duplicates ?? []))];
		let current = true;
		duplicatesRead = false;
		Promise.all(
			ids.map((branch_id) =>
				kamosu
					.getRecipe({ branch_id })
					.then((recipe) => ({ branch_id, title: recipe.versions.at(-1)?.content.title ?? '' }))
					.catch((error: unknown) => {
						if (!(error instanceof OperationError)) throw error;
						return undefined;
					}),
			),
		).then((read) => {
			if (!current) return;
			duplicates = read.filter((found) => found !== undefined);
			duplicatesRead = true;
		});
		return () => {
			current = false;
		};
	});

	/** The lists under one recipe's card, each only where it has entries. */
	const notesOf = (agent: AgentJob) =>
		[
			{ label: m.report_agent_left_undone(), items: agent.agent_report?.left_undone ?? [] },
			{ label: m.report_agent_unsure(), items: agent.agent_report?.unsure ?? [] },
			{ label: m.report_agent_unused_foods(), items: agent.agent_report?.unused_foods ?? [] },
		].filter((list) => list.items.length);

	const writtenBy = $derived(arrived.filter((row) => madeBranches.includes(row.branch_id)));
	const writtenNew = $derived(writtenBy.filter((row) => row.status !== 'unchanged'));
	const writtenBefore = $derived(writtenBy.filter((row) => row.status === 'unchanged'));

	const toLookAt = $derived(made.filter((agent) => notesOf(agent).length));

	/** How many entries of a list show before "Show N more". */
	const SHOWN = 3;

	/** A failure is the agent's own words only where it said the thing holds no recipe. */
	const failure = (loss: Report['unreadable'][number]) => {
		const agent = failed.find((one) => (one.branch_id ?? undefined) === loss.branch_id);
		if (!agent) return loss.reason;
		return agent.could_not_be_read
			? m.report_agent_said({ reason: loss.reason })
			: m.report_agent_failed({ reason: loss.reason });
	};

	/**
	 * A photo, a PDF or pasted text the agent failed on before it made a
	 * recipe, which can be sent again from what Kamosu kept (#273, #279).
	 */
	const FILES = ['picture', 'pdf', 'pasted_text'] as const;
	const failedOnAFile = $derived.by(() => {
		const agent = agentJobs[0];
		if (job?.status !== 'failed' || !agent || agent.branch_id !== null) return undefined;
		const kind = FILES.find((one) => one === handed);
		if (!kind) return undefined;
		return { kind, gone: agent.original_gone === true, endedAt: job.updated_at };
	});

	/** What a failed file is called, where no recipe arrived to name it. */
	const unnamed = $derived(kind ? kind.unnamed() : m.report_unreadable_unnamed());

	/** Nothing waits for the cook: no pair, no change held, nothing an agent noted. */
	const nothingNeedsYou = $derived(
		pairs.length === 0 &&
			report?.offered.length === 0 &&
			duplicatesRead &&
			duplicates.length === 0 &&
			toLookAt.length === 0,
	);

	/** Under a row behind Show it: what became of it. */
	const rowNote = (row: Report['arrived'][number]) => {
		if (keptAsItWas.includes(row)) return m.report_as_it_was_row();
		return [
			madeBranches.includes(row.branch_id)
				? m.report_written_by_agent_row()
				: row.bare
					? m.report_bare_recipe()
					: '',
			row.status === 'unchanged' ? m.report_unchanged() : '',
		]
			.filter(Boolean)
			.join(' · ');
	};

	/**
	 * A phrase with a link where `{title}` stands, so the sentence is
	 * translated whole and the link still sits inside it.
	 */
	const around = (phrase: (args: { title: string }) => string) =>
		phrase({ title: '\u0000' }).split('\u0000') as [string, string];
</script>

{#snippet withLink(phrase: (args: { title: string }) => string, title: string, branchId: string)}
	{@const [before, after] = around(phrase)}
	{before}<a href="/recipes/{branchId}" class="text-accent underline">{title}</a>{after}
{/snippet}

{#snippet notes(items: string[])}
	<ul class="list-disc pl-4 text-read">
		{#each items as item, index (index)}
			<li class="mt-1">{item}</li>
		{/each}
	</ul>
{/snippet}

<Screen {card} {title}>
	{#each watched as id (id)}
		<AgentWork
			jobId={id}
			mine={yours[id] ?? false}
			saying="report"
			hasRecipe={hasRecipe(id)}
			{panel}
			gutter={false}
			ended={() => agentEnded(id)}
		/>
	{/each}
	{#if unreachable}
		<p class={['mt-4 border-l-3 border-support px-3 py-2 text-body', panel]} role="alert">
			{unreachable}
		</p>
	{:else if agentsOwn && (running || job?.status === 'cancelled')}
		{#if running}
			<Section heading={m.report_needs_you()}>
				<p class="text-read text-ink-2">{m.report_agent_nothing_yet()}</p>
			</Section>
		{:else if !watched.includes(jobId)}
			<!-- Stopped before this page opened; one stopped here says so in its tracker. -->
			<p class={['mt-4 border-l-3 border-rule px-3 py-2 text-read text-ink-2', panel]}>
				{hasRecipe(jobId) ? m.agent_stopped() : m.agent_stopped_nothing()}
			</p>
		{/if}
	{:else if job === undefined || job.status === 'queued'}
		<p class="mt-4 text-body text-ink-2">{m.report_waiting()}</p>
	{:else if running}
		<p class="mt-4 text-body text-ink-2">{m.report_running({ done, total })}</p>
		<div
			class="mt-3 h-1 overflow-hidden rounded-sm bg-ground-2"
			role="progressbar"
			aria-valuemin={0}
			aria-valuemax={total}
			aria-valuenow={done}
		>
			<div class="h-full bg-accent" style="width: {total ? (done / total) * 100 : 0}%"></div>
		</div>

		<Section heading={m.report_needs_you()}>
			<p class={['rounded-sm border border-rule px-3 py-3 text-read text-ink-2', panel]}>
				{m.report_needs_you_waiting()}
			</p>
		</Section>
	{:else if !report}
		<p class={['mt-4 border-l-3 border-support px-3 py-2 text-body', panel]} role="alert">
			{m.report_stopped({ reason: job.error ?? job.status })}
		</p>
	{:else}
		<p class="mt-4 text-body text-ink-2">
			{#if keptAsItWas.length}
				{m.report_lede_kept({ title: keptAsItWas[0].title })}
			{:else if handed && arrived.length === 0 && made.length === 0}
				{m.report_lede_nothing()}
			{:else}
				{m.report_lede({ count: arrived.length })}
			{/if}
			{#if report.unreadable.length}
				{m.report_lede_unreadable({ count: report.unreadable.length })}
			{/if}
			{#if pairs.length}
				{m.report_lede_pairs({ count: pairs.length })}
			{/if}
		</p>

		<Section heading={m.report_needs_you()}>
			{#if nothingNeedsYou}
				<p class="text-read text-ink-2">{m.report_needs_you_none()}</p>
			{/if}

			{#each duplicates as duplicate (duplicate.branch_id)}
				<p class={['mb-2 border-l-3 border-accent px-3 py-2 text-read', panel]}>
					{@render withLink(m.report_agent_duplicate, duplicate.title, duplicate.branch_id)}
				</p>
			{/each}

			{#each toLookAt as agent (agent.job_id)}
				<div class={['mb-2 rounded-sm border border-rule px-3 pb-3', panel]}>
					<p class="pt-3 text-read">
						{@render withLink(m.report_agent_look, titleOf(agent.branch_id), agent.branch_id ?? '')}
					</p>
					{#each notesOf(agent) as list (list.label)}
						<p class="mt-3 text-read font-semibold">{list.label}</p>
						{@render notes(list.items.slice(0, SHOWN))}
						{#if list.items.length > SHOWN}
							<details>
								<summary class="cursor-pointer py-2 text-read font-medium text-accent">
									{m.report_agent_more({ count: list.items.length - SHOWN })}
								</summary>
								{@render notes(list.items.slice(SHOWN))}
							</details>
						{/if}
					{/each}
				</div>
			{/each}

			{#if pairs.length}
				<div class={['rounded-sm border border-rule px-3 pb-3', panel]}>
					{#if settled}
						<p class="pt-3 text-read">
							{#if linkedPairs.length === pairs.length}
								{m.report_settled_all({ titles: linkedPairs.map(pairName).join(', ') })}
							{:else if linkedPairs.length}
								{m.report_settled({
									titles: linkedPairs.map(pairName).join(', '),
									count: pairs.length - linkedPairs.length,
								})}
							{:else}
								{m.report_settled_none({ count: pairs.length })}
							{/if}
							<button
								type="button"
								class="ml-1 text-accent underline"
								onclick={() => (settled = false)}
							>
								{m.report_settled_change()}
							</button>
						</p>
					{:else}
						<p class="pt-3 pb-1 text-read text-ink-2">{m.report_pairs_blurb()}</p>
						<ul>
							{#each pairs as pair (pairKey(pair))}
								<li class="border-b border-rule last:border-b-0">
									<label class="flex cursor-pointer items-start gap-3 py-2">
										<input
											type="checkbox"
											class="mt-1 size-6 shrink-0 accent-accent"
											checked={ticked.includes(pairKey(pair))}
											onchange={(event) => tick(pair, event.currentTarget.checked)}
										/>
										<span class="flex-1">
											<span class="block text-body leading-tight">
												{pair.recipes[0].title}
												<span class="text-ink-2">&amp;</span>
												{pair.recipes[1].title}
											</span>
											<span class="block text-read text-ink-2">
												{why(pair)} · {m.report_pair_ingredients({
													first: pair.recipes[0].ingredients,
													second: pair.recipes[1].ingredients,
												})}
											</span>
										</span>
										<span class="flex shrink-0 gap-1">
											{#each pair.recipes as side (side.branch_id)}
												{#if thumb(side.main_photo)}
													<img
														src={thumb(side.main_photo)}
														alt=""
														class="size-8 rounded-sm object-cover"
													/>
												{:else}
													<span class="size-8 rounded-sm bg-ground-2"></span>
												{/if}
											{/each}
										</span>
									</label>
								</li>
							{/each}
						</ul>
						{#if relateError}
							<p class="text-read text-support" role="alert">{relateError}</p>
						{/if}
						<div class="mt-3 flex items-center gap-2">
							<button
								type="button"
								class="min-h-12 rounded-sm bg-accent px-4 font-semibold text-on-accent disabled:opacity-40"
								disabled={ticked.length === 0 || relating}
								onclick={() => answer(ticked)}
							>
								{ticked.length ? m.report_relate({ count: ticked.length }) : m.report_relate_none()}
							</button>
							<button
								type="button"
								class="min-h-12 px-2 text-ink-2"
								disabled={relating}
								onclick={() => answer([])}
							>
								{m.report_relate_dismiss()}
							</button>
						</div>
					{/if}
				</div>
			{/if}

			{#each report.offered as changed (changed.branch_id)}
				<p class="mt-2 text-read">
					<a href="/recipes/{changed.branch_id}" class="text-accent underline"
						>{m.report_changed({ title: changed.title })}</a
					>
				</p>
			{/each}
		</Section>

		<Section heading={m.report_unreadable()}>
			{#if report.unreadable.length === 0}
				<p class="text-read text-ink-2">{m.report_unreadable_none()}</p>
			{:else}
				<ul class="grid gap-2">
					{#each report.unreadable as loss, index (index)}
						<li class={['border-l-3 border-support px-3 py-2', panel]}>
							<p class="text-body font-semibold">
								{loss.name ?? (handed ? unnamed : (loss.foreign_id ?? unnamed))}
							</p>
							<p class="text-read text-ink-2">{failure(loss)}</p>
						</li>
					{/each}
				</ul>
				{#if failedOnAFile}
					<SendAgain
						{jobId}
						kind={failedOnAFile.kind}
						gone={failedOnAFile.gone}
						endedAt={failedOnAFile.endedAt}
					/>
				{/if}
			{/if}
		</Section>

		<Section heading={m.report_arrived()}>
			<p class="text-body">{m.report_arrived_count({ recipes: arrived.length, photos })}</p>
			{#if bare.length}
				<p class="mt-2 text-read">
					{m.report_bare({ count: bare.length, titles: bare.map((row) => row.title).join(', ') })}
				</p>
			{/if}
			{#each waitedOn as row (row.branch_id)}
				<p class="mt-2 text-read">{m.report_waiting_for_agent({ title: row.title })}</p>
			{/each}
			{#if writtenNew.length}
				<p class="mt-2 text-read">
					{m.report_written_by_agent({ titles: writtenNew.map((row) => row.title).join(', ') })}
				</p>
			{/if}
			{#if writtenBefore.length}
				<p class="mt-2 text-read">
					{m.report_written_by_agent_unchanged({
						titles: writtenBefore.map((row) => row.title).join(', '),
					})}
				</p>
			{/if}
			{#each keptAsItWas as row (row.branch_id)}
				<p class="mt-2 text-read">{m.report_as_it_was({ title: row.title })}</p>
			{/each}
			{#if siteIcons}
				<p class="mt-2 text-read text-ink-2">{m.report_site_icons({ count: siteIcons })}</p>
				{#if icons.length}
					<p class="mt-1 flex flex-wrap gap-1" aria-hidden="true">
						{#each icons as icon (icon)}
							<img src={icon} alt="" class="size-4 rounded-sm opacity-70 grayscale" />
						{/each}
					</p>
				{/if}
			{/if}
			{#each extraPhotos as extra (extra.branch_id)}
				<p class="mt-2 text-read text-ink-2">
					{extra.count === 1
						? m.report_extra_photo_one({ title: extra.title })
						: m.report_extra_photo_many({ title: extra.title, count: extra.count })}
				</p>
			{/each}
			{#each brokenPhotos as broken (broken.branch_id)}
				<p class="mt-2 text-read text-ink-2">
					{m.report_unreadable_photos({ title: broken.title })}
				</p>
			{/each}

			{#if arrived.length}
				<details class="mt-3">
					<summary class="cursor-pointer py-2 font-medium text-accent">
						{m.report_show_all({ count: arrived.length })}
					</summary>
					<ul>
						{#each arrived as row (row.branch_id)}
							<li class="flex items-center gap-3 border-b border-rule py-2 last:border-b-0">
								{#if thumb(row.main_photo)}
									<img
										src={thumb(row.main_photo)}
										alt=""
										loading="lazy"
										class="size-12 shrink-0 rounded-sm object-cover"
									/>
								{:else}
									<span
										class="grid size-12 shrink-0 place-items-center rounded-sm bg-ground-2 text-label text-ink-2"
										>{m.report_no_photo()}</span
									>
								{/if}
								<span>
									<a href="/recipes/{row.branch_id}" class="block text-body leading-tight"
										>{row.title}</a
									>
									{#if rowNote(row)}
										<span class="block text-read text-ink-2">{rowNote(row)}</span>
									{/if}
								</span>
							</li>
						{/each}
					</ul>
				</details>
			{/if}
		</Section>
	{/if}
</Screen>
