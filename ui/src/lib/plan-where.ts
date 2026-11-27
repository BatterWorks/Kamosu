/**
 * Where a Planned Meal stands, in words (#487): a day, or any day of a week.
 *
 * One place says each, so "Plan this", Move, Plan again, writing on the plan
 * and the note that answers each name a week the same way.
 */
import { m } from '#lib/paraglide/messages.js';
import { aPlannedDate, aPlannedDay } from '#lib/dates.js';
import { mondayOf, today, weeksFrom, type Day, type Where } from '#lib/plan-days.js';

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

/** Where a meal stands, bare, as a button is aimed and a note says where it went: `Thursday 5 November`, `any day next week`. */
export const whereName = (where: Where): string =>
	'day' in where ? aPlannedDay(where.day) : m.plan_any_day_of({ week: weekInline(where.week) });

/**
 * What the + on a day or a week does, and the title of the sheet it opens:
 * `Add to Thursday 5 November`, `Add to any day next week` (#717). Two phrases,
 * so a Language can put an article before a day: `Ajouter au samedi`.
 */
export const addTo = (where: Where): string =>
	'day' in where
		? m.plan_add_to_day({ day: aPlannedDay(where.day) })
		: m.plan_add_to_any({ week: weekInline(where.week) });

/** Where a meal stands, as a line under its sheet's title: `On Thursday 5 November`, `Any day next week` (#717). */
export const whereStands = (where: Where): string => capitalised(whereSaid(where));

/** What a meal is called on the plan: its recipe's title, or its words. */
export const mealName = (meal: { words: string | null; title: string | null }): string =>
	meal.words ?? meal.title ?? '';

const capitalised = (text: string): string =>
	text.replace(/^\p{L}/u, (first) => first.toUpperCase());
