/**
 * The days of the Meal Plan (#486, ADR 0058).
 *
 * A planned day is a calendar date, `2026-11-05`, with no time and no time
 * zone: the day the cook means wherever they are. So each is worked out here
 * as a date and never as a moment, and the arithmetic is done in UTC, where
 * every day is 24 hours long. A week runs Monday to Sunday.
 */

/** A calendar date as the plan holds one. */
export type Day = string;

const asDate = (day: Day): Date => new Date(`${day}T00:00:00Z`);
const asDay = (date: Date): Day => date.toISOString().slice(0, 10);
const two = (n: number) => String(n).padStart(2, '0');

/** Today where the cook is, as their own clock says. */
export const today = (now: Date = new Date()): Day =>
	`${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;

export const addDays = (day: Day, days: number): Day =>
	asDay(new Date(asDate(day).getTime() + days * 86_400_000));

/** Where a day falls in its week: 0 for Monday, 6 for Sunday. */
export const weekday = (day: Day): number => (asDate(day).getUTCDay() + 6) % 7;

/** The Monday of the week a day is in. */
export const mondayOf = (day: Day): Day => addDays(day, -weekday(day));

/** The seven days of the week that starts on a Monday. */
export const weekOf = (monday: Day): Day[] =>
	Array.from({ length: 7 }, (_, index) => addDays(monday, index));

/** How many weeks one Monday is after another: 1 for next week, −1 for last. */
export const weeksFrom = (from: Day, to: Day): number =>
	Math.round((asDate(to).getTime() - asDate(from).getTime()) / (7 * 86_400_000));

/** Whether a string is a calendar date, as a link to a week carries one. */
export const isDay = (text: string | null): text is Day =>
	text !== null && /^\d{4}-\d{2}-\d{2}$/.test(text) && asDay(asDate(text)) === text;

/**
 * Where a Planned Meal stands (#487): on a day, or on any day of a week, the
 * week named by its Monday.
 */
export type Where = { day: Day } | { week: Day };

/** Where a meal the plan answered stands. */
export const whereOf = (meal: { week: Day; day: Day | null }): Where =>
	meal.day ? { day: meal.day } : { week: meal.week };

/** Whether two places are the same list: one day, or one week's with no day. */
export const sameWhere = (a: Where, b: Where): boolean =>
	'day' in a ? 'day' in b && a.day === b.day : 'week' in b && a.week === b.week;

/** The Monday of the week a place is in. */
export const weekOfWhere = (where: Where): Day =>
	'day' in where ? mondayOf(where.day) : where.week;
