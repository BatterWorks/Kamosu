/**
 * A Replay (#406): an example's ended Job, answered as it stood while it
 * waited and was read, for a few seconds from when it was picked, and as it
 * is after.
 */

import { describe, expect, it } from 'vitest';
import { standIn } from '$lib/api/stand-in';
import type { GetJobOutput, ListImportsOutput } from '$lib/api/catalogue';
import {
	READY_MS,
	WAITING_MS,
	replayAddress,
	replayIn,
	replayingClient,
	replayedStatus,
	type Replaying,
} from './replay';

const PICKED = Date.parse('2026-10-22T19:42:00.000Z');

/** The example's Job as the Sample Library holds it: ended, its recipe made. */
const ENDED: GetJobOutput = {
	id: 'j_card',
	operation: 'read_by_agent',
	status: 'completed',
	progress: {},
	error: null,
	errorCode: null,
	created_at: '2026-09-30T10:00:00.000Z',
	updated_at: '2026-09-30T10:04:00.000Z',
	result: {
		import_id: 'i_picture',
		cookbook_id: 'c_camille',
		source_kind: 'picture',
		arrived: [{ branch_id: 'b_orange', title: 'Orange Cake', status: 'created' }],
		offered: [],
		unreadable: [],
		left_out: [],
		related_candidates: [],
		agent_jobs: [
			{
				job_id: 'j_card',
				branch_id: 'b_orange',
				status: 'completed',
				made_by_agent: true,
				agent_report: { left_undone: ['The card gives no yield.'] },
			},
		],
	},
};

const LISTED: ListImportsOutput = {
	imports: [
		{
			import_id: 'i_picture',
			source_kind: 'picture',
			created_at: '2026-09-30T10:00:00.000Z',
			remembered: 1,
			arrivals: [
				{
					job_id: 'j_card',
					status: 'completed',
					created_at: '2026-09-30T10:00:00.000Z',
					arrived: 1,
					created: 1,
					offered: 0,
					unreadable: 0,
					agent_jobs: [{ job_id: 'j_card', branch_id: 'b_orange', status: 'completed' }],
				},
				{
					job_id: 'j_other',
					status: 'completed',
					created_at: '2026-09-20T10:00:00.000Z',
					arrived: 1,
					created: 1,
					offered: 0,
					unreadable: 0,
				},
			],
		},
	],
};

function replayed(at: number, replay: Replaying | null = { jobId: 'j_card', startedAt: PICKED }) {
	const kamosu = standIn({
		get_job: (input: { job_id: string }) => ({ ...ENDED, id: input.job_id }),
		list_imports: LISTED,
		cancel_job: { cancelled: true },
	});
	let now = at;
	const client = replayingClient(
		kamosu.client,
		() => replay,
		() => now,
	);
	return { client, kamosu, later: (ms: number) => (now += ms) };
}

describe('a Replay', () => {
	it('is asked for by its address, from the moment it was picked', () => {
		const address = replayAddress('j_card', PICKED);
		expect(address).toBe(`/imports/j_card?replay=${PICKED}`);
		expect(replayIn(new URL(address, 'http://kamosu.test'), 'j_card')).toEqual({
			jobId: 'j_card',
			startedAt: PICKED,
		});
		expect(replayIn(new URL('/imports/j_card', 'http://kamosu.test'), 'j_card')).toBeNull();
		expect(
			replayIn(new URL('/imports/j_card?replay=soon', 'http://kamosu.test'), 'j_card'),
		).toBeNull();
	});

	it('waits, then is read, then is the Job as it is', () => {
		const replay = { jobId: 'j_card', startedAt: PICKED };
		expect(replayedStatus(replay, 'j_card', PICKED)).toBe('queued');
		expect(replayedStatus(replay, 'j_card', PICKED + WAITING_MS)).toBe('running');
		expect(replayedStatus(replay, 'j_card', PICKED + READY_MS - 1)).toBe('running');
		expect(replayedStatus(replay, 'j_card', PICKED + READY_MS)).toBeNull();
		// Another Job, and a Replay picked in the future, are no Replay.
		expect(replayedStatus(replay, 'j_other', PICKED)).toBeNull();
		expect(replayedStatus(replay, 'j_card', PICKED - 1)).toBeNull();
	});

	it('answers the Job as it read while it waited: sent when picked, nothing arrived, no recipe yet', async () => {
		const { client } = replayed(PICKED + 500);
		const job = await client.getJob({ job_id: 'j_card' });
		expect(job.status).toBe('queued');
		expect(job.created_at).toBe('2026-10-22T19:42:00.000Z');
		expect(job.result).toEqual({
			import_id: 'i_picture',
			cookbook_id: 'c_camille',
			source_kind: 'picture',
			arrived: [],
			offered: [],
			unreadable: [],
			left_out: [],
			related_candidates: [],
			agent_jobs: [{ job_id: 'j_card', branch_id: null, status: 'queued' }],
		});
	});

	it('is read from when it stopped waiting, and is the real Job once Ready', async () => {
		const { client, later } = replayed(PICKED + 500);
		later(WAITING_MS);
		const reading = await client.getJob({ job_id: 'j_card' });
		expect(reading.status).toBe('running');
		expect(reading.updated_at).toBe('2026-10-22T19:42:01.500Z');

		later(READY_MS);
		expect(await client.getJob({ job_id: 'j_card' })).toEqual(ENDED);
	});

	it('answers every other Job, and every Job with no Replay, as it is', async () => {
		expect(await replayed(PICKED).client.getJob({ job_id: 'j_other' })).toEqual({
			...ENDED,
			id: 'j_other',
		});
		expect(await replayed(PICKED, null).client.getJob({ job_id: 'j_card' })).toEqual(ENDED);
	});

	it("draws the example's row in the list as it read then, and no other", async () => {
		const { client } = replayed(PICKED + 2000);
		const [source] = (await client.listImports()).imports;
		expect(source.arrivals[0]).toEqual({
			job_id: 'j_card',
			status: 'running',
			created_at: '2026-10-22T19:42:00.000Z',
			arrived: 0,
			created: 0,
			offered: 0,
			unreadable: 0,
			agent_jobs: [{ job_id: 'j_card', branch_id: null, status: 'running' }],
		});
		expect(source.arrivals[1]).toEqual(LISTED.imports[0].arrivals[1]);
	});

	it('is stopped in this browser alone, and sends nothing to stop', async () => {
		// A moment of its own, so no other test's Replay is the one stopped.
		const startedAt = PICKED + 3_600_000;
		const { client, kamosu, later } = replayed(startedAt + 2000, { jobId: 'j_card', startedAt });
		expect(await client.cancelJob({ job_id: 'j_card' })).toEqual({ cancelled: true });
		expect(kamosu.calls.some((call) => call.operation === 'cancel_job')).toBe(false);
		later(READY_MS);
		const stopped = await client.getJob({ job_id: 'j_card' });
		expect(stopped.status).toBe('cancelled');
		expect((stopped.result as { agent_jobs: unknown[] }).agent_jobs).toEqual([
			{ job_id: 'j_card', branch_id: null, status: 'cancelled' },
		]);

		// Picked again, it plays again from Sent.
		const again = replayed(startedAt + READY_MS + 100, {
			jobId: 'j_card',
			startedAt: startedAt + READY_MS,
		});
		expect((await again.client.getJob({ job_id: 'j_card' })).status).toBe('queued');
	});

	it('is stopped here when Stop lands just after it turned Ready', async () => {
		const startedAt = PICKED + 7_200_000;
		const { client, kamosu } = replayed(startedAt + READY_MS + 200, {
			jobId: 'j_card',
			startedAt,
		});
		expect(await client.cancelJob({ job_id: 'j_card' })).toEqual({ cancelled: true });
		expect(kamosu.calls.some((call) => call.operation === 'cancel_job')).toBe(false);
	});

	it('stops a Job that is no Replay as it always did', async () => {
		const { client, kamosu } = replayed(PICKED);
		await client.cancelJob({ job_id: 'j_other' });
		expect(kamosu.calls.find((call) => call.operation === 'cancel_job')?.input).toEqual({
			job_id: 'j_other',
		});
	});
});
