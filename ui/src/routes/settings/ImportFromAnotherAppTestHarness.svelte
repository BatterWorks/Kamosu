<!--
	Test-only: ImportFromAnotherApp.svelte takes its uploader from context, so
	a test hands the provider a stand-in for `POST /api/uploads`, with no
	network.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { Uploader } from '$lib/api/upload';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import ImportFromAnotherApp from './ImportFromAnotherApp.svelte';

	interface Props {
		client: KamosuClient;
		upload: Uploader;
	}

	let { client, upload }: Props = $props();
</script>

<Kamosu {client} {upload}>
	<ImportFromAnotherApp every={1} />
</Kamosu>
