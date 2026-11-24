<!--
	Renaming, merging and deleting a Tag: Settings, not the recipe page (#104).

	They were put here on 22 September 2026, after first being shown
	inside the recipe's own tag sheet. The answer was: "I don't think a recipe page is the place
	to rename tags, it makes no sense." That is right, and the line is a useful one
	to keep: the sheet on a recipe answers *which of my words describe this
	dish*, and this answers *what my words are*. One is about a dish and the
	other is about a library.

	**Each tag is one row that opens a sheet** (#653, option A of 24 November
	2026, its panel as redrawn; the prototype is
	`.dev/prototypes/653-settings/tags-A-*.html`). The page was a text box, a
	merge menu and two small words on every tag, 178 controls tapped under
	48px on the dev library. Now a row says the tag and how many recipes have
	it, and the sheet holds the rest one thing at a time: the name, which a tap
	turns into a field; the tag's recipes; merging, which picks the other tag
	from a list that can be searched; and deleting, apart at the foot. Merging
	and deleting each ask once, inside the sheet.

	THE COUNT IS ON EVERY ROW, and it is the same number the shelf shows for the
	same tag, because both count Lineages (`recipes_with_tag`). That is what
	makes deleting an informed act rather than a leap.

	NOTHING HERE CHANGES A RECIPE (ADR 0035), and the page says so once at the
	top rather than on every row. A rename reaches every recipe carrying the Tag
	immediately, which is the whole point of keeping Tags once per Cookbook.

	ONE COOKBOOK'S TAGS (#131, screen choice 5). Tags belong to the Cookbook that
	writes the recipes, so the words listed here are the ones you and your
	Co-authors file by.

	A TAG IS NAMED PER LANGUAGE, NOT SPLIT BY IT (ADR 0006). So a row shows the
	name in the reader's own Language where there is one, marks the Language it
	fell back to where there is not, and the sheet offers to add the missing one:
	the only place in Kamosu where a Tag known only as *mijoté* can be given an
	English name.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Refusal from '#lib/Refusal.svelte';
	import ListRow from '#lib/ListRow.svelte';
	import SheetFrame from '#lib/SheetFrame.svelte';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import {
		byWord,
		tagLanguage,
		tagWord,
		type ReadingLanguage,
		type Tag,
		type TagLanguage,
	} from '#lib/tags.js';
	import { languageName } from '#lib/language.js';

	interface Props {
		/**
		 * The Language this Person reads RECIPES in, which Settings has already
		 * asked for. Not the interface locale: the two are separate settings,
		 * and a Tag's name belongs to the Language its recipes are read in
		 * (ADR 0006).
		 */
		readingLanguage: ReadingLanguage;
	}

	let { readingLanguage }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	/** Your Cookbook's Tags, as last read. */
	let held = $state<Tag[]>([]);
	let failed = $state<string | undefined>(undefined);
	/** The act running right now, so nothing can be fired twice. */
	let busy = $state(false);
	/** The Tag whose sheet is open, by id, so a fresh list keeps it open. */
	let openId = $state<string | null>(null);
	/** Which step of the sheet shows: the tag itself, its name being typed, a merge, or a delete. */
	let step = $state<'tag' | 'rename' | 'merge' | 'merging' | 'delete'>('tag');
	/** The name being typed, in the Language of the name on screen. */
	let typed = $state('');
	/** The missing name being typed, in the reader's own Language. */
	let added = $state('');
	/** What *Find a tag* holds. */
	let found = $state('');
	/** The Tag a merge goes into, once chosen. */
	let intoId = $state<string | null>(null);
	/** What a rename just reached, said once on the page and cleared by the next act. */
	let renamed = $state<number | null>(null);

	async function load() {
		try {
			held = (await kamosu.listTags({})).tags;
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			held = [];
		}
	}

	// Read again whenever the Reading Language changes: the Core chose each
	// word and its mark for the Language asked for when the list was read, so
	// a list kept across a change shows one Language while renames write
	// another (#112).
	$effect(() => {
		void readingLanguage;
		void load();
	});

	/** In the order a reader can predict: by the word they see. */
	const listed = $derived(byWord(held));
	const open = $derived(listed.find((tag) => tag.id === openId));
	const into = $derived(listed.find((tag) => tag.id === intoId));

	/** The other Tags a merge can go into, narrowed by what is typed. */
	const others = $derived.by(() => {
		const wanted = found.trim().toLocaleLowerCase();
		return listed.filter(
			(tag) =>
				tag.id !== openId &&
				(wanted === '' || tagWord(tag).name.toLocaleLowerCase().includes(wanted)),
		);
	});

	const recipes = (count: number) =>
		count === 1 ? m.settings_tags_one_recipe() : m.settings_tags_recipes({ count });

	function openTag(tag: Tag) {
		openId = tag.id;
		step = 'tag';
		failed = undefined;
		renamed = null;
		added = '';
	}

	function close() {
		if (busy) return;
		openId = null;
		intoId = null;
		found = '';
		failed = undefined;
	}

	function startRenaming(tag: Tag) {
		typed = tag.name ?? '';
		step = 'rename';
	}

	async function act(run: () => Promise<void>, after: 'close' | 'tag') {
		failed = undefined;
		busy = true;
		try {
			await run();
			await load();
			busy = false;
			if (after === 'close') close();
			else step = 'tag';
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			failed = error.message;
		} finally {
			busy = false;
		}
	}

	/**
	 * Name a Tag in one Language, or change the name it has there. Which
	 * Language is the one the name on screen is written in, renaming what you
	 * can see; the missing name goes in the reader's own (ADR 0006).
	 *
	 * The number said afterwards comes from the rename's own answer rather than
	 * the row's count, which may be stale.
	 */
	function rename(tag: Tag, language: TagLanguage, name: string) {
		const word = name.trim();
		if (word === '' || word === tag.name) {
			step = 'tag';
			return;
		}
		renamed = null;
		return act(async () => {
			renamed = (await kamosu.renameTag({ tag_id: tag.id, language, name: word })).recipes;
		}, 'tag');
	}

	// Neither says how far a rename reached any more: that tag is gone.
	const merge = (fromId: string, toId: string) =>
		act(
			() =>
				kamosu.mergeTags({ keep_tag_id: toId, merge_tag_id: fromId }).then(() => {
					renamed = null;
				}),
			'close',
		);

	const remove = (tagId: string) =>
		act(
			() =>
				kamosu.deleteTag({ tag_id: tagId }).then(() => {
					renamed = null;
				}),
			'close',
		);
</script>

{#snippet back(label: string, to: typeof step)}
	<button
		type="button"
		class="flex min-h-12 items-center gap-1 text-body text-accent"
		onclick={() => (step = to)}
	>
		<svg
			viewBox="0 0 24 24"
			aria-hidden="true"
			class="h-[20px] w-[20px] shrink-0"
			fill="none"
			stroke="currentColor"
			stroke-width="1.75"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="m15 5-7 7 7 7" />
		</svg>
		{label}
	</button>
{/snippet}

<p class="mb-4 text-read text-ink">{m.settings_tags_blurb()}</p>

{#if renamed !== null && openId === null}
	<p class="mb-4 text-read text-accent" role="status">
		{renamed === 0
			? m.settings_tags_renamed_none()
			: renamed === 1
				? m.settings_tags_renamed_one()
				: m.settings_tags_renamed({ count: renamed })}
	</p>
{/if}

{#if listed.length === 0}
	<p class="text-body text-ink-2">{m.settings_tags_none()}</p>
{:else}
	<div class="list-rows">
		{#each listed as tag (tag.id)}
			{@const word = tagWord(tag)}
			<ListRow
				title={word.elsewhere ? `${word.name} (${word.elsewhere.toUpperCase()})` : word.name}
				line={recipes(tag.recipes)}
				onclick={() => openTag(tag)}
			/>
		{/each}
	</div>
{/if}

{#if open}
	{@const tag = open}
	{@const word = tagWord(tag)}
	<SheetFrame labelledby="{uid}-tag" tall={85} class="flex flex-col bg-ground" onclose={close}>
		<div class="relative min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-6">
			<button
				type="button"
				aria-label={m.settings_tags_close()}
				class="absolute top-2 right-2 flex h-12 w-12 items-center justify-center text-ink"
				onclick={close}
			>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-6 w-6"
					fill="none"
					stroke="currentColor"
					stroke-width="1.75"
					stroke-linecap="round"
				>
					<path d="M6 6l12 12M18 6 6 18" />
				</svg>
			</button>

			{#if step === 'tag' || step === 'rename'}
				<p class="text-label font-medium text-accent uppercase">{m.settings_tags_sheet_kicker()}</p>
				{#if step === 'rename'}
					<form
						class="pr-12"
						onsubmit={(event) => {
							event.preventDefault();
							void rename(tag, tagLanguage(word.elsewhere, readingLanguage), typed);
						}}
					>
						<!-- Names the sheet while the name is a field (SheetFrame's labelledby). -->
						<label id="{uid}-tag" class="sr-only" for="{uid}-name"
							>{m.settings_tags_name_label()}</label
						>
						<input
							id="{uid}-name"
							data-sheet-focus
							bind:value={typed}
							class="mt-1 min-h-12 w-full rounded-sm border border-accent bg-card px-2 font-display text-title font-semibold text-ink"
						/>
						<p class="mt-2 text-read text-ink">
							{tag.recipes === 0
								? m.settings_tags_rename_reaches_none()
								: tag.recipes === 1
									? m.settings_tags_rename_reaches_one()
									: m.settings_tags_rename_reaches({ count: tag.recipes })}
						</p>
						<div class="mt-4 grid grid-cols-2 gap-3">
							<button type="button" class="second-button" onclick={() => (step = 'tag')}>
								{m.settings_tags_keep_name({ name: word.name })}
							</button>
							<button
								type="submit"
								class="main-button"
								disabled={busy}
								aria-busy={busy ? 'true' : undefined}
							>
								{m.settings_tags_save_name()}
							</button>
						</div>
					</form>
				{:else}
					<h2 id="{uid}-tag" class="pr-12">
						<button
							type="button"
							class="mt-1 w-full rounded-sm text-left font-display text-title font-semibold break-words text-ink"
							aria-label={m.settings_tags_rename_named({ tag: word.name })}
							onclick={() => startRenaming(tag)}>{word.name}</button
						>
					</h2>
					<p class="mt-1 text-read text-ink">
						{tag.recipes === 1
							? m.settings_tags_on_one()
							: m.settings_tags_on({ count: tag.recipes })}
					</p>
					{#if renamed !== null}
						<p class="mt-2 text-read text-accent" role="status">
							{renamed === 0
								? m.settings_tags_renamed_none()
								: renamed === 1
									? m.settings_tags_renamed_one()
									: m.settings_tags_renamed({ count: renamed })}
						</p>
					{/if}
				{/if}

				{#if step === 'tag'}
					{#if word.elsewhere}
						<!--
							The missing name, offered where the Tag has none in the
							reader's own Language (ADR 0006): the fallback fixed
							rather than merely displayed.
						-->
						<form
							class="mt-4"
							onsubmit={(event) => {
								event.preventDefault();
								// Emptied once the name is added; kept to try again if refused.
								void rename(tag, readingLanguage, added)?.then(() => {
									if (failed === undefined) added = '';
								});
							}}
						>
							<label class="block text-read text-ink" for="{uid}-added">
								{m.settings_tags_add_name({ language: languageName(readingLanguage) })}
							</label>
							<div class="mt-1 flex gap-2">
								<input
									id="{uid}-added"
									bind:value={added}
									class="min-h-12 min-w-0 flex-1 rounded-sm border border-rule bg-card px-2 text-body text-ink"
								/>
								<button type="submit" class="second-button" disabled={busy}>
									{m.settings_tags_add_name_do()}
								</button>
							</div>
						</form>
					{/if}

					<div class="mt-5 list-rows">
						{#if tag.recipes > 0}
							<ListRow
								title={tag.recipes === 1
									? m.settings_tags_see_one()
									: m.settings_tags_see({ count: tag.recipes })}
								href="/recipes?tag={encodeURIComponent(tag.id)}"
							/>
						{/if}
						{#if listed.length > 1}
							<ListRow
								title={m.settings_tags_merge_row()}
								line={m.settings_tags_merge_row_line()}
								onclick={() => {
									found = '';
									step = 'merge';
								}}
							/>
						{/if}
					</div>

					<div class="mt-10 border-t border-rule pt-3">
						<button
							type="button"
							class="flex min-h-12 items-center gap-2 text-body text-support"
							onclick={() => (step = 'delete')}
						>
							<svg
								viewBox="0 0 24 24"
								aria-hidden="true"
								class="h-[20px] w-[20px] shrink-0"
								fill="none"
								stroke="currentColor"
								stroke-width="1.75"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
							</svg>
							{m.settings_tags_delete_row()}
						</button>
					</div>
				{/if}
			{:else if step === 'merge'}
				{@render back(word.name, 'tag')}
				<h2 id="{uid}-tag" class="mt-2 pr-12 font-display text-title font-semibold text-ink">
					{m.settings_tags_merge_into_title({ from: word.name })}
				</h2>
				<label class="sr-only" for="{uid}-find">{m.settings_tags_find()}</label>
				<input
					id="{uid}-find"
					type="search"
					data-sheet-focus
					bind:value={found}
					placeholder={m.settings_tags_find()}
					class="mt-3 min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
				/>
				<div class="mt-3 list-rows">
					{#each others as other (other.id)}
						<ListRow
							title={tagWord(other).name}
							line={recipes(other.recipes)}
							onclick={() => {
								intoId = other.id;
								step = 'merging';
							}}
						/>
					{:else}
						<p class="py-3 text-body text-ink-2">{m.settings_tags_find_none()}</p>
					{/each}
				</div>
			{:else if step === 'merging' && into}
				{@const intoName = tagWord(into).name}
				{@render back(m.settings_tags_other_tags(), 'merge')}
				<h2 id="{uid}-tag" class="mt-2 pr-12 font-display text-title font-semibold text-ink">
					{m.settings_tags_merge_title({ from: word.name, into: intoName })}
				</h2>
				<p class="mt-2 text-body text-ink">
					{tag.recipes === 0
						? m.settings_tags_merge_said_none({ from: word.name })
						: tag.recipes === 1
							? m.settings_tags_merge_said_one({ from: word.name, into: intoName })
							: m.settings_tags_merge_said({
									count: tag.recipes,
									from: word.name,
									into: intoName,
								})}
				</p>
				<button
					type="button"
					class="mt-5 main-button w-full"
					disabled={busy}
					aria-busy={busy ? 'true' : undefined}
					onclick={() => void merge(tag.id, into.id)}
				>
					{m.settings_tags_merge_do({ into: intoName })}
				</button>
			{:else if step === 'delete'}
				{@render back(word.name, 'tag')}
				<h2 id="{uid}-tag" class="mt-2 pr-12 font-display text-title font-semibold text-ink">
					{m.settings_tags_delete_title({ tag: word.name })}
				</h2>
				<p class="mt-2 text-body text-ink">
					{tag.recipes === 0
						? m.settings_tags_delete_said_none()
						: tag.recipes === 1
							? m.settings_tags_delete_said_one()
							: m.settings_tags_delete_said({ count: tag.recipes })}
				</p>
				<button
					type="button"
					class="mt-5 second-button w-full loses"
					disabled={busy}
					aria-busy={busy ? 'true' : undefined}
					onclick={() => void remove(tag.id)}
				>
					{m.settings_tags_delete_do({ tag: word.name })}
				</button>
			{/if}

			{#if failed}
				<p class="mt-4 text-body text-support" role="alert"><Refusal words={failed} /></p>
			{/if}
		</div>
	</SheetFrame>
{/if}
