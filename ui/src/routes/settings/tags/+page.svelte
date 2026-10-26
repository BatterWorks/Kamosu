<!--
	Tags (#104, #469). One card, your Cookbook's, since #131 put Tags in the
	Cookbook that writes the recipes they file. Renaming and merging are here
	and not on the recipe page, the choice of 22 September 2026 that
	`Tags.svelte` records: a tag is discovered on a recipe, and this is where
	the list is tidied.

	The Tags are named in the account's Reading Language, not the interface's,
	or a Tag would be named in the wrong Language for anybody whose two
	settings differ (ADR 0006). Until the preferences land, English, the
	default `set_reading_preferences` is offered with.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import { cookbookCalled } from '$lib/cookbook';
	import type { GetCookbookOutput, GetReadingPreferencesOutput } from '$lib/api/catalogue';
	import Tags from '../Tags.svelte';

	const kamosu = useKamosu();

	let cookbook = $state<GetCookbookOutput | undefined>(undefined);
	let preferences = $state<GetReadingPreferencesOutput | undefined>(undefined);
	let signedIn = $state(false);

	$effect(() => {
		let current = true;
		Promise.all([kamosu.getCookbook(), kamosu.getReadingPreferences()])
			.then(([book, answer]) => {
				if (!current) return;
				cookbook = book;
				preferences = answer;
				signedIn = true;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});
</script>

<Screen title={m.settings_tags()}>
	{#if signedIn}
		<Tags
			cookbook={cookbook ? cookbookCalled(cookbook) : m.settings_cookbook()}
			readingLanguage={preferences?.reading_language ?? 'en'}
		/>
	{/if}
</Screen>
