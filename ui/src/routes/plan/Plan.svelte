<!--
	The Plan: one Person's Meal Plan, a week at a time (#486, ADR 0058).

	**What it looks like was chosen on 31 October 2026**, option B of the
	prototype recorded on #486: a calendar week. On the phone each day is a row,
	its date in a block on the left and its recipes beside it; on the wide
	layout the seven days stand side by side as columns, each recipe a card
	with its photograph. A week runs Monday to Sunday, arrows move a week at a
	time, and a week with nothing planned says how to plan something.

	Today is the filled date on the phone and carries a tag on the wide layout.
	Days of the week already gone are drawn faded, and stay: a plan is not
	emptied as the week goes by.

	A planned recipe opens at its planned Yield. One that can no longer be read
	keeps its name, says so, and offers only to be taken off.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import { rereads } from '$lib/offline/device.svelte';
	import { useRoom } from '$lib/room.svelte';
	import { aPlannedDay, aPlannedWeek, aPlannedWeekday } from '$lib/dates';
	import { isMultiplier, said, toSearch } from '$lib/how-much';
	import { addDays, isDay, mondayOf, today, weekOf, weeksFrom, type Day } from '$lib/plan-days';
	import ChangeSheet from './ChangeSheet.svelte';

	type Meal = GetMealPlanOutput['planned_meals'][number];

	const kamosu = useKamosu();
	const room = useRoom();

	/** Today, read once: a plan left open past midnight is read again on the next visit. */
	const now = today();
	const thisMonday = mondayOf(now);
	/** The week on screen, by its Monday. A link may name one (`?week=`). */
	let monday = $state<Day>(
		(() => {
			const asked = page.url.searchParams.get('week');
			return isDay(asked) ? mondayOf(asked) : thisMonday;
		})(),
	);
	const days = $derived(weekOf(monday));
	const away = $derived(weeksFrom(thisMonday, monday));

	let read = $state<{ monday: Day; meals: Meal[] } | undefined>(undefined);
	/** The week on screen, once read: never another week's, still held while this one is asked. */
	const meals = $derived(read?.monday === monday ? read.meals : undefined);
	let failed = $state(false);
	/** The Planned Meal whose sheet is open. */
	let changing = $state<Meal | null>(null);

	/** Read again once writes made with no network have reached the server (#77). */
	const reread = rereads('get_meal_plan');

	$effect(() => {
		void reread.count;
		const from = monday;
		// Another week's failure is not this one's.
		failed = false;
		let current = true;
		kamosu
			.getMealPlan({ from, to: addDays(from, 6) })
			.then((answered) => {
				if (!current) return;
				read = { monday: from, meals: answered.planned_meals };
				failed = false;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});

	const on = (day: Day) => (meals ?? []).filter((meal) => meal.day === day);
	const empty = $derived(meals !== undefined && meals.length === 0);

	function weekName(weeks: number): string {
		if (weeks === 0) return m.plan_this_week();
		if (weeks === 1) return m.plan_next_week();
		if (weeks === -1) return m.plan_last_week();
		return weeks > 0 ? m.plan_weeks_ahead({ count: weeks }) : m.plan_weeks_ago({ count: -weeks });
	}

	/** How much of a recipe is planned, in words: `For 6 servings`, `×2 the recipe`. */
	function howMuch(meal: Meal): string {
		if (meal.gone) return m.plan_gone();
		const wanted = meal.planned_yield ?? meal.written_yield;
		if (!wanted) return m.plan_as_written();
		return isMultiplier(wanted)
			? m.plan_times({ amount: said(wanted) ?? '' })
			: m.plan_for({ amount: said(wanted) ?? '' });
	}

	/**
	 * The recipe at its planned Yield. One planned as written says so, or the
	 * page would open at the Shopping List's amount or an open cooking's.
	 */
	const opens = (meal: Meal) =>
		`/recipes/${meal.branch_id}${meal.planned_yield ? toSearch(meal.planned_yield) : '?written'}`;

	/** What a write answered: the meal as it now stands, or gone from the plan. */
	function landed(id: string, now: Meal | null) {
		if (!read) return;
		read.meals = now
			? read.meals.map((meal) => (meal.planned_meal_id === id ? now : meal))
			: read.meals.filter((meal) => meal.planned_meal_id !== id);
		changing = now && changing?.planned_meal_id === id ? now : changing;
	}

	const arrow = (back: boolean) => (back ? 'M15 5l-7 7 7 7' : 'M9 5l7 7-7 7');
</script>

{#snippet weekBar()}
	<div class="mt-4 flex items-center gap-3 border-y border-rule py-2">
		{#each [true, false] as back (back)}
			<button
				type="button"
				class="flex h-12 w-12 shrink-0 items-center justify-center rounded-sm border border-rule bg-card text-accent {back
					? 'order-first'
					: 'order-last'}"
				aria-label={back ? m.plan_week_before() : m.plan_week_after()}
				onclick={() => (monday = addDays(monday, back ? -7 : 7))}
			>
				<svg
					viewBox="0 0 24 24"
					class="h-[22px] w-[22px]"
					fill="none"
					stroke="currentColor"
					stroke-width="1.8"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"><path d={arrow(back)} /></svg
				>
			</button>
		{/each}
		<p class="min-w-0 flex-1 text-center" aria-live="polite">
			<span class="block font-display text-line font-semibold">{weekName(away)}</span>
			<span class="block text-read text-ink-2">{aPlannedWeek(days[0], days[6])}</span>
		</p>
	</div>
	{#if away !== 0}
		<p class="mt-2 text-center">
			<button
				type="button"
				class="tap-out text-read text-accent underline"
				onclick={() => (monday = thisMonday)}
			>
				{m.plan_back_to_this_week()}
			</button>
		</p>
	{/if}
{/snippet}

{#snippet picture(meal: Meal, size: string)}
	{#if meal.main_photo && !meal.gone}
		<img
			alt=""
			class="{size} shrink-0 rounded-sm bg-ground-2 object-cover"
			src="/api/photographs/{meal.main_photo}/card"
		/>
	{:else}
		<span
			aria-hidden="true"
			class="{size} flex shrink-0 items-center justify-center rounded-sm border border-dashed border-rule font-display text-title text-ink-2"
		>
			{meal.gone ? '–' : meal.title.slice(0, 1)}
		</span>
	{/if}
{/snippet}

{#snippet row(meal: Meal)}
	<li class="flex items-center gap-3 py-2">
		{#if meal.gone}
			<span class="flex min-w-0 flex-1 items-center gap-3">
				{@render picture(meal, 'h-[52px] w-[52px]')}
				<span class="min-w-0">
					<span class="block font-display text-line font-semibold text-ink-2">{meal.title}</span>
					<span class="block text-read text-support">{howMuch(meal)}</span>
				</span>
			</span>
		{:else}
			<a href={opens(meal)} class="flex min-w-0 flex-1 items-center gap-3 text-ink">
				{@render picture(meal, 'h-[52px] w-[52px]')}
				<span class="min-w-0">
					<span class="block font-display text-line font-semibold">{meal.title}</span>
					<span class="block text-read text-ink-2">{howMuch(meal)}</span>
				</span>
			</a>
		{/if}
		<button
			type="button"
			class="min-h-12 shrink-0 px-1 text-read text-accent underline"
			onclick={() => (changing = meal)}
		>
			{meal.gone ? m.plan_remove() : m.plan_change()}
		</button>
	</li>
{/snippet}

{#snippet card(meal: Meal)}
	<li class="overflow-hidden rounded-sm border border-rule bg-card">
		{#if meal.gone}
			<span class="block">
				{@render picture(meal, 'block aspect-[4/3] w-full rounded-none')}
				<span class="block px-2 pt-2">
					<span class="block font-display text-read font-semibold text-ink-2">{meal.title}</span>
					<span class="block text-read text-support">{howMuch(meal)}</span>
				</span>
			</span>
		{:else}
			<a href={opens(meal)} class="block text-ink">
				{@render picture(meal, 'block aspect-[4/3] w-full rounded-none')}
				<span class="block px-2 pt-2">
					<span class="block font-display text-read font-semibold">{meal.title}</span>
					<span class="block text-read text-ink-2">{howMuch(meal)}</span>
				</span>
			</a>
		{/if}
		<button
			type="button"
			class="min-h-[40px] px-2 text-left text-read text-accent underline"
			onclick={() => (changing = meal)}
		>
			{meal.gone ? m.plan_remove() : m.plan_change()}
		</button>
	</li>
{/snippet}

<Screen title={m.plan_title()} blurb={m.plan_blurb()} fills>
	{@render weekBar()}

	{#if failed && meals === undefined}
		<p class="mt-6 text-read text-support" role="alert">{m.plan_failed()}</p>
	{:else if empty}
		<div
			class="mt-6 max-w-2xl rounded-sm border border-dashed border-rule bg-ground-2 p-6 text-body text-ink-2"
		>
			<p>{m.plan_empty()}</p>
			<p class="mt-2">{m.plan_empty_how()}</p>
			<a
				href="/recipes"
				class="mt-4 inline-flex min-h-12 items-center rounded-sm border border-accent px-4 font-medium text-accent"
			>
				{m.plan_empty_choose()}
			</a>
		</div>
	{:else if meals !== undefined}
		{#if room.wide}
			<div class="mt-4 grid grid-cols-7 gap-2">
				{#each days as day (day)}
					<section class={day < now ? 'opacity-60' : ''}>
						<h2
							class="flex flex-wrap items-baseline gap-1 border-b-2 py-2 {day === now
								? 'border-accent text-accent'
								: 'border-rule'}"
						>
							<span
								aria-hidden="true"
								class="text-label uppercase {day === now ? '' : 'text-ink-2'}"
								>{aPlannedWeekday(day)}</span
							>
							<span aria-hidden="true" class="font-display text-line font-semibold"
								>{Number(day.slice(8))}</span
							>
							{#if day === now}
								<span class="rounded-pill bg-accent px-2 text-label text-on-accent uppercase"
									>{m.plan_today()}</span
								>
							{/if}
							<span class="sr-only">{aPlannedDay(day)}</span>
						</h2>
						{#if on(day).length > 0}
							<ul class="mt-2 grid gap-2">
								{#each on(day) as meal (meal.planned_meal_id)}
									{@render card(meal)}
								{/each}
							</ul>
						{:else}
							<p
								class="mt-2 rounded-sm border border-dashed border-rule p-4 text-center text-read text-ink-2"
							>
								{m.plan_nothing_short()}
							</p>
						{/if}
					</section>
				{/each}
			</div>
		{:else}
			<div class="mt-2">
				{#each days as day (day)}
					<section class="flex gap-3 border-b border-rule py-2 {day < now ? 'opacity-60' : ''}">
						<h2 class="w-[52px] shrink-0 pt-1 text-center">
							<span
								aria-hidden="true"
								class="block text-label uppercase {day === now
									? 'font-semibold text-accent'
									: 'text-ink-2'}">{aPlannedWeekday(day)}</span
							>
							<span
								aria-hidden="true"
								class="mx-auto mt-1 block font-display text-title leading-none font-semibold {day ===
								now
									? 'flex h-[40px] w-[40px] items-center justify-center rounded-pill bg-accent text-on-accent'
									: ''}">{Number(day.slice(8))}</span
							>
							<span class="sr-only">{aPlannedDay(day)}</span>
						</h2>
						{#if on(day).length > 0}
							<ul class="min-w-0 flex-1 divide-y divide-rule">
								{#each on(day) as meal (meal.planned_meal_id)}
									{@render row(meal)}
								{/each}
							</ul>
						{:else}
							<p class="flex-1 py-3 text-read text-ink-2">{m.plan_nothing()}</p>
						{/if}
					</section>
				{/each}
			</div>
		{/if}
	{/if}
</Screen>

{#if changing}
	<ChangeSheet meal={changing} {landed} onClose={() => (changing = null)} opens={opens(changing)} />
{/if}
