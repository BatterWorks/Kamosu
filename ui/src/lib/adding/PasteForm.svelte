<!--
	A whole recipe pasted as text, read, checked, and made (#175).

	This sits in the add sheet as one of its sources (#650),
	because you usually hold the text before any recipe page exists: making a
	title up first, only for the paste to replace it, was the long way round.
	It opens inside that sheet, as From a web page does.

	Reading asks `read_pasted_recipe`, which saves nothing. What it made is
	shown in `PasteSheet`, the sheet a PDF chosen from the add sheet raises too (#176).

	On an instance with a Runner the text goes to Kamosu's AI agent instead,
	with no draft to sort (#278): the box says so and its button is Send. No
	sheet asks first, since the text is already in view. Whether there is a
	Runner is read again when the button is pressed.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Refusal from '#lib/Refusal.svelte';
	import WatchExamples from './WatchExamples.svelte';
	import { useKamosu } from '#lib/kamosu.js';
	import type { ReadPastedRecipeOutput } from '#lib/api/catalogue.js';
	import { readPasted } from '#lib/pasted.svelte.js';
	import type { Adding } from './adding.svelte';
	import PasteSheet from './PasteSheet.svelte';

	interface Props {
		adding: Adding;
		/** What is typed, where the sheet holding the form keeps it (#650). */
		text?: string;
		/**
		 * The text was read: hand it on, where the sheet holding this form
		 * raises the sheet it is checked on in its own place, one sheet at a
		 * time (#650). Without it, the form raises that sheet itself.
		 */
		onread?: (pasted: ReadPastedRecipeOutput) => void;
	}

	let { adding, text = $bindable(''), onread }: Props = $props();

	const kamosu = useKamosu();

	let reading = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** What came back, or nothing while the text has not been read yet. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);

	async function read() {
		reading = true;
		failed = undefined;
		// A new try: what the last one said, a Demo's refusal included, goes.
		adding.quiet();
		try {
			if (await adding.readRunner()) {
				if (text.trim() === '') failed = m.write_paste_nothing();
				else await adding.send({ text });
				return;
			}
			const answer = await readPasted(kamosu, text);
			if (typeof answer === 'string') {
				failed = answer;
			} else if (onread) {
				onread(answer);
			} else {
				pasted = answer;
			}
		} finally {
			reading = false;
		}
	}
</script>

<form
	class="grid gap-2"
	onsubmit={(event) => {
		event.preventDefault();
		read();
	}}
>
	<p class="text-read text-ink">
		{adding.runner ? m.plus_agent_paste_hint() : m.plus_paste_hint()}
	</p>
	<textarea
		bind:value={text}
		rows="8"
		data-sheet-focus
		aria-label={m.write_paste_label()}
		class="block w-full rounded-sm border border-ink-2 bg-card p-2 text-body text-ink"></textarea>
	{#if failed ?? (adding.runner ? adding.failed : undefined)}
		<p class="text-read text-support" role="alert"><Refusal words={failed ?? adding.failed} /></p>
	{/if}
	<WatchExamples {adding} />
	<button
		class="main-button w-full"
		disabled={reading || adding.working !== null}
		aria-busy={reading || adding.working === 'send' ? 'true' : undefined}
	>
		{#if adding.runner}
			{reading ? m.send_doing() : m.send_do()}
		{:else}
			{reading ? m.write_paste_reading() : m.write_paste_read()}
		{/if}
	</button>
</form>

{#if pasted}
	<PasteSheet {adding} {pasted} heading={m.plus_paste()} onback={() => (pasted = null)} />
{/if}
