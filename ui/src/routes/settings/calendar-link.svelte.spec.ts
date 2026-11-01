/**
 * Your plan in your calendar (#491): the Settings page that makes, shows once
 * and stops a Calendar Link, against a stand-in Kamosu.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, screen } from '@testing-library/svelte';
import CalendarLink from './CalendarLink.svelte';
import { renderScreen } from '../../testing/render';

const SECRET = '7f3a9c2e';
const made = {
	secret: SECRET,
	path: `/calendar/${SECRET}.ics`,
	url: `https://recipes.kamosu.example/calendar/${SECRET}.ics`,
	made_at: '2026-11-01T13:00:00.000Z',
};
const live = (last_read_at: string | null) => ({
	calendar_link: { made_at: '2026-11-01T13:00:00.000Z', last_read_at },
});

function onAn(userAgent: string) {
	vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(userAgent);
}

const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)';
const ANDROID = 'Mozilla/5.0 (Linux; Android 15; Pixel 9)';

describe('your plan in your calendar', () => {
	afterEach(() => {
		cleanup();
		vi.restoreAllMocks();
	});

	it('makes the link and shows it this once, with a way into an iPhone calendar', async () => {
		onAn(IPHONE);
		const { kamosu } = renderScreen(CalendarLink, {
			get_calendar_link: { calendar_link: null },
			make_calendar_link: made,
		});

		// The plain words stand there before anything is made.
		expect(
			await screen.findByText(/Anyone who has the link can see your plan/),
		).toBeInTheDocument();
		expect(screen.getByText(/up to a day in Google Calendar/)).toBeInTheDocument();
		expect(screen.getByText(/reached from the internet/)).toBeInTheDocument();

		const make = await screen.findByRole('button', { name: 'Make the link' });
		kamosu.answer('get_calendar_link', live(null));
		await fireEvent.click(make);

		expect(await screen.findByText(made.url)).toBeInTheDocument();
		expect(screen.getByText('Your calendar link. It is shown this once.')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Add to iPhone calendar' })).toHaveAttribute(
			'href',
			`webcal://recipes.kamosu.example/calendar/${SECRET}.ics`,
		);
		expect(screen.getByRole('button', { name: 'Copy the link' })).toBeInTheDocument();
		expect(screen.getByText(/From URL, and paste the link/)).toBeInTheDocument();

		// Done puts it away for good, and the page says a link is on.
		await fireEvent.click(screen.getByRole('button', { name: 'Done' }));
		expect(screen.queryByText(made.url)).not.toBeInTheDocument();
		expect(screen.getByText('Your calendar link is on')).toBeInTheDocument();
		expect(screen.getByText(/no calendar has read it yet/)).toBeInTheDocument();
	});

	it('builds the link at the address it was opened at where there is no public address', async () => {
		onAn(ANDROID);
		renderScreen(CalendarLink, {
			get_calendar_link: { calendar_link: null },
			make_calendar_link: { ...made, url: null },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Make the link' }));
		expect(
			await screen.findByText(`${location.origin}/calendar/${SECRET}.ics`),
		).toBeInTheDocument();
		// Nothing on Android answers a webcal link, so it is not offered.
		expect(screen.queryByRole('link', { name: 'Add to iPhone calendar' })).not.toBeInTheDocument();
	});

	it('says when a calendar last read it, and stops it', async () => {
		onAn(IPHONE);
		const { kamosu } = renderScreen(CalendarLink, {
			get_calendar_link: live('2026-11-01T14:02:00.000Z'),
			end_calendar_link: { ended: true },
		});
		expect(await screen.findByText(/a calendar last read it/)).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Make a new link' })).toBeInTheDocument();
		expect(screen.getByText(/A new link stops the old one/)).toBeInTheDocument();

		kamosu.answer('get_calendar_link', { calendar_link: null });
		await fireEvent.click(screen.getByRole('button', { name: 'Stop the link' }));
		expect(await screen.findByRole('button', { name: 'Make the link' })).toBeInTheDocument();
		expect(kamosu.calls.map((call) => call.operation)).toContain('end_calendar_link');
		expect(screen.queryByText('Your calendar link is on')).not.toBeInTheDocument();
	});

	it('says why it cannot make one where Kamosu refuses, as a Demo does', async () => {
		onAn(IPHONE);
		renderScreen(CalendarLink, {
			get_calendar_link: { calendar_link: null },
			make_calendar_link: { refuse: 'bad_request', message: 'this is a Demo' },
		});
		await fireEvent.click(await screen.findByRole('button', { name: 'Make the link' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('this is a Demo');
	});
});
