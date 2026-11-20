<!--
	One screen: its title, a line saying what it is for, and whatever it holds.

	Every section reads the same way because they are all drawn by this, which is
	also what keeps the gutter and the room above the tab bar in one place.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		title: string;
		blurb?: string;
		/**
		 * Names this screen's title as the other half of an expanding pair, so a
		 * card elsewhere grows into it rather than the screen swapping (ADR 0012).
		 */
		expandsFrom?: string;
		/**
		 * Drawn as a card, where the screen is what is open beside a list on
		 * the wide layout (#199).
		 */
		card?: boolean;
		/**
		 * Runs to the window's right edge on the wide layout, for one of the
		 * browsing screens (#201). The phone's column is unchanged.
		 */
		fills?: boolean;
		/**
		 * As wide as a recipe's page on the wide layout, for a screen that
		 * lays two columns out there (#202). The phone's column is unchanged.
		 */
		columns?: boolean;
		/**
		 * An act at the end of the title's line, as *Add a recipe* on Recipes
		 * and Home (#650). The title keeps its words; the act stands beside it.
		 */
		aside?: Snippet;
		/** More classes on the screen's column, as Recipes' room for its letters (#650). */
		class?: string;
		children?: Snippet;
	}

	let {
		title,
		blurb,
		expandsFrom,
		card = false,
		fills = false,
		columns = false,
		aside,
		class: more,
		children,
	}: Props = $props();
</script>

<!-- The tab, the history and a bookmark read the page's name (#224). -->
<svelte:head>
	<title>{title} · {m.app_name()}</title>
</svelte:head>

<div
	class={card
		? 'open-card'
		: [
				'mx-auto px-gutter pt-6 pb-tabbar',
				columns ? 'two-column-page' : 'max-w-2xl',
				fills && 'wide:max-w-none',
				more,
			]}
>
	{#snippet heading()}
		<h1
			class="font-display text-title font-semibold"
			style={expandsFrom
				? `view-transition-name: ${expandsFrom}; view-transition-class: expanding`
				: undefined}
		>
			{title}
		</h1>
	{/snippet}
	{#if aside}
		<div class="flex items-center justify-between gap-3">
			{@render heading()}
			{@render aside()}
		</div>
	{:else}
		{@render heading()}
	{/if}
	{#if blurb}
		<p class="mt-1 max-w-2xl text-read text-ink-2">{blurb}</p>
	{/if}

	{#if children}
		<div class="mt-6">{@render children()}</div>
	{/if}
</div>
