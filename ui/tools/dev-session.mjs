/**
 * `just dev-session <label>` and `just dev-session-end <label>` (#216).
 *
 * Signing in makes a Session on the dev instance named `<label>` and keeps its
 * cookie in `.dev/sessions/<label>.cookie`. It prints that path and nothing
 * else: never the cookie, never the password. A label whose Session is still
 * good is handed back as it is, so asking twice does not pile Sessions up and
 * does not spend the sign-in throttle.
 *
 * Ending revokes that one Session, by asking the instance which Session the
 * cookie itself belongs to. No other Session is named, so none can be ended
 * by mistake.
 */

import fs from 'node:fs';
import path from 'node:path';
import {
	ROOT,
	SESSIONS_DIR,
	ToolError,
	devEnv,
	devOrigin,
	need,
	op,
	reach,
	readSecret,
	run,
	sessionFile,
	sessionOf,
} from './dev.mjs';

/**
 * The Secret out of the `Set-Cookie` lines of a sign-in's answer.
 * @param {string[]} setCookies
 */
export function secretFrom(setCookies) {
	for (const line of setCookies) {
		const match = /^kamosu_session=([^;]+)/.exec(line);
		if (match) return match[1];
	}
	return null;
}

/** @param {string} label */
async function start(label) {
	const file = sessionFile(label);
	const env = devEnv();
	const origin = devOrigin(env);
	const shown = path.relative(ROOT, file);

	if (fs.existsSync(file) && (await sessionOf(origin, readSecret(file)))) {
		console.log(shown);
		return;
	}

	const response = await reach(`${origin}/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			name: need(env, 'KAMOSU_DEV_ADMIN_NAME'),
			password: need(env, 'KAMOSU_DEV_ADMIN_PASSWORD'),
			session_name: label,
		}),
	});
	const secret = secretFrom(response.headers.getSetCookie());
	if (!response.ok || !secret) {
		const answer = await response.json().catch(() => null);
		throw new ToolError(
			`the dev instance refused the sign-in: ${answer?.error?.message ?? response.status}`,
		);
	}
	fs.mkdirSync(SESSIONS_DIR, { recursive: true });
	fs.writeFileSync(file, `${secret}\n`, { mode: 0o600 });
	fs.chmodSync(file, 0o600);
	console.log(shown);
}

/** @param {string} label */
async function end(label) {
	const file = sessionFile(label);
	if (!fs.existsSync(file)) {
		throw new ToolError(`no Session labelled ${label} was signed in from here`);
	}
	const origin = devOrigin();
	const secret = readSecret(file);
	const own = await sessionOf(origin, secret);
	if (own) {
		await op(origin, secret, 'revoke_session', { session_id: own.id });
		console.log(`ended the Session labelled ${label} (${own.id})`);
	} else {
		console.log(`the Session labelled ${label} had already ended`);
	}
	fs.rmSync(file);
}

if (process.argv[1] === import.meta.filename) {
	await run(async () => {
		const [, , verb, label] = process.argv;
		if (verb === 'start') await start(label);
		else if (verb === 'end') await end(label);
		else throw new ToolError('usage: dev-session.mjs start|end <label>');
	});
}
