<!--
	Which Ingredient Lines one Step uses, ticked by hand (#456, option B of
	25 October 2026; the prototype's `b.html`).

	Kamosu works this out of the Step's words on every read (ADR 0019), and a
	cook corrects it here when it read wrong (ADR 0054). The sheet decides
	nothing and sends nothing: it hands back what was ticked, and the writing
	screen sends it after the save, on the Version the save left. That is why
	it says so at its foot, and why its button is *Done* and not *Save*.

	EVERY LINE IS OFFERED, under its heading, because the line the reading
	missed is the one a cook is here for. A heading is there to read and never
	to tick: `set_step_uses` refuses a section. A line another Step already
	uses says which, and can still be ticked, since salt goes in twice.

	Ticking nothing is a real answer, the Step that adds nothing, and the sheet
	says so in words rather than leaving an empty list to be read as a fault.

	The few lengths written in pixels, the box, its border, the tag's padding
	and the labels' spacing, are the chosen prototype's, which the scale does
	not hold.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';

	interface Props {
		/** The Step's number as the screen shows it. */
		number: number;
		/** The Step's words, as they stand on the screen. */
		text: string;
		/** Every row of the ingredient list, headings included, in order. */
		lines: { id: number; kind: 'ingredient' | 'section'; text: string }[];
		/** The lines this Step uses now, by row id. */
		ticked: number[];
		/** Whether what this Step uses was set by hand. */
		byHand: boolean;
		/** The other Steps using a line, by row id, as the screen numbers them. */
		elsewhere: (line: number) => number[];
		/** The number of the next Step that can be corrected, where there is one. */
		next: number | null;
		onDone: (ticked: number[]) => void;
		onNext: (ticked: number[]) => void;
		onPutBack: () => void;
		onCancel: () => void;
	}

	let {
		number,
		text,
		lines,
		ticked,
		byHand,
		elsewhere,
		next,
		onDone,
		onNext,
		onPutBack,
		onCancel,
	}: Props = $props();

	const uid = $props.id();
	/** Seeded once: the sheet is opened for one Step and closed again. */
	// svelte-ignore state_referenced_locally
	let chosen = $state(new Set(ticked));

	function toggle(line: number) {
		const now = new Set(chosen);
		if (now.has(line)) now.delete(line);
		else now.add(line);
		chosen = now;
	}

	/** In the order of the list, which is the order `set_step_uses` is sent in. */
	const inOrder = () => lines.filter((line) => chosen.has(line.id)).map((line) => line.id);
</script>

<SheetFrame labelledby="{uid}-title" tall={85} class="flex flex-col bg-ground" onclose={onCancel}>
	<div class="px-gutter pt-6">
		<div class="flex items-center justify-between gap-2">
			<h3
				id="{uid}-title"
				class="font-display text-label font-semibold tracking-[0.05em] text-accent uppercase"
			>
				{m.step_uses_title({ number })}
			</h3>
			{#if byHand}
				<span
					class="shrink-0 rounded-sm border border-accent px-[6px] py-[1px] text-label tracking-[0.05em] whitespace-nowrap text-accent uppercase"
					>{m.step_uses_by_hand()}</span
				>
			{/if}
		</div>
		<p class="mt-2 line-clamp-3 text-read text-ink-2">{text}</p>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-3">
		<ul>
			{#each lines as line (line.id)}
				{#if line.kind === 'section'}
					<li
						class="border-b border-rule pt-4 pb-1 font-display text-label tracking-[0] text-ink-2 uppercase"
					>
						{line.text}
					</li>
				{:else}
					{@const also = elsewhere(line.id)}
					<li>
						<button
							type="button"
							role="checkbox"
							aria-checked={chosen.has(line.id)}
							class="flex min-h-12 w-full items-start gap-3 border-b border-rule py-3 text-left"
							onclick={() => toggle(line.id)}
						>
							<!--
								Shape only: the state is on the checkbox, where a screen
								reader reads it, and the box is what an eye reads.
							-->
							<span
								class="mt-[2px] flex h-[20px] w-[20px] shrink-0 items-center justify-center rounded-sm border-[1.5px]
								{chosen.has(line.id) ? 'border-accent bg-accent text-on-accent' : 'border-ink-2'}"
								aria-hidden="true"
							>
								{#if chosen.has(line.id)}
									<svg
										viewBox="0 0 24 24"
										class="h-[14px] w-[14px]"
										fill="none"
										stroke="currentColor"
										stroke-width="3"
										stroke-linecap="round"
										stroke-linejoin="round"><path d="M5 12l5 5 9-10" /></svg
									>
								{/if}
							</span>
							<span class="min-w-0 flex-1">
								<span class="block text-line [overflow-wrap:anywhere]">{line.text}</span>
								{#if also.length > 0}
									<span class="block text-read text-ink-2">
										{m.step_uses_also({ steps: also.join(', ') })}
									</span>
								{/if}
							</span>
						</button>
					</li>
				{/if}
			{/each}
		</ul>
		{#if chosen.size === 0}
			<p class="py-3 text-body text-ink-2">{m.step_uses_nothing()}</p>
		{/if}
	</div>

	<div class="grid gap-2 border-t border-rule px-gutter pt-3 pb-4">
		<div class="flex gap-2">
			<button
				type="button"
				class="min-h-12 flex-1 rounded-sm bg-accent font-display text-body font-semibold text-on-accent"
				onclick={() => onDone(inOrder())}
			>
				{m.step_uses_done()}
			</button>
			{#if next !== null}
				<button
					type="button"
					class="min-h-12 flex-1 rounded-sm border border-accent text-body text-accent"
					onclick={() => onNext(inOrder())}
				>
					{m.step_uses_next({ number: next })}
				</button>
			{/if}
		</div>
		<div class="flex justify-between gap-2">
			{#if byHand}
				<button
					type="button"
					class="min-h-12 text-body text-ink-2 underline underline-offset-3"
					onclick={onPutBack}
				>
					{m.step_uses_put_back()}
				</button>
			{/if}
			<button
				type="button"
				class="min-h-12 text-body text-ink-2 underline underline-offset-3 {byHand ? '' : 'mx-auto'}"
				onclick={onCancel}
			>
				{m.step_uses_cancel()}
			</button>
		</div>
		<p class="text-center text-read text-ink-2">{m.step_uses_note()}</p>
	</div>
</SheetFrame>
