// `just check` and `just test` run through this (#616). It decides how much
// of each to run and says so on its first line:
//
// - **full**: everything, exactly as before. Any change outside the
//   documentation, and every run `just release-prepare` makes.
// - **docs-only**: every path the branch changed since its merge base on `dev`
//   is documentation, so nothing is built. `check` has nothing to look at,
//   since none of its checks reads documentation; `test` runs the tests that
//   do.
// - **remembered pass**: this recipe already passed on these exact contents,
//   in this mode or in full, and the worktree is clean. Nothing runs.
//
// A pass is recorded under the Cargo target folder, never in Git, keyed by the
// recipe, the hash of the working tree's contents and the mode. The hash is of
// the files as they stand, committed or not, so a run that passed before a
// commit is remembered after it. Only a clean worktree is answered from the
// record: "clean" is the condition the maintainer set, so a dirty one always
// runs.

import { spawnSync } from 'node:child_process';
import {
	copyFileSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readdirSync,
	rmSync,
	statSync,
	writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const RECIPES = ['check', 'test'];

// What counts as documentation: nothing under it is built, and no test reads
// it but the ones in DOCS_READING_TESTS. The root files are named one by one,
// because a root Markdown file can be read by a test: tests/defences.rs reads
// SECURITY.md, so SECURITY.md is not on this list. A file missing here only
// costs a full run.
const DOCUMENTATION_FOLDER = 'docs/';
const ROOT_DOCUMENTS = new Set(['CHANGELOG.md', 'README.md', 'AGENTS.md', 'GLOSSARY.md']);

// The tests that read documentation, which `just test` runs in docs-only mode.
// The Docker Hub page test reads docs/docker-hub/ and docs/runner.md. Searched
// on 2026-11-16: nothing else under src/, tests/, build.rs or ui/ reads docs/
// or the root files above. A new test that reads one belongs here.
export const DOCS_READING_TESTS = ['ui/tools/docker-hub-page.test.mjs'];

// Where the pass records live, inside the Cargo target folder. `just
// dev-clean` deletes only the profile folders, so it keeps them.
const RECORD_FOLDER = 'kamosu-passes';

// Records older than this are swept when a new one is written.
const RECORD_LIFETIME_MS = 30 * 24 * 60 * 60 * 1000;

// The branch every piece of work is cut from (AGENTS.md, "Branching").
const INTEGRATION_BRANCHES = ['origin/dev', 'dev'];

export function isDocumentation(file) {
	return file.startsWith(DOCUMENTATION_FOLDER) || ROOT_DOCUMENTS.has(file);
}

// Docs-only when something changed and every change is documentation. A
// branch with no change at all runs in full: there is nothing to narrow it to.
export function modeFor(changedPaths) {
	return changedPaths.length > 0 && changedPaths.every(isDocumentation) ? 'docs-only' : 'full';
}

function git(root, args, env = {}) {
	const result = spawnSync('git', args, {
		cwd: root,
		encoding: 'utf8',
		env: { ...process.env, ...env },
	});
	if (result.error) {
		throw result.error;
	}
	return result;
}

function gitOut(root, args, env) {
	const result = git(root, args, env);
	if (result.status !== 0) {
		throw new Error(`git ${args.join(' ')} failed: ${result.stderr.trim()}`);
	}
	return result.stdout;
}

const nulSeparated = (text) => text.split('\0').filter((entry) => entry !== '');

// The merge base with the integration branch, or null when there is none to
// compare with, which makes the run full.
export function mergeBase(root) {
	for (const branch of INTEGRATION_BRANCHES) {
		if (git(root, ['rev-parse', '--verify', '--quiet', `${branch}^{commit}`]).status !== 0) {
			continue;
		}
		const result = git(root, ['merge-base', 'HEAD', branch]);
		if (result.status === 0) {
			return result.stdout.trim();
		}
	}
	return null;
}

// Every path that differs from the merge base: committed, staged, unstaged or
// untracked. A rename counts as both of its paths.
export function changedPaths(root, base) {
	const tracked = nulSeparated(gitOut(root, ['diff', '--name-only', '--no-renames', '-z', base]));
	const untracked = nulSeparated(
		gitOut(root, ['ls-files', '--others', '--exclude-standard', '-z']),
	);
	return [...new Set([...tracked, ...untracked])];
}

// The hash of the working tree as Git would commit it after `git add -A`,
// worked out in a throwaway index so the real one is left alone.
export function workingTreeHash(root) {
	const scratch = mkdtempSync(path.join(tmpdir(), 'kamosu-gate-'));
	try {
		const index = path.join(scratch, 'index');
		const real = path.resolve(root, gitOut(root, ['rev-parse', '--git-path', 'index']).trim());
		// Starting from a copy of the real index lets Git skip every file whose
		// stat has not changed. A repository with no index yet starts empty.
		if (existsSync(real)) {
			copyFileSync(real, index);
		}
		const env = { GIT_INDEX_FILE: index };
		gitOut(root, ['add', '--all', '--', ':/'], env);
		return gitOut(root, ['write-tree'], env).trim();
	} finally {
		rmSync(scratch, { recursive: true, force: true });
	}
}

export function isClean(root) {
	return gitOut(root, ['status', '--porcelain', '--untracked-files=all']).trim() === '';
}

export function recordFolder(root, env = process.env) {
	return path.resolve(root, env.CARGO_TARGET_DIR || 'target', RECORD_FOLDER);
}

const recordName = (recipe, tree, mode) => `${recipe}-${tree}-${mode}`;

// The earlier pass that answers this run, or null. A full pass answers a
// docs-only run too; a docs-only pass never answers a full one.
export function rememberedPass(folder, recipe, tree, mode) {
	const answering = mode === 'full' ? ['full'] : [mode, 'full'];
	for (const candidate of answering) {
		const file = path.join(folder, recordName(recipe, tree, candidate));
		try {
			return { mode: candidate, at: statSync(file).mtime };
		} catch {
			// Not recorded in that mode.
		}
	}
	return null;
}

export function recordPass(folder, recipe, tree, mode, now = Date.now()) {
	mkdirSync(folder, { recursive: true });
	writeFileSync(
		path.join(folder, recordName(recipe, tree, mode)),
		`${new Date(now).toISOString()}\n`,
	);
	for (const name of readdirSync(folder)) {
		const file = path.join(folder, name);
		try {
			if (now - statSync(file).mtimeMs > RECORD_LIFETIME_MS) {
				rmSync(file, { force: true });
			}
		} catch {
			// Swept by another run at the same moment.
		}
	}
}

// What each recipe runs in each mode, as commands from the repository root.
// No check in `_check-full` reads documentation (formatters, lints and
// freshness gates all look at code), so a docs-only `check` runs nothing.
function stepsFor(recipe, mode) {
	if (mode === 'full') {
		return [['just', [`_${recipe}-full`]]];
	}
	return recipe === 'check' ? [] : [['node', ['--test', ...DOCS_READING_TESTS]]];
}

function runStep(root, [command, args]) {
	const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
	if (result.error) {
		throw result.error;
	}
	return result.status ?? 1;
}

// One run of a recipe. Returns its exit status. `run` and `log` are seams for
// the tests; everything else is read from the repository at `root`.
export function gate({
	root,
	recipe,
	env = process.env,
	run = (step) => runStep(root, step),
	log = (line) => console.log(line),
}) {
	if (!RECIPES.includes(recipe)) {
		throw new Error(`gate: unknown recipe "${recipe}", expected one of ${RECIPES.join(', ')}`);
	}
	const asked = env.KAMOSU_CHECK_MODE ?? '';
	if (asked !== '' && asked !== 'full') {
		throw new Error(`gate: KAMOSU_CHECK_MODE is "${asked}", and only "full" is understood`);
	}
	const forced = asked === 'full';
	const base = forced ? null : mergeBase(root);
	const mode = base === null ? 'full' : modeFor(changedPaths(root, base));
	const tree = workingTreeHash(root);
	const folder = recordFolder(root, env);

	if (!forced && isClean(root)) {
		const earlier = rememberedPass(folder, recipe, tree, mode);
		if (earlier) {
			log(
				`just ${recipe}: remembered pass. These exact contents passed in ${earlier.mode} mode at ${earlier.at.toISOString()}, and the worktree is clean, so nothing runs.`,
			);
			return 0;
		}
	}

	const why = forced
		? 'every check, as a release asks'
		: mode === 'docs-only'
			? 'only documentation changed since dev, so nothing is built'
			: 'something beside documentation changed since dev';
	log(`just ${recipe}: ${mode} mode (${why}).`);

	for (const step of stepsFor(recipe, mode)) {
		const status = run(step);
		if (status !== 0) {
			return status;
		}
	}

	if (workingTreeHash(root) === tree) {
		recordPass(folder, recipe, tree, mode);
	} else {
		log(`just ${recipe}: the files changed during the run, so this pass is not remembered.`);
	}
	return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	const root = gitOut(process.cwd(), ['rev-parse', '--show-toplevel']).trim();
	process.exitCode = gate({ root, recipe: process.argv[2] });
}
