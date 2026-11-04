/**
 * The back arrow reads where SvelteKit numbers a history entry, which is no
 * part of what SvelteKit documents (`way-back.ts`). SvelteKit 3 moved it, and
 * the arrow quietly stopped going back while every screen test still passed,
 * because the tests wrote the entry themselves (#525). This reads SvelteKit's
 * own client, so the next move fails here instead.
 */

import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { HISTORY_METADATA } from './way-back';

const kit = dirname(createRequire(import.meta.url).resolve('@sveltejs/kit/package.json'));
const client = (file: string) => readFileSync(join(kit, 'src/runtime/client', file), 'utf8');

describe('where SvelteKit numbers a history entry', () => {
	it('is the key the back arrow reads', () => {
		expect(client('constants.js')).toContain(`HISTORY_METADATA_KEY = '${HISTORY_METADATA}'`);
	});

	it('holds the number as historyIndex', () => {
		expect(client('client.js')).toMatch(
			/\[HISTORY_METADATA_KEY\]: \/\*\* @satisfies \{HistoryMetadata\} \*\/ \(\{\s*historyIndex:/,
		);
	});
});
