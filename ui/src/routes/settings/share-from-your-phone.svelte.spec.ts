/**
 * Save from your phone (#452, option B; named so by #469): the Settings page that sets a
 * phone up to add recipes from any app's Share menu, against a stand-in
 * Kamosu.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen, within } from '@testing-library/svelte';
import Share from './ShareFromYourPhone.svelte';
import { renderScreen } from '../../testing/render';
import type { Answers } from '$lib/api/stand-in';

/**
 * The page's own component, not the Settings screen around it: the screen is
 * the Settings spec's to load, with the mocks it loads it under.
 */
const signedIn: Answers = {};

const withRunner = { runner_status: { has_runner: true, last_asked_at: null } };
const withoutRunner = { runner_status: { has_runner: false, last_asked_at: null } };

const minted = {
	id: 'ak_phone',
	name: 'iPhone',
	read_only: false,
	operator: false,
	add_only: true,
	everything_to_agent: true,
	secret: 'f00dcafe',
};

function onAn(userAgent: string) {
	vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(userAgent);
}

const steps = () =>
	screen
		.getAllByRole('heading', { level: 2 })
		.map((heading) => heading.textContent?.replace(/\s+/g, ' ').trim());

describe('share from your phone', () => {
	afterEach(() => {
		cleanup();
		vi.restoreAllMocks();
	});

	it('lays an iPhone out as three steps, and shows the code once in the second', async () => {
		onAn('Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)');
		const { kamosu } = renderScreen(Share, {
			...signedIn,
			...withRunner,
			mint_access_key: minted,
		});

		await screen.findByRole('radio', { name: /Only what Kamosu can't read alone/ });
		expect(steps()).toEqual([
			'1 Choose what the agent reads',
			'2 Make your setup code',
			'3 Install the Shortcut',
		]);
		await fireEvent.click(screen.getByRole('radio', { name: /Everything/ }));
		await fireEvent.click(screen.getByRole('button', { name: 'Make the code' }));

		const code = await screen.findByText(`kamosu1 ${location.origin} f00dcafe`);
		expect(code).toBeInTheDocument();
		expect(steps()[1]).toBe('2 Copy your setup code');
		expect(screen.getByText("Copy this code now. It won't be shown again.")).toBeInTheDocument();
		expect(kamosu.calls.find((call) => call.operation === 'mint_access_key')?.input).toEqual({
			name: 'iPhone',
			add_only: true,
			everything_to_agent: true,
		});

		// Done puts the code away for good.
		await fireEvent.click(screen.getByRole('button', { name: 'Done' }));
		expect(screen.queryByText(`kamosu1 ${location.origin} f00dcafe`)).not.toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Make the code' })).toBeInTheDocument();
	});

	it('asks no agent question without a Runner, and says what works instead', async () => {
		onAn('Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)');
		const { kamosu } = renderScreen(Share, {
			...signedIn,
			...withoutRunner,
			mint_access_key: { ...minted, everything_to_agent: false },
		});

		await screen.findByText('This Kamosu has no AI agent, so:');
		expect(steps()[0]).toBe('1 What works here');
		expect(screen.queryByRole('radio')).not.toBeInTheDocument();
		expect(screen.getByText("photos, text and PDFs can't be read.")).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Make the code' }));
		await screen.findByText(`kamosu1 ${location.origin} f00dcafe`);
		expect(kamosu.calls.find((call) => call.operation === 'mint_access_key')?.input).toMatchObject({
			add_only: true,
			everything_to_agent: false,
		});
	});

	it('sets nothing up on Android, and keeps the agent choice with its Session', async () => {
		onAn('Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/141.0');
		const { kamosu } = renderScreen(Share, {
			...signedIn,
			...withRunner,
			get_agent_choice: { everything_to_agent: false },
			set_agent_choice: { everything_to_agent: true },
		});

		const everything = await screen.findByRole('radio', { name: /Everything/ });
		expect(
			screen.getByText(/Once Kamosu is on your Home screen, it's in every app's Share menu/),
		).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Make the code' })).not.toBeInTheDocument();
		await fireEvent.click(everything);
		await vi.waitFor(() =>
			expect(kamosu.calls.find((call) => call.operation === 'set_agent_choice')?.input).toEqual({
				everything_to_agent: true,
			}),
		);
		expect(screen.getByText('Kept for this phone.')).toBeInTheDocument();
	});

	it('names each step for a screen reader, and the question once in step 1', async () => {
		onAn('Mozilla/5.0 (iPhone)');
		renderScreen(Share, { ...signedIn, ...withRunner });
		const group = await screen.findByRole('group', { name: "What goes to Kamosu's AI agent" });
		expect(within(group).getAllByRole('radio')).toHaveLength(2);
	});
});
