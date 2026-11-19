/**
 * Throwaway instances of Kamosu for the dev tools that must never touch the
 * dev instance: `just ux-measure` (#629) and `just ux-walk` (#639). Each is
 * the debug binary on 127.0.0.1, on a spare port, its data in a folder under
 * the system's temp folder, and gone when the run ends, pass or fail.
 */

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { ToolError } from './dev.mjs';

/**
 * What a run leaves standing until it ends: the throwaway instances' process
 * groups and their folder. A stop from the terminal does not reach the
 * instances, which are started apart, and Playwright ends the process on one
 * as soon as its browser is closed, so `leaveNothingNow` does it all without
 * handing control back. It ends the instances outright, since what they hold
 * is a copy made to be thrown away: one asked to stop politely writes to its
 * folder on its way out, and wrote it back after it was removed.
 */
const standing = { groups: new Set(), folder: '' };

/** End every instance at once, give them a moment to be gone, and remove the folder. */
export function leaveNothingNow() {
	for (const group of standing.groups) {
		try {
			process.kill(-group, 'SIGKILL');
		} catch {
			// Already gone.
		}
	}
	if (standing.groups.size > 0) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
	standing.groups.clear();
	if (standing.folder) fs.rmSync(standing.folder, { recursive: true, force: true });
}

/**
 * The folder a run's instances keep their data in, under the system's temp
 * folder, removed by `leaveNothingNow`. Stopped from the terminal, the run
 * stops its instances and removes the folder before it ends: they are started
 * apart from it, so the signal would not reach them.
 * @param {string} tool the tool's name, which the folder's name starts with
 */
export function throwawayFolder(tool) {
	for (const signal of /** @type {const} */ (['SIGINT', 'SIGTERM'])) {
		process.once(signal, () => {
			console.error('stopped: the throwaway instances are stopping and their folder is gone');
			leaveNothingNow();
			process.exit(130);
		});
	}
	standing.folder = fs.mkdtempSync(path.join(os.tmpdir(), `kamosu-${tool}-`));
	return standing.folder;
}

/** A port nothing on this machine is listening on, asked of the system. */
async function sparePort() {
	return new Promise((done, failed) => {
		const server = net.createServer();
		server.on('error', failed);
		server.listen(0, '127.0.0.1', () => {
			const { port } = /** @type {net.AddressInfo} */ (server.address());
			server.close(() => done(port));
		});
	});
}

/**
 * The environment a throwaway instance runs in: this one's, with its data,
 * address and port, and never a Demo's switch, which would refuse sign-in.
 * @param {string} data
 * @param {number} port
 */
export function instanceEnv(data, port) {
	const env = { ...process.env };
	delete env.KAMOSU_DEMO_ERASES_LIBRARY_DAILY;
	return { ...env, KAMOSU_DATA_DIR: data, KAMOSU_ADDRESS: '127.0.0.1', KAMOSU_PORT: String(port) };
}

/**
 * One throwaway Kamosu: the debug binary, bound to 127.0.0.1 on a spare port,
 * its data in `data`, its log in `log`, outside the folder removed at the
 * end so a failed start can still be read, stopped as the process group it
 * was started as. `api` is where Node reaches it; `origin` is the same instance
 * as the browser opens it, at `localhost`, which Chrome holds safe enough for
 * a Session's Secure cookie.
 * @param {string} binary
 * @param {string} data
 * @param {string} logFile
 */
export async function startInstance(binary, data, logFile) {
	const port = await sparePort();
	const log = fs.openSync(logFile, 'w');
	const child = spawn(binary, ['serve'], {
		env: instanceEnv(data, port),
		stdio: ['ignore', log, log],
		detached: true,
	});
	/** @type {Error | undefined} */
	let failed;
	child.on('error', (error) => (failed = error));
	const exited = new Promise((done) => child.on('close', done));
	if (child.pid !== undefined) standing.groups.add(child.pid);
	const stop = async () => {
		if (child.pid === undefined || child.exitCode !== null || child.signalCode !== null) return;
		process.kill(-child.pid, 'SIGTERM');
		const late = setTimeout(() => child.pid && process.kill(-child.pid, 'SIGKILL'), 60_000);
		await exited;
		clearTimeout(late);
		standing.groups.delete(child.pid);
	};
	const api = `http://127.0.0.1:${port}`;
	for (let tries = 0; ; tries++) {
		if (failed || child.exitCode !== null) {
			throw new ToolError(
				`a throwaway Kamosu did not start (${failed ?? `exit ${child.exitCode}`}); its log is ${logFile}`,
			);
		}
		const up = await fetch(`${api}/api/op/instance_status`, { method: 'POST', body: '{}' })
			.then(() => true)
			.catch(() => false);
		if (up) break;
		if (tries > 600)
			throw new ToolError(
				`a throwaway Kamosu did not answer within a minute; its log is ${logFile}`,
			);
		await new Promise((done) => setTimeout(done, 100));
	}
	return { api, origin: `http://localhost:${port}`, stop };
}
