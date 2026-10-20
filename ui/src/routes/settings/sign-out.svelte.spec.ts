/**
 * Signing out, from the foot of the Settings menu (#380, option A of 20
 * October 2026): a person finds it without being told, it asks first, and it
 * never loses what this device wrote offline without saying so.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, screen, within } from '@testing-library/svelte';
import Settings from './+page.svelte';
import Account from './account/+page.svelte';
import { keepingForTests, renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';
import { went } from '../../testing/navigation';
import { signingIn } from '$lib/shell/signing-in.svelte';

afterEach(() => {
	Reflect.deleteProperty(navigator, 'onLine');
	signingIn.signedIn = undefined;
});

const session = (id: string, current = false) => ({
	id,
	name: current ? 'Safari · iPhone' : 'Firefox · Linux',
	created_at: '2026-09-12T09:00:00Z',
	last_used_at: '2026-09-20T18:28:00Z',
	revoked: false,
	current,
});

const signedIn = (over: Answers = {}): Answers => ({
	get_person: { person_id: 'p_1', name: 'Stéphane' },
	list_sessions: { sessions: [session('s_laptop'), session('s_phone', true)] },
	revoke_session: { revoked: true, current: true },
	...over,
});

/** What Settings › Account reads, for the Sessions list's way in. */
const accountAnswers = (): Answers => ({
	...signedIn(),
	instance_status: {
		version: '0.1.0',
		setup_complete: true,
		password_minimum: 15,
		passkey_origin: null,
	},
	list_passkeys: {
		public_address: 'https://kamosu.example',
		available: true,
		unavailable_because: null,
		origin: 'https://kamosu.example',
		passkeys: [],
	},
	list_access_keys: { access_keys: [] },
	get_reading_preferences: { reading_language: 'en', reading_measures: 'us' },
	list_accounts: { refuse: 'unauthorized' },
});

/** An outbox still holding writes the server has not taken, as many as each try says. */
function holding(...tries: number[]) {
	const keeping = keepingForTests();
	const sendWaiting = vi.fn(async () => tries.shift() ?? 0);
	return { keeping: { ...keeping, sendWaiting }, sendWaiting };
}

const sheet = () => screen.findByRole('dialog');

describe('signing out (#380)', () => {
	it('stands at the foot of Settings, saying who is signed in', async () => {
		renderScreen(Settings, signedIn());

		expect(await screen.findByText('Signed in as Stéphane')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Sign out' })).toBeEnabled();
	});

	it('asks first, says the recipes kept here go, and does nothing on Cancel', async () => {
		const { kamosu } = renderScreen(Settings, signedIn());

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		const asked = await sheet();
		expect(within(asked).getByRole('heading', { name: 'Sign out on this device?' })).toBeVisible();
		expect(
			within(asked).getByText(
				'The recipes kept here for offline use are removed. They come back when you sign in again.',
			),
		).toBeInTheDocument();

		await fireEvent.click(within(asked).getByRole('button', { name: 'Cancel' }));
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('revoke_session');
		expect(went).not.toHaveBeenCalled();
	});

	it('ends the Session in your hand and goes to the sign-in screen', async () => {
		const { kamosu } = renderScreen(Settings, signedIn());

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		await fireEvent.click(within(await sheet()).getByRole('button', { name: 'Sign out' }));

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/'));
		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_phone' },
		});
		// So the sidebar is gone before Home is drawn, not a moment after (#194).
		expect(signingIn.signedIn).toBe(false);
	});

	it('sends what this device wrote offline before it asks', async () => {
		const { keeping, sendWaiting } = holding(0);
		renderScreen(Settings, signedIn(), keeping);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		expect(await sheet()).toHaveAccessibleName('Sign out on this device?');
		expect(sendWaiting).toHaveBeenCalledOnce();
	});

	it('says how many changes have not reached Kamosu, and signs out anyway only when told', async () => {
		const { keeping } = holding(3);
		const { kamosu } = renderScreen(Settings, signedIn(), keeping);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		const warned = await sheet();
		expect(
			within(warned).getByRole('heading', { name: '3 changes haven’t reached Kamosu yet' }),
		).toBeVisible();
		expect(within(warned).getByText(/Signing out now loses them\./)).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('revoke_session');

		await fireEvent.click(within(warned).getByRole('button', { name: 'Sign out anyway' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/'));
		expect(kamosu.calls).toContainEqual({
			operation: 'revoke_session',
			input: { session_id: 's_phone' },
		});
	});

	it('says one change in the singular', async () => {
		const { keeping } = holding(1);
		renderScreen(Settings, signedIn(), keeping);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		const warned = await sheet();
		expect(
			within(warned).getByRole('heading', { name: '1 change hasn’t reached Kamosu yet' }),
		).toBeVisible();
		expect(within(warned).getByText(/Signing out now loses it\./)).toBeInTheDocument();
	});

	it('tries again when asked to wait, and asks the ordinary question once all is sent', async () => {
		const { keeping, sendWaiting } = holding(2, 2, 0);
		renderScreen(Settings, signedIn(), keeping);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		await fireEvent.click(
			within(await sheet()).getByRole('button', { name: 'Wait, and try again' }),
		);
		await vi.waitFor(() => expect(sendWaiting).toHaveBeenCalledTimes(2));
		expect(await sheet()).toHaveAccessibleName('2 changes haven’t reached Kamosu yet');

		await fireEvent.click(
			within(await sheet()).getByRole('button', { name: 'Wait, and try again' }),
		);
		await vi.waitFor(async () =>
			expect(await sheet()).toHaveAccessibleName('Sign out on this device?'),
		);
	});

	it('offline, says it waits for the server and cannot be pressed', async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		renderScreen(Settings, signedIn());

		const button = await screen.findByRole('button', { name: 'Signing out waits for the server' });
		expect(button).toBeDisabled();
	});

	it('says in the sheet when the server refuses, and stays signed in', async () => {
		renderScreen(
			Settings,
			signedIn({ revoke_session: { refuse: 'busy', message: 'Kamosu is busy.' } }),
		);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		await fireEvent.click(within(await sheet()).getByRole('button', { name: 'Sign out' }));

		expect(await within(await sheet()).findByRole('alert')).toHaveTextContent('Kamosu is busy.');
		expect(within(await sheet()).getByRole('button', { name: 'Sign out' })).toBeEnabled();
		expect(went).not.toHaveBeenCalled();
	});

	it('goes to the sign-in screen when the Session had already ended elsewhere', async () => {
		renderScreen(Settings, signedIn({ list_sessions: { refuse: 'unauthorized' } }));

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		await fireEvent.click(within(await sheet()).getByRole('button', { name: 'Sign out' }));

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/'));
		expect(signingIn.signedIn).toBe(false);
	});

	it('opens at once while it sends, and can be put away before the server answers', async () => {
		const keeping = { ...keepingForTests(), sendWaiting: () => new Promise<number>(() => {}) };
		renderScreen(Settings, signedIn(), keeping);

		await fireEvent.click(await screen.findByRole('button', { name: 'Sign out' }));
		const sending = await sheet();
		expect(within(sending).getByRole('status')).toHaveTextContent(
			'Sending what this device wrote offline first…',
		);
		expect(within(sending).getByRole('button', { name: 'Sign out' })).toBeDisabled();

		await fireEvent.click(within(sending).getByRole('button', { name: 'Cancel' }));
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('asks the same question from the Sessions list', async () => {
		const { kamosu } = renderScreen(Account, accountAnswers());

		const heading = await screen.findByRole('heading', { name: 'Sessions' });
		const list = heading.nextElementSibling as HTMLElement;
		const mine = within(list).getAllByRole('listitem')[0];
		await fireEvent.click(within(mine).getByRole('button', { name: 'Sign out' }));

		expect(await sheet()).toHaveAccessibleName('Sign out on this device?');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('revoke_session');
	});
	it("greys the Sessions list's Sign out offline too", async () => {
		Object.defineProperty(navigator, 'onLine', { value: false, configurable: true });
		renderScreen(Account, accountAnswers());

		const heading = await screen.findByRole('heading', { name: 'Sessions' });
		const list = heading.nextElementSibling as HTMLElement;
		const mine = within(list).getAllByRole('listitem')[0];
		expect(
			within(mine).getByRole('button', { name: 'Signing out waits for the server' }),
		).toBeDisabled();
	});
});
