# Releasing Kamosu

A release is cut by two commands, decided in [ADR 0047](adr/0047-a-release-is-cut-by-two-commands.md):

- `just release-prepare <version>` opens the version-bump pull request with the release checklist, creates a draft GitHub Release, and once the bump has merged, opens the release pull request into `main`. It merges nothing.
- `just release-publish <version>` merges that pull request, tags the merge, has the release hook build and push the images, and publishes the GitHub Release as latest.

An agent carries out the release. The maintainer's one task is to approve the release title and notes, and `release-publish` runs only after that approval (see [Approval](#approval)).

Both commands work out where the release stands from git and GitHub on every run. Running one again after a failure, a crash or a closed terminal carries on at the first step not done. Neither command force-pushes, moves a tag, or replaces a published version.

A release deploys nothing. It is for the people who pull the published image.

## `dev` is frozen while a release is open

**From the moment `release-prepare` cuts the bump branch until `release-publish` has merged the release, nothing else may merge into `dev`.**

Pull requests can still be merged into `dev` during that time, and GitHub will not stop one. What happens depends on when it lands:

- **Merged while the bump pull request is open**, it rides into the release with the bump merge, and no check sees it. It ships under a version that was tested without it. Its changelog line lands in the dated section when the bump merges, so the changelog lists it, but the draft notes were written before it and do not.
- **Merged after the bump**, it makes `release-publish` refuse with `dev tip`.

Say so in every session that shares the repository before running `release-prepare`, and leave other pull requests open until the release is published. The one way into a release after the bump branch is cut is a commit on that branch.

## Before you start

- Run the commands from the repository root on the maintainer's build machine. `release-publish` needs the release hook and the registry's credentials, which exist only there.
- The working tree must be clean, untracked files included. `release-prepare` refuses otherwise. It switches the checkout to the bump branch, so no other session may be working in the same checkout.
- `gh` must be logged in to an account that can push branches and tags, open and merge pull requests, and create releases on the repository.
- `KAMOSU_RELEASE_HOOK` must hold the path to the release hook (see [The release hook](#the-release-hook)). Only `release-publish` reads it.
- `## Unreleased` in `CHANGELOG.md` on `dev` must have entries. With nothing under it there is nothing to release.
- The version is a bare semantic version such as `0.2.0`: no `v`, no suffix, no leading zeros, and higher than the latest `v*` tag. The git tag and the GitHub Release add the `v`. The image tags do not.
- Kamosu's version is set by `release-prepare` and by nothing else. Never edit it by hand in any of the four files.

## The release, start to finish

### 1. Open the version-bump pull request and the draft release

```sh
just release-prepare 0.2.0
```

The first run:

1. Fetches `origin` with its tags, and refuses if the version is not higher than the latest release tag or is already published.
2. Cuts `docs/release-v0.2.0` from `origin/dev`.
3. Sets the version in `Cargo.toml`, `Cargo.lock`, `ui/package.json` and `ui/package-lock.json`, renames `## Unreleased` in `CHANGELOG.md` to `## v0.2.0 - <today>`, leaves a new, empty `## Unreleased` above it, and commits.
4. Refuses unless the four files agree on the version, the dated section has entries, and every `[#N]` in it has a `[#N]: <url>` definition somewhere in the file.
5. Runs `just check`, then `just test`. A cold build takes several minutes, and the output streams to the terminal.
6. Pushes the branch.
7. Opens **Prepare the v0.2.0 release** into `dev`, carrying the [checklist](#the-checklist).
8. Creates a draft GitHub Release for `v0.2.0` targeting `main`, titled `v0.2.0`, with the changelog section as its notes. Link definitions the section uses but does not hold are copied in, so `[#N]` markers are links on the release page.

It prints both addresses and stops.

### 2. Work through the checklist

Do each check named in [The checklist](#the-checklist) and tick its box on the version-bump pull request on GitHub. `release-publish` refuses while any box is unticked, and it reads an item only in its exact wording, so change a box's mark and never its text.

When a check finds something to fix, such as a stale README section, fix it on the bump branch:

```sh
git switch docs/release-v0.2.0
# edit, then
git add <the files you changed or added>
git commit -m 'docs: <what the fix does>'
just release-prepare 0.2.0
```

With the bump open, `release-prepare` opens nothing new. It checks the branch again, runs `just check` and `just test` on the new commit, and pushes it, so the fix merges with the bump and ships in this release.

Push the bump branch only this way, never by hand. `release-prepare` skips the two gates when the remote branch already holds the local commit, because it pushes only after they pass. A commit pushed any other way makes it refuse with `bump branch`.

A fix that adds a changelog entry goes under the dated `## v0.2.0` section, not under `## Unreleased`.

### 3. Draft the release title and notes

Edit the draft GitHub Release until its title and notes are what should go public:

```sh
gh release edit v0.2.0 --title 'v0.2.0' --notes-file <file>
```

The notes start as the changelog section. Rewrite them for readers of the release page where that helps, and include any entry a review fix added. Then tick **The release title and notes are drafted.**

### 4. Merge the version-bump pull request

Once every box is ticked, merge it with a merge commit, like any other pull request into `dev`. `release-publish` checks that the tip of `dev` is exactly this merge, so that `main` receives what `release-prepare` tested.

### 5. Open the release pull request

```sh
just release-prepare 0.2.0
```

The same command, run again, sees that the bump has merged and opens **Release v0.2.0** from `dev` into `main`. Its body links the version-bump pull request and carries no checklist. If the draft release has gone missing, it creates it again from the changelog on `dev`. It prints both addresses and stops, and it opens nothing twice however often it is run.

### 6. Get the maintainer's approval

Give the maintainer the exact title and notes now on the draft, and stop. Wait for the reply "approved". The rule, and the cases where it must be asked again, are in [Approval](#approval).

### 7. Publish

```sh
KAMOSU_RELEASE_HOOK=/path/to/hook just release-publish 0.2.0
```

Before merging, it refuses unless all of these hold:

- The release hook is set and is an executable file.
- **Release v0.2.0** is open from `dev` into `main` with that exact title.
- The version-bump pull request from `docs/release-v0.2.0` has merged, and every checklist item on it is present in its exact wording and ticked.
- The draft release for `v0.2.0` exists, is still a draft, and has a title and notes.
- `## Unreleased` on `dev` has no entries.
- The tip of `dev` is the merge commit of the version-bump pull request.
- No `v0.2.0` tag exists yet.

Then it:

1. Merges the release pull request with a merge commit, pinned to the `dev` tip it checked, so GitHub refuses the merge if `dev` moved in between.
2. Checks that the merge commit is on `origin/main` and that its second parent is the version-bump merge.
3. Creates the annotated tag `v0.2.0` on the merge commit and pushes it.
4. Runs `<hook> images 0.2.0`.
5. Publishes the draft release and marks it latest.

The release goes public at step 5, once the images exist. A machine that keeps a second copy of the repository brings its `main` and `dev` level after this, as it does after any merge.

## The checklist

These are the checks only a reader can make. `release-prepare` puts them on the version-bump pull request as tick boxes, and the agent does each one and ticks it before the bump merges.

- **The changelog section lists everything that shipped since the previous release, and nothing that did not.** Compare the `## v<version>` section against the pull requests merged into `dev` since the last release tag (`git log --merges --first-parent v<previous>..origin/dev`). Nothing listed may have been reverted or replaced, and every change a cook, an Operator or an agent at the MCP Door would notice needs its line. A step someone must take when upgrading sits under `### After upgrading`.
- **The README describes what shipped, and its image-tag examples name this version.** New screens, settings, environment variables and Operations are described, and nothing it describes was removed or changed without an update. An example that names a version tag names this one. `latest` needs no change.
- **No ADR is contradicted by what shipped, and `GLOSSARY.md` holds every word the release added.** An ADR the release went against is amended or superseded, not left stale. A word the interface, the Catalogue or the changelog now uses as a term has its entry.
- **A library left by the previous release opens under this one: every migration runs, and no Version's id differs from the fingerprint of its content.** How to check it is under [The previous release's library](#the-previous-releases-library).
- **`just audit` is clean, or each advisory it reports is recorded in `deny.toml` with its reasoning.** Read what it reports instead of obeying it. A remedy `npm audit` suggests is often a downgrade.
- **`THIRD_PARTY_NOTICES.md` matches the dependencies that ship.** Everything bundled in the repository or downloaded at runtime by an Operator is named there with its licence, and nothing removed since the previous release is still listed. Compare it against `git diff v<previous>..origin/dev -- assets/ Cargo.toml ui/package.json src/meaning.rs`.
- **The release title and notes are drafted.** Step 3.

A check that fails is fixed on the bump branch, as in step 2, and its box is ticked once the fix is pushed. Never tick a box that is not true. A fix merged into `dev` by another pull request after the bump makes `release-publish` refuse, so the bump branch is the only way in.

### The previous release's library

`tests/behaviour.rs` carries a database it wrote itself forward from an earlier schema. A release does the same to a copy of a real library, where a migration meets rows no test thought to write.

1. Get a library the previous release left. On an instance still running that release, take a Backup, copy the zip out of `/data/backups`, and unzip it into an empty folder. The folder then holds `kamosu.db`. An instance that follows `dev` has already been migrated and proves nothing here.
2. On the bump branch, run:

   ```sh
   just release-library <folder>
   ```

It copies the database to a temporary folder, opens the copy as Kamosu does at startup, which runs every migration, and then asks the copy how many Versions sit under an id that is not `version_fingerprint(content)`. It prints the count and fails unless it is zero. The folder you gave it is left as it was.

`version_fingerprint` is registered by Kamosu on its own connections, so the `sqlite3` command-line tool cannot ask this. Never open the database of a running instance with another program.

A count above zero means a migration rewrote a Version's content without giving it a new id. Fix the migration on the bump branch. `AGENTS.md`, "Changing what a recipe holds", says what moves an id.

## Approval

Only the maintainer approves a release's title and notes, and the approval is given in the agent's session. This is a rule the agent follows. No script checks it.

1. After the checklist, the agent gives the maintainer the release title and notes as they stand on the draft GitHub Release, and stops.
2. The agent runs `just release-publish` only after the maintainer replies "approved" to that draft in the current session.
3. A resumed or new session that cannot see that reply in its own conversation asks again.
4. If the title or the notes changed after the reply, the agent asks again.

`release-publish` publishes whatever is on the draft when it runs, so the draft on GitHub must be the text that was approved.

## The release hook

The machines that build the images and the credentials that push them are not described in this repository. One executable outside it handles all of that. `release-publish` knows only this contract:

- `KAMOSU_RELEASE_HOOK` holds its path. A bare command name is not looked up on `PATH`.
- It is called from the repository root as `<hook> images <version>`, with the bare version, such as `0.2.0`. Its output streams to the terminal.
- Exit status 0 means done. Anything else fails the run, and the release stays a draft.
- It must be safe to run again. `release-publish` records a success in a marker under the git directory, `.git/kamosu-release/v<version>/images-done`, and skips the step on the next run. A run on another machine, or after the marker is removed, repeats it.

`images` must:

- Build from the tagged commit, never from a working tree, and pass every build `--build-arg VERSION=<version>` and `--build-arg GIT_SHA=<the tagged commit>`. The Dockerfile writes them to the `org.opencontainers.image.version` and `org.opencontainers.image.revision` labels, which are empty without them.
- Push four tags to `battermanz/kamosu` on Docker Hub, each for amd64 and arm64: `<version>` and `latest` from a Docker build, `podman-<version>` and `podman-latest` from a Podman build. The two pairs are separate builds, not copies of each other.
- Skip a tag that already points at the image it would push, and refuse if a different image sits under the version. A published version is never overwritten.
- Never move `latest` or `podman-latest` backwards. A patch to an older line leaves them where they are.

## When a step fails

Every refusal names its check in brackets, as in `release-prepare refused (changelog links): ...`. Fix the cause and run the same command again. It carries on where it stopped.

### `release-prepare`

| Refusal | What happened | What to do |
| --- | --- | --- |
| `clean working tree` | Local changes, or untracked files, would leak into the bump commit. | Commit, stash or remove them. |
| `version` | Not a bare version, not higher than the latest `v*` tag, or already published. | Pick the next version. A published one is never reused. |
| `changelog` | `## Unreleased` on `dev` is empty, or the bump branch has neither it nor the dated section. | Nothing is ready to release, or the changelog was edited by hand. Fix it through an ordinary pull request first. |
| `changelog` | The dated `## v<version>` section on the bump branch has no entries. | Add the entries on `docs/release-v<version>`, commit, and run again. |
| `changelog` | The bump has merged, but `CHANGELOG.md` on `dev` has no `## v<version>` section. | Something changed the changelog after the bump. Stop and ask the maintainer. |
| `version files` | A version file has no version to set, or the four do not agree on the version after the bump. | Fix the file on `docs/release-v<version>`, commit, and run again. |
| `changelog links` | A `[#N]` in the section has no `[#N]: <url>` definition. | Add the definition at the foot of the section on the bump branch, commit, and run again. |
| `just check` | A formatter, a lint or a freshness gate failed, on the bump or on a review fix committed after it. | Fix it on the bump branch, commit, and run again. Nothing is pushed until it passes. |
| `just test` | A test failed, on the bump or on a review fix committed after it. | The same. |
| `bump branch` | `origin/docs/release-v<version>` holds commits the local branch lacks, pushed by hand, from another clone or through GitHub. | Bring them in with `git pull --ff-only`, run `just check` and `just test` yourself, since `release-prepare` does not test a commit already on `origin`, and run again. |
| `bump pull request` | The version-bump pull request was closed without merging. | Reopen it, or delete `docs/release-v<version>` on `origin` and locally to start the version over. |
| `release pull request` | A pull request from `dev` into `main` is already open under another title. | Finish or close that release first. GitHub allows only one. |

While the version-bump pull request is open, the command opens nothing. It tests and pushes any commit on the local bump branch that the remote does not have, creates the draft release if a crash lost it, and prints both addresses.

### `release-publish`

| Refusal | What happened | What to do |
| --- | --- | --- |
| `version` | The argument is not a bare version such as `0.2.0`. | Pass the version without a `v` or a suffix. |
| `release hook` | `KAMOSU_RELEASE_HOOK` is unset or does not name an executable file. | Set it to the hook's path. |
| `release pull request` | No open or merged pull request from `dev` into `main` is titled exactly `Release v<version>`. | Restore the title if it was edited, or run `just release-prepare <version>`. |
| `release checklist` | An item on the merged version-bump pull request is unticked, or its wording was changed or deleted. | Do the check and tick the box, or restore the item's exact text. The bump has merged, so a check that finds something to fix can no longer ship it in this release: stop and ask the maintainer. |
| `draft release` | Before the merge: the draft has no title or no notes. | Finish drafting it, and get the maintainer's approval again, since the text changed. |
| `draft release` | Before the merge: there is no draft. | Run `just release-prepare <version>` to create it, redraft the title and notes, and get the maintainer's approval again. |
| `draft release` | The release was published before its pull request merged, or the draft went missing after the merge. | `release-prepare` cannot repair either, because the version then counts as published or tagged. Stop and ask the maintainer. |
| `changelog` | `## Unreleased` on `dev` has entries, added after the bump branch was cut. | Those changes are not in this release's notes. Stop and ask the maintainer: they belong to the next version. |
| `dev tip` | The bump has not merged, or something merged into `dev` after it. | Merge the bump, or stop and ask the maintainer, as above. |
| `tag` | A `v<version>` tag exists before the merge, the local and remote tags differ, the tag is not annotated, or it points away from the release merge. | A tag is never moved. Delete it by hand only if the version was never published. Otherwise work out which is right before going further. |
| `merge commit` | The release merge is not on `origin/main`, its second parent is not the version-bump merge, or the release pull request merged but no merged version-bump pull request from `docs/release-v<version>` exists. | This check runs before tagging, so a first run tagged nothing. Look at what landed on `main` before going further, and stop and ask the maintainer. |
| `images` | The hook's `images` step failed. The tag is pushed and the release is still a draft. | Fix the cause from the hook's output and run again. Publishing waits for the images. |

### Errors without a check name

A failed `git` or `gh` call, such as a push, a fetch, `gh pr create` or the merge request to GitHub, is not a refusal. It stops the run with the command that failed and its error output, often with a stack trace. Fix the cause, whether authentication, the network, a conflict or branch protection, and run the same command again. It carries on from where git and GitHub say the release stands.

When GitHub refuses the merge, the usual cause is that `dev` moved after the checks, because the merge is pinned to the tip that was checked. Running again then reports a `dev tip` refusal. A check GitHub requires on `main` that has not finished gives the same kind of error: wait for it and run again.

A release that turns out broken after publishing is never fixed by republishing its version. Release the next patch version.
