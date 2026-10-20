<!--
	Settings: reached through the gear in the header (#102, #218) on the phone
	and the rail's fifth place on the wide layout, never from the tab bar.

	A menu of rows (#221, option 2 of the audit of 9 October 2026), each
	opening a page of its own: Account, This device, Kitchens and cookbooks,
	This instance, the Security Record. The sections themselves are drawn by
	`Settings.svelte` on the page that holds them.

	Below them, set apart, who is signed in and *Sign out* (#380, option A of
	20 October 2026), the same on both layouts. It was a button on one row of
	the Sessions list, three pages down, and the owner could not find it.
	Signing out needs the server, so offline it waits for it, greyed.

	This is also where the signature transition is shown in the shell: the gear
	and this screen's title carry the same `view-transition-name`, so the gear
	grows into the page rather than the screen swapping (ADR 0012). Every recipe
	card that opens into its page later is this same pair, with a different name.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import { OperationError } from '$lib/api/client';
	import { useKamosu } from '$lib/kamosu';
	import { Online } from '$lib/offline/device.svelte';
	import { PAGES } from './pages';
	import SignOut from './SignOut.svelte';

	const kamosu = useKamosu();
	const online = new Online();

	let name = $state<string | undefined>(undefined);
	let signingOut = $state(false);

	onMount(() => {
		kamosu
			.getPerson()
			.then((person) => (name = person.name))
			.catch((error: unknown) => {
				// Not known, then not said: the button stands either way.
				if (!(error instanceof OperationError)) throw error;
			});
	});
</script>

<Screen title={m.settings_title()} expandsFrom="settings">
	<ul class="border-t border-rule">
		{#each PAGES as entry (entry.page)}
			<li class="border-b border-rule">
				<a
					href={entry.href}
					class="flex min-h-[64px] items-center justify-between gap-3 py-[10px] text-ink"
				>
					<span class="min-w-0">
						<span class="block font-display text-line font-semibold">{entry.title()}</span>
						<span class="block text-read text-ink-2">{entry.inside()}</span>
					</span>
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						class="h-6 w-6 shrink-0 text-ink-2"
						fill="none"
						stroke="currentColor"
						stroke-width="1.75"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="m9 5 7 7-7 7" />
					</svg>
				</a>
			</li>
		{/each}
	</ul>

	<div class="mt-[40px] border-t border-rule pt-3">
		{#if name}
			<p class="text-read text-ink-2">{m.settings_signed_in_as({ name })}</p>
		{/if}
		<button
			type="button"
			disabled={!online.current || signingOut}
			onclick={() => (signingOut = true)}
			class={[
				'mt-1 flex min-h-12 items-center gap-[10px] font-display text-line font-semibold',
				online.current ? 'text-accent' : 'text-ink-2 opacity-55',
			]}
		>
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="h-[22px] w-[22px] shrink-0"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4M10 17l5-5-5-5M15 12H3" />
			</svg>
			{online.current ? m.sign_out() : m.offline_waits_sign_out()}
		</button>
	</div>
</Screen>

{#if signingOut}
	<SignOut onclose={() => (signingOut = false)} />
{/if}
