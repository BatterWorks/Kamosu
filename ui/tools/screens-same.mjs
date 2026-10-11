/**
 * `just screens-same <before> <after>` (#216): names the pictures that differ
 * between two folders `just screens` filled, and fails if any do.
 *
 * Files are compared byte for byte. That is enough because `just screens`
 * writes the same bytes for the same pixels, and it needs no image library.
 * A picture in one folder and not the other counts as a difference.
 */

import fs from 'node:fs';
import path from 'node:path';
import { ToolError, fromRoot, run } from './dev.mjs';

/**
 * The pictures in a folder.
 * @param {string} folder
 */
function pictures(folder) {
	if (!fs.existsSync(folder)) throw new ToolError(`there is no folder at ${folder}`);
	return fs.readdirSync(folder).filter((name) => name.endsWith('.png'));
}

/**
 * What differs between two folders of pictures, one line for each.
 * @param {string} before
 * @param {string} after
 * @returns {{ compared: number, differences: string[] }}
 */
export function compare(before, after) {
	const was = pictures(before);
	const is = pictures(after);
	const names = [...new Set([...was, ...is])].sort();
	if (names.length === 0) throw new ToolError('neither folder holds a picture');
	const differences = [];
	for (const name of names) {
		if (!is.includes(name)) differences.push(`${name}: only in ${before}`);
		else if (!was.includes(name)) differences.push(`${name}: only in ${after}`);
		else if (
			!fs.readFileSync(path.join(before, name)).equals(fs.readFileSync(path.join(after, name)))
		) {
			differences.push(`${name}: differs`);
		}
	}
	return { compared: names.length, differences };
}

if (process.argv[1] === import.meta.filename) {
	await run(async () => {
		const [, , before, after] = process.argv;
		if (!before || !after) throw new ToolError('usage: just screens-same <before> <after>');
		const { compared, differences } = compare(fromRoot(before), fromRoot(after));
		if (differences.length) {
			throw new ToolError(
				`${differences.length} of ${compared} pictures are not the same:\n${differences.join('\n')}`,
			);
		}
		console.log(`all ${compared} pictures are the same`);
	});
}
