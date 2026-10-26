<!--
	`/operator` was the Operator's screen until #469 made its two rooms pages
	of Settings' This instance group. An address somebody kept lands on
	People, and `/operator#runner-keys`, which a waiting Job linked to (#277),
	on Runner and House Notes.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { RUNNER_KEYS } from '$lib/agent/agent';

	onMount(
		() =>
			void goto(location.hash === `#${RUNNER_KEYS}` ? '/settings/runner' : '/settings/people', {
				replaceState: true,
			}),
	);
</script>
