<!--
	A labelled block within a screen: an uppercase heading over a rule, and what
	it introduces. One component so every section on every screen is the same
	distance from the thing above it.

	Anything else it is given goes onto the section itself, which is how one
	that takes a dropped file listens for it (#205).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	import type { HTMLAttributes } from 'svelte/elements';

	interface Props extends HTMLAttributes<HTMLElement> {
		heading: string;
		/** A small act at the heading's end, as the recipe's jump to its Method (#649). */
		aside?: Snippet;
		children: Snippet;
	}

	let { heading, aside, children, class: more, ...rest }: Props = $props();

	/** The rule under a heading, and the words of one, written once for both shapes. */
	const RULED = 'mb-3 border-b border-rule pb-2';
	const WORDS = 'text-label font-medium text-accent uppercase';
</script>

<section {...rest} class={['mt-8', more]}>
	{#if aside}
		<!-- An act stands beside the heading rather than inside it, so the heading's name stays its words. -->
		<div class="{RULED} flex items-baseline justify-between gap-3">
			<h2 class={WORDS}>{heading}</h2>
			{@render aside()}
		</div>
	{:else}
		<h2 class="{RULED} {WORDS}">{heading}</h2>
	{/if}
	{@render children()}
</section>
