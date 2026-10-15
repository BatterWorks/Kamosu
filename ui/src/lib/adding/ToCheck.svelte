<!--
	What an `Adding` read and holds for somebody to check before anything is
	made, on the sheet a paste is checked on: a PDF (#176), or a web page with
	no recipe data read from its own text (#301), which says why it is asked.

	Written once for every place that holds an `Adding`, so a link read on the
	Recipes + and one read where a search found nothing open the same sheet.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';
	import PasteSheet from './PasteSheet.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();
</script>

{#if adding.fromPdf}
	{#key adding.fromPdf}
		<PasteSheet
			{adding}
			pasted={adding.fromPdf}
			heading={m.plus_pdf()}
			onback={() => (adding.fromPdf = null)}
		/>
	{/key}
{:else if adding.fromLink}
	{#key adding.fromLink}
		<PasteSheet
			{adding}
			pasted={adding.fromLink}
			heading={m.plus_link()}
			why={m.plus_link_read_as_text()}
			onback={() => (adding.fromLink = null)}
		/>
	{/key}
{/if}
