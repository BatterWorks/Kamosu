<!--
	The route itself: reads the URL's Job id and hands it to Report.svelte,
	which holds everything the screen actually does.

	Keyed on the id, so each import gets a Report of its own. On the wide
	layout one Report follows another under the same route (#200), and a
	Report kept across two imports would show the first one's pairs, already
	answered, under the second one's address.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { replayIn } from '$lib/agent/replay';
	import Report from './Report.svelte';

	/** A Replay picked again is a Report of its own, played from Sent (#406). */
	const replay = $derived(replayIn(page.url, page.params.jobId));
</script>

{#if page.params.jobId}
	{#key `${page.params.jobId}@${replay?.startedAt ?? ''}`}
		<Report jobId={page.params.jobId} {replay} />
	{/key}
{/if}
