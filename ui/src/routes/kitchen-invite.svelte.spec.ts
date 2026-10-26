/**
 * The page a Kitchen Invite link opens (#469, choice B): the Kitchen as its
 * members see it, said before anything happens, and nothing done until
 * *Join* is pressed.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { AuthClient } from '$lib/auth';
import type { ReadKitchenInviteOutput } from '$lib/api/catalogue';
import { went } from '../testing/navigation';
import LinkTestHarness from './LinkTestHarness.svelte';
import { signingIn } from '$lib/shell/signing-in.svelte';
import KitchenInviteRoute from './kitchen-invite/[secret]/+page.svelte';

/** Marie's Kitchen, which she shares with Nadia. */
const SUNDAY = {
	id: 'k_sunday',
	name: 'Sunday kitchen',
	nickname: null,
	members: [
		{ person_id: 'p_2', name: 'Marie' },
		{ person_id: 'p_3', name: 'Nadia' },
	],
	cookbooks: [
		{ id: 'c_2', name: null, authors: [{ person_id: 'p_2', name: 'Marie' }] },
		{ id: 'c_3', name: null, authors: [{ person_id: 'p_3', name: 'Nadia' }] },
	],
};

const invite = (over: Partial<ReadKitchenInviteOutput> = {}): ReadKitchenInviteOutput => ({
	kitchen: SUNDAY,
	invited_by: { person_id: 'p_2', name: 'Marie' },
	already_member: false,
	...over,
});

function open(answers: Answers) {
	const kamosu = standIn({
		instance_status: {
			version: '0.1.0',
			setup_complete: true,
			password_minimum: 15,
			passkey_origin: null,
			demo: false,
		},
		...answers,
	});
	const authenticate = vi.fn<AuthClient['authenticate']>(async () => {});
	render(LinkTestHarness, {
		props: {
			route: KitchenInviteRoute,
			secret: 'the-kitchen-secret',
			client: kamosu.client,
			auth: {
				authenticate,
				signInWithPasskey: async () => 'declined' as const,
				readyPasskey: () => {},
				arrive: async () => {},
			},
		},
	});
	return { kamosu, authenticate };
}

describe('a Kitchen Invite link', () => {
	it('shows the Kitchen, who is in it and whose recipes it holds, doing nothing yet', async () => {
		const { kamosu } = open({
			read_kitchen_invite: invite(),
			accept_kitchen_invite: SUNDAY,
		});

		expect(await screen.findByRole('heading', { name: 'Sunday kitchen' })).toBeInTheDocument();
		expect(screen.getByText('Marie invites you')).toBeInTheDocument();
		expect(screen.getByText('Nadia')).toBeInTheDocument();
		expect(
			screen.getByText('Recipes here: Marie’s Cookbook and Nadia’s Cookbook'),
		).toBeInTheDocument();
		expect(
			screen.getByText('Joining shows them your recipes, and theirs to you.'),
		).toBeInTheDocument();
		// Opening the link spends nothing.
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('accept_kitchen_invite');

		await fireEvent.click(screen.getByRole('button', { name: 'Join' }));
		await waitFor(() =>
			expect(kamosu.calls).toContainEqual({
				operation: 'accept_kitchen_invite',
				input: { secret: 'the-kitchen-secret' },
			}),
		);
		await waitFor(() => expect(went).toHaveBeenCalledWith('/settings/kitchens'));
	});

	it('leaves the link unspent on Not now', async () => {
		const { kamosu } = open({ read_kitchen_invite: invite() });

		const notNow = await screen.findByRole('link', { name: 'Not now' });
		expect(notNow).toHaveAttribute('href', '/');
		expect(kamosu.calls.map((call) => call.operation)).not.toContain('accept_kitchen_invite');
	});

	it('says a used or cancelled link is gone, and asks for a new one', async () => {
		// Refused as a stranger is refused, so the page asks whether you are
		// signed in before it offers the sign-in form.
		open({ read_kitchen_invite: { refuse: 'unauthorized' }, list_kitchens: { kitchens: [] } });
		expect(
			await screen.findByText('This invite has been used or cancelled. Ask for a new one.'),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Join' })).not.toBeInTheDocument();
	});

	it('says so when the reader already cooks in that Kitchen', async () => {
		open({ read_kitchen_invite: invite({ already_member: true }) });
		expect(await screen.findByText('You already cook in this Kitchen.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Join' })).not.toBeInTheDocument();
	});

	it('signs a stranger in first, saying where an account comes from, then shows the Kitchen', async () => {
		let signedIn = false;
		const { authenticate } = open({
			read_kitchen_invite: () => (signedIn ? invite() : { refuse: 'unauthorized' }),
			list_kitchens: () => (signedIn ? { kitchens: [] } : { refuse: 'unauthorized' }),
		});
		authenticate.mockImplementation(async () => {
			signedIn = true;
		});

		expect(await screen.findByRole('heading', { name: 'Sign in to join' })).toBeInTheDocument();
		// A Kitchen link makes no account: an Operator's Invite does (#469).
		expect(
			screen.getByText('No account here? Ask whoever runs this Kamosu for one.'),
		).toBeInTheDocument();
		await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Camille' } });
		await fireEvent.input(screen.getByLabelText('Password'), { target: { value: 'a password' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Log in' }));

		expect(await screen.findByRole('heading', { name: 'Sunday kitchen' })).toBeInTheDocument();
	});

	it('tells the shell a Person is signed in once the link is read (#214)', async () => {
		signingIn.signedIn = undefined;
		open({ read_kitchen_invite: invite() });
		await screen.findByRole('heading', { name: 'Sunday kitchen' });
		expect(signingIn.signedIn).toBe(true);
	});
});
