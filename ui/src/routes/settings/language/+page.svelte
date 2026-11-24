<!--
	Language and measures (#469): the two settings that follow your account
	to every device and agent, on one page.

	**Language** (#112) is two settings shown as one until somebody wants them
	apart (the choice of 23 September 2026, option C, kept by #653's option B
	of 24 November 2026). The interface locale is Paraglide's and lives in this
	browser; the Reading Language is the account's, and it decides which
	Language a recipe's title, a Tag and a Food are shown in at every Door
	(ADR 0006). Most people want both the same, so the one control moves both,
	and the sentence under it says so, with the mark an untranslated title
	carries. A French speaker keeping an English library is ordinary too, so
	the titles can split off. Whether they are split is read off the facts when
	the page opens, and never stored: a browser whose locale differs from the
	account's Reading Language opens split, because showing it folded would
	claim the titles follow a control they do not follow.

	**A Language is one line of a list, the one in use ticked; a measure is
	one of three equal buttons in a row** (#653, choice 4 of 24 November 2026,
	`lang-opt-A.html` and `lang-opt-B.html`): no choice wraps onto a second
	line in any Language, and each Language's list is the one a phone's own
	Settings shows.

	**Every choice applies at once and says so** (#653, the audit's G7): a
	tick beside the section's heading. A change of Kamosu's own words reloads
	the page, so the line saying what changed waits out the reload, and the
	install card stays away on that load (G9, `Notices.svelte`).

	**Measures** (#49, ADR 0016) live on the account, so a cook who switches
	to metric on her phone finds the recipe in metric on the iPad on the
	worktop, and an agent at the MCP door reads it the same way. Setting them
	stores nothing on any recipe and makes no Version. Each choice is named for
	what it shows, with an example line (G8).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import Screen from '#lib/shell/Screen.svelte';
	import Section from '#lib/shell/Section.svelte';
	import { languageName } from '#lib/language.js';
	import { WORDS_CHANGED } from '#lib/offline/put-away.js';
	import type { GetReadingPreferencesOutput } from '#lib/api/catalogue.js';

	const kamosu = useKamosu();

	/** Where the Language Kamosu just changed to waits out the reload (#112, #653). */
	const MOVED_KEY = 'kamosu.reading-language-moved';

	const names: Record<Locale, () => string> = {
		en: () => m.language_en(),
		fr: () => m.language_fr(),
		es: () => m.language_es(),
		nl: () => m.language_nl(),
	};

	type Measures = GetReadingPreferencesOutput['reading_measures'];

	const measureNames: Record<Measures, () => string> = {
		metric: () => m.measures_metric(),
		us: () => m.measures_us(),
		as_written: () => m.measures_as_written(),
	};

	/** The example line for each choice, and what Kamosu would show under it. */
	const examples: Record<Measures, () => { line: string; shown?: string }> = {
		metric: () => ({ line: m.measures_example_cup(), shown: m.measures_example_grams() }),
		us: () => ({ line: m.measures_example_grams_line(), shown: m.measures_example_cups() }),
		as_written: () => ({ line: m.measures_example_cup() }),
	};

	let preferences = $state<GetReadingPreferencesOutput | undefined>(undefined);

	const locale = getLocale();
	let split = $state(false);
	/** The Language Kamosu's words just changed to, said once after the reload. */
	const changedTo = changedBeforeReload();
	/** Which section was just saved, for its tick. */
	let saved = $state<'language' | 'titles' | 'measures' | undefined>(undefined);
	/** A change of Language running, so a second tap cannot race the first. */
	let changing = $state(false);

	$effect(() => {
		let current = true;
		kamosu
			.getReadingPreferences()
			.then((answer) => {
				if (!current) return;
				preferences = answer;
				split = answer.reading_language !== locale;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * Choosing Kamosu's language reloads the page, which would take the line
	 * saying what changed with it. So it is left for the next load of this
	 * page, in this tab only, and read once.
	 */
	function changedBeforeReload(): Locale | undefined {
		try {
			const left = sessionStorage.getItem(MOVED_KEY);
			sessionStorage.removeItem(MOVED_KEY);
			return locales.find((known) => known === left && known === locale);
		} catch {
			return undefined;
		}
	}

	/** A Language other than the one being read in: the mark a title would carry. */
	const markedExample = $derived(
		locales.find((other) => other !== preferences?.reading_language) ?? 'en',
	);

	/**
	 * Store part of the reading preferences on the account, sending the rest
	 * back as they are. False where the account refused, and the page is put
	 * back to what the account still holds. `set_reading_preferences` takes
	 * the two together, so changing Measures sends the account's own Language
	 * back, never the interface's.
	 */
	async function savePreferences(change: Partial<GetReadingPreferencesOutput>): Promise<boolean> {
		if (!preferences) return false;
		const previous = preferences;
		const next = { ...previous, ...change };
		if (
			next.reading_language === previous.reading_language &&
			next.reading_measures === previous.reading_measures
		) {
			return true;
		}
		preferences = next;
		try {
			await kamosu.setReadingPreferences(next);
			return true;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			preferences = previous;
			return false;
		}
	}

	async function chooseWords(next: Locale) {
		if (next === locale || changing) return;
		changing = true;
		// The account first: `setLocale` reloads the page, and a save still in
		// flight when it does may never land. A refused save still changes the
		// words, which is what was asked; the page then opens split, saying
		// truthfully that the titles did not follow.
		if (preferences && !split) await savePreferences({ reading_language: next });
		try {
			sessionStorage.setItem(MOVED_KEY, next);
			sessionStorage.setItem(WORDS_CHANGED, '1');
		} catch {
			// Only the line saying what changed is lost; the change is made.
		}
		setLocale(next);
	}

	// Each says it saved only once the account took it, and a refusal takes
	// the tick away with the choice. One save at a time, so a refusal puts
	// back what the account still holds and not what another save changed.
	async function chooseTitles(next: Locale) {
		if (changing || preferences?.reading_language === next) return;
		changing = true;
		saved = undefined;
		if (await savePreferences({ reading_language: next })) saved = 'titles';
		changing = false;
	}

	async function chooseMeasures(next: Measures) {
		if (changing || preferences?.reading_measures === next) return;
		changing = true;
		saved = undefined;
		if (await savePreferences({ reading_measures: next })) saved = 'measures';
		changing = false;
	}

	async function rejoin() {
		if (changing) return;
		changing = true;
		saved = undefined;
		if (await savePreferences({ reading_language: locale })) {
			split = false;
			saved = 'language';
		}
		changing = false;
	}
</script>

{#snippet ticked(language: Locale, pressed: boolean, choose: () => void)}
	<li>
		<button
			type="button"
			aria-pressed={pressed}
			onclick={choose}
			lang={language}
			class={['list-row justify-between text-body', pressed && 'font-semibold']}
		>
			{names[language]()}
			{#if pressed}
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[22px] w-[22px] shrink-0 text-accent"
					fill="none"
					stroke="currentColor"
					stroke-width="2.2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="m5 12 5 5 9-10" />
				</svg>
			{/if}
		</button>
	</li>
{/snippet}

{#snippet savedMark(on: boolean)}
	{#if on}
		<span role="status" class="text-read font-semibold text-accent">✓ {m.settings_saved()}</span>
	{/if}
{/snippet}

{#snippet mark()}
	<!-- The mark a title carries (Tile.svelte's colours, at the size of this
	     sentence). The sentence is whole without it, so a screen reader is not
	     read a code. -->
	<span aria-hidden="true" class="rounded-sm bg-support px-1 text-label text-ground uppercase"
		>{markedExample}</span
	>
{/snippet}

<Screen title={m.settings_language_page()}>
	<Section heading={m.settings_language()}>
		{#snippet aside()}{@render savedMark(saved === 'language')}{/snippet}
		{#if preferences && !split}
			<p class="mb-3 text-read text-ink">
				{m.settings_language_follows()}
				{@render mark()}
			</p>
		{:else if preferences}
			<p class="mb-3 text-read text-ink">{m.settings_language_words_only()}</p>
		{/if}
		<!-- Paraglide compiles every phrase to a function, so this list can only
		     offer languages that were actually compiled. -->
		<ul class="list-rows" aria-label={m.settings_language()}>
			{#each locales as each (each)}
				{@render ticked(each, locale === each, () => chooseWords(each))}
			{/each}
		</ul>

		{#if changedTo}
			<p role="status" class="mt-3 border-l-3 border-accent bg-card px-3 py-2 text-body text-ink">
				✓ {m.settings_language_now({ language: languageName(changedTo) })}
			</p>
		{/if}

		{#if preferences && !split}
			<button type="button" class="quiet-button mt-3 text-accent" onclick={() => (split = true)}>
				{m.settings_language_split()}
			</button>
		{/if}
	</Section>

	{#if preferences && split}
		<Section heading={m.settings_reading_language()}>
			{#snippet aside()}{@render savedMark(saved === 'titles')}{/snippet}
			<p class="mb-3 text-read text-ink">
				{m.settings_reading_language_explained()}
				{@render mark()}
			</p>
			<ul class="list-rows" aria-label={m.settings_reading_language()}>
				{#each locales as each (each)}
					{@render ticked(each, preferences.reading_language === each, () => chooseTitles(each))}
				{/each}
			</ul>
			<button type="button" class="quiet-button mt-3 text-accent" onclick={rejoin}>
				{m.settings_reading_language_rejoin()}
			</button>
		</Section>
	{/if}

	{#if preferences}
		{@const chosen = preferences.reading_measures}
		{@const example = examples[chosen]()}
		<Section heading={m.settings_measures()}>
			{#snippet aside()}{@render savedMark(saved === 'measures')}{/snippet}
			<p class="mb-3 text-read text-ink">{m.measures_explained()}</p>
			<p class="mb-3 inline-block border-l-3 border-rule bg-card px-3 py-1">
				<span class="block text-body text-ink">{example.line}</span>
				{#if example.shown}
					<span class="block text-read text-ink-2">{example.shown}</span>
				{/if}
			</p>
			<div class="segmented" role="group" aria-label={m.settings_measures()}>
				{#each Object.entries(measureNames) as [measures, name] (measures)}
					<button
						type="button"
						aria-pressed={chosen === measures}
						onclick={() => void chooseMeasures(measures as Measures)}>{name()}</button
					>
				{/each}
			</div>
			<p class="mt-3 text-read text-ink">{m.measures_after()}</p>
		</Section>
	{/if}
</Screen>
