<!--
	`/settings/account` was the Account page until #469 split it by job. An
	address somebody kept, a passkey's "open it here" link among them, lands
	on Signing in, where most of it went.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => void goto('/settings/signing-in', { replaceState: true }));
</script>
