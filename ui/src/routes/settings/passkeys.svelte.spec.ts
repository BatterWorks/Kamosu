/**
 * Passkeys and changing your password on Settings › Account (#370, choices A
 * and A), against a stand-in Kamosu. The device's own prompt is the one thing
 * stood in for here: `makePasskey` answers what each test says it did.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, screen, waitFor, within } from '@testing-library/svelte';
import Account from './account/+page.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import type { ListPasskeysOutput } from '$lib/api/catalogue';

/**
 * What the device does when asked for a passkey: makes one whose browser JSON
 * is `made`, or has its prompt closed.
 */
const device: { made?: object } = {};

/** A browser that can ask its device for a passkey, and the device it asks. */
beforeEach(() => {
	function PublicKeyCredential() {}
	vi.stubGlobal(
		'PublicKeyCredential',
		Object.assign(PublicKeyCredential, {
			parseRequestOptionsFromJSON: () => ({}),
			parseCreationOptionsFromJSON: () => ({}),
		}),
	);
	Object.defineProperty(navigator, 'credentials', {
		configurable: true,
		value: {
			create: async () => {
				if (!device.made) throw new DOMException('closed', 'NotAllowedError');
				const made = device.made;
				return Object.assign(Object.create(PublicKeyCredential.prototype), {
					toJSON: () => made,
				});
			},
		},
	});
});
afterEach(() => {
	vi.unstubAllGlobals();
	delete (navigator as { credentials?: unknown }).credentials;
	device.made = undefined;
});

const HERE = () => window.location.origin;

const passkeys = (
	listed: ListPasskeysOutput['passkeys'],
	more: Partial<ListPasskeysOutput> = {},
): ListPasskeysOutput => ({
	public_address: HERE(),
	available: true,
	unavailable_because: null,
	origin: HERE(),
	passkeys: listed,
	...more,
});

const IPHONE = {
	id: 'pk_1',
	name: 'Safari · iPhone',
	made_for: 'localhost',
	works_here: true,
	created_at: '2026-10-12T08:00:00.000Z',
	last_used_at: '2026-10-19T08:31:00.000Z',
};
const PIXEL = {
	id: 'pk_2',
	name: 'Chrome · Android',
	made_for: 'recipes.kamosu.example',
	works_here: false,
	created_at: '2026-09-02T08:00:00.000Z',
	last_used_at: null,
};

function account(more: Answers) {
	return renderScreen(Account, {
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
		},
		list_sessions: {
			sessions: [
				{
					id: 's_1',
					name: 'Firefox · Linux',
					created_at: '2026-10-12T08:00:00.000Z',
					last_used_at: null,
					revoked: false,
					current: true,
				},
				{
					id: 's_2',
					name: 'Chrome · Mac',
					created_at: '2026-10-11T08:00:00.000Z',
					last_used_at: null,
					revoked: false,
					current: false,
				},
			],
		},
		list_access_keys: { access_keys: [] },
		get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
		get_person: { person_id: 'p_1', name: 'Stéphane' },
		list_accounts: { refuse: 'unauthorized' },
		list_kitchens: { kitchens: [] },
		...more,
	});
}

describe('the passkey list (#370)', () => {
	it('lists each passkey above the devices, with when it was added and used', async () => {
		account({ list_passkeys: passkeys([IPHONE, PIXEL]) });

		const heading = await screen.findByRole('heading', { name: 'Passkeys' });
		expect(
			heading.compareDocumentPosition(screen.getByRole('heading', { name: 'Sessions' })) &
				Node.DOCUMENT_POSITION_FOLLOWING,
		).toBeTruthy();
		const iphone = (await screen.findByText('Safari · iPhone')).closest('li')!;
		expect(iphone).toHaveTextContent(/Added .* · Last used/);
		const pixel = screen.getByText('Chrome · Android').closest('li')!;
		expect(pixel).toHaveTextContent(
			'Made for recipes.kamosu.example, so it doesn’t work at this address.',
		);
		expect(within(iphone).queryByText(/Made for/)).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Add a passkey' })).toBeInTheDocument();
	});

	it('says there are none yet', async () => {
		account({ list_passkeys: passkeys([]) });
		expect(await screen.findByText('No passkeys yet.')).toBeInTheDocument();
	});

	it('asks before removing one, and removes it', async () => {
		const { kamosu } = account({
			list_passkeys: passkeys([IPHONE]),
			remove_passkey: { removed: true },
		});
		const row = (await screen.findByText('Safari · iPhone')).closest('li')!;
		await fireEvent.click(within(row).getByRole('button', { name: 'Remove' }));

		const sheet = screen.getByRole('dialog', { name: 'Remove “Safari · iPhone”?' });
		expect(sheet).toHaveTextContent('It won’t log you in here any more.');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Remove the passkey' }));
		await waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'remove_passkey')?.input).toEqual({
				passkey_id: 'pk_1',
			}),
		);
	});

	it('asks for the password, then the device, then names the passkey for it', async () => {
		device.made = { id: 'made' };
		const { kamosu } = account({
			list_passkeys: passkeys([]),
			start_adding_passkey: { ceremony_id: 'pk_ceremony', options: {} },
			finish_adding_passkey: IPHONE,
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Add a passkey' }));

		const sheet = screen.getByRole('dialog', { name: 'Add a passkey' });
		await fireEvent.input(within(sheet).getByLabelText('Password'), {
			target: { value: 'the right password' },
		});
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Continue' }));

		await waitFor(() =>
			expect(screen.queryByRole('dialog', { name: 'Add a passkey' })).not.toBeInTheDocument(),
		);
		expect(kamosu.calls.find((call) => call.operation === 'start_adding_passkey')?.input).toEqual({
			password: 'the right password',
		});
		const finished = kamosu.calls.find((call) => call.operation === 'finish_adding_passkey');
		expect(finished?.input).toEqual({
			ceremony_id: 'pk_ceremony',
			credential: '{"id":"made"}',
			// Named for this browser, as a new Session is. jsdom says neither
			// its browser nor its device, so the unknown device's name.
			name: 'A browser',
		});
	});

	it('says a wrong password, and keeps the sheet open', async () => {
		account({
			list_passkeys: passkeys([]),
			start_adding_passkey: { refuse: 'bad_request', reason: 'wrong_password' },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Add a passkey' }));
		const sheet = screen.getByRole('dialog', { name: 'Add a passkey' });
		await fireEvent.input(within(sheet).getByLabelText('Password'), {
			target: { value: 'not it' },
		});
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Continue' }));
		expect(await within(sheet).findByRole('alert')).toHaveTextContent(
			'That isn’t your current password.',
		);
	});

	it('says closing the device prompt by closing the sheet, and nothing else', async () => {
		const { kamosu } = account({
			list_passkeys: passkeys([]),
			start_adding_passkey: { ceremony_id: 'pk_ceremony', options: {} },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Add a passkey' }));
		const sheet = screen.getByRole('dialog', { name: 'Add a passkey' });
		await fireEvent.input(within(sheet).getByLabelText('Password'), {
			target: { value: 'the right password' },
		});
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Continue' }));
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('finish_adding_passkey');
		expect(screen.queryByRole('alert')).not.toBeInTheDocument();
	});

	it('says why an instance with no https:// name offers none, and offers no button', async () => {
		account({
			list_passkeys: passkeys([], {
				public_address: 'http://192.0.2.10:5266',
				available: false,
				unavailable_because: 'not_https',
				origin: null,
			}),
		});
		expect(
			await screen.findByText(/this instance’s public address is http:\/\/192\.0\.2\.10:5266/),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Add a passkey' })).not.toBeInTheDocument();
	});

	it('sends a page opened at another address to the public one', async () => {
		account({
			list_passkeys: passkeys([IPHONE], { origin: 'https://recipes.kamosu.example' }),
		});
		const link = await screen.findByRole('link', { name: 'https://recipes.kamosu.example' });
		expect(link).toHaveAttribute('href', 'https://recipes.kamosu.example/settings/account');
		expect(screen.queryByRole('button', { name: 'Add a passkey' })).not.toBeInTheDocument();
		// What is there can still be removed from here.
		expect(screen.getByRole('button', { name: 'Remove' })).toBeInTheDocument();
	});
});

describe('changing your password (#370)', () => {
	async function open(more: Answers) {
		const rendered = account({ list_passkeys: passkeys([]), ...more });
		await fireEvent.click(await screen.findByRole('button', { name: 'Change your password' }));
		return { ...rendered, sheet: screen.getByRole('dialog', { name: 'Change your password' }) };
	}

	async function fill(sheet: HTMLElement, current: string, next: string, again = next) {
		await fireEvent.input(within(sheet).getByLabelText('Current password'), {
			target: { value: current },
		});
		await fireEvent.input(within(sheet).getByLabelText('New password'), {
			target: { value: next },
		});
		await fireEvent.input(within(sheet).getByLabelText('New password again'), {
			target: { value: again },
		});
	}

	it('signs the other devices out to start with and keeps the keys, and says what it did', async () => {
		const { kamosu, sheet } = await open({
			change_password: { ended_sessions: 1, ended_access_keys: 0 },
		});
		const signOut = within(sheet).getByRole('checkbox', { name: /Sign out my other devices/ });
		const endKeys = within(sheet).getByRole('checkbox', { name: /End my Access Keys too/ });
		expect(signOut).toBeChecked();
		expect(endKeys).not.toBeChecked();
		expect(sheet).toHaveTextContent('1 other device. This one stays signed in.');
		expect(sheet).toHaveTextContent('Your passkeys keep working.');

		await fill(sheet, 'the old password', 'a brand new passphrase');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Change the password' }));

		expect(await screen.findByRole('status')).toHaveTextContent(
			'Password changed. 1 other device is signed out.',
		);
		expect(kamosu.calls.find((call) => call.operation === 'change_password')?.input).toEqual({
			current_password: 'the old password',
			new_password: 'a brand new passphrase',
			sign_out_other_devices: true,
			end_access_keys: false,
		});
	});

	it('refuses a short or mismatched new password without asking Kamosu', async () => {
		const { kamosu, sheet } = await open({});
		await waitFor(() =>
			expect(within(sheet).getByText(/At least 15 characters/)).toBeInTheDocument(),
		);
		await fill(sheet, 'the old password', 'too short');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Change the password' }));
		expect(within(sheet).getByRole('alert')).toHaveTextContent('at least 15 characters');

		await fill(sheet, 'the old password', 'a brand new passphrase', 'a brand new pasphrase');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Change the password' }));
		expect(within(sheet).getByRole('alert')).toHaveTextContent("aren't the same");
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('change_password');
	});

	it('says a wrong current password under it', async () => {
		const { sheet } = await open({
			change_password: { refuse: 'bad_request', reason: 'wrong_password' },
		});
		await fill(sheet, 'not my password', 'a brand new passphrase');
		await fireEvent.click(within(sheet).getByRole('button', { name: 'Change the password' }));
		expect(await within(sheet).findByRole('alert')).toHaveTextContent(
			'That isn’t your current password.',
		);
	});
});
