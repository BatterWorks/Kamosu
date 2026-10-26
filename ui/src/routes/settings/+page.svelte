<!--
	`/settings`: the menu on the phone, reached through the gear in the header
	(#102, #218). On the wide layout the menu is already beside every page, so
	this address opens the first page there (#469), as the prototype showed.
-->
<script lang="ts">
	import { useRoom } from '$lib/room.svelte';
	import SettingsMenu from './SettingsMenu.svelte';
	import SigningIn from './SigningIn.svelte';

	const room = useRoom();
</script>

{#if room.wide}
	<SigningIn />
{:else}
	<SettingsMenu />
{/if}
