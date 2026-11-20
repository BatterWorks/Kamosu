<!--
	Recipes: one shelf, and the one home for searching (#62, ADR 0027).

	Everything the Kitchens you cook in hold, merged, alphabetical, one card per
	Lineage. There is no Kitchen switcher and no modes — a switcher is filing
	through the back door, and it taxes every search with a permissions question
	you should never have to answer to find the katsu curry on a Tuesday.

	The filters are **held here and nowhere else**, which is the whole of
	how "no filter sticks" is built: leaving the screen destroys this component,
	and coming back makes a new one on the whole shelf.

	They answer different questions (#231, #445). *Whose recipes* is one
	Cookbook or one Kitchen, held one at a time, since a Cookbook holds the same
	recipes in every Kitchen that sees it. *How it was cooked* is any of five
	choices at once. A Tag is the word a recipe is filed by. Each group is one
	button on a single row under the search box, which opens its list
	(`FilterList`): the screen is about its recipes, and four headed lines of
	chips stood between them and the search box. What is held stands under the
	buttons, on a line of its own, as a chip whose × lets it go in one tap
	(#650). *Only ones I wrote
	or cooked* is about the reader and the recipe, whoever's Cookbook it is in,
	so it is a tick beside the count and it combines with everything else.
	Beside a Cooking choice it says whose cookings judge it: the reader's alone.

	**Finding the way down a long shelf** (#650, option A, chosen 20 November
	2026). The count says the order the shelf is in, the search box stays at
	the top of the window while the list scrolls, and the letters A to Z down
	the right edge take the list to any part of it (`Letters`). *Add a recipe*
	stands beside the title, where the unlabelled + beside the search box was
	read as a search button (R4), and opens the one sheet every way in is on
	(`AddRecipe`).

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
	import type { ListKitchensOutput, SearchRecipesInput } from '#lib/api/catalogue.js';

	/** One way a recipe has been cooked, as the shelf filters by it (#445). */
	export type Cooking = NonNullable<SearchRecipesInput['cooking']>[number];
	/**
	 * The five, in the order the list offers them. Written as the keys of a
	 * record over the Catalogue's own type, so a sixth added there fails the
	 * build here until it is placed.
	 */
	const COOKING = Object.keys({
		never_cooked: true,
		cooked_unrated: true,
		again: true,
		tweak: true,
		no: true,
	} satisfies Record<Cooking, true>) as Cooking[];

	/** One button of the row under the search box, and the list it opens (#445). */
	type Group = 'cookbooks' | 'kitchens' | 'cooking' | 'tags';

	/**
	 * The shelf as it was left, for going back to it (#191). SvelteKit keeps it
	 * in `sessionStorage` as text, so it holds plain values only and is
	 * deliberately small: what the
	 * reader chose and where they had scrolled, never the answer. The answer is
	 * asked again, from the phone's copy first, so it is as current as any.
	 */
	export interface LeftShelf {
		typed: string;
		/** Whose recipes: everyone's, one Kitchen's or one Cookbook's (#231). */
		filter: { kind: 'all' } | { kind: 'kitchen'; id: string } | { kind: 'cookbook'; id: string };
		/** Only what the reader cooked or wrote, on top of whatever else is held (#231). */
		cookedOrWritten: boolean;
		/** How it was cooked, any of these (#445). Absent from a shelf kept by an older Kamosu. */
		cooking?: Cooking[];
		tag: string | null;
		/** How far down the page was, in pixels. */
		scrolled: number;
	}

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

	/**
	 * Whether a kept shelf has the shape this screen restores. A snapshot lives
	 * in `sessionStorage` for as long as the tab does, so one written by an
	 * older Kamosu can be handed back after an update; one that no longer fits
	 * is dropped, and the shelf starts whole instead of holding nonsense.
	 */
	function isLeftShelf(held: unknown): held is LeftShelf {
		if (typeof held !== 'object' || held === null) return false;
		const { typed, filter, cookedOrWritten, cooking, tag, scrolled } = held as Record<
			string,
			unknown
		>;
		if (typeof filter !== 'object' || filter === null) return false;
		const { kind, id } = filter as Record<string, unknown>;
		return (
			typeof typed === 'string' &&
			typeof cookedOrWritten === 'boolean' &&
			(cooking === undefined ||
				(Array.isArray(cooking) && cooking.every((each) => COOKING.includes(each as Cooking)))) &&
			(tag === null || typeof tag === 'string') &&
			typeof scrolled === 'number' &&
			(kind === 'all' || ((kind === 'kitchen' || kind === 'cookbook') && typeof id === 'string'))
		);
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import { MeaningSearch } from '#lib/meaning.svelte.js';
	import { RecipeSearch } from '#lib/search.svelte.js';
	import { byWord, sameWord, tagWord, type Tag } from '#lib/tags.js';
	import { cookbookOnChip, kitchenName } from '#lib/cookbook.js';
	import { ratingVerdict } from '#lib/rating.js';
	import FilterList, { type Choice } from './FilterList.svelte';
	import Screen from '#lib/shell/Screen.svelte';
	import { recipeSearchBox } from '#lib/shell/slash.js';
	import Empty from '#lib/shell/Empty.svelte';
	import Tile from './Tile.svelte';
	import AddOrImport from '#lib/AddOrImport.svelte';
	import AddRecipe from '#lib/adding/AddRecipe.svelte';
	import AddButton from '#lib/adding/AddButton.svelte';
	import { useAdding } from '#lib/adding/adding.svelte.js';
	import DropARecipe from './DropARecipe.svelte';
	import { Online } from '#lib/offline/device.svelte.js';
	import NoticeCard from '#lib/NoticeCard.svelte';
	import { takeTakenOut } from '#lib/adding/added.js';
	import MeaningOffer from './MeaningOffer.svelte';
	import Letters from './Letters.svelte';
	import { firstOfEach, type Letter } from './letters';

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
	/** How the shown recipes were cooked: any of these, or nothing held (#445). */
	let cooking = $state<Cooking[]>([]);
	/** The group whose list is open over the shelf, if one is. */
	let open = $state<Group | null>(null);
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

	/** A Cookbook as its button and its list name it: *Mine*, or whose it is. */
	function cookbookName(cookbook: (typeof cookbooks)[number]): string {
		return cookbook.id === ownCookbook ? m.recipes_cookbook_mine() : cookbookOnChip(cookbook);
	}

	/** One Cooking choice in the reader's words; the three ratings as the cooking log says them (#650). */
	function cookingName(choice: Cooking): string {
		switch (choice) {
			case 'never_cooked':
				return m.recipes_cooking_never();
			case 'cooked_unrated':
				return m.recipes_cooking_unrated();
			default:
				return ratingVerdict(choice);
		}
	}

	/**
	 * The row under the search box, one button per group with something to
	 * choose (#445). What each holds is said by its chip (`held`), not on it.
	 */
	const groups = $derived.by(() => {
		const row: { key: Group; title: string }[] = [];
		if (severalCookbooks) row.push({ key: 'cookbooks', title: m.recipes_cookbooks_title() });
		if (several) row.push({ key: 'kitchens', title: m.recipes_kitchens_title() });
		row.push({ key: 'cooking', title: m.recipes_cooking_title() });
		if (chips.length > 0) row.push({ key: 'tags', title: m.tags_title() });
		return row;
	});

	/**
	 * Every filter that is on, as a chip under the buttons whose × lets it go
	 * in one tap (#650, T4): a Cookbook or a Kitchen, each Cooking choice, a
	 * Tag. A button names its group and nothing else, so what narrows the
	 * shelf is said once, on the chip that takes it off.
	 */
	const held = $derived.by(() => {
		const on: { key: string; name: string; off: () => void }[] = [];
		const whose = filter;
		if (whose.kind === 'cookbook') {
			const cookbook = cookbooks.find(({ id }) => id === whose.id);
			if (cookbook)
				on.push({
					key: 'cookbook',
					name: cookbookName(cookbook),
					off: () => (filter = { kind: 'all' }),
				});
		} else if (whose.kind === 'kitchen') {
			const kitchen = kitchens.find(({ id }) => id === whose.id);
			if (kitchen)
				on.push({
					key: 'kitchen',
					name: kitchenName(kitchen),
					off: () => (filter = { kind: 'all' }),
				});
		}
		for (const choice of COOKING.filter((each) => cooking.includes(each)))
			on.push({
				key: `cooking-${choice}`,
				name: cookingName(choice),
				off: () => (cooking = cooking.filter((each) => each !== choice)),
			});
		if (tag !== null && filteringWord)
			on.push({ key: 'tag', name: filteringWord, off: () => (tag = null) });
		return on;
	});

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
				// Sent only when held, so the ask with nothing held is the one
				// the phone kept answers for before Cooking existed.
				...(cooking.length > 0 ? { cooking: $state.snapshot(cooking) } : {}),
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
			cooking: $state.snapshot(cooking),
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
		cooking = COOKING.filter((each) => left.cooking?.includes(each));
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

	/**
	 * Adding a recipe, from the sheet beside the title, a file or a link
	 * dropped on the screen (#204), or a search that found nothing (#650).
	 */
	const adding = useAdding();
	const online = new Online();
	let addRecipe = $state<AddRecipe | undefined>(undefined);

	/** A recipe just taken back out by the Undo on its page, said once here (#650). */
	const takenOut = takeTakenOut();

	/**
	 * Where each letter starts on the shelf, for the letters down the edge
	 * (#650). Only the whole shelf in its A to Z order has them, never a
	 * search's results, and only once it is longer than a few screens' worth:
	 * twelve cards are two flicks of a thumb.
	 */
	const starts = $derived(
		query === null && unmatched === null && entries.length > 12
			? firstOfEach(entries.map((entry) => entry.title))
			: null,
	);
	const startsAt = $derived(
		new Map([...(starts ?? new Map<Letter, number>())].map(([letter, at]) => [at, letter])),
	);

	/** The list at a letter's first recipe, under the search box pinned over it. */
	function jump(letter: Letter) {
		document.getElementById(`letter-${letter}`)?.scrollIntoView({ block: 'start' });
	}
</script>

<!--
	The screen's column keeps clear of the letters down its edge on the phone,
	so no button or line sits under a letter, at 360 wide too (U15): the strip
	is 26 wide, and 4 more keep it off what is beside it. Kept whether the
	letters show or not, so the page never moves sideways under the cook's
	finger as a search answers.
-->
<Screen title={m.recipes_title()} fills class="pr-[30px] wide:pr-gutter">
	{#snippet aside()}
		<AddButton onclick={() => addRecipe?.open()} />
	{/snippet}

	<!--
		Adding is one sheet, opened from the title's line (#650). What a picked
		or dropped file is doing is said here, under it.
	-->
	<AddRecipe {adding} bind:this={addRecipe} opensOnAddQuery />
	<DropARecipe {adding} offline={!online.current} ondrop={() => addRecipe?.close()} />

	{#if takenOut}
		<NoticeCard class="mb-4">{m.added_taken_out()}</NoticeCard>
	{/if}

	<!--
		The search box stays at the top of the window while the list scrolls
		(#650, R7), so a cook deep in the S's can search without the long way
		back up, below the phone's own status bar where the app is installed. It
		is a child of the screen's whole column, since a sticky box sticks only
		while its parent is in view. It stops at the phone's column on the wide
		layout (#201), where the grid under it runs on to the window's edge.
	-->
	<search
		class="sticky top-[env(safe-area-inset-top)] z-10 -mx-gutter bg-ground px-gutter py-2 wide:mx-0 wide:px-0"
	>
		<div>
			<label class="relative block max-w-2xl">
				<span class="sr-only">{m.recipes_search()}</span>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="pointer-events-none absolute top-1/2 left-3 h-[22px] w-[22px] -translate-y-1/2 text-ink-2"
					fill="none"
					stroke="currentColor"
					stroke-width="1.75"
					stroke-linecap="round"><circle cx="11" cy="11" r="6.5" /><path d="m16 16 4.5 4.5" /></svg
				>
				<input
					type="search"
					bind:value={typed}
					{@attach recipeSearchBox}
					placeholder={m.recipes_search()}
					class="min-h-12 w-full rounded-sm border border-ink-2 bg-card pr-[44px] pl-[44px] text-body text-ink placeholder:text-ink-2 [&::-webkit-search-cancel-button]:hidden"
				/>
				{#if typed !== ''}
					<button
						type="button"
						aria-label={m.recipes_search_clear()}
						class="absolute top-0 right-0 flex h-12 w-12 items-center justify-center text-ink-2"
						onclick={(event) => {
							typed = '';
							(event.currentTarget.previousElementSibling as HTMLInputElement | null)?.focus();
						}}
					>
						<svg
							viewBox="0 0 24 24"
							aria-hidden="true"
							class="h-[18px] w-[18px]"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
						>
					</button>
				{/if}
			</label>
		</div>
	</search>

	<!--
		Every filter behind one row of buttons, one per group (#445), each a
		quiet button drawn at 40 and tapped at 48, 8 apart (#650, R3). A group
		with nothing to choose has no button, for the reason it had no line: a
		control that does nothing teaches people that controls do nothing.
		Cooking always has its five.
	-->
	<div
		class="relative mt-3 -ml-gutter flex [scrollbar-width:none] flex-nowrap gap-2 overflow-x-auto py-1 pl-gutter wide:ml-0 wide:flex-wrap wide:overflow-visible wide:pl-0"
	>
		{#each groups as group (group.key)}
			<button
				type="button"
				aria-haspopup="dialog"
				onclick={() => (open = group.key)}
				class="quiet-button shrink-0 gap-[6px] text-body text-ink"
			>
				{group.title}
				<svg
					class="h-[12px] w-[12px] shrink-0 text-ink-2"
					viewBox="0 0 12 12"
					fill="none"
					stroke="currentColor"
					stroke-width="1.6"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"><path d="M2.5 4.5 6 8l3.5-3.5" /></svg
				>
			</button>
		{/each}
	</div>

	<!--
		A filter that is on stands as an indigo chip on a line of its own under
		the buttons, whose × lets it go in one tap (#650, T4). Chosen over the
		chosen page's chip at the end of the buttons' row, which a phone scrolled
		off its edge, where a cook would not see a filter was on (U12). The line
		wraps, so every chip held is in view.
	-->
	{#if held.length > 0}
		<div class="mt-2 flex flex-wrap gap-x-2 gap-y-3 py-1">
			{#each held as on (on.key)}
				<button
					type="button"
					aria-label={m.recipes_filter_off({ name: on.name })}
					onclick={on.off}
					class="quiet-button shrink-0 gap-2 border-accent bg-accent text-body text-on-accent"
				>
					<span class="max-w-[60vw] truncate">{on.name}</span>
					<svg
						viewBox="0 0 24 24"
						aria-hidden="true"
						class="h-[18px] w-[18px] shrink-0"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
					>
				</button>
			{/each}
		</div>
	{/if}

	{#if open === 'cookbooks'}
		<FilterList
			title={m.recipes_cookbooks_title()}
			kind="one"
			choices={cookbooks.map((cookbook) => ({
				id: cookbook.id,
				name: cookbookName(cookbook),
				held: filter.kind === 'cookbook' && filter.id === cookbook.id,
			}))}
			onpick={(id) => hold('cookbook', id)}
			onclear={() => {
				if (filter.kind === 'cookbook') filter = { kind: 'all' };
			}}
			onclose={() => (open = null)}
		/>
	{:else if open === 'kitchens'}
		<FilterList
			title={m.recipes_kitchens_title()}
			kind="one"
			choices={kitchens.map((kitchen) => ({
				id: kitchen.id,
				name: kitchenName(kitchen),
				held: filter.kind === 'kitchen' && filter.id === kitchen.id,
			}))}
			onpick={(id) => hold('kitchen', id)}
			onclear={() => {
				if (filter.kind === 'kitchen') filter = { kind: 'all' };
			}}
			onclose={() => (open = null)}
		/>
	{:else if open === 'cooking'}
		<FilterList
			title={m.recipes_cooking_title()}
			kind="any"
			choices={COOKING.map((choice) => ({
				id: choice,
				name: cookingName(choice),
				held: cooking.includes(choice),
			}))}
			onpick={(id) => {
				// Held in the list's order, whatever order they were tapped in,
				// so one set of choices is one ask the phone keeps one answer for.
				const choice = id as Cooking;
				const now = cooking.includes(choice)
					? cooking.filter((held) => held !== choice)
					: [...cooking, choice];
				cooking = COOKING.filter((each) => now.includes(each));
			}}
			onclear={() => (cooking = [])}
			onclose={() => (open = null)}
		/>
	{:else if open === 'tags'}
		<FilterList
			title={m.tags_title()}
			kind="tags"
			choices={chips
				.toSorted((one, other) => other.recipes - one.recipes)
				.map((held): Choice => {
					const word = tagWord(held);
					return {
						id: held.id,
						name: word.name,
						held: tag === held.id,
						elsewhere:
							word.elsewhere && word.said ? { shown: word.elsewhere, said: word.said } : null,
						count: held.recipes,
					};
				})}
			onpick={(id) => (tag = tag === id ? null : id)}
			onclear={() => (tag = null)}
			onclose={() => (open = null)}
		/>
	{/if}

	<!--
		The count, and the tick beside it (#231). The row is drawn whatever the
		answer, so a tick that narrowed the shelf to nothing is still there to
		take off. The count is the answer's and only stands where there is one.
	-->
	<!-- The count says the order (#650, R7), and the tick reads at full size,
	     tapped at 48, its square 22 across. -->
	<div class="mt-2 mb-3 flex items-center gap-6">
		{#if count !== null}
			<p class="text-read text-ink" role="status">{count}</p>
		{/if}
		<!-- A checkbox drawn as the chosen page drew it: a square that fills
		     with the accent. -->
		<label class="ml-auto flex min-h-12 items-center gap-2 text-read text-ink wide:ml-0">
			<span class="grid h-[22px] w-[22px] shrink-0 place-items-center">
				<input
					type="checkbox"
					bind:checked={cookedOrWritten}
					class="col-start-1 row-start-1 h-[22px] w-[22px] appearance-none rounded-sm border {cookedOrWritten
						? 'border-accent bg-accent'
						: 'border-ink-2 bg-card'}"
				/>
				{#if cookedOrWritten}
					<span
						aria-hidden="true"
						class="pointer-events-none col-start-1 row-start-1 text-read leading-none text-on-accent"
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
			Nothing found is not an empty screen. It offers the things you were
			about to do anyway, because with a library this size a search that
			finds nothing usually means you do not have it yet (ADR 0027). What
			the search looks through is the search box's own hint (#650), so no
			sentence here says it again.
		-->
		<div class="max-w-2xl">
			<h2 class="font-display text-shelf-heading font-semibold">
				{m.recipes_nothing_title({ query: unmatched })}
			</h2>
			<div class="mt-4">
				<AddOrImport title={unmatched} onlink={() => addRecipe?.open('link')} />
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
					<Tile {entry} {query} />
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
			<!-- The Tag's own words only where the Tag alone narrows the shelf:
			     with a Cooking choice or a Cookbook held too, recipes tagged with
			     it may well exist, and "nothing is tagged" would be false (#445). -->
			{#if filteringWord && !cookedOrWritten && cooking.length === 0 && filter.kind === 'all'}
				{m.tags_shelf_none({ tag: filteringWord })}
			{:else if filteringWord || cookedOrWritten || cooking.length > 0 || filter.kind !== 'all'}
				<!-- The same lie avoided for the other filters (#231): the
				     library is still there behind whatever is held. -->
				{m.recipes_none_held()}
			{:else}
				{m.recipes_empty()}
			{/if}
		</Empty>
	{:else}
		<!-- Room on the right for the letters on the wide layout, where the column
		     keeps none (#650). -->
		<ul class={['tile-grid', starts && 'wide:pr-[36px]']}>
			{#each entries as entry, at (entry.lineage_id)}
				{@const letter = startsAt.get(at)}
				<Tile {entry} {query} anchor={letter ? `letter-${letter}` : undefined} />
			{/each}
		</ul>
		{#if starts}
			<Letters held={new Set(starts.keys())} onjump={jump} />
		{/if}
	{/if}
</Screen>
