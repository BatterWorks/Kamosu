# AGENTS.md

Project conventions for Kamosu. Host-specific conventions live in
`AGENTS.local.md`; vocabulary lives in `GLOSSARY.md`; decisions live in
`docs/adr/`. **Where the spec and an ADR disagree, the ADR wins.**

## Dependencies

**The registry is the source of truth for every version number.** A version
you write from memory is a version that was current on your training cutoff,
which is not today.

Add a dependency by letting its package manager resolve it:

```sh
cargo add <crate>              # writes the current version into Cargo.toml
npm install <package>          # writes the current version into package.json
npx sv create                  # scaffolds SvelteKit at its current version
```

A version string typed by hand into `Cargo.toml` or `package.json` is a bug,
the same way a permission check inside a Door is a bug.

**Kamosu's own version is set by `just release-prepare` and by nothing else**
(ADR 0047). It stands in `Cargo.toml`, `Cargo.lock`, `ui/package.json` and
`ui/package-lock.json`, and a hand edit to any of the four is the same bug.

**Svelte's own documentation for models is committed to this repo.** Read
`docs/svelte/llms-small.txt` before writing any Svelte or SvelteKit code, and
search `docs/svelte/llms-full.txt` for what the small file omits. This is not
optional: Svelte was chosen over React on the argument that a model reaching for
Svelte 4 habits is a risk closable at build time, and these files are what closes
it (ADR 0012). Both are pinned; `docs/svelte/README.md` says to which version and
how to refresh them.

**Read the library's current documentation before writing against it.** Use the
**Context7** MCP server before writing code that calls a library API, even when
you believe you know the API. Getting the version number right and then calling
a function that was renamed two releases ago is the same failure wearing a
different hat.

Two calls, in order:

1. `context7_resolve-library-id` with `libraryName` (**and** `query` — both are
   required; the query says what you're trying to do and ranks the results).
2. `context7_query-docs` with the `libraryId` that returned (pin a version when
   one is offered, e.g. `/tokio-rs/axum/axum_v0_8_4`) and a `query` scoped to
   **one concept per call** — multi-concept queries return mush.

**Take the current version when a dependency is first added, then hold it.**
Chasing a major release mid-build is a rewrite, not an upgrade; upgrade
deliberately, as its own piece of work.

**Holding a version is not the same as ignoring an advisory.** `just audit`
checks both trees — cargo-deny against RustSec, `npm audit` against GitHub's
database — and it is the one reason to move a version outside a deliberate
upgrade. A published vulnerability or a yanked release is a fact about the
world, not a new feature you are chasing. This is why no Dependabot or Renovate
runs here: automated version-bump pull requests would fight the paragraph above
every week, and `just audit` gives the security half without the churn. Read
what it reports rather than obeying it — `npm audit`'s suggested remedy is
routinely a downgrade that breaks more than it fixes. Advisories judged not to
apply are recorded, with the reasoning, in `deny.toml`.

**Kamosu pins its compiler.** `rust-toolchain.toml` fixes the Rust version so
every machine and every CI run builds identically. Pinning the toolchain and
letting crates float are not in conflict: one is the kitchen, the other is the
ingredients.

## Branching

All work happens on a branch off `dev`. Never commit to `dev` or `main`
directly, whatever the size of the change. Both are protected on GitHub: each
takes a pull request and nothing else, from an admin too.

Before the first edit, check the current branch and cut one if you are sitting
on a protected branch:

```sh
git branch --show-current
git switch -c <prefix>/<short-slug> dev
```

Use `feature/`, `fix/`, `docs/`, or `refactor/` as the prefix. Push that branch
and open a pull request against `dev`; `main` is the release branch and receives
only merges from `dev`, which `.github/workflows/release-source.yml` checks.

**While a release is open, nothing else merges into `dev`.** From the moment
`just release-prepare` cuts `docs/release-v<version>` until `just
release-publish` has merged the release, leave every other pull request open
(see Releasing).

`dev` refuses a direct push, so cutting the branch after the work is committed
means moving commits off a branch you cannot push, not a quick fix. Check first.

Every merge is a merge commit, including `dev` into `main`. Do not squash, do
not rebase-merge, and never force-push `dev` or `main`.

**Whoever built a pull request merges it.** GitHub asks for nobody's approval,
so the gate is the work's own: `just check` and `just test` pass, the review's findings are
settled, and live acceptance has passed on the code being merged. Then merge
with a merge commit, as `gh pr merge <number> --merge` does, and confirm the
merge commit is on `dev`. A host that keeps a second copy of the repository
says in its `AGENTS.local.md` what to do after a merge; do that before moving on.

**A pull request closes its ticket by naming it**: `Closes #N` in the title, the
body or a commit message. GitHub acts on that only for the default branch, which
is `main`, so `.github/workflows/close-issues-on-dev.yml` does it when a pull
request merges into `dev`. It reads one number after each keyword, so two
tickets take `Closes #1, closes #2`, and it does not read the sentence around
the keyword: "does not fix #7" closes #7. Never close a ticket by hand before
its pull request has merged.

## The changelog

`CHANGELOG.md` opens with `## Unreleased`, and a release is cut from it.

**A ticket that changes what a person running Kamosu sees or does adds its line
under `## Unreleased`, in the same commit as the change.** That person is a
cook, an Operator, or an agent at the MCP Door. A ticket with nothing for them
to notice adds no line: a refactor, a test, a tool under `ui/tools/`, a change
to these instructions.

- **One line per change**, written for that reader in the voice the file
  already has: what Kamosu does now, and what it did before where that is the
  news.
- **Under the heading it belongs to**: Added, Changed or Fixed. A change someone
  must act on when upgrading goes under `### After upgrading`, which comes
  first.
- **A fix to something still under `## Unreleased` gets no line of its own.**
  Correct the line already there. The same goes for a change that is reverted
  or replaced before a release: take its line out.
- **End the line with its issue as `[#N]`**, and add `[#N]: <url>` to the block
  of definitions at the foot of the section, in number order. A change with no
  issue names its pull request the same way.
- **The changelog is public.** It names no instance, no host and no person.

## Releasing

A release is cut by two commands and nothing else (ADR 0047). **Read
`docs/releasing.md` before running either**: it is the runbook, with the
checklist, the release hook's contract and every refusal.

```sh
just release-prepare <version>   # the bump pull request, its checklist and a draft release; run again after the bump merges, the pull request into main
just release-publish <version>   # merge, tag, have the hook build the images, publish
just release-library <folder>    # the checklist's library check: a copy of a real library through every migration
```

- **Only the maintainer approves a release's title and notes.** Show the draft
  as it stands on GitHub and stop. Run `just release-publish` only after the
  reply "approved" in this same session, and ask again if the text changed or
  the session is new.
- **Both commands resume.** One run again after a failure carries on at the
  first step not done. Never finish a step by hand in its place.
- **Neither force-pushes, moves a tag or replaces a published version.** A
  release found broken after publishing is fixed by the next patch version.
- **The bump branch is pushed by `just release-prepare` and no other way.** It
  runs `just check` and `just test` before it pushes, and trusts a commit
  already on the remote to have passed them.
- **The scripts are in `ui/tools/`**, and their tests run in `just test`.
  `release-publish` reads a checklist item only in its exact wording, which
  `ui/tools/release-common.mjs` holds for both commands.

## Building, testing and running

Everything runs through `just` at the repo root — never a hand-rolled
`cargo run &`, which leaks a process holding the port.

```sh
just check      # fmt + clippy + prettier + eslint + svelte-check + the freshness gates (below) + the copy guard
just test       # the behaviour suite (real Operations, real SQLite) + the screen tests
just format     # apply Prettier to ui/ — the Svelte half of `cargo fmt`
just audit      # both dependency trees against their advisory databases (needs network)
just css        # regenerate assets/app.css and the icon PNGs from ui/ (their source)
just client     # regenerate the typed client from the Catalogue (see The interface)
just ui-build   # build the Svelte app into ui/build, which the binary embeds
just dev-start  # server on 5266 AND vite on 5174, both on 0.0.0.0; prints both URLs
just dev-status # each half running? since when? build-cache size
just dev-logs   # follow .dev/kamosu.log and .dev/vite.log together
just dev-stop   # stops both process groups
just docker-build && just docker-run   # the one-mount-one-port install, as a stranger runs it
```

**Looking at the dev instance has recipes of its own** (#216), so no ticket
writes a browser script by hand. The scripts are in `ui/tools/`, and Playwright
is a dev dependency of `ui/`: never load it from anywhere else.

```sh
just dev-session <label>          # sign in as the account in .env; prints the cookie file's path, nothing else
just dev-session-end <label>      # end that one Session
just screens <folder> <path>…     # a whole-page picture of each path at every size; fails on a console error or a sideways scroll
just screens-same <before> <after>   # names the pictures that differ, and fails if any do
just proto-new <issue>-<slug> <path>…   # .dev/prototypes/<issue>-<slug>/: the real screens as static pages
just proto-serve <name> <port>    # serve .dev/prototypes/<name>/ to the LAN, caching off
```

- **The sizes are written once**, in `ui/tools/sizes.mjs`. A script that needs a
  screen size imports it from there.
- **"Is the phone unchanged?" is two sets of pictures.** Run `just screens` into
  a "before" folder under `.dev/` when a ticket is claimed, while the worktree is
  still clean, and into an "after" folder once it is built. `just screens-same`
  compares them. No stash and no second worktree.
- **A Session's Secret stays out of the transcript.** A script is handed the
  path of a cookie file and gives the cookie to its browser itself, as
  `signedInContext` in `ui/tools/dev.mjs` does. `screens` and `proto-new` sign in
  on their own as a Session named `tools`.
- **`just screens … --webkit` runs in Safari's engine**, which has shown a fault
  Chrome did not (#198). Fetch it once with `cd ui && npx playwright install webkit`.
- **`.env` at the repo root says where and as whom.** `KAMOSU_DEV_HOST` is the
  dev instance's address, reached over HTTPS because a Session cookie is Secure.
  `KAMOSU_DEV_ADMIN_NAME` and `KAMOSU_DEV_ADMIN_PASSWORD` are the account. The
  browser is the Chrome on the path unless `KAMOSU_DEV_BROWSER` names another.
- **A path with `?` in it is quoted** at the prompt, as in
  `just screens .dev/after '/recipes?q=tart'`, or the shell reads it as a pattern.
- **`just proto-serve` serves a prototype by name and nothing else.** The whole
  LAN can read what it serves, and the repo root holds `.env`.

**Development is two processes side by side** (ADR 0028): the binary serves the
Operations on **5266**, vite serves the interface on **5174** and proxies
everything the binary owns to it, so a browser only ever talks to one origin.
Open the 5174 URL. Both ports are fixed with no environment variable — 5173 is
taken on the dev host, and vite runs with `--strictPort` so a silent fallback to
another port cannot happen. `dev-start` is always safe to re-run for both.

**`just test` needs Linux and the `sqlite3` command-line tool on the path.**
One behaviour test runs `sqlite3` as the other process that opens the database
beside a running Kamosu, and reads `/proc/self/fdinfo` (#130, #215). The
SQLite Kamosu bundles hides the fault that test looks for, and an older one
does not.

**It needs `python3` on the path too.** The Runner's tests start the real
program as a Runner, and its agent command there is a short Python stand-in
that writes a recipe with the Job Key it was handed (#274).

`dev-clean` deletes `target/` to reclaim build cache; the next build is a full
rebuild. Dev state lives in gitignored `.dev/`.

**Layout is settled by a formatter in both languages** — `cargo fmt` for Rust,
Prettier for `ui/` — and `just check` fails on either. Never argue with them and
never hand-format; run `just format` and move on. Two things `ui/.prettierignore`
excludes are worth knowing before you try to "fix" them: the generated files,
because `just check` compares them byte-for-byte against a fresh regeneration,
and `ui/src/app.css`, because Tailwind passes that file's indentation through
into the compiled stylesheet that ships.

**Lints are held to the same standard in both languages** — `cargo clippy -D
warnings` for Rust, `eslint --max-warnings 0` for `ui/`. The ESLint net is
deliberately narrow, because svelte-check and a strict `tsconfig` already catch
most of what it would: what is left is the part a type checker cannot see.
`ui/eslint.config.js` explains the two Svelte rules Kamosu turns off and what
each was guarding — read the reasoning before turning either back on.

## Design tokens

What Kamosu looks like is **one generated stylesheet**, `assets/app.css`,
compiled from `ui/src/app.css` — Tailwind 4 theme tokens that replace Tailwind's
palette, type scale, spacing and radii wholesale (`just css` regenerates).

**`ui/src/app.css` is the current value of every token; the dated design records
under `docs/design/` are not.** A record says what was chosen on one day and is
never updated, so a token it names may since have moved — `--hero-h` did, when
#81 put the recipe's title on its hero. Read the record for *why* a direction
was taken and the stylesheet for what is true now, and when you move a token,
say in a comment beside it which decision moved it.
The committed output is what the binary embeds at compile time, so a cargo build
never needs Node — but `just check` regenerates it into a temp directory and
**fails if what is committed has drifted from `ui/`**. Never hand-edit
`assets/app.css`; change `ui/src/app.css` and run `just css`.

**What a control looks like under the pointer is written once**, at the foot of
`ui/src/app.css` (#203, ADR 0044), for every link, button and list row there is.
Never write a `hover:` class on a screen. A surface says what darkens on it through
`--under-pointer`, and a control with a fill of its own is named in that block
fill by fill. A new fill class on something clickable does nothing under the
pointer until it has a line there, and a screen test fails until it does. The rules sit outside every layer on purpose,
so they beat the utility a control was written with. A picture that is itself
the thing to click takes the class `brightens`.

**A type size is a count of `--type-unit`, and no other length is written in rem**
(#228, ADR 0046). The unit is 1px until the reader's own text size moves it, up
to 1.35px, so `calc(26 * var(--type-unit))` is 26px at rest. The root's font
size is the system's text size, and a width in rem would grow with someone's
reading. Never write a font size in px on a screen: use a `text-*` token, and add
one beside the others when a new size is chosen. `ui/src/lib/type-unit.spec.ts`
fails on a px font size or a rem length in `ui/src/app.css`. Kamosu has no dark
mode, on purpose (ADR 0045).

**Tailwind reads `ui/src/` and nothing else**, declared with `source(none)` and
an explicit `@source` at the top of `ui/src/app.css`. The automatic detection
that replaced sweeps every non-gitignored file under `ui/` — including config
files and the prose inside their comments — and a bare utility word written in a
comment emits a real rule into the stylesheet that ships. If a new place ever
holds markup (the Rust-rendered Share Link page will), add an `@source` line for
it rather than removing the scoping.

## The interface

The frontend is **Svelte 5 + SvelteKit on `adapter-static` + TypeScript +
Tailwind 4 + Paraglide**, and it is **compiled into the binary** (ADR 0028) —
never copied beside it. `rust-embed` reads `ui/build` from disk in a debug build
and embeds it in a release build, so a frontend change recompiles no Rust and a
release binary that would ship no interface fails in `build.rs` instead.

**The interface calls Kamosu through a client generated from the Catalogue.**
`kamosu catalogue` prints every Operation's declaration as JSON; `just client`
turns that into `ui/src/lib/api/catalogue.ts`. It is the third thing built by
walking the one declaration list, after the two Doors, so the frontend and the
Core cannot disagree about an Operation's shape. Never edit the generated file:
change `src/catalogue.rs`, run `just client`, and commit both. `just check`
fails if what is committed has drifted.

**Screens are tested against a Catalogue-derived stand-in.** `standIn({ … })` in
`ui/src/lib/api/stand-in.ts` answers what a test says — and checks every answer
against the output schema the Catalogue declares first. A test can lie about the
values; it cannot lie about the shape. Screens take their client from context,
never from `fetch`.

**Every bottom sheet is drawn by one frame, `ui/src/lib/SheetFrame.svelte`**
(#196, ADR 0044): a bottom sheet on the phone layout, a window in the middle of
the room on the wide one. This is the panel that rises over a screen, and has
nothing to do with the **Sheet** of `GLOSSARY.md`, a recipe set for paper. The frame owns the dimmed page, Escape, the caret going in and
coming back, and Enter. A screen hands it content and never writes
`role="dialog"` itself; a screen test fails if one does.

**A file or a link is dropped only where a drop has one meaning, and both halves
of that are in `ui/src/lib/drop.svelte.ts`** (#204, ADR 0044). `DropZone` is one
place that takes a drop: it says what is held over it and hands over what was let
go. `keepDropsOut`, started once by the root layout, turns away every drop no
zone took, so a browser never opens a dropped file in place of Kamosu. `FilePlace`
is a zone for one file of one kind, a photograph or a Crouton export (#205): the
place wears `DropCover` where it is big enough and `DropHere` where it is not. A drop is
the act a button already does, run through that button's own code, and it says
offline what the button says. Never read `dataTransfer` on a screen.

**`svelte-check` runs strict and its warnings are failures**, Svelte's
accessibility warnings included (ADR 0012). **Paraglide compiles every phrase to
a function**, so a misspelt key is a build error; messages live in
`ui/messages/{en,fr,es}.json` and all three are present from the first day.

## Project layout

```
build.rs            refuses a release build that would ship no interface
src/backups.rs      a Backup: one archive of the database and the
                    Photographs, three kept at three distances (ADR 0039)
src/bundles.rs      a Bundle: one recipe's worth of Vault, written as a zip of
                    readable notes and a .kamosu/ sidecar (ADR 0020)
src/catalogue.rs    the Catalogue: every Operation declared once
src/core/           the Core, one file per area, all adding to one `Core`:
  mod.rs            the Core itself, startup, `execute` with the only
                    authorisation check in Kamosu, and Credential resolution
  accounts.rs       people, logins, Sessions, Access Keys, invites, recovery,
                    a Person's Reading Language, Operator
  kitchens.rs       Kitchens, their members, and leaving one: who sees
                    whose recipes (ADR 0041)
  cookbooks.rs      Cookbooks: who may change a recipe, joining and
                    separating two, variations, and the may-see and
                    may-change checks every area calls (ADR 0041, #131)
  recipes.rs        creating, saving, reading a recipe; threads and divergence;
                    unfolding components; `parse_recipe_content`
  tags.rs           Tags, and putting one on a recipe
  related.rs        Related Recipes (#105)
  runners.rs        Runner Keys, a Runner taking a Job, renewing its hold and
                    reporting how it ended, the Job Key that comes with one
                    and the one recipe its writes are held to, who reads an
                    agent's Job, and whether the instance has a Runner (ADR
                    0048, #260, #262, #263, #264, #266, #268)
  shelves.rs        the home shelves, and the shelf entry every list shows
  translation.rs    translation and a Branch's Language
  reading_again.rs  a Co-author asking for an agent to read a recipe again
                    from its kept Original, a file handed over again, or
                    its Source's link, and whether one can (#272, #273)
  readings.rs       Ingredient Lines, Readings, Foods and Food merges
  originals.rs      pasted text, a PDF or a picture handed to an agent,
                    kept under originals/ for its Job's Job Key alone and
                    in no Backup, and deleted 30 days after its Job ended.
                    The same file handed over again is known by its
                    contents, and a Job that failed on one is asked
                    again from it (#269, #270, #271, #273)
  cooking.rs        Attempts and promotion
  keeping_photographs.rs
                    storing, reading and sweeping Photographs
  imports.rs        Crouton, staged uploads, the import ledger
  share_links.rs    Share Links, the public address, and the page a
                    stranger holding one sees (ADR 0018, ADR 0026)
  sheets.rs         Sheets: a recipe set as a PDF (ADR 0023)
  carrying_bundles.rs
                    Bundles out and in: writing one, previewing and
                    receiving one (ADR 0020)
  shopping_list.rs  the shopping list
  search.rs         word search and Meaning Search
  keeping_backups.rs
                    taking and listing Backups
  shared.rs         the few helpers several areas call and none owns
src/cookbook_migration.rs
                    migration 36's Rust half: every recipe moved from the
                    Kitchen that held it into a Cookbook (#131)
src/crouton.rs      reading a Crouton export into import candidates: lines rebuilt
                    from split fields, favicons and nutrition text left out (#69)
src/db.rs           the SQLite file under /data — WAL on, migrations forward-only
src/entities.rs     the HTML entities importers undo in text that was never HTML
src/jobs.rs         slow work: two lanes (one for strangers, ADR 0032), progress, results
                    and an agent's Job, which waits for a Runner in neither lane
                    and is running while one holds it, with a Job Key minted
                    each time a Runner takes it. The hold has a deadline, and
                    a Job is tried a fixed number of times. It is cancelled
                    waiting or held. One reading an Original waits with no
                    recipe until its agent creates one, unless it reads a
                    recipe again (ADR 0048, #261, #262, #263, #266, #267,
                    #269, #273)
src/language.rs     a Branch's Language, read off the recipe's own text (ADR 0006)
src/meaning.rs      Meaning Search: the model Kamosu does not ship, and the
                    index it builds from the recipes (ADR 0029)
src/operations.rs   the functions the Catalogue's declarations name
src/schema.rs       the Catalogue's declared input, compiled into the thing
                    that checks it — shape only, in the Core at dispatch, so
                    both Doors inherit it (#85)
src/reading.rs      reading an Ingredient Line into a Reading — where the
                    number ends, the Unit ends and the Food begins. Its
                    vocabulary is units.rs's, not its own (ADR 0036)
src/step_uses.rs    which Ingredient Lines a Step uses, by the short names
                    recipes call them after the list (#184). A short name
                    holds the name's main word, which sits at the end the
                    Branch's Language puts it (#212)
src/page_text.rs    the text of a page's own article, found as a reader
                    would and written out as a browser copies it, for a
                    link with no recipe data on an instance with no
                    Runner (#301)
src/runner.rs       the Runner: `kamosu runner`, which asks one Kamosu for
                    Jobs and starts an agent command in a fresh folder for
                    each. Its client is its own, apart from the guarded one
                    (ADR 0033, ADR 0048, #274)
src/pdf.rs          the text out of a recipe PDF, for the paste reader; a
                    scan holds none and is refused (#176)
src/web_door.rs     Axum router generated by walking the Catalogue
src/mcp_door.rs     MCP endpoint generated by walking the Catalogue
src/browser_policy.rs
                    the three safety headers every answer carries: CSP,
                    nosniff and Referrer-Policy (#139)
src/config.rs       the four optional environment variables
src/http_min.rs     dependency-free HTTP client for the self-run healthcheck
src/design_tokens.rs embeds and serves the generated stylesheet, fonts and icons
src/interface.rs    embeds and serves the built Svelte app; owns the fallback
tests/parity.rs     both Doors materialise every Operation — drift breaks the build
tests/behaviour.rs  behaviour through real Doors against a real database file
tests/runner.rs     the real program started as a Runner beside a test
                    Kamosu, with a stand-in for the agent (#274)
tests/*_corpus.rs   the same, against the real 86-recipe Crouton export —
                    #[ignore]d, since samples/crouton/ is personal and gitignored:
                    run with `cargo test --test <name> -- --ignored`.
                    They carry `test = false`, so `just test` does not build
                    them; `just check` type-checks them instead. Naming one
                    still builds and runs it, exactly as that command does.
                    meaning_corpus.rs also wants a network and ~220 MB of model,
                    which it keeps in .dev/meaning-model between runs
tests/release_library.rs
                    a copy of a real library carried through every migration,
                    for the release checklist. `#[ignore]`d and `test = false`
                    like the corpus checks; `just release-library` runs it
ui/                 the Svelte app; ui/src/app.css is the design tokens' source
ui/tools/           the dev tools `just` runs against the dev instance: sign in,
                    photograph screens, capture and serve a prototype (#216).
                    The two release commands live here too (ADR 0047)
docs/releasing.md   the release runbook
docs/docker-hub/    the Docker Hub page of the image: its overview and its one-line
                    description, published by a release and never edited on
                    Docker Hub (#235)
docs/svelte/        Svelte's documentation for models, pinned (read this first)
```

## How the program is shaped

Every Operation is declared once in the Catalogue — name, input, output,
required permission, the function that performs it — and **both Doors are built
by walking that declaration list at startup** (ADR 0001). There is no place to
hand-write a web-only route; adding an Operation means adding one entry to the
Catalogue and both Doors grow it in the same commit.

- The web API is deliberately RPC-shaped (`POST /api/op/<name>`). Rewriting it
  as REST destroys the parity guarantee; don't.
- **A permission check written inside a Door is a bug.** Authorisation happens
  once, in the Core, keyed on the Credential. Doors only carry the Secret.
- **What a Job Key may call is the `job_key` mark on each declaration, and that
  mark is a security boundary** (ADR 0048). A new Operation is `JobKey::Never`.
  Marking one for a Job Key takes a decision of the ADR's weight, and
  `the_job_keys_list_is_these_operations` in `tests/behaviour.rs` fails until
  the name is added there on purpose.
- **A Job Key's write is held to its Job's recipe in `Core::execute`, never in
  the Operation** (#264). The mark says which: `WritesItsRecipe` names the
  recipe in `branch_id`, and any other recipe answers as one that is not
  there. `CreatesItsRecipe` creates the recipe of a Job that has none, once,
  and the Core makes it the Job's own (#269). Where the asker's own change
  would start a Copy, the write is refused and the Job fails. No Operation
  decides what a Job may reach. A Job Key may use two routes outside the
  Catalogue: `POST /api/photographs`, by the mark on `upload_photograph`, and
  `GET /api/originals/<job_id>` for its own Job's Original.
- **A save knows which Job is writing, and that is all any Operation knows of
  Jobs** (#265). `save_onto_branch` writes it on the Version's row beside the
  Access Key, folds a Job's saves into the one Version that Job wrote, and
  refuses a save that would leave a second, failing the Job. This is about
  how many Versions a Job leaves, never about what it may reach.
- The MCP door speaks revision `2026-07-28`, stateless, no handshake.

## Changing what a recipe holds

A Version's id is the fingerprint of its content, and that is the whole of how
two people who never spoke are known to hold the same recipe (ADR 0004). **A
field holding nothing is no part of it** (ADR 0038): `null`, the empty list and
the empty object are stripped before the hash, so a field added to a recipe
starts out empty on everything already written and moves no existing id.

That makes the ordinary case free and leaves exactly one trap. **Before adding a
field to `parse_recipe_content`, ask what it defaults to on a recipe that
predates it.**

- **Empty (`null` or `[]`) — the normal answer.** Nothing else to do. No id
  moves, no migration, no Bundle already written is orphaned.
- **Anything else** — `0`, `"metric"`, `false` — **re-fingerprints the entire
  library.** Every Version id changes, and every row naming one has to change
  with it: `branches`, `branch_versions`, `readings`, `attempts`,
  `meaning_vectors`. Migration 29 is the worked example and the only time
  Kamosu has done it. Either give the field an empty default instead, or write
  that migration deliberately and say so in an ADR.

`a_field_added_to_a_recipe_moves_no_existing_id` in `src/core/recipes.rs` fails the build
on the second case. A new field also goes into `CONTENT_FIELDS` beside
`parse_recipe_content`, the list `edit_recipe` merges onto the head (#164);
`an_edit_can_change_every_field_a_recipe_holds` fails the build until it does. Two behaviour tests guard the rest: one sweeps a database
carried forward from an earlier schema, which fails if a migration rewrote
`versions.content` without re-fingerprinting it, and one drives every path that
writes a Version and checks the same thing on what the Doors produced. `version_fingerprint(content)`
is registered on every SQLite connection, so the same question can be asked of
a live instance:

```sh
sqlite3 /data/kamosu.db 'SELECT COUNT(*) FROM versions WHERE id <> version_fingerprint(content);'
```

It answers zero from a Kamosu binary, and only from one — the function is
Kamosu's, not SQLite's.

## Meaning Search ships no model

**Kamosu ships no weights** (ADR 0029), so nothing about EmbeddingGemma is in
the image, in the repository, or in a Backup. An Operator accepts Google's
terms through an ordinary Operation at either Door, `download_meaning_model`
fetches ~220 MB into `/data/model/` at a **pinned revision**, and
`build_meaning_index` reads the library. An instance that never does any of
that searches by words and is complete — which is the state every instance
starts in and most stay in.

Two consequences worth knowing before touching `src/meaning.rs`:

- **The index is derived, never truth.** Every vector rebuilds from the recipes,
  which is what makes turning Meaning Search off a thing that discards nothing.
  It is brought level with the library on a short tick rather than inside a
  save, because re-embedding a recipe is seconds of model inference and a save
  is an Immediate Operation.
- **The database lock is never held across inference.** `build` reads what is
  owed, releases, embeds, and takes the lock again to write one recipe's rows.
  Holding it across a first build would freeze the instance for minutes.
- **Exactly one build runs at a time**, held by a mutex on `MeaningSearch`. What
  is owed is read before anything is written, so two builds would find the same
  recipes owing and put every one of them in the index twice.

What it costs an instance that turns it on: about **220 MB** on disk under
`/data/model/`, a few hundred megabytes of memory once the model is loaded, and
a first index build measured in minutes on a small machine — 86 recipes took
several on the dev host. An instance that leaves it off pays none of that: the
model is never loaded and the tick that keeps the index level returns
immediately.

## Where the truth lives

**The database is the truth; the Vault is a published copy** (ADR 0003). This
inverts Hatchdoor, this project's usual precedent — if you know Hatchdoor, do
not assume its Markdown-is-truth rule applies here. Deleting Kamosu's database
loses every account, cooking history and shopping list, and no Vault rebuilds
them.

Vocabulary lives in `GLOSSARY.md`; decisions live in `docs/adr/`. Where the spec
and an ADR disagree, the ADR wins.
