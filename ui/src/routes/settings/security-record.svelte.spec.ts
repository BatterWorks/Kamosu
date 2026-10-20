/**
 * The Security Record on Settings (#371, choice A), against a stand-in
 * Kamosu: a page of its own, "Yours" for everybody and "This instance" for an
 * Operator.
 */

import { describe, expect, it } from 'vitest';
import { screen, within } from '@testing-library/svelte';
import Settings from './+page.svelte';
import Security from './security/+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import type { ListSecurityRecordOutput } from '$lib/api/catalogue';

type Event = ListSecurityRecordOutput['events'][number];

const today = (hour: number, minute = 0) => {
	const at = new Date();
	at.setHours(hour, minute, 0, 0);
	return at.toISOString();
};
const daysAgo = (days: number) => {
	const at = new Date();
	at.setDate(at.getDate() - days);
	at.setHours(12, 0, 0, 0);
	return at.toISOString();
};

let id = 0;
function event(more: Partial<Event> & Pick<Event, 'event' | 'at'>): Event {
	return {
		id: ++id,
		first_at: more.at,
		count: 1,
		by: 'Stéphane',
		about: null,
		credential: null,
		detail: {},
		...more,
	};
}

const MINE: Event[] = [
	event({
		event: 'signed_in',
		at: today(9, 15),
		credential: { kind: 'session', name: 'Safari · iPhone' },
		detail: { how: 'passkey' },
	}),
	event({ event: 'sign_in_failed', at: today(8, 54), first_at: today(8, 52), count: 3 }),
	event({
		event: 'access_key_minted',
		at: daysAgo(1),
		credential: { kind: 'access_key', name: 'Claude' },
		detail: { read_only: true, operator: false },
	}),
	event({
		event: 'password_changed',
		at: daysAgo(7),
		credential: { kind: 'session', name: 'Firefox on Linux' },
		detail: { ended_sessions: 2, ended_access_keys: 0 },
	}),
];

const INSTANCE: Event[] = [
	event({
		event: 'house_notes_changed',
		at: today(10, 2),
		credential: { kind: 'operator_key', name: 'Hermes' },
		detail: { taken_away: false },
	}),
	event({
		event: 'sign_in_crowd',
		at: today(3, 20),
		first_at: today(3, 12),
		count: 41,
		by: null,
	}),
	event({
		event: 'operator_raised',
		at: daysAgo(3),
		about: 'Camille',
		credential: { kind: 'session', name: 'Firefox on Linux' },
	}),
];

const record = (events: Event[]): ListSecurityRecordOutput => ({ events, kept_for_days: 90 });

const signedIn: Answers = {
	list_sessions: { sessions: [] },
	list_accounts: { refuse: 'unauthorized' },
	meaning_search_status: { refuse: 'unauthorized' },
};

function security(more: Answers) {
	return renderScreen(Security, { ...signedIn, ...more });
}

/** A row's two lines, read as one. */
const rowOf = (text: string) => screen.getByText(text).closest('li')!;

describe('the Security Record (#371)', () => {
	it('is a fifth row on the Settings menu', async () => {
		renderScreen(Settings, { ...signedIn, get_person: { person_id: 'p_1', name: 'Stéphane' } });
		const row = await screen.findByRole('link', { name: /Security record/ });
		expect(row).toHaveAttribute('href', '/settings/security');
		expect(row).toHaveTextContent('Sign-ins, wrong passwords, keys and changes, the last 90 days');
	});

	it('shows a cook their own record by day, and no instance record', async () => {
		security({
			list_security_record: record(MINE),
			list_instance_security_record: { refuse: 'unauthorized' },
		});

		expect(await screen.findByRole('heading', { name: 'Yours' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Today' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Yesterday' })).toBeInTheDocument();
		expect(rowOf('Signed in on Safari · iPhone')).toHaveTextContent(/With a passkey · 09:15$/);
		const wrong = rowOf('3 wrong passwords at your name');
		expect(wrong).toHaveTextContent('From 08:52 to 08:54 · Nobody signed in with them');
		expect(wrong).toHaveClass('border-l-support');
		expect(rowOf('Signed in on Safari · iPhone')).not.toHaveClass('border-l-support');
		expect(rowOf('Access Key made: Claude')).toHaveTextContent(/Reads only · \d\d:\d\d$/);
		expect(rowOf('Password changed')).toHaveTextContent(
			/On Firefox on Linux · 2 other devices signed out · \d\d:\d\d$/,
		);
		expect(screen.queryByRole('heading', { name: 'This instance' })).not.toBeInTheDocument();
	});

	it('shows an Operator the instance record too, saying who did what and through what', async () => {
		security({
			list_security_record: record(MINE),
			list_instance_security_record: record(INSTANCE),
		});

		const heading = await screen.findByRole('heading', { name: 'This instance' });
		const section = heading.closest('section')!;
		expect(within(section).getByText('House Notes changed').closest('li')).toHaveTextContent(
			'By Stéphane, through the key “Hermes” · 10:02',
		);
		const crowd = within(section).getByText('Sign-ins refused: too many at once').closest('li')!;
		expect(crowd).toHaveTextContent('41 tries · From 03:12 to 03:20 · Most likely names made up');
		expect(crowd).toHaveClass('border-l-support');
		expect(within(section).getByText('Camille made an Operator').closest('li')).toHaveTextContent(
			/By Stéphane, on Firefox on Linux · 12:00$/,
		);
	});

	it('says when there is nothing yet', async () => {
		security({
			list_security_record: record([]),
			list_instance_security_record: { refuse: 'unauthorized' },
		});
		expect(await screen.findByText('Nothing in the last 90 days.')).toBeInTheDocument();
	});
});

describe('how a row is said (#371)', () => {
	it('says the day a run began on, when it is not the day it is filed under', async () => {
		const lastNight = new Date();
		lastNight.setDate(lastNight.getDate() - 1);
		lastNight.setHours(23, 50, 0, 0);
		security({
			list_security_record: record([
				event({
					event: 'sign_in_failed',
					at: today(0, 10),
					first_at: lastNight.toISOString(),
					count: 4,
				}),
			]),
			list_instance_security_record: { refuse: 'unauthorized' },
		});
		const row = (await screen.findByText('4 wrong passwords at your name')).closest('li')!;
		expect(row).toHaveTextContent(/From \w{3},? \d+ \w+ 23:50 to 00:10/);
	});

	it('says what a recovery link ended, Runner Keys among it (#378)', async () => {
		security({
			list_security_record: record([
				event({
					event: 'signed_in',
					at: today(11, 0),
					credential: { kind: 'session', name: 'replacement browser' },
					detail: {
						how: 'recovery_link',
						ended_sessions: 1,
						ended_access_keys: 2,
						ended_runner_keys: 1,
					},
				}),
			]),
			list_instance_security_record: { refuse: 'unauthorized' },
		});
		const row = (await screen.findByText('Signed in on replacement browser')).closest('li')!;
		expect(row).toHaveTextContent(
			/1 other device signed out · 2 Access Keys ended · 1 Runner Key ended · 11:00$/,
		);
	});

	it('says a new Operator arrived through an Operator Invite', async () => {
		security({
			list_security_record: record([]),
			list_instance_security_record: record([
				event({
					event: 'operator_raised',
					at: today(11, 0),
					about: 'Marc',
					by: null,
					detail: { how: 'invite' },
				}),
			]),
		});
		const row = (await screen.findByText('Marc made an Operator')).closest('li')!;
		expect(row).toHaveTextContent('With an Operator Invite · 11:00');
	});
});

it('says a run within one minute as that minute (#371)', async () => {
	security({
		list_security_record: record([
			event({ event: 'sign_in_failed', at: today(9, 57), first_at: today(9, 57), count: 2 }),
		]),
		list_instance_security_record: { refuse: 'unauthorized' },
	});
	const row = (await screen.findByText('2 wrong passwords at your name')).closest('li')!;
	expect(row).toHaveTextContent(
		/^2 wrong passwords at your name\s*09:57 · Nobody signed in with them$/,
	);
});
