<!--
	The Plan: one Person's Meal Plan, a week at a time (#486, #487, ADR 0058).

	**What it looks like was chosen on 31 October 2026**, option B of the
	prototype recorded on #486: a calendar week. On the phone each day is a row,
	its date in a block on the left and its meals beside it; on the wide
	layout the seven days stand side by side as columns, each recipe a card
	with its photograph. A week runs Monday to Sunday, arrows move a week at a
	time, and a week with nothing planned says how to plan something.

	**#487 added, the same day, from its own prototype:** the week's meals with
	no day above the days (option A), a shaded row before Monday on the phone
	and a band across the top on the wide layout, kept when empty and saying
	so; a + on every day and on that list, to write words such as "leftovers"
	(option A); written words drawn as a pencil in a dashed square; and on the
	wide layout, any meal dragged to another day, to the week's list, or up
	and down. On the phone, moving is in the meal's sheet (option A).

	Today is the filled date on the phone and carries a tag on the wide layout.
	Days of the week already gone are drawn faded, and stay: a plan is not
	emptied as the week goes by.

	A planned recipe opens at its planned Yield. One that can no longer be read
	keeps its name, says so, and offers only to be moved or taken off.

	**#488 added sending to the Shopping List**, from its prototype on 31
	October 2026: one button, "Send to shopping", under every week (option A),
	opening the sheet that chooses the days; the answer under the week, naming
	what went, what had gone before and what was left behind; and under a
	recipe that went, "Sent to shopping", or "Sent for 4 · 2 servings more to
	send" once planned for more, or "Sent for 4" once planned for fewer
	(option A). It says what went, never what the list holds, so it stays true
	once the list is emptied.
-->
<script lang="ts">
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import type { GetMealPlanOutput, SendToShoppingListOutput } from '$lib/api/catalogue';
	import Screen from '$lib/shell/Screen.svelte';
	import { landedSend, rereads } from '$lib/offline/device.svelte';
	import { useRoom } from '$lib/room.svelte';
	import { aPlannedDay, aPlannedWeek, aPlannedWeekday } from '$lib/dates';
	import { isMultiplier, said, toSearch } from '$lib/how-much';
	import {
		addDays,
		isDay,
		mondayOf,
		sameWhere,
		today,
		weekOf,
		weekOfWhere,
		weeksFrom,
		whereOf,
		type Day,
		type Where,
	} from '$lib/plan-days';
	import { anyDayName, mealName, weekInline, whereName, whereSaid } from '$lib/plan-where';
	import { TICK, amountOf, countOf, sentOf } from '$lib/plan-sent';
	import { yieldScale } from '$lib/offline/shopping';
	import ChangeSheet from './ChangeSheet.svelte';
	import SendToShopping from './SendToShopping.svelte';
	import MoveSheet from './MoveSheet.svelte';
	import WriteSheet from './WriteSheet.svelte';

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
	/** A move refused by the server, said under the week until the next one. */
	let moveFailed = $state(false);
	/** The Planned Meal whose sheet is open, by id, so a read again shows it as it now is. */
	let changingId = $state<string | null>(null);
	const changing = $derived(meals?.find((meal) => meal.planned_meal_id === changingId) ?? null);
	/** Moving a meal, or planning it again: the sheet with the list of days. */
	let moving = $state<{ meal: Meal; again: boolean } | null>(null);
	/** Where words are being written, from a day's +. */
	let writing = $state<Where | null>(null);
	/** What the last move or plan again did, said under the week. */
	let notice = $state<{ text: string; week: Day } | null>(null);
	/** Asked again after a write that moves the order of a list. */
	let again = $state(0);
	/** Choosing what to send to the Shopping List. */
	let sending = $state(false);
	/** What the last send to the Shopping List did, said under the week; `back` once a held one landed. */
	let sent = $state<{ answer: SendToShoppingListOutput; back: boolean } | null>(null);

	/** Read again once writes made with no network have reached the server (#77). */
	const reread = rereads('get_meal_plan');

	// A send held with no network has landed: say what it did, and put it away.
	$effect(() => {
		const answer = landedSend.answer;
		if (!answer) return;
		sent = { answer, back: true };
		landedSend.answer = null;
		again += 1;
	});

	/** A send answered: said under the week, and the week read again for its marks. */
	function answered(answer: SendToShoppingListOutput) {
		sent = { answer, back: false };
		sending = false;
		notice = null;
		again += 1;
	}

	type Went = SendToShoppingListOutput['sent'][number];

	/**
	 * One recipe that went: `Chili, 8 people, for 2 days · now 12 people on
	 * the list`, or `Katsu Curry, 2 servings more` for one planned for more.
	 */
	function wentLine(went: Went): string {
		const amount = amountOf(went.written_yield, went.added_factor);
		let line: string = went.raised
			? m.plan_sent_line_more({ title: went.title, amount })
			: m.plan_sent_line({ title: went.title, amount });
		if (went.planned_meals.length > 1)
			line += m.plan_sent_days({ count: went.planned_meals.length });
		if (went.was_on_list)
			line += m.plan_sent_now({
				amount: amountOf(went.written_yield, yieldScale(went.shopping_yield, went.written_yield)),
			});
		return line;
	}

	/**
	 * The recipes that had gone before and sent nothing now, each named once:
	 * not one that went now for another day, nor one named as planned for
	 * fewer.
	 */
	function alreadySent(answer: SendToShoppingListOutput): string[] {
		const went = new Set(answer.sent.map((each) => each.branch_id));
		const fewer = new Set(
			answer.already_sent.filter((meal) => meal.planned_fewer).map((meal) => meal.branch_id),
		);
		const named = answer.already_sent.filter(
			(meal) => !went.has(meal.branch_id) && !fewer.has(meal.branch_id),
		);
		return [...new Map(named.map((meal) => [meal.branch_id, meal.title])).values()];
	}

	/** A list of names in a sentence. */
	const listed = (names: string[]) => names.join(', ');

	$effect(() => {
		void reread.count;
		void again;
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

	/** One list of the week: a day's, or the week's with no day, in its order. */
	const listOf = (where: Where) =>
		(meals ?? []).filter((meal) =>
			'day' in where ? meal.day === where.day : meal.day === null && meal.week === where.week,
		);
	const anyDay = $derived<Where>({ week: monday });
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
		if (!now && changingId === id) changingId = null;
	}

	/**
	 * Move a meal to a place, at a position in that list, and read the week
	 * again. A refusal is the caller's to say: the sheet says it in itself, a
	 * drag under the week.
	 */
	async function move(meal: Meal, where: Where, position?: number): Promise<void> {
		await kamosu.movePlannedMeal({
			planned_meal_id: meal.planned_meal_id,
			...where,
			...(position === undefined ? {} : { position }),
		});
		again += 1;
	}

	/** A move from a drag, whose refusal is said under the week. */
	async function dragMove(meal: Meal, where: Where, position: number) {
		moveFailed = false;
		notice = null;
		try {
			await move(meal, where, position);
		} catch (error: unknown) {
			if (!(error instanceof OperationError)) throw error;
			moveFailed = true;
		}
	}

	/** Moved, or planned again, from the list of days: said under the week. */
	async function moved(meal: Meal, where: Where, isAgain: boolean) {
		if (isAgain) {
			await (meal.words !== null
				? kamosu.planWords({ words: meal.words, ...where })
				: kamosu.planRecipe({
						branch_id: meal.branch_id ?? '',
						...where,
						...(meal.planned_yield ? { planned_yield: meal.planned_yield } : {}),
					}));
			again += 1;
		} else await move(meal, where);
		const name = mealName(meal);
		const text = isAgain
			? m.plan_planned_again({ meal: name, where: whereSaid(where) })
			: m.plan_moved({ meal: name, where: whereSaid(where) });
		notice = { text, week: weekOfWhere(where) };
		moveFailed = false;
		moving = null;
		changingId = null;
	}

	// ── Dragging, on the wide layout ─────────────────────────────────────────
	//
	// A meal is dragged to a day, to the week's list with no day, or up and
	// down in its own. Where it would land is drawn as a line; letting go moves
	// it there with the same Operation the sheet's Move uses.

	let dragged = $state<Meal | null>(null);
	let over = $state<{ where: Where; index: number } | null>(null);

	/** Where in a list the pointer is: the number of other meals before it. */
	function indexAt(zone: HTMLElement, event: DragEvent, across: boolean): number {
		let index = 0;
		for (const item of zone.querySelectorAll<HTMLElement>('[data-meal]')) {
			if (item.dataset.meal === dragged?.planned_meal_id) continue;
			const box = item.getBoundingClientRect();
			const after = across
				? event.clientY > box.bottom ||
					(event.clientY > box.top && event.clientX > box.left + box.width / 2)
				: event.clientY > box.top + box.height / 2;
			if (after) index += 1;
		}
		return index;
	}

	function dragOver(event: DragEvent, where: Where, across = false) {
		if (!dragged) return;
		event.preventDefault();
		over = { where, index: indexAt(event.currentTarget as HTMLElement, event, across) };
	}

	/** The pointer left a list for somewhere that is not inside it: nothing is lit. */
	function dragLeave(event: DragEvent) {
		const zone = event.currentTarget as HTMLElement;
		if (!zone.contains(event.relatedTarget as Node | null)) over = null;
	}

	async function drop(event: DragEvent, where: Where, across = false) {
		if (!dragged) return;
		event.preventDefault();
		const meal = dragged;
		const index = indexAt(event.currentTarget as HTMLElement, event, across);
		dragged = null;
		over = null;
		await dragMove(meal, where, index);
	}

	/**
	 * Whether a meal is dragged over this list. An empty list keeps its box
	 * and lights it, rather than shrinking to a line under the pointer.
	 */
	const overIt = (where: Where) => over !== null && sameWhere(over.where, where);

	/** Whether the line showing where a dragged meal lands goes before this one. */
	const lineBefore = (where: Where, index: number) =>
		over !== null && sameWhere(over.where, where) && over.index === index;
	/** The others in a list while one is dragged, so the line counts as the server will. */
	const others = (list: Meal[]) =>
		dragged ? list.filter((meal) => meal.planned_meal_id !== dragged?.planned_meal_id) : list;

	const arrow = (back: boolean) => (back ? 'M15 5l-7 7 7 7' : 'M9 5l7 7-7 7');
	const CART = 'M3 4h2l2.2 10.2a1.5 1.5 0 0 0 1.5 1.2h8.6a1.5 1.5 0 0 0 1.5-1.1L20.5 8H6.2';
	const PENCIL = 'M4 20h4L19 9l-4-4L4 16v4ZM13.5 6.5l4 4';
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
				onclick={() => {
					monday = addDays(monday, back ? -7 : 7);
					notice = null;
					sent = null;
					moveFailed = false;
				}}
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
				onclick={() => {
					monday = thisMonday;
					notice = null;
					sent = null;
					moveFailed = false;
				}}
			>
				{m.plan_back_to_this_week()}
			</button>
		</p>
	{/if}
	<div class="mt-2 flex justify-end">
		<button
			type="button"
			class="inline-flex min-h-12 items-center gap-2 rounded-sm border border-accent bg-card px-4 text-read font-medium text-accent"
			onclick={() => (sending = true)}
		>
			<svg
				viewBox="0 0 24 24"
				class="h-[18px] w-[18px]"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
				><path d={CART} /><circle cx="9.5" cy="19.5" r="1.3" /><circle
					cx="17"
					cy="19.5"
					r="1.3"
				/></svg
			>
			{m.plan_send()}
		</button>
	</div>
	{#if sent}
		{@render sentNote(sent.answer, sent.back)}
	{/if}
	{#if room.wide && meals !== undefined && !empty}
		<p class="mt-2 text-read text-ink-2">{m.plan_drag_hint()}</p>
	{/if}
	{#if moveFailed}
		<p class="mt-3 text-read text-support" role="alert">{m.plan_write_failed()}</p>
	{/if}
	{#if notice}
		<p class="mt-3 border-l-3 border-accent bg-card px-3 py-2 text-read" role="status">
			{notice.text}
			{#if notice.week !== monday}
				{@const week = notice.week}
				<button
					type="button"
					class="ms-1 text-accent underline"
					onclick={() => {
						monday = week;
						notice = null;
						moveFailed = false;
					}}>{m.plan_see_week({ week: weekInline(week) })}</button
				>
			{/if}
		</p>
	{/if}
{/snippet}

{#snippet sentNote(answer: SendToShoppingListOutput, back: boolean)}
	<div
		class="mt-3 rounded-sm border-l-3 bg-ground-2 px-3 py-2 text-read text-ink {answer.outcome ===
		'refused'
			? 'border-support'
			: 'border-support-2'}"
		role="status"
	>
		{#if answer.outcome === 'waiting'}
			<p><strong>{m.plan_send_waiting()}</strong> {m.plan_send_waiting_more()}</p>
		{:else if answer.outcome === 'refused'}
			<p><strong>{m.plan_send_refused()}</strong> {m.plan_send_refused_more()}</p>
		{:else}
			{#if answer.sent.length > 0}
				<p><strong>{back ? m.plan_sent_back() : m.plan_sent_heading()}</strong></p>
				<ul class="mt-1 list-disc ps-gutter">
					{#each answer.sent as went (went.branch_id)}
						<li>{wentLine(went)}</li>
					{/each}
				</ul>
				{#if alreadySent(answer).length > 0}
					<p class="mt-1">
						{m.plan_sent_already({ titles: listed(alreadySent(answer)) })}
					</p>
				{/if}
			{:else if answer.skipped.length === 0 && answer.already_sent.length === 0}
				<p><strong>{m.plan_sent_none()}</strong></p>
			{:else if answer.skipped.length === 0}
				<p><strong>{m.plan_sent_nothing()}</strong> {m.plan_sent_nothing_more()}</p>
			{:else}
				<p><strong>{m.plan_sent_nothing()}</strong></p>
				{#if alreadySent(answer).length > 0}
					<p class="mt-1">
						{m.plan_sent_already({ titles: listed(alreadySent(answer)) })}
					</p>
				{/if}
			{/if}
			{@const fewer = answer.already_sent.filter((meal) => meal.planned_fewer)}
			{#if fewer.length > 0}
				<p class="mt-1">
					{m.plan_sent_fewer({
						titles: listed([
							...new Map(fewer.map((meal) => [meal.branch_id, meal.title])).values(),
						]),
					})}
				</p>
			{/if}
			{#if answer.skipped.length > 0}
				<p class="mt-1">
					{m.plan_sent_not({
						meals: answer.skipped
							.map((meal) =>
								meal.words !== null
									? m.plan_sent_words({ words: meal.words })
									: m.plan_sent_gone({ title: meal.title ?? '' }),
							)
							.join(', '),
					})}
				</p>
			{/if}
		{/if}
		<p class="mt-2">
			<a href="/shopping" class="text-accent underline">{m.plan_sent_see_list()}</a>
		</p>
	</div>
{/snippet}

{#snippet sentMark(meal: Meal)}
	{@const state = sentOf(meal)}
	{#if state.kind !== 'none'}
		<span
			class="mt-[2px] flex items-start gap-1 text-read {state.kind === 'more'
				? 'text-accent'
				: 'text-support-2'}"
		>
			<svg
				viewBox="0 0 24 24"
				class="mt-[3px] h-[16px] w-[16px] shrink-0"
				fill="none"
				stroke="currentColor"
				stroke-width="2.2"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"><path d={TICK} /></svg
			>
			<span>
				{#if state.kind === 'sent'}
					{m.plan_sent_mark()}
				{:else if state.kind === 'fewer'}
					{m.plan_sent_for({ amount: countOf(meal.written_yield, state.sent) })}
				{:else}
					{m.plan_sent_for_more({
						amount: countOf(meal.written_yield, state.sent),
						more: amountOf(meal.written_yield, state.more),
					})}
				{/if}
			</span>
		</span>
	{/if}
{/snippet}

{#snippet pencil(size: string)}
	<span
		aria-hidden="true"
		class="{size} flex shrink-0 items-center justify-center rounded-sm border border-dashed border-rule text-ink-2"
	>
		<svg
			viewBox="0 0 24 24"
			class="h-[22px] w-[22px]"
			fill="none"
			stroke="currentColor"
			stroke-width="1.7"
			stroke-linecap="round"
			stroke-linejoin="round"><path d={PENCIL} /></svg
		>
	</span>
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
			{meal.gone ? '–' : (meal.title ?? '').slice(0, 1)}
		</span>
	{/if}
{/snippet}

{#snippet changeButton(meal: Meal, size: string)}
	<button
		type="button"
		class="{size} text-read text-accent underline"
		onclick={() => (changingId = meal.planned_meal_id)}
	>
		{meal.gone ? m.plan_remove() : m.plan_change()}
	</button>
{/snippet}

{#snippet row(meal: Meal)}
	<li class="flex items-center gap-3 py-2">
		{#if meal.words !== null}
			<span class="flex min-w-0 flex-1 items-center gap-3">
				{@render pencil('h-[52px] w-[52px]')}
				<span class="min-w-0 text-body break-words text-ink">{meal.words}</span>
			</span>
		{:else if meal.gone}
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
					{@render sentMark(meal)}
				</span>
			</a>
		{/if}
		{@render changeButton(meal, 'min-h-12 shrink-0 px-1')}
	</li>
{/snippet}

{#snippet card(meal: Meal)}
	<li
		class="overflow-hidden rounded-sm border border-rule {meal.words !== null
			? 'border-dashed bg-ground-2'
			: 'bg-card'} cursor-grab {dragged?.planned_meal_id === meal.planned_meal_id
			? 'opacity-40'
			: ''}"
		draggable="true"
		data-meal={meal.planned_meal_id}
		ondragstart={(event) => {
			dragged = meal;
			event.dataTransfer?.setData('text/plain', meal.planned_meal_id);
			if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
		}}
		ondragend={() => {
			dragged = null;
			over = null;
		}}
	>
		{#if meal.words !== null}
			<span class="flex items-start gap-1 px-2 pt-2">
				<svg
					viewBox="0 0 24 24"
					class="mt-[2px] h-[18px] w-[18px] shrink-0 text-ink-2"
					fill="none"
					stroke="currentColor"
					stroke-width="1.7"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"><path d={PENCIL} /></svg
				>
				<span class="min-w-0 text-read break-words text-ink">{meal.words}</span>
			</span>
		{:else if meal.gone}
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
					{@render sentMark(meal)}
				</span>
			</a>
		{/if}
		{@render changeButton(meal, 'min-h-[40px] px-2 text-left')}
	</li>
{/snippet}

{#snippet dropLine(where: Where, index: number)}
	{#if lineBefore(where, index)}
		<li aria-hidden="true" class="h-[4px] rounded-pill bg-accent"></li>
	{/if}
{/snippet}

{#snippet cards(where: Where, list: Meal[], gridClass: string)}
	<ul class={gridClass}>
		{#each list as meal (meal.planned_meal_id)}
			{#if meal.planned_meal_id !== dragged?.planned_meal_id}
				{@render dropLine(where, others(list).indexOf(meal))}
			{/if}
			{@render card(meal)}
		{/each}
		{@render dropLine(where, others(list).length)}
	</ul>
{/snippet}

{#snippet plus(where: Where, wide: boolean)}
	<button
		type="button"
		class={wide
			? 'mt-2 min-h-[36px] w-full rounded-sm border border-dashed border-accent text-read text-accent'
			: 'mt-2 flex h-[32px] w-[40px] items-center justify-center rounded-sm border border-dashed border-accent bg-card text-line text-accent'}
		aria-label={m.plan_add_to({ where: whereName(where) })}
		onclick={() => (writing = where)}
	>
		<span aria-hidden="true">+{wide ? ` ${m.plan_add()}` : ''}</span>
	</button>
{/snippet}

<Screen title={m.plan_title()} blurb={m.plan_blurb()} fills>
	{@render weekBar()}

	{#if failed && meals === undefined}
		<p class="mt-6 text-read text-support" role="alert">{m.plan_failed()}</p>
	{:else if meals !== undefined}
		{#if empty}
			<!-- Above the week rather than in its place, so each day keeps the +
			     the sentence names (#487). -->
			<div
				class="mt-6 max-w-2xl rounded-sm border border-dashed border-rule bg-ground-2 p-6 text-body text-ink-2"
			>
				<p>{m.plan_empty()}</p>
				<p class="mt-2">{m.plan_empty_how_write()}</p>
				<a
					href="/recipes"
					class="mt-4 inline-flex min-h-12 items-center rounded-sm border border-accent px-4 font-medium text-accent"
				>
					{m.plan_empty_choose()}
				</a>
			</div>
		{/if}
		{#if room.wide}
			{@const list = listOf(anyDay)}
			<section
				class="mt-4 rounded-sm bg-ground-2 px-3 pt-2 pb-3"
				aria-label={anyDayName(monday)}
				ondragover={(event) => dragOver(event, anyDay, true)}
				ondragleave={dragLeave}
				ondrop={(event) => void drop(event, anyDay, true)}
			>
				<h2
					class="border-b-2 border-accent pt-1 pb-2 text-label font-semibold text-accent uppercase"
				>
					{anyDayName(monday)}
				</h2>
				{#if list.length > 0}
					{@render cards(anyDay, list, 'mt-2 grid grid-cols-7 gap-2')}
				{:else}
					<p class="mt-2 text-read {overIt(anyDay) ? 'text-accent' : 'text-ink-2'}">
						{m.plan_any_empty_short()}
					</p>
				{/if}
				<div class="w-[160px]">{@render plus(anyDay, true)}</div>
			</section>
			<div class="mt-4 grid grid-cols-7 gap-2">
				{#each days as day (day)}
					{@const list = listOf({ day })}
					<section
						class={day < now ? 'opacity-60' : ''}
						role="group"
						aria-label={aPlannedDay(day)}
						ondragover={(event) => dragOver(event, { day })}
						ondragleave={dragLeave}
						ondrop={(event) => void drop(event, { day })}
					>
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
						{#if list.length > 0}
							{@render cards({ day }, list, 'mt-2 grid gap-2')}
						{:else}
							<p
								class="mt-2 rounded-sm border border-dashed p-4 text-center text-read {overIt({
									day,
								})
									? 'border-accent text-accent'
									: 'border-rule text-ink-2'}"
							>
								{m.plan_nothing_short()}
							</p>
						{/if}
						{@render plus({ day }, true)}
					</section>
				{/each}
			</div>
		{:else}
			{@const list = listOf(anyDay)}
			<div class="mt-2">
				<section
					class="my-2 flex gap-3 rounded-sm bg-ground-2 py-2 pr-2 {away < 0 ? 'opacity-60' : ''}"
				>
					<div class="flex w-[52px] shrink-0 flex-col items-center">
						<h2
							class="pt-2 text-center text-label leading-[1.2] font-semibold text-accent uppercase"
						>
							{m.plan_any_day()}
							<span class="sr-only">{anyDayName(monday)}</span>
						</h2>
						{@render plus(anyDay, false)}
					</div>
					{#if list.length > 0}
						<ul class="min-w-0 flex-1 divide-y divide-rule">
							{#each list as meal (meal.planned_meal_id)}
								{@render row(meal)}
							{/each}
						</ul>
					{:else}
						<p class="flex-1 py-3 text-read text-ink-2">{m.plan_any_empty()}</p>
					{/if}
				</section>
				{#each days as day (day)}
					{@const list = listOf({ day })}
					<section class="flex gap-3 border-b border-rule py-2 {day < now ? 'opacity-60' : ''}">
						<div class="flex w-[52px] shrink-0 flex-col items-center">
							<h2 class="w-[52px] pt-1 text-center">
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
							{@render plus({ day }, false)}
						</div>
						{#if list.length > 0}
							<ul class="min-w-0 flex-1 divide-y divide-rule">
								{#each list as meal (meal.planned_meal_id)}
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

{#if changing && !moving}
	{@const list = listOf(whereOf(changing))}
	<ChangeSheet
		meal={changing}
		place={list.findIndex((meal) => meal.planned_meal_id === changing.planned_meal_id)}
		count={list.length}
		{landed}
		onClose={() => (changingId = null)}
		opens={opens(changing)}
		onOrder={(position) => move(changing, whereOf(changing), position)}
		onMove={() => (moving = { meal: changing, again: false })}
		onAgain={() => (moving = { meal: changing, again: true })}
	/>
{/if}

{#if moving}
	{@const meal = moving.meal}
	{@const isAgain = moving.again}
	<MoveSheet
		{meal}
		again={isAgain}
		onChosen={(where) => moved(meal, where, isAgain)}
		onClose={() => (moving = null)}
	/>
{/if}

{#if sending}
	<SendToShopping onSent={answered} onClose={() => (sending = false)} />
{/if}

{#if writing}
	<WriteSheet
		where={writing}
		onWritten={() => {
			writing = null;
			again += 1;
		}}
		onClose={() => (writing = null)}
	/>
{/if}
