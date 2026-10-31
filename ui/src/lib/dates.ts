/**
 * Every date Kamosu writes, in the Language the interface is set to (#224).
 *
 * The audit of 9 October 2026 found five ways of writing a date on one visit:
 * some calls passed the app's locale and the rest took the browser's, so a
 * French interface could show an English date, and a Session's time carried
 * seconds. One style per use, each named for what it is for, and nothing
 * else calls `toLocaleDateString` directly. `howLongAgo` is for the one time
 * whose point is how recent it is (#275).
 */

import { getLocale } from '$lib/paraglide/runtime';
import { m } from '$lib/paraglide/messages';

/**
 * Kamosu's English is the day-first kind, `25 Sept 2026`. Intl reads a bare
 * `en` as American and writes `Sep 25, 2026`, so English is asked for as
 * British; French and Spanish are day-first already.
 */
const locale = (): string => (getLocale() === 'en' ? 'en-GB' : getLocale());

const at = (when: string | Date): Date => (when instanceof Date ? when : new Date(when));

/** A day in the year, the way the diary names a cooking: `20 August`. */
export const aDay = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { day: 'numeric', month: 'long' });

/** A date: `25 Sept 2026`. */
export const aDate = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' });

/** A date with its minute, as an arrival or a Session's last use is: `7 Oct 2026, 01:58`. */
export const aMoment = (when: string | Date): string =>
	at(when).toLocaleString(locale(), {
		day: 'numeric',
		month: 'short',
		year: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
		hourCycle: 'h23',
	});

/**
 * A time of day, for something that happened today: `13:10`. A time from
 * another day is written as `aMoment` writes it, since a bare hour would put
 * yesterday's into today (#277).
 */
export const aTime = (when: string | Date, now: Date = new Date()): string =>
	at(when).toDateString() === now.toDateString()
		? at(when).toLocaleTimeString(locale(), {
				hour: '2-digit',
				minute: '2-digit',
				hourCycle: 'h23',
			})
		: aMoment(when);

/** A time of day alone, `13:10`, under a heading that already names the day (#371). */
export const aClock = (when: string | Date): string =>
	at(when).toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit', hourCycle: 'h23' });

/** A day of the week and its date, as the Security Record heads one: `Mon 13 Oct` (#371). */
export const aWeekday = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { weekday: 'short', day: 'numeric', month: 'short' });

/**
 * A planned day (#486), which is a date with no time zone and so is written as
 * the date it is, wherever the cook is: `Thursday 5 November`.
 */
export const aPlannedDay = (day: string): string =>
	at(`${day}T00:00:00Z`).toLocaleDateString(locale(), {
		weekday: 'long',
		day: 'numeric',
		month: 'long',
		timeZone: 'UTC',
	});

/** A planned day's weekday alone, as the Plan heads a day: `Thu`. */
export const aPlannedWeekday = (day: string): string =>
	at(`${day}T00:00:00Z`).toLocaleDateString(locale(), { weekday: 'short', timeZone: 'UTC' });

/** The seven days from a Monday, as the Plan heads a week: `26 October – 1 November`. */
export const aPlannedWeek = (monday: string, sunday: string): string => {
	const day = (date: string, month: boolean) =>
		at(`${date}T00:00:00Z`).toLocaleDateString(locale(), {
			day: 'numeric',
			...(month ? { month: 'long' } : {}),
			timeZone: 'UTC',
		});
	const sameMonth = monday.slice(0, 7) === sunday.slice(0, 7);
	return `${day(monday, !sameMonth)} – ${day(sunday, true)}`;
};

/** A month, as the diary heads one: `September 2026`. */
export const aMonth = (when: string | Date): string =>
	at(when).toLocaleDateString(locale(), { month: 'long', year: 'numeric' });

/**
 * How long ago, in the largest whole unit that fits: `8 seconds ago`, `3 days
 * ago`. For a time read to tell whether something is alive, such as when a
 * Runner last asked for work (#275), where a date would have to be compared
 * with the clock. A time a moment ahead, from a clock a little out, is one
 * second ago rather than "now" or "in 2 seconds".
 */
export const howLongAgo = (when: string | Date, now: Date = new Date()): string => {
	const seconds = Math.max(1, Math.floor((now.getTime() - at(when).getTime()) / 1000));
	const [count, unit]: [number, Intl.RelativeTimeFormatUnit] =
		seconds < 60
			? [seconds, 'second']
			: seconds < 3600
				? [Math.floor(seconds / 60), 'minute']
				: seconds < 86_400
					? [Math.floor(seconds / 3600), 'hour']
					: [Math.floor(seconds / 86_400), 'day'];
	return new Intl.RelativeTimeFormat(locale(), { numeric: 'always' }).format(-count, unit);
};

/** Rows filed under the day they happened on, and that day's heading. */
export interface Day<T> {
	heading: string;
	rows: T[];
}

/**
 * Rows by the day `when` says each happened on, in the order given, as the
 * Security Record (#371) and the list of what was sent to the agent (#463)
 * file them: Today, Yesterday, then `Mon 13 Oct`, and a day of another year
 * with its year.
 */
export function byDay<T>(rows: T[], when: (row: T) => string, now: Date = new Date()): Day<T>[] {
	const days: Day<T>[] = [];
	const yesterday = new Date(now);
	yesterday.setDate(now.getDate() - 1);
	for (const row of rows) {
		const day = at(when(row));
		const heading =
			day.toDateString() === now.toDateString()
				? m.security_today()
				: day.toDateString() === yesterday.toDateString()
					? m.security_yesterday()
					: day.getFullYear() === now.getFullYear()
						? aWeekday(day)
						: aDate(day);
		const last = days.at(-1);
		if (last?.heading === heading) last.rows.push(row);
		else days.push({ heading, rows: [row] });
	}
	return days;
}
