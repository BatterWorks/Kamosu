<!--
	Test-only. SvelteKit takes the route's `snapshot` as a reader leaves Recipes
	and hands it back when they go back (#191). A test has no browser history to
	walk, so this renders the real route and lends out the snapshot it
	registered with `$app/navigation` (#530): the test plays SvelteKit's part,
	leaving with `capture` and coming back with `restore`.
-->
<script lang="ts">
	import type { KamosuClient } from '#lib/api/catalogue.js';
	import Kamosu from '#lib/shell/Kamosu.svelte';
	import { kept } from '../../testing/navigation';
	import Recipes from './+page.svelte';

	interface Props {
		client: KamosuClient;
	}

	let { client }: Props = $props();

	/** The route's own snapshot, exactly what SvelteKit is given. */
	export function snapshot() {
		const registered = kept.get('recipes-shelf');
		if (!registered) throw new Error('the route has not registered its snapshot');
		return registered;
	}
</script>

<Kamosu {client}>
	<Recipes />
</Kamosu>
