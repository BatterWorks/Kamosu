<!--
	Adding a recipe: one sheet for every way in, opened by the labelled
	*Add a recipe* beside the title of Recipes and of Home (#650, option A,
	chosen 20 November 2026). It replaced the unlabelled + at the end of the
	search box (#174), which the UX audit found read as a search button (R4)
	and was the only way in from Recipes alone (H1).

	The sheet lists the sources, each with its icon, its name and a line saying
	what it takes, at full contrast (L2), and closes by a Close of its own (L1).
	A source that needs words, a web page's address, pasted text, a video's
	link or a title, opens its field inside the same sheet with a way back to
	the list (L5), so nothing about adding stands under the search box where an
	address could be typed into search. A source that needs a file closes the
	sheet and opens the phone's own picker, in the same tap, since a browser
	opens one only for a tap. The acts are `#lib/adding`'s, the same ones a
	search that found nothing offers (`AddOrImport`).

	**On an instance with a Runner the list is two groups** (#278, option 2):
	the quick ways on top, and under a heading of their own a photo of a page,
	a PDF, pasted text and a video, which go to Kamosu's AI agent and take
	minutes. Whether there is a Runner is read from the server each time the
	sheet opens, never from what the phone kept, and the first time the sheet
	waits for the answer rather than redraw under the finger. **In a Demo the
	agent's group opens with "Watch it read an example"** (#406).

	**Offline every row waits for the server** and says so in its own words,
	greyed (#76). None is queued: the outbox keeps Attempts and the shopping
	list, never a new recipe (ADR 0013).

	**One sheet at a time.** What a source raises after it, the sheet a paste
	or a PDF is checked on and the one a photo is shown on before it goes, takes
	the place of this one while it stands, and going back from it shows this
	one again where it was. What was typed into a field is held here, not in
	the field, so it is still there.

	**`/recipes?add` arrives with the sheet open**, and `?add=paste` with the
	box for pasted text (#412), where a screen that answers the address says so.

	A file or a link dropped on Recipes is the same act as its row (#204): the
	drop zone takes this sheet's `adding`, and what it starts is said under the
	title here, as the file a row picked is.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { m } from '#lib/paraglide/messages.js';
	import { Online } from '#lib/offline/device.svelte.js';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import type { ReadPastedRecipeOutput } from '#lib/api/catalogue.js';
	import { useAdding, type Adding } from './adding.svelte';
	import AddressForm from './AddressForm.svelte';
	import TitleForm from './TitleForm.svelte';
	import PasteForm from './PasteForm.svelte';
	import PasteSheet from './PasteSheet.svelte';
	import VideoForm from './VideoForm.svelte';
	import ToCheck from './ToCheck.svelte';
	import FilePicker from './FilePicker.svelte';
	import Said from './Said.svelte';
	import { ICON, WATCH_ICON, waitsForServer, type Source } from './sources.js';

	interface Props {
		/** The acts, where the screen shares them with something else of its own: a drop zone. */
		adding?: Adding;
		/** Whether `?add` in the address opens the sheet: Recipes answers it (#412). */
		opensOnAddQuery?: boolean;
	}

	let { adding = useAdding(), opensOnAddQuery = false }: Props = $props();

	const online = new Online();
	const heading = $props.id();

	/** A source whose words are asked for inside the sheet. */
	type Field = 'link' | 'paste' | 'video' | 'write';

	/** What the sheet shows: the list, one source's field, or nothing while it is closed. */
	let step = $state<'list' | Field | null>(null);
	/** The words typed into each field, kept while the sheet goes and comes back. */
	let address = $state('');
	let pastedText = $state('');
	/** Pasted text, read and waiting to be checked, on the sheet that takes this one's place. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);

	let picker = $state<FilePicker | undefined>(undefined);
	let pdfPicker = $state<FilePicker | undefined>(undefined);
	let picturePicker = $state<FilePicker | undefined>(undefined);

	interface Row {
		act: Source;
		label: () => string;
		why: () => string;
	}

	/** On an instance with no Runner, as on every instance before #278. */
	const ALL: Row[] = [
		{ act: 'link', label: m.plus_link, why: m.plus_link_why },
		{ act: 'file', label: m.plus_file, why: m.plus_file_why },
		{ act: 'pdf', label: m.plus_pdf, why: m.plus_pdf_why },
		{ act: 'paste', label: m.plus_paste, why: m.plus_paste_why },
		{ act: 'write', label: m.plus_write, why: m.plus_write_why },
	];
	/** With a Runner: the quick ways, then what Kamosu's AI agent reads (#278). */
	const QUICK: Row[] = [
		{ act: 'link', label: m.plus_link, why: m.plus_link_why },
		{ act: 'file', label: m.plus_file, why: m.plus_file_why },
		{ act: 'write', label: m.plus_write, why: m.plus_write_why },
	];
	const BY_AGENT: Row[] = [
		{ act: 'picture', label: m.plus_picture, why: m.plus_picture_why },
		{ act: 'pdf', label: m.plus_agent_pdf, why: m.plus_agent_pdf_why },
		{ act: 'paste', label: m.plus_agent_paste, why: m.plus_agent_paste_why },
		{ act: 'video', label: m.plus_video, why: m.plus_video_why },
	];
	const groups = $derived(adding.runner ? [QUICK, BY_AGENT] : [ALL]);
	/** The open field's own row, whose name heads the sheet while it is open. */
	const opened = $derived(groups.flat().find((row) => row.act === step));

	/**
	 * Whether this sheet stands. Another sheet the adding raised (a paste or a
	 * PDF to check, a photo to send, the Demo's examples) takes its place.
	 */
	const standing = $derived(
		step !== null &&
			pasted === null &&
			adding.toSend === null &&
			adding.fromPdf === null &&
			adding.fromLink === null &&
			!adding.showingReplays,
	);

	/**
	 * Open the sheet at its list, or straight at one source's field. Whether
	 * there is a Runner is read each time; the first time, the sheet waits for
	 * the answer, so its rows do not move under the finger.
	 */
	export async function open(at: 'list' | Field = 'list') {
		adding.quiet();
		const first = adding.runner === null;
		const reading = (
			online.current ? adding.readRunner() : Promise.resolve(adding.runner === true)
		).then((runner) => (runner ? adding.readReplays() : undefined));
		if (online.current && first) await reading;
		step = at;
	}

	export function close() {
		step = null;
	}

	/** A row tapped: its field in the sheet, or the phone's picker in the same tap. */
	function choose(act: Source) {
		// A new choice, so what the last one said is put away (#278).
		adding.quiet();
		if (act === 'file' || act === 'pdf' || act === 'picture') {
			step = null;
			(act === 'file' ? picker : act === 'pdf' ? pdfPicker : picturePicker)?.open();
		} else {
			step = act;
		}
	}

	/** The panel of examples a Demo replays (#406), which stands in this sheet's place. */
	function watch() {
		step = null;
		adding.quiet();
		adding.watchExamples();
	}

	/**
	 * Arriving at `/recipes?add` opens the sheet, and `?add=paste` the box for
	 * pasted text (#412): where an export turned away sends the cook with one
	 * recipe. A PDF and a photo stop at the list, since a browser opens its
	 * picker only for a tap. The address is put back without it, so a reload
	 * or a step back does not open the sheet again.
	 */
	onMount(() => {
		if (!opensOnAddQuery) return;
		const asked = page.url.searchParams.get('add');
		if (asked === null) return;
		const url = new URL(page.url.href);
		url.searchParams.delete('add');
		void goto(url, { replace: true, reset: false });
		void open(asked === 'paste' ? 'paste' : 'list');
	});
</script>

{#snippet icon(d: string)}
	<span class="flex h-[36px] w-[36px] shrink-0 items-center justify-center text-accent">
		<svg
			viewBox="0 0 24 24"
			aria-hidden="true"
			class="h-[26px] w-[26px]"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			stroke-linecap="round"
			stroke-linejoin="round"><path {d} /></svg
		>
	</span>
{/snippet}

{#snippet row(d: string, label: string, why: string, waits: string | null, onclick: () => void)}
	<li>
		<button type="button" class="list-row disabled:opacity-55" disabled={waits !== null} {onclick}>
			{@render icon(d)}
			<span class="min-w-0">
				{#if waits}
					<span class="block text-body text-ink-2">{waits}</span>
				{:else}
					<span class="block text-body font-medium text-accent">{label}</span>
					<span class="block text-read text-ink">{why}</span>
				{/if}
			</span>
		</button>
	</li>
{/snippet}

<FilePicker {adding} bind:this={picker} />
<FilePicker {adding} kind="pdf" bind:this={pdfPicker} />
<FilePicker {adding} kind="picture" bind:this={picturePicker} />

{#if standing}
	<SheetFrame labelledby={heading} tall={88} class="flex flex-col bg-card" onclose={close}>
		<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-6">
			<div class="mb-3 flex items-center gap-2">
				{#if step !== 'list'}
					<button
						type="button"
						class="-ml-3 flex h-12 w-12 shrink-0 items-center justify-center text-accent"
						aria-label={m.shell_back()}
						onclick={() => {
							adding.quiet();
							step = 'list';
						}}
					>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-6 w-6"
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
							stroke-linecap="round"
							stroke-linejoin="round"><path d="m15 5-7 7 7 7" /></svg
						>
					</button>
				{/if}
				<h2 id={heading} class="min-w-0 flex-1 font-display text-shelf-heading font-semibold">
					{step === 'list' ? m.plus_label() : opened?.label()}
				</h2>
				<button type="button" class="quiet-button gap-2 text-ink" onclick={close}>
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						class="h-[18px] w-[18px]"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
					>
					{m.plus_close()}
				</button>
			</div>

			{#if step === 'list'}
				{#each groups as rows, at (at)}
					{#if at > 0}
						<h3 class="mt-[20px] pb-2 text-read font-semibold text-ink">{m.plus_agent_group()}</h3>
					{/if}
					<ul class="list-rows">
						{#if at > 0 && adding.replays.length > 0}
							{@render row(
								WATCH_ICON,
								m.plus_replays(),
								m.plus_replays_why(),
								online.current ? null : m.offline_waits_replays(),
								watch,
							)}
						{/if}
						{#each rows as source (source.act)}
							{@render row(
								ICON[source.act],
								source.label(),
								source.why(),
								online.current ? null : waitsForServer(source.act, at > 0),
								() => choose(source.act),
							)}
						{/each}
					</ul>
				{/each}
			{:else if step === 'link'}
				<AddressForm {adding} bind:link={address} />
				<Said {adding} />
			{:else if step === 'paste'}
				<PasteForm {adding} bind:text={pastedText} onread={(read) => (pasted = read)} />
			{:else if step === 'video'}
				<VideoForm {adding} />
			{:else}
				<TitleForm {adding} />
				<Said {adding} />
			{/if}
		</div>
	</SheetFrame>
{/if}

{#if pasted}
	<PasteSheet {adding} {pasted} heading={m.plus_paste()} onback={() => (pasted = null)} />
{/if}

<!-- What a picked or dropped file is doing, and what it said, under the title. -->
{#if step === null}
	{#if adding.working === 'file'}
		<p class="mt-2 text-read text-ink" role="status">{m.bring_in_file_working()}</p>
	{:else if adding.working === 'pdf'}
		<p class="mt-2 text-read text-ink" role="status">{m.plus_pdf_working()}</p>
	{:else if adding.working === 'link'}
		<!-- A link that was dropped (#204): the address field, whose button says this, is not open. -->
		<p class="mt-2 text-read text-ink" role="status">{m.recipes_import_working()}</p>
	{/if}
	{#if !adding.fromPdf && !adding.fromLink && !adding.toSend}
		<Said {adding} />
	{/if}
{/if}

<!--
	A PDF, or a page read from its own text, is checked on the sheet a paste is
	(#176, #301), and a photo or a PDF for Kamosu's AI agent is shown on its own
	before it goes (#278).
-->
<ToCheck {adding} />
