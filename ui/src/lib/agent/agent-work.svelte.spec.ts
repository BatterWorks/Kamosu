/**
 * The tracking line for an agent's Job (#277), option B as chosen on
 * 15 October 2026: Sent, Waiting, Reading, Ready, the words under them, and
 * stopping it.
 */

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { m } from '$lib/paraglide/messages';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetJobOutput } from '$lib/api/catalogue';
import AgentWorkTestHarness from './AgentWorkTestHarness.svelte';

const job = (status: GetJobOutput['status']): GetJobOutput => ({
	id: 'j_agent',
	operation: 'read_by_agent',
	status,
	progress: { done: null, total: null, message: null },
	result: null,
	error: null,
	errorCode: null,
	created_at: new Date(Date.now() - 10 * 60_000).toISOString(),
	updated_at: new Date(Date.now() - 6 * 60_000).toISOString(),
});

const HEARD = { has_runner: true, last_asked_at: new Date().toISOString() };
const SILENT = {
	has_runner: true,
	last_asked_at: new Date(Date.now() - 20 * 60_000).toISOString(),
};

function draw(answers: Answers, props: Record<string, unknown> = {}) {
	const kamosu = standIn({ runner_status: HEARD, ...answers });
	render(AgentWorkTestHarness, { props: { client: kamosu.client, ...props } });
	return kamosu;
}

/** The step the tracker says the Job is at. */
const currentStep = () =>
	screen.getAllByRole('listitem').find((step) => step.getAttribute('aria-current') === 'step');

describe("an agent's Job on the tracking line", () => {
	it('says a waiting Job is waiting, and that the cook can leave', async () => {
		draw({ get_job: job('queued') });

		await screen.findByText(m.agent_leave());
		expect(currentStep()).toHaveTextContent(m.agent_step_waiting());
		expect(screen.getByRole('button', { name: m.agent_stop() })).toBeInTheDocument();
	});

	it('follows the Job from waiting to read to ended, without being reloaded', async () => {
		const ended = vi.fn();
		const kamosu = draw({ get_job: job('queued') }, { ended });
		await screen.findByText(m.agent_leave());

		kamosu.answer('get_job', job('running'));
		await vi.waitFor(() => expect(currentStep()).toHaveTextContent(m.agent_step_reading()));
		expect(currentStep()).toHaveTextContent(/since/);

		kamosu.answer('get_job', job('completed'));
		await vi.waitFor(() =>
			expect(ended).toHaveBeenCalledWith('completed', false, expect.any(String)),
		);
		expect(screen.queryByText(m.agent_leave())).toBeNull();
	});

	it('offers stopping only to the Person who asked', async () => {
		draw({ get_job: job('running') }, { mine: false, saying: 'updating' });

		await screen.findByText(m.agent_updating());
		expect(screen.queryByRole('button', { name: m.agent_stop() })).toBeNull();
	});

	it('asks before stopping, and stops nothing when told No', async () => {
		const kamosu = draw({ get_job: job('running'), cancel_job: { cancelled: true } });
		await fireEvent.click(await screen.findByRole('button', { name: m.agent_stop() }));

		expect(screen.getByText(m.agent_stop_ask())).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('button', { name: m.agent_stop_no() }));
		expect(kamosu.calls.some((call) => call.operation === 'cancel_job')).toBe(false);
		expect(screen.getByRole('button', { name: m.agent_stop() })).toBeInTheDocument();
	});

	it('stops the Job when told Yes, and says what stays', async () => {
		const ended = vi.fn();
		const kamosu = draw({ get_job: job('queued'), cancel_job: { cancelled: true } }, { ended });
		await fireEvent.click(await screen.findByRole('button', { name: m.agent_stop() }));
		await fireEvent.click(screen.getByRole('button', { name: m.agent_stop_yes() }));

		await screen.findByText(m.agent_stopped());
		expect(kamosu.calls.find((call) => call.operation === 'cancel_job')?.input).toEqual({
			job_id: 'j_agent',
		});
		expect(ended).toHaveBeenCalledWith('cancelled', false, expect.any(String));
		// It goes on saying so after the Job reads as cancelled.
		kamosu.answer('get_job', job('cancelled'));
		await new Promise((wake) => setTimeout(wake, 30));
		expect(screen.getByText(m.agent_stopped())).toBeInTheDocument();
	});

	it('says nothing arrived when an Import with no recipe yet is stopped', async () => {
		draw({ get_job: job('queued'), cancel_job: { cancelled: true } }, { hasRecipe: false });
		await fireEvent.click(await screen.findByRole('button', { name: m.agent_stop() }));
		await fireEvent.click(screen.getByRole('button', { name: m.agent_stop_yes() }));

		expect(await screen.findByText(m.agent_stopped_nothing())).toBeInTheDocument();
	});

	it('tells a cook to let the Operator know when no agent is answering a waiting Job', async () => {
		draw({
			get_job: job('queued'),
			runner_status: SILENT,
			list_accounts: { refuse: 'unauthorized' },
		});

		await screen.findByText(new RegExp(m.agent_no_answer_cook()));
		expect(screen.getByText(new RegExp(m.agent_no_answer()))).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: m.agent_runner_keys() })).toBeNull();
	});

	it('points the Operator at the Runner Keys', async () => {
		draw({ get_job: job('queued'), runner_status: SILENT, list_accounts: { accounts: [] } });

		const keys = await screen.findByRole('link', { name: m.agent_runner_keys() });
		expect(keys).toHaveAttribute('href', '/operator#runner-keys');
	});

	it('says nothing about an agent beside a Job one is reading', async () => {
		const kamosu = draw({ get_job: job('running'), runner_status: SILENT });

		await screen.findByText(m.agent_leave());
		expect(screen.queryByText(new RegExp(m.agent_no_answer()))).toBeNull();
		expect(kamosu.calls.some((call) => call.operation === 'runner_status')).toBe(false);
	});

	it('shows nothing while the Job cannot be read, and comes back when it can', async () => {
		const kamosu = draw({ get_job: { refuse: 'internal', message: 'out of reach' } });
		await new Promise((wake) => setTimeout(wake, 30));
		expect(screen.queryByText(m.agent_leave())).toBeNull();

		kamosu.answer('get_job', job('queued'));
		expect(await screen.findByText(m.agent_leave())).toBeInTheDocument();
	});
});
