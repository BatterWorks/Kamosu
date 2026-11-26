<!--
	HOW MUCH ARE YOU MAKING (#109) — one picker, in two rooms.

	The cooking screen asks it before a fresh cooking starts, in the place the
	amounts and the Step will stand; the recipe page opens it under the
	ingredients' heading, for the errands; planning a recipe asks it too.

	FOUR CHOICES, EACH SAYING WHAT IT MAKES (#651, option B, chosen on the
	prototype on 20 November 2026): half, as written, double and triple, a row
	each with a ring, and what it comes to in the recipe's own noun where it
	counts ("Double · 8 servings"). − and + did the same job beside them and
	are behind *Another amount* now, open by themselves when what is chosen is
	none of the four. A recipe that never said what it makes — a third of the
	real library — has four multiples of the recipe and a sentence saying why,
	and nothing to count one serving at a time from. Before #651 the choices
	were chips reading `×2 · 8` (#109, option B), which the UX audit found
	needed decoding (C3).

	It chooses and says nothing else. What a choice does to the amounts comes
	back from the Core (ADR 0016), and whoever holds this decides when to ask.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import {
		choiceName,
		choices,
		counts,
		isMultiplier,
		nounFor,
		said,
		same,
		stepped,
		type Wanted,
		type Written,
		type YieldNouns,
	} from '#lib/how-much.js';

	interface Props {
		written: Written;
		wanted: Wanted;
		onchoose: (wanted: Wanted) => void;
		/** The indigo room at the stove, or the paper of every other screen. */
		room: 'cook' | 'page';
		/** The noun for one and for many, where the Core gave them (#705). */
		nouns?: YieldNouns;
	}

	let { written, wanted, onchoose, room, nouns }: Props = $props();

	const quick = $derived(choices(written));
	const fewer = $derived(stepped(written, wanted, -1));
	const more = $derived(stepped(written, wanted, 1));
	/** What is chosen, counted in the recipe's noun: the number between − and +. */
	const shown = $derived(
		wanted && written && wanted.noun === written.noun ? wanted.amount : (written?.amount ?? ''),
	);
	const cook = $derived(room === 'cook');
	/** − and + are open: asked for, or what is chosen is none of the four. */
	let another = $state(false);
	const onAChoice = $derived(quick.some((choice) => same(choice.wanted, wanted)));
	const stepping = $derived(counts(written) && (another || !onAChoice));

	/**
	 * The four are one group a keyboard enters once and moves through with the
	 * arrow keys, as radio buttons are (#651): the chosen one, or the first
	 * where none is, takes Tab, and an arrow chooses the next one round.
	 */
	let group = $state<HTMLElement | undefined>(undefined);
	const tabStop = $derived(
		Math.max(
			0,
			quick.findIndex((choice) => same(choice.wanted, wanted)),
		),
	);
	function arrow(event: KeyboardEvent, at: number) {
		const by = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
		if (by === undefined) return;
		event.preventDefault();
		const to = (at + by + quick.length) % quick.length;
		another = false;
		onchoose(quick[to].wanted);
		group?.querySelectorAll<HTMLElement>('[role="radio"]')[to]?.focus();
	}
</script>

{#if cook}
	<h2 class="font-display text-title font-semibold">{m.how_much_question()}</h2>
{:else}
	<p class="text-label text-ink-2 uppercase">{m.how_much_question()}</p>
{/if}
<div
	bind:this={group}
	role="radiogroup"
	aria-label={m.how_much_question()}
	class="grid gap-2 {cook ? 'max-w-[420px] pt-4 wide:max-w-[560px]' : 'pt-3'}"
>
	{#each quick as choice, at (choice.times)}
		{@const chosen = same(choice.wanted, wanted)}
		<button
			type="button"
			role="radio"
			aria-checked={chosen}
			tabindex={at === tabStop ? 0 : -1}
			class="second-button how-much-choice"
			onkeydown={(event) => arrow(event, at)}
			onclick={() => {
				another = false;
				onchoose(choice.wanted);
			}}
		>
			<span class="how-much-ring" aria-hidden="true"></span>
			<span class="min-w-0 flex-1 text-left font-semibold"
				>{choiceName(choice.of, !!choice.count)}</span
			>
			{#if choice.count && written}
				<span class="font-display text-line"
					>{choice.count} {nounFor(choice.count, written, nouns)}</span
				>
			{/if}
		</button>
	{/each}
</div>
{#if stepping && written}
	<div class="flex items-center gap-4 pt-4 {cook ? 'max-w-[420px] wide:max-w-[560px]' : ''}">
		<button
			type="button"
			class="second-button how-much-step"
			disabled={fewer === undefined}
			aria-label={m.how_much_fewer()}
			onclick={() => {
				// Open from now on, so − and + stay under the thumb when the next
				// amount is one of the four: at 8 of a recipe for 4 (Double) they
				// folded away into Another amount (round 4 of #681's climb).
				another = true;
				if (fewer !== undefined) onchoose(fewer);
			}}
		>
			−
		</button>
		<p class="min-w-0 flex-1 text-center font-display text-step font-semibold" aria-live="polite">
			{isMultiplier(wanted) ? said(wanted) : shown}
			<span class="font-sans text-body font-normal {cook ? 'text-cook-soft' : 'text-ink-2'}">
				{isMultiplier(wanted) ? '' : nounFor(shown, written, nouns)}
			</span>
		</p>
		<button
			type="button"
			class="second-button how-much-step"
			disabled={more === undefined}
			aria-label={m.how_much_more()}
			onclick={() => {
				another = true;
				if (more !== undefined) onchoose(more);
			}}
		>
			+
		</button>
	</div>
{:else if counts(written)}
	<button type="button" class="quiet-button mt-3 self-start" onclick={() => (another = true)}>
		{m.how_much_another()}
	</button>
{/if}
{#if !counts(written)}
	<!--
		Nothing to count from: no Yield at all, or one like `a dozen` or `1½`
		that a whole serving cannot be stepped off. The second is not the recipe
		saying nothing, and is not told it is.
	-->
	<p class="pt-3 {cook ? 'text-step-reading text-cook-soft' : 'text-read text-ink-2'}">
		{written ? m.how_much_multiply_only() : m.how_much_says_nothing()}
	</p>
{/if}
