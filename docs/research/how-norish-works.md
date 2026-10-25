# How Norish works

**Purpose:** a description of what the self-hosted recipe app Norish does for the people who use it, and how it does it underneath, from its own source, docs, releases and tracker.
**Method:** the source of `norish-recipes/norish` on its default branch `main` at commit `c79a86b3271a7801401e65c88228874c7a648bb6` (2026-10-14, the v0.25.1-beta release), cloned shallowly and read on 2026-10-22; nothing was built or run. Every file path below is at that commit, and `B` in a link stands for `https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6`. The official docs are the Docusaurus site under `apps/docs/docs/` in the same commit, published at [docs.norish.dev](https://docs.norish.dev). Releases, issues, discussions, pull requests, stars and contributors were read through `gh api` on 2026-10-22. The project website [norish.dev](https://norish.dev) and the Docker Hub API were fetched on the same day. Where this note says the code does something, that is a reading of the code, not an observed run.

## What it is and who it is for

The README calls Norish "a real-time, household-first recipe app for planning meals, sharing groceries, and cooking together" ([README.md](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/README.md)). The repository description says "A realtime, self-hosted recipe app for families & friends". The website's tagline is "Any recipe, any source", and it describes "The open-source recipe app for families & friends. Import recipes from any link, plan your meals together, and keep your grocery lists in sync when shopping together" ([norish.dev](https://norish.dev)).

The project glossary describes the scope as "recipes, groceries, stores, and a meal calendar shared across a household, served by a single self-hosted backend with web and mobile clients" (`CONTEXT.md`).

The early README, at tag v0.13.6-beta, gave the origin. The maintainer built it because he and his girlfriend cook and shop together, could not "get used to the aesthetic of Tandoor or Mealie", and needed a list that is "fully-realtime" so that "we can keep track of who picked what grocery items". It also said Norish "is minimalistic by nature" and that he was "not sure yet if I will ever add complex cookbook structures, inventory tracking etc." (`README.md` at [v0.13.6-beta](https://github.com/norish-recipes/norish/blob/v0.13.6-beta/README.md)). Both cookbooks and a pantry shipped in 2026. The name is "Nora (our dog) + dish" (`apps/docs/docs/intro.mdx`).

## History and project

| Fact | Value | Source |
|---|---|---|
| Docker Hub repository registered | 2025-10-31 | Docker Hub API, `norishapp/norish` |
| GitHub repository created | 2025-12-03; first commit on `main` the same day | `gh api repos/norish-recipes/norish`, commits API |
| First release on GitHub | v0.13.1-beta, 2025-12-04 | [v0.13.1-beta](https://github.com/norish-recipes/norish/releases/tag/v0.13.1-beta) |
| Current release | v0.25.1-beta, 2026-10-14 | [v0.25.1-beta](https://github.com/norish-recipes/norish/releases/tag/v0.25.1-beta) |
| Next release in progress | v0.25.2-beta, open as PR [#638](https://github.com/norish-recipes/norish/pull/638) | pulls API |
| Releases | 31 on GitHub in about ten months; 12 minor versions from 0.14 to 0.25, one every two to six weeks | releases API |
| Beta status | every tag carries `-beta`; none is marked as a GitHub prerelease | releases API |
| Licence | AGPL-3.0 | `LICENSE`, repo metadata |
| Stars and forks | 1,371 stars, 123 forks | repo metadata |
| Issues and PRs | 345 issues (62 open), 277 PRs, 45 discussions | search API, GraphQL |
| Commits on `main` | 558 | commits API pagination |
| Contributors | the contributors API lists 3 accounts: `mikevanes` with 555 commits, `claude` with 2, `YONN2222` with 1 | contributors API |
| Outside contributors | release notes thank more than 20 people by handle; among the last 100 closed PRs, 52 merged ones are the maintainer's and 19 come from 13 other accounts | release notes, pulls API |
| Maintainer | Mike van Es (`mikevanes`) | users API, release notes |
| Organisation | `norish-recipes`, created 2025-12-03 | orgs API |
| Funding | Buy Me a Coffee and GitHub Sponsors | `.github/FUNDING.yml` |
| Company | "I am officially registering Norish as a company in the Dutch Chamber of Commerce" | [v0.16.0-beta notes](https://github.com/norish-recipes/norish/releases/tag/v0.16.0-beta) |
| Hosted offering | none; the website says "no subscription, no account with us" | [norish.dev](https://norish.dev), `apps/landing/` |
| Docker pulls | 554,854 for `norishapp/norish` | Docker Hub API |

Versions before 0.13 existed only as Docker images: Docker Hub still lists tags from v0.9.6-beta to v0.12.0-beta, pushed on 2025-12-03 and 2025-12-04. The v0.13.1-beta notes say fixes were "available in version 0.12 and up".

Development is one person's, helped by AI agents. `AGENTS.md` and `CONTEXT.md` are written for coding agents, tickets live as markdown under `.scratch/` with statuses such as `ready-for-agent`, and the maintainer's backlog is a private Linear project (`.scratch/README.md`). The repository has branches named `claude/...`. The maintainer has written twice about pace: in v0.16.0 he said "my work-life balance slipping" and "releases may come a bit slower", and in v0.20.0 "Currently I am very busy at work and can't always find the motivation work at Norish after hours" (`apps/docs/docs/release-notes/0.20.0-beta.md`).

## Architecture

| Part | What it is | Source |
|---|---|---|
| Language | TypeScript on Node.js 22; one small Python 3.14 service | `AGENTS.md`, `docker/Dockerfile` |
| Repository | pnpm workspaces with Turborepo since v0.17.0 | `package.json`, v0.17.0 notes |
| Web app | Next.js 16 App Router, React 19, HeroUI v3, Tailwind CSS v4, TanStack Query, served by a custom Node server | `AGENTS.md`, `apps/web/server/index.ts` |
| API layer | tRPC routers in `packages/trpc`, domain logic in `packages/api` and `packages/shared-server` | `AGENTS.md` |
| Database | PostgreSQL through Drizzle ORM; 74 SQL migrations applied at startup | `packages/db/src/migrations/`, `packages/api/src/startup/migrations.ts` |
| Redis | required since v0.14.0: Better Auth sessions, realtime pub/sub with a resume buffer, BullMQ job queues, the idempotency store | v0.14.0 notes, `docs/adr/realtime/`, `packages/trpc/src/idempotency-middleware.ts` |
| Background jobs | BullMQ with 16 queues, workers started on demand and shut down when idle | `packages/queue/src/queue-names.ts`, v0.15.3 notes |
| Renderer | Obscura, a headless browser engine in its own container, driven over the Chrome DevTools Protocol by `playwright-core` | [ADR-0019](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/imports/0019-obscura-is-the-only-rendered-page-engine.md) |
| Recipe scraper | `recipe-scrapers` 15.12.0 wrapped in FastAPI, started by the Node server inside the same container | `apps/parser-api/pyproject.toml`, `docker/Dockerfile` |
| Video | `yt-dlp` binary and `ffmpeg` in the image | `docker/Dockerfile` |
| AI | Vercel AI SDK v7 with 11 provider adapters, plus TypeSafe's SDK for the Decision Model | `packages/shared-server/package.json` |
| Auth | Better Auth with its generic OAuth, API key and Expo plugins | `packages/auth/src/auth.ts` |
| Mobile | an Expo SDK 55 React Native app, "parked pending a rewrite" | `AGENTS.md` |
| Docs | Docusaurus, versioned per release | `apps/docs/` |

Background queues: recipe import, image import, paste import, CalDAV sync, scheduled tasks, nutrition estimation, auto-tagging, auto-categorisation, allergy detection, recipe provenance, ingredient linking, image generation, store lookup, ingredient review, ingredient icons and new ingredients. Imports and enrichment run two jobs at a time; image calls, store lookups and ingredient rounds run one at a time, because image APIs are "billed per call and rate-limited hard" and the store lookup is "the whole good-citizen fence in front of somebody else's supermarket" (`packages/queue/src/config.ts`).

The scheduled tasks check recurring groceries, sweep orphaned media, refresh the ingredient catalogue nightly, and delete old data. Done groceries and planned meals older than the start of the month three months back are deleted; `SCHEDULER_CLEANUP_MONTHS` sets the three (`packages/queue/src/scheduler/old-calendar-cleanup.ts`, `old-groceries-cleanup.ts`). The meal calendar therefore keeps no long history.

## How clients talk to the server

The web client calls tRPC procedures over HTTP at `/api/trpc` for queries and mutations (`apps/web/app/api/trpc/[trpc]/route.ts`). A WebSocket at `/trpc` on the same port carries subscriptions only (`apps/docs/docs/configuration/websocket.md`, `packages/trpc/src/context.ts`). The server pings every 20 seconds, and since v0.24.0 it refuses a WebSocket from an origin other than `AUTH_URL`, `TRUSTED_ORIGINS` or the request's own host.

Realtime was rebuilt in 2026 ([ADR-0032](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/realtime/0032-realtime-is-one-hub-one-catalogue-one-idiom.md)):

- Every event is declared once in a per-domain catalogue with a scope (`household`, `user`, `policy`, `broadcast` or `internal`) and a zod schema.
- Each server process holds one Redis subscriber connection and fans events out in process. Clients receive an envelope `{ meta, payload }`.
- A publish is written to a per-channel Redis Stream in the same Lua script that publishes it. The stream keeps about 1,000 entries for 24 hours, so a reconnecting client resumes from its cursor ([ADR-0034](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/realtime/0034-resume-is-a-bounded-buffer-not-a-guarantee.md)).
- A client that falls too far behind is told it is "Lagged" and refetches. ADR-0034 calls the buffer "a delivery buffer, not a transactional outbox": an event can be lost between the database commit and the stream write, and the refetch is the guarantee.
- When a user joins, leaves or is kicked from a household, the server closes that user's sockets with code 4000 so every subscription restarts under the new identity ([ADR-0033](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/realtime/0033-a-scope-change-restarts-the-socket.md)).

Every mutation that carries an `x-operation-id` header is claimed in Redis and runs at most once; a repeat returns the stored response. The claim lasts seven days (`packages/trpc/src/idempotency-middleware.ts`, [ADR-0002](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0002-replay-idempotency-via-operation-id-middleware.md)). Clients may mint entity ids on create so queued edits keep pointing at the right row ([ADR-0003](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0003-client-minted-entity-ids-on-create.md)).

A separate public API lives under `/api/v1`, built from a subset of the same tRPC procedures with `trpc-to-openapi` (`packages/trpc/src/openapi.ts`). The glossary calls it "a deliberately small subset of what the app can do, not the app's own data API" (`CONTEXT.md`). Its 19 operations, from the published spec (`apps/docs/docs/reference/openapi.json`):

| Area | Operations |
|---|---|
| Health | `GET /health` |
| Recipes | get by id, search, create, import from a URL, import from pasted text |
| Groceries | list, create, mark done, mark not done, delete, assign to a store |
| Stores | list, create |
| Planned recipes | today, this week, this month, create, delete |

The import endpoint answers `{ recipeId, status: "queued" | "exists" }`, and a `409` means an import of that URL is already running (`apps/docs/docs/release-notes/0.20.0-beta.md`).

## The recipe data model

A recipe row holds a name, description, source URL, servings, prep, cook and total minutes, notes, calories, fat, carbs and protein, meal categories, a provenance group, a dish colour and a `version` integer ([`packages/db-schema/src/schema/recipes.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/db-schema/src/schema/recipes.ts)). Around it sit separate tables for ingredient lines, steps, step ingredients, step images, a gallery of images, videos, tags, cuisines, ratings, favourites and share links.

| Field | How it is kept |
|---|---|
| Owner | `user_id`, set to null when the account is deleted |
| Source URL | unique per owner, so one person cannot hold the same URL twice |
| Categories | an array from a fixed set: Breakfast, Lunch, Dinner, Snack |
| Tags | an open set, shared across the instance, unique by lower-cased name |
| Cuisines | a curated list the administrator owns ([ADR-0012](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/recipes/0012-cuisines-are-curated-tags-are-not.md)) |
| Provenance | ISO country code, the country's name in the recipe's language, a region and a note, never translated |
| Photos | up to 10 gallery images, any number of images per step, and videos; one image may be marked as AI-generated |
| Dish colour | a hex colour taken from the primary image and used to tint the page ([ADR-0023](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/ui/0023-recipe-pages-take-their-hue-from-the-dish.md)) |
| Nutrition | four numbers per serving on the recipe; per-line estimates in `recipe_nutrition_estimates` |
| Headings | an ingredient or step row that starts with `#` |
| Sub-recipes | a step can link another recipe with `/` |

### Ingredient lines

An ingredient line is stored as a text `name`, a numeric `amount`, a text `unit`, an order and a measurement system ([`packages/db-schema/src/schema/recipe-ingredients.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/db-schema/src/schema/recipe-ingredients.ts)). On import, each line goes through the `parse-ingredient` npm library (version 2.2.0) with Norish's own unit dictionary. The import code keeps the library's `quantity`, `unitOfMeasure` and `description`, and stores the description as the line's name ([`packages/api/src/parser/parsers/ingredients.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/api/src/parser/parsers/ingredients.ts)). So "2 onions, diced" is stored as amount 2, no unit, name "onions, diced". The whole original line is not kept as one string. No code at this commit reads the library's `quantity2` field outside the Mealie and Tandoor archive readers, so the upper end of a range such as "1-2 cups" is not stored by this path. Before parsing, a comma between digits becomes a decimal point, and multi-word units such as "cuillères à soupe" are collapsed so the library sees them (`packages/shared/src/lib/helpers.ts`). The open v0.25.2 candidate includes "an ingredient line keeps every word, and its amount" for issue [#635](https://github.com/norish-recipes/norish/issues/635).

The editor works the same way from the other side: ingredients are free text, one per line, "Norish reads the amount and unit as you type" (`apps/docs/docs/recipes/creating-and-editing.md`).

### The ingredient catalogue

Since v0.25.0, every line, grocery and pantry entry points at an **Ingredient Alias**, and every alias at an **Ingredient** ([ADR-0037](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0037-ingredient-identity-is-an-alias-pointing-at-an-ingredient.md)). "onion", "onions", "onion, diced" and the Dutch "ui" are four aliases of one Ingredient. The line keeps its own text; the alias is how Norish knows the food.

- The catalogue is seeded from the Open Food Facts ingredients taxonomy under ODbL and refreshed every night ([ADR-0038](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0038-the-catalogue-is-seeded-from-open-food-facts-under-odbl.md)). It is one catalogue for the whole server.
- A new text is matched by exact spelling, then with preparation, size and leading amounts stripped, then as a plural or diminutive, using per-language word lists the administrator can edit (`CONTEXT.md`, "Ingredient Alias" and "Ingredient Words").
- When nothing matches and AI is on, the Decision Model or the language model decides; only then is a new Ingredient minted, and it is **flagged** for a person to merge or mark distinct.
- Ingredients form a tree. "red onion" is a child of "onion" and borrows its aisle, icon and nutrition until it has its own.
- Each Ingredient has an icon. Norish ships icons for the seeded foods; a person can upload one or have the image provider draw one.

### Units and conversion

A recipe can hold its ingredients and steps in two measurement systems at once. Each line and step row records `metric` or `us`, and `original_system` on the recipe records which one was written first ([`packages/db-schema/src/schema/recipes.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/db-schema/src/schema/recipes.ts)). An edit replaces only the rows of the system being edited (`packages/db/src/repositories/recipes.ts`).

Conversion without AI arrived in v0.25.0. It uses the catalogue: cups become millilitres for foods on a "poured" list such as water, milk, oil and syrup, and grams for any other food whose density the catalogue knows; teaspoons and tablespoons stay; oven temperatures in step text convert too ([`packages/shared-server/src/recipes/measurement-conversion.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/shared-server/src/recipes/measurement-conversion.ts), `apps/docs/docs/recipes/recipe-page.md`). "Convert with AI" remains as an option. A reader can choose a system that every recipe they may edit converts to as it opens. In a discussion in February the maintainer explained why it had needed AI: "1cup of flower is not the same as 1cup of feathers ... we need to create a huge standardized set of products" ([discussion #250](https://github.com/norish-recipes/norish/discussions/250)).

### Linking ingredients to steps

A **Step Ingredient** links a step to one ingredient line with a fractional `share`: half the water is 0.5 ([`packages/db-schema/src/schema/step-ingredients.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/db-schema/src/schema/step-ingredients.ts)). The displayed amount is worked out when shown, as share times the line's current amount, so it follows edits, servings and the measurement system. An amount typed as "3 of the 5 eggs" is stored as 0.6. The share column is unconstrained numeric because a four-decimal column turned "150 of the 650 g" into 150.02 g ([#550](https://github.com/norish-recipes/norish/issues/550)). The step's text is never rewritten.

In the editor, typing `@` in a step picks an ingredient, and a picker adds one without naming it in the text. AI can fill steps that have no links; it never removes or replaces a link a person made (`apps/docs/docs/recipes/step-ingredients.md`). This replaced an earlier Cooklang-style `@salt{2 g}` syntax in step text, retired in v0.20.0 without migration.

### Nutrition

Since v0.25.0, a recipe without stated nutrition gets calories, fat, carbs and protein worked out from its lines without AI (`apps/docs/docs/recipes/nutrition.md`, [ADR-0039](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0039-ingredient-nutrition-comes-from-vendored-open-datasets-corrected-per-household.md)):

- Each line becomes grams by its weight, by the weight of one piece when counted, or by the weight of a spoonful when measured by volume. A weight in brackets wins, as in "1 can (400 g) tomatoes".
- Values per 100 g come from CIQUAL 2025, CALNUT, USDA FoodData Central and CoFID, vendored into the repository.
- A household can correct an ingredient's values, piece weight or spoon weight, and only that household sees the correction.
- Pinches and "to taste" are not counted, and the card lists every line left out and why.
- With AI on, lines the datasets cannot count are estimated one by one. When Norish has data for more than 80% of the lines, it does not call AI at all (`apps/docs/docs/recipes/enrichment.md`).

## People and sharing

### Accounts and roles

The first person to sign in becomes the owner and admin, and registration then turns off (`apps/docs/docs/quick-start.mdx`). `ENABLE_REGISTRATION` defaults to `false`. There are two roles: one Owner and any number of Admins; v0.22.0 added a Users screen where an admin can grant admin and delete accounts (`apps/docs/docs/configuration/users.md`). User emails are stored encrypted with keys derived from `MASTER_KEY` ([ADR-0022](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/imports/0022-recipe-archives-are-portability-not-backup.md)). Sessions live in Redis; since v0.22.0 each request checks the user still exists, after a bug let deleted accounts keep using open sessions.

### Households

A person belongs to at most one household. Creating or joining one while already in one fails with "You are already in a household. Leave it first" ([`packages/trpc/src/routers/households/households.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/trpc/src/routers/households/households.ts)). A household has a name, one admin and a 6-digit join code that expires after 10 minutes (`packages/db/src/repositories/households.ts`). The household admin can kick members. OIDC group claims can make members join a household automatically. The maintainer said of one-to-many households: "Originally I had it so one user could join multiple households, but I removed that as I didn't see the need" ([#8](https://github.com/norish-recipes/norish/issues/8)).

The household is what the grocery list, stores, aisles, pantry and calendar share. Each grocery and planned meal row belongs to a user, and lists are read across every user in the caller's household (`packages/trpc/src/routers/groceries/groceries-helpers.ts`, `packages/trpc/src/middleware.ts`).

### Who may view, edit and delete

Recipes follow one server-wide policy, set by an administrator, with a separate level for view, edit and delete: `everyone`, `household` or owner only (`apps/docs/docs/configuration/admin-settings.md`). The default is view by everyone on the server, edit and delete by the owner's household ([`packages/config/src/zod/server-config.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/config/src/zod/server-config.ts), line 844). The owner and any server admin always pass the check ([`packages/auth/src/permissions.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/auth/src/permissions.ts)). There is no per-recipe setting. Cookbooks reuse the same policy ([ADR-0027](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/cookbooks/0027-cookbooks-reuse-the-recipe-permission-policy.md)). Ingredients have their own policy for editing what someone else added, with `household` as the default.

### A deleted account

Deleting an account deletes its groceries, meal plan, cookbook memberships and preferences. Its recipes and cookbooks stay and become **Orphaned**: "every reader may see it, edit it and delete it, under every view policy including the strictest, where it was private a moment earlier" (`CONTEXT.md`). The glossary calls that "the price of the guarantee" that "no departure quietly empties a shared Library". A household whose admin leaves passes to a remaining member (`apps/docs/docs/configuration/users.md`).

### Sharing outside the household

Since v0.18.0 a recipe can get a public link. The token is 24 random bytes, stored only as a hash, shown once, and expires after one day, week, month or year, or never (the default) (`packages/shared/src/contracts/zod/recipe-shares.ts`, `packages/db/src/repositories/recipe-shares.ts`). The page omits favourites, rating, provenance and cooking mode. Users manage their links from the user menu, and admins see every link. Between households on one server the policy above is the only mechanism; sharing between households is open as [#8](https://github.com/norish-recipes/norish/issues/8) with `status:planned`. Between servers, the Recipe Archive carries recipes.

## Recipe changes over time

Norish keeps no history of a recipe. There is no version or revision table; an edit replaces the recipe's ingredient, step, tag and image rows in place (`packages/db/src/repositories/recipes.ts`). The only earlier state kept is the original measurement system beside a converted copy.

The `version` integer on recipes and on most other tables is an optimistic-concurrency token, not a history. An update names the version it read; the repository adds `WHERE version = ?` and increments it, and a write that matches nothing is "stale" (`packages/db/src/repositories/mutation-outcomes.ts`). For a recipe, the `update` procedure returns `{ success: true }` before the write runs, then logs and drops a stale write without publishing an event ([`packages/trpc/src/routers/recipes/recipes.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/trpc/src/routers/recipes/recipes.ts), lines 252 to 292). So when two people edit at once, the first save wins and the second is discarded. For edits queued offline the rule is written down as "first write wins": the dropped change is parked as **Conflicted** so the person can reapply it, and "Offline edits always lose to concurrent online edits, even on non-competing fields" ([ADR-0004](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0004-first-writer-wins-on-replay-conflicts.md)).

There is no copy or fork of a recipe; the recipe router has no such procedure, and "Duplicate" in the interface applies to planned meals. Recipe variations are a discussion idea ([discussion #569](https://github.com/norish-recipes/norish/discussions/569)), and a cooking log per recipe is an open request ([#564](https://github.com/norish-recipes/norish/issues/564)) with no maintainer reply.

## Cooking

Cooking mode arrived in v0.19.0. It shows one step per screen with that step's linked ingredients and amounts; on a phone, swiping up or down changes step and sideways shows the ingredient list (`apps/docs/docs/recipes/recipe-page.md`). The bottom bar holds progress, timers, ingredients, a keep-screen-on button and the next step. It also shows **Ready At**, the time cooking mode opened plus the recipe's total time.

- **Keep-awake** uses the browser's Screen Wake Lock API through a context in `apps/web/app/(app)/recipes/[id]/components/wake-lock-context.tsx`. It changed from a toggle to a button in v0.25.0.
- **Servings** scale every line, and step amounts follow.
- **Timers** are found in step text by [`packages/shared/src/lib/timer-parser.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/shared/src/lib/timer-parser.ts). It looks for a number followed by a keyword from per-language lists of hour, minute and second words that the administrator can edit (`packages/config/src/timer-keywords.default.json`). It also reads `1:30` style times, taking a following keyword to decide between hours and minutes. When another number sits just before the match, as in "12 to 15 minutes" or "12 tot 15 minuten", it treats the pair as a range and uses the larger value. Timers persist in browser storage under `norish-timers` and raise a notification through the service worker when they end (`apps/web/stores/timers.ts`).
- **Ratings** are 1 to 5 per person, and recipes show the average (`packages/shared/src/contracts/zod/ratings.ts`, `packages/db/src/repositories/ratings.ts`). Favourites are per person too.

A **Cooking Session** is never stored: "nothing about it is written down, so a session is never resumed, never shared with the household, and never outlives the screen it runs on. Reopening cooking mode begins a new session at the first step" (`CONTEXT.md`). Ready At is "a projection and never a promise", which the glossary gives as the reason it appears only inside cooking mode. The docs add: "Closing cooking mode ends the session, and opening it again starts at the first step."

## Planning and shopping

### The calendar

Planned items have a date, a slot (Breakfast, Lunch, Dinner or Snack), a sort order and either a recipe or a free-text note (`packages/db-schema/src/schema/planned-items.ts`). There is no servings field on a planned item. The calendar was rebuilt in v0.16.0, an upgrade that "permanently deleted" all existing calendar data. It can pick a random recipe from a category, weighted slightly towards liked and highly rated recipes. Each person can push their plan to a CalDAV calendar, with a time range per slot and encrypted credentials (`packages/db-schema/src/schema/caldav-config.ts`). After planning a recipe, Norish by default opens the groceries panel; a preference can add the ingredients straight away or do nothing (`apps/docs/docs/groceries/planned-recipes.md`). A meal-plan generator is requested in [#289](https://github.com/norish-recipes/norish/issues/289), where the maintainer wrote "I can see the usefulness for this indeed will plan it."

### The grocery list

A grocery row holds its text as written, an amount, a unit, an optional purchase amount, a done flag, a store and links to the recipe line or recurring item it came from (`packages/db-schema/src/schema/groceries.ts`). Adding groceries merges into an existing undone row when the normalised name, the source recipe line, the recurring source and the unit all match, and adds the amounts (`packages/trpc/src/routers/groceries/groceries-helpers.ts`). Lines from different recipes stay separate rows, and the client groups rows with the same name and unit within one store and aisle for display, so "300 g and 400 g of flour count as 700 g" (`packages/shared/src/lib/grocery-grouping.ts`, `apps/docs/docs/groceries/prices.md`). Sub-recipe ingredients are not added; that is open as [#332](https://github.com/norish-recipes/norish/issues/332).

Recurring groceries take a rule such as "every other week on monday", read with per-language word lists, and come back on the list when due (`packages/config/src/recurrence-config.default.json`, `packages/db-schema/src/schema/recurring-groceries.ts`).

### Stores, aisles and prices

- A **Store** is a heading the household names, colours and orders. Groceries assigned to none are "Unsorted" and shown first.
- **Aisles** (v0.23.0) sit inside a store. An aisle is remembered per store and per Ingredient, so filing "melk" files every spelling of milk at that store; Norish never guesses an aisle from words ([ADR-0031](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0031-an-aisle-is-a-fact-about-a-name-at-a-store.md)).
- **Prices** (v0.23.0): a store can point at a shop's search page with a `{query}` slot. Norish reads product pages one visit at a time, links a grocery to a product only when the product's name is the grocery's name, allowing one typo in names of five letters or more and two in names of nine or more, and refreshes a price when it is more than twelve hours old and someone looks at the list (`apps/docs/docs/release-notes/0.23.1-beta.md`, `0.23.0-beta.md`). A row shows "as many whole packs as its amount needs" ([ADR-0029](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0029-line-cost-is-pack-arithmetic-over-what-the-shop-presents.md)). The Decision Model, when set up, picks among the shop's products when names do not match.

### The pantry

The pantry (v0.24.0, its own page in v0.25.0) is one list of Ingredients per household, with no amounts or dates (`apps/docs/docs/groceries/pantry.md`). It is consulted in one place: when a recipe is added to the groceries, lines whose food is in the pantry are shown apart under "In your pantry" and left off unless ticked. A pantry food covers a recipe line for the same Ingredient in any spelling or language, or for a more general food: "red onion" covers "onion", but "onion" never covers "red onion", and "salt" never covers "salted butter" ([ADR-0036](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/groceries/0036-a-pantry-ingredient-is-a-name-matched-exactly.md), `CONTEXT.md`). Groceries typed by hand are never checked against it.

### Live sync

Every grocery change publishes a household-scoped realtime event, so a check-off appears on the other shopper's phone without a refresh. This was the founding requirement in the early README. A check-off made offline or on a flaky connection waits in the outbox (see offline below); v0.24.0 fixed a case where such a check-off "came undone" because nothing replayed it ([#565](https://github.com/norish-recipes/norish/issues/565)).

## Getting recipes in and out

### From a web page

A URL import runs as a BullMQ job ([`packages/api/src/parser/index.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/api/src/parser/index.ts)):

1. A video URL goes to video import instead.
2. Obscura renders the page and returns its HTML. The user's site authentication tokens for that domain go with it.
3. With "always use AI" on, the AI reads the page.
4. Otherwise the HTML goes to `recipe-scrapers`, first in its supported-sites mode, then in "wild" mode that reads schema.org data from any site (`apps/parser-api/app/main.py`).
5. If the Decision Model judges that parse incomplete, AI extraction runs and wins if it finds a recipe.
6. If the scraper finds nothing and the site answered 401, 403 or similar, the import fails with "refused the request".
7. If AI is on and the Decision Model or a heuristic judges the page likely to be a recipe, AI extraction runs.

Obscura is a separate container built by the project from upstream's stealth release. It blocks trackers and refuses loopback and private-network addresses at DNS resolution, which ADR-0019 calls "the only thing standing between an authenticated URL import and the deployment's internal network". A failed render fails the import; there is no second engine. Since v0.25.1, Obscura needs a shared `OBSCURA_CDP_TOKEN` of at least 32 characters. About a third of one user's imports from Cloudflare-protected sites fail with the challenge page ([#641](https://github.com/norish-recipes/norish/issues/641)); the maintainer replied he "Will look into just a plain fetch".

**Site authentication tokens** let a user save a cookie or header for a domain, encrypted at rest, so imports reach pages that need a login. Since v0.22.0 several accounts per site can be saved, and each import picks one at random to spread load (`apps/docs/docs/configuration/site-authentication.md`). Before v0.22.0 every token was sent with every import, "so an Instagram session cookie travelled to unrelated recipe blogs".

Other ways in: pasting a link into the dashboard search bar, a prompt to import a copied link in Chromium browsers (v0.24.0), and the community [Norish Quick Import Extension](https://github.com/AfoxDesignz/Norish-Quick-Import-Extension) for the browser.

### From video, photos and text

- For a video, `yt-dlp` downloads the video, the transcription provider turns its audio into text, and the AI reads the transcript together with the post's description (`packages/api/src/video/processors/generic.ts`). YouTube "Prefers captions + description, falls back to audio transcription", and Instagram and Facebook read the post's caption, asking import triage whether it holds a recipe before transcribing (`packages/api/src/video/processors/`). No video frames are read. The video itself is saved with the recipe. Videos over 120 seconds are refused by default (`VIDEO_MAX_LENGTH_SECONDS`). Thirteen platforms count as video links, from YouTube and TikTok to Bilibili and RedNote.
- Up to 10 photos or screenshots become one recipe, read by the AI (`packages/shared/src/contracts/zod/recipe.ts`, `apps/docs/docs/recipes/importing.md`).
- Pasted plain text goes to the AI; JSON-LD is parsed without AI, up to 25 recipes per paste (`packages/queue/src/paste-import/parser.ts`).

Without an AI provider, photo import is hidden and plain text cannot be read.

### From other apps

The **Recipe Archive** import reads Norish's own `.norishrecipes` files and exports from Mela, Paprika, Mealie (current and older formats), Tandoor and Recipe Keeper (`packages/shared-server/src/archive/`, `apps/docs/docs/recipes/recipe-archive.md`). Imported recipes become the importer's. A recipe matching one in the household by URL, or by name when either lacks a URL, is overwritten, not duplicated; since v0.25.1 two recipes with the same name and different URLs are both kept. Collections come in as cookbooks by title. v0.25.1 added linked step ingredients from Mealie and Tandoor, and favourites from Mela and Mealie.

### Out

Export arrived in v0.21.0 after [#313](https://github.com/norish-recipes/norish/issues/313), which had the most reactions of any issue. A user exports every recipe they can see as one `.norishrecipes` zip with photos and videos; an admin can export the whole server ([ADR-0022](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/imports/0022-recipe-archives-are-portability-not-backup.md)). The archive carries the author's display name, the exporter's own rating and favourite, cookbook titles and a `formatVersion: 1` manifest, and streams to the browser without being stored on the server. It carries no accounts, emails, plans, groceries or settings, and the docs say it "is **not** a backup". In February the maintainer had planned "the official https://schema.org/Recipe format so that any app should work with it"; the format that shipped is Norish's own. There is no print layout; it is requested in [#505](https://github.com/norish-recipes/norish/issues/505) and an open PR [#650](https://github.com/norish-recipes/norish/pull/650) adds one.

### Backups

Norish has no backup feature. Nothing in the source runs `pg_dump` or archives the uploads folder. The release notes for v0.17.0 and v0.25.0 tell operators to back up the PostgreSQL database and the uploads folder themselves before upgrading.

## AI and agents

AI is off by default, and "Most of Norish works without AI" (`apps/docs/docs/configuration/ai-provider.md`). It is set with `AI_ENABLED`, `AI_PROVIDER`, `AI_MODEL`, `AI_API_KEY` and `AI_ENDPOINT`, or in the admin screens. The providers are OpenAI, Ollama, LM Studio, a generic OpenAI-compatible endpoint, Perplexity, Azure, Mistral, Anthropic, DeepSeek, Google and Groq (`packages/config/src/zod/server-config.ts`, line 292). Norish asks for `response_format: json_schema` and falls back to `json_object`; an endpoint supporting neither does not work. The default model for new installs is `gpt-6-luna`.

All model calls go through one runtime with four entry points: structured generation, transcription, image generation and Decisions ([ADR-0015](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/ai/0015-norish-issues-every-model-request-through-one-runtime.md)). There are 14 prompts, each editable by the administrator; a feature appends its input after the prompt rather than filling placeholders, so an edited prompt keeps working when inputs change ([ADR-0016](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/ai/0016-prompts-are-appended-to-never-interpolated.md), `packages/shared-server/src/ai/prompts/`). A model that refuses a parameter such as `temperature` gets the request again without it ([ADR-0014](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/ai/0014-norish-never-claims-to-know-a-models-parameters.md)).

### What AI does

| Feature | What it does | Default for new recipes |
|---|---|---|
| Import | reads pages the scraper cannot, photos, pasted text and video transcripts | runs when AI is on |
| Auto-tagging | adds tags, never removes any | off |
| Allergy detection | adds allergy tags for allergies set in the household | on |
| Auto-categorisation | sets Breakfast, Lunch, Dinner or Snack on recipes with none | off |
| Nutrition estimation | fills lines the datasets cannot count | off |
| Recipe provenance | country, region, cuisines and a note | off |
| Ingredient linking | links ingredients to steps that have none | off |
| Image generation | draws a picture for a recipe with no image | off |
| Unit conversion | an alternative to the built-in conversion | on request |
| Ingredient upkeep | proposes clean names, decides where a new name belongs, draws ingredient icons | off |

Source: `apps/docs/docs/configuration/ai-provider.md`, `apps/docs/docs/recipes/enrichment.md`.

Enrichment runs after a recipe is saved and "cannot delay a save, undo one, or turn a successful import into a failure" (`apps/docs/docs/release-notes/0.20.0-beta.md`). Anything a person entered or a source stated wins over an automatic run. The trigger is an event published after commit and not persisted, so "a brief process or Redis interruption can therefore miss enrollment by accepted design" (`CONTEXT.md`). Each enrichment can also be run by hand from a recipe's menu, and an admin can run all enabled ones over the whole library. Image generation has its own provider settings, because many text providers cannot draw; a generated image always takes the recipe's primary slot ([ADR-0025](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/recipes/0025-a-generated-image-takes-the-primary-slot.md)).

### The Decision Model

Since v0.24.0 an optional second provider answers closed questions with a probability for each option. Only TypeSafe AI's API is supported, with the model `jev-latest` ([ADR-0035](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/ai/0035-decisions-are-the-runtimes-fourth-entry-point.md)). It categorises, tags under the predefined-tags strategy, asks one question per household allergen, picks the provenance country, picks shop products, checks whether a page holds a recipe before AI extraction, and checks every enrichment's claims before they are written. When it is unsure or fails, Norish falls back to the language model. An open PR adds an OpenAI-compatible decision provider ([#656](https://github.com/norish-recipes/norish/pull/656)).

### API keys

A user creates API keys under Settings, User, API keys and sends one as `x-api-key` or `Authorization: Bearer` (`apps/docs/docs/reference/api.mdx`). Keys come from Better Auth's API key plugin:

- A key is created with a name only, `expiresIn: null`, and no permissions (`packages/db/src/repositories/api-keys.ts`). There are no scopes: a key acts as its user.
- The plugin is configured with `enableSessionForAPIKeys: true` and a rate limit of 500 requests per hour (`packages/auth/src/auth.ts`, line 448). The table's column defaults say 100 requests per minute (`packages/db-schema/src/schema/auth.ts`).
- The `/api/v1` handler and the internal `/api/trpc` handler build their context with the same `createHttpContextFromHeaders` (`packages/trpc/src/context.ts`, `packages/trpc/src/openapi.ts`).

Keys can be disabled, enabled and deleted. Before v0.25.0 there was no documentation; in [discussion #400](https://github.com/norish-recipes/norish/discussions/400) the maintainer answered "You can call the import endpoint `/api/import` I believe", and then promised "a large api rework incoming which will include scalar documentation". The reference now lives on the docs site.

### MCP

Norish has no MCP server in its released code. Issue [#120](https://github.com/norish-recipes/norish/issues/120), "MCP Server Support & JSON Format for import", asked for a server for Claude Desktop and for documentation of the JSON format. The maintainer's only reply addressed the second part: "The JSON is a universal standard following https://schema.org/Recipe". He closed the issue as completed on 2026-01-04 without saying anything about MCP.

Two community efforts exist:

- [Kellojo/norish-mcp](https://github.com/Kellojo/norish-mcp), MIT, 4 stars, created 2026-07-09, wraps the public API in 19 tools: recipes, imports, the meal plan, groceries, stores and health.
- Open PR [#657](https://github.com/norish-recipes/norish/pull/657), "feat(mcp): add a permission-scoped MCP server", from an outside contributor on 2026-10-19. It adds Streamable HTTP at `/api/mcp` with seven tools: `search_recipes`, `get_recipe`, `create_recipe`, `update_recipe`, `delete_recipe`, `list_groceries` and `plan_meal`. It authenticates with the user's API key, applies the same permission helpers as the tRPC API, and limits each user to 120 requests a minute. It has no review or maintainer comment yet.

An in-app assistant is requested in [#318](https://github.com/norish-recipes/norish/issues/318); the maintainer wrote "At the very least I want to include a recipe maker but a full assistant might be useful."

## Search

Search builds a PostgreSQL full-text document at query time from the fields asked for, with weights: title A, tags B, ingredient names C, description and steps D ([`packages/db/src/repositories/recipes.ts`](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/packages/db/src/repositories/recipes.ts), lines 377 to 446). It uses the `simple` configuration, so there is no stemming or language handling. Each typed word becomes a prefix match, the words are joined with OR, and results rank by `ts_rank`. The default fields are title and ingredients. There is no stored search index. Filters cover tags, categories with AND or OR, minimum rating, maximum time and sort order (`packages/shared/src/contracts/zod/recipe.ts`). A separate name search, `searchRecipesByName`, uses a plain `ILIKE` on the name. Offline, only queries already cached answer: "a novel search term while Offline finds nothing" ([ADR-0001](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0001-web-offline-is-a-persisted-query-cache.md)).

## Offline and mobile

The web app is a PWA with a Serwist service worker ([ADR-0006](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0006-serwist-owns-the-service-worker.md)). Offline support arrived in v0.20.0 and targets "flaky connectivity and backend-down page loads, explicitly not multi-day local-first" (ADR-0001). The design persists the TanStack Query cache to IndexedDB instead of building a local database; an earlier attempt at a domain database of about 31,000 lines was abandoned.

**What is cached.** A Cache Warmer keeps a guaranteed **Warm Set** on the device: the 50 most recent recipes in full with their primary image (`WARM_FULL_RECIPE_COUNT = 50` in `apps/web/lib/query-cache/warm-set.ts`), all groceries, recurring groceries and stores, every cookbook the reader can see with its membership, and the calendar around today (`CONTEXT.md`, `apps/docs/docs/offline.md`). A recipe the user creates joins the set at once. Other pages opened while online stay best-effort. A page outside the cache shows an "offline unavailable" screen. The backend counts as unreachable after five seconds.

**The outbox.** Any mutation that cannot reach the server is held in an IndexedDB outbox and shown as Queued, and "Queued means durably stored" ([ADR-0009](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0009-offline-guarantees-are-end-to-end.md)). It guarantees:

- Replay in order once the server answers, run by one tab at a time through the Web Locks API (`apps/web/lib/outbox/leader.ts`).
- At most one execution per operation, through the server's seven-day idempotency claim.
- A server error is retried three times with backoff from 2 to 30 seconds, then the entry is parked as "Needs attention" (`apps/web/lib/outbox/replay.ts`). A create that is parked parks the edits that depend on it. Parked entries are never dropped silently.
- A change that lost to a newer server write is parked as Conflict and can be retried by hand.
- Signing out discards queued changes, after a warning; the cache and outbox are kept per user ([ADR-0005](https://github.com/norish-recipes/norish/blob/c79a86b3271a7801401e65c88228874c7a648bb6/docs/adr/offline/0005-offline-security-posture.md)).

The service worker has no Background Sync handler (`apps/web/app/sw.ts`), so replay happens when the app is open again. For groceries ADR-0009 supports two offline actions, creating groceries and checking them off, and accepts that a follow-up edit to a grocery created offline "can fail if the server later merges that create into another row".

**Native apps.** There are none released. An Expo app was started in v0.17.0 and targeted iOS first; the maintainer wrote in March that Android "won't work as we rely heavily on expo-swift-ui" ([discussion #391](https://github.com/norish-recipes/norish/discussions/391)). It is now "parked pending a rewrite" and "expected to break at compile and runtime against the current release" (`AGENTS.md`). The website lists "Native apps soon". A share target for the PWA is requested in [#570](https://github.com/norish-recipes/norish/issues/570); the web manifest has none.

## Languages and translation

The interface ships in 16 locales: English, Dutch, German in formal and informal forms, French, Spanish, Russian, Korean, Norwegian, Polish, Danish, Italian, Brazilian Portuguese, Bulgarian, Finnish and Ukrainian (`packages/i18n/src/messages/`, `apps/docs/docs/configuration/localization.md`). Translations come from contributors through PRs; Swedish is an open PR ([#664](https://github.com/norish-recipes/norish/pull/664)). `DEFAULT_LOCALE` and `ENABLED_LOCALES` limit the choice. Each locale also has unit names, timer words, recurrence words and ingredient words, because the parsers read text in every language at once; a recipe's language is not recorded (`CONTEXT.md`, "Ingredient Words").

Recipes themselves are not translated. Provenance text is written in the recipe's language and "never translated". Ingredient Aliases do cross languages for matching, so a pantry entry covers a recipe naming the food in another language, and a food shows in the reader's language when a translation exists (`apps/docs/docs/groceries/pantry.md`).

## Deployment and operations

The stack is four containers: `norishapp/norish`, PostgreSQL (`postgres:17-alpine` in the examples), Redis, and `norishapp/obscura` (`apps/docs/docs/quick-start.mdx`). The app image is built on `node:22-alpine` and adds `ffmpeg`, `vips`, a bundled Python for the scraper, and `yt-dlp`; it is published for amd64 and arm64 as `latest`, a version tag and `rc-v...` release candidates (`docker/Dockerfile`, `.github/workflows/`). The container starts as root, gives the uploads folder to UID 1000, and then runs the server as that user through `su-exec` (`docker/entrypoint.sh`). Since v0.25.0 a compose `user: "1000:1000"` line is no longer needed. The image has a health check on `/api/v1/health`.

| Setting | Purpose |
|---|---|
| `DATABASE_URL` or `DATABASE_HOST` and friends | PostgreSQL |
| `REDIS_URL` | events, sessions and jobs |
| `MASTER_KEY` | required; derives the keys that encrypt emails, tokens and secrets, and changing it makes them unreadable |
| `AUTH_URL`, `TRUSTED_ORIGINS` | public address, callbacks and WebSocket origin check |
| `OBSCURA_ENDPOINT`, `OBSCURA_CDP_TOKEN` | the renderer |
| `UPLOADS_DIR` | images and videos on disk, 10 MB per image and 100 MB per video by default |
| `INGREDIENT_CATALOGUE_URL` | the nightly Open Food Facts fetch, or a mirror, or empty to turn it off |

Source: `apps/docs/docs/configuration/server-runtime.md`, `parser.md`, `database.md`.

Environment variables seed the stored configuration when a setting is missing, and the admin screens edit the stored values after that (`packages/api/src/startup/seed-config.ts`). This is why a changed default "only applies to a new installation" (`apps/docs/docs/release-notes/0.24.0-beta.md`).

**Authentication** offers email and password, any OIDC provider, GitHub and Google (`apps/docs/docs/configuration/authentication.md`). OIDC claim mapping can grant the admin role from a `norish_admin` group and join households from groups prefixed `norish_household_`. Sign-in is rate-limited to 20 requests per 60 seconds by default. Since v0.25.0, a password account can no longer start signing in through OIDC, Google or GitHub under the same email, after an account takeover advisory in Better Auth. LDAP is requested in [#42](https://github.com/norish-recipes/norish/issues/42) with `status:planned`.

**Upgrades** run database migrations and one-off backfills at startup (`packages/api/src/startup/`). Several releases changed the compose file by hand: v0.13.6 made a headless Chrome mandatory, v0.14.0 made Redis mandatory, v0.21.0 replaced Chrome with Obscura "with no alias and no deprecation period", and v0.25.1 made `OBSCURA_CDP_TOKEN` mandatory. The Obscura tag is pinned, so `docker compose pull` does not move it. v0.16.0 deleted all calendar data.

## What users ask for and complain about

Reaction counts are low across the tracker: the most-reacted issue ever has 6 reactions. The requests below are ranked by reactions and by comments, with the maintainer's own words where he answered.

| Request or complaint | State | What the maintainer said |
|---|---|---|
| Export recipes, [#313](https://github.com/norish-recipes/norish/issues/313), 6 reactions | shipped in v0.21.0 | "Export has been implemented a while back." |
| Public link for a recipe, [#46](https://github.com/norish-recipes/norish/issues/46), 6 | shipped in v0.18.0 | |
| Cookbooks, [#471](https://github.com/norish-recipes/norish/issues/471), 5 | shipped in v0.22.0 | "Initially I thought I wouldn't do this but I do have a plan to include these actually." |
| Ingredients as first-class records, [#469](https://github.com/norish-recipes/norish/issues/469), 3 | shipped in v0.25.0 | "probably needs a round of refinement" |
| Save videos and images to the recipe, and keep failed imports, [#45](https://github.com/norish-recipes/norish/issues/45), 3 | open since 2025-12-06 | "I will add this! I like the idea"; videos are now saved, keeping a failed import is still open |
| Building from source needed a paid HeroUI Pro licence, [#481](https://github.com/norish-recipes/norish/issues/481), 3 | fixed in July | "hero-ui pro is completely removed" |
| Several households per user and sharing between them, [#8](https://github.com/norish-recipes/norish/issues/8) | open, planned | "I will actually look into this. I like the idea." |
| Meal-plan generator, [#289](https://github.com/norish-recipes/norish/issues/289) | open | "will plan it" |
| Week view in the calendar, [#542](https://github.com/norish-recipes/norish/issues/542); meal-plan queue, [#557](https://github.com/norish-recipes/norish/issues/557) | open | |
| Print layout, [#505](https://github.com/norish-recipes/norish/issues/505) | open, PR [#650](https://github.com/norish-recipes/norish/pull/650) | listed as planned in v0.20.0 |
| PDF import, [#175](https://github.com/norish-recipes/norish/issues/175), [#637](https://github.com/norish-recipes/norish/issues/637) | open | listed for v0.17.0 in the v0.16.0 notes, not shipped |
| Bring! and other external lists, [#156](https://github.com/norish-recipes/norish/issues/156), [#422](https://github.com/norish-recipes/norish/issues/422) | open, planned | |
| SQLite instead of PostgreSQL and Redis, [#366](https://github.com/norish-recipes/norish/issues/366) | closed | "As for Redis this is too integrated, with BullMQ for background processing. So Redis will remain a mandatory dependency." |
| Slow scrolling on a list of 180 recipes, [#110](https://github.com/norish-recipes/norish/issues/110) | open since 2025-12-20 | the reporter found it "Still very laggy" after two fixes |
| Cloudflare-protected sites fail to import, [#641](https://github.com/norish-recipes/norish/issues/641) | open | "Will look into just a plain fetch" |

The most-commented issues are operational: a 6.2 GiB memory climb traced to a health check that left sockets open ([#178](https://github.com/norish-recipes/norish/issues/178)), CPU from WebSocket reconnect loops ([#58](https://github.com/norish-recipes/norish/issues/58)), containers that crash or never show the interface after an upgrade ([#426](https://github.com/norish-recipes/norish/issues/426), [#435](https://github.com/norish-recipes/norish/issues/435), [#444](https://github.com/norish-recipes/norish/issues/444)), Mealie imports ([#240](https://github.com/norish-recipes/norish/issues/240)), OIDC household mapping ([#158](https://github.com/norish-recipes/norish/issues/158)) and Azure OpenAI ([#144](https://github.com/norish-recipes/norish/issues/144)). All are closed. In discussions, the most upvoted posts are the browser extension, recipe variations, user management (shipped in v0.22.0), why unit conversion needed AI (answered by v0.25.0), and an Android app.

## Recent direction

| Release | Date | What it added |
|---|---|---|
| v0.19.0 | 2026-06-19 | HeroUI v3 redesign, home screen with today's meals, cooking mode |
| v0.20.0 | 2026-08-07 | offline web app, recipe provenance, step ingredients, one enrichment pipeline |
| v0.21.0 | 2026-08-22 | Obscura replaces Chrome, recipe export, image generation, mobile recipe page tinted by its photo |
| v0.22.0 | 2026-09-03 | cookbooks, user management, several logins per site |
| v0.23.0 | 2026-09-09 | aisles and shop prices |
| v0.24.0 | 2026-09-24 | TypeSafe's Jev as Decision Model, basic pantry, import from a copied link |
| v0.25.0 | 2026-10-13 | ingredient catalogue, nutrition without AI, ingredient icons, conversion without AI, pantry page, cookbook covers, security fixes |
| v0.25.1 | 2026-10-14 | Recipe Keeper import, cookbooks in exports, better Mealie and Tandoor import |
| v0.25.2 (candidate) | open | ingredient parsing that keeps every word, read-only containers |

Source: `apps/docs/docs/release-notes/`, GitHub releases.

The direction since June is a deeper data model under the same household app: step-level ingredient links, an ingredient catalogue with nutrition, aisles, prices and a pantry, and a second, cheaper AI model for closed decisions. The last published plan is the "Up next" list in v0.20.0: import and export, ingredients as first-class records, a print layout, rotating site accounts, Crowdin for translations, then the mobile apps. Three of the six have shipped: export, ingredient records and account rotation. Print is an open PR, the repository holds no Crowdin configuration, and the mobile app is parked. An RSS feed is the one issue labelled `status:in-progress` ([#305](https://github.com/norish-recipes/norish/issues/305)), with a `feature/rss-feed` branch. There is no published roadmap beyond release notes; the backlog is in a private Linear project.

Open PRs from outside contributors, opened 2026-10-16 to 2026-10-21, include the MCP server, bulk image import, keeping source photos with thumbnails, serving under a URL base path, an "Enhanced" toggle for the AI enrichments, password change in account settings, removing a rating, and an import status endpoint.

## Not established

- What happened before the GitHub repository: versions 0.1 to 0.12 and where their source lived. Docker Hub shows the image repository registered on 2025-10-31, and the GitHub history starts with a single "first commit" on 2025-12-03.
- Why the contributors API lists only three accounts while release notes thank more than 20 people. Squash merges under the maintainer's name would explain it, but this was not checked commit by commit.
- Which API key rate limit applies at runtime: the plugin's 500 per hour or the column default of 100 per minute.
- Whether an API key reaches the internal `/api/trpc` procedures as well as `/api/v1`. The shared context builder and `enableSessionForAPIKeys: true` suggest it does; nothing was called to confirm it.
- How the web client tells a person their recipe edit was dropped as stale, if it does. The server returns success before the write and publishes nothing for a stale write; the client's handling was not traced.
- Whether a range such as "1-2 cups" keeps its upper bound anywhere in the editor path, which was not traced beyond the import parser.
- Whether CalDAV sync reads changes back from the external calendar or only pushes.
- How AI-converted measurement copies are kept when the recipe is edited afterwards.
- Whether Norish emails anyone, for password resets or otherwise; no mail configuration appears in the docs.
- The maintainer's response to PR #657, the MCP server, and to the other outside PRs opened in October.
- Docker Hub pull counts per tag, and how many instances run Norish.
- Nothing was run live: every behaviour above is read from code, docs, ADRs, release notes or the tracker.
