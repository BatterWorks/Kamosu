/**
 * What the list of everything sent to Kamosu's AI agent says about each row
 * (#463), written apart from the screen so its tests read it without one.
 */

import { m } from '$lib/paraglide/messages';
import { byDay as byDayOf, type Day } from '$lib/dates';
import type { ListAgentJobsOutput } from '$lib/api/catalogue';

export type Sent = ListAgentJobsOutput['jobs'][number];

/** One thing the cook can do about a failure. */
export type Next = Sent['next'][number];

/** Which rows the filter over the list keeps. */
export type Showing = 'all' | 'failed' | 'working';

/** Whether an agent has yet to finish with it: it waits, or is being read. */
export const working = (row: Sent): boolean => row.status === 'queued' || row.status === 'running';

/** Whether a row is one the filter keeps. */
export const shows = (row: Sent, showing: Showing): boolean =>
	showing === 'all' ||
	(showing === 'failed' && row.status === 'failed') ||
	(showing === 'working' && working(row));

/** The rows by the day they were sent, newest first as the Core sends them. */
export const byDay = (rows: Sent[], now: Date = new Date()): Day<Sent>[] =>
	byDayOf(rows, (row) => row.created_at, now);

/** What kind of thing was sent, the small line over what it was. */
export function kindOf(row: Sent): string {
	if (row.read_again) return m.sent_kind_read_again();
	switch (row.kind) {
		case 'link':
			return m.sent_kind_link();
		case 'pasted_text':
			return m.sent_kind_text();
		case 'pdf':
			return m.sent_kind_pdf();
		case 'picture':
			return row.pages > 1 ? m.sent_kind_photos() : m.sent_kind_photo();
	}
}

/** A link as a person reads it: no scheme, no `www.`. */
export const shortLink = (link: string): string =>
	link.replace(/^https?:\/\//i, '').replace(/^www\./i, '');

/**
 * What was sent, as far as Kamosu holds it: the recipe read again, the
 * link, the first words, or how many photos. Once Kamosu has deleted a file
 * or text, what kind it was and why it is not shown.
 */
export function whatWasSent(row: Sent): { said: string; gone: boolean; quoted: boolean } {
	if (row.read_again && row.recipe) return { said: row.recipe.title, gone: false, quoted: false };
	if (row.link) return { said: shortLink(row.link), gone: false, quoted: false };
	if (!row.kept) {
		const said =
			row.kind === 'picture'
				? row.pages > 1
					? m.sent_gone_photos()
					: m.sent_gone_photo()
				: row.kind === 'pdf'
					? m.sent_gone_pdf()
					: m.sent_gone_text();
		return { said, gone: true, quoted: false };
	}
	if (row.first_words) return { said: row.first_words, gone: false, quoted: true };
	if (row.kind === 'picture')
		return {
			said: row.pages > 1 ? m.sent_photos({ count: row.pages }) : m.sent_one_photo(),
			gone: false,
			quoted: false,
		};
	if (row.kind === 'pdf') return { said: m.sent_a_pdf(), gone: false, quoted: false };
	return { said: m.sent_kind_text(), gone: false, quoted: false };
}
