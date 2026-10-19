/**
 * The dev tools' own rules (#216): the parts that decide a name, a refusal or
 * a difference, tested without a browser or a dev instance. What they do
 * against a real screen is checked live, through the `just` recipes.
 */

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ToolError, chromePath, readEnv, sessionFile, signedInArgs, slugOf } from './dev.mjs';
import { secretFrom } from './dev-session.mjs';
import { fileFor, folderOf, server } from './proto-serve.mjs';
import { indexOf, pageOf, photoName } from './proto-new.mjs';
import { pictureName } from './screens.mjs';
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
