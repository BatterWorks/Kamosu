<!--
	What an `Adding` read and holds for somebody to check before anything is
	made, on the sheet a paste is checked on: a PDF (#176), or a web page with
	no recipe data read from its own text (#301), which says why it is asked.
	A photo or a PDF bound for Kamosu's AI agent is shown on a sheet of its own
	before it goes (#278). In a Demo, the examples it replays are a panel of
	their own (#406).

	Written once for every place that holds an `Adding`, so a link read on the
	Recipes + and one read where a search found nothing open the same sheet.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Adding } from './adding.svelte';
	import PasteSheet from './PasteSheet.svelte';
	import SendSheet from './SendSheet.svelte';
	import Replays from './Replays.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();
</script>

{#if adding.toSend}
	{#key adding.toSend}
		<SendSheet {adding} chosen={adding.toSend} />
	{/key}
{:else if adding.fromPdf}
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
{:else if adding.showingReplays}
	<Replays {adding} />
{/if}
