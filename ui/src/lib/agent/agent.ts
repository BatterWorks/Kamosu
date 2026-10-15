/**
 * An agent's Job as a cook sees it (#277): where it stands, and whether an
 * agent is there to take it.
 *
 * A Job an agent reads waits until a Runner asks for work and is running
 * while one holds it (ADR 0048). Those two states are the tracker's Waiting
 * and Reading. Whether a Runner is there is read from `runner_status`, which
 * says when one was last heard from, asking for work or renewing its hold on
 * a Job.
 */

import type { GetJobOutput, RunnerStatusOutput } from '$lib/api/catalogue';
import type { Arrival } from '$lib/imports';

/** Where an agent's Job stands while it has not ended. */
export type Working = 'waiting' | 'reading';

/**
 * How long a Runner may go unheard before a waiting Job says no agent is
 * answering. Written once, here (#277). A Runner with nothing to do asks every
 * 10 seconds, one that lost its connection tries again within 2 minutes, and
 * one working on a Job renews its hold every 40 seconds, so 5 minutes of
 * silence is no Runner.
 */
export const LATELY_MS = 5 * 60 * 1000;

/** How often a screen with an agent's Job on it reads the Job again. */
export const EVERY_MS = 3000;

/** The Operation of a Job an agent reads: its own Import, or another's recipe (#269, #272). */
export const AGENTS_OWN = 'read_by_agent';

/** Where on the Operator's screen the Runner Keys are, which a notice links to. */
export const RUNNER_KEYS = 'runner-keys';

/** Which sentence goes under the steps. */
export type Saying = 'leave' | 'report' | 'updating';

/** The tracker's state for a Job, or none once it has ended. */
export const working = (status: GetJobOutput['status'] | undefined): Working | undefined =>
	status === 'queued' ? 'waiting' : status === 'running' ? 'reading' : undefined;

/**
 * Whether no agent is answering: no Runner, or none heard from lately. The
 * time is the server's and the clock is this device's, so a clock a minute
 * out moves the notice by a minute, which 5 minutes absorbs.
 */
export function noAgentAnswering(status: RunnerStatusOutput, now: Date = new Date()): boolean {
	if (!status.has_runner || status.last_asked_at === null) return true;
	return now.getTime() - new Date(status.last_asked_at).getTime() > LATELY_MS;
}

/**
 * Where the agent's work on one arrival stands, for its row in a list. One
 * that reads counts over one that waits, since it is the further along.
 */
export function arrivalWorking(arrival: Arrival): Working | undefined {
	const states = (arrival.agent_jobs ?? []).map((job) => working(job.status));
	if (states.includes('reading')) return 'reading';
	if (states.includes('waiting')) return 'waiting';
	return undefined;
}
