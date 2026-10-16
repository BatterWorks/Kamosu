/**
 * What the service worker does with each request (#76, ADR 0013).
 *
 * Kept apart from `src/service-worker.ts` so it can be exercised in a test
 * with a cache and a network of the test's own: the worker file only hands
 * browser events to this.
 *
 * The rules, in one place:
 *
 * - **A read is answered from the phone first and refreshed behind**, so
 *   opening Kamosu never waits on the network for something it already holds.
 * - **A write goes to the server and nowhere else.** It is never kept, never
 *   queued and never replayed: an offline edit queue is a merge, and Kamosu
 *   never merges.
 * - **After this phone writes, the next read of anything asks the server
 *   first**, so a change you just made is never shown undone by an older copy.
 *   A write that changes no recipe, such as opening one, is not counted.
 * - **When the server does not answer, the kept copy does**, whether the
 *   network failed, something in front of Kamosu answered for it, or Kamosu
 *   could not answer just then.
 * - **What the phone keeps belongs to the Session that filled it.** Logging in,
 *   logging out, being refused and ending this phone's own Session all empty
 *   it, so a second person on the same phone never reads the first one's
 *   recipes. Ending another device's Session leaves it.
 */

import {
	ASSETS_CACHE,
	KEPT_HEADER,
	NOTES_CACHE,
	PHOTOGRAPHS_CACHE,
	READS_CACHE,
	readsChangedBy,
	policyFor,
	readKey,
	serverAnswered,
} from './reads';

export interface WorkerWorld {
	caches: CacheStorage;
	fetch: typeof fetch;
	now: () => number;
	/** The origin the app is served from. */
	origin: string;
	/** The cache holding this build's app files. */
	shell: string;
	/**
	 * Paths of what SvelteKit compiled. **Empty in development and only in
	 * development**, which is the one thing here that can tell the two apart.
	 */
	built: readonly string[];
	/**
	 * Every path put in the shell cache on install: what was compiled, and the
	 * files served as they are out of `ui/static/`. The second half is there in
	 * development too, so this says nothing about which build it is.
	 */
	precached: readonly string[];
	/** Tell every open page something: a read came back changed, or the server's reach. */
	tell: (message: WorkerMessage) => void;
}

/** Keeps the worker alive until background work finishes: the fetch event's own. */
type WaitUntil = (work: Promise<unknown>) => void;

/**
 * What a page is told: that a read it was shown has since changed, or whether
 * the server was reached — which `navigator.onLine` cannot say.
 */
export type WorkerMessage =
	{ type: 'kamosu:refreshed'; operation: string } | { type: 'kamosu:reach'; reached: boolean };

interface Envelope {
	ok?: boolean;
	result?: { current?: boolean };
	error?: { kind?: string };
}

const parse = (text: string): Envelope | undefined => {
	try {
		return JSON.parse(text) as Envelope;
	} catch {
		return undefined;
	}
};

/** Where the moment of the last write is noted, so it outlives the worker. */
const LAST_WRITE = '/__kamosu/last-write';

/**
 * How young a kept answer is when it is not refreshed behind: the server gave
 * it a moment ago, so it is as current as asking again.
 *
 * This is what stops a page chasing its own tail. Told that a read changed,
 * the page asks again — and if that ask refreshed behind too, a read whose
 * answer differs every time would be told of a change forever. Home's shuffled
 * *never cooked* (#151) is one, and it asked sixty times a second on every
 * phone that had kept it. The page's ask again arrives within milliseconds;
 * the margin is for a slow phone.
 */
const JUST_KEPT_MS = 10_000;

/** A kept answer a write changed, refreshed behind on its next ask however young. */
const OWED_HEADER = 'x-kamosu-owed';

export function createWorker(world: WorkerWorld) {
	const announce = (operation: string) => world.tell({ type: 'kamosu:refreshed', operation });

	/**
	 * Every request to the server goes through here, and every page is told
	 * each time whether it got there — every time, not only on a change, so a
	 * page opened while the server was already gone learns it from its first
	 * read rather than from its first failed button.
	 */
	async function toServer(request: Request): Promise<Response> {
		// Only an Operation's answer is judged: a picture or a stylesheet is
		// not the envelope, and says nothing about whether Kamosu is there.
		const operation = new URL(request.url).pathname.startsWith('/api/op/');
		const judge = (outcome: unknown) => {
			if (!operation) return;
			const answered = serverAnswered(outcome);
			if (answered !== undefined) world.tell({ type: 'kamosu:reach', reached: answered });
		};
		try {
			const response = await world.fetch(request);
			judge(response);
			return response;
		} catch (error) {
			judge(error);
			throw error;
		}
	}

	/**
	 * When this phone last changed something on the server. Written to the
	 * notes cache as well as held here, because the browser stops an idle
	 * worker and a fresh one would otherwise start at zero — and show an older
	 * copy over a change made a minute before.
	 */
	let lastWrite: number | undefined;

	async function lastWritten(): Promise<number> {
		if (lastWrite === undefined) {
			const noted = await (await world.caches.open(NOTES_CACHE)).match(LAST_WRITE);
			lastWrite = Number((await noted?.text()) ?? 0) || 0;
		}
		return lastWrite;
	}

	async function wrote(): Promise<void> {
		lastWrite = world.now();
		await (await world.caches.open(NOTES_CACHE)).put(LAST_WRITE, new Response(String(lastWrite)));
	}

	/**
	 * Mark what was kept for these reads as owing a refresh, so the next ask is
	 * still answered from the phone and refreshed behind it, however recently
	 * it was kept. Never removed: offline, the old copy is the only one. The
	 * refresh keeps a new copy, which carries no mark.
	 */
	async function aged(operations: readonly string[]): Promise<void> {
		const store = await world.caches.open(READS_CACHE);
		const paths = new Set(operations.map((operation) => `/__kamosu/read/${operation}`));
		for (const request of await store.keys()) {
			if (!paths.has(new URL(request.url).pathname)) continue;
			const kept = await store.match(request);
			if (!kept) continue;
			const headers = new Headers(kept.headers);
			headers.set(OWED_HEADER, '1');
			await store.put(request, new Response(await kept.text(), { headers }));
		}
	}

	/** Everything this Session's reads put on the phone, its pictures included. */
	async function forget(): Promise<void> {
		lastWrite = 0;
		await Promise.all([
			world.caches.delete(READS_CACHE),
			world.caches.delete(PHOTOGRAPHS_CACHE),
			world.caches.delete(NOTES_CACHE),
		]);
	}

	/** A write, or anything else not kept: straight to the server. */
	async function passThrough(
		request: Request,
		operation: string,
		write: boolean,
	): Promise<Response> {
		const response = await toServer(request);
		const envelope = parse(await response.clone().text());
		if (envelope?.ok && write) {
			const changed = readsChangedBy(operation);
			await (changed ? aged(changed) : wrote());
			// Ending the Session in this phone's hand is signing out, and the
			// server says whether it was (#291). Another device's leaves this
			// phone signed in and its library where it is.
			if (operation === 'revoke_session' && envelope.result?.current !== false) await forget();
		} else if (envelope?.error?.kind === 'unauthorized') {
			await forget();
		}
		return response;
	}

	async function answer(
		request: Request,
		operation: string,
		waitUntil: WaitUntil,
	): Promise<Response> {
		const policy = policyFor(operation);
		if (policy === undefined) return passThrough(request, operation, true);
		if (policy === 'never') return passThrough(request, operation, false);

		const input: unknown = await request
			.clone()
			.json()
			.catch(() => ({}));
		const key = readKey(world.origin, operation, input);
		const store = await world.caches.open(READS_CACHE);
		const kept = await store.match(key);
		// Read now: once `kept` is handed to the page its body is spent, and the
		// refresh behind needs it to tell whether anything changed.
		const before = kept ? await kept.clone().text() : undefined;

		/**
		 * The server's answer, kept when it is one. An answer that is not one —
		 * the proxy's 502 while Kamosu restarts, a hotel wifi's page, Kamosu
		 * saying it is busy or went wrong — throws as the network failing does,
		 * so the kept copy answers in its place (#291). Kamosu refusing the read,
		 * a recipe not found or a Session ended, is its answer, and is not.
		 */
		const refresh = async () => {
			const response = await toServer(request);
			if (serverAnswered(response) === false || response.status >= 500) {
				if (kept) throw new Error(`not Kamosu's answer: ${response.status}`);
				return response;
			}
			if (response.status !== 200) {
				if (response.status === 401) await refused();
				return response;
			}
			const body = await response.clone().text();
			const envelope = parse(body);
			if (envelope?.ok) {
				await store.put(
					key,
					new Response(body, {
						headers: {
							'content-type': 'application/json',
							[KEPT_HEADER]: String(world.now()),
						},
					}),
				);
				if (before !== undefined && before !== body) announce(operation);
			} else if (envelope?.error?.kind === 'unauthorized') {
				await refused();
			}
			return response;
		};

		/**
		 * The Session is gone. If the page was already shown a kept answer, it
		 * is told to ask again — which now reaches the server and is refused,
		 * so a signed-out phone lands on the sign-in form rather than a library.
		 */
		async function refused() {
			await forget();
			if (kept) announce(operation);
		}

		const keptAt = Number(kept?.headers.get(KEPT_HEADER) ?? 0);
		const current = kept !== undefined && keptAt >= (await lastWritten());
		if (policy === 'phone-first' && current) {
			if (world.now() - keptAt >= JUST_KEPT_MS || kept.headers.has(OWED_HEADER)) {
				waitUntil(refresh().catch(() => undefined));
			}
			return kept;
		}

		try {
			return await refresh();
		} catch (error) {
			if (kept) return kept;
			throw error;
		}
	}

	/**
	 * Cache first, and kept for good: the same URL is always the same bytes. A
	 * Display Copy only — `card` or `page` — since the phone keeps what it
	 * shows, and an original is for Export, PDF and Backup (ADR 0013).
	 */
	async function photograph(request: Request): Promise<Response> {
		const store = await world.caches.open(PHOTOGRAPHS_CACHE);
		const kept = await store.match(request);
		if (kept) return kept;
		const response = await toServer(request);
		if (response.status === 200) await store.put(request, response.clone());
		return response;
	}

	/** From the phone, refreshed behind: these are replaced in place on an upgrade. */
	async function asset(request: Request, waitUntil: WaitUntil): Promise<Response> {
		const store = await world.caches.open(ASSETS_CACHE);
		const kept = await store.match(request);
		const refresh = toServer(request).then(async (response) => {
			if (response.status === 200) await store.put(request, response.clone());
			return response;
		});
		if (kept) {
			waitUntil(refresh.catch(() => undefined));
			return kept;
		}
		return refresh;
	}

	/** The server first, and the last copy when there is none. */
	async function serverFirst(request: Request, cache: string): Promise<Response> {
		const store = await world.caches.open(cache);
		try {
			const response = await toServer(request);
			if (response.status === 200) await store.put(request, response.clone());
			return response;
		} catch (error) {
			const kept = await store.match(request);
			if (kept) return kept;
			throw error;
		}
	}

	/**
	 * Every screen is the same shell (ADR 0028), so any navigation inside the
	 * app is answered with it.
	 *
	 * In development SvelteKit compiles nothing, so `built` is empty and the
	 * server answers instead. Vite's shell works its base path out of the
	 * address it was opened at, so handing the copy cached at `/` to a browser
	 * asking for `/recipes/<id>` draws "404 Not Found". A compiled shell fixes
	 * its base and does not care (#95). Never ask this question of `precached`,
	 * which holds `ui/static/`'s files in development too.
	 */
	async function screen(request: Request): Promise<Response> {
		const store = await world.caches.open(world.shell);
		if (world.built.length > 0) {
			const kept = await store.match('/');
			if (kept) return kept;
		}
		try {
			return await toServer(request);
		} catch (error) {
			const kept = await store.match('/');
			if (kept) return kept;
			throw error;
		}
	}

	/**
	 * The answer to one request, or `undefined` to leave it to the browser.
	 * Anything on another origin, and anything not listed here, is left alone.
	 */
	function handle(request: Request, waitUntil: WaitUntil): Promise<Response> | undefined {
		const url = new URL(request.url);
		if (url.origin !== world.origin) return undefined;
		const path = url.pathname;

		if (request.method === 'POST') {
			if (path.startsWith('/api/op/'))
				return answer(request, path.slice('/api/op/'.length), waitUntil);
			// A new Session, whoever it belongs to: nothing kept for the last one
			// may be shown to it.
			if (path.startsWith('/auth/')) {
				return toServer(request).then(async (response) => {
					if (response.status === 200) await forget();
					return response;
				});
			}
			return undefined;
		}
		if (request.method !== 'GET') return undefined;

		if (world.precached.includes(path)) {
			return world.caches
				.open(world.shell)
				.then((store) => store.match(path))
				.then((kept) => kept ?? toServer(request));
		}
		if (/^\/api\/photographs\/[^/]+\/(card|page)$/.test(path)) return photograph(request);
		if (path.startsWith('/assets/')) return asset(request, waitUntil);
		// The Share Link page is drawn by the server, not the app (ADR 0013).
		// Kept once opened, like anything else a person has read.
		if (path.startsWith('/s/')) return serverFirst(request, READS_CACHE);
		if (request.mode === 'navigate' && !path.startsWith('/api/') && !path.startsWith('/auth/')) {
			return screen(request);
		}
		return undefined;
	}

	return { handle };
}
