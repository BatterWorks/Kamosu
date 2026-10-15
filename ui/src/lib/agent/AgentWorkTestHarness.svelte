<!--
	Test-only. AgentWork reads its Job itself, from the client in context, so a
	test wraps it the way the shell does and shortens the time between reads.
-->
<script lang="ts">
	import type { GetJobOutput, KamosuClient } from '$lib/api/catalogue';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import AgentWork from './AgentWork.svelte';
	import type { Saying } from './agent';

	interface Props {
		client: KamosuClient;
		mine?: boolean;
		saying?: Saying;
		hasRecipe?: boolean;
		ended?: (status: GetJobOutput['status']) => void;
	}

	let { client, mine = true, saying, hasRecipe, ended }: Props = $props();
</script>

<Kamosu {client}>
	<AgentWork jobId="j_agent" {mine} {saying} {hasRecipe} {ended} every={5} />
</Kamosu>
