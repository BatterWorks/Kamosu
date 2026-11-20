<!--
	Home: the computed shelves that answer *show me something* (#64, ADR 0011,
	ADR 0027).

	The first screen is a suggestion rather than a search box. Everything on it
	is worked out from recipes and cookings that already exist — *Recipes* is
	where you go when you know what you are looking for, and this is where you
	go when you do not.

	**The arrangement was chosen on 2026-08-30 from four full-screen
	options drawn over the real 86-recipe library; the reasoning is on #64.** It
	takes the labelled section from one option and the sideways rail from
	another. What the choice settles:

	- **A shelf is a section**, headed the way every other section in the app is
	  headed. Home is the library's own furniture rather than a second visual
	  system next to it.
	- **A section carries a rail, not a grid.** The card at the right edge is
	  cut in half on purpose: a peeking card is the only thing that says *there
	  is more this way* without a control saying it, and it is what puts two
	  whole shelves above the fold and all four within one flick.

	The cards themselves are #62's, unchanged — `Tile`, the same component the
	library's shelf draws, taking the same shape the same Operation family
	answers in. A recipe is one object across both screens, not two treatments
	of one.

	**Which shelves exist is the Core's answer, never this screen's.** An empty
	shelf never arrives, so there is no case here where a heading has to decide
	whether to draw itself — and an instance with nothing on its shelf sends no
	shelves at all, which is what lets the empty state be said once instead of
	five times.

	**Since #650 (option A, chosen 20 November 2026) Home also leads the way to
	the library.** *Add a recipe* stands beside the title, opening the same
	sheet Recipes' does (H1); under the title a line names who the cook cooks
	with and in which Kitchen, leading to that page (H7); and a search box
	takes the cook to Recipes with the caret in its search (H1). The subtitle
	that explained nothing went (H2), and so did the bare count beside each
	shelf's heading (H3). The cards are narrower on the phone, so the third on
	each rail shows by about half and the rail reads as one that scrolls (H4).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';
	import type { HomeShelvesOutput } from '#lib/api/catalogue.js';
	import Tile from './recipes/Tile.svelte';
	import AddRecipe from '#lib/adding/AddRecipe.svelte';
	import AddButton from '#lib/adding/AddButton.svelte';
	import { useAdding } from '#lib/adding/adding.svelte.js';
	import Section from '#lib/shell/Section.svelte';
	import Empty from '#lib/shell/Empty.svelte';
	import { searchRecipes } from '#lib/shell/slash.js';
	import { useKamosu } from '#lib/kamosu.js';
	import { OperationError } from '#lib/api/client.js';
	import { cookingWith } from '#lib/cookbook.js';
	import { demo } from '#lib/demo.svelte.js';
	import DemoWelcome from './DemoWelcome.svelte';

	interface Props {
		/** The answer `home_shelves` gave. Asked for by the route, so signing in and reading Home are one round trip. */
		home: HomeShelvesOutput;
		/** A question waiting on the reader, drawn above the shelves: a Cookbook join (#135). */
		asking?: Snippet;
	}

	let { home, asking }: Props = $props();

	const kamosu = useKamosu();
	const adding = useAdding();
	let addRecipe = $state<AddRecipe | undefined>(undefined);

	/**
	 * Who the reader cooks with, in words, or nothing while it is not known.
	 * Asked beside the shelves and never instead of them: a Kitchen that cannot
	 * be read just now, offline say, leaves Home standing without the line.
	 */
	let household = $state<string | null>(null);

	$effect(() => {
		let current = true;
		Promise.all([kamosu.getPerson({}), kamosu.listKitchens({})])
			.then(([me, { kitchens }]) => {
				if (current) household = cookingWith(me.person_id, kitchens);
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	type ShelfName = HomeShelvesOutput['shelves'][number]['name'];

	/** Each shelf's rail, for the earlier and later buttons a pointer gets (#223). */
	let rails = $state<Partial<Record<ShelfName, HTMLUListElement>>>({});

	/**
	 * Most of a window of tiles, so the last one seen is still in view after
	 * the move, and never less than one tile.
	 */
	function scroll(name: ShelfName, by: 1 | -1) {
		const rail = rails[name];
		if (rail)
			rail.scrollBy({ left: by * Math.max(rail.clientWidth - 80, 168), behavior: 'smooth' });
	}

	/**
	 * What each shelf is called, and the sentence under it. The sentence does
	 * the explaining so the heading can stay two words, with the longer phrase
	 * carrying the detail underneath.
	 *
	 * One entry per shelf rather than a map of headings beside a switch of
	 * sentences: the two always change together, and two lists keyed by the same
	 * five names are two chances to add a shelf to one of them.
	 *
	 * The 30 in *quick tonight* is the Core's, carried in the answer. Writing it
	 * into the phrase file instead would let the sentence a reader believes
	 * drift away from the line that actually chose the recipes underneath it.
	 */
	const shelf: Record<ShelfName, { heading: () => string; why: () => string }> = {
		recently_added: {
			heading: m.home_shelf_recently_added,
			why: m.home_shelf_recently_added_why,
		},
		cooked_most: {
			heading: m.home_shelf_cooked_most,
			why: m.home_shelf_cooked_most_why,
		},
		quick_tonight: {
			heading: m.home_shelf_quick_tonight,
			why: () => m.home_shelf_quick_tonight_why({ minutes: home.quick_tonight_minutes }),
		},
		never_cooked: {
			heading: m.home_shelf_never_cooked,
			why: m.home_shelf_never_cooked_why,
		},
		recently_opened: {
			heading: m.home_shelf_recently_opened,
			why: m.home_shelf_recently_opened_why,
		},
	};
</script>

<svelte:head>
	<title>{m.home_title()} · {m.app_name()}</title>
</svelte:head>

<!-- A Demo's welcome card, above the heading until it is put away (#407). -->
{#if demo.is && demo.welcome !== 'closed'}
	<div class="mx-auto max-w-2xl px-gutter pt-4 wide:max-w-none">
		<DemoWelcome />
	</div>
{/if}

<div class="pt-6 pb-tabbar">
	<div class="mx-auto max-w-2xl px-gutter wide:max-w-none">
		<div class="flex items-center justify-between gap-3">
			<h1 class="font-display text-title font-semibold">{m.home_title()}</h1>
			{#if home.shelves.length > 0}
				<!-- An empty Home has its own, as its main button, below. -->
				<AddButton onclick={() => addRecipe?.open()} />
			{/if}
		</div>
		<AddRecipe {adding} bind:this={addRecipe} />
		{#if household}
			<!-- Who the reader cooks with, leading to the page that says more (H7). -->
			<a
				href="/settings/kitchens"
				class="tap-tall mt-1 inline-block max-w-2xl text-body text-accent underline underline-offset-4"
				>{household}<span aria-hidden="true">&nbsp;›</span></a
			>
		{/if}
		{#if home.shelves.length > 0}
			<!--
				The search box Recipes holds, standing here too (H1). A tap on it,
				or typing into it, takes the cook to Recipes with the caret in that
				box, the way `/` does from any screen (ADR 0044): one search, with
				its list under it, rather than a second one here with none.
			-->
			<label class="relative mt-3 block max-w-2xl">
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
				<!-- Read-only, and left by a tap or a key: a field merely tabbed onto
				     or reached by a screen reader keeps the cook on Home. -->
				<input
					type="search"
					readonly
					placeholder={m.recipes_search()}
					onclick={searchRecipes}
					onkeydown={(event) => {
						if (event.key === 'Enter' || event.key.length === 1) {
							event.preventDefault();
							void searchRecipes();
						}
					}}
					class="min-h-12 w-full rounded-sm border border-ink-2 bg-card pr-[44px] pl-[44px] text-body text-ink placeholder:text-ink-2"
				/>
			</label>
		{/if}
		{#if asking}
			<div class="mt-4 grid max-w-2xl gap-3">{@render asking()}</div>
		{/if}
	</div>

	{#if home.shelves.length === 0}
		<!--
			A brand-new instance, and the first thing anybody ever sees. One
			sentence saying why there is nothing here, said by the empty-state
			piece as every empty list says itself (#632, chosen on #671), and
			under it the thing that was going to happen next anyway: the button
			that opens the sheet every way in is on (#650). It is the same answer
			*nothing found* gives on the Recipes screen, because it is the same
			situation: you do not have it yet, and adding it was what you came to
			do (ADR 0027).
		-->
		<div class="mx-auto mt-8 max-w-2xl px-gutter wide:mx-0">
			<Empty>{m.home_empty_title()}</Empty>
			<div class="mt-4"><AddButton main onclick={() => addRecipe?.open()} /></div>
		</div>
	{:else}
		{#each home.shelves as on (on.name)}
			<!--
				A shelf's heading is a section heading like every other (#650), with
				the earlier and later buttons at its end for a pointer only (#223):
				a finger flicks the rail, and a mouse has no way to scroll one
				sideways but shift-wheel. `pointer-only` is the hover block's gate
				(#203), so an iPad gets them with a trackpad and not before. The
				count that stood there went (H3): it read as a link to all of them.
			-->
			<Section
				heading={shelf[on.name].heading()}
				class="mx-auto max-w-2xl px-gutter wide:max-w-none"
			>
				{#snippet aside()}
					<span class="pointer-only gap-4">
						<button
							type="button"
							class="shelf-arrow"
							aria-label={m.shelf_earlier()}
							onclick={() => scroll(on.name, -1)}
						>
							<svg
								viewBox="0 0 24 24"
								aria-hidden="true"
								class="h-4 w-4"
								fill="none"
								stroke="currentColor"
								stroke-width="1.75"
								stroke-linecap="round"
								stroke-linejoin="round"><path d="m15 5-7 7 7 7" /></svg
							>
						</button>
						<button
							type="button"
							class="shelf-arrow"
							aria-label={m.shelf_later()}
							onclick={() => scroll(on.name, 1)}
						>
							<svg
								viewBox="0 0 24 24"
								aria-hidden="true"
								class="h-4 w-4"
								fill="none"
								stroke="currentColor"
								stroke-width="1.75"
								stroke-linecap="round"
								stroke-linejoin="round"><path d="m9 5 7 7-7 7" /></svg
							>
						</button>
					</span>
				{/snippet}
				<p class="-mt-2 text-read text-ink">{shelf[on.name].why()}</p>
				<!--
					The gutter is padding on the rail rather than on the page, so the
					first card lines up with its heading and the last one can still be
					pushed clear of the right edge. Snap points mean a flick lands on
					a card rather than between two.

					`scroll-pl-gutter` is what keeps that alignment true: a snap point
					is measured from the scroller's edge, not from its padding, so
					without it the rail silently scrolls the gutter away at rest and
					the first card sits flush against the side of the phone while its
					own heading is still indented.

					On the wide layout the rail runs to the window's right edge
					(#201), and the cards keep their width: an iPad on its side
					then shows two whole shelves before scrolling, where wider
					cards showed one and a half. On the phone they are narrower
					(`--tile-w-rail`, #650), so the third shows by about half.
				-->
				<ul
					bind:this={rails[on.name]}
					class="-mx-gutter mt-3 flex snap-x snap-mandatory scroll-pl-gutter gap-3 overflow-x-auto px-gutter [&>li]:w-[var(--tile-w-rail)] [&>li]:shrink-0 [&>li]:snap-start wide:[&>li]:w-[var(--tile-w)]"
				>
					{#each on.recipes as entry (entry.lineage_id)}
						<Tile {entry} />
					{/each}
				</ul>
			</Section>
		{/each}
	{/if}
</div>
