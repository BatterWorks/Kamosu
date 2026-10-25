<!--
	One group of the shelf's filters, open over the shelf (#445): what a button
	in the row under the search box opens. Every filter went behind that row
	so the screen is about its recipes, the design chosen on #445 over a
	headed line per group.

	A heading, one line saying how the group works, the choices, then Clear
	and Done. Cookbooks, Kitchens and Cooking are rows with a tick on what is
	held; Tags are chips that wrap, since a library files by dozens of words.
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
	}
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';

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
	<div class="flex min-h-0 flex-1 flex-col px-gutter pt-6 pb-[28px]">
		<h2 id={heading} class="font-display text-shelf-heading font-semibold">{title}</h2>
		<p class="mt-2 text-read text-ink-2">
			{kind === 'any' ? m.recipes_filter_any() : m.recipes_filter_one()}
		</p>
		<div class="min-h-0 flex-1 overflow-y-auto">
			{#if kind === 'tags'}
				<!-- `relative` holds each chip's screen-reader words inside the list,
				     as it did on the line these chips stood on before (#112). -->
				<div class="relative mt-4 flex flex-wrap content-start gap-2">
					{#each choices as choice (choice.id)}
						<button
							type="button"
							aria-pressed={choice.held}
							onclick={() => pick(choice.id)}
							class="min-h-8 shrink-0 rounded-sm border px-3 text-read {choice.held
								? 'border-accent bg-accent text-on-accent'
								: 'border-rule text-ink-2'}"
						>
							{choice.name}
							{#if choice.elsewhere}
								<span class="text-label uppercase {choice.held ? '' : 'text-support'}">
									<span aria-hidden="true">{choice.elsewhere.shown}</span>
									<span class="sr-only">{choice.elsewhere.said}</span>
								</span>
							{/if}
						</button>
					{/each}
				</div>
			{:else}
				<ul class="mt-3">
					{#each choices as choice (choice.id)}
						<li>
							<!-- Room kept on the right for the tick, held or not, so a
							     long name never runs under it. -->
							<button
								type="button"
								aria-pressed={choice.held}
								onclick={() => pick(choice.id)}
								class="relative block min-h-12 w-full border-b border-rule py-3 pr-8 text-left text-line"
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
			<button
				type="button"
				onclick={onclear}
				class="min-h-12 flex-1 rounded-sm border border-rule text-ink-2"
			>
				{m.recipes_filter_clear()}
			</button>
			<button
				type="button"
				onclick={onclose}
				class="min-h-12 flex-1 rounded-sm bg-accent text-on-accent"
			>
				{m.recipes_filter_done()}
			</button>
		</div>
	</div>
</SheetFrame>
