<!--
	A Planned Meal's sheet (#486): how much of it, opening it at that amount,
	and taking it off the plan. Chosen with the Plan on 31 October 2026, as
	drawn: the recipe page's own "how much" picker, which saves as it is
	tapped, a large button that opens the recipe at what is planned, and the
	taking off below it, in the colour of a thing that removes.

	A recipe that can no longer be read has no Yield to change and nothing to
	open, so its sheet says why and offers only the taking off.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import HowMuch from '$lib/HowMuch.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput } from '$lib/api/catalogue';
	import { aPlannedDay } from '$lib/dates';
	import { said, type Wanted } from '$lib/how-much';

	type Meal = GetMealPlanOutput['planned_meals'][number];

	interface Props {
		meal: Meal;
		/** Where opening it goes: the recipe at its planned Yield. */
		opens: string;
		/** A write landed: the meal as it now stands, or null once it is off the plan. */
		landed: (id: string, now: Meal | null) => void;
		onClose: () => void;
	}

	let { meal, opens, landed, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();
	let failed = $state(false);

	async function choose(wanted: Wanted) {
		try {
			const answer = await kamosu.setPlannedYield({
				planned_meal_id: meal.planned_meal_id,
				planned_yield: wanted,
			});
			failed = false;
			landed(meal.planned_meal_id, answer.planned_meal);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	async function takeOff() {
		try {
			await kamosu.removePlannedMeal({ planned_meal_id: meal.planned_meal_id });
			landed(meal.planned_meal_id, null);
			onClose();
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	const amount = $derived(said(meal.planned_yield ?? meal.written_yield));
</script>

<SheetFrame labelledby="{uid}-title" tall={85} class="flex flex-col bg-ground" onclose={onClose}>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<p class="text-label text-support-2 uppercase">{aPlannedDay(meal.day)}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">{meal.title}</h3>
		{#if meal.gone}
			<p class="mt-3 text-read text-ink-2">{m.plan_gone_explained()}</p>
		{:else}
			<div class="mt-4 border-t border-rule pt-3">
				<HowMuch
					written={meal.written_yield}
					wanted={meal.planned_yield}
					room="page"
					onchoose={(wanted) => void choose(wanted)}
				/>
			</div>
		{/if}
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		{#if !meal.gone}
			<a
				href={opens}
				class="flex min-h-12 items-center justify-center rounded-sm bg-accent px-4 font-display font-semibold text-on-accent"
				data-sheet-focus
			>
				{amount ? m.plan_open_for({ amount }) : m.plan_open()}
			</a>
		{/if}
		<button
			type="button"
			class="min-h-12 text-read text-support underline"
			onclick={() => void takeOff()}
		>
			{m.plan_take_off()}
		</button>
		{#if !meal.gone}
			<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
				{m.plan_done()}
			</button>
		{/if}
	</div>
</SheetFrame>
