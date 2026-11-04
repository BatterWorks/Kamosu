import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vitest/config';
import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { loadEnv, type Plugin } from 'vite';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

/// The backend, in development: two processes side by side (ADR 0028). Vite
/// serves the interface and proxies everything the binary owns to it, so a
/// browser only ever talks to one origin and no CORS question exists.
const BACKEND = 'http://127.0.0.1:5266';

/// The hostname a TLS test proxy serves the dev interface under, read from
/// `KAMOSU_DEV_HOST` in the repo-root `.env` (gitignored, AGENTS.local.md) so
/// no machine's own address is written into the repository. Unset, vite
/// accepts only its defaults, localhost and IP addresses.
const DEV_HOST = loadEnv('development', '..', 'KAMOSU_DEV_HOST').KAMOSU_DEV_HOST;

/// Every plugin project.inlang/settings.json names, loaded before Paraglide
/// compiles, and a stop if one does not load. Paraglide only warns when a
/// plugin fails: without the one that reads messages/, every phrase is
/// undefined, and a build would ship an interface with no words in it (#532).
/// A plugin is a file in node_modules, so a lockfile version `just audit`
/// reads. A URL is refused, since that would be whatever a CDN serves.
function inlangPluginsLoad(): Plugin {
	return {
		name: 'kamosu:inlang-plugins-load',
		async buildStart() {
			const settings = JSON.parse(
				await readFile(resolve(import.meta.dirname, 'project.inlang/settings.json'), 'utf8'),
			);
			for (const module of settings.modules ?? []) {
				if (/^https?:/.test(module)) {
					throw new Error(
						`project.inlang/settings.json names a plugin by URL, ${module}. Add it with npm install and name its file in node_modules.`,
					);
				}
				let key;
				try {
					({ key } =
						(await import(pathToFileURL(resolve(import.meta.dirname, module)).href)).default ?? {});
				} catch (error) {
					throw new Error(`Paraglide's plugin ${module} did not load: run npm ci in ui/.`, {
						cause: error,
					});
				}
				if (typeof key !== 'string') {
					throw new Error(`Paraglide's plugin ${module} is not an inlang plugin.`);
				}
			}
		},
	};
}

export default defineConfig(({ command }) => ({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
			},

			// adapter-static with a fallback: SvelteKit's server half is unused
			// (ADR 0012). The Rust binary serves `build/` — one artefact, one version.
			adapter: adapter({ fallback: 'index.html', strict: false }),

			// SvelteKit 3 would ask the server for a new version every hour and
			// set `updated.current`. Kamosu reads `updated` nowhere: the service
			// worker already takes up a new build, and the page reloads on its
			// `controllerchange`. Off, so the interface makes no request it did
			// not make before (#525).
			version: { pollInterval: 0 },
		}),

		// Paraglide compiles every phrase into a function, so a misspelt key is a
		// build error rather than a blank button in a kitchen (ADR 0012). The
		// locale is a setting, not a URL segment: no `/fr/…` routes exist, which
		// is what lets the whole app be one prerendered shell.
		//
		// What it compiles is in project.inlang/paraglide.config.js, which
		// `npm run check` reads too. Its plugins come from node_modules (#532).
		inlangPluginsLoad(),
		paraglideVitePlugin({
			project: './project.inlang',
			// Served (the dev server and the screen tests), one module per
			// language rather than one per phrase. Paraglide 3 serves every
			// phrase as a module of its own, near 300 requests a screen, and the
			// dev instance's TLS proxy dropped some of them, which broke `just
			// screens` (#525). Paraglide 2 did the same, and so does what came
			// with it: a build writes the other layout into the same folder, so
			// a running dev server needs `just dev-start` again after one. A
			// build keeps Paraglide 3's own default.
			...(command === 'serve' && { outputStructure: 'locale-modules' }),
		}),
	],

	server: {
		// Fixed here, not picked per session: vite's own default 5173 is taken on
		// the dev host. `strictPort` makes a silent fallback to another port
		// impossible — a wrong port must fail loudly, not quietly work elsewhere.
		port: 5174,
		strictPort: true,
		host: '0.0.0.0',
		// The TLS test proxy serves this exact hostname to a browser. Keep the
		// allowlist narrow: accepting arbitrary Host headers enables DNS rebinding.
		allowedHosts: DEV_HOST ? [DEV_HOST] : [],
		// Everything the binary owns, proxied so dev and production serve the same
		// URLs. `/favicon.svg` is named by app.html; the two root PNGs are asked
		// for by Safari's own convention. All three would 404 in dev otherwise.
		//
		// `^/s/` is the Share Link page (#65), which the server renders itself —
		// and it is a REGULAR EXPRESSION rather than a prefix on purpose. Vite
		// matches a plain string as a prefix, so a bare `/s` would swallow
		// `/settings`, `/shopping` and `/share` and hand three of the app's own
		// screens to the binary. Anchoring the trailing slash keeps it to the one
		// path the binary actually owns. `^/calendar/` is a Calendar Link's file
		// (#491), anchored the same way.
		proxy: Object.fromEntries(
			[
				'/api',
				'/auth',
				'/assets',
				'/favicon.svg',
				'/favicon-32.png',
				'/apple-touch-icon.png',
				'^/s/',
				'^/calendar/',
			].map((path) => [path, { target: BACKEND, changeOrigin: false }]),
		),
	},

	build: {
		rolldownOptions: {
			// A phrase imported from Paraglide's messages that will always be
			// undefined is one Paraglide did not compile, for whatever reason:
			// Paraglide only warns, and the build would ship an interface
			// with no words in it, so here the first one stops the build (#532).
			// Vite logs an error handed to it and carries on, so this throws.
			onLog(level, log, handler) {
				if (log.code === 'IMPORT_IS_UNDEFINED' && log.message.includes('virtual:paraglide/')) {
					throw new Error(
						`${log.message}\nA phrase that does not compile leaves the interface without words. Paraglide's warnings are above.`,
					);
				}
				handler(level, log);
			},
		},
	},

	test: {
		expect: { requireAssertions: true },
		environment: 'jsdom',
		setupFiles: ['./src/testing/setup.ts'],
		// One jsdom per worker rather than per file: building it 34 times was 43 of
		// the screen tests' 48 seconds (#122). What a shared environment lets one
		// file leave for the next, `setup.ts` puts back at the top of every file.
		isolate: false,
		// The dev tools' own rules are tested beside them, outside src/ (#216).
		include: ['src/**/*.{test,spec}.{js,ts}', 'tools/**/*.test.js'],
	},

	// Screens are exercised as a browser would run them, so the browser build of
	// Svelte is what the tests resolve — the server build cannot mount anything.
	resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
}));
