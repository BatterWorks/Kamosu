<!--
	Meaning Search (#63, ADR 0029, #469): a page of This instance, where it is
	managed and never where it is discovered. A settings screen is where
	features go to be undiscovered, which is why the offer lives inside
	*nothing found* instead, and ADR 0029 forbids making it twice. So before
	anybody has answered the offer this page says where it is made, and offers
	nothing itself.
-->
<script lang="ts">
	import Refusal from '$lib/Refusal.svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import OperatorOnly from '../OperatorOnly.svelte';

	const kamosu = useKamosu();
	const meaning = new MeaningSearch(kamosu);

	let asking = $state(false);
	$effect(() => {
		if (asking) return meaning.ask();
	});
</script>

<OperatorOnly title={m.settings_meaning()} onallowed={() => (asking = true)}>
	{#if meaning.status}
		{@const status = meaning.status}
		{#if status.on}
			<p class="text-body text-ink">{m.settings_meaning_on({ model: status.model })}</p>
			{#if status.recipes_not_yet_indexed > 0}
				<!-- Said rather than hidden: the index catches up by itself within
				     the minute, and pretending an edit made a second ago is already
				     findable by meaning would be a small lie with no upside. -->
				<p class="mt-1 text-read text-ink-2">
					{m.settings_meaning_catching_up({ count: status.recipes_not_yet_indexed })}
				</p>
			{/if}
			<!-- No warning and no confirmation: everything this discards is
			     derived from the recipes and rebuilds itself (ADR 0003, ADR 0009). -->
			<button type="button" class="quiet-button mt-3 text-accent" onclick={() => meaning.turnOff()}>
				{m.settings_meaning_turn_off()}
			</button>
		{:else if meaning.working}
			<p class="text-body text-ink-2" role="status">{meaning.saying}</p>
		{:else if status.state === 'unasked'}
			<p class="text-body text-ink-2">{m.settings_meaning_unasked()}</p>
		{:else}
			<p class="text-body text-ink-2">
				{status.state === 'declined' ? m.settings_meaning_declined() : m.settings_meaning_off()}
			</p>
			<button type="button" class="quiet-button mt-3 text-accent" onclick={() => meaning.turnOn()}>
				{m.recipes_nothing_meaning_turn_on()}
			</button>
		{/if}
		{#if meaning.failed}
			<p class="mt-2 text-read text-support" role="alert"><Refusal words={meaning.failed} /></p>
		{/if}
	{/if}
</OperatorOnly>
