/**
 * The dev tools' own rules (#216): the parts that decide a name, a refusal or
 * a difference, tested without a dev instance. How a screen is settled and
 * what the tools keep off it (#529) is tested in a real Chrome against a page
 * served here; what they do against Kamosu's own screens is checked live,
 * through the `just` recipes.
 */

import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import {
	ROOT,
	ToolError,
	chromePath,
	devEnv,
	holdCodeStill,
	holdWindowStill,
	leaveNoTrace,
	openSettled,
	readEnv,
	sessionFile,
	signedInArgs,
	slugOf,
} from './dev.mjs';
import { secretFrom } from './dev-session.mjs';
import { fileFor, folderOf, server } from './proto-serve.mjs';
import { indexOf, pageOf, photoName } from './proto-new.mjs';
import { PICTURE_LIMIT, pictureName, piecesOf } from './screens.mjs';
import { compare } from './screens-same.mjs';
import { SIZES } from './sizes.mjs';

/** @type {string} */
let tmp;
beforeEach(() => {
	tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'kamosu-tools-'));
});
afterEach(() => {
	fs.rmSync(tmp, { recursive: true, force: true });
});

/**
 * @param {string} folder
 * @param {Record<string, string>} files
 */
function fill(folder, files) {
	const dir = path.join(tmp, folder);
	fs.mkdirSync(dir, { recursive: true });
	for (const [name, content] of Object.entries(files)) {
		fs.mkdirSync(path.dirname(path.join(dir, name)), { recursive: true });
		fs.writeFileSync(path.join(dir, name), content);
	}
	return dir;
}

describe('the env file', () => {
	it('gives each value by its key, without its quotes or the comments', () => {
		const env = readEnv('# the dev account\nNAME=cook\nHOST="dev.kamosu.example"\n\nEMPTY=\n');
		expect(env).toEqual({ NAME: 'cook', HOST: 'dev.kamosu.example', EMPTY: '' });
	});
});

describe('a Session label', () => {
	it('names a cookie file under .dev/sessions', () => {
		expect(sessionFile('live-216')).toMatch(/\.dev\/sessions\/live-216\.cookie$/);
	});

	it('is refused when it would name a file somewhere else', () => {
		expect(() => sessionFile('../../etc/passwd')).toThrow(ToolError);
		expect(() => sessionFile('')).toThrow(ToolError);
	});
});

describe('signing in', () => {
	it('takes the Secret from the Session cookie and no other', () => {
		const lines = ['other=1; Path=/', '__Host-kamosu_session=abc123; Path=/; HttpOnly; Secure'];
		expect(secretFrom(lines)).toBe('abc123');
		expect(secretFrom(['other=1'])).toBeNull();
		// The name it had before #368, which no Kamosu sets any longer.
		expect(secretFrom(['kamosu_session=abc123; Path=/'])).toBeNull();
	});
});

describe('the browser', () => {
	it('is the one .env names, before anything on the path', () => {
		const dir = fill('bin', { 'google-chrome': '' });
		expect(chromePath({ KAMOSU_DEV_BROWSER: '/opt/other/chrome' }, dir)).toBe('/opt/other/chrome');
	});

	it('is the Chrome on the path when .env names none', () => {
		const dir = fill('bin', { chromium: '' });
		expect(chromePath({}, `/nowhere${path.delimiter}${dir}`)).toBe(path.join(dir, 'chromium'));
	});

	it('is asked for by name when there is none', () => {
		expect(() => chromePath({}, '/nowhere')).toThrow(/KAMOSU_DEV_BROWSER/);
	});
});

describe('a picture', () => {
	it('is named for its path and its size', () => {
		expect(slugOf('/')).toBe('home');
		expect(pictureName('/recipes/b_12?tab=notes', 'phone')).toBe(
			'recipes-b_12-tab-notes.phone.png',
		);
	});

	it('of a page too tall for one is named piece by piece', () => {
		expect(pictureName('/recipes', 'phone-sideways', 1)).toBe('recipes.phone-sideways-1.png');
		expect(pictureName('/recipes', 'phone-sideways', 2)).toBe('recipes.phone-sideways-2.png');
	});

	it('is one piece while the page fits, the limit itself included', () => {
		expect(piecesOf(844, PICTURE_LIMIT)).toEqual([{ y: 0, height: 844 }]);
		expect(piecesOf(PICTURE_LIMIT, PICTURE_LIMIT)).toEqual([{ y: 0, height: PICTURE_LIMIT }]);
	});

	it('of a taller page is pieces that stack to its whole height, none past the limit', () => {
		// The recipe list sideways on the phone, measured on #522.
		const pieces = piecesOf(40811, PICTURE_LIMIT);
		expect(pieces).toEqual([
			{ y: 0, height: 32767 },
			{ y: 32767, height: 8044 },
		]);
		for (const height of [32768, 65534, 65535, 100000]) {
			const each = piecesOf(height, PICTURE_LIMIT);
			let top = 0;
			for (const piece of each) {
				expect(piece.y).toBe(top);
				expect(piece.height).toBeGreaterThan(0);
				expect(piece.height).toBeLessThanOrEqual(PICTURE_LIMIT);
				top += piece.height;
			}
			expect(top).toBe(height);
		}
	});

	it('is taken at the phone and at every size #192 names', () => {
		expect(SIZES.phone).toEqual([390, 844]);
		expect(SIZES.macbook).toEqual([1512, 982]);
		expect(SIZES['ipad-upright']).toEqual([834, 1194]);
		expect(Object.keys(SIZES)).toHaveLength(9);
	});
});

describe('what a tool is asked for', () => {
	it('is refused before any browser starts when a path has no slash', async () => {
		await expect(signedInArgs(['--session', 'c', 'before', 'recipes'], 'x')).rejects.toThrow(
			/starts with a slash.*recipes/,
		);
	});

	it('is refused with its usage when the folder, the paths or the Session is missing', async () => {
		await expect(signedInArgs(['--session', 'c', 'before'], 'just x <folder>')).rejects.toThrow(
			'usage: just x <folder>',
		);
		await expect(signedInArgs(['before', '/recipes'], 'just x <folder>')).rejects.toThrow(
			ToolError,
		);
	});
});

describe('comparing two sets of pictures', () => {
	it('passes when every picture is the same', () => {
		const before = fill('before', { 'a.phone.png': 'one', 'b.phone.png': 'two' });
		const after = fill('after', { 'a.phone.png': 'one', 'b.phone.png': 'two' });
		expect(compare(before, after)).toEqual({ compared: 2, differences: [] });
	});

	it('names the picture that differs', () => {
		const before = fill('before', { 'a.phone.png': 'one', 'b.phone.png': 'two' });
		const after = fill('after', { 'a.phone.png': 'one', 'b.phone.png': 'TWO' });
		expect(compare(before, after).differences).toEqual(['b.phone.png: differs']);
	});

	it('counts a picture only one set has', () => {
		const before = fill('before', { 'a.phone.png': 'one', 'gone.phone.png': 'x' });
		const after = fill('after', { 'a.phone.png': 'one', 'new.phone.png': 'y' });
		const { differences } = compare(before, after);
		expect(differences).toHaveLength(2);
		expect(differences[0]).toMatch(/^gone\.phone\.png: only in .*before$/);
		expect(differences[1]).toMatch(/^new\.phone\.png: only in .*after$/);
	});

	it('pairs the pieces of a tall page by name', () => {
		const pieces = { 'r.phone-sideways-1.png': 'top', 'r.phone-sideways-2.png': 'foot' };
		expect(compare(fill('before', pieces), fill('after', pieces))).toEqual({
			compared: 2,
			differences: [],
		});
	});

	it('counts a page that became pieces, or took more of them, as differing', () => {
		const whole = fill('before', { 'r.phone-sideways.png': 'all' });
		const two = fill('after', {
			'r.phone-sideways-1.png': 'top',
			'r.phone-sideways-2.png': 'foot',
		});
		expect(compare(whole, two).differences).toEqual([
			expect.stringMatching(/^r\.phone-sideways-1\.png: only in .*after$/),
			expect.stringMatching(/^r\.phone-sideways-2\.png: only in .*after$/),
			expect.stringMatching(/^r\.phone-sideways\.png: only in .*before$/),
		]);
		const three = fill('third', {
			'r.phone-sideways-1.png': 'top',
			'r.phone-sideways-2.png': 'foot',
			'r.phone-sideways-3.png': 'more',
		});
		expect(compare(two, three).differences).toEqual([
			expect.stringMatching(/^r\.phone-sideways-3\.png: only in .*third$/),
		]);
	});

	it('refuses two empty folders, which would pass by holding nothing', () => {
		expect(() => compare(fill('before', {}), fill('after', {}))).toThrow(ToolError);
		expect(() => compare(path.join(tmp, 'missing'), fill('after', {}))).toThrow(ToolError);
	});
});

describe('a captured page', () => {
	it('loads the stylesheet from the folder’s root and asks for no icon', () => {
		const page = pageOf({
			title: 'Recipes <Kamosu>',
			htmlAttributes: ' lang="en"',
			styles: '<style>.x{}</style>\n',
			body: '<body><p>hi</p></body>',
		});
		expect(page).toContain('<html lang="en">');
		expect(page).toContain('<link rel="stylesheet" href="/assets/app.css">');
		expect(page).toContain('<link rel="icon" href="data:,">');
		expect(page).toContain('<title>Recipes &lt;Kamosu></title>');
		expect(page).not.toContain('<script');
	});

	it('keeps a photograph under a name a file can have', () => {
		expect(photoName('/api/photographs/p_1f?width=640&v=2')).toBe('p_1f_width_640_v_2');
	});

	it('is listed in the index under its path, sizes in the order of the list', () => {
		const index = indexOf('216-tools', ['recipes.macbook.html', 'recipes.phone.html']);
		expect(index.indexOf('recipes.phone.html')).toBeLessThan(index.indexOf('recipes.macbook.html'));
		expect(index).toContain('<a href="captured/recipes.phone.html">phone</a> 390 × 844');
	});
});

describe('serving a prototype', () => {
	it('serves a prototype by its name, and no other folder', () => {
		const prototypes = fill('prototypes', { '216-tools/index.html': 'hi' });
		expect(folderOf('216-tools', prototypes)).toBe(path.join(prototypes, '216-tools'));
		expect(() => folderOf('..', prototypes)).toThrow(ToolError);
		expect(() => folderOf('.', prototypes)).toThrow(ToolError);
		expect(() => folderOf('/etc', prototypes)).toThrow(ToolError);
		expect(() => folderOf('missing', prototypes)).toThrow(ToolError);
	});

	it('gives a file inside the folder and nothing outside it', () => {
		const folder = fill('proto', { 'index.html': 'hi', 'captured/a.html': 'a' });
		fill('secret', { 'key.txt': 'no' });
		expect(fileFor(folder, '/')).toBe(path.join(folder, 'index.html'));
		expect(fileFor(folder, '/captured/a.html?v=3')).toBe(path.join(folder, 'captured/a.html'));
		expect(fileFor(folder, '/../secret/key.txt')).toBeUndefined();
		expect(fileFor(folder, '/%2e%2e/secret/key.txt')).toBeUndefined();
		expect(fileFor(folder, '/missing.html')).toBeUndefined();
	});

	it('tells the browser to keep nothing, found or not', async () => {
		const folder = fill('proto', { 'index.html': '<p>hi</p>', 'assets/app.css': 'p{}' });
		const running = server(folder);
		await new Promise((resolve) => running.listen(0, '127.0.0.1', () => resolve(undefined)));
		try {
			const { port } = /** @type {import('node:net').AddressInfo} */ (running.address());
			const page = await fetch(`http://127.0.0.1:${port}/`);
			expect(page.headers.get('cache-control')).toBe('no-store');
			expect(page.headers.get('content-type')).toBe('text/html; charset=utf-8');
			expect(await page.text()).toBe('<p>hi</p>');
			const css = await fetch(`http://127.0.0.1:${port}/assets/app.css`);
			expect(css.headers.get('content-type')).toBe('text/css; charset=utf-8');
			const missing = await fetch(`http://127.0.0.1:${port}/nope`);
			expect(missing.status).toBe(404);
			expect(missing.headers.get('cache-control')).toBe('no-store');
		} finally {
			running.close();
		}
	});
});

// A real Chrome: each test takes a few seconds, more on a busy machine.
describe('a screen photographed', { timeout: 20_000 }, () => {
	/** @type {import('playwright').Browser} */
	let browser;
	/** @type {import('node:http').Server} */
	let site;
	/** Whether a page said anything over the stand-in's WebSocket. */
	let spoke = false;
	/** The stand-in's WebSockets, ended with it. */
	/** @type {import('node:stream').Duplex[]} */
	const sockets = [];
	let origin = '';
	/** Every Operation the page asked the stand-in instance for. */
	/** @type {string[]} */
	const asked = [];

	/** The pages the stand-in serves, by path. */
	const PAGES = {
		// Asks for its list only after Playwright's own "network idle" (500ms
		// with nothing open) has passed, as a pane waiting on the Person does.
		'/late': `<p id="pane">Loading</p><script>
			setTimeout(async () => {
				const answer = await fetch('/api/op/slow', { method: 'POST' });
				document.getElementById('pane').textContent = await answer.text();
			}, 800);
		</script>`,
		'/restless': `<p id="n">0</p><script>
			setInterval(() => { const n = document.getElementById('n'); n.textContent = +n.textContent + 1; }, 100);
		</script>`,
		// Loaded again once by itself, as the dev server does when it writes a
		// file it watches.
		'/reloads': `<p id="state">first</p><img src="/slow.svg"><script>
			if (sessionStorage.getItem('again')) document.getElementById('state').textContent = 'again';
			else setTimeout(() => { sessionStorage.setItem('again', '1'); location.reload(); }, 300);
		</script>`,
		// Sends itself to the path before it without loading a page, as
		// /settings/account sends itself to Signing in.
		'/goes-back': `<script>setTimeout(() => history.replaceState(null, '', '/tall'), 200);</script>`,
		// Loaded again once, at the path before it, as the dev server's reload
		// does when it lands after the tools have moved on.
		'/sent-back': `<p id="where">here</p><script>
			if (!sessionStorage.getItem('sent')) {
				sessionStorage.setItem('sent', '1');
				history.replaceState(null, '', '/tall');
				location.reload();
			}
		</script>`,
		// Hears what the stand-in's WebSocket says, as Vite's client does.
		'/socket': `<p id="heard"></p><script>
			const socket = new WebSocket(location.origin.replace('http', 'ws') + '/socket');
			socket.onopen = () => socket.send('ping');
			socket.onmessage = (event) => { document.getElementById('heard').textContent += JSON.parse(event.data).type + ' '; };
		</script>`,
		'/tall': `<div style="height: 5000px">tall</div><script>
			window.heard = [];
			addEventListener('resize', () => window.heard.push(innerWidth + 'x' + innerHeight));
		</script>`,
		'/sessions': `<pre id="out"></pre><script>
			(async () => {
				await fetch('/api/op/note_recipe_opened', { method: 'POST', body: '{}' });
				const answer = await fetch('/api/op/list_sessions', { method: 'POST', body: '{}' });
				document.getElementById('out').textContent = await answer.text();
			})();
		</script>`,
	};

	beforeAll(async () => {
		const env = fs.existsSync(path.join(ROOT, '.env')) ? devEnv() : {};
		browser = await chromium.launch({ executablePath: chromePath(env) });
		site = http.createServer((request, response) => {
			const url = request.url ?? '/';
			if (url.startsWith('/api/op/')) {
				const name = url.slice('/api/op/'.length);
				asked.push(name);
				const json = (/** @type {unknown} */ body) => {
					response.setHeader('content-type', 'application/json');
					response.end(JSON.stringify(body));
				};
				if (name === 'slow') {
					setTimeout(() => response.end('Arrived'), 400);
				} else if (name === 'list_sessions') {
					json({
						ok: true,
						result: {
							sessions: [
								{
									id: 's1',
									created_at: '2026-10-15T08:00:00Z',
									last_used_at: '2026-11-08T20:07:00Z',
									current: true,
								},
								{
									id: 's2',
									created_at: '2026-10-01T08:00:00Z',
									last_used_at: '2026-11-08T19:00:00Z',
									current: false,
								},
							],
						},
					});
				} else {
					json({ ok: true, result: {} });
				}
				return;
			}
			if (url === '/empty') {
				response.statusCode = 204;
				response.end();
				return;
			}
			if (url === '/slow.svg') {
				// Held long enough that a look at the page is waiting on it when
				// the page loads again.
				response.setHeader('content-type', 'image/svg+xml');
				setTimeout(() => response.end('<svg xmlns="http://www.w3.org/2000/svg"/>'), 600);
				return;
			}
			response.setHeader('content-type', 'text/html');
			response.end(PAGES[/** @type {keyof typeof PAGES} */ (url)] ?? '');
		});
		// A WebSocket written out by hand, as far as a test needs: the
		// handshake, five short text frames from the server, and a note of
		// whether the page said anything back.
		site.on('upgrade', (request, socket) => {
			sockets.push(socket);
			socket.on('data', () => (spoke = true));
			const accept = crypto
				.createHash('sha1')
				.update(`${request.headers['sec-websocket-key']}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
				.digest('base64');
			socket.write(
				`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`,
			);
			for (const type of ['connected', 'update', 'full-reload', 'error', 'custom']) {
				const text = Buffer.from(JSON.stringify({ type }));
				socket.write(Buffer.concat([Buffer.from([0x81, text.length]), text]));
			}
		});
		await new Promise((done) => site.listen(0, '127.0.0.1', () => done(undefined)));
		const address = /** @type {import('node:net').AddressInfo} */ (site.address());
		origin = `http://127.0.0.1:${address.port}`;
	});
	afterAll(async () => {
		await browser?.close();
		for (const socket of sockets) socket.destroy();
		await new Promise((done) => site?.close(done));
	});

	beforeEach(() => {
		asked.length = 0;
	});

	it('waits for a request the page makes after the first quiet moment', async () => {
		const page = await browser.newPage();
		expect(await openSettled(page, `${origin}/late`)).toBeUndefined();
		expect(await page.textContent('#pane')).toBe('Arrived');
		await page.close();
	});

	it('waits out the page loading again by itself', async () => {
		const page = await browser.newPage();
		expect(await openSettled(page, `${origin}/reloads`, { limit: 8000 })).toBeUndefined();
		expect(await page.textContent('#state')).toBe('again');
		await page.close();
	});

	it('leaves a screen that sends itself to the path before', async () => {
		const page = await browser.newPage();
		await openSettled(page, `${origin}/tall`);
		expect(await openSettled(page, `${origin}/goes-back`, { limit: 8000 })).toBeUndefined();
		expect(new URL(page.url()).pathname).toBe('/tall');
		await page.close();
	});

	it('sends on a page loaded again at the path before', async () => {
		const page = await browser.newPage();
		await openSettled(page, `${origin}/tall`);
		expect(await openSettled(page, `${origin}/sent-back`, { limit: 8000 })).toBeUndefined();
		expect(new URL(page.url()).pathname).toBe('/sent-back');
		expect(await page.textContent('#where')).toBe('here');
		await page.close();
	});

	it('fails at once on a path that answers with no page', async () => {
		const page = await browser.newPage();
		const started = Date.now();
		await expect(openSettled(page, `${origin}/empty`)).rejects.toThrow(/ERR_ABORTED/);
		expect(Date.now() - started).toBeLessThan(3000);
		await page.close();
	});

	it('says so when the page never stops changing', async () => {
		const page = await browser.newPage();
		expect(await openSettled(page, `${origin}/restless`, { limit: 2000 })).toMatch(
			/still changing after 2s: the page just changed/,
		);
		await page.close();
	});

	it('keeps the resize of a full-page picture from the page', async () => {
		const [width, height] = SIZES['mini-sideways'];
		const context = await browser.newContext({ viewport: { width, height } });
		const page = await context.newPage();
		await page.goto(`${origin}/tall`);
		await page.screenshot({ fullPage: true });
		// Chrome's own way of taking it, which the guard must stop: the page hears it.
		expect(await page.evaluate(() => /** @type {any} */ (window).heard.length)).toBeGreaterThan(0);
		await context.close();

		const held = await browser.newContext({ viewport: { width, height } });
		await holdWindowStill(held);
		const heldPage = await held.newPage();
		await heldPage.goto(`${origin}/tall`);
		await heldPage.screenshot({ fullPage: true });
		const heard = await heldPage.evaluate(() => /** @type {any} */ (window).heard);
		expect(heard.filter((/** @type {string} */ size) => size !== `${width}x${height}`)).toEqual([]);
		await held.close();
	});

	it('keeps the dev server code updates from the page, and passes the rest', async () => {
		const context = await browser.newContext();
		await holdCodeStill(context);
		const page = await context.newPage();
		await page.goto(`${origin}/socket`);
		await expect.poll(() => page.textContent('#heard')).toBe('connected custom ');
		await expect.poll(() => spoke).toBe(true);
		await context.close();
	});

	it('shows its own Session as last used when it began, and opens no recipe', async () => {
		const context = await browser.newContext();
		await leaveNoTrace(context);
		const page = await context.newPage();
		await openSettled(page, `${origin}/sessions`);
		const { sessions } = JSON.parse((await page.textContent('#out')) ?? '').result;
		expect(sessions[0].last_used_at).toBe('2026-10-15T08:00:00Z');
		expect(sessions[1].last_used_at).toBe('2026-11-08T19:00:00Z');
		expect(asked).toEqual(['list_sessions']);
		await context.close();
	});
});
