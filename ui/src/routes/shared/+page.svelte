<!-- What Android's Share menu answers (#452): `Shared.svelte` draws what the worker kept. -->
<script lang="ts">
	import { page } from '$app/state';
	import { readAnswer, type SharedAnswer } from '$lib/offline/shared';
	import Shared from './Shared.svelte';

	let answer = $state<SharedAnswer | undefined>(undefined);
	let read = $state(false);

	$effect(() => {
		const search = page.url.searchParams;
		readAnswer(caches, search)
			.catch(() => undefined)
			.then((kept) => {
				answer = kept;
				read = true;
			});
	});
</script>

{#if read}
	<Shared {answer} />
{/if}
