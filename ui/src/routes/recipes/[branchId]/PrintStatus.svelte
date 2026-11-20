<!--
	What printing a Sheet (#75, ADR 0023) has to say, under the acts on a
	recipe (#649). Print is asked for from the More sheet, which closes as it
	is tapped, so what follows is said here, in the place the cook is looking
	(U12): the Sheet being set, set and waiting for the tap that shares it on
	an iPhone (#149), taking a while, or failed. Nothing at all while nobody
	is printing. `sheet.svelte.ts` holds the waiting.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import type { Sheet } from './sheet.svelte';
	import ActIcon from './ActIcon.svelte';

	let { sheet }: { sheet: Sheet } = $props();
</script>

{#if sheet.printing === 'ready'}
	<!-- Already on the phone, so sharing it waits for nothing (#149). -->
	<div class="mx-gutter mt-3">
		<button type="button" class="second-button w-full" onclick={sheet.share}>
			<ActIcon name="print" class="h-[20px] w-[20px] shrink-0" />{m.recipe_share_sheet()}
		</button>
	</div>
{:else if sheet.printing === 'setting'}
	<p class="mx-gutter mt-3 text-read text-ink-2" role="status">{m.recipe_print_setting()}</p>
{:else if sheet.printing === 'stillSetting'}
	<p class="mx-gutter mt-3 text-read text-ink-2" role="status">
		{sheet.sharing ? m.recipe_share_still_going() : m.recipe_print_still_going()}
	</p>
{:else if sheet.printing === 'failed'}
	<p class="mx-gutter mt-3 text-read text-support" role="alert">{m.recipe_print_failed()}</p>
{/if}
