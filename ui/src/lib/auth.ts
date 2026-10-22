import { getContext, setContext } from 'svelte';
import { OperationError, readEnvelope } from './api/client';
import { watched } from './mistake.svelte';
import { sessionBegan } from './offline/library.svelte';
import { presentPasskey } from './passkeys';

export interface AuthClient {
	authenticate(
		mode: 'first-person' | 'login' | 'invite' | 'recover',
		input: { name?: string; password: string; session_name: string; link?: string },
	): Promise<void>;
	/**
	 * Sign in with a passkey the device holds for this address (#370). Says
	 * `declined` when the person closed the device's prompt, which is no
	 * mistake and says nothing.
	 */
	signInWithPasskey(sessionName: string): Promise<'signed-in' | 'declined'>;
	/**
	 * Ask Kamosu for a passkey challenge before the button is pressed, so the
	 * press asks the device at once. Safari opens the device's prompt only
	 * close to a tap, and a round trip in between can be too long.
	 */
	readyPasskey(): void;
	/**
	 * Arrive in a Demo as its visitor, with no name and no password (ADR
	 * 0052). Refused on any instance that is not a Demo.
	 */
	arrive(sessionName: string): Promise<void>;
}

const KEY = Symbol('auth');

export function provideAuth(client: () => AuthClient): void {
	setContext(KEY, client);
}

export function useAuth(): AuthClient {
	const client = getContext<(() => AuthClient) | undefined>(KEY)?.();
	if (!client) throw new Error('no authentication client in context');
	return client;
}

/**
 * Authenticating is the one ask a screen makes that cannot be an Operation —
 * there is no Credential yet — so the Catalogue's client carries none of it,
 * and `watched` below is what puts its mistakes where an Operation's go (#98).
 */
async function reach(
	mode: Parameters<AuthClient['authenticate']>[0],
	input: Parameters<AuthClient['authenticate']>[1],
): Promise<void> {
	const path = {
		'first-person': '/auth/first-person',
		login: '/auth/login',
		invite: '/auth/invite',
		recover: '/auth/recover',
	}[mode];

	await post(path, input);
	// Whoever this is, the library on this phone was not filled for them (#76).
	sessionBegan();
}

/** A POST to an `/auth/…` route, refused in the Core's own words. */
async function post(path: string, body: object): Promise<Response> {
	let response: Response;
	try {
		response = await fetch(path, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body),
		});
	} catch (cause) {
		// Never reached, named exactly as `httpTransport` names it: a phone with
		// no network signing in is not Kamosu having gone wrong, and the bare
		// `TypeError` this used to let through would have said it was.
		throw new OperationError('authentication', 'internal', 'Kamosu could not be reached.', {
			cause,
			reached: false,
		});
	}
	// The Core's own words, in the envelope every Door route answers with. A
	// spent Invite and a wrong password are different things to be told, and
	// this used to say "authentication failed" to both (#126). Called only for
	// the refusal it throws: an answer that is not ok carries no result.
	if (!response.ok) await readEnvelope('authentication', response);
	return response;
}

type Begun = { ceremony_id: string; options: object };

/**
 * A challenge asked for ahead of the press, and when. Kamosu keeps one for ten
 * minutes and the device's prompt allows five, so one older than four is
 * asked for again rather than spent.
 */
let ready: { at: number; begun: Promise<Begun> } | undefined;
const READY_FOR = 4 * 60 * 1000;

async function begin(): Promise<Begun> {
	const answer = (await (await post('/auth/passkey/start', {})).json()) as { result: Begun };
	return answer.result;
}

function readyPasskey(): void {
	if (ready && Date.now() - ready.at < READY_FOR) return;
	const begun = begin();
	// Asked for nothing yet: a refusal here is said when the button is pressed.
	begun.catch(() => {});
	ready = { at: Date.now(), begun };
}

/**
 * Ask the device for a passkey that answers Kamosu's challenge, and Kamosu
 * for the Session it unlocks. No name: the device says whose passkey it holds.
 * Each challenge is spent once, so the next press asks for a fresh one.
 */
async function signInWithPasskey(sessionName: string): Promise<'signed-in' | 'declined'> {
	const asked = ready && Date.now() - ready.at < READY_FOR ? ready.begun : begin();
	ready = undefined;
	const begun = await asked;
	const answer = await presentPasskey(begun.options);
	if (!('credential' in answer)) return 'declined';
	await post('/auth/passkey/finish', {
		ceremony_id: begun.ceremony_id,
		credential: answer.credential,
		session_name: sessionName,
	});
	sessionBegan();
	return 'signed-in';
}

async function arrive(sessionName: string): Promise<void> {
	await post('/auth/demo', { session_name: sessionName });
	sessionBegan();
}

export const realAuth = (): AuthClient => ({
	authenticate: (mode, input) => watched(reach(mode, input)),
	signInWithPasskey: (sessionName) => watched(signInWithPasskey(sessionName)),
	readyPasskey,
	arrive: (sessionName) => watched(arrive(sessionName)),
});
