<!--
	A whole recipe pasted as text, read, checked, and made (#175).

	This sits in the + beside the Recipes search box as a fourth source,
	because you usually hold the text before any recipe page exists: making a
	title up first, only for the paste to replace it, was the long way round.
	It opens under the search row like From a link does (ADR 0027).

	Reading asks `read_pasted_recipe`, which saves nothing. What it made is
	shown in `PasteSheet`, the sheet a PDF chosen from the + raises too (#176).

	On an instance with a Runner the text goes to Kamosu's AI agent instead,
	with no draft to sort (#278): the box says so and its button is Send. No
	sheet asks first, since the text is already in view. Whether there is a
	Runner is read again when the button is pressed.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import type { ReadPastedRecipeOutput } from '$lib/api/catalogue';
	import { readPasted } from '$lib/pasted.svelte';
	import type { Adding } from './adding.svelte';
	import PasteSheet from './PasteSheet.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	const kamosu = useKamosu();

	let text = $state('');
	let reading = $state(false);
	let failed = $state<string | undefined>(undefined);
	/** What came back, or nothing while the text has not been read yet. */
	let pasted = $state<ReadPastedRecipeOutput | null>(null);

	async function read() {
		reading = true;
		failed = undefined;
		try {
			if (await adding.readRunner()) {
				if (text.trim() === '') failed = m.write_paste_nothing();
				else await adding.send({ text });
				return;
			}
			const answer = await readPasted(kamosu, text);
			if (typeof answer === 'string') {
				failed = answer;
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
	<p class="text-read text-ink-2">
		{adding.runner ? m.plus_agent_paste_hint() : m.plus_paste_hint()}
	</p>
	<textarea
		bind:value={text}
		rows="8"
		aria-label={m.write_paste_label()}
		class="block w-full rounded-sm border border-rule bg-card p-2 text-body text-ink"></textarea>
	{#if failed ?? (adding.runner ? adding.failed : undefined)}
		<p class="text-read text-support" role="alert">{failed ?? adding.failed}</p>
	{/if}
	<button
		class="min-h-12 rounded-sm bg-accent px-4 py-3 font-display text-body text-on-accent disabled:opacity-60"
		disabled={reading || adding.working !== null}
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
