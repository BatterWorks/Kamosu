/**
 * Where a Planned Meal stands, in words (#487): a day, or any day of a week.
 *
 * One place says each, so "Plan this", Move, Plan again, writing on the plan
 * and the note that answers each name a week the same way.
 */
import { m } from '$lib/paraglide/messages';
import { aPlannedDate, aPlannedDay } from '$lib/dates';
import { mondayOf, today, weeksFrom, type Day, type Where } from '$lib/plan-days';

const away = (monday: Day): number => weeksFrom(mondayOf(today()), monday);

/** A week inside a sentence: `this week`, `next week`, `the week of 9 November`. */
export function weekInline(monday: Day): string {
	const weeks = away(monday);
	if (weeks === 0) return m.plan_inline_this_week();
	if (weeks === 1) return m.plan_inline_next_week();
	if (weeks === -1) return m.plan_inline_last_week();
	return m.plan_inline_week_of({ date: aPlannedDate(monday) });
}

/** The heading of a week's list with no day: `Any day this week`. */
export function anyDayName(monday: Day): string {
	const weeks = away(monday);
	if (weeks === 0) return m.plan_any_this_week();
	if (weeks === 1) return m.plan_any_next_week();
	if (weeks === -1) return m.plan_any_last_week();
	return m.plan_any_that_week();
}

/** What a button aims at: `Thursday 5 November`, or the week, `next week`. */
export const whereTarget = (where: Where): string =>
	'day' in where ? aPlannedDay(where.day) : weekInline(where.week);

/** Where something went, in a sentence: `on Thursday 5 November`, `any day next week`. */
export const whereSaid = (where: Where): string =>
	'day' in where
		? m.plan_on_day({ day: aPlannedDay(where.day) })
		: m.plan_any_day_of({ week: weekInline(where.week) });

/** Where a meal stands, bare, as a sheet heads it and a button is aimed: `Thursday 5 November`, `any day next week`. */
export const whereName = (where: Where): string =>
	'day' in where ? aPlannedDay(where.day) : m.plan_any_day_of({ week: weekInline(where.week) });

/** Where a meal stands, as a sheet heads it: `Thursday 5 November`, `Any day the week of 9 November`. */
export const whereHeading = (where: Where): string => capitalised(whereName(where));

/** What a meal is called on the plan: its recipe's title, or its words. */
export const mealName = (meal: { words: string | null; title: string | null }): string =>
	meal.words ?? meal.title ?? '';

const capitalised = (text: string): string =>
	text.replace(/^\p{L}/u, (first) => first.toUpperCase());

/** A week as a label starts it: `This week`, `The week of 9 November`. */
export const weekLabel = (monday: Day): string => capitalised(weekInline(monday));
