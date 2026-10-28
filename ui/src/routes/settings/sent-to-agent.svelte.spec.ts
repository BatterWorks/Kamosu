/**
 * Everything sent to Kamosu's AI agent (#463, option A), a page of Settings'
 * Add recipes, against a stand-in Kamosu.
 */

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, screen, within } from '@testing-library/svelte';
import Settings from './+page.svelte';
import SentPage from './agent/+page.svelte';
import { renderScreen } from '../../testing/render';
import { went } from '../../testing/navigation';
import type { Answers } from '$lib/api/stand-in';
import type { Sent } from '$lib/sent-to-agent';

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
function sent(more: Partial<Sent>): Sent {
	return {
		job_id: `j_${++id}`,
		created_at: today(9),
		ended_at: null,
		status: 'queued',
		kind: 'link',
		pages: 1,
		link: null,
		kept: true,
		first_words: null,
		preview: null,
		read_again: false,
		fills_in: false,
		recipe: null,
		reason: null,
		in_agents_words: false,
		next: [],
		...more,
	};
}

const READING = sent({
	job_id: 'j_reading',
	created_at: today(10, 41),
	status: 'running',
	link: 'https://www.youtube.com/watch?v=y2AYFpYn8ws',
});
const PHOTOS = sent({
	job_id: 'j_photos',
	created_at: today(10, 39),
	kind: 'picture',
	pages: 3,
	preview: '/api/originals/j_photos/preview',
});
const REEL = sent({
	job_id: 'j_reel',
	created_at: daysAgo(1),
	ended_at: daysAgo(1),
	status: 'failed',
	link: 'https://www.instagram.com/reel/C8kqZr3t0Lm',
	reason: 'Instagram showed a sign-in page instead of the reel.',
	in_agents_words: true,
	next: ['send_link_again', 'paste_instead'],
});
const MADE = sent({
	job_id: 'j_made',
	created_at: daysAgo(1),
	ended_at: daysAgo(1),
	status: 'completed',
	kind: 'pasted_text',
	first_words: 'Pad kra pao. Serves 2. 300 g minced pork',
	recipe: { branch_id: 'b_pad', title: 'Pad kra pao' },
});
const JUMBLE = sent({
	job_id: 'j_jumble',
	created_at: daysAgo(1),
	ended_at: daysAgo(1),
	status: 'failed',
	kind: 'pasted_text',
	first_words: 'Korean Fried Chicken. 1 kg chicken wings',
	reason: 'The pasted text is a jumble of two dishes.',
	in_agents_words: true,
	next: ['send_again'],
});
const BARE = sent({
	job_id: 'j_bare',
	created_at: daysAgo(3),
	ended_at: daysAgo(3),
	status: 'failed',
	link: 'https://en.wikipedia.org/wiki/Tarte_Tatin',
	fills_in: true,
	recipe: { branch_id: 'b_tatin', title: 'Tarte Tatin' },
	reason: 'Kamosu tried this Job 3 times, and it never finished.',
	next: ['read_recipe_again'],
});
const STOPPED = sent({
	job_id: 'j_stopped',
	created_at: daysAgo(5),
	ended_at: daysAgo(5),
	status: 'cancelled',
	link: 'https://www.tiktok.com/@someone/video/1',
});
const GONE = sent({
	job_id: 'j_gone',
	created_at: daysAgo(40),
	ended_at: daysAgo(40),
	status: 'completed',
	kind: 'picture',
	kept: false,
	recipe: { branch_id: 'b_cake', title: 'Orange drizzle cake' },
});

const EVERY = [READING, PHOTOS, REEL, MADE, JUMBLE, BARE, STOPPED, GONE];

const signedIn: Answers = {
	list_sessions: { sessions: [] },
	list_accounts: { refuse: 'unauthorized' },
	meaning_search_status: { refuse: 'unauthorized' },
	runner_status: { has_runner: true, last_asked_at: today(10, 40) },
};

function sentPage(jobs: Sent[], more: Answers = {}) {
	return renderScreen(SentPage, { ...signedIn, list_agent_jobs: { jobs }, ...more });
}

/** The row a piece of its text is in. */
const rowOf = (text: string | RegExp) => screen.getByText(text).closest('li')!;

describe('the list of everything sent to the AI agent (#463)', () => {
	it('is a row of Add recipes on the Settings menu, under Brought in', async () => {
		renderScreen(Settings, {
			...signedIn,
			get_person: { person_id: 'p_1', name: 'Stéphane' },
		});
		const row = await screen.findByRole('link', { name: /Sent to the AI agent/ });
		expect(row).toHaveAttribute('href', '/settings/agent');
		expect(row).toHaveTextContent('Each link, text or photo, and how it ended');
		const links = screen.getAllByRole('link').map((link) => link.getAttribute('href'));
		expect(links.indexOf('/settings/agent')).toBe(links.indexOf('/imports') + 1);
	});

	it('lists each send under its day, saying what was sent and how it ended', async () => {
		sentPage(EVERY);

		expect(await screen.findByRole('heading', { name: 'Today' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Yesterday' })).toBeInTheDocument();

		const reading = rowOf('youtube.com/watch?v=y2AYFpYn8ws');
		expect(reading).toHaveTextContent('Web link · 10:41');
		expect(within(reading).getByText('Reading')).toBeInTheDocument();

		const photos = rowOf('3 photos');
		expect(photos).toHaveTextContent('Photos · 10:39');
		expect(photos).toHaveTextContent("Waiting for Kamosu's AI agent");
		expect(photos.querySelector('img')).toHaveAttribute('src', '/api/originals/j_photos/preview');

		const made = rowOf(/Pad kra pao\. Serves 2/);
		expect(made).toHaveTextContent('“Pad kra pao. Serves 2. 300 g minced pork”');
		expect(within(made).getByRole('link', { name: 'Pad kra pao' })).toHaveAttribute(
			'href',
			'/recipes/b_pad',
		);

		const reel = rowOf('instagram.com/reel/C8kqZr3t0Lm');
		expect(reel).toHaveTextContent("Didn't work");
		expect(reel).toHaveTextContent(
			'The AI agent said: "Instagram showed a sign-in page instead of the reel."',
		);
		expect(within(reel).getByRole('button', { name: 'Try again' })).toBeInTheDocument();
		expect(
			within(reel).getByRole('button', { name: 'Paste the text instead' }),
		).toBeInTheDocument();

		// Kamosu's own words are not put in the agent's mouth.
		const bare = rowOf('en.wikipedia.org/wiki/Tarte_Tatin');
		expect(bare).toHaveTextContent(
			"The AI agent couldn't finish: Kamosu tried this Job 3 times, and it never finished.",
		);
		expect(bare).toHaveTextContent('The recipe stays as Kamosu read it: Tarte Tatin');
		expect(within(bare).getByRole('button', { name: 'Read it again' })).toBeInTheDocument();

		expect(rowOf('tiktok.com/@someone/video/1')).toHaveTextContent(
			'You stopped it. Nothing arrived.',
		);

		const gone = rowOf('A photo. Kamosu keeps one for 30 days only.');
		expect(gone.querySelector('img')).toBeNull();
		expect(within(gone).getByRole('link', { name: 'Orange drizzle cake' })).toBeInTheDocument();
	});

	it('shows only what did not work, or only what is still working', async () => {
		sentPage(EVERY);

		await fireEvent.click(await screen.findByRole('button', { name: "Didn't work 3" }));
		const shown = () => screen.getAllByRole('listitem').length;
		expect(shown()).toBe(3);
		expect(screen.queryByText('youtube.com/watch?v=y2AYFpYn8ws')).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Still working 2' }));
		expect(shown()).toBe(2);
		expect(screen.getByRole('button', { name: 'Still working 2' })).toHaveAttribute(
			'aria-pressed',
			'true',
		);

		await fireEvent.click(screen.getByRole('button', { name: 'All 8' }));
		expect(shown()).toBe(8);
	});

	it('sends the same file again, and opens the new Job', async () => {
		const { kamosu } = sentPage([JUMBLE], {
			read_job_again: { job_id: 'j_again', source_kind: 'pasted_text' },
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Send the same text again' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_again'));
		expect(kamosu.calls).toContainEqual({
			operation: 'read_job_again',
			input: { job_id: 'j_jumble' },
		});
	});

	it('reads a recipe again, and opens the new Job', async () => {
		const { kamosu } = sentPage([BARE], {
			read_recipe_again: { job_id: 'j_read', branch_id: 'b_tatin' },
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Read it again' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_read'));
		expect(kamosu.calls).toContainEqual({
			operation: 'read_recipe_again',
			input: { branch_id: 'b_tatin' },
		});
	});

	it('says why, under the row, when asking again is refused', async () => {
		sentPage([JUMBLE], {
			read_job_again: { refuse: 'bad_request', message: 'Kamosu deleted what was handed over.' },
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Send the same text again' }));
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Kamosu deleted what was handed over.',
		);
		expect(went).not.toHaveBeenCalled();
	});

	it('sends a link again as the + sends one', async () => {
		const { kamosu } = sentPage([REEL], {
			import_by_agent: { job_id: 'j_link', source_kind: 'web' },
		});

		await fireEvent.click(await screen.findByRole('button', { name: 'Try again' }));
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_link'));
		expect(kamosu.calls).toContainEqual({
			operation: 'import_by_agent',
			input: { link: REEL.link },
		});
	});

	it('offers another photo where the agent said this one holds no recipe', async () => {
		sentPage([
			sent({
				status: 'failed',
				kind: 'picture',
				reason: 'The picture you sent is solid black.',
				in_agents_words: true,
				next: ['choose_another'],
			}),
		]);

		const choose = await screen.findByRole('button', { name: 'Choose another photo' });
		const field = document.querySelector<HTMLInputElement>('input[type="file"][accept^="image"]')!;
		const opened = vi.spyOn(field, 'click');
		await fireEvent.click(choose);
		expect(opened).toHaveBeenCalled();
	});

	it('opens the paste box under a link that made nothing', async () => {
		sentPage([REEL]);

		const paste = await screen.findByRole('button', { name: 'Paste the text instead' });
		expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
		await fireEvent.click(paste);
		expect(paste).toHaveAttribute('aria-expanded', 'true');
		expect(rowOf('instagram.com/reel/C8kqZr3t0Lm')).toHaveTextContent(
			'The recipe from instagram.com/reel/C8kqZr3t0Lm, pasted here:',
		);
		expect(
			within(rowOf('instagram.com/reel/C8kqZr3t0Lm')).getByRole('textbox'),
		).toBeInTheDocument();
		// It sends to the agent, as the + does on an instance with a Runner.
		expect(
			await within(rowOf('instagram.com/reel/C8kqZr3t0Lm')).findByRole('button', {
				name: 'Send',
			}),
		).toBeInTheDocument();
	});

	it('reads the list again while the agent waits on a row', async () => {
		const { kamosu } = sentPage([PHOTOS]);
		await screen.findByText('3 photos');

		kamosu.answer('list_agent_jobs', {
			jobs: [
				{
					...PHOTOS,
					status: 'completed',
					ended_at: today(10, 45),
					recipe: { branch_id: 'b_card', title: 'Apple cake' },
				},
			],
		});
		expect(
			await screen.findByRole('link', { name: 'Apple cake' }, { timeout: 5000 }),
		).toBeInTheDocument();
	});

	it('keeps reading again after one reading fails, while a row still waits', async () => {
		const { kamosu } = sentPage([PHOTOS]);
		await screen.findByText('3 photos');

		kamosu.answer('list_agent_jobs', { refuse: 'busy' });
		await vi.waitFor(
			() => expect(kamosu.calls.filter((c) => c.operation === 'list_agent_jobs')).toHaveLength(2),
			{ timeout: 5000 },
		);
		expect(screen.getByText('3 photos')).toBeInTheDocument();

		kamosu.answer('list_agent_jobs', {
			jobs: [
				{
					...PHOTOS,
					status: 'completed',
					ended_at: today(10, 45),
					recipe: { branch_id: 'b_card', title: 'Apple cake' },
				},
			],
		});
		expect(
			await screen.findByRole('link', { name: 'Apple cake' }, { timeout: 5000 }),
		).toBeInTheDocument();
	}, 15000);

	it('says when nothing has been sent', async () => {
		sentPage([]);
		expect(await screen.findByText(/Nothing sent to the AI agent yet\./)).toBeInTheDocument();
		expect(screen.queryByRole('group')).not.toBeInTheDocument();
	});
});
