<!--
	Moving a Planned Meal, or planning it again (#487): the list of days and
	weeks "Plan this" uses, and one button.

	**Chosen on 31 October 2026**, as drawn in the prototype recorded on #487:
	under the meal's name, where it is now, or that it stays there too, and at
	what amount; then the list; then "Move it to <day>" or "Plan it again for
	<day>", which waits until something is chosen. Planning again keeps the
	original where it is, and the copy keeps its amount.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import PlanPicker from '$lib/PlanPicker.svelte';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput } from '$lib/api/catalogue';
	import { said } from '$lib/how-much';
	import { whereOf, type Where } from '$lib/plan-days';
	import { mealName, whereSaid, whereTarget } from '$lib/plan-where';

	type Meal = GetMealPlanOutput['planned_meals'][number];

	interface Props {
		meal: Meal;
		/** Planning it again, rather than moving it. */
		again: boolean;
		/** Chosen: the write is the Plan's, which says what it did. */
		onChosen: (where: Where) => Promise<void>;
		onClose: () => void;
	}

	let { meal, again, onChosen, onClose }: Props = $props();

	const uid = $props.id();
	let chosen = $state<Where | null>(null);
	let working = $state(false);
	let failed = $state(false);

	const now = $derived(whereSaid(whereOf(meal)));
	const amount = $derived(said(meal.planned_yield));

	async function go() {
		if (!chosen || working) return;
		working = true;
		try {
			await onChosen(chosen);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			working = false;
		}
	}
</script>

<SheetFrame labelledby="{uid}-title" tall={88} class="flex flex-col bg-ground" onclose={onClose}>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<p class="text-label text-support-2 uppercase">{again ? m.plan_again() : m.plan_move()}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">
			{mealName(meal)}
		</h3>
		<p class="mt-2 text-read text-ink-2">
			{#if !again}
				{m.plan_now_on({ where: now })}
			{:else if amount}
				{m.plan_stays_on_amount({ where: now, amount })}
			{:else}
				{m.plan_stays_on({ where: now })}
			{/if}
		</p>
		<PlanPicker
			{chosen}
			skip={again ? undefined : meal.planned_meal_id}
			onchoose={(where) => (chosen = where)}
		/>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="min-h-12 rounded-sm bg-accent px-4 font-display font-semibold text-on-accent disabled:opacity-40"
			disabled={!chosen || working}
			onclick={() => void go()}
		>
			{#if !chosen}
				{m.plan_choose_a_day()}
			{:else if again}
				{m.plan_again_for({ where: whereTarget(chosen) })}
			{:else}
				{m.plan_move_it_to({ where: whereTarget(chosen) })}
			{/if}
		</button>
		<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
