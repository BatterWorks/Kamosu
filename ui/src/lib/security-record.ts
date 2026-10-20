/**
 * The Security Record in words (#371): each row the Core answers, said as a
 * line and a second line under it, and the rows grouped by the day they
 * happened on. Both records read the same way, a Person's own and the
 * instance's every Operator reads.
 */

import { m } from '$lib/paraglide/messages';
import { aClock, aWeekday } from '$lib/dates';
import type { ListSecurityRecordOutput } from '$lib/api/catalogue';

export type SecurityEvent = ListSecurityRecordOutput['events'][number];

/** How long the Core keeps an event, for the Settings menu's line (`SECURITY_RECORD_DAYS`). */
export const KEPT_FOR_DAYS = 90;

export interface Said {
	/** What happened. */
	line: string;
	/** How, by whom and when. */
	under: string;
	/** Something to look at twice: wrong passwords, or a crowd at the sign-in. */
	warn: boolean;
}

/** One row, said. */
export function say(event: SecurityEvent): Said {
	const name = event.credential?.name ?? '';
	const about = event.about ?? m.security_someone();
	const detail = event.detail;
	const when = aClock(event.at);
	// A run that began on an earlier day says which, since it is filed under
	// the day it last happened on.
	const from =
		new Date(event.first_at).toDateString() === new Date(event.at).toDateString()
			? aClock(event.first_at)
			: `${aWeekday(event.first_at)} ${aClock(event.first_at)}`;
	// A run whose tries all fell in one minute is said as that minute.
	const span = from === when ? when : m.security_from_to({ from, to: when });
	const under = (...parts: (string | undefined)[]) =>
		[...parts, when].filter((part) => part).join(' · ');
	const operatorKey = event.credential?.kind === 'operator_key';

	switch (event.event) {
		case 'signed_in':
			return {
				line: m.security_signed_in({ name }),
				under: under(howSignedIn(detail.how), ...whatEnded(detail)),
				warn: false,
			};
		case 'sign_in_failed':
			return {
				line:
					event.count === 1
						? m.security_wrong_one()
						: m.security_wrong({ count: String(event.count) }),
				under: [span, event.count > 1 ? m.security_nobody_in() : undefined]
					.filter((part) => part)
					.join(' · '),
				warn: true,
			};
		case 'passkey_added':
			return { line: m.security_passkey_added({ name }), under: under(), warn: false };
		case 'passkey_removed':
			return { line: m.security_passkey_removed({ name }), under: under(), warn: false };
		case 'password_changed':
			return {
				line: m.security_password_changed(),
				under: under(name ? m.security_on({ name }) : undefined, ...whatEnded(detail)),
				warn: false,
			};
		case 'access_key_minted':
			return {
				line: operatorKey ? m.security_operator_key_made({ name }) : m.security_key_made({ name }),
				under: under(detail.read_only ? m.security_reads_only() : undefined),
				warn: false,
			};
		case 'access_key_ended':
			return {
				line: operatorKey
					? m.security_operator_key_ended({ name })
					: m.security_key_ended({ name }),
				under: under(),
				warn: false,
			};
		case 'session_ended':
			return { line: m.security_session_ended({ name }), under: under(), warn: false };
		case 'house_notes_changed':
			return {
				line: detail.taken_away
					? m.security_house_notes_taken_away()
					: m.security_house_notes_changed(),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'public_address_changed':
			return {
				line: m.security_address_changed({ address: detail.address ?? '' }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'operator_raised':
			return {
				line: m.security_raised({ name: about }),
				under: under(detail.how === 'invite' ? m.security_how_operator_invite() : saidBy(event)),
				warn: false,
			};
		case 'operator_stood_down':
			return {
				line: m.security_stood_down({ name: about }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'account_disabled':
			return {
				line: m.security_disabled({ name: about }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'account_deleted':
			return {
				line: m.security_deleted({ name: about }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'operator_key_minted':
			return {
				line: m.security_operator_key_made({ name: detail.key ?? name }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'sign_in_crowd':
			return {
				line: m.security_crowd(),
				under: [
					m.security_crowd_tries({ count: String(event.count) }),
					span,
					m.security_crowd_note(),
				].join(' · '),
				warn: true,
			};
		case 'runner_key_minted':
			return {
				line: m.security_runner_made({ name: detail.runner_key ?? '' }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'runner_key_revoked':
			return {
				line: m.security_runner_revoked({ name: detail.runner_key ?? '' }),
				under: under(saidBy(event)),
				warn: false,
			};
		case 'backup_fetched':
			return {
				line: m.security_backup_fetched({ name: detail.backup ?? '' }),
				under: under(saidBy(event)),
				warn: false,
			};
	}
}

function howSignedIn(how: SecurityEvent['detail']['how']): string | undefined {
	switch (how) {
		case 'password':
			return m.security_how_password();
		case 'passkey':
			return m.security_how_passkey();
		case 'invite':
			return m.security_how_invite();
		case 'recovery_link':
			return m.security_how_recovery();
		case 'setup':
			return m.security_how_setup();
		default:
			return undefined;
	}
}

/** What a password change or a recovery link ended with it. */
function whatEnded(detail: SecurityEvent['detail']): (string | undefined)[] {
	const sessions = detail.ended_sessions ?? 0;
	const keys = detail.ended_access_keys ?? 0;
	return [
		sessions === 0
			? undefined
			: sessions === 1
				? m.security_devices_out_one()
				: m.security_devices_out({ count: String(sessions) }),
		keys === 0
			? undefined
			: keys === 1
				? m.security_keys_ended_one()
				: m.security_keys_ended({ count: String(keys) }),
	];
}

/** Who changed the instance, and through which Session or key. */
function saidBy(event: SecurityEvent): string | undefined {
	if (!event.by) return undefined;
	const credential = event.credential;
	switch (credential?.kind) {
		case 'session':
			return m.security_by_session({ by: event.by, name: credential.name });
		case 'access_key':
		case 'operator_key':
			return m.security_by_key({ by: event.by, name: credential.name });
		case 'job_key':
			return m.security_by_agent({ by: event.by });
		default:
			return m.security_by({ by: event.by });
	}
}

export interface Day {
	heading: string;
	events: SecurityEvent[];
}

/** The rows by the day they last happened on, newest first as the Core sends them. */
export function byDay(events: SecurityEvent[], now: Date = new Date()): Day[] {
	const days: Day[] = [];
	const yesterday = new Date(now);
	yesterday.setDate(now.getDate() - 1);
	for (const event of events) {
		const at = new Date(event.at);
		const heading =
			at.toDateString() === now.toDateString()
				? m.security_today()
				: at.toDateString() === yesterday.toDateString()
					? m.security_yesterday()
					: aWeekday(at);
		const last = days.at(-1);
		if (last?.heading === heading) last.events.push(event);
		else days.push({ heading, events: [event] });
	}
	return days;
}
