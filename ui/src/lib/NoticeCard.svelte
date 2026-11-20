<!--
	A notice card (#632): something a screen says that is not its content,
	with what to do about it. The card chosen on #632 is `notices-G.html`: the
	card's white, a hairline round it and a bar along the top in its tone's
	colour, a title in the display face, its words in full ink, and its
	actions as a main button and secondary buttons at their own width.

	Three tones. `plain` says something (the bar is a hairline's colour),
	`waiting` asks for something that waits on the cook (indigo), `warning`
	says something went wrong (beni, its title too, and read out at once).

	A screen shows one card at a time, and never on top of the confirmation of
	what the person just did (U7). Where it sits is the caller's: anything
	else given goes onto the card itself.
-->
<script lang="ts" module>
	export type NoticeAction = {
		label: string;
		act: () => void;
		/** The card's main act, filled. At most one. */
		main?: boolean;
		/** An act that loses or ends something, in beni (U11). */
		loses?: boolean;
		/** Pressed, and waiting: its label should say what is happening. */
		busy?: boolean;
		disabled?: boolean;
	};
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	interface Props extends HTMLAttributes<HTMLElement> {
		/** Omitted where the words say everything, as an offer inside a recipe does. */
		title?: string;
		tone?: 'plain' | 'waiting' | 'warning';
		actions?: NoticeAction[];
		/**
		 * Whether a screen reader reads it out as it appears and as it changes.
		 * Off for a card that holds a form or a list a person works through, as
		 * reading again or keeping a cooking does (#666): read out, every word
		 * typed or label turned would read the whole card again. It is then a
		 * named region, found by its title.
		 */
		announce?: boolean;
		children: Snippet;
	}

	let {
		title,
		tone = 'plain',
		actions = [],
		announce = true,
		children,
		class: more,
		...rest
	}: Props = $props();
</script>

<section
	{...rest}
	class={[
		'border border-t-3 border-rule bg-card p-4 text-read text-ink',
		tone === 'waiting' && 'border-t-accent',
		tone === 'warning' && 'border-t-support',
		more,
	]}
	aria-label={title}
	role={!announce ? undefined : tone === 'warning' ? 'alert' : 'status'}
>
	{#if title}
		<h2 class={['mb-1 font-display text-line font-semibold', tone === 'warning' && 'text-support']}>
			{title}
		</h2>
	{/if}
	{@render children()}
	{#if actions.length > 0}
		<div class="mt-3 flex flex-wrap gap-2">
			{#each actions as action (action.label)}
				<button
					type="button"
					onclick={action.act}
					disabled={action.disabled || action.busy}
					aria-busy={action.busy ? 'true' : undefined}
					class={[action.main ? 'main-button' : 'second-button', action.loses && 'loses']}
				>
					{action.label}
				</button>
			{/each}
		</div>
	{/if}
</section>
