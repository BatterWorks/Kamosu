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
-->
<script lang="ts">
	type Props = {
		title: string;
		/** One line under the title, in the secondary ink. */
		line?: string;
		/** The picture's address; `null` for an empty square in a list that has pictures. */
		picture?: string | null;
	} & ({ href: string; onclick?: never } | { onclick: () => void; href?: never });

	let { title, line, picture, href, onclick }: Props = $props();
</script>

{#snippet inside()}
	{#if picture}
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
			<span class="mt-[2px] block text-read leading-[1.35] text-ink-2">{line}</span>
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
	<a {href} class="list-row">{@render inside()}</a>
{:else}
	<button type="button" {onclick} class="list-row">{@render inside()}</button>
{/if}
