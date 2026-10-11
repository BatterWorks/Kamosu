# A release is cut by two commands, and only the maintainer approves its notes

`just release-prepare <version>` and `just release-publish <version>` cut a release of Kamosu, and nothing else does. Settled on 2026-10-11 on [#234](https://github.com/BatterWorks/Kamosu/issues/234). Until then v0.1.0 had been cut by hand and its steps were not kept, so `dev` ran 39 commits ahead of the only published image and nobody could cut the second release without working the first one out again.

## What was decided

- **Two commands, run on the maintainer's build machine.** `release-prepare` sets the version in `Cargo.toml`, `Cargo.lock`, `ui/package.json` and `ui/package-lock.json`, dates the changelog's `## Unreleased` section, runs `just check` and `just test`, and opens a version-bump pull request into `dev` beside a draft GitHub Release. Run again once the bump has merged, it opens **Release v\<version\>** from `dev` into `main`. `release-publish` merges that pull request, tags the merge, has the release hook build and push the images, and publishes the GitHub Release as latest. Hosted CI plays no part.
- **Kamosu's own version is set by `release-prepare` and by nothing else.** `AGENTS.md` calls a version typed by hand a bug. That rule is about dependencies, and this is its other half.
- **The release is reviewed on the version-bump pull request.** It carries a checklist of the checks only a reader can make. A fix the review needs is committed to the bump branch and pushed by running `release-prepare` again, so it ships in the release that found it. `release-publish` refuses while any box is unticked or reworded.
- **`dev` is frozen from the bump branch until the release has merged.** `release-publish` refuses unless the tip of `dev` is the bump's merge commit and `## Unreleased` is empty, so nothing that merged after the bump reaches `main`. A pull request merged while the bump is still open is not caught by either check, which is why the freeze is a rule and not only a refusal.
- **Only the maintainer approves the title and notes, in the agent's session.** The agent shows the draft as it stands on GitHub and stops. It runs `release-publish` only after the reply "approved" in that same session, and asks again when the text changed or the session is new. No script checks this. It is a rule in `AGENTS.md` and the runbook.
- **Both commands resume.** Each works out where the release stands from git and GitHub on every run, so one run again after a failure carries on at the first step not done. The hook leaves no trace in either, so its success is a marker file under the git directory.
- **Nothing is ever overwritten.** Neither command force-pushes, moves a tag or replaces a published version. A release found broken after publishing is fixed by the next patch version.
- **Everything about the build machines sits behind one hook outside this repository.** `KAMOSU_RELEASE_HOOK` names an executable, called as `<hook> images <version>`. The runbook states its contract and nothing about what is behind it.
- **A release deploys nothing.** `release-publish` ends when the GitHub Release is public.
- **The scripts are Kamosu's own copy.** They began as Hatchdoor's `release-common.mjs`, `release-prepare.mjs` and `release-publish.mjs` with their tests, and live in `ui/tools/`, where Prettier and ESLint cover them.

## Why

- **A procedure kept in someone's memory drifts.** Hatchdoor's hand-cut releases left Docker Hub with two spellings of its version tags and one release whose Podman images were copied from a moving tag. Two commands that refuse on a wrong state do not forget a step.
- **The checklist is where judgement goes.** A script can tell that four files agree on a version. It cannot tell that the README still describes what shipped, or that a library left by the previous release opens under this one. Those are tick boxes, and the second command reads them.
- **The review needs a way to ship its own fixes.** With the checklist on the `dev` to `main` pull request, a stale README found by the review could not reach the release, because by then nothing else may merge into `dev`. Hatchdoor met this and moved the checklist to the bump (its ADR-37). Kamosu starts there.
- **The repository is public and the build machines are not.** A hook with a written contract keeps every host, credential and private registry out of it.

## What differs from Hatchdoor

- **No deploy step.** The instances the maintainer runs follow `dev` on a nightly build with its own health gate and rollback. A release is for everyone else, who pull the published image, so the hook is called for `images` only.
- **No What's new page, no docs-freshness check, no roadmap.** Kamosu has none of the three, so their checks and refusals are left out.
- **The changelog keeps an empty `## Unreleased`.** `release-prepare` dates the section and leaves a new, empty one above it, because a ticket adds its line under that heading in the same commit as its change.
- **Two gates in place of one.** Kamosu has no `just check-full`. The bump must pass `just check` and then `just test`.
- **Five checklist items of Kamosu's own**: the glossary, the previous release's library, `just audit`, `THIRD_PARTY_NOTICES.md` and the Docker Hub overview. The last came with [#235](https://github.com/BatterWorks/Kamosu/issues/235), when the hook took on publishing that page.

## Considered options

- **GitHub Actions.** Rejected: the images are built on machines the public repository must not describe, and a hosted runner would need their credentials.
- **One set of scripts shared with Hatchdoor.** Rejected: a change made for one project would then be able to break a release of the other.
- **Letting "release fix" pull requests into `dev` after the bump.** Rejected, as Hatchdoor rejected it: it reopens the window in which a change ships under a version whose changelog does not list it.

## Consequences

- **A release can only be cut where the hook and the registry's credentials are.** A contributor cannot publish one. That is intended.
- **The approval rests on the agent's instructions.** Nothing on GitHub records when the notes were approved.
- **The bump branch is pushed by `release-prepare` and no other way.** It skips the two gates when the remote branch already holds the local commit, since it pushes only after they pass. A commit pushed by hand makes it refuse.
- **The hook's contract spans two repositories.** A change to it changes both.
- **The checklist's wording is part of the scripts.** `release-publish` reads an item only in its exact text, so changing one means changing `ui/tools/release-common.mjs`, and a bump pull request opened before the change keeps the old wording.
