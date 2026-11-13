<!--
	Test-only, for the same reason the Thread's harness exists: Recipe.svelte
	takes `branchId` as a real prop so a test can drive it directly, which the
	shared prop-less `renderScreen` helper cannot do.
-->
<script lang="ts">
	import type { KamosuClient } from '#lib/api/catalogue.js';
	import type { FileFetcher } from '#lib/api/files.js';
	import type { Uploader } from '#lib/api/upload.js';
	import type { Device } from '#lib/offline/device.svelte.js';
	import type { Wanted } from '#lib/how-much.js';
	import Kamosu from '#lib/shell/Kamosu.svelte';
	import Recipe from './Recipe.svelte';

	interface Props {
		client: KamosuClient;
		branchId: string;
		device?: Pick<Device, 'installed' | 'apple'>;
		files?: FileFetcher;
		/** Where a file handed over again goes (#279). */
		upload?: Uploader;
		/** The amount the address names (#486). */
		openAt?: Wanted;
	}

	let { client, branchId, device, files, upload, openAt }: Props = $props();
</script>

<Kamosu {client} {files} {upload}>
	<Recipe {branchId} {device} {openAt} />
</Kamosu>
