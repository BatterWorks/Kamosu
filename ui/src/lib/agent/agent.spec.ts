/**
 * When no agent is answering, and where an arrival's agent stands (#277).
 */

import { describe, expect, it } from 'vitest';
import { LATELY_MS, arrivalWorking, noAgentAnswering } from './agent';
import type { Arrival } from '$lib/imports';

const NOW = new Date('2026-10-15T13:20:00.000Z');
const ago = (ms: number) => new Date(NOW.getTime() - ms).toISOString();

describe('whether an agent is answering', () => {
	it('is not, with no Runner, or one never heard from', () => {
		expect(noAgentAnswering({ has_runner: false, last_asked_at: ago(1000) }, NOW)).toBe(true);
		expect(noAgentAnswering({ has_runner: true, last_asked_at: null }, NOW)).toBe(true);
	});

	it('is, heard from within the last five minutes, and is not after', () => {
		expect(noAgentAnswering({ has_runner: true, last_asked_at: ago(LATELY_MS - 1000) }, NOW)).toBe(
			false,
		);
		expect(noAgentAnswering({ has_runner: true, last_asked_at: ago(LATELY_MS + 1000) }, NOW)).toBe(
			true,
		);
	});
});

describe("where an arrival's agent stands", () => {
	const arrival = (statuses: Arrival['status'][]): Arrival => ({
		job_id: 'j_1',
		status: 'completed',
		created_at: NOW.toISOString(),
		arrived: 1,
		created: 1,
		offered: 0,
		unreadable: 0,
		agent_jobs: statuses.map((status, index) => ({
			job_id: `j_a${index}`,
			branch_id: `b_${index}`,
			status,
		})),
	});

	it('is reading over waiting, and nothing once every agent has ended', () => {
		expect(arrivalWorking(arrival(['queued', 'running']))).toBe('reading');
		expect(arrivalWorking(arrival(['queued', 'completed']))).toBe('waiting');
		expect(arrivalWorking(arrival(['completed', 'cancelled', 'failed']))).toBeUndefined();
		expect(arrivalWorking({ ...arrival([]), agent_jobs: undefined })).toBeUndefined();
	});
});
