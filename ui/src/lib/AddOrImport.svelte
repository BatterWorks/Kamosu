<!--
	What a search that matched nothing offers (#62, ADR 0027): the things a
	person who does not have the recipe yet was about to do anyway. Adding it
	by the name just typed is the main act; adding one from a web page or from
	a recipe file are the two beside it (#650, chosen 20 November 2026).

	**Every offer *does* the thing rather than pointing at a screen to do it
	on.** Writing one is a tap, since the name is the one just typed: asking
	for a word somebody has just finished typing is asking them to type it
	twice. A file opens the phone's picker in the same tap. A web page opens
	the add sheet at its address field, the one *Add a recipe* opens (`onlink`),
	so the address is never typed under the search box.

	**The acts themselves live in `#lib/adding`**, shared with that sheet.
	**Nobody is asked where it goes.** A recipe you write goes into your own
	Cookbook, always (ADR 0041), so the tap writes it.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import NeedsServer from '#lib/offline/NeedsServer.svelte';
	import { useAdding } from '#lib/adding/adding.svelte.js';
	import FilePicker from '#lib/adding/FilePicker.svelte';
	import Said from '#lib/adding/Said.svelte';
	import ToCheck from '#lib/adding/ToCheck.svelte';

	interface Props {
		/** The name that was searched for, which writing one takes. */
		title: string;
		/** Open the add sheet at its address field. */
		onlink: () => void;
	}

	let { title, onlink }: Props = $props();

	const adding = useAdding();

	let picker = $state<FilePicker | undefined>(undefined);
</script>

<FilePicker {adding} bind:this={picker} />

<div class="grid gap-2">
	<button
		type="button"
		onclick={() => adding.write(title)}
		disabled={adding.working !== null}
		class="main-button w-full"
	>
		{m.recipes_nothing_add({ query: title })}
	</button>

	<NeedsServer
		label={m.recipes_nothing_import()}
		waiting={m.offline_waits_import_link()}
		disabled={adding.working !== null}
		onclick={onlink}
		shapeClass="second-button w-full"
		lookClass=""
		idleClass="text-ink-2 opacity-55"
	/>

	<NeedsServer
		label={adding.working === 'file' ? m.bring_in_file_working() : m.bring_in_file()}
		waiting={m.offline_waits_bring_in()}
		disabled={adding.working !== null}
		onclick={() => picker?.open()}
		shapeClass="second-button w-full"
		lookClass=""
		idleClass="text-ink-2 opacity-55"
	/>
</div>

<!--
	A PDF or a page read from its own text is checked on the sheet a paste is
	checked on before anything is made (#301).
-->
<ToCheck {adding} />
{#if !adding.fromLink}
	<Said {adding} />
{/if}
