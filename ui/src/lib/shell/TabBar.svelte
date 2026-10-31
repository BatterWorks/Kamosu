<!--
	The tab bar: Home · Recipes · Plan · Shopping · Cooked.

	Five sections, fixed to the bottom of the screen, sized for a thumb at arm's
	length with wet hands. Settings are not here — they live behind the gear in
	the header (#102, #218), because they are not somewhere you go while
	cooking (ADR 0011). The Plan is the fifth, since #486 (ADR 0058): the days
	ahead are somewhere a cook goes every week, as the shopping is.

	Since #218 it wears the rail's look (#194): indigo, each place an icon over
	its word, the current one lit. `PlaceLink.svelte` draws a place for both,
	so the phone and the tablet read as one app at the bottom of the screen.

	A Demo's strip stands on it (#407), fixed with it, so it sits on the bar
	whatever height the bar takes.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import PlaceLink from './PlaceLink.svelte';
	import { isCurrent, places } from './places';

	interface Props {
		/** The address on screen, which says which place is the current one. */
		pathname: string;
		/** What stands on the bar: a Demo's strip (#407). */
		above?: Snippet;
	}

	let { pathname, above }: Props = $props();
</script>

<div class="fixed inset-x-0 bottom-0 z-20">
	{@render above?.()}
	<nav aria-label={m.nav_sections()} class="bg-accent pb-safe">
		<ul class="mx-auto flex max-w-2xl">
			{#each places as place (place.href)}
				<li class="flex flex-1 flex-col">
					<PlaceLink {place} current={isCurrent(place.href, pathname)} edge="top" />
				</li>
			{/each}
		</ul>
	</nav>
</div>
