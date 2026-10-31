<!--
	"Plan this" (#486): put the recipe on a day of your Meal Plan, at a Yield.

	**Chosen on 31 October 2026**, sheet 2 of the prototype recorded on #486:
	the days ahead as a list, today first, each saying what is already planned
	on it, with a heading at each week ("This week", "Next week", "The week
	after") that stays in view while its days scroll. That heading was asked for
	when choosing. Below the days, the recipe page's own "how much" picker,
	starting at the recipe as written.

	It plans the Branch on screen, so a cook choosing between a recipe's
	versions plans the one they are reading.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import HowMuch from '$lib/HowMuch.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput } from '$lib/api/catalogue';
	import { aPlannedDay, aPlannedWeek } from '$lib/dates';
	import type { Wanted, Written } from '$lib/how-much';
	import { addDays, mondayOf, today, weekOf, weeksFrom, type Day } from '$lib/plan-days';

	interface Props {
		branchId: string;
		title: string;
		written: Written;
		/** Planned: on which day, at what. */
		onPlanned: (day: Day, wanted: Wanted) => void;
		onClose: () => void;
	}

	let { branchId, title, written, onPlanned, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();

	const now = today();
	const thisMonday = mondayOf(now);
	/** Today to the Sunday of the week after next, a week at a time. */
	const weeks = [0, 1, 2].map((ahead) => {
		const monday = addDays(thisMonday, ahead * 7);
		return { monday, days: weekOf(monday).filter((day) => day >= now) };
	});
	const last = addDays(thisMonday, 20);

	let chosen = $state<Day | null>(null);
	let wanted = $state<Wanted>(null);
	let working = $state(false);
	let failed = $state(false);
	/** What is already planned on each day, by title, once read. */
	let holds = $state<Record<Day, string[]>>({});

	$effect(() => {
		let current = true;
		kamosu
			.getMealPlan({ from: now, to: last })
			.then((answer: GetMealPlanOutput) => {
				if (!current) return;
				const byDay: Record<Day, string[]> = {};
				for (const meal of answer.planned_meals) (byDay[meal.day] ??= []).push(meal.title);
				holds = byDay;
			})
			// What a day holds is a help, not the point: the days can be
			// chosen without it.
			.catch(() => {});
		return () => {
			current = false;
		};
	});

	function weekName(monday: Day): string {
		const ahead = weeksFrom(thisMonday, monday);
		return ahead === 0
			? m.plan_this_week()
			: ahead === 1
				? m.plan_next_week()
				: m.plan_the_week_after();
	}

	function dayName(day: Day): string {
		if (day === now) return m.plan_today_day({ day: aPlannedDay(day) });
		if (day === addDays(now, 1)) return m.plan_tomorrow_day({ day: aPlannedDay(day) });
		return aPlannedDay(day);
	}

	async function plan() {
		if (!chosen || working) return;
		working = true;
		try {
			await kamosu.planRecipe({
				branch_id: branchId,
				day: chosen,
				...(wanted ? { planned_yield: wanted } : {}),
			});
			onPlanned(chosen, wanted);
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
		<p class="text-label text-support-2 uppercase">{m.plan_this()}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">{title}</h3>
		<fieldset class="mt-4">
			<legend class="text-label text-ink-2 uppercase">{m.plan_which_day()}</legend>
			{#each weeks as week (week.monday)}
				{#if week.days.length > 0}
					<p
						class="sticky top-[-16px] z-10 mt-3 flex items-baseline justify-between gap-3 border-b-2 border-accent bg-ground pt-2 pb-1 text-label font-semibold text-accent uppercase"
					>
						{weekName(week.monday)}
						<span class="text-read font-normal tracking-[0] text-ink-2 normal-case"
							>{aPlannedWeek(week.monday, addDays(week.monday, 6))}</span
						>
					</p>
					<ul>
						{#each week.days as day (day)}
							<li>
								<label
									class="flex min-h-[52px] items-center gap-3 border-b border-rule px-1 py-2 {chosen ===
									day
										? 'bg-ground-2'
										: ''}"
								>
									<input
										type="radio"
										name="{uid}-day"
										value={day}
										class="h-[20px] w-[20px] shrink-0 accent-accent"
										bind:group={chosen}
										data-sheet-focus={day === now ? true : undefined}
									/>
									<span class="min-w-0 flex-1">
										<span class="block text-body">{dayName(day)}</span>
										{#if holds[day]}
											<span class="block truncate text-read text-ink-2"
												>{holds[day].join(' · ')}</span
											>
										{/if}
									</span>
								</label>
							</li>
						{/each}
					</ul>
				{/if}
			{/each}
		</fieldset>
		<div class="mt-4 border-t border-rule pt-3">
			<HowMuch {written} {wanted} room="page" onchoose={(next) => (wanted = next)} />
		</div>
		{#if failed}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="min-h-12 rounded-sm bg-accent px-4 font-display font-semibold text-on-accent disabled:opacity-40"
			disabled={!chosen || working}
			onclick={() => void plan()}
		>
			{chosen ? m.plan_it_for({ day: aPlannedDay(chosen) }) : m.plan_choose_a_day()}
		</button>
		<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
