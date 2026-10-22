<!--
	The way on from a Demo's refusal of a visitor's own photo, PDF, text or
	link (#406, R1 as chosen on 22 October 2026): one button under the words
	that refused it, which opens the panel of examples (`Replays`) in its place.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();
</script>

{#if adding.refusedInDemo}
	<button
		type="button"
		class="mt-2 flex min-h-12 w-full items-center justify-center rounded-sm border border-accent px-4 text-body text-accent"
		onclick={() => adding.watchExamples()}
	>
		{m.plus_replays()}
	</button>
{/if}
