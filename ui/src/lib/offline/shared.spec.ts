/**
 * The service worker's half of Android's share target (#452): what a share
 * is turned into, and where the page goes after.
 */

import { describe, expect, it } from 'vitest';
import { readAnswer, receiveShare } from './shared';

const ORIGIN = 'https://kamosu.example';

function shared(fields: Record<string, string | File[]>, referrer = ''): Request {
	const form = new FormData();
	for (const [name, value] of Object.entries(fields)) {
		if (typeof value === 'string') form.append(name, value);
		else for (const file of value) form.append(name, file);
	}
	// What a browser hands the worker, which reads only the form. jsdom's own
	// FormData does not cross into the Request the test runner builds.
	return { formData: async () => form, referrer } as unknown as Request;
}

/** A server that answers what each path is given, and notes what it was sent. */
function server(addThis: unknown) {
	const sent: { path: string; body: string }[] = [];
	let uploads = 0;
	const send = async (request: Request) => {
		const path = new URL(request.url).pathname;
		sent.push({ path, body: await request.clone().text() });
		const result = path === '/api/uploads' ? { upload_id: `u_${++uploads}` } : (addThis as object);
		return new Response(JSON.stringify({ ok: true, result }));
	};
	return { send, sent };
}

/** Cache Storage as the worker has it, held in memory. */
function aCache(): CacheStorage {
	const stores = new Map<string, Map<string, Response>>();
	const open = async (name: string) => {
		if (!stores.has(name)) stores.set(name, new Map());
		const store = stores.get(name)!;
		return {
			put: async (key: string, response: Response) => void store.set(key, response),
			match: async (key: string) => store.get(key)?.clone(),
		} as unknown as Cache;
	};
	return { open, delete: async (name: string) => stores.delete(name) } as unknown as CacheStorage;
}

const caches = aCache();

/** What the page the worker sent it to shows. */
const landsOn = async (response: Response) =>
	readAnswer(caches, new URL(response.headers.get('location')!).searchParams);

describe('a share from Android', () => {
	it('hands a link to add_this as a link, and goes to the page with its answer', async () => {
		const { send, sent } = server({
			outcome: 'added',
			message: 'Added to your shelf: Lemon tart',
			branch_id: 'b_1',
			title: 'Lemon tart',
			shortcut_version: 1,
		});
		const response = await receiveShare(
			shared({ title: 'Lemon tart', text: 'Look!', url: 'https://example.com/tart' }),
			send,
			ORIGIN,
			caches,
		);
		expect(response.status).toBe(303);
		expect(sent).toEqual([
			{ path: '/api/op/add_this', body: JSON.stringify({ link: 'https://example.com/tart' }) },
		]);
		expect(await landsOn(response)).toEqual({
			outcome: 'added',
			message: 'Added to your shelf: Lemon tart',
			branchId: 'b_1',
			title: 'Lemon tart',
		});
	});

	it('stages each file in order, then hands them over together', async () => {
		const { send, sent } = server({ outcome: 'sent', message: 'Sent.', shortcut_version: 1 });
		const page = (name: string) => new File(['x'], name, { type: 'image/jpeg' });
		await receiveShare(shared({ files: [page('1.jpg'), page('2.jpg')] }), send, ORIGIN, caches);
		expect(sent.map((each) => each.path)).toEqual([
			'/api/uploads',
			'/api/uploads',
			'/api/op/add_this',
		]);
		expect(JSON.parse(sent[2].body)).toEqual({
			files: [{ upload_id: 'u_1' }, { upload_id: 'u_2' }],
		});
	});

	it('hands text over as text, which Kamosu reads as a link when it is one', async () => {
		const { send, sent } = server({ outcome: 'sent', message: 'Sent.', shortcut_version: 1 });
		await receiveShare(shared({ text: 'https://instagram.com/p/x' }), send, ORIGIN, caches);
		expect(JSON.parse(sent[0].body)).toEqual({ text: 'https://instagram.com/p/x' });
	});

	it('says offline that nothing was sent, and keeps nothing', async () => {
		const response = await receiveShare(
			shared({ url: 'https://example.com/tart' }),
			async () => {
				throw new TypeError('Failed to fetch');
			},
			ORIGIN,
			caches,
		);
		expect((await landsOn(response))?.outcome).toBe('offline');
	});

	it('says so when the share carried nothing', async () => {
		const { send, sent } = server({});
		const response = await receiveShare(shared({ title: 'x' }), send, ORIGIN, caches);
		expect((await landsOn(response))?.outcome).toBe('nothing');
		expect(sent).toEqual([]);
	});

	it('turns away a share a page on another site sent, adding nothing', async () => {
		const { send, sent } = server({});
		const response = await receiveShare(
			shared({ text: 'hello' }, 'https://elsewhere.example/page'),
			send,
			ORIGIN,
			caches,
		);
		expect(response.headers.get('location')).toBe(`${ORIGIN}/`);
		expect(sent).toEqual([]);
	});

	it('shows nothing an address made elsewhere says', async () => {
		const search = new URLSearchParams({ outcome: 'added', message: 'Pay here' });
		expect(await readAnswer(caches, search)).toBeUndefined();
	});
});
