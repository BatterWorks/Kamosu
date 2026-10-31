<!--
	A Planned Meal's sheet (#486, #487): how much of it, opening it at that
	amount, moving it, planning it again, its place in its list, and taking it
	off the plan. Chosen with the Plan on 31 October 2026, as drawn: the recipe
	page's own "how much" picker, which saves as it is tapped, a large button
	that opens the recipe at what is planned, then Move and Plan again side by
	side (#487, option A), Earlier and Later where its list holds more than
	one, and the taking off below, in the colour of a thing that removes.

	Written words have no Yield and nothing to open: their sheet holds the
	words to change, saved on Done.

	A recipe that can no longer be read has no Yield to change, nothing to
	open and nothing to plan again, so its sheet says why and offers to move it
	or take it off.
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
	import { mealName, weekLabel } from '$lib/plan-where';

	type Meal = GetMealPlanOutput['planned_meals'][number];

	interface Props {
		meal: Meal;
		/** Its place in its list, from 0, and how many the list holds. */
		place: number;
		count: number;
		/** Where opening it goes: the recipe at its planned Yield. */
		opens: string;
		/** A write landed: the meal as it now stands, or null once it is off the plan. */
		landed: (id: string, now: Meal | null) => void;
		/** Put it at another place in its own list. */
		onOrder: (position: number) => Promise<void>;
		onMove: () => void;
		onAgain: () => void;
		onClose: () => void;
	}

	let { meal, place, count, opens, landed, onOrder, onMove, onAgain, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();
	let failed = $state(false);
	/**
	 * The words as typed in the sheet, saved on Done, seeded ONCE from the
	 * words it had when the sheet opened: a read again landing mid-word must
	 * not overwrite them.
	 */
	// svelte-ignore state_referenced_locally
	let words = $state(meal.words ?? '');

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

	async function order(position: number) {
		try {
			await onOrder(position);
			failed = false;
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		}
	}

	async function done() {
		const typed = words.trim();
		if (meal.words !== null && typed && typed !== meal.words) {
			try {
				const answer = await kamosu.setPlannedWords({
					planned_meal_id: meal.planned_meal_id,
					words: typed,
				});
				landed(meal.planned_meal_id, answer.planned_meal);
			} catch (error: unknown) {
				if (!(error instanceof OperationError)) throw error;
				failed = true;
				return;
			}
		}
		onClose();
	}

	const amount = $derived(said(meal.planned_yield ?? meal.written_yield));
	const heading = $derived(
		meal.day ? aPlannedDay(meal.day) : m.plan_any_day_label({ week: weekLabel(meal.week) }),
	);
	const framed =
		'flex min-h-12 items-center justify-center rounded-sm border border-accent bg-card px-3 font-semibold text-accent disabled:opacity-40';
</script>

<SheetFrame labelledby="{uid}-title" tall={85} class="flex flex-col bg-ground" onclose={onClose}>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<p class="text-label text-support-2 uppercase">{heading}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">
			{mealName(meal)}
		</h3>
		{#if meal.words !== null}
			<label class="mt-4 block text-label text-ink-2 uppercase" for="{uid}-words"
				>{m.plan_words_label()}</label
			>
			<input
				id="{uid}-words"
				class="mt-2 block min-h-12 w-full rounded-sm border border-rule bg-card px-3 text-body text-ink"
				bind:value={words}
			/>
			<p class="mt-2 text-read text-ink-2">{m.plan_words_kept()}</p>
		{:else if meal.gone}
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
		{#if meal.words === null && !meal.gone}
			<a
				href={opens}
				class="flex min-h-12 items-center justify-center rounded-sm bg-accent px-4 font-display font-semibold text-on-accent"
				data-sheet-focus
			>
				{amount ? m.plan_open_for({ amount }) : m.plan_open()}
			</a>
		{/if}
		<div class="grid grid-cols-2 gap-2">
			<button type="button" class={framed} onclick={onMove}>{m.plan_move()}</button>
			{#if !meal.gone}
				<button type="button" class={framed} onclick={onAgain}>{m.plan_again()}</button>
			{/if}
		</div>
		{#if count > 1}
			<div class="grid grid-cols-2 gap-2">
				<button
					type="button"
					class={framed}
					disabled={place === 0}
					onclick={() => void order(place - 1)}
				>
					<span aria-hidden="true">↑&nbsp;</span>{m.plan_earlier()}
				</button>
				<button
					type="button"
					class={framed}
					disabled={place === count - 1}
					onclick={() => void order(place + 1)}
				>
					<span aria-hidden="true">↓&nbsp;</span>{m.plan_later()}
				</button>
			</div>
			<p class="text-read text-ink-2" aria-live="polite">
				{meal.day
					? m.plan_place_on_day({ place: place + 1, count })
					: m.plan_place_any({ place: place + 1, count })}
			</p>
		{/if}
		<button
			type="button"
			class="min-h-12 text-read text-support underline"
			onclick={() => void takeOff()}
		>
			{m.plan_take_off()}
		</button>
		<button
			type="button"
			class="min-h-12 text-read text-accent underline"
			onclick={() => void done()}
		>
			{m.plan_done()}
		</button>
	</div>
</SheetFrame>
