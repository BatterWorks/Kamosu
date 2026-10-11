/**
 * `just proto-serve <folder> <port>` (#216): serves a prototype folder to the
 * LAN and tells the browser to cache nothing, so a phone always shows the
 * latest build of a page.
 *
 * `<folder>` is a prototype's name under `.dev/prototypes/`. The port is the
 * caller's to choose, because which ports a host lets through its firewall is
 * that host's business.
 */

import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { PROTOTYPES_DIR, ROOT, ToolError, run } from './dev.mjs';

/** What a browser is told each kind of file is. */
const TYPES = {
	'.html': 'text/html; charset=utf-8',
	'.css': 'text/css; charset=utf-8',
	'.js': 'text/javascript; charset=utf-8',
	'.mjs': 'text/javascript; charset=utf-8',
	'.json': 'application/json',
	'.svg': 'image/svg+xml',
	'.png': 'image/png',
	'.jpg': 'image/jpeg',
	'.jpeg': 'image/jpeg',
	'.webp': 'image/webp',
	'.ico': 'image/x-icon',
	'.woff2': 'font/woff2',
	'.ttf': 'font/ttf',
	'.txt': 'text/plain; charset=utf-8',
};

/**
 * The folder a prototype's name means. Only a prototype is served: this
 * answers the whole LAN, and the repo root holds `.env` and every cookie file.
 * @param {string} name
 * @param {string} [prototypes]
 */
export function folderOf(name, prototypes = PROTOTYPES_DIR) {
	const found = path.join(prototypes, name);
	if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name) || !fs.existsSync(found)) {
		throw new ToolError(`there is no prototype named ${name} under .dev/prototypes/`);
	}
	return found;
}

/**
 * The file an address asks for, or nothing when it is not inside the folder.
 * @param {string} folder
 * @param {string} url
 */
export function fileFor(folder, url) {
	let wanted;
	try {
		wanted = decodeURIComponent(new URL(url, 'http://prototype').pathname);
	} catch {
		return undefined;
	}
	let file = path.join(folder, wanted);
	if (file !== folder && !file.startsWith(folder + path.sep)) return undefined;
	if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, 'index.html');
	return fs.existsSync(file) ? file : undefined;
}

/**
 * A server for one folder. Every answer says not to keep it.
 * @param {string} folder
 */
export function server(folder) {
	return http.createServer((request, response) => {
		const file = fileFor(folder, request.url ?? '/');
		response.setHeader('Cache-Control', 'no-store');
		if (!file) {
			response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
			response.end('Nothing here.\n');
			return;
		}
		const type =
			TYPES[/** @type {keyof typeof TYPES} */ (path.extname(file).toLowerCase())] ??
			'application/octet-stream';
		response.writeHead(200, { 'Content-Type': type });
		fs.createReadStream(file).pipe(response);
	});
}

/** This machine's address on the LAN, which is the one another device can open. */
function lanAddress() {
	for (const addresses of Object.values(os.networkInterfaces())) {
		for (const address of addresses ?? []) {
			if (address.family === 'IPv4' && !address.internal) return address.address;
		}
	}
	return 'localhost';
}

if (process.argv[1] === import.meta.filename) {
	await run(async () => {
		const [, , name, portText] = process.argv;
		const port = Number(portText);
		if (!name || !Number.isInteger(port) || port < 1 || port > 65535) {
			throw new ToolError('usage: just proto-serve <folder> <port>');
		}
		const folder = folderOf(name);
		await new Promise((resolve, reject) => {
			server(folder)
				.once('error', (/** @type {NodeJS.ErrnoException} */ error) =>
					reject(
						error.code === 'EADDRINUSE'
							? new ToolError(`port ${port} is taken; choose another`)
							: error,
					),
				)
				.listen(port, '0.0.0.0', () => {
					console.log(`serving ${path.relative(ROOT, folder)} with caching off`);
					console.log(`http://${lanAddress()}:${port}/`);
					resolve(undefined);
				});
		});
	});
}
