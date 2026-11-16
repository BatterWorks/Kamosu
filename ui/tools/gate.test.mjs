// `just check` and `just test` choose their mode and remember a pass through
// gate.mjs (#616). These drive it against throwaway Git repositories, with the
// steps it would run recorded instead of run.

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { after, test } from 'node:test';

import { DOCS_READING_TESTS, gate, isDocumentation, modeFor } from './gate.mjs';

const scratch = await mkdtemp(path.join(tmpdir(), 'kamosu-gate-test-'));
after(() => rm(scratch, { recursive: true, force: true }));

let repositories = 0;

function git(root, ...args) {
	return execFileSync('git', args, {
		cwd: root,
		encoding: 'utf8',
		env: {
			...process.env,
			GIT_AUTHOR_NAME: 'Test',
			GIT_AUTHOR_EMAIL: 'test@example.com',
			GIT_COMMITTER_NAME: 'Test',
			GIT_COMMITTER_EMAIL: 'test@example.com',
		},
	});
}

async function put(root, file, text) {
	await mkdir(path.dirname(path.join(root, file)), { recursive: true });
	await writeFile(path.join(root, file), text);
}

// A repository shaped like Kamosu's, with `dev` holding one commit and a work
// branch cut from it. Its target folder is its own.
async function repository() {
	repositories += 1;
	const root = path.join(scratch, `repo-${repositories}`);
	await mkdir(root);
	git(root, 'init', '--quiet', '--initial-branch=dev');
	await put(root, '.gitignore', 'target/\n');
	await put(root, 'src/main.rs', 'fn main() {}\n');
	await put(root, 'ui/src/app.css', 'body {}\n');
	await put(root, 'docs/adr/0001-first.md', '# First\n');
	await put(root, 'README.md', '# Kamosu\n');
	await put(root, 'SECURITY.md', '# Security\n');
	git(root, 'add', '--all');
	git(root, 'commit', '--quiet', '-m', 'dev');
	git(root, 'switch', '--quiet', '-c', 'work');
	return root;
}

function commit(root, message = 'work') {
	git(root, 'add', '--all');
	git(root, 'commit', '--quiet', '-m', message);
}

// One run of the gate. `fail` makes every step exit 1.
function runGate(root, recipe, { env = {}, fail = false } = {}) {
	const steps = [];
	const lines = [];
	const status = gate({
		root,
		recipe,
		env: { CARGO_TARGET_DIR: path.join(root, 'target'), ...env },
		run: (step) => {
			steps.push([step[0], ...step[1]]);
			return fail ? 1 : 0;
		},
		log: (line) => lines.push(line),
	});
	return { status, steps, first: lines[0], lines };
}

test('documentation is docs/ and the four root files, and nothing else', () => {
	for (const file of [
		'docs/adr/0060-something.md',
		'docs/docker-hub/overview.md',
		'docs/svelte/llms-full.txt',
		'CHANGELOG.md',
		'README.md',
		'AGENTS.md',
		'GLOSSARY.md',
	]) {
		assert.ok(isDocumentation(file), file);
	}
	// SECURITY.md is read by tests/defences.rs.
	for (const file of [
		'SECURITY.md',
		'src/docs.rs',
		'ui/README.md',
		'tests/fixtures/tandoor/README.md',
		'docsx/a.md',
		'justfile',
	]) {
		assert.ok(!isDocumentation(file), file);
	}
});

test('no change at all, or one change outside the documentation, is full', () => {
	assert.equal(modeFor([]), 'full');
	assert.equal(modeFor(['docs/a.md', 'src/main.rs']), 'full');
	assert.equal(modeFor(['docs/a.md', 'CHANGELOG.md']), 'docs-only');
});

test('a branch changing only an ADR runs docs-only, and names the mode first', async () => {
	const root = await repository();
	await put(root, 'docs/adr/0002-second.md', '# Second\n');
	commit(root);

	const check = runGate(root, 'check');
	assert.equal(check.status, 0);
	assert.match(check.first, /^just check: docs-only mode/);
	assert.deepEqual(check.steps, []);

	const testRun = runGate(root, 'test');
	assert.match(testRun.first, /^just test: docs-only mode/);
	assert.deepEqual(testRun.steps, [['node', '--test', ...DOCS_READING_TESTS]]);
});

test('the Docker Hub page test is among the docs-only tests', () => {
	assert.ok(DOCS_READING_TESTS.includes('ui/tools/docker-hub-page.test.mjs'));
});

test('a branch touching src/ or ui/ runs both in full', async () => {
	for (const file of ['src/main.rs', 'ui/src/app.css']) {
		const root = await repository();
		await put(root, 'docs/adr/0002-second.md', '# Second\n');
		await put(root, file, '// changed\n');
		commit(root);
		for (const recipe of ['check', 'test']) {
			const run = runGate(root, recipe);
			assert.match(run.first, new RegExp(`^just ${recipe}: full mode`), file);
			assert.deepEqual(run.steps, [['just', `_${recipe}-full`]], file);
		}
	}
});

test('uncommitted and untracked changes count, a deleted source file included', async () => {
	const root = await repository();
	await put(root, 'docs/new.md', '# New\n');
	assert.match(runGate(root, 'check').first, /docs-only mode/);

	await put(root, 'build.rs', 'fn main() {}\n');
	assert.match(runGate(root, 'check').first, /full mode/);

	const other = await repository();
	git(other, 'rm', '--quiet', 'src/main.rs');
	await put(other, 'docs/new.md', '# New\n');
	commit(other);
	assert.match(runGate(other, 'check').first, /full mode/);
});

test('the comparison is with the merge base, not with where dev has moved since', async () => {
	const root = await repository();
	await put(root, 'docs/adr/0002-second.md', '# Second\n');
	commit(root);
	git(root, 'switch', '--quiet', 'dev');
	await put(root, 'src/main.rs', 'fn main() { /* moved on */ }\n');
	commit(root, 'dev moves');
	git(root, 'switch', '--quiet', 'work');
	assert.match(runGate(root, 'check').first, /docs-only mode/);
});

test('a second run on the same clean tree reports the earlier pass and runs nothing', async () => {
	const root = await repository();
	await put(root, 'src/main.rs', 'fn main() { /* work */ }\n');
	commit(root);

	const first = runGate(root, 'check');
	assert.equal(first.steps.length, 1);

	const second = runGate(root, 'check');
	assert.equal(second.status, 0);
	assert.deepEqual(second.steps, []);
	assert.match(second.first, /^just check: remembered pass/);

	// Each recipe has its own record.
	assert.equal(runGate(root, 'test').steps.length, 1);
});

test('a pass on uncommitted files is remembered once they are committed', async () => {
	const root = await repository();
	await put(root, 'src/main.rs', 'fn main() { /* work */ }\n');
	const dirty = runGate(root, 'check');
	assert.equal(dirty.steps.length, 1);

	// Still dirty: the same contents run again, because the worktree is not clean.
	assert.equal(runGate(root, 'check').steps.length, 1);

	commit(root);
	assert.match(runGate(root, 'check').first, /remembered pass/);
});

test('any edit runs it again, and so does a worktree that is not clean', async () => {
	const root = await repository();
	await put(root, 'src/main.rs', 'fn main() { /* one */ }\n');
	commit(root);
	runGate(root, 'test');

	await put(root, 'src/main.rs', 'fn main() { /* two */ }\n');
	commit(root);
	assert.match(runGate(root, 'test').first, /full mode/);
	assert.match(runGate(root, 'test').first, /remembered pass/);

	// An untracked file makes the worktree unclean.
	await put(root, 'notes.txt', 'scratch\n');
	assert.match(runGate(root, 'test').first, /full mode/);
});

test('a failed run is not remembered', async () => {
	const root = await repository();
	await put(root, 'src/main.rs', 'fn main() { /* broken */ }\n');
	commit(root);
	assert.equal(runGate(root, 'check', { fail: true }).status, 1);
	assert.match(runGate(root, 'check').first, /full mode/);
});

test('a full pass answers a docs-only run, a docs-only pass never answers a full one', async () => {
	// A docs-only pass, then dev moves under the branch so that the same
	// contents now differ from the merge base in code: the run must be full.
	const root = await repository();
	await put(root, 'docs/adr/0002-second.md', '# Second\n');
	commit(root);
	assert.match(runGate(root, 'test').first, /docs-only mode/);
	assert.match(runGate(root, 'test').first, /remembered pass/);
	git(root, 'switch', '--quiet', 'dev');
	git(root, 'rm', '--quiet', 'src/main.rs');
	commit(root, 'dev drops a file the branch still has');
	git(root, 'switch', '--quiet', 'work');
	git(root, 'merge', '--quiet', '-s', 'ours', '-m', 'keep it', 'dev');
	// The merge leaves the branch's files exactly as they were.
	const full = runGate(root, 'test');
	assert.match(full.first, /^just test: full mode/);
	assert.deepEqual(full.steps, [['just', '_test-full']]);

	const another = await repository();
	await put(another, 'docs/adr/0002-second.md', '# Second\n');
	commit(another);
	runGate(another, 'check', { env: { KAMOSU_CHECK_MODE: 'full' } });
	assert.match(runGate(another, 'check').first, /remembered pass\. .* in full mode/);
});

test('a release runs both in full whatever the record says', async () => {
	const root = await repository();
	await put(root, 'docs/adr/0002-second.md', '# Second\n');
	commit(root);
	const env = { KAMOSU_CHECK_MODE: 'full' };
	for (const recipe of ['check', 'test']) {
		runGate(root, recipe, { env });
		const again = runGate(root, recipe, { env });
		assert.match(again.first, new RegExp(`^just ${recipe}: full mode`));
		assert.deepEqual(again.steps, [['just', `_${recipe}-full`]]);
	}
});

test('a tree that changes during the run is not remembered', async () => {
	const root = await repository();
	await put(root, 'src/main.rs', 'fn main() { /* one */ }\n');
	commit(root);
	const lines = [];
	gate({
		root,
		recipe: 'check',
		env: { CARGO_TARGET_DIR: path.join(root, 'target') },
		run: () => {
			execFileSync('sh', ['-c', 'echo changed > src/main.rs'], { cwd: root });
			return 0;
		},
		log: (line) => lines.push(line),
	});
	assert.match(lines.at(-1), /not remembered/);
	git(root, 'checkout', '--', 'src/main.rs');
	assert.match(runGate(root, 'check').first, /full mode/);
});

test('KAMOSU_CHECK_MODE takes only "full"', async () => {
	const root = await repository();
	assert.throws(
		() => runGate(root, 'check', { env: { KAMOSU_CHECK_MODE: 'Full' } }),
		/only "full"/,
	);
});

test('with no dev branch to compare with, the run is full', async () => {
	const root = await repository();
	git(root, 'branch', '-m', 'dev', 'elsewhere');
	await put(root, 'docs/adr/0002-second.md', '# Second\n');
	commit(root);
	assert.match(runGate(root, 'check').first, /full mode/);
});
