# How Mealie works

**Purpose:** a description of Mealie, the self-hosted recipe manager at github.com/mealie-recipes/mealie: what it does for the people using it, how it does it underneath, and what its users ask for.
**Method:** Mealie's source code on its default branch `mealie-next` at commit `7f099c91fb3da974b810c8d695b98a87d1573765` (2026-10-22 11:41 UTC), read on 2026-10-22 from a shallow clone. Every file path below is at that commit. Issues, discussions, pull requests, releases, stars and contributors were read through `gh api` the same day; all 1,364 discussions were pulled with their upvote counts. Two outside primary sources were fetched: the Home Assistant integration page and the `recipe-scrapers` source at the version Mealie pins. Nothing was run live: no Mealie instance was started and no import was tried.

The default branch is 61 commits ahead of the last release, v3.29.0. Where a feature exists in the code but in no release yet, the text says so.

## What it is and who makes it

Mealie describes itself as "a self hosted recipe manager, meal planner and shopping list with a RestAPI backend and a reactive frontend built in Vue for a pleasant user experience for the whole family" (`README.md`). The README lists six key features: import from a URL, a meal planner, a shopping list organised by supermarket section, cookbooks, Docker deployment, and translations into 35 or more languages.

| Fact | Value | Source |
|---|---|---|
| Repository created | 2020-12-07 | `gh api repos/mealie-recipes/mealie` |
| First release | v0.0.1, 2021-01-04 | releases list |
| v1.0.0 | 2024-01-20, after betas from May 2022 and release candidates from October 2023 | releases list |
| v2.0.0 | 2024-10-22, which added Households | releases list |
| v3.0.0 | 2025-07-11, which moved the frontend to Nuxt 3 and Vue 3 | releases list |
| Current release | v3.29.0, 2026-10-17 | releases list |
| Releases | 107 in all; 39 in the twelve months to 2026-10-22, 26 of them minor versions | releases list |
| Licence | AGPL-3.0-only | `pyproject.toml` |
| Stars, forks | 13,652 and 1,593 | `gh api`, 2026-10-22 |
| Contributors | 325 accounts on GitHub's contributor list, bots included; 334 with anonymous committers | `gh api .../contributors` |
| Open issues and pull requests | 110 | `gh api` |
| Funding | GitHub Sponsors for `hay-kot` and Buy Me a Coffee | `.github/FUNDING.yml`, `README.md` |

Hayden (`hay-kot`) wrote Mealie and is listed as its author (`pyproject.toml`). He holds 1,764 commits, many of them automated translation merges in his name. Michael Genson (`michael-genson`) is the second human contributor with 661 commits, then `boc-the-git` (235) and `Kuchenpirat` (171). In the twelve months to 2026-10-22 the default branch took 1,553 commits: 547 from Renovate, 375 under `hay-kot`, 184 from `michael-genson`, then a long tail. The v1.0.0 notes thank `michael-genson`, `boc-the-git` and `Kuchenpirat` for doing "a bulk of the work" and carry Hayden's last plug for his commercial recipe app, Recipinned ([v1.0.0](https://github.com/mealie-recipes/mealie/releases/tag/v1.0.0)).

Releases are drafted by hand on GitHub; a release triggers the image build and the version comes from the tag (`docs/docs/contributors/developers-guide/maintainers.md`). Every push to `mealie-next` publishes a `nightly` image. The v3.28.0 notes say: "With the popularity of agentic AI, pull requests are popping and the contributors list keeps growing!" ([v3.28.0](https://github.com/mealie-recipes/mealie/releases/tag/v3.28.0)). The repository carries `AGENTS.md` and `CLAUDE.md` for coding agents. Maintainers have also turned away AI-written pull requests: a recipe history pull request was closed as "rather complex ... for the functionality it provides" and "100% AI generated" ([#7313](https://github.com/mealie-recipes/mealie/pull/7313)).

Mealie has no telemetry. The v2.0.0 notes say so and ask users to fill a survey instead ([v2.0.0](https://github.com/mealie-recipes/mealie/releases/tag/v2.0.0)). That October 2024 survey had 525 respondents. About 85% of instances had two users or fewer, and most respondents had used Mealie for less than a year (`docs/docs/news/surveys/2024-october/overview.md`).

## Architecture

| Layer | What it is | Source |
|---|---|---|
| Backend | Python 3.14, FastAPI 0.143, SQLAlchemy 2.1, Alembic, Pydantic 2 | `pyproject.toml` |
| Web server | Uvicorn, one worker by default (`UVICORN_WORKERS`) | `backend-config.md` |
| Frontend | Nuxt 4, Vue 3, Vuetify 4, TypeScript, with `ssr: false` | `frontend/package.json`, `frontend/nuxt.config.ts`, `AGENTS.md` |
| Database | SQLite by default, PostgreSQL as the option | `mealie/core/settings/settings.py` line 269 |
| Background work | In-process: an asyncio scheduler with daily, hourly and five-minute loops, plus FastAPI `BackgroundTasks` | `mealie/app.py`, `mealie/services/scheduler/scheduler_service.py` |
| Packaging | One container serving the API and the statically built frontend | `AGENTS.md`, `docker/Dockerfile` |

In production the Nuxt app is generated as static files and served by FastAPI's SPA module (`mealie/routes/spa/`), so one container and one port serve everything. In development the two run apart, on ports 3000 and 9000 (`AGENTS.md`).

The backend follows a controller, service and repository split. Controllers in `mealie/routes/**/controller_*.py` handle HTTP. Services in `mealie/services/` hold the logic. Repositories in `mealie/repos/` wrap SQLAlchemy, and every repository is scoped to the caller's group and household automatically (`AGENTS.md`). TypeScript types for the frontend are generated from the Pydantic schemas, and the frontend's API clients are written by hand over them (`AGENTS.md`, `frontend/app/lib/api/`).

There is no queue, no Redis and no separate worker process. The scheduled tasks are registered in `mealie/app.py`: daily, it purges expired tokens, registrations, password resets and old group exports, writes "made this" timeline events for today's meal plans, and deletes checked shopping list items beyond 100 per list (`delete_old_checked_shopping_list_items.py`, `MAX_CHECKED_ITEMS = 100`). Every five minutes it posts due meal plan webhooks. Hourly it unlocks locked users. The daily run happens at `DAILY_SCHEDULE_TIME`, 23:45 local time by default.

Database migrations are Alembic scripts, 160 of them, from `2022-02-21` to `2026-10-21` (`mealie/alembic/versions/`). They run at startup (`mealie/db/init_db.py` line 119).

### The API

The frontend talks to the backend only through a JSON REST API under `/api`, which third parties use too. The route files declare 249 handlers with `@router.get`, `post`, `put`, `patch` or `delete`, before the generic CRUD routes a mixin adds (`mealie/routes/`). FastAPI serves interactive Swagger documentation at `/docs`, which `API_DOCS` turns off (`docs/docs/documentation/getting-started/api-usage.md`).

List endpoints share one pagination and filtering scheme: `page`, `perPage` (`-1` returns everything), `orderBy`, and a `queryFilter` string language with `AND`, `OR`, parentheses and comparison operators over model attributes, for example `createdAt >= "2021-02-22"` (`api-usage.md`, `mealie/services/query_filter/`). The same query filter language drives cookbooks and meal plan rules.

Long-running imports stream progress as server-sent events: `/recipes/create/url/stream`, `/create/html-or-json/stream` and `/create/ai/stream` sit beside their plain versions (`mealie/routes/recipe/recipe_crud_routes.py`), and the frontend reads them with `sse.js` (`frontend/app/lib/api/user/recipes/recipe.ts`).

Recipes, foods, shopping lists and shopping list items carry an `extras` field of free key and value pairs for third-party integrations, reachable only through the API (`api-usage.md`).

## The recipe data model

A recipe is one row in `recipes` with child tables (`mealie/db/models/recipe/recipe.py`).

| Part | How it is stored |
|---|---|
| Identity | A UUID and a slug unique within the group (`recipe_slug_group_id_key`) |
| Owner | `user_id`. The household is not a column: `household_id` is an association proxy through the owner's user row |
| Text | name, description, `org_url` (source), and normalised copies of name and description for search |
| Times | total, prep and perform time as text, plus `*_seconds` integers added in v3.29.0 ([#8539](https://github.com/mealie-recipes/mealie/pull/8539)) |
| Yield | `recipe_yield` text, `recipe_yield_quantity` and `recipe_servings` as numbers |
| Ingredients | ordered rows in `recipes_ingredients` |
| Instructions | ordered rows with an optional section `title`, `text`, `summary`, links to ingredients and to notes, and on the default branch only, `duration_seconds` |
| Notes | title and text, each with a reference id that steps can link to (v3.26.0) |
| Nutrition | one row of free-text strings: calories, carbohydrate, cholesterol, fat, fibre, protein, saturated, sodium, sugar, trans and unsaturated fat (`nutrition.py`) |
| Organisers | many-to-many categories, tags and tools |
| Assets | named files with an icon, stored on disk under the recipe's folder |
| Image | one hero image, re-encoded and stored on disk with thumbnails |
| Settings | `public`, `show_nutrition`, `show_assets`, `landscape_view`, `disable_comments`, `locked`, `disable_amount` (`settings.py`) |
| Social | per-user ratings and favourites (`users_to_recipes`), comments, timeline events |
| Extras | key and value pairs |

The recipe-level `rating` is the average of users' non-zero ratings, recomputed by a SQLAlchemy `before_update` hook (`recipe.py`, `calculate_rating`).

### Ingredients: as written and as parsed

Each ingredient row holds `quantity` (a float), `unit_id`, `food_id`, `note`, `original_text`, a section `title`, a `reference_id` that steps link to, an optional `referenced_recipe_id`, and a list of substitutions (`mealie/db/models/recipe/ingredient.py`). The API adds a computed `display` string built from quantity, unit, food and note (`mealie/schema/recipe/recipe_ingredient.py`, `_format_display`).

A recipe is "unparsed" until someone parses it. The recipe setting `disable_amount` defaults to true, and while it is on the ingredient is shown as its note text alone. The FAQ walks a user through seeding foods and units, turning the setting off, then opening the parser page per recipe; "Scaling up this recipe or adding it to a Shopping List will now smartly take care of ingredient amounts and duplicate combinations" only after that (`docs/docs/documentation/getting-started/faq.md`). Without it, scaling can yield "`2 1 cup broth`", in the FAQ's own example.

Three parsers are registered: `nlp`, `brute` and `openai` (`recipe_ingredient.py`, `RegisteredParser`).

- The NLP parser calls `ingredient-parser-nlp` 2.9.1 with the group's units passed as custom units (`mealie/services/parser_services/ingredient_parser.py` line 305). No language is passed to it.
- The brute parser splits the line by rules in `mealie/services/parser_services/brute/`.
- The AI parser sends the lines to the group's default AI provider with the prompt `parse-recipe-ingredients.txt`.

Each parse returns per-field confidence. The parsed food and unit are matched against the group's stores with fuzzy matching (`rapidfuzz`), and a user can accept a suggested new food or unit, which needs the "organize" permission (`docs/.../permissions-and-public-access.md`). Since v3.28.0 the NLP and brute parsers set `original_text` ([#8458](https://github.com/mealie-recipes/mealie/pull/8458)).

Foods, units and labels belong to the group, not the household. A food has a name, plural, description, aliases, a label (used to sort shopping lists), an `on_hand` flag, extras, and common substitutions (`ingredient.py`). A unit has name, plural, abbreviation, plural abbreviation, `use_abbreviation`, `fraction`, aliases, and a `standard_quantity` with `standard_unit` that ties it to a known unit. Mealie ships seed data in 42 locales for foods and units (`mealie/repos/seed/resources/`). v3.28.0 cut the food seed from more than 2,000 entries to about 500 and linked each to a label ([v3.28.0](https://github.com/mealie-recipes/mealie/releases/tag/v3.28.0)).

Substitutions arrived in v3.26.0 in two kinds: per recipe ingredient and common per food. The recipe finder can take them into account ([v3.26.0](https://github.com/mealie-recipes/mealie/releases/tag/v3.26.0)).

### Units and conversion

Metric and US conversion shipped in v3.29.0 ([#8292](https://github.com/mealie-recipes/mealie/pull/8292)) and answers the most-upvoted feature request in the repository, [discussion #3881](https://github.com/mealie-recipes/mealie/discussions/3881) (110 upvotes). It runs in the browser at display time and does not change the stored recipe (`frontend/app/composables/recipes/use-unit-conversion.ts`). It converts only an ingredient whose unit carries a standardised unit from a fixed table of eight: fluid ounce, cup, ounce, pound, millilitre, litre, gram, kilogram (`frontend/app/composables/recipes/unit-systems.ts`). A code comment gives the reason: volume never becomes weight because that "needs a density per food, which we don't store." The target ladders are gram and kilogram, millilitre and litre, ounce and pound, and teaspoon through gallon. Results are rounded to a step of 1, 2 or 5 within 2.5% for decimals and 5% for fractions.

On the server, `pint` is used to merge shopping list quantities between compatible units (`mealie/services/parser_services/parser_utils/unit_utils.py`).

### Nested recipes

An ingredient can point at another recipe through `referenced_recipe_id`. This shipped in [#4800](https://github.com/mealie-recipes/mealie/pull/4800), merged 2025-11-04, after [discussion #1761](https://github.com/mealie-recipes/mealie/discussions/1761) gathered 101 upvotes. Saving a recipe that would create a cycle is refused with `RecursiveRecipe` (`mealie/services/recipe/recipe_service.py`, `has_recursive_recipe_link`). When a recipe goes on a shopping list, its sub-recipes' ingredients go with it, and a sub-recipe gets no list-level reference of its own (`mealie/services/household_services/shopping_lists.py`, `get_nested_recipe_ids`).

### Organisers and cookbooks

Categories, tags and tools are group-wide many-to-many organisers. The docs advise one or two categories per recipe and free use of tags (`features.md`). A cookbook is a saved query filter over organisers, foods, food labels, rating and, since v3.29.0, time; it belongs to a household and can be public (`features.md`, v2.0.0 notes).

### Photos

A recipe has one hero image and any number of assets. Asset uploads accept images, video (m4v, mov, mp4, webm), audio, documents including PDF, and CSV or JSON, and a downloaded asset is capped at 5 MB (`recipe_crud_routes.py` lines 100 to 125). Step photos are assets referenced from the step's markdown text, which is why duplicating a recipe rewrites the old recipe's id inside step text (`recipe_service.py`, `duplicate_one`). v3.28.0 lets a user drag an image from another website into a step. Each "I made this" timeline event can carry one photo. A request for more than one gallery image per recipe is open with 23 upvotes ([discussion #4134](https://github.com/mealie-recipes/mealie/discussions/4134)).

## People and sharing

### Groups and households

Mealie has three levels (`features.md`, "Groups and Households"):

- An instance holds groups. "Groups are fully isolated instances of Mealie ... There is no data shared between groups."
- A group holds households. Households share recipes, foods, units, labels and organisers with the rest of the group.
- Meal plans, shopping lists, notifiers, webhooks, cookbooks and recipe actions belong to a household.

Households arrived in v2.0.0, a change the notes say "touched 300+ files and 10k+ lines of code" ([v2.0.0](https://github.com/mealie-recipes/mealie/releases/tag/v2.0.0), [#3970](https://github.com/mealie-recipes/mealie/pull/3970)). A recipe belongs to its owner's household, and every user in the group can browse it, favourite it, comment on it and mark it made.

### Roles and permissions

A user row carries booleans, not a role (`mealie/db/models/users/users.py`):

| Flag | Allows |
|---|---|
| `admin` | the admin tools, and editing or deleting any recipe |
| `can_manage` | group settings, including AI providers and prompts |
| `can_manage_household` | household settings |
| `can_invite` | making invitation links |
| `can_organize` | creating, editing, merging and deleting foods, units, tags, categories, tools, labels and recipe actions |
| `advanced` | showing advanced options in the interface |

Recipe edit rights are decided in one SQL statement in `RecipeService.can_update` (`recipe_service.py` lines 94 to 141). An admin may edit anything. Otherwise the owner may edit. A non-owner may edit only if the recipe is not locked, and, for a user from another household, only if that household's `lock_recipe_edits_from_other_households` is off; it defaults to on (`mealie/db/models/household/preferences.py`). Only the owner may lock or unlock (`can_lock_unlock`). Deleting a recipe needs ownership or admin, whatever the edit rules say (`can_delete`). Anyone in the group may set a recipe's last-made date, even on a locked recipe (`update_last_made`).

On the default branch, route permissions are now declared through a `require()` dependency, checked by a test against real users, and enforced in CI by semgrep (commits [#8763](https://github.com/mealie-recipes/mealie/pull/8763), [#8767](https://github.com/mealie-recipes/mealie/pull/8767), [#8769](https://github.com/mealie-recipes/mealie/pull/8769), after v3.29.0).

### Joining

Self sign-up is off by default (`ALLOW_SIGNUP = False`, `settings.py` line 178). An invitation token names a group and household and has a use count, one by default (`mealie/db/models/household/invite_tokens.py`). A first install creates a default admin, `changeme@example.com` with the password `MyPassword` (`settings.py` lines 285 and 290). Five wrong passwords lock an account for 24 hours (`SECURITY_MAX_LOGIN_ATTEMPTS`, `SECURITY_USER_LOCKOUT_TIME`).

### Public access and share links

Groups and households start private (`private_group` and `private_household` default to true). A logged-out visitor can see a recipe only if the group is not private, the household is not private and allows outside viewers, and the recipe's `public` setting is on (`docs/.../permissions-and-public-access.md`). A public group gets a public home page of its recipes and cookbooks.

The Share button makes a private link that "bypass[es] all group and recipe permissions". Its token expires after 30 days by default (`mealie/schema/recipe/recipe_share_token.py`) and a daily task purges expired ones. Since v3.23.0 no expiring link is made when the group and household are already public ([#7941](https://github.com/mealie-recipes/mealie/pull/7941)), and since v3.29.0 the dialog shows a QR code ([#8558](https://github.com/mealie-recipes/mealie/pull/8558)). Recipes cannot be shared between groups on one instance except by export and import.

### When someone leaves or is deleted

A user is in exactly one group and household (`users.group_id`, `users.household_id`). Deleting a user cascades to their API tokens, comments, timeline events, password resets, meal plan entries and shopping lists (`users.py`, `sp_args`; meal plans and lists since v3.23.0, [#8120](https://github.com/mealie-recipes/mealie/pull/8120)). Recipes do not cascade and are not reassigned. They keep a `user_id` that points nowhere, which also leaves them without a household, since the household is read through the user. The bug is open and confirmed: [#3171](https://github.com/mealie-recipes/mealie/issues/3171), "Deleting a User Should Re-assign Recipes", since 2024-02-13. A maintainer's answer to a user stuck with such recipes was a hand-run `DELETE FROM recipes WHERE user_id NOT IN (SELECT id FROM users)`, marked "UNTESTED!!!" ([comment](https://github.com/mealie-recipes/mealie/issues/3171)). A separate open bug reports that deleting some users fails with "unexpected error" ([#6541](https://github.com/mealie-recipes/mealie/issues/6541)).

## Recipe changes over time

Mealie keeps no history of a recipe's content. An update is a full overwrite: the code says "Updates are a full overwrite, so a client that round-trips a recipe without echoing `image` back would otherwise clear it" (`recipe_service.py`, `_preserve_omitted_image`). `date_updated` is set on each save. There is no version table, no diff and no undo.

There is no concurrency control. Neither `update_one` nor `patch_one` compares a version or timestamp, so the last save wins. Two PATCH requests that overlap can duplicate a recipe's ingredients and steps; the bug has been open since 2023-12-09 ([#2808](https://github.com/mealie-recipes/mealie/issues/2808)). Michael Genson wrote in 2024: "There's some sort of db-level race condition going on and there doesn't seem to be an easy way around it."

What does exist:

- **The timeline.** Each recipe has `recipe_timeline_events` with type `system`, `info` or `comment`, a subject, a message, an optional photo and a timestamp (`mealie/schema/recipe/recipe_timeline_events.py`). Creating a recipe writes a `system` event. "I made this" writes a `comment` event and sets the household's last-made date. Meal plan entries for today write an `info` event each night. Edits write nothing to it.
- **Locking.** The owner can lock a recipe against edits by others (see above).
- **Duplicating.** `POST /recipes/{slug}/duplicate` copies the recipe, minus comments, under the caller as owner, gives ingredients new reference ids, copies the asset folder and rewrites asset links in step text (`duplicate_one`). The copy keeps no link to its original. This landed in 2022 ([#1750](https://github.com/mealie-recipes/mealie/pull/1750)).

Requests for history have not been accepted. [Discussion #880](https://github.com/mealie-recipes/mealie/discussions/880), "Recipe versioning" (25 upvotes), asked for variations under a parent recipe and was closed in 2024 as a duplicate of the sub-recipe discussion. [Discussion #4584](https://github.com/mealie-recipes/mealie/discussions/4584), "Recipe Revision History" (11 upvotes), is open. In it a contributor noted that editing an ingredient and then pressing the close button still saved the change. The resulting pull request, [#7313](https://github.com/mealie-recipes/mealie/pull/7313), stored JSON copies of each version with a diff view and restore, and was closed unmerged on 2026-03-26 with the comment quoted above, which also cited "a significant disk-space footprint".

## Cooking

| Feature | What exists | Source |
|---|---|---|
| Cook mode | A view of ingredients and instructions side by side, each scrolling on its own; when steps link ingredients, the steps alone, with unlinked ingredients below. Its state lives in the URL as `?cook=` since v3.24.0 | `RecipePage.vue`, [#7287](https://github.com/mealie-recipes/mealie/pull/7287) |
| Screen awake | A wake lock switch on recipe and shopping list pages, shown only over HTTPS in a browser that supports the API | `WakelockSwitch.vue`, FAQ |
| Scaling | A scale control that multiplies parsed quantities in the browser; used by cook mode, print and the ingredient list | `RecipePageScale`, `RecipePage.vue` |
| Ingredient ticks | Checkboxes kept per recipe in browser storage (`ingredientStorageKey`) | `RecipePage.vue` |
| Step durations | A whole-minute duration per step, stored in seconds and shown read-only. On the default branch, not in a release. "No automatic timer" | [#8705](https://github.com/mealie-recipes/mealie/pull/8705) |
| Timers | None. A kitchen timer was removed in January 2025 | [#4857](https://github.com/mealie-recipes/mealie/pull/4857) |
| Cook log | "I made this" timeline entries with date, note and photo; automatic entries from meal plans | `create_timeline_events.py` |
| Ratings | Per user, averaged onto the recipe; favourites per user | `users_to_recipes` |
| Comments | Per recipe, can be turned off per recipe or per household | `RecipeComment`, `RecipeSettings` |
| Recipe finder | Finds recipes makeable from foods marked on hand, optionally counting substitutions | [#4542](https://github.com/mealie-recipes/mealie/pull/4542) |

The timer was removed because a web timer "can only ring when the page that they are on is currently opened" and so is "inherently unreliable"; the maintainers chose to "push people to using reliable timers by just not including an unreliable one" ([#4857](https://github.com/mealie-recipes/mealie/pull/4857)). A poll followed ([discussion #5356](https://github.com/mealie-recipes/mealie/discussions/5356)). The original timer request has 35 upvotes and is open ([discussion #1573](https://github.com/mealie-recipes/mealie/discussions/1573)).

## Planning and shopping

### Meal planner

A meal plan entry has a date, a type (breakfast, lunch, dinner, side, snack, drink or dessert), a title, text, an optional recipe and a `recipe_scale` (`mealie/db/models/household/mealplan.py`, `mealie/schema/meal_plan/new_meal.py`). An entry without a recipe is a note, for leftovers or eating out. Plans belong to the household.

Plan rules restrict which recipes a random pick may choose, by day of week and meal type, through the same query filter language: tags, categories, foods, food labels, rating, and since v3.29.0 time (`GroupMealPlanRules.query_filter_string`). Since v3.29.0 each planned recipe has its own servings, and adding a day or a plan to a shopping list follows them ([#8701](https://github.com/mealie-recipes/mealie/pull/8701)).

Since v3.29.0 a household can publish its plan as an iCal feed. "Once turned on, anyone with the iCal link can subscribe without any additional auth" ([v3.29.0](https://github.com/mealie-recipes/mealie/releases/tag/v3.29.0), [#8461](https://github.com/mealie-recipes/mealie/pull/8461)). The token is `mealplan_ical_token` on the household preferences. This answers [discussion #850](https://github.com/mealie-recipes/mealie/discussions/850) (50 upvotes), opened in 2021.

### Shopping lists

A shopping list belongs to a household and is owned by a user; items have quantity, unit, food, note, label, position, a checked flag, extras, and references to the recipes they came from with each recipe's scale (`mealie/db/models/household/shopping_list.py`). A household can keep several lists.

Adding a recipe's ingredients merges them into existing items (`ShoppingListService.can_merge`, `merge_items`):

- Checked items never merge.
- Items merge only if they have the same food. Two items with no food merge only if their notes are equal.
- Different units merge only if both units carry a standardised unit and `pint` can convert between them; the sum is converted with `merge_quantity_and_unit`. Otherwise quantities are simply added when the units match.
- Different notes are joined with " | ".
- Recipe references merge, adding the scales.

Foods marked `on_hand` are left unticked when a recipe goes onto the list, and since v3.28.0 an explicitly ticked on-hand item is still added ([#8441](https://github.com/mealie-recipes/mealie/pull/8441)). Items are grouped by the food's label, and the label order is set per list. v3.29.0 replaced the item form with one entry box that suggests foods from the store and falls back to the typed text as a note ([#8719](https://github.com/mealie-recipes/mealie/pull/8719)).

There is no pantry or stock count beyond `on_hand`. [Discussion #2448](https://github.com/mealie-recipes/mealie/discussions/2448), "Pantry Ingredients list" (31 upvotes), drew this answer from Kuchenpirat in 2025: "This is something we probably won't get into it, because the constant manual upkeep required to keep this up to date is something we don't want mealie to represent", with a pointer to Grocy.

### Sync between people

There is no push. The shopping list page polls the server every 5 seconds while the user is active, stops after 5 idle minutes, and gives up after 17,280 failures, which is 24 hours (`frontend/app/composables/shopping-list-page/sub-composables/use-shopping-list-data.ts`). No other page polls. Changes from two people meet on the server as separate bulk create, update and delete calls; for the same item, the client keeps whichever copy has the later `updatedAt` when it merges its queue into the server's list (`use-shopping-list-item-actions.ts`, `mergeListItemsByLatest`).

## Getting recipes in and out

### From a web page

`POST /recipes/create/url` tries four strategies in order and stops at the first that yields a recipe (`mealie/services/scraper/recipe_scraper.py`, `DEFAULT_SCRAPER_STRATEGIES`):

1. `RecipeScraperPackage`: the `recipe-scrapers` library 15.12.0, called with `supported_only=False`, so any page with schema.org Recipe data is read even if no site scraper exists. Version 15.12.0 maps 726 host names to site-specific scrapers (`recipe_scrapers/__init__.py`, `SCRAPERS`, at tag 15.12.0).
2. `RecipeScraperOpenAITranscription`: when the group has an audio provider and `yt-dlp` recognises the URL as a video.
3. `RecipeScraperOpenAI`: the AI import workflow on the page's HTML, when an AI provider is set.
4. `RecipeScraperOpenGraph`: a stub recipe from the page's Open Graph title, description and image, read with `extruct`.

Fetching escalates in three steps: a direct request through `curl_cffi` impersonating several real browsers' TLS fingerprints, then an optional proxy (`SCRAPER_PROXY_URL`, mode `always` or `fallback`), then an optional self-hosted FlareSolverr (`backend-config.md`, "How Mealie fetches a page"). Since v3.26.0 every server-side fetch, including imports, webhooks, recipe actions and OIDC avatars, refuses private and local addresses unless `HTTP_ALLOW_LIST` names them, against SSRF and DNS rebinding ([#7914](https://github.com/mealie-recipes/mealie/pull/7914)). This was a breaking change for anyone pointing webhooks at their own network.

Other entry points: bulk URL import (`/create/url/bulk`, run in a background task), paste of HTML or schema.org JSON (`/create/html-or-json`), a single Mealie recipe zip (`/create/zip`), a bookmarklet and an iOS Shortcut (`docs/docs/documentation/community-guide/`). On the default branch, a pasted Mealie recipe JSON now imports with its full structure: foods, units, step titles, ingredient links, notes, tools, and tags and categories when ticked ([#8651](https://github.com/mealie-recipes/mealie/pull/8651), not released).

### From other apps

The group migrations page reads exports from Tandoor, Nextcloud Cookbook, Paprika, Chowdown, Plan to Eat, Recipe Keeper, Copy Me That, My Recipe Box, DVO Cook'n X3, and pre-v1 Mealie (`mealie/services/migrations/`, `features.md`). Since v3.28.0 the page says these are for other applications, not Mealie upgrades ([#8289](https://github.com/mealie-recipes/mealie/pull/8289)). On the default branch, migrations are scoped to the group and the archive size is limited ([#8761](https://github.com/mealie-recipes/mealie/pull/8761)).

### With AI

The "Import with AI" page (v3.23.0, [#8043](https://github.com/mealie-recipes/mealie/pull/8043)) takes any mix of a URL, pasted text, HTML or schema.org JSON, and photos (`docs/.../installation/ai-providers.md`). Photos need an image provider; a video URL needs an audio provider. The workflow has four steps (`mealie/services/recipe/import_workflow/workflow.py`):

1. Compile source: read each input into text. Images go to the image provider, web pages are fetched, videos are transcribed.
2. Build recipe: turn the compiled text into Mealie's recipe schema with the default provider, using structured output (`response_format`).
3. Translate, if a target language was chosen.
4. Resolve organisers: ask which tags, categories and tools the recipe "refers to" and match them against existing ones. New ones are created only if the user ticks "Create new organizers". The docs: "It won't guess a cuisine from the ingredients or a course from the kind of dish."

Pasted content wins where it disagrees with other sources. Several photos are treated as pages of one recipe and the first becomes the cover. The image input accepts `image/*` (`RecipeImportImages.vue`), with HEIC through `pillow-heif`.

Video import downloads the best audio with `yt-dlp`, re-encoded by ffmpeg to 32 kbps mono MP3, and fetches subtitles. If the video has subtitles, they are used and the audio is not sent; otherwise the audio goes to the transcription model (`mealie/services/openai/transcription.py`). Any URL that one of `yt-dlp`'s non-generic extractors claims counts as a video. `YTDLP_COOKIEFILE` supplies cookies for sign-in-only videos.

PDF is not an import format. PDFs can be attached to a recipe as assets only. Hayden in 2024, under the PDF import request: "Overall parsing PDFs is more or less an impossible problem ... It's a huge problem set that no one on the core team is interested in diving into and we don't really have the capacity to take on the ask" ([discussion #1588](https://github.com/mealie-recipes/mealie/discussions/1588), 59 upvotes, open).

### Out

- **One recipe** as JSON or as a zip of the JSON and its image (`mealie/services/recipe/template_service.py`). Jinja2 templates were removed in v3.0.0 ([#5631](https://github.com/mealie-recipes/mealie/pull/5631)).
- **Many recipes** through Data Management, with the organise permission: a zip holding each recipe's JSON under `recipes/<slug>/` with its image and asset folder (`mealie/services/exporter/recipe_exporter.py`).
- **Foods and units** as JSON.
- **Print** through the browser's print view, which can include linked ingredients since v3.24.0.
- **Every recipe page** served by the backend has the recipe injected into its `<head>` as schema.org JSON-LD (`mealie/routes/spa/__init__.py`, `inject_recipe_json`).

Export to PDF, for single recipes and for whole cookbooks, is the most-upvoted open issue, at 20, open since 2022 ([#1306](https://github.com/mealie-recipes/mealie/issues/1306)).

### Backups

An admin backup is a zip of `database.json`, every table dumped through SQLAlchemy, plus the data folder without its `backups` and `.temp` directories and without zip files (`mealie/services/backups_v2/backup_v2.py`). Its name carries the app version. Restore drops every table, reloads them and copies the data folder back, including the `.secret` file that signs tokens, then logs everyone out. On PostgreSQL the restore needs a superuser (`docs/.../usage/backups-and-restoring.md`). The docs call it "a destructive action" that "cannot be undone".

Mealie takes no automatic backups: "Due to some issues in the past, Mealie no longer performs automatic backups" (`features.md`). The FAQ still says the opposite, "automatic daily backups that export your data in json, plain-text markdown files, and/or custom Jinja2 templates" (`faq.md`, "Why a database?"); that answer is stale. For SQLite the docs call stopping the container and copying `/app/data/` "the **best** way to backup your data".

## AI and agents

AI arrived in v1.7.0 with the AI ingredient parser. URL import fell back to AI from v1.9.0, and image import was in nightly by August 2024 ([discussion #1588](https://github.com/mealie-recipes/mealie/discussions/1588), [#3974](https://github.com/mealie-recipes/mealie/pull/3974)).

Mealie speaks only the OpenAI API, through the `openai` Python client 3.33.0. "Mealie works with any OpenAI-compatible API", and the docs name Azure and Ollama (`ai-providers.md`). Providers are configured per group in the database, not in the environment (`mealie/db/models/group/ai_providers.py`). A provider has a name, base URL, API key, model, a timeout of 300 seconds by default, and extra headers and parameters. A group names three roles: a default provider, an image provider and an audio provider. Without a default provider every AI feature is off. v3.27.0 added a connectivity test that sends a real chat message, checks image support and checks transcription with a silent WAV (`mealie/services/openai/openai.py`). The API key is stored as a plain string column; the source holds no encryption code for it.

| AI feature | Provider used |
|---|---|
| AI ingredient parser | default |
| URL import fallback when no recipe data is found | default |
| Import with AI from text, HTML or JSON | default |
| Import with AI from photos | image for reading, default for building |
| Import from a video URL | audio for transcription when no subtitles, default for building |
| Translation during import | default |
| Organiser matching | default |

Prompts are text files: `build-recipe`, `parse-recipe-ingredients`, `parse-recipe-video`, `resolve-organizers`, `translate-recipe`, and two system prompts, `compile-source` and `transcribe-audio` (`mealie/services/openai/prompts/`). On the default branch, group managers can add instructions to every prompt, append to one prompt, or replace it, and are told when a replaced prompt changes upstream ([#8736](https://github.com/mealie-recipes/mealie/pull/8736), docs tagged v3.30.0, not released). This replaces the `OPENAI_CUSTOM_PROMPT_DIR` variable. The docs say the free OpenAI tier is not enough and suggest depositing $5 to reach tier 1.

### API tokens

A user makes long-lived tokens on their profile page. Each is an HS256 JWT signed with the instance secret, valid five years, and stored in `long_live_tokens` so it can be revoked (`mealie/routes/users/api_tokens.py`, `mealie/core/dependencies/dependencies.py`). A token carries the user's id, a name and an optional `integration_id`. Tokens have no scopes: a token can do everything its user can.

### MCP

Mealie ships no MCP server. [Discussion #7051](https://github.com/mealie-recipes/mealie/discussions/7051) (5 upvotes) asks for one through `fastapi_mcp`, with no maintainer reply. Community servers wrap the REST API with a user token: `rldiao/mealie-mcp-server` (153 stars, pushed 2026-10-03), and two forks or rewrites announced in [discussion #7630](https://github.com/mealie-recipes/mealie/discussions/7630), `amercat37/mealie-mcp-server` with Authentik OIDC in front, and `retr083/mealie-mcp` with its own OAuth 2.1 server for use as a remote connector.

## Search

Search runs in the database, with two modes chosen per query (`mealie/schema/response/query_search.py`, `mealie/schema/recipe/recipe.py`, `filter_search_query`):

- **Tokenised**, always on SQLite, and on PostgreSQL when the query contains quotes. Punctuation becomes spaces, quoted phrases stay whole, and each term is matched with `LIKE '%term%'` against the normalised name, the normalised description, and the ingredient note and original text. Results with the whole query in the name sort first.
- **Fuzzy**, PostgreSQL only with `pg_trgm`. The same columns are matched with trigram word similarity (`%>`), each column on its own so GIN indexes are used, and results sort by trigram distance to the name.

Normalisation lowercases, strips punctuation and transliterates to ASCII with `text-unidecode` (`mealie/db/models/_model_base.py`). Tags, categories and food names are not searched as text; they are filters. Filters over organisers, foods, households, rating, last made, and since v3.29.0 time, combine with the text search through the query filter language. No embedding or semantic search exists.

## Offline and mobile

Mealie is a PWA through `@vite-pwa/nuxt` (`frontend/nuxt.config.ts`). The Workbox service worker precaches the built JavaScript, CSS, HTML and images, falls back to `/` for navigation outside `/api` and `/docs`, and checks for updates every 120 seconds. It declares no runtime caching, so no recipe, list or plan is cached: with no network, the app shell loads but the data does not.

The shopping list is the one exception. Its creates, updates and deletes queue in `localStorage` under `shopping-list-queue` and are sent when the device is back online (`use-shopping-list-item-actions.ts`). If the server's list was updated more than 5 minutes after the queue's last sync, the client drops the whole queue with the log line "Out of sync with server; clearing queue". While offline, the list keeps local changes and does not overwrite them with server data.

There is no official native app. Community clients exist and the maintainers allow them; asked about an App Store name, Hayden replied that "there are already 3 apps for Mealie in the App Store" and preferred a name like "Pocket for Mealie" ([discussion #6600](https://github.com/mealie-recipes/mealie/discussions/6600), [#6951](https://github.com/mealie-recipes/mealie/discussions/6951)). Since v3.23.0 native OIDC endpoints serve mobile and desktop clients ([#7804](https://github.com/mealie-recipes/mealie/pull/7804)), and v3.29.0 added public OIDC clients for Google and Microsoft ([#8653](https://github.com/mealie-recipes/mealie/pull/8653)). Since v3.28.0 cookies are partitioned (CHIPS) so Mealie works embedded in Home Assistant ([#8522](https://github.com/mealie-recipes/mealie/pull/8522)).

## Languages and translation

The interface ships in 42 locale files, translated on Crowdin (`frontend/app/lang/messages/`, `crowdin.yml`). The backend has 42 message files for server-side text such as timeline subjects (`mealie/lang/messages/`). Foods and units seed in each language, and since v3.28.0 languages without plurals seed no plural forms.

A recipe has one language. Its content is never translated except once, at AI import, if the user picks a target language; the docs say recipes "keep the language of their source unless you choose to translate them" (`ai-providers.md`). Requests for one recipe in several languages ([discussion #1684](https://github.com/mealie-recipes/mealie/discussions/1684), 22 upvotes) and for multilingual food names ([discussion #4219](https://github.com/mealie-recipes/mealie/discussions/4219), 29 upvotes) are open. Alphabetical sorting ignores locale ([#6853](https://github.com/mealie-recipes/mealie/issues/6853), open).

## Deployment and operations

| Topic | What Mealie does | Source |
|---|---|---|
| Images | `ghcr.io/mealie-recipes/mealie` and `hkotel/mealie` on Docker Hub, tagged `latest`, `nightly` and the version; amd64 and arm64 | `README.md`, `.github/workflows/publish.yml` |
| Base image | `python:3.14-slim`, pinned by digest; Node 24 for the frontend build stage; ffmpeg and gosu installed | `docker/Dockerfile` |
| User | Starts as root, changes the `abc` user to `PUID` and `PGID` (911 by default), chowns `/app`, then drops to it with `gosu`; can also start directly as that uid | `docker/entry.sh` |
| Port | 9000 inside the container | `docker/Dockerfile` |
| Data | `/app/data`: the SQLite file, recipe images and assets, user folders, backups, `.secret` | docs, `backup_v2.py` |
| Memory | The compose example sets a 1,000 MB limit, because Python "can pre-allocate larger amounts of memory than is necessary" | `installation/sqlite.md` |
| SQLite | Recommended for 1 to 20 users; WAL is opt-in through `SQLITE_MIGRATE_JOURNAL_WAL` | `sqlite.md`, `backend-config.md` |
| PostgreSQL | `DB_ENGINE=postgres` and `POSTGRES_*`; fuzzy search needs it. On the default branch an unknown `DB_ENGINE` is refused instead of falling back to SQLite ([#8605](https://github.com/mealie-recipes/mealie/pull/8605)) | `backend-config.md` |
| Secrets | `_FILE` variants for Postgres, SMTP, LDAP and OIDC settings | `docker/entry.sh` |
| Subpath | Not supported | FAQ |
| Without Docker | Possible but "recommended that you don't" | FAQ |
| Health | `HEALTHCHECK` with `healthcheck.sh` | `docker/Dockerfile` |
| Branding | Colours per light and dark theme, and since v3.29.0 a name and logo | `backend-config.md` |

Authentication options, all configured by environment variable (`backend-config.md`):

- Built-in username and password, with `bcrypt`. `ALLOW_PASSWORD_LOGIN=false` hides the form.
- LDAP with bind user, filters for users and admins, STARTTLS and CA file.
- OIDC with sign-up, a required group, an admin group, auto-redirect, chosen claims and scopes. v3.23.0 added native endpoints for passkey-only providers such as Pocket ID and authentik. An open issue says Mealie looks users up by a mutable claim rather than the subject ([#6197](https://github.com/mealie-recipes/mealie/issues/6197)), and another that logout does not end the OIDC session ([#6098](https://github.com/mealie-recipes/mealie/issues/6098)).
- Reverse proxy header auth since v3.29.0, with an optional shared secret header ([#8183](https://github.com/mealie-recipes/mealie/pull/8183)). This answers [discussion #801](https://github.com/mealie-recipes/mealie/discussions/801) (32 upvotes, 2021).

There is no built-in passkey or second factor. Sessions changed in v3.25.0: tokens refresh themselves, a session ends when the browser closes unless "Remember Me" is ticked, and `TOKEN_TIME` caps a session, at 48 hours by default and at most 9,600 hours ([v3.25.0](https://github.com/mealie-recipes/mealie/releases/tag/v3.25.0)).

Upgrades are a new image; migrations run on start. The docs say "You MUST read the release notes prior to upgrading" and to back up first (`updating.md`). Pre-v1 data needs an export and re-import (`migrating-to-mealie-v1.md`). Breaking changes have reached ordinary users several times within v3: the SSRF guard in v3.26.0 and the login behaviour in v3.25.0. Requests for major-version image tags are open ([discussion #5751](https://github.com/mealie-recipes/mealie/discussions/5751), [#6175](https://github.com/mealie-recipes/mealie/discussions/6175)).

## Integrations

- **Notifiers** send event messages through Apprise, which reaches many services, plus `json`, `form` and `xml` targets that post the event data (`features.md`, `mealie/services/event_bus_service/publisher.py`). The events cover recipes, meal plan entries, shopping lists, cookbooks, tags, user sign-up, migrations, exports and imports: 21 types in `event_types.py`.
- **Webhooks** are scheduled, not event-driven. Each fires at its set time on the day of a meal plan and carries that day's plan; "At this time only mealplan webhooks are supported" (`post_webhooks.py`).
- **Recipe actions** are household-defined buttons on every recipe. A `link` action opens a URL with `${id}`, `${slug}`, `${url}`, `${servings}`, `${yieldQuantity}` or `${yieldText}` filled in; a `post` action sends the recipe JSON to a URL (`features.md`). The docs' own example sends a recipe to Bring!.
- **iCal feed** for the meal plan, since v3.29.0.
- **Home Assistant** has a core Mealie integration for Mealie 3.2.0 and later. It makes a calendar per meal type, refreshed hourly, and a to-do list per shopping list, refreshed every 5 minutes, and offers actions to get, set, randomise and delete plans, get recipes, import a recipe from a URL and read list items. A known limit: editing a food item in the list turns it into a note item ([home-assistant.io/integrations/mealie](https://www.home-assistant.io/integrations/mealie/)). The docs say: "In a lot of ways, Home Assistant is why this project exists!" (`community-guide/home-assistant.md`).
- **Community guides** cover n8n backups, the Bring! API, a bookmarklet, an iOS Shortcut and SWAG (`docs/docs/documentation/community-guide/`).

## What users ask for and complain about

Feature requests live in GitHub Discussions; issues are for confirmed bugs and old v1 tasks (`maintainers.md`). The roadmap page holds one line, a link to file a feature request (`roadmap.md`). There is no published roadmap.

### Most-upvoted feature requests

| Upvotes | Discussion | State on 2026-10-22 |
|---|---|---|
| 110 | [#3881](https://github.com/mealie-recipes/mealie/discussions/3881) Convert units between metric, imperial and US | Shipped in v3.29.0, discussion still open |
| 101 | [#1761](https://github.com/mealie-recipes/mealie/discussions/1761) Add recipe as ingredient | Shipped, [#4800](https://github.com/mealie-recipes/mealie/pull/4800) |
| 64 | [#2748](https://github.com/mealie-recipes/mealie/discussions/2748) Nutrition per food item | Open; the FoodData Central pull request [#6747](https://github.com/mealie-recipes/mealie/pull/6747) closed unmerged |
| 59 | [#1588](https://github.com/mealie-recipes/mealie/discussions/1588) PDF import | Declined, see Hayden's comment above |
| 50 | [#850](https://github.com/mealie-recipes/mealie/discussions/850) Meal planner as iCal | Shipped in v3.29.0 |
| 41 | [#1899](https://github.com/mealie-recipes/mealie/discussions/1899) Shopping list from the meal plan | Partly shipped: a day's meals since late 2023, servings since v3.29.0 |
| 38 | [#2134](https://github.com/mealie-recipes/mealie/discussions/2134) Households under groups | Shipped in v2.0.0 |
| 37 | [#3743](https://github.com/mealie-recipes/mealie/discussions/3743) Nutrition data in the meal planner | Open |
| 36 | [#3717](https://github.com/mealie-recipes/mealie/discussions/3717) Alternative ingredient | Shipped as substitutions in v3.26.0 |
| 35 | [#1573](https://github.com/mealie-recipes/mealie/discussions/1573) Timers in steps | Open; timer removed, step durations on the default branch without a timer |
| 35 | [#3918](https://github.com/mealie-recipes/mealie/discussions/3918) Translate a recipe on import | Shipped through AI import |
| 32 | [#801](https://github.com/mealie-recipes/mealie/discussions/801) Forward auth header | Shipped in v3.29.0 |
| 31 | [#2448](https://github.com/mealie-recipes/mealie/discussions/2448) Pantry | Declined by Kuchenpirat; `on_hand` flag offered instead |
| 29 | [#4219](https://github.com/mealie-recipes/mealie/discussions/4219) Multilingual ingredient names | Open |
| 25 | [#880](https://github.com/mealie-recipes/mealie/discussions/880) Recipe versioning | Closed as a duplicate of #1761 |
| 23 | [#4134](https://github.com/mealie-recipes/mealie/discussions/4134) More images per recipe | Open |
| 18 | [#1386](https://github.com/mealie-recipes/mealie/discussions/1386) Turn off meal planning and shopping | Open; v3.24.0 added a way to disable services ([#7434](https://github.com/mealie-recipes/mealie/pull/7434)) |
| 18 | [#5921](https://github.com/mealie-recipes/mealie/discussions/5921) Recipe author field | Open; "Show recipe owner" is on the default branch ([#8823](https://github.com/mealie-recipes/mealie/pull/8823)) |

The pattern of the last year is that the oldest and most-upvoted requests are being closed one release at a time. Three remain refused on principle: PDF import, a pantry, and an in-browser timer. Nutrition per food and recipe history remain open with no accepted design.

### Long-standing bugs and complaints

- **Concurrent saves duplicate content**, [#2808](https://github.com/mealie-recipes/mealie/issues/2808), open since 2023, 11 comments.
- **Deleting a user orphans their recipes**, [#3171](https://github.com/mealie-recipes/mealie/issues/3171), open since 2024, 5 upvotes.
- **Login behind Nginx Proxy Manager fails silently with 401**, [#6681](https://github.com/mealie-recipes/mealie/issues/6681), 5 upvotes, 14 comments.
- **OIDC logout and identity**, [#6098](https://github.com/mealie-recipes/mealie/issues/6098), [#6197](https://github.com/mealie-recipes/mealie/issues/6197).
- **Only binds to IPv4**, [#6144](https://github.com/mealie-recipes/mealie/issues/6144).
- **Recipe actions point at the home page for private recipes**, [#4619](https://github.com/mealie-recipes/mealie/issues/4619).
- **Restoring a backup without images**, [#5791](https://github.com/mealie-recipes/mealie/issues/5791).
- **Shopping list wishes** from the 2024 survey free text: users ask for summed shopping quantities, per-store aisle order, a condensed list and voice entry (`docs/docs/news/surveys/2024-october/q9.md`). Several have since shipped: summing with parsed foods, label order per list, and the condensed view in v3.28.0 ([#8402](https://github.com/mealie-recipes/mealie/pull/8402)).

The parse step is the friction the FAQ itself describes: structured ingredients, and with them scaling, merging and conversion, need foods and units seeded and each recipe parsed by hand. An open bug calls the import-then-parse flow a "Workflow issue" ([#6623](https://github.com/mealie-recipes/mealie/issues/6623)).

### Recent direction

| Release | Date | Headline changes |
|---|---|---|
| v3.23.0 | 2026-08-18 | One "Import with AI" page; import split into separate provider calls; native OIDC endpoints |
| v3.24.0 | 2026-08-24 | Meal planner editor; lower idle memory and faster start; cook mode in the URL; services can be disabled; yt-dlp cookie file |
| v3.25.0 | 2026-09-02 | Token refresh and new "Remember Me"; rating in filters; delete unused organisers; image lightbox |
| v3.26.0 | 2026-09-14 | SSRF guard on all outbound requests, breaking; ingredient substitutions; notes linked to steps; bottom sheets; OIDC avatars |
| v3.27.0 | 2026-09-17 | AI provider connectivity test; fixes |
| v3.28.0 | 2026-09-24 | Food seed cut to about 500 with labels; drag images from the web; Python 3.14; partitioned cookies; condensed list |
| v3.29.0 | 2026-10-17 | Metric and US conversion; structured times and time filters; simpler list entry; iCal feed; reverse proxy auth; branding; share QR code |
| Default branch | after 2026-10-17 | Prompt customisation in the app; step durations; recipe owner shown; route permissions declared and checked; full-structure paste of Mealie JSON |

## Not established

- Whether the NLP ingredient parser reads languages other than English. Mealie passes no language to `ingredient-parser-nlp`, and its README shows only English; its documentation was not read.
- How many sites the `recipe-scrapers` site scrapers work on today. 726 is the count of host names in its table at 15.12.0, not of working sites.
- The size of the 249-handler count once the CRUD mixin routes are added; the OpenAPI schema was not generated.
- Whether AI provider API keys are encrypted at rest anywhere outside the code read. No encryption code was found and the column is a plain string.
- The behaviour of any feature in a running instance. Nothing was started, imported or synced.
- Docker pull counts, GitHub Sponsors figures and any other funding.
- The full set of third-party mobile apps and their states. Only the ones named in discussions are listed.
- Whether a nested recipe's ingredients are scaled by the parent's quantity on the shopping list. The default branch has a fix for scaling merged duplicates ([#8764](https://github.com/mealie-recipes/mealie/pull/8764)), but the path for nested recipes was not traced.
- The exact field list of the bulk recipe export zip; only the exporter's folder layout was read.
