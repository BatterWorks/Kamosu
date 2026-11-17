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
import { DatabaseSync } from 'node:sqlite';
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
import { coverage, tableFingerprints, tableOf } from './ux-measure.mjs';
import { measureInPage, valuesOf } from './ux-measures.mjs';

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

	it('is refused before any browser starts when it names a Language the interface lacks', async () => {
		await expect(
			signedInArgs(['--session', 'c', '--locale', 'de', 'before', '/recipes'], 'x'),
		).rejects.toThrow(/--locale/);
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
describe('a screen photographed', () => {
	/** @type {import('playwright').Browser} */
	let browser;
	/** @type {import('node:http').Server} */
	let site;
	/** Whether a page said anything over the stand-in's WebSocket. */
	let spoke = false;
	/** The stand-in's WebSockets, ended with it. */
	/** @type {import('node:stream').Duplex[]} */
	const sockets = [];
	/** The answers the stand-in never gives, ended with it. */
	/** @type {import('node:http').ServerResponse[]} */
	const held = [];
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
		// Asks for more code while the next page is on its way, as the recipe
		// list preloaded a recipe's page as the cooking screen replaced it
		// (#635). The answer never comes, and once the next page is in,
		// nothing says how the request ended.
		'/leaves-behind': `<p>leaving</p><script>
			addEventListener('beforeunload', () => setTimeout(() => {
				const link = document.createElement('link');
				link.rel = 'modulepreload';
				link.href = '/never.js';
				document.head.append(link);
			}, 100));
		</script>`,
		// Starts loading a page that never comes, then changes its own address
		// while a slow answer is on its way, which is still the screen's to
		// wait for.
		'/stays': `<p id="pane">Loading</p><script>
			setTimeout(() => {
				location.href = '/empty';
				setTimeout(async () => {
					const answer = fetch('/api/op/slower', { method: 'POST' });
					history.pushState(null, '', '/stays-on');
					document.getElementById('pane').textContent = await (await answer).text();
				}, 300);
			}, 100);
		</script>`,
		// Answered late, so the page before it has time to ask for more.
		'/arrives-late': '<p>arrived</p>',
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
				} else if (name === 'slower') {
					setTimeout(() => response.end('Arrived'), 2500);
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
			if (url === '/never.js') {
				held.push(response);
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
			if (url === '/arrives-late') {
				setTimeout(() => response.end(PAGES[url]), 400);
				return;
			}
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
		for (const response of held) response.destroy();
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

	it('is not held by what the page before asked for as it left', async () => {
		// Routed as the tools' own contexts are, which changes how Chrome
		// reports each request to Playwright.
		const context = await browser.newContext();
		await leaveNoTrace(context);
		const page = await context.newPage();
		const before = held.length;
		await openSettled(page, `${origin}/leaves-behind`);
		expect(await openSettled(page, `${origin}/arrives-late`, { limit: 5000 })).toBeUndefined();
		expect(held.length).toBeGreaterThan(before);
		await context.close();
	});

	it('waits for its own request after a page that never came', async () => {
		const page = await browser.newPage();
		expect(await openSettled(page, `${origin}/stays`, { limit: 8000 })).toBeUndefined();
		expect(await page.textContent('#pane')).toBe('Arrived');
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

describe('what a screen was answered', () => {
	it('is every string and number of each answer, a refusal’s words included, whitespace made single', () => {
		const values = valuesOf({
			ok: true,
			result: { title: ' Tarte  tatin', yield: 4, tags: ['Sweet'] },
		});
		valuesOf({ ok: false, error: { message: 'No such recipe' } }, values);
		expect([...values].sort()).toEqual(['4', 'No such recipe', 'Sweet', 'Tarte tatin']);
	});
});

describe('a screen measured', () => {
	/** @type {import('playwright').Browser} */
	let browser;
	beforeAll(async () => {
		const env = fs.existsSync(path.join(ROOT, '.env')) ? devEnv() : {};
		browser = await chromium.launch({ executablePath: chromePath(env) });
	});
	afterAll(async () => {
		await browser?.close();
	});

	/**
	 * Measure a page as `just ux-measure` does, in a real Chrome.
	 * @param {string} html
	 * @param {{ values?: string[], probe?: boolean, breakage?: boolean, size?: number[] }} [options]
	 */
	async function measure(
		html,
		{ values = [], probe = false, breakage = false, size = [390, 844] } = {},
	) {
		const page = await browser.newPage({ viewport: { width: size[0], height: size[1] } });
		await page.setContent(html);
		const result = await page.evaluate(measureInPage, {
			values,
			accents: ['#1d2b4c'],
			probe,
			breakage,
		});
		await page.close();
		return result;
	}

	it('counts a word only when it holds a letter or a digit', async () => {
		const { words } = await measure('<p>Add 3 eggs — then stir · ok</p>');
		expect(words.interfacePage).toBe(6);
	});

	it('splits sentences after a full stop, a question mark, an exclamation mark or an ellipsis', async () => {
		const { words } = await measure('<p>Saved. Is it right? Yes! Wait… and then</p>');
		expect(words.sentences).toEqual(['Saved.', 'Is it right?', 'Yes!', 'Wait…', 'and then']);
	});

	it('takes a text for content on an exact match, or three words inside one with nothing touching', async () => {
		const { words } = await measure(
			['Plan', 'Plant', 'fritters with lime', 'ake the cake', 'the cake', 'Bake the cake']
				.map((t) => `<p>${t}</p>`)
				.join(''),
			{ values: ['Plantain fritters with lime', 'Plan', 'Bake the cake until golden'] },
		);
		expect(words.interfaceTexts).toEqual(['Plant', 'ake the cake', 'the cake']);
	});

	it('ends a sentence where content comes between, so a title does not join two', async () => {
		const { words } = await measure(
			'<p>Cook <b>Tarte tatin</b> for the dinner with friends and family next week.</p>',
			{ values: ['Tarte tatin'] },
		);
		expect(words.sentences).toEqual(['Cook', 'for the dinner with friends and family next week.']);
	});

	it('counts a placeholder’s size and colour with the rest of the text', async () => {
		const { type } = await measure(
			'<style>input::placeholder { color: rgb(200, 0, 0) }</style><p style="font-size: 16px; color: rgb(0, 0, 0)">Seen</p><input placeholder="Search" style="font-size: 20px">',
		);
		expect(type.sizes).toEqual([16, 20]);
		expect(type.colours).toHaveLength(2);
	});

	it('counts no word nobody can see: a box of one pixel, or nothing drawn', async () => {
		const { words } = await measure(
			'<p>Seen</p><span style="position: absolute; width: 1px; height: 1px">Hidden one pixel</span><p style="opacity: 0">Hidden clear</p>',
		);
		expect(words.interfaceTexts).toEqual(['Seen']);
	});

	/** A phone-sized page holding what each measure looks for. */
	const SCREEN = `<!doctype html><html><head><style>
		body { margin: 0 16px; background: #faf7f0; color: #222; font: 16px/1.4 sans-serif; }
		header { height: 60px; display: flex; align-items: center; }
		nav { position: fixed; bottom: 0; left: 0; right: 0; margin: 0; height: 64px; background: #fff; display: flex; }
		nav a { flex: 1; height: 64px; display: flex; align-items: center; justify-content: center; color: #222; text-decoration: none; }
		.small { width: 24px; height: 24px; padding: 0; background: none; border: 0; position: relative; color: #222; font: inherit; }
		.grown::after { content: ''; position: absolute; inset: -12px; }
		.main { height: 48px; width: 120px; background: #1d2b4c; color: #fff; border: 0; }
		.notice { border: 1px solid #c33; padding: 8px; }
		.apart { margin: 40px 0; display: flex; gap: 40px; }
		.close { display: flex; gap: 4px; margin-top: 40px; }
	</style></head><body>
		<header><button class="small" aria-label="Menu">≡</button></header>
		<main>
			<h1>Tarte tatin</h1>
			<p>This screen shows everything you saved for later today. Plan</p>
			<p>Bake the apples until golden, then turn the tart over.</p>
			<button class="main">Cook</button>
			<div role="status" class="notice">Saved</div>
			<div class="apart">
				<button class="small grown" aria-label="Grown">+</button>
				<button class="small" aria-label="Tiny">x</button>
			</div>
			<div class="close">
				<button class="small grown" aria-label="Left">a</button>
				<button class="small grown" aria-label="Right">b</button>
			</div>
		</main>
		<nav><a href="/">Home</a><a href="/plan">Plan</a></nav>
	</body></html>`;
	const VALUES = ['Tarte tatin', 'Bake the apples until golden, then turn the tart over.'];

	it('tells the words a page was answered from the interface’s own', async () => {
		const { words } = await measure(SCREEN, { values: VALUES });
		expect(words.contentPage).toBe(2 + 10);
		expect(words.explanationsFirst).toEqual([
			'This screen shows everything you saved for later today.',
		]);
		// The same sentence ending in "Plan", then Cook, Saved, x, a, b, Home and Plan.
		expect(words.interfacePage).toBe(10 + 7);
		expect(words.interfaceTexts).toContain('Cook');
		expect(words.interfaceTexts).not.toContain('Tarte tatin');
	});

	it('counts the room bars and notice cards take, once where they overlap, and the main buttons', async () => {
		const result = await measure(SCREEN, { values: VALUES });
		expect(result.notices).toBe(1);
		const notice = result.room - (60 + 64) / 844;
		expect(notice).toBeGreaterThan(30 / 844);
		expect(notice).toBeLessThan(50 / 844);
		expect(result.controls.mainButtons).toEqual(['Cook']);
		// A notice pinned over the tab bar covers no room the bar has not.
		const doubled = await measure(
			SCREEN.replace(
				'.notice {',
				'.notice { position: fixed; bottom: 0; left: 0; right: 0; height: 40px;',
			),
			{ values: VALUES },
		);
		expect(doubled.room).toBe(Math.round(((60 + 64) / 844) * 1000) / 1000);
	});

	it('calls a plain action outside a bar unmarked, and one inside a bar not', async () => {
		const { unmarked } = await measure(SCREEN, { values: VALUES });
		expect(unmarked).toContain('Tiny');
		expect(unmarked).not.toContain('Menu');
		expect(unmarked).not.toContain('Home');
	});

	it('reports a small button by what a finger hits, so one grown to 48 is not small', async () => {
		const { tap, gaps } = await measure(SCREEN, { values: VALUES, probe: true });
		const small = tap.small.map((/** @type {{ label: string }} */ s) => s.label);
		expect(small).toContain('Tiny');
		expect(small).not.toContain('Grown');
		expect(small).not.toContain('Cook');
		// Right's layer lies over Left's, so a finger at the overlap taps Right:
		// Left keeps 12 + 24 + 4 - 12 across, and the two layers overlap.
		const left = tap.small.find((/** @type {{ label: string }} */ s) => s.label === 'Left');
		expect(left?.width).toBe(28);
		expect(small).not.toContain('Right');
		expect(gaps.overlapping).toEqual([['Left', 'Right']]);
		expect(gaps.smallest).toEqual({ px: 4, between: ['Left', 'Right'] });
	});

	it('walks a long list’s tapped areas in a few seconds (#638)', async () => {
		// Chrome hit-tests the whole page for every point the walk asks, and a
		// list of boxes that each clip what they hold made every point dear:
		// the recipe list took 20 seconds, and 150 rows like these over 15.
		// They now take under two, so the bound leaves room for a busy machine.
		const row = (/** @type {number} */ i) =>
			`<li style="position: relative; overflow: hidden; height: 96px"><a href="/r/${i}" style="display: flex; height: 96px; position: relative; overflow: hidden"><span style="position: absolute; inset: 0; overflow: hidden"><span style="display: block; width: 80px; height: 80px; background: #ccc"></span></span><span style="margin-left: 96px">Recipe ${i}</span></a></li>`;
		const list = `<body style="margin: 0"><ul style="margin: 0; padding: 0; list-style: none">${Array.from({ length: 150 }, (_, i) => row(i)).join('')}</ul></body>`;
		const started = Date.now();
		const { tap, gaps } = await measure(list, { probe: true, size: [402, 874] });
		expect(Date.now() - started).toBeLessThan(6000);
		expect(tap.small).toEqual([]);
		expect(tap.covered).toEqual([]);
		expect(gaps.overlapping).toEqual([]);
	});

	it('names what covers a control by all its words, those lying far down the page too', async () => {
		const { tap } = await measure(
			`<body style="margin: 0; height: 5000px">
			<button style="position: absolute; top: 100px; left: 100px; width: 60px; height: 60px">Hidden</button>
			<div style="position: absolute; top: 0; left: 0; width: 300px; height: 300px">
				<p style="position: absolute; top: 4000px; margin: 0">Far</p><p style="margin: 0">Near</p>
			</div>
		</body>`,
			{ probe: true },
		);
		expect(tap.covered).toEqual([{ label: 'Hidden', by: 'Far Near' }]);
	});

	it('at 360, names text cut off, boxes drawn over each other and a sideways scroll', async () => {
		const result = await measure(
			`<body style="margin: 0 16px">
			<p style="width: 100px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis">A title far too long for its box</p>
			<div style="width: 80px; overflow: hidden; white-space: nowrap"><span>Clipped by its parent box</span></div>
			<p style="width: 100px">Fits</p>
			<div style="position: relative; height: 40px">
				<span>Under</span><button style="position: absolute; left: 0; top: 0">Over</button>
			</div>
			<div style="width: 400px">Wide</div>
			<nav style="position: fixed; bottom: 0; left: 0; right: 0; height: 60px; background: #fff">
				<a href="/" style="position: absolute; left: 0">Home tab</a><a href="/plan" style="position: absolute; left: 20px">Plan tab</a>
			</nav>
		</body>`,
			{ breakage: true, size: [360, 760] },
		);
		expect(result.cutOff).toEqual([
			'A title far too long for its box',
			'Clipped by its parent box',
		]);
		expect(result.overlaps).toEqual([
			['Under', 'Over'],
			['Home tab', 'Plan tab'],
		]);
		expect(result.sideways).toBe(16 + 400 - 360);
	});
});

describe('what ux-measure covers', () => {
	const found = {
		shortRecipe: 'b_short',
		longRecipe: 'b_long',
		shortFood: 'f_short',
		longFood: 'f_long',
		search: 'tart',
		oldVersion: 'v_old',
		importJob: 'j_1',
		secrets: {
			invite: '/invite/s1',
			kitchen: '/kitchen-invite/s2',
			cookbook: '/cookbook-invite/s3',
			recovery: '/recover/s4',
		},
	};
	const rows = coverage(found);
	/** @param {(row: typeof rows[number]) => boolean} which */
	const keys = (which) =>
		new Set(rows.filter(which).map((r) => `${r.screen} ${r.size} ${r.locale}`));

	it('measures each everyday screen at the four sizes in the four languages', () => {
		const everyday = rows.filter((r) => r.group === 'everyday' && !r.breakage);
		const screens = new Set(everyday.map((r) => r.screen));
		expect([...screens].sort()).toEqual(
			[
				'home',
				'recipes',
				'search',
				'recipe-short',
				'recipe-long',
				'share',
				'plus-menu',
				'cooking',
				'shopping',
				'plan',
				'food-short',
				'food-long',
			].sort(),
		);
		expect(everyday).toHaveLength(12 * 4 * 4);
		expect(new Set(everyday.map((r) => r.size))).toEqual(
			new Set(['phone-402', 'phone-412', 'ipad-upright', 'macbook']),
		);
		expect(everyday.find((r) => r.screen === 'search')).toMatchObject({
			path: '/recipes',
			open: 'search',
			typed: 'tart',
		});
		expect(everyday.find((r) => r.screen === 'share')?.path).toBe('/recipes/b_short/share');
	});

	it('opens every size as a touch screen but the MacBook, so a phone hides what only a mouse sees', () => {
		expect(new Set(rows.filter((r) => !r.touch).map((r) => r.size))).toEqual(new Set(['macbook']));
		expect(rows.filter((r) => r.size === 'macbook').every((r) => !r.touch)).toBe(true);
	});

	it('photographs in English and French only', () => {
		expect(new Set(rows.filter((r) => r.picture).map((r) => r.locale))).toEqual(
			new Set(['en', 'fr']),
		);
		expect(rows.filter((r) => !r.picture).every((r) => !['en', 'fr'].includes(r.locale))).toBe(
			true,
		);
	});

	it('measures Settings and the operator pages at the two phones in English, and every page but /tokens', () => {
		const settings = rows.filter((r) => r.group === 'settings' && !r.breakage);
		expect(new Set(settings.map((r) => r.size))).toEqual(new Set(['phone-402', 'phone-412']));
		expect(new Set(settings.map((r) => r.locale))).toEqual(new Set(['en']));
		const paths = new Set(rows.map((r) => r.path));
		for (const route of [
			'/imports/from/mealie',
			'/recipes/b_long/versions/v_old',
			'/imports/j_1',
			'/settings',
			'/settings/account',
			'/settings/tags',
			'/operator',
			'/foods',
			'/about',
		]) {
			expect(paths).toContain(route);
		}
		expect(paths).not.toContain('/tokens');
	});

	it('opens the four secret-link pages signed out, on the empty instance', () => {
		const secret = rows.filter((r) => r.group === 'secret' && !r.breakage);
		expect(new Set(secret.map((r) => r.path))).toEqual(new Set(Object.values(found.secrets)));
		expect(secret.every((r) => r.instance === 'empty' && r.signedOut)).toBe(true);
	});

	it('checks every screen for breakage at 360 px in English and French', () => {
		const measured = keys((r) => !r.breakage);
		const screens = new Set([...measured].map((k) => k.split(' ')[0]));
		const broken = rows.filter((r) => r.breakage);
		expect(broken.every((r) => r.size === 'narrowest')).toBe(true);
		for (const screen of screens) {
			for (const locale of ['en', 'fr']) {
				expect(broken.some((r) => r.screen === screen && r.locale === locale)).toBe(true);
			}
		}
	});

	it('leaves the nine sizes just screens photographs as they were', () => {
		expect(Object.keys(SIZES)).toHaveLength(9);
		expect(SIZES.phone).toEqual([390, 844]);
	});
});

describe('the dev database ux-measure watches', () => {
	it('sees a changed row, and not the last-used stamps the dev server writes by itself', () => {
		const file = path.join(tmp, 'watched.db');
		const db = new DatabaseSync(file);
		db.exec(
			"CREATE TABLE sessions (id TEXT, name TEXT, last_used_at TEXT); INSERT INTO sessions VALUES ('s1', 'phone', '1')",
		);
		const first = tableFingerprints(file);
		db.exec("UPDATE sessions SET last_used_at = '2'");
		expect(tableFingerprints(file)).toEqual(first);
		db.exec("UPDATE sessions SET name = 'laptop'");
		expect(tableFingerprints(file)).not.toEqual(first);
		db.close();
	});
});

describe('the table ux-measure writes', () => {
	it('gives a measured row its counts and a 360 row its breakage, each blank where it was not counted', () => {
		const row = {
			screen: 'home',
			path: '/',
			instance: 'library',
			size: 'phone-402',
			locale: 'en',
			breakage: false,
		};
		const measures = {
			words: {
				interfaceFirst: 12,
				interfacePage: 40,
				contentFirst: 3,
				contentPage: 9,
				explanationsFirst: [],
				explanationsPage: ['A b c d e f.'],
				long: [],
			},
			room: 0.19,
			controls: { first: 14, mainButtons: ['Cook'] },
			unmarked: [],
			tap: { small: [], cutShort: [], covered: [], inSentence: [] },
			gaps: { overlapping: [], smallest: { px: 8, between: ['a', 'b'] } },
			type: { sizes: [14, 16, 26], colours: ['1,2,3,1'] },
			length: { screenfuls: 2.5, mainActionAt: 0.4 },
			notices: 0,
		};
		const table = tableOf([
			{ ...row, measures, picture: 'pictures/en/home.phone-402.png' },
			{
				...row,
				size: 'narrowest',
				breakage: true,
				broken: { cutOff: ['Long'], overlaps: [], sideways: 0 },
			},
		]).split('\n');
		const header = table[0].split(',');
		const cells = (/** @type {string} */ line) =>
			Object.fromEntries(line.split(',').map((v, i) => [header[i], v]));
		expect(cells(table[1])).toMatchObject({
			interface_words_first: '12',
			room_percent: '19',
			main_buttons_first: '1',
			text_cut_off: '',
		});
		expect(cells(table[2])).toMatchObject({
			interface_words_first: '',
			text_cut_off: '1',
			sideways_px: '0',
		});
	});
});
