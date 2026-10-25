<!--
	Which household of a Mealie backup is the cook's (#418, option B of 24
	October 2026): a sheet to confirm, risen over the import page once the file
	has been looked at. The prototype is
	`.dev/prototypes/418-household/b-two-phone.html`, with `b-two-macbook.html`
	for the window on a wide screen.

	Each household comes with its cooks and how many recipes it holds, since
	Mealie names the first of every group's households "Family" and the name
	alone may not say which. The first is chosen to start with, so the button
	always says what it brings. Choosing is no harm done: another household
	comes in by importing the same backup again, which the sheet says.

	The import page opens it once the file is looked at, and the Report page
	of a Job that ended asking opens it too, for a cook who went there.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { ICON } from '$lib/adding/sources';
	import type { ImportFromAnotherAppOutput } from '$lib/api/catalogue';

	type Households = NonNullable<ImportFromAnotherAppOutput['which_household']>['households'];

	interface Props {
		households: Households;
		/** The file's name, as the cook knows it, where the screen knows it. */
		file?: string;
		choose: (household: string) => void;
		close: () => void;
	}

	let { households, file, choose, close }: Props = $props();

	const uid = $props.id();
	// The first to start with, as the sheet was opened: a new backup is a new sheet.
	// svelte-ignore state_referenced_locally
	let chosen = $state(households[0]?.name ?? '');
	const recipes = $derived(households.find((home) => home.name === chosen)?.recipes ?? 0);

	function bring() {
		if (chosen) choose(chosen);
	}
</script>

<SheetFrame
	labelledby="{uid}-title"
	tall={78}
	class="overflow-y-auto border-t border-rule bg-ground px-gutter pt-6"
	onclose={close}
	onconfirm={bring}
>
	{#if file}
		<p class="text-read break-words text-ink-2">
			<svg
				viewBox="0 0 24 24"
				aria-hidden="true"
				class="mr-2 inline-block h-[18px] w-[18px] align-[-3px]"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"><path d={ICON.file} /></svg
			>{file}
		</p>
	{/if}
	<h2 id="{uid}-title" class="mt-1 font-display text-list-title font-semibold text-ink">
		{m.import_household_title()}
	</h2>
	<p class="mt-2 text-read text-ink">{m.import_household_lead({ count: households.length })}</p>

	<fieldset class="mt-3 border-t border-rule">
		<legend class="sr-only">{m.import_household_title()}</legend>
		{#each households as home (home.name)}
			<label class="flex min-h-12 items-center gap-3 border-b border-rule py-2 break-words">
				<input
					type="radio"
					name="{uid}-household"
					value={home.name}
					bind:group={chosen}
					class="h-[22px] w-[22px] shrink-0 accent-accent"
				/>
				<span class="min-w-0">
					<span class="block text-body font-semibold text-ink">{home.name}</span>
					<span class="block text-read text-ink-2"
						>{[...home.cooks, m.import_household_recipes({ count: home.recipes })].join(
							' · ',
						)}</span
					>
				</span>
			</label>
		{/each}
	</fieldset>
	<p class="mt-3 text-read text-ink-2">{m.import_household_again()}</p>

	<div class="mt-6 mb-4 flex flex-col gap-2">
		<button
			type="button"
			onclick={bring}
			class="min-h-12 rounded-sm bg-accent px-4 text-body font-semibold text-on-accent"
		>
			{m.import_household_bring({ count: recipes })}
		</button>
		<button
			type="button"
			onclick={close}
			class="min-h-12 rounded-sm border border-accent px-4 text-body font-medium text-accent"
		>
			{m.import_household_not_now()}
		</button>
	</div>
</SheetFrame>
