<!--
	One group of the shelf's filters, open over the shelf (#445): what a button
	in the row under the search box opens. Every filter went behind that row
	so the screen is about its recipes, the design chosen on #445 over a
	headed line per group.

	A heading with its Close, one line saying what a tap does and how to undo
	it, the choices, then Show all recipes and Done (#650, chosen 20 November
	2026, over "One at a time. Tap it again to let go.", which the UX audit
	found left unclear when a tag applies, T1). Cookbooks, Kitchens and
	Cooking are rows with a tick on what is held; Tags are chips that wrap,
	since a library files by dozens of words, the most used first with how
	many recipes carry each, drawn at 40 and tapped at 48, 8 apart (T3).
	A choice applies the moment it is tapped, so the shelf behind is already
	narrowed when the list closes. Picking in a one-at-a-time group closes it;
	Cooking holds several and stays open until Done.
-->
<script lang="ts" module>
	export interface Choice {
		id: string;
		name: string;
		held: boolean;
		/** A Tag named in another Language: its code shown, and said in words (#112). */
		elsewhere?: { shown: string; said: string } | null;
		/** How many recipes carry a Tag, shown beside its word (#650). */
		count?: number;
	}
</script>

<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import SheetFrame from '#lib/SheetFrame.svelte';

	interface Props {
		title: string;
		/**
		 * `one`: rows, one held at a time (Cookbooks, Kitchens). `any`: rows,
		 * several held, matching any (Cooking). `tags`: chips that wrap, one
		 * held at a time.
		 */
		kind: 'one' | 'any' | 'tags';
		choices: Choice[];
		onpick: (id: string) => void;
		onclear: () => void;
		onclose: () => void;
	}

	const { title, kind, choices, onpick, onclear, onclose }: Props = $props();

	const heading = $props.id();

	function pick(id: string) {
		onpick(id);
		if (kind !== 'any') onclose();
	}
</script>

<!-- Dimmed with ink, as the chosen page dimmed it. The padding is inside, so
     the frame's own keeps the buttons clear of a phone's home indicator. -->
<SheetFrame labelledby={heading} tall={85} dim="ink" class="flex flex-col bg-card" {onclose}>
	<div class="flex min-h-0 flex-1 flex-col px-gutter pt-4 pb-[28px]">
		<div class="flex items-center gap-2">
			<h2 id={heading} class="min-w-0 flex-1 font-display text-shelf-heading font-semibold">
				{title}
			</h2>
			<button type="button" class="quiet-button gap-2 text-ink" onclick={onclose}>
				<svg
					viewBox="0 0 24 24"
					aria-hidden="true"
					class="h-[18px] w-[18px]"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18" /></svg
				>
				{m.plus_close()}
			</button>
		</div>
		<p class="mt-3 text-body text-ink">
			{kind === 'any'
				? m.recipes_filter_any()
				: kind === 'tags'
					? m.recipes_filter_tags()
					: m.recipes_filter_one()}
		</p>
		<div class="min-h-0 flex-1 overflow-y-auto">
			{#if kind === 'tags'}
				<!-- `relative` holds each chip's screen-reader words inside the list,
				     as it did on the line these chips stood on before (#112). The
				     padding keeps each chip's tapped area inside the scrolling box. -->
				<div class="relative mt-4 flex flex-wrap content-start gap-2 p-1">
					{#each choices as choice (choice.id)}
						<button
							type="button"
							aria-pressed={choice.held}
							onclick={() => pick(choice.id)}
							class={[
								'quiet-button gap-[6px] text-body',
								choice.held ? 'border-accent bg-accent text-on-accent' : 'text-ink',
							]}
						>
							{choice.name}
							{#if choice.elsewhere}
								<span class="text-label uppercase {choice.held ? '' : 'text-support'}">
									<span aria-hidden="true">{choice.elsewhere.shown}</span>
									<span class="sr-only">{choice.elsewhere.said}</span>
								</span>
							{/if}
							{#if choice.count !== undefined}
								<span class="text-read {choice.held ? '' : 'text-ink-2'}">{choice.count}</span>
							{/if}
						</button>
					{/each}
				</div>
			{:else}
				<ul class="mt-3 list-rows">
					{#each choices as choice (choice.id)}
						<li>
							<!-- Room kept on the right for the tick, held or not, so a
							     long name never runs under it. -->
							<button
								type="button"
								aria-pressed={choice.held}
								onclick={() => pick(choice.id)}
								class="relative list-row pr-8 text-line"
							>
								{choice.name}
								{#if choice.held}
									<span
										aria-hidden="true"
										class="absolute top-1/2 right-1 -translate-y-1/2 text-accent">✓</span
									>
								{/if}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
		<div class="mt-4 flex gap-3">
			<!-- Lets the group go and shows the shelf, as the chosen page did. -->
			<button
				type="button"
				onclick={() => {
					onclear();
					onclose();
				}}
				class="second-button flex-1"
			>
				{m.recipes_filter_clear()}
			</button>
			<button type="button" onclick={onclose} class="main-button flex-1">
				{m.recipes_filter_done()}
			</button>
		</div>
	</div>
</SheetFrame>
