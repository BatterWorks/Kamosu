// The release hook publishes these two files to Docker Hub as they are
// (docs/releasing.md, "The release hook"). Docker Hub refuses text over its
// size limits, and it renders the overview away from this repository, so a
// relative address in it leads nowhere. This fails before a release does (#235).

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

const OVERVIEW = 'docs/docker-hub/overview.md';
const SHORT_DESCRIPTION = 'docs/docker-hub/short-description.txt';
const COMPOSE_FILE = 'docker-compose.yml';

// Docker Hub's limits for the two fields.
const SHORT_DESCRIPTION_MAX_CHARACTERS = 100;
const OVERVIEW_MAX_BYTES = 25_000;

async function read(file) {
	return readFile(path.join(repositoryRoot, file), 'utf8');
}

// What is wrong with a short description, as a list of plain sentences. The
// hook publishes the one line without its final newline.
function descriptionFaults(text) {
	const lines = text.replace(/\n$/, '').split('\n');
	if (lines.length !== 1) {
		return [`${lines.length} lines, and Docker Hub takes one`];
	}
	const [line] = lines;
	const faults = [];
	if (line.trim() === '') {
		faults.push('empty');
	}
	if (line !== line.trim()) {
		faults.push('a space at its start or end');
	}
	const characters = [...line].length;
	if (characters > SHORT_DESCRIPTION_MAX_CHARACTERS) {
		faults.push(`${characters} characters, limit ${SHORT_DESCRIPTION_MAX_CHARACTERS}`);
	}
	return faults;
}

function overviewFaults(text) {
	const bytes = Buffer.byteLength(text, 'utf8');
	return bytes > OVERVIEW_MAX_BYTES ? [`${bytes} bytes, limit ${OVERVIEW_MAX_BYTES}`] : [];
}

// Prose only: a path inside a code block or a code span is not an address
// Docker Hub turns into a link.
function prose(markdown) {
	return markdown.replace(/^```.*\n[\s\S]*?^```$/gm, '').replace(/`[^`\n]*`/g, '');
}

// The target of every Markdown link and image, every link reference
// definition, and every HTML `href` or `src`.
function addresses(markdown) {
	const text = prose(markdown);
	return [
		...text.matchAll(/\]\(\s*<?([^)\s>]*)/g),
		...text.matchAll(/^\s*\[[^\]]+\]:\s*<?(\S*?)>?\s*$/gm),
		...text.matchAll(/\b(?:href|src)\s*=\s*["']?([^"'\s>]*)/gi),
	].map(([, address]) => address);
}

// A Compose file as Docker reads it: no comment lines and no blank ones.
function withoutComments(yaml) {
	return yaml
		.split('\n')
		.filter((line) => line.trim() !== '' && !line.trim().startsWith('#'))
		.join('\n');
}

test('the short description is one line within the limit', async () => {
	assert.deepEqual(descriptionFaults(await read(SHORT_DESCRIPTION)), []);
});

test('the overview fits the size limit', async () => {
	assert.deepEqual(overviewFaults(await read(OVERVIEW)), []);
});

test('a short description past the limit is a fault, and one at it is not', () => {
	assert.deepEqual(descriptionFaults(`${'a'.repeat(100)}\n`), []);
	assert.deepEqual(descriptionFaults('a'.repeat(101)), ['101 characters, limit 100']);
	// The limit counts characters, so an accented letter is one of them.
	assert.deepEqual(descriptionFaults('é'.repeat(100)), []);
	assert.deepEqual(descriptionFaults('one\ntwo\n'), ['2 lines, and Docker Hub takes one']);
	assert.deepEqual(descriptionFaults('\n'), ['empty']);
	assert.deepEqual(descriptionFaults(' padded'), ['a space at its start or end']);
});

test('an overview past the limit is a fault, and one at it is not', () => {
	assert.deepEqual(overviewFaults('a'.repeat(25_000)), []);
	assert.deepEqual(overviewFaults('a'.repeat(25_001)), ['25001 bytes, limit 25000']);
	// The limit counts bytes, so an accented letter is two of them.
	assert.deepEqual(overviewFaults('é'.repeat(12_501)), ['25002 bytes, limit 25000']);
});

test('every link and image in the overview is an absolute address', async () => {
	const found = addresses(await read(OVERVIEW));
	assert.notEqual(found.length, 0, 'the overview links somewhere');
	assert.deepEqual(
		found.filter((address) => !/^https:\/\/[^/]/.test(address)),
		[],
		'relative addresses do not resolve on Docker Hub',
	);
});

test('the address finder sees every way Markdown can point somewhere', () => {
	assert.deepEqual(
		addresses(
			[
				'[guide](docs/guide.md) and ![shot](assets/shot.png)',
				'[ref]: ../README.md',
				'<a href="#tags">Tags</a> <img src=assets/logo.svg>',
				'`[not](a-link)` in a code span',
				'```yaml',
				'- [nor](this)',
				'```',
			].join('\n'),
		),
		['docs/guide.md', 'assets/shot.png', '../README.md', '#tags', 'assets/logo.svg'],
	);
});

test("the overview's Compose file is the repository's, without its comments", async () => {
	const block = /^```yaml\n([\s\S]*?)^```$/m.exec(await read(OVERVIEW));
	assert.ok(block, 'a Compose file in a yaml code block');
	assert.equal(withoutComments(block[1]), withoutComments(await read(COMPOSE_FILE)));
});
