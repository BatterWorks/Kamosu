<!--
	Language and measures (#469): the two settings that follow your account
	to every device and agent, on one page.

	**Language** (#112) is two settings shown as one until somebody wants them
	apart (the choice of 23 September 2026, option C). The interface locale
	is Paraglide's and lives in this browser; the Reading Language is the
	account's, and it decides which Language a recipe's title, a Tag and a
	Food are shown in at every Door (ADR 0006). Most people want both the
	same, so the one control moves both. A French speaker keeping an English
	library is ordinary too, so the recipes can split off. Whether they are
	split is read off the facts when the page opens, and never stored: a
	browser whose locale differs from the account's Reading Language opens
	split, because showing it folded would claim the recipes follow a control
	they do not follow.

	**Measures** (#49, ADR 0016) live on the account, so a cook who switches
	to metric on her phone finds the recipe in metric on the iPad on the
	worktop, and an agent at the MCP door reads it the same way. Setting them
	stores nothing on any recipe and makes no Version.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale, locales, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import Section from '$lib/shell/Section.svelte';
	import { languageName } from '$lib/language';
	import type { ReadingLanguage } from '$lib/tags';
	import type { GetReadingPreferencesOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	/** Where the line saying what moved waits out a reload (#112). */
	const MOVED_KEY = 'kamosu.reading-language-moved';

	const names: Record<Locale, () => string> = {
		en: () => m.language_en(),
		fr: () => m.language_fr(),
		es: () => m.language_es(),
	};

	type Measures = GetReadingPreferencesOutput['reading_measures'];

	const measureNames: Record<Measures, () => string> = {
		us: () => m.measures_us(),
		metric: () => m.measures_metric(),
		as_written: () => m.measures_as_written(),
	};

	let preferences = $state<GetReadingPreferencesOutput | undefined>(undefined);

	const locale = getLocale();
	let split = $state(false);
	/** The Language the shelf just moved to, said once beneath the picker. */
	let moved = $state<ReadingLanguage | undefined>(movedBeforeReload());
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
	 * Choosing a language with the two together reloads the page, which would
	 * take the line saying what moved with it. So it is left for the next load
	 * of this page, in this tab only, and read once.
	 */
	function movedBeforeReload(): ReadingLanguage | undefined {
		try {
			const left = sessionStorage.getItem(MOVED_KEY);
			sessionStorage.removeItem(MOVED_KEY);
			return locales.find((known) => known === left);
		} catch {
			return undefined;
		}
	}

	/** A Language other than the one being read in: the mark a card would carry. */
	const markedExample = $derived(
		locales.find((other) => other !== preferences?.reading_language) ?? 'en',
	);

	/**
	 * Store part of the reading preferences on the account, sending the rest
	 * back as they are. False where the account refused, and the page is put
	 * back to what the account still holds. `set_reading_preferences` takes
	 * the two together, so changing Measures sends the account's own Language
	 * back, never the interface's: quietly overwriting it would be a lie
	 * about what the button did.
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
		// truthfully that the recipes did not follow.
		if (preferences && !split) {
			const before = preferences.reading_language;
			if ((await savePreferences({ reading_language: next })) && before !== next) {
				try {
					sessionStorage.setItem(MOVED_KEY, next);
				} catch {
					// Only the line saying what moved is lost; the move is made.
				}
			}
		}
		setLocale(next);
	}

	async function chooseReading(next: ReadingLanguage) {
		if (changing || preferences?.reading_language === next) return;
		changing = true;
		if (await savePreferences({ reading_language: next })) moved = next;
		changing = false;
	}

	async function rejoin() {
		if (changing) return;
		changing = true;
		const before = preferences?.reading_language;
		if (await savePreferences({ reading_language: locale })) {
			split = false;
			moved = before === locale ? undefined : locale;
		}
		changing = false;
	}
</script>

{#snippet choice(label: string, pressed: boolean, choose: () => void)}
	<li>
		<button
			type="button"
			aria-pressed={pressed}
			onclick={choose}
			class="min-h-12 rounded-sm border px-4 text-body
				{pressed ? 'border-accent bg-accent text-on-accent' : 'border-rule bg-card text-ink'}"
		>
			{label}
		</button>
	</li>
{/snippet}

<Screen title={m.settings_language_page()}>
	<Section heading={m.settings_language()}>
		<!-- Paraglide compiles every phrase to a function, so this list can only
		     offer languages that were actually compiled. -->
		<ul class="flex flex-wrap gap-2" aria-label={m.settings_language()}>
			{#each locales as each (each)}
				{@render choice(names[each](), locale === each, () => chooseWords(each))}
			{/each}
		</ul>

		{#if preferences && !split}
			<p class="mt-3 text-read text-ink-2">
				{m.settings_language_follows()}
				<button type="button" class="quiet-button text-accent" onclick={() => (split = true)}>
					{m.settings_language_split()}
				</button>
			</p>
		{:else if preferences}
			<h3 id="reading-language" class="mt-6 mb-2 text-body font-semibold text-ink">
				{m.settings_reading_language()}
			</h3>
			<p class="mb-3 text-read text-ink-2">
				{m.settings_reading_language_explained()}
				<!-- An example of the mark a card carries (Tile.svelte's colours, at
				     the size of this sentence). The sentence is whole without it, so a
				     screen reader is not read a code. -->
				<span aria-hidden="true" class="rounded-sm bg-support px-1 text-label text-ground uppercase"
					>{markedExample}</span
				>
			</p>
			<ul class="flex flex-wrap gap-2" aria-labelledby="reading-language">
				{#each locales as each (each)}
					{@render choice(names[each](), preferences.reading_language === each, () =>
						chooseReading(each),
					)}
				{/each}
			</ul>
			<button type="button" class="quiet-button mt-3 text-accent" onclick={rejoin}>
				{m.settings_reading_language_rejoin()}
			</button>
		{/if}
		{#if preferences && moved}
			<p role="status" class="mt-3 border-l-3 border-accent bg-card px-3 py-2 text-read text-ink">
				{m.settings_reading_language_moved({ language: languageName(moved) })}
			</p>
		{/if}
	</Section>

	{#if preferences}
		{@const chosen = preferences.reading_measures}
		<Section heading={m.settings_measures()}>
			<p class="mb-3 text-read text-ink-2">{m.measures_explained()}</p>
			<ul class="flex flex-wrap gap-2">
				{#each Object.entries(measureNames) as [measures, name] (measures)}
					{@render choice(
						name(),
						chosen === measures,
						() => void savePreferences({ reading_measures: measures as Measures }),
					)}
				{/each}
			</ul>
		</Section>
	{/if}
</Screen>
