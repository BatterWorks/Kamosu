<!--
	This device (#76, #469): the copy of the library kept on the device in
	your hand, and putting Kamosu on its Home Screen, where the copy stays.
	Where the "not right now" facts live once their cards are put away: the
	card says a thing once, and this is where it can be read again.

	Measures were here and moved to Language and measures, since they follow
	the account to every device rather than staying on this one.
-->
<script lang="ts">
	import { aDate } from '$lib/dates';
	import { m } from '$lib/paraglide/messages';
	import { useRoom } from '$lib/room.svelte';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import InstallSteps from '$lib/offline/InstallSteps.svelte';
	import { thisDevice } from '$lib/offline/device.svelte';
	import { readableSize, useLibrary } from '$lib/offline/library.svelte';

	const room = useRoom();
	const device = thisDevice();
	const library = useLibrary();

	/** Whether there is anything to say about the copy yet: none while it is unknown. */
	const said = $derived(!device.secure || ['filling', 'kept', 'absent'].includes(library.phase));
</script>

<Screen title={m.settings_device()}>
	{#if said}
		<Section heading={m.settings_offline_copy()}>
			<div class="space-y-3 text-body">
				{#if !device.secure}
					<p class="text-support">{m.offline_insecure_body()}</p>
				{:else if library.phase === 'filling'}
					<p>{m.offline_library_filling({ done: library.done, total: library.total })}</p>
				{:else if library.phase === 'kept' && library.filledAt}
					<p>
						{(room.wide ? m.offline_phone_kept_device : m.offline_phone_kept)({
							count: library.held.size,
							date: aDate(library.filledAt),
						})}
					</p>
					<button
						type="button"
						onclick={() => void library.fill()}
						class="min-h-12 rounded-sm border border-rule bg-card px-4 text-body text-accent"
					>
						{m.offline_phone_refresh()}
					</button>
				{:else if library.phase === 'absent'}
					<p>
						{m.offline_library_body({
							count: library.count,
							size: readableSize(library.bytes),
						})}
					</p>
					<button
						type="button"
						onclick={() => void library.fill()}
						class="min-h-12 rounded-sm border border-accent bg-accent px-4 text-body text-on-accent"
					>
						{m.offline_library_fetch()}
					</button>
				{/if}
			</div>
		</Section>
	{/if}

	{#if device.secure}
		<Section heading={m.settings_home_screen()}>
			<div class="text-body">
				{#if device.installed}
					<p>{m.offline_phone_installed()}</p>
				{:else}
					<InstallSteps apple={device.apple} />
				{/if}
			</div>
		</Section>
	{/if}
</Screen>
