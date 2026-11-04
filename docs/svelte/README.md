# Svelte's documentation for models

Svelte publishes its documentation written for language models. Committing it is
part of [ADR 0012](../adr/0012-the-compiler-is-the-reviewer-the-frontend-does-not-have.md),
not optional hygiene: Svelte was chosen over React on the argument that the one
real risk — a model reaching for Svelte 4 habits instead of Svelte 5 runes — is
closable at build time, and this is what closes it.

| File             | What it is                                              |
| ---------------- | ------------------------------------------------------- |
| `llms-small.txt` | The abridged docs. Named in `AGENTS.md`; read it first.  |
| `llms-full.txt`  | The complete docs. A searchable lookup, not a read.      |

Search the full file rather than reading it — 1.2 MB:

```sh
grep -n -A 20 '## \$derived' docs/svelte/llms-full.txt
```

**The abridged file still describes SvelteKit 2 in three places.** Svelte had
not regenerated it for SvelteKit 3 when it was last fetched. Kamosu is on
SvelteKit 3, and the full file is right where these disagree:

- Configuration lives in `ui/vite.config.ts`, passed to the `sveltekit()`
  plugin. There is no `svelte.config.js`.
- Shared code is imported as `#lib/<file>.js` (or `.svelte`), a subpath import
  declared in `ui/package.json`. The `$lib` alias is gone.
- `ui/tsconfig.json` extends `$app/tsconfig`, and the service worker in
  `ui/src/service-worker/` is a TypeScript project of its own.

## Pinned

Fetched **4 November 2026** from <https://svelte.dev/docs/llms>, against:

| Package             | Version |
| ------------------- | ------- |
| `svelte`            | 5.57.3  |
| `@sveltejs/kit`     | 3.1.1   |

| File             | SHA-256                                                            |
| ---------------- | ------------------------------------------------------------------ |
| `llms-small.txt` | `cea89187dfb7a2541124c37f41846632b2b9a670e377903387a19d562d6f45c4` |
| `llms-full.txt`  | `0bbb5f5084d332962eaf7b3cfd780680cb9b492302fbc542a7194ec893a8a3ef` |

**Refreshed deliberately, never automatically.** These files are the syntax
every frontend session works from, so they move when someone decides to move
them — alongside a Svelte upgrade, as its own piece of work. A silent refresh
would change what every future session believes about the framework without
anybody choosing it.

To refresh, run both fetches, update the table above, and say in the commit
message which Svelte version the new copies describe:

```sh
curl -sS -o docs/svelte/llms-small.txt https://svelte.dev/llms-small.txt
curl -sS -o docs/svelte/llms-full.txt  https://svelte.dev/llms-full.txt
sha256sum docs/svelte/llms-*.txt
```
