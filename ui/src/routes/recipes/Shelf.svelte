<!--
	Recipes: one shelf, and the one home for searching (#62, ADR 0027).

	Everything the Kitchens you cook in hold, merged, alphabetical, one card per
	Lineage. There is no Kitchen switcher and no modes — a switcher is filing
	through the back door, and it taxes every search with a permissions question
	you should never have to answer to find the katsu curry on a Tuesday.

	The filters are **held here and nowhere else**, which is the whole of
	how "no filter sticks" is built: leaving the screen destroys this component,
	and coming back makes a new one on the whole shelf.

	They answer two different questions (#231). *Whose recipes* is one Cookbook
	or one Kitchen, each a headed line of chips that works as the Tags line
	does: a tap holds a chip and a second tap lets it go. One of the two is
	held at a time, since a Cookbook holds the same recipes in every Kitchen
	that sees it. *Cooked or written by me* is about the reader and the recipe,
	whoever's Cookbook it is in, so it is a tick beside the count and it
	combines with everything else.

	Going back is the one return that is not a fresh start (#191). Open a result,
	find it is not the one, go back: the words, the filters, the answer and the
	place in the list are all as they were. `capture` and `restore` below are
	what the route hands SvelteKit for that, and SvelteKit restores only on back
	and forward, never on the tab bar. So the filters still do not stick: a
	visit by the tab is a new entry in the history, and nothing carries into it.

	The tag filter (#104) is the third, and it is the one exception to "nothing
	is written to the URL" — `?tag=` exists so that a chip on a recipe page can
	say which tag it meant, and the route reads it into a prop this screen takes
	once. It is a message from the other screen, not a place the filter lives:
	nothing here ever writes it, so arriving by the tab bar lands on the whole
	shelf as it always did. Going back to a shelf that arrived tagged restores
	whatever tag was held when it was left, which may be none.

	Searching is one Operation, the same one that answers the unsearched shelf.
	Meaning Search arrived inside that Operation rather than beside it, so
	nothing here had to learn a second way of asking (ADR 0029) — the answer
	simply gained two things: a match can say it was found by meaning, and an
	answer can say that nothing was close enough and these are the nearest.
-->
<script lang="ts" module>
	import type { ListKitchensOutput } from '$lib/api/catalogue';

	/**
	 * The shelf as it was left, for going back to it (#191). SvelteKit keeps it
	 * in `sessionStorage`, so it is plain JSON and deliberately small: what the
	 * reader chose and where they had scrolled, never the answer. The answer is
	 * asked again, from the phone's copy first, so it is as current as any.
	 */
	export interface LeftShelf {
		typed: string;
		/** Whose recipes: everyone's, one Kitchen's or one Cookbook's (#231). */
		filter: { kind: 'all' } | { kind: 'kitchen'; id: string } | { kind: 'cookbook'; id: string };
		/** Only what the reader cooked or wrote, on top of whatever else is held (#231). */
		cookedOrWritten: boolean;
		tag: string | null;
		/** How far down the page was, in pixels. */
		scrolled: number;
	}

	/**
	 * Whether a kept shelf has the shape this screen restores. A snapshot lives
	 * in `sessionStorage` for as long as the tab does, so one written by an
	 * older Kamosu can be handed back after an update; one that no longer fits
	 * is dropped, and the shelf starts whole instead of holding nonsense.
	 */
	/**
	 * Every Cookbook some Kitchens see, once each however many of them see it,
	 * the reader's own first: `list_kitchens` answers each Kitchen's Cookbooks
	 * with the reader's own at the head.
	 */
	function cookbooksSeenIn(kitchens: ListKitchensOutput['kitchens']) {
		const seen = new Map(
			kitchens.flatMap((kitchen) => kitchen.cookbooks).map((cookbook) => [cookbook.id, cookbook]),
		);
		return [...seen.values()];
	}

	function isLeftShelf(held: unknown): held is LeftShelf {
		if (typeof held !== 'object' || held === null) return false;
		const { typed, filter, cookedOrWritten, tag, scrolled } = held as Record<string, unknown>;
		if (typeof filter !== 'object' || filter === null) return false;
		const { kind, id } = filter as Record<string, unknown>;
		return (
			typeof typed === 'string' &&
			typeof cookedOrWritten === 'boolean' &&
			(tag === null || typeof tag === 'string') &&
			typeof scrolled === 'number' &&
			(kind === 'all' || ((kind === 'kitchen' || kind === 'cookbook') && typeof id === 'string'))
		);
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { MeaningSearch } from '$lib/meaning.svelte';
	import { RecipeSearch } from '$lib/search.svelte';
	import { byWord, sameWord, tagWord, type Tag } from '$lib/tags';
	import { cookbookOnChip, kitchenName } from '$lib/cookbook';
	import Screen from '$lib/shell/Screen.svelte';
	import { recipeSearchBox } from '$lib/shell/slash';
	import Empty from '$lib/shell/Empty.svelte';
	import Tile from './Tile.svelte';
	import AddOrImport from '$lib/AddOrImport.svelte';
	import Plus from './Plus.svelte';
	import MeaningOffer from './MeaningOffer.svelte';

	interface Props {
		/** The Tag named by `?tag=`, where the reader arrived from a chip (#104). */
		tag?: string | null;
	}

	/** Read once, on purpose: see `tag` below. */
	const { tag: arrivedTagged = null }: Props = $props();

	const kamosu = useKamosu();

	/** What is typed, moment to moment. The Operation is asked on a short delay. */
	let typed = $state('');
	/** Whose recipes are showing: everyone's, one Kitchen's or one Cookbook's. */
	let filter = $state<LeftShelf['filter']>({ kind: 'all' });
	/** Whether the shelf is down to what this Person created, branched or cooked (ADR 0027). */
	let cookedOrWritten = $state(false);
	/**
	 * The Tag this shelf is narrowed to, or nothing (#104). Held exactly as
	 * the two above are: here, and nowhere else. It composes with both: the
	 * recipes Marc tagged *spicy*, searched for *chicken*, is one ask.
	 *
	 * Seeded once from the route's `?tag=`, because a chip on a recipe page
	 * arrives by navigating. The query string is how that screen says which
	 * tag, not a place this filter lives: coming back to Recipes by the tab
	 * destroys this component and makes a new one on the whole shelf, exactly as
	 * the other two filters already behave.
	 */
	let tag = $state<string | null>(untrack(() => arrivedTagged));

	let kitchens = $state<ListKitchensOutput['kitchens']>([]);
	/**
	 * Every word the Cookbooks on this shelf file by, one entry per word
	 * however many of them use it (#131, question 3): "Dessert" from two
	 * Cookbooks is one chip, and filtering by it finds what each filed there.
	 * Every Kitchen's own list is read beside it, once, so a filter tap swaps
	 * one row of chips for another already here instead of putting a round
	 * trip between the thumb and the row.
	 */
	let tags = $state<Tag[]>([]);
	/**
	 * Each Kitchen's own words, by Kitchen id: what its Cookbooks file by, one
	 * entry per word. Read with the rest rather than narrowed from `tags`,
	 * because `tags` keeps ONE Tag per word and that Tag may belong to a
	 * Cookbook the Kitchen does not see, which would hide a word the Kitchen
	 * really files by.
	 */
	let kitchenTags = $state<Record<string, Tag[]>>({});
	/** Each Cookbook's own words, by Cookbook id, read the same way and for the same reason (#231). */
	let cookbookTags = $state<Record<string, Tag[]>>({});
	/**
	 * Whether this Kamosu matches on meaning, and whether this reader is the
	 * one to be asked about it. Both come from the Operation rather than being
	 * worked out here: only an Operator can turn it on, and it is never offered
	 * twice to somebody who declined (ADR 0029).
	 */
	const meaning = new MeaningSearch(kamosu);

	/**
	 * A filter naming a Kitchen only appears where there is more than one to
	 * name. With a single Kitchen it would filter nothing, and a control that
	 * does nothing teaches people that controls do nothing.
	 */
	const several = $derived(kitchens.length > 1);
	/** Every Cookbook the reader sees, their own first. */
	const cookbooks = $derived(cookbooksSeenIn(kitchens));
	/** The reader's own, which its chip calls *Mine*. */
	const ownCookbook = $derived(cookbooks[0]?.id ?? null);
	/**
	 * A line of Cookbooks only appears where the reader sees more than their
	 * own, for the reason a single Kitchen gets no line.
	 */
	const severalCookbooks = $derived(cookbooks.length > 1);
	/**
	 * Whether the Kitchens and their words have been asked about, answered or
	 * not. The rows of chips above the list are drawn from them, so the list
	 * only stops moving down the page once this is true.
	 */
	let chipsSettled = $state(false);

	$effect(() => {
		let current = true;
		kamosu
			.listKitchens({})
			.then(async (held) => {
				if (!current) return;
				kitchens = held.kitchens;
				// Every word of every Cookbook on the shelf, merged by the Core
				// into one entry per word — the same merge the shelf itself is
				// (ADR 0027). Failing costs only the row, never the shelf.
				const read = (
					input: { everywhere: true } | { kitchen_id: string } | { cookbook_id: string },
				) =>
					kamosu.listTags(input).catch((error: unknown) => {
						if (!(error instanceof OperationError)) throw error;
						return { tags: [] };
					});
				// Not `cookbooks`: that follows `kitchens`, and reading it here
				// would run this again on the answer it has just been given.
				const seen = cookbooksSeenIn(held.kitchens);
				const [words, perKitchen, perCookbook] = await Promise.all([
					read({ everywhere: true }),
					Promise.all(held.kitchens.map((kitchen) => read({ kitchen_id: kitchen.id }))),
					// One Cookbook is no line of chips, so its words are never asked for.
					Promise.all(seen.length > 1 ? seen.map(({ id }) => read({ cookbook_id: id })) : []),
				]);
				if (current) {
					tags = words.tags;
					kitchenTags = Object.fromEntries(
						held.kitchens.map((kitchen, index) => [kitchen.id, perKitchen[index]?.tags ?? []]),
					);
					cookbookTags = Object.fromEntries(
						seen.map((cookbook, index) => [cookbook.id, perCookbook[index]?.tags ?? []]),
					);
				}
			})
			.catch((error: unknown) => {
				// The shelf itself is what this screen is for; failing to learn
				// the Kitchens costs only the filters, so it is not an error
				// the whole screen wears.
				if (!(error instanceof OperationError)) throw error;
			})
			.finally(() => {
				if (current) chipsSettled = true;
			});
		return () => {
			current = false;
		};
	});

	/**
	 * A Kitchen or a Cookbook held with no chip to let it go, which only going
	 * back can produce (#191): the reader has left that Kitchen since, or its
	 * line is gone. It falls back to the whole shelf, as the Tag filter below
	 * does for the same reason.
	 */
	$effect(() => {
		const held = filter;
		if (held.kind === 'all' || !chipsSettled) return;
		const offered =
			held.kind === 'kitchen'
				? several && kitchens.some((kitchen) => kitchen.id === held.id)
				: severalCookbooks && cookbooks.some((cookbook) => cookbook.id === held.id);
		if (!offered) filter = { kind: 'all' };
	});

	/** Hold a Kitchen or a Cookbook, or let it go where it is the one held. */
	function hold(kind: 'kitchen' | 'cookbook', id: string) {
		filter = filter.kind === kind && filter.id === id ? { kind: 'all' } : { kind, id };
	}

	/**
	 * The Tags to offer: those of the Cookbook being filtered to, those of
	 * the Cookbooks seen in the Kitchen being filtered to, or all of them.
	 * Narrowed HERE rather than by asking again, so the row follows a chip
	 * the moment it is tapped — a chip that can only ever find nothing is
	 * worse than no chip.
	 */
	const chips = $derived.by(() => {
		// Read into a local first, as the search below does: a union held
		// in state does not stay narrowed across the closure that reads it.
		const held = filter;
		if (held.kind === 'kitchen') return byWord(kitchenTags[held.id] ?? []);
		if (held.kind === 'cookbook') return byWord(cookbookTags[held.id] ?? []);
		return byWord(tags);
	});

	/** The Tag being filtered by, once the list holding it has been read. */
	const filtering = $derived(
		[tags, ...Object.values(kitchenTags), ...Object.values(cookbookTags)]
			.flat()
			.find((held) => held.id === tag) ?? null,
	);
	/** Its word, for the count and the nothing-found line. */
	const filteringWord = $derived(filtering ? tagWord(filtering).name : null);

	/**
	 * A Tag chip that goes out of reach keeps its filter on with no way to turn
	 * it off — which is what happens when a Kitchen or a Cookbook chip narrows
	 * the row past the Tag being filtered by. So the filter follows the row: taking the chip
	 * away takes the narrowing with it.
	 */
	$effect(() => {
		if (tag === null || tags.length === 0 || chips.some((held) => held.id === tag)) return;
		// The same word in the new row is the same filter under that row's
		// own id: each Cookbook keeps its own Tag for a word, and the Core
		// matches a Tag by its word across the Cookbooks on the shelf (#231).
		const word = filtering;
		tag = (word && chips.find((held) => sameWord(held, word))?.id) || null;
	});

	// Asked on open, and again whenever turning Meaning Search on or declining
	// it changed the answer — so the offer disappears the moment it is answered.
	$effect(() => meaning.ask());

	/**
	 * The shelf's search: whatever is typed, under whichever filters are held.
	 * Turning Meaning Search on changes what this same search finds, so it is
	 * asked again — which is the only confirmation worth giving: the recipe you
	 * were looking for appears.
	 */
	const search = new RecipeSearch(
		kamosu,
		() => {
			const query = typed.trim();
			// Read into a local first: a union held in state does not stay
			// narrowed across the property reads below.
			const asked = filter;
			return {
				query: query === '' ? null : query,
				kitchen_id: asked.kind === 'kitchen' ? asked.id : null,
				cookbook_id: asked.kind === 'cookbook' ? asked.id : null,
				mine: cookedOrWritten,
				tag_id: tag,
			};
		},
		() => meaning.generation,
	);
	const answer = $derived(search.answer);
	const failed = $derived(search.failed);

	const entries = $derived(answer?.recipes ?? []);
	/** What the answer was actually for — never what the field says now. */
	const query = $derived(answer?.query ?? null);
	/**
	 * Whether these are matches or merely the nearest there were. Meaning
	 * Search always has a nearest neighbour, so *nothing found* means *nothing
	 * close enough* — and showing the closest under that label is not the same
	 * act as letting them pass as matches (ADR 0027).
	 */
	const closest = $derived(answer?.closest ?? false);
	/**
	 * What was searched for and not found, or `null` where this screen is
	 * showing a shelf rather than *nothing found*. Carries the query rather
	 * than a flag because the block below needs both facts at once: that
	 * nothing matched, and what did not match.
	 */
	const unmatched = $derived(query !== null && (entries.length === 0 || closest) ? query : null);

	/**
	 * How many recipes are showing, in words, or `null` where the answer is
	 * not a list to count: still loading, failed, or nothing found.
	 */
	const count = $derived.by(() => {
		if (failed || !answer || unmatched !== null || entries.length === 0) return null;
		const one = entries.length === 1;
		// Counted by the Tag rather than by the shelf, because that is the
		// question the reader just asked. It counts what is on this answer, so
		// it is the Tag's own number only while no other filter is narrowing it
		// further — Settings counts a Tag across its whole Kitchen and says so
		// there.
		if (filteringWord && query === null) {
			return one
				? m.tags_shelf_count_one({ tag: filteringWord })
				: m.tags_shelf_count({ count: entries.length, tag: filteringWord });
		}
		if (query === null)
			return one ? m.recipes_count_one() : m.recipes_count({ count: entries.length });
		return one ? m.recipes_found_one() : m.recipes_found({ count: entries.length });
	});

	/** The shelf as the reader is leaving it: the route hands this to SvelteKit (#191). */
	export function capture(): LeftShelf {
		return {
			typed,
			filter: $state.snapshot(filter),
			cookedOrWritten,
			tag,
			scrolled: window.scrollY,
		};
	}

	/**
	 * Where to scroll once the list is back, and the answer that was showing
	 * when the shelf was restored. Not `$state`: the effect below follows the
	 * answer, and setting this must not set it off.
	 */
	let scrollBack: { to: number; over: typeof answer } | null = null;

	/**
	 * Going back to the shelf as it was left (#191). SvelteKit calls this just
	 * after building the screen, which has already started asking for the whole
	 * shelf; the held values change the ask, and the search drops the answer to
	 * the old one. Anything that is not a kept shelf is ignored.
	 */
	export function restore(left: unknown): void {
		if (!isLeftShelf(left)) return;
		typed = left.typed;
		filter = left.filter;
		cookedOrWritten = left.cookedOrWritten;
		tag = left.tag;
		scrollBack = { to: left.scrolled, over: answer };
	}

	/**
	 * SvelteKit puts the page back where it was as soon as the screen is built,
	 * but the list is not there yet: the page is one "Loading…" tall and the
	 * browser clamps the scroll to the top. So the shelf scrolls itself, once,
	 * when the first answer after the restore has drawn the list and the chip
	 * rows above it have settled, since a row arriving later would push the
	 * list down under the reader. A failed search cancels it: scrolling on some
	 * later answer, to words typed a minute afterwards, would be a jump nobody
	 * asked for.
	 */
	$effect(() => {
		// All three read before anything returns: Svelte runs this again only
		// for what it saw read, and `scrollBack` is set while this is waiting.
		const arrived = answer;
		const refused = failed;
		const settled = chipsSettled;
		if (scrollBack === null) return;
		if (refused) {
			scrollBack = null;
			return;
		}
		if (!settled || arrived === undefined || arrived === scrollBack.over) return;
		const to = scrollBack.to;
		scrollBack = null;
		window.scrollTo(0, to);
	});
</script>

<Screen title={m.recipes_title()} blurb={m.recipes_blurb()} fills>
	<search>
		<!--
			The + for every new recipe sits at the end of the search box (#174),
			there with the shelf, while a search runs, and when it finds nothing.

			The box stops at the phone's column on the wide layout (#201), where
			the grid under it runs on to the window's edge.
		-->
		<div class="max-w-2xl">
			<Plus>
				<label class="block">
					<span class="sr-only">{m.recipes_search()}</span>
					<input
						type="search"
						bind:value={typed}
						{@attach recipeSearchBox}
						placeholder={m.recipes_search()}
						class="min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body"
					/>
				</label>
			</Plus>
		</div>

		<!--
			Whose recipes: a line of Cookbooks and a line of Kitchens, each under
			its heading and each drawn as the Tags line below is (#231). Either
			is absent where it would hold one chip, which would filter nothing.
		-->
		{#snippet line(
			title: string,
			named: { id: string; name: string; held: boolean }[],
			choose: (id: string) => void,
		)}
			<h2 class="mt-4 text-label text-ink-2 uppercase">{title}</h2>
			<div
				class="relative -mx-gutter mt-2 flex [scrollbar-width:none] flex-nowrap gap-2 overflow-x-auto px-gutter wide:mx-0 wide:flex-wrap wide:overflow-visible wide:px-0"
			>
				{#each named as chip (chip.id)}
					<button
						type="button"
						aria-pressed={chip.held}
						onclick={() => choose(chip.id)}
						class="min-h-8 shrink-0 rounded-sm border px-3 text-read {chip.held
							? 'border-accent bg-accent text-on-accent'
							: 'border-rule text-ink-2'}"
					>
						{chip.name}
					</button>
				{/each}
			</div>
		{/snippet}

		{#if severalCookbooks}
			{@render line(
				m.recipes_cookbooks_title(),
				cookbooks.map((cookbook) => ({
					id: cookbook.id,
					name: cookbook.id === ownCookbook ? m.recipes_cookbook_mine() : cookbookOnChip(cookbook),
					held: filter.kind === 'cookbook' && filter.id === cookbook.id,
				})),
				(id) => hold('cookbook', id),
			)}
		{/if}
		{#if several}
			{@render line(
				m.recipes_kitchens_title(),
				kitchens.map((kitchen) => ({
					id: kitchen.id,
					name: kitchenName(kitchen),
					held: filter.kind === 'kitchen' && filter.id === kitchen.id,
				})),
				(id) => hold('kitchen', id),
			)}
		{/if}

		<!--
			The tags, as ONE SIDEWAYS LINE and never a block that wraps (#104).
			Twenty-four tags wrapped is six rows of chips standing between the
			search field and the recipes, which is the measurement that decided
			this; Home already settled that a section carries a rail rather than
			a grid. Full-bleed, so the line visibly runs off the edge — that is
			what says there is more of it.

			Absent entirely until a Kitchen has a tag: on a new instance, and on
			this project's own 86-recipe library the day it was imported, this
			row would otherwise be a heading over nothing.
		-->
		{#if chips.length > 0}
			<h2 class="mt-4 text-label text-ink-2 uppercase">{m.tags_title()}</h2>
			<!-- `relative` holds each chip's screen-reader words inside this row.
			     `.sr-only` is positioned absolutely, and with nothing positioned
			     above it that means against the page: a chip scrolled off to the
			     right pushed the whole shelf sideways, 615px wide on a 390px phone,
			     as soon as a Tag carried a Language mark (#112). -->
			<div
				class="relative -mx-gutter mt-2 flex [scrollbar-width:none] flex-nowrap gap-2 overflow-x-auto px-gutter wide:mx-0 wide:flex-wrap wide:overflow-visible wide:px-0"
			>
				{#each chips as held (held.id)}
					{@const word = tagWord(held)}
					<button
						type="button"
						aria-pressed={tag === held.id}
						onclick={() => (tag = tag === held.id ? null : held.id)}
						class="min-h-8 shrink-0 rounded-sm border px-3 text-read {tag === held.id
							? 'border-accent bg-accent text-on-accent'
							: 'border-rule text-ink-2'}"
					>
						{word.name}
						{#if word.elsewhere}
							<span class="text-label uppercase {tag === held.id ? '' : 'text-support'}">
								<span aria-hidden="true">{word.elsewhere}</span>
								<span class="sr-only">{word.said}</span>
							</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</search>

	<!--
		The count, and the tick beside it (#231). The row is drawn whatever the
		answer, so a tick that narrowed the shelf to nothing is still there to
		take off. The count is the answer's and only stands where there is one.
	-->
	<div class="mt-4 mb-3 flex min-h-8 items-center gap-6">
		{#if count !== null}
			<p class="text-label text-ink-2 uppercase" role="status">{count}</p>
		{/if}
		<!-- A checkbox drawn as the chosen page drew it: a square that fills
		     with the accent, and words that darken with it. -->
		<label
			class="ml-auto flex min-h-8 items-center gap-2 text-read wide:ml-0 {cookedOrWritten
				? 'text-ink'
				: 'text-ink-2'}"
		>
			<span class="grid h-4 w-4 place-items-center">
				<input
					type="checkbox"
					bind:checked={cookedOrWritten}
					class="col-start-1 row-start-1 h-4 w-4 appearance-none rounded-sm border {cookedOrWritten
						? 'border-accent bg-accent'
						: 'border-ink-2'}"
				/>
				{#if cookedOrWritten}
					<span
						aria-hidden="true"
						class="pointer-events-none col-start-1 row-start-1 text-label leading-none text-on-accent"
						>✓</span
					>
				{/if}
			</span>
			{m.recipes_history()}
		</label>
	</div>

	{#if failed}
		<p class="text-body text-support" role="alert">{m.recipes_failed()}</p>
	{:else if !answer}
		<p class="text-body text-ink-2">{m.loading()}</p>
	{:else if unmatched !== null}
		<!--
			Nothing found is not an empty screen. It says what was looked
			through, and offers the two things you were about to do anyway —
			because with a library this size a search that finds nothing usually
			means you do not have it yet (ADR 0027).
		-->
		<div class="max-w-2xl">
			<h2 class="font-display text-shelf-heading font-semibold">
				{m.recipes_nothing_title({ query: unmatched })}
			</h2>
			<p class="mt-2 text-read text-ink-2">
				{meaning.status?.on ? m.recipes_nothing_why_meaning() : m.recipes_nothing_why()}
			</p>
			<div class="mt-4">
				<AddOrImport title={unmatched} />
			</div>
			<!--
				Offered here rather than buried in settings: this is the moment a
				person can see exactly what they are missing (ADR 0029). Whether
				to make the offer at all is the Operation's answer, not this
				screen's — it is never made to somebody who cannot act on it,
				and never again to an Operator who declined.
			-->
			{#if meaning.status?.offer}
				<MeaningOffer {meaning} status={meaning.status} />
			{/if}
		</div>

		{#if closest}
			<!--
				The closest anyway, under exactly that label. Meaning-matching
				always has a nearest neighbour, so an empty screen would be a
				lie about what Kamosu knows — and an unlabelled list would be a
				worse one, letting a weak match pass as a good one (ADR 0027).
			-->
			<p class="mt-6 mb-3 text-label text-ink-2 uppercase" role="status">
				{m.recipes_closest()}
			</p>
			<ul class="tile-grid">
				{#each entries as entry (entry.lineage_id)}
					<Tile {entry} />
				{/each}
			</ul>
		{/if}
	{:else if entries.length === 0}
		<!--
			A tag narrowing the shelf to nothing is not an empty library, and
			saying "you have no recipes" there would be a lie about the other
			eighty-five. It says which word found nothing and offers the way
			back out, which is the chip that is still lit above.
		-->
		<Empty>
			{#if filteringWord}
				{m.tags_shelf_none({ tag: filteringWord })}
			{:else if cookedOrWritten || filter.kind !== 'all'}
				<!-- The same lie avoided for the other filters (#231): the
				     library is still there behind whatever is held. -->
				{m.recipes_none_held()}
			{:else}
				{m.recipes_empty()}
			{/if}
		</Empty>
	{:else}
		<ul class="tile-grid">
			{#each entries as entry (entry.lineage_id)}
				<Tile {entry} />
			{/each}
		</ul>
	{/if}
</Screen>
