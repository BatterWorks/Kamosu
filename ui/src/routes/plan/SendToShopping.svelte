<!--
	Sending days and weeks of the plan to the Shopping List (#488).

	**Chosen on 31 October 2026**, option A of the prototype recorded on #488:
	one button, then this sheet. It lists this week, next week and the one
	after. Each week has a box that takes all of it, its "any day" list
	included; under it, the "any day" row and each day holding something, each
	with a box and what it would send: a recipe at its planned amount, "2
	servings more" for one planned for more since it went, "sent" for one with
	nothing more to send, and written words and a recipe that can no longer be
	read saying they stay. Days already gone are faded and can still be ticked.
	The button counts the recipes that would go.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput, SendToShoppingListOutput } from '$lib/api/catalogue';
	import { aPlannedDay, aPlannedWeek } from '$lib/dates';
	import { addDays, mondayOf, today, weekOf, weeksFrom, type Day } from '$lib/plan-days';
	import { weekInline } from '$lib/plan-where';
	import { TICK, amountOf, factorOf, sentOf } from '$lib/plan-sent';

	type Meal = GetMealPlanOutput['planned_meals'][number];

	const PENCIL = 'M4 20h4L19 9l-4-4L4 16v4ZM13.5 6.5l4 4';

	interface Props {
		/** Sent: the answer, which the Plan says under the week. */
		onSent: (answer: SendToShoppingListOutput) => void;
		onClose: () => void;
	}

	let { onSent, onClose }: Props = $props();

	const kamosu = useKamosu();
	const uid = $props.id();
	const now = today();
	const thisMonday = mondayOf(now);
	const weeks = [0, 1, 2].map((n) => addDays(thisMonday, 7 * n));

	let meals = $state<Meal[] | undefined>(undefined);
	let failed = $state(false);
	let working = $state(false);
	/** What is ticked: `week:<monday>`, `any:<monday>` or `day:<day>`. */
	let chosen = $state<string[]>([]);

	$effect(() => {
		kamosu
			.getMealPlan({ from: thisMonday, to: addDays(thisMonday, 20) })
			.then((answered) => (meals = answered.planned_meals))
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				failed = true;
			});
	});

	const weekName = (monday: Day) => {
		const away = weeksFrom(thisMonday, monday);
		return away === 0 ? m.plan_this_week() : away === 1 ? m.plan_next_week() : m.plan_week_after();
	};

	/** One row: the week's "any day" list, or a day. */
	interface Row {
		key: string;
		label: string;
		past: boolean;
		meals: Meal[];
	}

	function rowsOf(monday: Day): Row[] {
		const all = meals ?? [];
		const any = all.filter((meal) => meal.week === monday && meal.day === null);
		const rows: Row[] = [
			{ key: `any:${monday}`, label: m.plan_any_day(), past: false, meals: any },
		];
		for (const day of weekOf(monday))
			rows.push({
				key: `day:${day}`,
				label: day === now ? `${m.plan_today()} · ${aPlannedDay(day)}` : aPlannedDay(day),
				past: day < now,
				meals: all.filter((meal) => meal.day === day),
			});
		return rows.filter((row) => row.meals.length > 0);
	}

	/** The meals what is ticked covers, each once. */
	const covered = $derived(
		(meals ?? []).filter((meal) =>
			chosen.some((key) => {
				const [kind, at] = key.split(':');
				if (kind === 'week') return meal.week === at;
				if (kind === 'any') return meal.week === at && meal.day === null;
				return meal.day === at;
			}),
		),
	);
	/** How many recipes would go: each Branch once, where something is left to send. */
	const going = $derived(
		new Set(
			covered
				.filter((meal) => meal.words === null && !meal.gone)
				.filter((meal) => ['none', 'more'].includes(sentOf(meal).kind))
				.map((meal) => meal.branch_id),
		).size,
	);

	function toggle(key: string) {
		if (chosen.includes(key)) {
			chosen = chosen.filter((each) => each !== key);
			return;
		}
		// A whole week takes the place of its own rows.
		const monday = key.startsWith('week:') ? key.slice(5) : null;
		chosen = [
			...chosen.filter(
				(each) =>
					!monday ||
					(each !== `any:${monday}` &&
						!(each.startsWith('day:') && mondayOf(each.slice(4)) === monday)),
			),
			key,
		];
	}

	async function send() {
		if (chosen.length === 0 || working) return;
		working = true;
		const ticked = (kind: string) =>
			chosen.filter((key) => key.startsWith(`${kind}:`)).map((key) => key.slice(kind.length + 1));
		try {
			onSent(
				await kamosu.sendToShoppingList({
					days: ticked('day'),
					weeks: ticked('week'),
					any_day_of: ticked('any'),
				}),
			);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			failed = true;
		} finally {
			working = false;
		}
	}

	/** What one meal would send, as its row says it. */
	const status = (meal: Meal): { text: string; more: string; quiet: boolean; sent: boolean } => {
		const title = meal.title ?? '';
		if (meal.words !== null)
			return { text: meal.words, more: m.plan_send_stays(), quiet: true, sent: false };
		if (meal.gone) return { text: title, more: m.plan_send_gone(), quiet: true, sent: false };
		const sent = sentOf(meal);
		if (sent.kind === 'more')
			return {
				text: title,
				more: m.plan_send_more({ amount: amountOf(meal.written_yield, sent.more) }),
				quiet: false,
				sent: false,
			};
		if (sent.kind === 'sent' || sent.kind === 'fewer')
			return { text: title, more: m.plan_send_was_sent(), quiet: true, sent: true };
		return {
			text: title,
			more: amountOf(meal.written_yield, factorOf(meal)),
			quiet: false,
			sent: false,
		};
	};
</script>

{#snippet box(on: boolean, faded: boolean)}
	<span
		aria-hidden="true"
		class="mt-[1px] flex h-6 w-6 shrink-0 items-center justify-center rounded-sm border-2 border-accent {on
			? 'bg-accent text-on-accent'
			: 'bg-card'} {faded ? 'opacity-50' : ''}"
	>
		{#if on}
			<svg
				viewBox="0 0 24 24"
				class="h-[18px] w-[18px]"
				fill="none"
				stroke="currentColor"
				stroke-width="3"
				stroke-linecap="round"
				stroke-linejoin="round"><path d={TICK} /></svg
			>
		{/if}
	</span>
{/snippet}

<SheetFrame labelledby="{uid}-title" tall={88} class="flex flex-col bg-ground" onclose={onClose}>
	<div class="min-h-0 flex-1 overflow-y-auto px-gutter pt-4 pb-4">
		<p class="text-label text-support-2 uppercase">{m.plan_send()}</p>
		<h3 id="{uid}-title" class="mt-1 font-display text-line font-semibold">
			{m.plan_send_title()}
		</h3>
		<p class="mt-2 text-read text-ink-2">{m.plan_send_blurb()}</p>
		{#if meals === undefined && failed}
			<p class="mt-4 text-read text-support" role="alert">{m.plan_failed()}</p>
		{:else if meals !== undefined}
			{#each weeks as monday (monday)}
				{@const whole = chosen.includes(`week:${monday}`)}
				<div class="mt-4">
					<button
						type="button"
						class="flex min-h-12 w-full flex-wrap items-center gap-x-3 py-2 text-left"
						aria-pressed={whole}
						onclick={() => toggle(`week:${monday}`)}
					>
						{@render box(whole, false)}
						<span class="font-display text-line font-semibold text-ink">{weekName(monday)}</span>
						<span class="basis-full ps-[36px] text-read text-ink-2">
							{m.plan_send_all_of({
								week: weekInline(monday),
								dates: aPlannedWeek(monday, addDays(monday, 6)),
							})}
						</span>
					</button>
					<ul class="mt-1 border-t border-rule">
						{#each rowsOf(monday) as row (row.key)}
							{@const on = whole || chosen.includes(row.key)}
							<li class="border-b border-rule">
								<button
									type="button"
									class="flex min-h-12 w-full items-start gap-3 py-2 text-left"
									aria-pressed={on}
									disabled={whole}
									onclick={() => toggle(row.key)}
								>
									{@render box(on, whole)}
									<span class="flex min-w-0 flex-col gap-[2px] {row.past ? 'opacity-60' : ''}">
										<span class="text-body font-semibold text-ink">{row.label}</span>
										{#each row.meals as meal (meal.planned_meal_id)}
											{@const said = status(meal)}
											<span class="text-read break-words {said.quiet ? 'text-ink-2' : 'text-ink'}">
												{#if meal.words !== null}
													<svg
														viewBox="0 0 24 24"
														class="me-1 inline h-[15px] w-[15px] align-[-2px]"
														fill="none"
														stroke="currentColor"
														stroke-width="1.7"
														stroke-linecap="round"
														stroke-linejoin="round"
														aria-hidden="true"><path d={PENCIL} /></svg
													>
												{/if}
												{said.text}
												<span class={said.sent ? 'text-support-2' : 'text-ink-2'}>{said.more}</span>
											</span>
										{/each}
									</span>
								</button>
							</li>
						{/each}
					</ul>
				</div>
			{/each}
		{/if}
		{#if failed && meals !== undefined}
			<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
		{/if}
	</div>
	<div class="grid gap-2 border-t border-rule px-gutter py-3">
		<button
			type="button"
			class="min-h-12 rounded-sm bg-accent px-4 font-display font-semibold text-on-accent disabled:opacity-40"
			disabled={chosen.length === 0 || working}
			onclick={() => void send()}
		>
			{#if chosen.length === 0}
				{m.plan_send_choose()}
			{:else if going > 0}
				{m.plan_send_count({ count: going })}
			{:else}
				{m.plan_send()}
			{/if}
		</button>
		<button type="button" class="min-h-12 text-read text-accent underline" onclick={onClose}>
			{m.plan_cancel()}
		</button>
	</div>
</SheetFrame>
