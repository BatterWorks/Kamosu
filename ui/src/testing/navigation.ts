/**
 * Where a screen asked to go, for every test file alike (#122).
 *
 * `$app/navigation` is mocked once, in `setup.ts`, with this one `goto`. Each
 * spec used to mock it with its own spy, which works only while every file gets
 * a fresh module registry: with isolation off, a component is imported once per
 * worker and keeps the `goto` of whichever file imported it first, so a later
 * file's spy was never called. One spy, cleared after every test, has no first.
 */

import { onMount } from 'svelte';
import { vi } from 'vitest';

export const went = vi.fn();

/**
 * The snapshots screens have registered with `$app/navigation`'s `snapshot`,
 * by id, as SvelteKit holds them (#530): added when the component mounts and
 * taken away when it is destroyed. A test has no history to walk, so it plays
 * SvelteKit's part with what is here, calling `capture` as a reader leaves and
 * `restore` as they come back.
 */
export const kept = new Map<string, SnapshotOptions>();

interface SnapshotOptions<T = unknown> {
	id?: string;
	capture: () => T;
	restore: (value: T) => void;
	reset?: () => void;
}

/** `snapshot` from `$app/navigation`, keeping the registration where a test can reach it. */
export function snapshot<T>(options: SnapshotOptions<T>): void {
	const id = options.id;
	// SvelteKit makes an id from the call site when none is given; a test has
	// no stable call site to make one from, so every screen here names its own.
	if (id === undefined) throw new Error('a snapshot here needs its own id');
	onMount(() => {
		// A test reads back what it captured, so the value's type is its own.
		kept.set(id, {
			...(options as unknown as SnapshotOptions),
			// SvelteKit writes what it captured out as text and reads it back
			// before `restore` sees it. JSON is stricter than SvelteKit's devalue,
			// so a value that passes here survives the real round trip too.
			capture: () => {
				const left = options.capture();
				return left === undefined ? undefined : JSON.parse(JSON.stringify(left));
			},
		});
		return () => kept.delete(id);
	});
}
