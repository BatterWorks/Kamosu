<!--
	Test-only, for the same reason the Thread's harness exists: Recipe.svelte
	takes `branchId` as a real prop so a test can drive it directly, which the
	shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '$lib/api/catalogue';
	import type { FileFetcher } from '$lib/api/files';
	import type { Uploader } from '$lib/api/upload';
	import type { Device } from '$lib/offline/device.svelte';
	import Kamosu from '$lib/shell/Kamosu.svelte';
	import Recipe from './Recipe.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		device?: Pick<Device, 'installed' | 'apple'>;
		files?: FileFetcher;
		/** Where a file handed over again goes (#279). */
		upload?: Uploader;
	}

	let { client, branchId, device, files, upload }: Props = $props();
</script>

<Kamosu {client} {files} {upload}>
	<Recipe {branchId} {device} />
</Kamosu>
