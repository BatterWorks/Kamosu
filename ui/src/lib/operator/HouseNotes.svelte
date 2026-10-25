<!--
	The agent's house notes, written in a sheet of their own (#344, option B of
	18 October 2026). The Runner section shows one row for them, and this is
	what the row opens: what house notes are, the box, Save, who saved them last
	and what an agent of your own needs to follow them.

	Save is off until the text differs from what is saved, and saving a blank
	box takes the notes away (`set_house_notes`). The box sets no length of its
	own: a browser counts an emoji as two, the Core as one, and the Core's
	refusal of notes past 4,000 characters is shown here in its own words. The sheet stays open after a
	save, so its line says at once who saved them and when.

	Enter in the box starts a new line. The sheet confirms nothing on Enter, so
	it is handed no `onconfirm`.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetHouseNotesOutput } from '$lib/api/catalogue';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { savedLine } from './house-notes';

	let {
		notes,
		onsaved,
		ondone,
	}: {
		notes: GetHouseNotesOutput;
		onsaved: (saved: GetHouseNotesOutput) => void;
		ondone: () => void;
	} = $props();

	const kamosu = useKamosu();

	let text = $state(untrack(() => notes.notes ?? ''));
	let saving = $state(false);
	let said = $state<string | undefined>(undefined);

	const changed = $derived(text.trim() !== (notes.notes ?? ''));

	async function save() {
		saving = true;
		said = undefined;
		try {
			const saved = await kamosu.setHouseNotes({ notes: text });
			text = saved.notes ?? '';
			onsaved(saved);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			saving = false;
		}
	}
</script>

<SheetFrame
	labelledby="house-notes-title"
	tall={85}
	class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
	onclose={ondone}
>
	<h2 id="house-notes-title" class="font-display text-list-title font-semibold text-ink">
		{m.operator_notes_heading()}
	</h2>
	<p class="mt-2 mb-3 text-read text-ink-2">{m.operator_notes_what()}</p>
	<form
		onsubmit={(event) => {
			event.preventDefault();
			void save();
		}}
	>
		<label class="block text-read text-ink-2">
			{m.operator_notes_heading()}
			<textarea
				bind:value={text}
				rows="6"
				class="mt-1 block w-full rounded-sm border border-rule bg-ground px-3 py-2 text-body text-ink"
			></textarea>
		</label>
		<button
			type="submit"
			disabled={saving || !changed}
			class="mt-2 min-h-12 w-full rounded-sm border border-accent px-4 text-body font-medium
			text-accent disabled:opacity-60"
		>
			{m.operator_notes_save()}
		</button>
	</form>
	{#if said}
		<p class="mt-2 text-read text-support" role="alert"><Refusal words={said} /></p>
	{/if}
	<p class="mt-2 text-read text-ink-2">
		{notes.notes ? savedLine(notes) : m.operator_notes_none()}
	</p>
	<p class="mt-1 text-read text-ink-2">{m.operator_notes_own_agent()}</p>
	<button
		type="button"
		onclick={ondone}
		class="mt-3 mb-4 min-h-12 w-full rounded-sm px-4 text-body text-ink-2"
	>
		{m.operator_notes_close()}
	</button>
</SheetFrame>
