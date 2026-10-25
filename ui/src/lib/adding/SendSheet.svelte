<!--
	A photo of a page or a PDF, shown once before it goes to Kamosu's AI agent
	(#278). A phone's photo picker makes it easy to tap the wrong picture, and
	the agent's answer is minutes away, so the cook sees what they chose before
	it goes rather than on the waiting page after.

	Chosen on 15 October 2026 from two options drawn over the real Recipes
	screen (option 2, recorded on #278). The screens never say "your agent":
	the agent is the instance's, run by its Operator, and a cook never set one
	up. They say Kamosu's AI agent.

	Send hands it over (`Adding.send`) and lands on its Import, which says
	whether it waits or is being read (#277). Back puts it away and sends
	nothing. Pasted text has no sheet: it is already in view in its own box.

	A Demo refuses it, and says so under Send with a button to the examples
	it replays instead (#406, R1).

	**Several photos are the pages of one recipe** (#452, option A of 24
	October 2026, `.dev/prototypes/452-share/pictures-a-three-phone.html`):
	side by side in the order they were picked, each numbered, with a cross to
	take it out, and a last tile that adds a page. One photo shows as it
	always did, with a way under it to add a second page.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Refusal from '$lib/Refusal.svelte';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import { MOST_PAGES, type Adding, type ToSend } from './adding.svelte';
	import FilePicker from './FilePicker.svelte';
	import { ICON, waitsForServer } from './sources';
	import WatchExamples from './WatchExamples.svelte';

	interface Props {
		adding: Adding;
		chosen: ToSend;
	}

	let { adding, chosen }: Props = $props();

	const uid = $props.id();

	/** Each photo as the browser holds it, let go when it is taken out or the sheet closes. */
	const shown = $derived(
		chosen.kind === 'picture' ? chosen.files.map((file) => URL.createObjectURL(file)) : [],
	);
	$effect(() => {
		const urls = shown;
		return () => {
			for (const url of urls) URL.revokeObjectURL(url);
		};
	});
	const pdf = $derived(chosen.files[0]);
	const pages = $derived(chosen.files.length);
	const roomForMore = $derived(chosen.kind === 'picture' && pages < MOST_PAGES);
	let pagePicker = $state<FilePicker | undefined>(undefined);

	/**
	 * How many pages the PDF has, where its page objects are written in plain
	 * text, or nothing. Each page is counted once by its object's number, so a
	 * PDF edited and saved again, which writes a changed page a second time,
	 * is not counted twice. A PDF that packs its objects away says nothing
	 * here, and the line is left out rather than guessed.
	 */
	let pdfPages = $state<number | null>(null);
	$effect(() => {
		if (chosen.kind !== 'pdf') return;
		let current = true;
		pdf
			.text()
			.then((text) => {
				const numbers = new Set<string>();
				for (const [, number, body] of text.matchAll(/(\d+)\s+\d+\s+obj\b([\s\S]*?)endobj/g)) {
					if (/\/Type\s*\/Page(?![A-Za-z])/.test(body)) numbers.add(number);
				}
				if (current) pdfPages = numbers.size > 0 ? numbers.size : null;
			})
			// A file gone since it was chosen says nothing here; Send says why.
			.catch(() => {});
		return () => {
			current = false;
		};
	});

	const online = new Online();
	const sending = $derived(adding.working === 'send');
	const send = () => adding.send({ files: chosen.files });
	const back = () => {
		if (!sending) adding.toSend = null;
	};
</script>

{#snippet plus(size: string)}
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class={size}
		fill="none"
		stroke="currentColor"
		stroke-width="1.75"
		stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg
	>
{/snippet}

{#if roomForMore}
	<FilePicker {adding} kind="page" bind:this={pagePicker} />
{/if}

<SheetFrame
	labelledby="{uid}-ask"
	tall={88}
	class="overflow-y-auto bg-ground px-gutter pt-4 pb-6 text-ink"
	onclose={back}
	onconfirm={() => {
		// Enter does what the button does, and offline the button waits.
		if (!sending && online.current) send();
	}}
>
	<p id="{uid}-ask" class="text-label text-ink-2 uppercase">
		{chosen.kind === 'pdf'
			? m.send_pdf_ask()
			: pages > 1
				? m.send_photos_ask({ count: pages })
				: m.send_photo_ask()}
	</p>
	{#if chosen.kind === 'pdf'}
		<div class="mt-3">
			<div class="flex items-center gap-3 rounded-sm border border-rule bg-card p-3">
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-8 w-8 shrink-0 text-accent"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"><path d={ICON.pdf} /></svg
				>
				<span class="min-w-0">
					<span class="block text-body break-words">{pdf.name}</span>
					{#if pdfPages !== null}
						<span class="block text-read text-ink-2">
							{pdfPages === 1 ? m.send_one_page() : m.send_pages({ count: pdfPages })}
						</span>
					{/if}
				</span>
			</div>
		</div>
	{:else if pages === 1}
		<div class="mt-3">
			<img
				src={shown[0]}
				alt={m.send_photo_alt()}
				class="block max-h-[34vh] w-full rounded-sm bg-ground-2 object-contain"
			/>
		</div>
		<p class="mt-1 text-read break-words text-ink-2">{chosen.files[0].name}</p>
		<button
			type="button"
			class="mt-2 inline-flex min-h-12 items-center gap-[6px] text-body text-accent disabled:opacity-40"
			disabled={sending}
			onclick={() => pagePicker?.open()}
		>
			{@render plus('h-[18px] w-[18px]')}
			{m.send_add_another_page()}
		</button>
	{:else}
		<ul
			class="-mx-gutter mt-3 flex [scrollbar-width:none] gap-2 overflow-x-auto px-gutter"
			aria-label={m.send_pages_order({ count: pages })}
		>
			{#each chosen.files as file, at (file)}
				<li class="relative h-[176px] w-[132px] shrink-0 overflow-hidden rounded-sm bg-ground-2">
					<img
						src={shown[at]}
						alt={m.send_page_alt({ number: at + 1 })}
						class="block h-full w-full object-cover"
					/>
					<span
						aria-hidden="true"
						class="absolute bottom-[6px] left-[6px] grid h-6 min-w-6 place-items-center round-mark bg-ink px-[6px] text-read text-ground"
						>{at + 1}</span
					>
					<button
						type="button"
						class="absolute top-1 right-1 grid h-8 w-8 place-items-center round-mark bg-ink/70 text-ground disabled:opacity-40"
						aria-label={m.send_take_out({ number: at + 1 })}
						disabled={sending}
						onclick={() => adding.takeOut(at)}
					>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-[18px] w-[18px]"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
						>
					</button>
				</li>
			{/each}
			{#if roomForMore}
				<li
					class="h-[176px] w-[132px] shrink-0 rounded-sm border border-dashed border-rule bg-card"
				>
					<button
						type="button"
						class="flex h-full w-full flex-col items-center justify-center gap-[6px] text-read text-accent disabled:opacity-40"
						disabled={sending}
						onclick={() => pagePicker?.open()}
					>
						{@render plus('h-[26px] w-[26px]')}
						{m.send_add_page()}
					</button>
				</li>
			{/if}
		</ul>
		<p class="mt-1 text-read text-ink-2">{m.send_pages_order({ count: pages })}</p>
	{/if}
	<p class="mt-3 text-read">{pages > 1 ? m.send_said_pages() : m.send_said()}</p>
	{#if adding.failed && !adding.refusedInDemo}
		<p class="mt-2 text-read text-support" role="alert"><Refusal words={adding.failed} /></p>
	{/if}
	<NeedsServer
		label={sending ? m.send_doing() : m.send_do()}
		waiting={waitsForServer(chosen.kind, true)}
		disabled={sending}
		onclick={send}
		shapeClass="mt-4 block w-full p-4 text-center font-display text-body disabled:opacity-60"
		lookClass="bg-accent text-on-accent"
	/>
	<!-- A Demo's refusal says itself under Send, with the way on to the examples (#406). -->
	{#if adding.refusedInDemo}
		<p class="mt-2 text-read text-support" role="alert"><Refusal words={adding.failed} /></p>
		<WatchExamples {adding} />
	{/if}
	<button
		type="button"
		class="mt-2 block w-full border border-rule p-4 text-center font-display text-body text-ink-2 disabled:opacity-40"
		disabled={sending}
		onclick={back}
	>
		{m.send_back()}
	</button>
</SheetFrame>
