<!--
	The days and weeks a meal can be put on (#486, #487): what "Plan this",
	Move and Plan again choose from.

	**Chosen on 31 October 2026**, option A of the prototype recorded on #487:
	one list, today first, a heading at each week that stays in view while its
	days scroll, and under each heading, first, a shaded row for any day of
	that week, then its days. Each row names what is already planned there.
	The list ends in "Show 3 more weeks", so a meal planned again can go
	further ahead than the three weeks shown first.

	**Since #654, panel A of its prototype (27 November 2026,
	`sheet-A-busy.html`):** only the chosen row looks chosen, the card's white
	with an indigo bar down its edge, and the rows for any day of a week are
	rows like the others, where their shading had read as chosen (PL2 of the
	UX audit). What a day holds is in full ink, the recipe being planned in
	indigo where a day already holds it, and the chosen row shows that recipe
	joining what it holds, under it, as the Meal Plan keeps every meal a day is
	given (ADR 0058, PL1).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { useKamosu } from '#lib/kamosu.js';
	import type { GetMealPlanOutput } from '#lib/api/catalogue.js';
	import { aPlannedDay, aPlannedWeek } from '#lib/dates.js';
	import { anyDayName, mealName } from '#lib/plan-where.js';
	import {
		addDays,
		mondayOf,
		today,
		weekOf,
		weeksFrom,
		whereOf,
		type Day,
		type Where,
	} from '#lib/plan-days.js';

	interface Props {
		/** What is chosen, or null until something is. */
		chosen: Where | null;
		/** A meal left out of what each row names: the one being moved. */
		skip?: string;
		/** The recipe being planned or moved: drawn in indigo where a row already holds it. */
		branchId?: string | null;
		/** Its name, shown joining what the chosen row holds. */
		joining?: string;
		onchoose: (where: Where) => void;
	}

	let { chosen, skip, branchId = null, joining, onchoose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	const now = today();
	const thisMonday = mondayOf(now);
	let shown = $state(3);
	const weeks = $derived(
		Array.from({ length: shown }, (_, ahead) => {
			const monday = addDays(thisMonday, ahead * 7);
			return { monday, days: weekOf(monday).filter((day) => day >= now) };
		}),
	);
	const last = $derived(addDays(thisMonday, shown * 7 - 1));

	/** A meal a row names: its name, and whether it is the recipe being planned. */
	type Held = { name: string; ours: boolean };
	/** What is already planned on each day, and on each week with no day, once read. */
	let holds = $state<Record<string, Held[]>>({});

	$effect(() => {
		const to = last;
		let current = true;
		kamosu
			.getMealPlan({ from: now, to })
			.then((answer: GetMealPlanOutput) => {
				if (!current) return;
				const by: Record<string, Held[]> = {};
				for (const meal of answer.planned_meals) {
					if (meal.planned_meal_id === skip) continue;
					(by[keyOf(whereOf(meal))] ??= []).push({
						name: mealName(meal),
						ours: branchId !== null && meal.branch_id === branchId,
					});
				}
				holds = by;
			})
			// What a day holds is a help, not the point: the days can be
			// chosen without it.
			.catch(() => {});
		return () => {
			current = false;
		};
	});

	const keyOf = (where: Where): string =>
		'day' in where ? `day:${where.day}` : `week:${where.week}`;
	const value = $derived(chosen ? keyOf(chosen) : null);

	function weekName(monday: Day): string {
		const ahead = weeksFrom(thisMonday, monday);
		if (ahead === 0) return m.plan_this_week();
		if (ahead === 1) return m.plan_next_week();
		if (ahead === 2) return m.plan_the_week_after();
		return m.plan_weeks_ahead({ count: ahead });
	}

	function dayName(day: Day): string {
		if (day === now) return m.plan_today_day({ day: aPlannedDay(day) });
		if (day === addDays(now, 1)) return m.plan_tomorrow_day({ day: aPlannedDay(day) });
		return aPlannedDay(day);
	}
</script>

{#snippet row(where: Where, name: string)}
	{@const key = keyOf(where)}
	{@const picked = value === key}
	<li>
		<label
			class="flex min-h-[52px] items-center gap-3 border-b border-rule py-2 ps-1 pe-2 {picked
				? 'bg-card shadow-[inset_3px_0_0_var(--color-accent)]'
				: ''}"
		>
			<input
				type="radio"
				name="{uid}-where"
				value={key}
				class="h-[20px] w-[20px] shrink-0 accent-accent"
				checked={value === key}
				onchange={() => onchoose(where)}
				data-sheet-focus={'day' in where && where.day === now ? true : undefined}
			/>
			<span class="min-w-0 flex-1">
				<span class="block text-body {picked ? 'font-semibold' : ''}">{name}</span>
				{#if holds[key]}
					<span class="block text-read text-ink"
						>{#each holds[key] as meal, at (at)}{#if at > 0}<span>
									·
								</span>{/if}{#if meal.ours}<span class="font-semibold text-accent">{meal.name}</span
								>{:else}{meal.name}{/if}{/each}</span
					>
				{/if}
				{#if picked && joining}
					<span class="block text-read font-semibold text-accent">+ {joining}</span>
				{/if}
			</span>
		</label>
	</li>
{/snippet}

<fieldset class="mt-4">
	<legend class="text-label text-ink uppercase">{m.plan_which_day()}</legend>
	{#each weeks as week (week.monday)}
		<p
			class="sticky top-[-16px] z-10 mt-3 flex items-baseline justify-between gap-3 border-b-2 border-accent bg-ground pt-2 pb-1 text-label font-semibold text-accent uppercase"
		>
			{weekName(week.monday)}
			<span class="text-read font-normal tracking-[0] text-ink-2 normal-case"
				>{aPlannedWeek(week.monday, addDays(week.monday, 6))}</span
			>
		</p>
		<ul>
			{@render row({ week: week.monday }, anyDayName(week.monday))}
			{#each week.days as day (day)}
				{@render row({ day }, dayName(day))}
			{/each}
		</ul>
	{/each}
	<button type="button" class="mt-3 second-button w-full" onclick={() => (shown += 3)}>
		{m.plan_more_weeks()}
	</button>
</fieldset>
