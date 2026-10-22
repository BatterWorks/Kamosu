<!--
	The layout of Brought in and a Report: reads the address and hands it to
	BroughtIn.svelte, which holds everything the two screens share.

	The list lives here rather than in the list's own page so that it is still
	standing when a Report is opened beside it on the wide layout (#200).

	What one source brought (`from/<kind>`) is under this layout too and takes
	nothing from it: it is a full page on every layout.

	An address that asks for a Replay (#406) hands it to the frame, which
	replays that Job to the list and the Report alike.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { replayIn } from '$lib/agent/replay';
	import BroughtIn from './BroughtIn.svelte';

	let { children } = $props();
</script>

{#if page.params.sourceKind !== undefined}
	{@render children()}
{:else}
	<BroughtIn open={page.params.jobId} replay={replayIn(page.url, page.params.jobId)}>
		{@render children()}
	</BroughtIn>
{/if}
