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
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';
	import { Online } from '$lib/offline/device.svelte';
	import type { Adding, ToSend } from './adding.svelte';
	import { ICON, waitsForServer } from './sources';
	import WatchExamples from './WatchExamples.svelte';

	interface Props {
		adding: Adding;
		chosen: ToSend;
	}

	let { adding, chosen }: Props = $props();

	const uid = $props.id();

	/** The photo as the browser holds it, let go when the sheet closes. */
	const shown = $derived(chosen.kind === 'picture' ? URL.createObjectURL(chosen.file) : null);
	$effect(() => {
		const url = shown;
		return () => {
			if (url) URL.revokeObjectURL(url);
		};
	});

	/**
	 * How many pages the PDF has, where its page objects are written in plain
	 * text, or nothing. Each page is counted once by its object's number, so a
	 * PDF edited and saved again, which writes a changed page a second time,
	 * is not counted twice. A PDF that packs its objects away says nothing
	 * here, and the line is left out rather than guessed.
	 */
	let pages = $state<number | null>(null);
	$effect(() => {
		if (chosen.kind !== 'pdf') return;
		let current = true;
		chosen.file
			.text()
			.then((text) => {
				const numbers = new Set<string>();
				for (const [, number, body] of text.matchAll(/(\d+)\s+\d+\s+obj\b([\s\S]*?)endobj/g)) {
					if (/\/Type\s*\/Page(?![A-Za-z])/.test(body)) numbers.add(number);
				}
				if (current) pages = numbers.size > 0 ? numbers.size : null;
			})
			// A file gone since it was chosen says nothing here; Send says why.
			.catch(() => {});
		return () => {
			current = false;
		};
	});

	const online = new Online();
	const sending = $derived(adding.working === 'send');
	const send = () => adding.send({ file: chosen.file });
	const back = () => {
		if (!sending) adding.toSend = null;
	};
</script>

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
		{chosen.kind === 'picture' ? m.send_photo_ask() : m.send_pdf_ask()}
	</p>
	<div class="mt-3">
		{#if shown}
			<img
				src={shown}
				alt={m.send_photo_alt()}
				class="block max-h-[34vh] w-full rounded-sm bg-ground-2 object-contain"
			/>
		{:else}
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
					<span class="block text-body break-words">{chosen.file.name}</span>
					{#if pages !== null}
						<span class="block text-read text-ink-2">
							{pages === 1 ? m.send_one_page() : m.send_pages({ count: pages })}
						</span>
					{/if}
				</span>
			</div>
		{/if}
	</div>
	{#if shown}
		<p class="mt-1 text-read break-words text-ink-2">{chosen.file.name}</p>
	{/if}
	<p class="mt-3 text-read">{m.send_said()}</p>
	{#if adding.failed && !adding.refusedInDemo}
		<p class="mt-2 text-read text-support" role="alert">{adding.failed}</p>
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
		<p class="mt-2 text-read text-support" role="alert">{adding.failed}</p>
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
