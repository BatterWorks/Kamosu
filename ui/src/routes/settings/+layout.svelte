<!--
	Settings and every page of it (#469). On the wide layout the menu stays
	in a column beside the page that is open, as Foods keeps its list beside
	a Food (#199, ADR 0044), so moving from one page to another is one click.
	The address alone says which page is open, the same on every layout.

	On the phone each page is a screen of its own, as it was, and the menu is
	the screen at `/settings`.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { useRoom } from '$lib/room.svelte';
	import SettingsMenu from './SettingsMenu.svelte';
	import { FIRST_PAGE } from './pages';

	let { children } = $props();

	const room = useRoom();

	/** `/settings` on the wide layout opens the first page beside the menu. */
	const current = $derived(page.url.pathname === '/settings' ? FIRST_PAGE : page.url.pathname);
</script>

{#if room.wide}
	<div class="list-beside">
		<SettingsMenu {current} />
		<div class="min-w-0">{@render children()}</div>
	</div>
{:else}
	{@render children()}
{/if}
