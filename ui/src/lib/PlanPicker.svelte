<!--
	The days and weeks a meal can be put on (#486, #487): what "Plan this",
	Move and Plan again choose from.

	**Chosen on 31 October 2026**, option A of the prototype recorded on #487:
	one list, today first, a heading at each week that stays in view while its
	days scroll, and under each heading, first, a shaded row for any day of
	that week, then its days. Each row names what is already planned there.
	The list ends in "Show 3 more weeks", so a meal planned again can go
	further ahead than the three weeks shown first.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import type { GetMealPlanOutput } from '$lib/api/catalogue';
	import { aPlannedDay, aPlannedWeek } from '$lib/dates';
	import { anyDayName, mealName } from '$lib/plan-where';
	import {
		addDays,
		mondayOf,
		today,
		weekOf,
		weeksFrom,
		whereOf,
		type Day,
		type Where,
	} from '$lib/plan-days';

	interface Props {
		/** What is chosen, or null until something is. */
		chosen: Where | null;
		/** A meal left out of what each row names: the one being moved. */
		skip?: string;
		onchoose: (where: Where) => void;
	}

	let { chosen, skip, onchoose }: Props = $props();

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

	/** What is already planned on each day, and on each week with no day, once read. */
	let holds = $state<Record<string, string[]>>({});

	$effect(() => {
		const to = last;
		let current = true;
		kamosu
			.getMealPlan({ from: now, to })
			.then((answer: GetMealPlanOutput) => {
				if (!current) return;
				const by: Record<string, string[]> = {};
				for (const meal of answer.planned_meals) {
					if (meal.planned_meal_id === skip) continue;
					(by[keyOf(whereOf(meal))] ??= []).push(mealName(meal));
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

{#snippet row(where: Where, name: string, any: boolean)}
	{@const key = keyOf(where)}
	<li>
		<label
			class="flex min-h-[52px] items-center gap-3 border-b border-rule px-1 py-2 {any ||
			value === key
				? 'bg-ground-2'
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
				<span class="block text-body {any ? 'font-semibold text-accent' : ''}">{name}</span>
				{#if holds[key]}
					<span class="block truncate text-read text-ink-2">{holds[key].join(' · ')}</span>
				{/if}
			</span>
		</label>
	</li>
{/snippet}

<fieldset class="mt-4">
	<legend class="text-label text-ink-2 uppercase">{m.plan_which_day()}</legend>
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
			{@render row({ week: week.monday }, anyDayName(week.monday), true)}
			{#each week.days as day (day)}
				{@render row({ day }, dayName(day), false)}
			{/each}
		</ul>
	{/each}
	<button
		type="button"
		class="mt-3 min-h-[44px] w-full rounded-sm border border-dashed border-rule text-read text-accent"
		onclick={() => (shown += 3)}
	>
		{m.plan_more_weeks()}
	</button>
</fieldset>
