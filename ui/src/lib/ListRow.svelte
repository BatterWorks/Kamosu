<!--
	One line of a list that opens something (#632): a picture where the list
	has them, a title, a line under it if there is one, and an arrow on the
	right saying it opens (U8). The row chosen on #632 is `rows-E.html`: on the
	paper between hairlines, the title in the display face, cut after two
	lines.

	The list around the rows wears `list-rows`, which draws the hairline above
	the first. A list where some rows have a picture gives the others `null`,
	an empty square that keeps the titles in line; a list with no pictures
	gives none, and no square is drawn.

	A row goes somewhere (`href`) or opens something on this screen
	(`onclick`), never both.
	A row may carry a small drawing before its title in place of a picture
	(`icon`), as Settings' Language row carries its globe (#653), and a row of
	a list kept beside the page it opens says it is that page (`current`).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		title: string;
		/** One line under the title, in the secondary ink. */
		line?: string;
		/** The picture's address; `null` for an empty square in a list that has pictures. */
		picture?: string | null;
		/** A small drawing before the title, where a row has no picture. */
		icon?: Snippet;
		/** The page this row opens is the one beside the list (ADR 0044). */
		current?: boolean;
		/**
		 * The line is what tells the rows apart, as on Settings' menu, so it is
		 * in full ink rather than the secondary (#653, the audit's G2).
		 */
		inkLine?: boolean;
	} & ({ href: string; onclick?: never } | { onclick: () => void; href?: never });

	let {
		title,
		line,
		picture,
		icon,
		current = false,
		inkLine = false,
		href,
		onclick,
	}: Props = $props();
</script>

{#snippet inside()}
	{#if icon}
		{@render icon()}
	{:else if picture}
		<img
			src={picture}
			alt=""
			class="h-[var(--row-picture)] w-[var(--row-picture)] shrink-0 rounded-sm object-cover"
		/>
	{:else if picture === null}
		<span
			aria-hidden="true"
			class="h-[var(--row-picture)] w-[var(--row-picture)] shrink-0 rounded-sm bg-ground-2"
		></span>
	{/if}
	<span class="min-w-0 flex-1">
		<span
			class="line-clamp-2 font-display text-list-title font-semibold [overflow-wrap:anywhere] text-ink"
			>{title}</span
		>
		{#if line}
			<span class={['mt-[2px] block text-read leading-[1.35]', inkLine ? 'text-ink' : 'text-ink-2']}
				>{line}</span
			>
		{/if}
	</span>
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-[20px] w-[20px] shrink-0 text-ink-2"
		fill="none"
		stroke="currentColor"
		stroke-width="1.75"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="m9 5 7 7-7 7" />
	</svg>
{/snippet}

{#if href !== undefined}
	<a {href} class="list-row" aria-current={current ? 'page' : undefined}>{@render inside()}</a>
{:else}
	<button type="button" {onclick} class="list-row">{@render inside()}</button>
{/if}
