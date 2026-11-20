<!--
	The back arrow on a recipe's photograph, on the phone (#649): the round
	arrow the shell draws (`BackArrow`), standing on the photograph's top left
	corner and scrolling away with it, so it never covers a Step. The choice
	of 20 November 2026 on #649 (`en-short-A-phone.html`). While it is drawn
	the shell draws no arrow of its own. The wide layout keeps the shell's,
	fixed beside the rail, which covers nothing.
-->
<script lang="ts">
	import { useRoom } from '#lib/room.svelte.js';
	import BackArrow from './BackArrow.svelte';
	import { holdArrowOnPhoto } from './back-on-photo.svelte.js';

	interface Props {
		/** Where the arrow goes with nothing of Kamosu behind the page. */
		parent: string;
	}

	let { parent }: Props = $props();

	const room = useRoom();

	$effect(() => {
		if (room.wide) return;
		return holdArrowOnPhoto();
	});
</script>

{#if !room.wide}
	<BackArrow {parent} layout="photo" />
{/if}
