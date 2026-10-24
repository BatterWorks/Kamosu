/**
 * What Android's Share menu sends Kamosu (#452, ADR 0055), caught by the
 * service worker.
 *
 * Installed from Chrome, Kamosu declares a share target in its manifest
 * (`generate-manifest.mjs`), so it is listed in every app's Share menu. A
 * share arrives as a `multipart/form-data` POST to `/shared`, which no server
 * route answers: the worker reads it, sends each file to `POST /api/uploads`
 * with the browser's own Session, hands everything to `add_this`, and sends
 * the page to `/shared` with what Kamosu said, which it keeps for that page
 * to read. Out of reach, it says so there and keeps nothing: a share is not a
 * write the outbox replays.
 *
 * It holds no rule of its own. A link is a link, files are files, and
 * anything else is text: `add_this` decides what each is and what to say.
 */

/** Where Android sends a share, and where the page that answers it is. */
export const SHARED_PATH = '/shared';

/** What the page after a share shows. */
export interface SharedAnswer {
	outcome: 'added' | 'sent' | 'already' | 'not_added' | 'offline' | 'nothing';
	message?: string;
	branchId?: string;
	/** The recipe's name, for the button that opens it. */
	title?: string;
}

/**
 * Where the worker keeps the answer for the page to read. The page's address
 * names it and carries nothing of it, so a link made elsewhere cannot put
 * words on a page of Kamosu's own. Only the latest is kept.
 */
export const SHARED_CACHE = 'kamosu-shared';
const KEPT_AT = '/__kamosu/shared/';

/** Keep an answer for the page, and say the page's address that names it. */
export async function keepAnswer(caches: CacheStorage, answer: SharedAnswer): Promise<string> {
	await caches.delete(SHARED_CACHE);
	const id = crypto.randomUUID();
	const store = await caches.open(SHARED_CACHE);
	await store.put(`${KEPT_AT}${id}`, new Response(JSON.stringify(answer)));
	return `${SHARED_PATH}?answer=${id}`;
}

/** The answer the page's address names, or none where it names none still kept. */
export async function readAnswer(
	caches: CacheStorage,
	search: URLSearchParams,
): Promise<SharedAnswer | undefined> {
	const id = search.get('answer');
	if (!id) return undefined;
	const kept = await (await caches.open(SHARED_CACHE)).match(`${KEPT_AT}${id}`);
	return kept ? ((await kept.json()) as SharedAnswer) : undefined;
}

interface Envelope {
	ok?: boolean;
	result?: {
		outcome?: SharedAnswer['outcome'];
		message?: string;
		branch_id?: string;
		title?: string;
		upload_id?: string;
	};
	error?: { message?: string };
}

/**
 * Read one share and hand it to Kamosu, answering where the page goes next.
 * `send` is the worker's way to the server.
 */
export async function receiveShare(
	request: Request,
	send: (request: Request) => Promise<Response>,
	origin: string,
	caches: CacheStorage,
): Promise<Response> {
	const go = async (answer: SharedAnswer) =>
		Response.redirect(`${origin}${await keepAnswer(caches, answer)}`, 303);
	// A share from the Share menu comes from no page. One a page on another
	// site sent would be added as this phone's Session, so it is turned away
	// here (#452). A page that sends no referrer gets past this, and what it
	// can do then is add a recipe, which ADR 0055 accepts of a leaked
	// Add-only key.
	if (request.referrer && new URL(request.referrer).origin !== origin)
		return Response.redirect(`${origin}/`, 303);
	let form: FormData;
	try {
		form = await request.formData();
	} catch {
		return go({ outcome: 'nothing' });
	}
	const text = (name: string) => {
		const value = form.get(name);
		return typeof value === 'string' ? value.trim() : '';
	};
	// Anything not a string is a file. Asked that way, and not with `instanceof`, since a
	// File made in another realm (a test's, say) is one too.
	const files = form.getAll('files').filter((value): value is File => typeof value !== 'string');
	const link = text('url');
	const shared = text('text');

	const call = async (path: string, body: BodyInit, type: string): Promise<Envelope> => {
		const response = await send(
			new Request(`${origin}${path}`, {
				method: 'POST',
				headers: { 'Content-Type': type },
				body,
				credentials: 'same-origin',
			}),
		);
		return (await response.json()) as Envelope;
	};

	try {
		let input: Record<string, unknown>;
		if (link) input = { link };
		else if (files.length > 0) {
			const ids: { upload_id: string }[] = [];
			for (const file of files) {
				const staged = await call('/api/uploads', file, 'application/octet-stream');
				if (!staged.ok || !staged.result?.upload_id)
					return go({ outcome: 'not_added', message: staged.error?.message });
				ids.push({ upload_id: staged.result.upload_id });
			}
			input = { files: ids };
		} else if (shared) input = { text: shared };
		else return go({ outcome: 'nothing' });

		const answered = await call('/api/op/add_this', JSON.stringify(input), 'application/json');
		if (!answered.ok || !answered.result?.outcome)
			return go({ outcome: 'not_added', message: answered.error?.message });
		return go({
			outcome: answered.result.outcome,
			message: answered.result.message,
			branchId: answered.result.branch_id,
			title: answered.result.title,
		});
	} catch {
		return go({ outcome: 'offline' });
	}
}
