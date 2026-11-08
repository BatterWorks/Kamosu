<!--
	The route: Shelf.svelte holds everything the screen actually does.

	`?tag=` is read here rather than in the screen, which is this project's
	pattern for every route-shaped input (see `Thread.svelte`): the screen takes
	an ordinary prop, so a test drives it without a router. A chip on a recipe
	page is what puts the tag in the URL (#104).

	`snapshot` is how going back finds the shelf as it was left (#191). SvelteKit
	calls `capture` as the reader leaves, keeps the answer with that entry in the
	browser's history, and calls `restore` only when the reader returns to that
	same entry: back, forward, or a reload of it. Arriving by the tab bar or a
	Tag chip is a new entry, so it restores nothing and the shelf starts whole.
	It is registered here, beside the router, so the screen stays free of it.
	The id is written out rather than made from the call site, so a release
	that moves this line still restores a shelf kept by the one before (#530).
	A shelf kept through the old `export const snapshot` is stored elsewhere by
	SvelteKit, so a tab open across the release that brought this starts whole
	once.
-->
<script lang="ts">
	import { snapshot } from '$app/navigation';
	import { page } from '$app/state';
	import Shelf from './Shelf.svelte';

	let shelf = $state<ReturnType<typeof Shelf>>();

	snapshot<unknown>({
		id: 'recipes-shelf',
		capture: () => shelf?.capture(),
		restore: (left) => shelf?.restore(left),
	});
</script>

<Shelf bind:this={shelf} tag={page.url.searchParams.get('tag')} />
