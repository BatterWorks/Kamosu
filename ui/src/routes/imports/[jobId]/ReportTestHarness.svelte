<!--
	Test-only: Report.svelte takes `jobId` as a real prop so a test can drive
	it directly, which the shared prop-less `renderScreen` helper cannot do.
	With `beside` it is drawn where the route draws it, in the frame that keeps
	the list of imports beside it on the wide layout. With `replay` it is drawn
	in that frame too, which plays the Replay (#406).
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Replaying } from '$lib/agent/replay';
	import type { Room } from '$lib/room.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import BroughtIn from '../BroughtIn.svelte';
	import Report from './Report.svelte';

	interface Props {
		client: KamosuClient;
		jobId: string;
		room?: Room;
		beside?: boolean;
		replay?: Replaying | null;
	}

	let { client, jobId, room, beside = false, replay = null }: Props = $props();
</script>

<Kamosu {client} {room}>
	{#if beside || replay}
		<BroughtIn open={jobId} {replay}>
			<Report {jobId} every={5} {replay} />
		</BroughtIn>
	{:else}
		<Report {jobId} every={5} />
	{/if}
</Kamosu>
