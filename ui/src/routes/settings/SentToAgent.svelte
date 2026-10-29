<!--
	Everything you sent to Kamosu's AI agent, and how each one ended (#463):
	option A of 28 October 2026, a page of its own under Settings › Add
	recipes (the decision is a comment on #463). The prototype is
	`.dev/prototypes/463-agent-list/`, its `a-list-phone.html` and
	`a-list-macbook.html`.

	**Why a list of its own, beside Brought in.** Brought in lists the same
	Jobs under the source each came in by, and a row there is a date and a
	count. A reel shared from a phone that failed two minutes later told
	nobody, and finding why took a look at the server. Here every send is one
	row, newest first under its day: what was sent, as far as Kamosu still
	holds it, and how it ended, a failure in the agent's own words with what
	to do next on the row itself.

	**What to do next is the server's to say** (`next`), as the Operations
	that do it would answer: read the recipe again, send the same file again
	while Kamosu keeps it, send a link again, or paste the recipe's text. Each
	lands where the + lands it: on the new Job's Report, which follows it.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import { aClock } from '$lib/dates';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Refusal from '$lib/Refusal.svelte';
	import AgentTag from '$lib/agent/AgentTag.svelte';
	import { EVERY_MS } from '$lib/agent/agent';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { useAdding } from '$lib/adding/adding.svelte';
	import PasteForm from '$lib/adding/PasteForm.svelte';
	import FilePicker from '$lib/adding/FilePicker.svelte';
	import ToCheck from '$lib/adding/ToCheck.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import {
		byDay,
		kindOf,
		shortLink,
		shows,
		whatWasSent,
		working,
		type Next,
		type Sent,
		type Showing,
	} from '$lib/sent-to-agent';

	interface Props {
		/** How often to read again while the agent waits on or reads a row. */
		every?: number;
	}

	let { every = EVERY_MS }: Props = $props();

	const kamosu = useKamosu();
	const adding = useAdding();
	const online = new Online();

	let sent = $state<Sent[] | undefined>(undefined);
	/** The Core's own refusal, where one arrived. Its words, not a paraphrase. */
	let refused = $state<string | undefined>(undefined);
	/** Moved while the agent waits on or reads a row, to read the list again. */
	let again = $state(0);

	$effect(() => {
		void again;
		let current = true;
		let timer: ReturnType<typeof setTimeout> | undefined;
		kamosu
			.listAgentJobs()
			.then((answer) => {
				if (!current) return;
				sent = answer.jobs;
				if (answer.jobs.some(working)) timer = setTimeout(() => (again += 1), every);
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (!current) return;
				// A second reading that fails leaves the list that is drawn,
				// and tries again while a row on it waits or is read.
				if (sent === undefined) refused = error.message || m.sent_unreachable();
				else if (sent.some(working)) timer = setTimeout(() => (again += 1), every);
			});
		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	let showing = $state<Showing>('all');
	const counts = $derived({
		all: sent?.length ?? 0,
		failed: sent?.filter((row) => shows(row, 'failed')).length ?? 0,
		working: sent?.filter(working).length ?? 0,
	});
	const days = $derived(byDay((sent ?? []).filter((row) => shows(row, showing))));

	/** The row a button was pressed on, while what it asked is under way. */
	let acting = $state<string | undefined>(undefined);
	/** Why what was asked on a row was refused, under that row. */
	let said = $state<{ jobId: string; words: string } | undefined>(undefined);
	/** The row whose recipe is being pasted instead, under it. */
	let pasting = $state<string | undefined>(undefined);

	async function act(row: Sent, ask: () => Promise<string | undefined>) {
		acting = row.job_id;
		said = undefined;
		try {
			const at = await ask();
			if (at) await goto(at);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = { jobId: row.job_id, words: error.message };
		} finally {
			acting = undefined;
		}
	}

	const readRecipeAgain = (row: Sent) =>
		act(row, async () => {
			const asked = await kamosu.readRecipeAgain({ branch_id: row.recipe?.branch_id ?? '' });
			return `/imports/${asked.job_id}`;
		});

	const sendAgain = (row: Sent) =>
		act(row, async () => {
			const asked = await kamosu.readJobAgain({ job_id: row.job_id });
			if (asked.job_id) return `/imports/${asked.job_id}`;
			return asked.branch_id ? `/recipes/${asked.branch_id}` : undefined;
		});

	/** A link sent again goes as the + sends one, and lands where it does. */
	const sendLinkAgain = (row: Sent) =>
		act(row, async () => {
			await adding.send({ link: row.link ?? '' });
			if (adding.failed) said = { jobId: row.job_id, words: adding.failed };
			return undefined;
		});

	/** The pickers another photo or PDF is chosen with, as the + chooses one. */
	let photoPicker = $state<FilePicker | undefined>(undefined);
	let pdfPicker = $state<FilePicker | undefined>(undefined);

	/** Each thing to do about a failure: what its button says, and what it does. */
	const NEXT: Record<Next, { label: (row: Sent) => string; run: (row: Sent) => void }> = {
		read_recipe_again: { label: () => m.read_again_do(), run: readRecipeAgain },
		send_again: {
			label: (row) =>
				row.kind === 'picture'
					? m.report_again_picture()
					: row.kind === 'pdf'
						? m.report_again_pdf()
						: m.report_again_pasted_text(),
			run: sendAgain,
		},
		send_link_again: { label: () => m.sent_try_again(), run: sendLinkAgain },
		choose_another: {
			label: (row) => (row.kind === 'pdf' ? m.sent_choose_pdf() : m.sent_choose_photo()),
			run: (row) => (row.kind === 'pdf' ? pdfPicker : photoPicker)?.open(),
		},
		paste_instead: {
			label: () => m.sent_paste_instead(),
			run: (row) => {
				pasting = pasting === row.job_id ? undefined : row.job_id;
				// The paste box says what it does by whether there is an agent,
				// which the + reads as it opens and this page has not.
				if (pasting && online.current) void adding.readRunner();
			},
		},
	};

	const BUTTON =
		'min-h-[40px] rounded-sm border border-accent px-3 text-read font-medium disabled:opacity-60';
</script>

{#snippet thumb(row: Sent)}
	<span
		class="relative flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-sm bg-ground-2 text-ink-2"
	>
		{#if row.preview}
			<img src={row.preview} alt="" loading="lazy" class="h-full w-full object-cover" />
			{#if row.pages > 1}
				<span
					class="absolute right-[2px] bottom-[2px] rounded-sm bg-ink px-1 text-label tracking-[0] text-on-accent"
					>{row.pages}</span
				>
			{/if}
		{:else}
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-6 w-6"
				fill="none"
				stroke="currentColor"
				stroke-width="1.6"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				{#if row.read_again}
					<path d="M20 11a8 8 0 1 0-2.3 5.7" /><path d="M20 5v6h-6" />
				{:else if row.kind === 'link'}
					<path d="M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1 1" />
					<path d="M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1-1" />
				{:else if row.kind === 'pdf'}
					<path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4" /><path d="M9.5 13h5M9.5 16.5h5" />
				{:else if row.kind === 'picture'}
					<rect x="3" y="5" width="18" height="14" rx="2" /><circle cx="9" cy="11" r="2" />
					<path d="m21 17-5-5-8 7" />
				{:else}
					<path d="M5 6h14M5 10h14M5 14h10M5 18h7" />
				{/if}
			</svg>
		{/if}
	</span>
{/snippet}

{#snippet recipeLink(words: string, recipe: { branch_id: string; title: string })}
	<span class="mt-1 block text-read text-ink-2"
		>{words}
		<a href="/recipes/{recipe.branch_id}" class="text-accent underline underline-offset-2"
			>{recipe.title}</a
		></span
	>
{/snippet}

{#snippet ending(row: Sent)}
	{#if row.status === 'queued'}
		<span class="mt-1 block text-read text-ink-2"><AgentTag at="waiting" /> {m.sent_waiting()}</span
		>
	{:else if row.status === 'running'}
		<span class="mt-1 block text-read text-ink-2"><AgentTag at="reading" /></span>
	{:else if row.status === 'completed'}
		{#if row.recipe}
			{@render recipeLink(
				`✓ ${row.read_again ? m.sent_read_again_into() : row.fills_in ? m.sent_filled_in() : m.sent_made()}`,
				row.recipe,
			)}
		{:else}
			<span class="mt-1 block text-read text-ink-2">✓ {m.sent_made_gone()}</span>
		{/if}
	{:else if row.status === 'cancelled'}
		{#if row.recipe}
			{@render recipeLink(m.sent_stopped_kept(), row.recipe)}
		{:else}
			<span class="mt-1 block text-read text-ink-2">{m.sent_stopped()}</span>
		{/if}
	{:else}
		<span class="mt-[6px] block border-l-3 border-support bg-card px-3 py-1">
			<span class="block text-read font-semibold text-support">{m.sent_failed()}</span>
			<span class="block text-read break-words text-ink-2">
				{row.reason === null
					? m.sent_failed_no_reason()
					: row.in_agents_words
						? m.report_agent_said({ reason: row.reason })
						: m.report_agent_failed({ reason: row.reason })}
			</span>
		</span>
		{#if row.recipe}
			{@render recipeLink(row.fills_in ? m.sent_stays_as_read() : m.sent_stays(), row.recipe)}
		{/if}
		{#if row.next.length > 0}
			<span class="mt-2 flex flex-wrap gap-2">
				{#each row.next as next (next)}
					{#if next === 'paste_instead'}
						<!-- Opens the paste box under the row, which asks the server
						     itself when its own button is pressed. -->
						<button
							type="button"
							class="{BUTTON} bg-card text-accent"
							aria-expanded={pasting === row.job_id}
							onclick={() => NEXT[next].run(row)}>{NEXT[next].label(row)}</button
						>
					{:else}
						<NeedsServer
							label={NEXT[next].label(row)}
							waiting={m.report_again_waits()}
							disabled={acting !== undefined || adding.working !== null}
							onclick={() => NEXT[next].run(row)}
							shapeClass={BUTTON}
							lookClass="bg-card text-accent"
						/>
					{/if}
				{/each}
			</span>
		{/if}
		{#if said?.jobId === row.job_id}
			<p class="mt-2 text-read text-support" role="alert"><Refusal words={said.words} /></p>
		{/if}
		{#if pasting === row.job_id}
			<div class="mt-3">
				{#if row.link}
					<p class="mb-2 text-read text-ink-2">{m.sent_paste_for({ link: shortLink(row.link) })}</p>
				{/if}
				<PasteForm {adding} />
			</div>
		{/if}
	{/if}
{/snippet}

<p class="text-read text-ink-2">{m.sent_blurb()}</p>

{#if refused}
	<p class="mt-4 border-l-3 border-support bg-card px-3 py-2 text-body" role="alert">
		<Refusal words={refused} />
	</p>
{:else if sent === undefined}
	<p class="mt-4 text-body text-ink-2">{m.loading()}</p>
{:else if sent.length === 0}
	<p class="mt-4 text-body text-ink-2">{m.sent_none()}</p>
{:else}
	<div class="mt-3 flex flex-wrap gap-2" role="group" aria-label={m.sent_show()}>
		{#each [['all', m.sent_show_all( { count: counts.all } )], ['failed', m.sent_show_failed( { count: counts.failed } )], ['working', m.sent_show_working( { count: counts.working } )]] as [choice, label] (choice)}
			<button
				type="button"
				aria-pressed={showing === choice}
				onclick={() => (showing = choice as Showing)}
				class="min-h-[40px] rounded-sm border px-3 text-read
					{showing === choice ? 'border-accent bg-accent text-on-accent' : 'border-rule bg-card text-ink'}"
				>{label}</button
			>
		{/each}
	</div>

	{#if days.length === 0}
		<p class="mt-4 text-body text-ink-2">{m.sent_none_showing()}</p>
	{/if}
	{#each days as day (day.heading)}
		<h3 class="mt-[18px] border-b border-rule pb-1 text-read font-medium text-ink-2">
			{day.heading}
		</h3>
		<ul>
			{#each day.rows as row (row.job_id)}
				{@const what = whatWasSent(row)}
				<li class="flex gap-3 border-b border-rule py-3">
					{@render thumb(row)}
					<span class="min-w-0 flex-1">
						<span class="block text-read text-ink-2">
							{kindOf(row)} · {aClock(row.created_at)}
						</span>
						<span
							class={[
								// Two lines of a quote, one of a link or a count. A clamp
								// draws its own box, so the quote is not also a block.
								'text-body break-words',
								what.gone ? 'text-ink-2 italic' : 'text-ink',
								what.quoted ? 'line-clamp-2' : ['block', !what.gone && 'truncate'],
							]}>{what.quoted ? `“${what.said}”` : what.said}</span
						>
						{@render ending(row)}
					</span>
				</li>
			{/each}
		</ul>
	{/each}
{/if}

<FilePicker {adding} kind="picture" bind:this={photoPicker} />
<FilePicker {adding} kind="pdf" bind:this={pdfPicker} />
<ToCheck {adding} />
