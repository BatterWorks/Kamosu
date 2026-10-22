/**
 * A Replay (#406, ADR 0052): in a Demo no model runs, so the agent is shown
 * at work on four ready examples, each an agent's Import that ended when the
 * Sample Library was built. Picking one opens its Import Report with
 * `?replay=<when>` in the address, and for a few seconds from then the Job is
 * answered as it stood while it waited and was read: Sent, Waiting, Reading.
 * Then it is answered as it is, and the Report the page reads is the real one.
 *
 * The tracker is the real one (`AgentWork.svelte`), shown the steps it would
 * see. What is replayed is the client the Report's family of screens reads
 * through, so the Report, its tracker and the list beside it agree. Nothing
 * is written: every visitor replays the same Jobs, and leaves them as they
 * were. Stopping one ends the Replay in this browser alone, until the page is
 * opened again.
 *
 * Sped up on purpose, as chosen on #406: a real read takes minutes, and the
 * tracker says so while it plays.
 */

import type { GetJobOutput, KamosuClient, ListImportsOutput } from '$lib/api/catalogue';

/** How long a Replay waits for an agent before it is read. */
export const WAITING_MS = 1500;
/** When a Replay is Ready, from when it was picked. */
export const READY_MS = 6000;
/** How often a screen reads a Job while it is replayed, so no step is missed. */
export const REPLAY_EVERY = 400;
/** The address's word for a Replay. */
export const REPLAY_PARAM = 'replay';

/** Which Job is replayed, and from when, in milliseconds. */
export interface Replaying {
	jobId: string;
	startedAt: number;
}

type Status = GetJobOutput['status'];

/** The address that replays an example's Job from `now`. */
export const replayAddress = (jobId: string, now: number = Date.now()): string =>
	`/imports/${jobId}?${REPLAY_PARAM}=${now}`;

/** The Replay an address asks for, or none. */
export function replayIn(url: URL, jobId: string | undefined): Replaying | null {
	const startedAt = Number(url.searchParams.get(REPLAY_PARAM));
	if (jobId === undefined || !Number.isSafeInteger(startedAt) || startedAt <= 0) return null;
	return { jobId, startedAt };
}

/** The Replays stopped in this browser, each by its Job and when it was picked. */
const stopped = new Set<string>();
const keyOf = (replay: Replaying) => `${replay.jobId}@${replay.startedAt}`;

/**
 * How the replayed Job stands at `now`, or none once it is Ready: the real
 * Job is then the answer. A Replay picked in the future, by an address typed
 * by hand or a clock moved back, is no Replay.
 */
export function replayedStatus(
	replay: Replaying | null,
	jobId: string,
	now: number,
): Status | null {
	if (replay === null || replay.jobId !== jobId) return null;
	if (stopped.has(keyOf(replay))) return 'cancelled';
	const gone = now - replay.startedAt;
	if (gone < 0 || gone >= READY_MS) return null;
	return gone < WAITING_MS ? 'queued' : 'running';
}

const at = (ms: number) => new Date(ms).toISOString();

/**
 * An agent's own Import as it read before its agent made the recipe: nothing
 * arrived, and its Job named with no recipe (`report_before_its_agent` and
 * `report_with_agent_jobs_as_they_stand` on the server).
 */
function reportBefore(result: unknown, status: Status): unknown {
	if (result === null || typeof result !== 'object') return result;
	const report = result as { agent_jobs?: { job_id: string }[] };
	return {
		...report,
		arrived: [],
		offered: [],
		unreadable: [],
		left_out: [],
		related_candidates: [],
		agent_jobs: (report.agent_jobs ?? []).map((agent) => ({
			job_id: agent.job_id,
			branch_id: null,
			status,
		})),
	};
}

/** The Job as it read at `status` in the Replay. */
function jobAt(job: GetJobOutput, replay: Replaying, status: Status): GetJobOutput {
	return {
		...job,
		status,
		error: null,
		errorCode: null,
		created_at: at(replay.startedAt),
		// A Job is held from the moment its Runner took it, which the tracker
		// says Reading began at.
		updated_at: at(status === 'queued' ? replay.startedAt : replay.startedAt + WAITING_MS),
		result: reportBefore(job.result, status),
	};
}

/** The list of what came in, with the replayed Job's row as it read then. */
function importsAt(
	answer: ListImportsOutput,
	replay: Replaying,
	status: Status,
): ListImportsOutput {
	return {
		imports: answer.imports.map((source) => ({
			...source,
			arrivals: source.arrivals.map((arrival) =>
				arrival.job_id !== replay.jobId
					? arrival
					: {
							...arrival,
							status,
							created_at: at(replay.startedAt),
							arrived: 0,
							created: 0,
							offered: 0,
							unreadable: 0,
							agent_jobs: (arrival.agent_jobs ?? []).map((agent) => ({
								...agent,
								branch_id: null,
								status,
							})),
						},
			),
		})),
	};
}

/**
 * `client`, with the Job `current` names replayed. Every other Operation, and
 * the Job itself once the Replay is Ready, goes through as it is. Stopping the
 * replayed Job never reaches the server, even in the moment after it turned
 * Ready and before the tracker read so: a Demo refuses `cancel_job`, and a
 * visitor who pressed Stop is owed the stop, not a refusal.
 */
export function replayingClient(
	client: KamosuClient,
	current: () => Replaying | null,
	now: () => number = Date.now,
): KamosuClient {
	const replayed: KamosuClient = Object.create(client);
	replayed.getJob = async (input) => {
		const job = await client.getJob(input);
		const replay = current();
		const status = replayedStatus(replay, input.job_id, now());
		return replay && status ? jobAt(job, replay, status) : job;
	};
	replayed.listImports = async (input) => {
		const answer = await client.listImports(input);
		const replay = current();
		const status = replay && replayedStatus(replay, replay.jobId, now());
		return replay && status ? importsAt(answer, replay, status) : answer;
	};
	replayed.cancelJob = async (input) => {
		const replay = current();
		if (replay && replay.jobId === input.job_id && now() >= replay.startedAt) {
			stopped.add(keyOf(replay));
			return { cancelled: true };
		}
		return client.cancelJob(input);
	};
	return replayed;
}
