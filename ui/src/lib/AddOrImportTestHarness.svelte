<!--
	Test-only: `AddOrImport` takes its uploader from context, exactly as it
	takes its client, so a test hands the provider a stand-in for
	`POST /api/uploads` and no network is reached.

	The add sheet stands beside it, as on Recipes, since the offer of a web page
	opens it at its address field (#650).

	`Arrived` is rendered beside it because the two are one behaviour split over
	a navigation: bringing a file in notes what happened, and that line is what
	shows it. In the app the layout draws `Arrived` above every screen (#93).
-->
<script lang="ts">
	import type { KamosuClient } from '#lib/api/catalogue.js';
	import type { Uploader } from '#lib/api/upload.js';
	import Kamosu from '#lib/shell/Kamosu.svelte';
	import AddOrImport from './AddOrImport.svelte';
	import AddRecipe from './adding/AddRecipe.svelte';
	import Arrived from './Arrived.svelte';

	interface Props {
		client: KamosuClient;
		upload: Uploader;
		/** Where the reader is, which the app's layout reads off the URL. */
		pathname?: string;
	}

	let { client, upload, pathname = '/recipes' }: Props = $props();

	let addRecipe = $state<AddRecipe | undefined>(undefined);
</script>

<Kamosu {client} {upload}>
	<Arrived {pathname} />
	<AddRecipe bind:this={addRecipe} />
	<AddOrImport title="Chicken curry" onlink={() => addRecipe?.open('link')} />
</Kamosu>
