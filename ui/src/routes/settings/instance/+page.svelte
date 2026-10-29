<!--
	`/settings/instance` was This instance until #469 made it a group of pages
	for an Operator. An address somebody kept lands on About Kamosu, where its
	version and its story went.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => void goto('/settings/about', { replaceState: true }));
</script>
