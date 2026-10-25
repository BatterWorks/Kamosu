# How KitchenOwl works

**Purpose:** a description of what the self-hosted grocery list and recipe app KitchenOwl does for the people who use it, and how it does it underneath, from its own source, docs, releases and tracker.
**Method:** the source of `TomBursch/kitchenowl` on its default branch `main` at commit `cac641c619963f13134aecb80fadfb81f0c9249b` (2026-10-18), cloned shallowly and read on 2026-10-22; nothing was built or run. Every file path below is at that commit. Releases, issues, discussions, stars and contributors were read through `gh api` on 2026-10-22. The docs site and the project website were fetched on the same day. Where this note says the code does something, that is a reading of the code, not an observed run.

## What it is and who it is for

The README calls KitchenOwl "a smart self-hosted grocery list and recipe manager". Its pitch: "Easily add items to your shopping list before you go shopping. You can also create recipes and get suggestions on what you want to cook. Track your expenses so you know how much you've spent." ([`README.md`](https://github.com/TomBursch/kitchenowl/blob/cac641c619963f13134aecb80fadfb81f0c9249b/README.md)).

The website frames it as "the last grocery app you'll ever need" and lists six areas: households, recipes, learned ordering and suggestions, shopping lists, a meal plan and balances ([kitchenowl.org](https://kitchenowl.org/)). The audience is a household that shops together: the website says households let people "work on recipes, expenses, shopping lists, and meal plans together as a family or with other users".

The shopping list is the centre. Recipes exist mainly to fill it, and the meal plan exists to pick recipes. The README still labels the project "Public Alpha: Still working on stuff (rarely things might break)".

## History and project

| Fact | Value | Source |
|---|---|---|
| Repository created | 2021-02-03 | `gh api repos/TomBursch/kitchenowl` |
| First release on GitHub | v0.0.9, 2021-08-03 | `gh api .../releases` |
| Current release | v0.7.10, 2026-07-26 | [v0.7.10](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.10) |
| Unreleased work on `main` | 24 commits after v0.7.10, the last on 2026-10-18 | `gh api .../compare/v0.7.10...main` |
| Releases per year | 6 in 2021, 30 in 2022, 16 in 2023, 10 in 2024, 9 in 2025, 6 in 2026 so far; 77 in all | `gh api .../releases` |
| Licence | AGPL-3.0 | `LICENSE`, repo metadata |
| Stars and forks | 3,762 stars, 255 forks | repo metadata |
| Open issues and PRs | 345 open, 319 of them issues | repo metadata, search API |
| Commits on `main` | 1,722 | commits API pagination |
| Contributors | 56 GitHub accounts, 60 counting anonymous ones | contributors API |
| Who writes it | Tom Bursch with 1,487 commits; the next human contributor has 23 | contributors API |
| Funding | GitHub Sponsors, Liberapay and PayPal | `.github/FUNDING.yml` |
| Hosted offering | a free "default cloud service", `app.kitchenowl.org` | [privacy policy](https://kitchenowl.org/privacy/), `backend/app/config.py` |
| Mirror | an official Codeberg mirror since v0.7.8 | [v0.7.8](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.8) |

The project is one maintainer's. Tom Bursch wrote 1,487 of the 1,722 contributions the API counts, 86%, and contributors arrive mostly through single pull requests listed under "New Contributors" in each release.

The hosted service is free. The privacy policy says "we will never monetize or sell your information, and this service and software will always stay free and open" and that backups of it are kept 90 days ([kitchenowl.org/privacy](https://kitchenowl.org/privacy/)). The OpenAPI spec names `https://app.kitchenowl.org/api` as the "Official KitchenOwl server instance" (`backend/app/config.py`).

In spring 2026 the repository went offline. The v0.7.8 notes say the maintainer's "account got flagged for manual review by an automated system", that he "stopped the rollout of v0.7.7" while the profile was down, and that the Codeberg mirror was set up because of it ([v0.7.8](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.8)).

Version numbers come in two forms. The app is 0.7.10 (`backend/pyproject.toml`, `kitchenowl/pubspec.yaml` as `0.7.10+124`). The backend also carries an integer API version, `BACKEND_VERSION = 124`, and refuses clients older than `MIN_FRONTEND_VERSION = 71` (`backend/app/config.py`). The client refuses servers older than `MIN_BACKEND_VERSION = 110` (`kitchenowl/lib/config.dart`). Both sides read the other's number from the health endpoint (`backend/app/controller/health_controller.py`, `kitchenowl/lib/services/api/api_service.dart`).

## Architecture

| Part | What it is | Source |
|---|---|---|
| Client | One Flutter app for Android, iOS, macOS, Windows, Linux and the web; state held in `flutter_bloc` cubits | `kitchenowl/pubspec.yaml`, `kitchenowl/lib/cubits/` |
| Server | Flask 3 on Python 3.14, served by uWSGI with gevent, 200 greenlets by default | `backend/pyproject.toml`, `Dockerfile` |
| Database | SQLite by default, PostgreSQL as the other option, through SQLAlchemy 2 with Alembic migrations (46 revisions) | `backend/app/config.py`, `backend/migrations/versions/` |
| Live updates | Flask-SocketIO on the same server | `backend/app/sockets/` |
| Scheduled jobs | APScheduler inside the web process, or Celery when a message broker is set | `backend/app/jobs/jobs.py` |
| Files | photos stored as UUID-named files under `/data/upload`, with a blurhash in the `file` table | `backend/app/models/file.py`, `backend/app/controller/upload/upload_controller.py` |
| Metrics | optional Prometheus endpoint behind basic auth | `backend/app/config.py` |

The SQLite connection loads the ICU extension and sets `journal_mode=WAL`, `synchronous=FULL` by default and `busy_timeout=5000` (`backend/app/config.py`). WAL arrived in v0.7.10 to fix "slow performance on slow systems (e.g., NAS)" ([#1092](https://github.com/TomBursch/kitchenowl/pull/1092)).

Background jobs run on three timers (`backend/app/jobs/jobs.py`):

- Every 30 minutes: delete expired access tokens, refresh tokens, password-reset challenges and OIDC requests.
- Daily at 03:00: for each household, cluster shopping trips, recompute item ordering and item suggestions, then recompute recipe suggestion scores and ranks.
- Monthly on the 2nd at 00:00: delete households with no members and files nothing references.

Without a broker, those jobs run in the web process. The maintainer explained: "Without the broker setup, the main process is blocked during those daily/weekly/monthly tasks and cannot react to incoming requests. This is usually not a problem on self-hosted instances" ([discussion #944](https://github.com/TomBursch/kitchenowl/discussions/944#discussioncomment-15358156)). The RabbitMQ compose file notes that a broker "is only supported with a separate front & backend container" (`docker-compose-rabbitmq.yml`).

## How clients talk to the server

All reads and writes go over a JSON REST API under `/api` (`backend/app/api/register_controller.py`). It is resource-shaped with many verbs on one path. Household-scoped resources sit under `/api/household/<id>/...`: `recipe`, `item`, `shoppinglist`, `planner`, `expense`, `category`, `tag`, `export` and `import`. Single objects sit at top level, for example `POST /api/recipe/<id>` to update and `DELETE /api/shoppinglist/<id>/item` to tick an item off. Updates use `POST` on the object's path, not `PUT` or `PATCH`.

Inputs are checked by Marshmallow schemas through a `validate_args` decorator (`backend/app/controller/*/schemas.py`). An OpenAPI 3.0.2 document is generated at `/api/openapi`, described by the project as "WIP KitchenOwl API documentation" (`backend/app/config.py`), and rendered on the docs site ([API reference](https://docs.kitchenowl.org/latest/reference/API/)).

Authentication is JWT. Access tokens live 15 minutes and refresh tokens 30 days by default (`JWT_REFRESH_TOKEN_EXPIRES`). Every token's `jti` is stored in a `token` table, and a token missing from the table is treated as revoked (`backend/app/controller/auth/auth_controller.py`). Refresh tokens rotate; reusing an old one deletes the whole token family and logs that device out (`backend/app/models/token.py`). The health check sits at a fixed obscure path, `/api/health/8M4F88S8ooi4sMbLBfkkV7ctWwgibW6V`.

Socket.IO carries server-to-client events only. On connect, the server puts the socket in one room per household the user belongs to (`backend/app/sockets/connection_socket.py`). REST handlers then emit four events to the household room: `shoppinglist:add`, `shoppinglist:delete`, `shoppinglist_item:add` and `shoppinglist_item:remove` (`backend/app/controller/shoppinglist/shoppinglist_controller.py`). The client listens for those four and never emits (`kitchenowl/lib/services/api/shoppinglist.dart`). Native clients use the websocket transport and the web build uses long polling (`kitchenowl/lib/services/api/api_service.dart`). Only shopping lists are live; recipes, plans and expenses are not pushed.

The server also has `shoppinglist_item:add` and `shoppinglist_item:remove` socket handlers for writes (`backend/app/sockets/shoppinglist_socket.py`). The remove handler calls the decorated Flask route function `removeShoppinglistItem` with the wrong arguments, so as written it cannot work. The client does not use either handler.

Because a missed event leaves a list stale, the client refreshes the shopping lists every 30 seconds while signed in, and on every reconnect ("Periodic refresh as fallback for missed WebSocket events", `kitchenowl/lib/cubits/shoppinglist_cubit.dart`). The most-commented open issue is about this layer: "Bug: Constant connection loss" ([#239](https://github.com/TomBursch/kitchenowl/issues/239), 24 comments).

## The shopping list

### Items and amounts

An **item** is a household-wide name: `item` has `name` (128 characters), an optional `icon`, an optional `category_id`, `default` and `default_key` for items seeded from a template, `ordering` and `support` (`backend/app/models/item.py`). Names are matched case-insensitively within a household (`Item.find_by_name`). Recipes and lists point at the same item rows, so "Milk" on a recipe and "Milk" on the list are one thing.

A list entry is the pair (list, item) in `shoppinglist_items`, with one free-text `description` and `created_by` (`backend/app/models/shoppinglist.py`). The pair is the primary key, so an item appears at most once per list. There is no quantity column and no unit table. The amount is whatever text sits in the description. The maintainer's answer to "there is no information about the amount": "you can add quantities in the description field. If you search for an item `300g Sugar` the 300g should be put into the description field automatically. Alternatively you can separate the description using a `,`" ([discussion #531](https://github.com/TomBursch/kitchenowl/discussions/531#discussioncomment-11047847)).

Two small grammars read those descriptions, both written in Lark:

- `description_splitter` reads a typed query like `300g Sugar` into a name and an amount. It knows a number, an optional `x`, `mg`, `g`, `kg`, `ml` or `l`, unicode fractions and `a/b` fractions (`backend/app/util/description_splitter.py`).
- `description_merger` adds two descriptions together when the same item arrives twice from recipes or when two items are merged. It splits on commas, adds numbers whose units match, converts within mass and within volume and simplifies, for example to `kg` when the sum is a whole thousand grams. Anything it cannot read is appended as another comma-separated part (`backend/app/util/description_merger.py`). So `200g` plus `300g` gives `500g`, `1x` plus `2` gives `3`, and `1 cup` plus `200g` gives `1 cup, 200g`. Cups, spoons and imperial units are not understood as units.

The merger runs when recipe ingredients are added to a list that already holds the item (`addRecipeItems` in `shoppinglist_controller.py`). It does not run when a person types a known item: the client sends `PUT /api/shoppinglist/<id>/item/<item_id>` with the new description, which overwrites the old one (`kitchenowl/lib/services/transactions/shoppinglist.dart`, `updateItemDescription` in `shoppinglist_controller.py`).

### Categories and ordering

Each household has its own categories, each with an `ordering` integer the household can rearrange (`backend/app/models/category.py`). A new household gets the template for its language: the English template holds 655 items and 10 categories, and each template item may carry an icon and a category (`backend/templates/l10n/en.json`, `backend/templates/attributes.json`, `backend/app/service/import_language.py`). On every container start, `upgrade_default_items.py` re-imports the templates into every household unless `SKIP_UPGRADE_DEFAULT_ITEMS` is set (`backend/entrypoint.sh`).

The client offers three sort orders: alphabetical, "algorithmic" and by category (`kitchenowl/lib/enums/shoppinglist_sorting.dart`). The algorithmic order is learned, and is what the website means by "it learns the order you usually tick off items":

1. Every tick-off writes a `history` row with status `DROPPED` and the time (`backend/app/models/history.py`). An offline tick-off sends its own `removed_at` time so the order survives the delay (`removeShoppinglistItemFunc`).
2. Each night, `clusterShoppings` runs one-dimensional DBSCAN over the drop times with `eps = 600` seconds and `min_samples = 5`. Each cluster is one shopping trip (`backend/app/jobs/cluster_shoppings.py`).
3. `ItemSort` builds a cost matrix: within each trip, every item gains cost from the items ticked before it, and older trips decay by a factor of 0.75. A greedy topological sort then writes `item.ordering` from 1 upwards (`backend/app/jobs/item_ordering.py`).

The order is one number per item, so it models one shop. The nightly job only reads the history of the household's first list, the one with the lowest id (`backend/app/jobs/jobs.py`).

### Suggestions and recent items

Below the list, the client shows **recent items**: items ticked off on that list and not currently on any list, newest first, up to a count the person sets (`History.get_recent`, `kitchenowl/lib/widgets/shopping_list/sliver_shopinglist_item_view.dart`). Tapping one adds it back with its last description. A discussion asks to raise the cap of 120 ([discussion #917](https://github.com/TomBursch/kitchenowl/discussions/917)).

The nightly job also mines association rules with mlxtend's Apriori: minimum support 0.001, pairs only, kept when lift is above 1.2 and confidence above 0.1. It stores each item's single-item support in `item.support` and the pair rules in `association` (`backend/app/jobs/item_suggestions.py`). `item.support` ranks item search results. The rules feed `GET /api/shoppinglist/<id>/suggested-items` (`shoppinglist_controller.py`), which the current client does not call.

Three things in that path read as faults:

- `Association.delete_all()` empties the whole table, for every household, each time one household's rules are computed (`backend/app/models/association.py`). Running over all households in a loop leaves only the last household's rules.
- The rule lookup filters on `Association.antecedent_id == recent.id`, the id of a `history` row, not the item id (`getSuggestionsBasedOnLastAddedItems`).
- The frequency fallback selects from `Item` with no household filter, so it can return other households' item names (`getSuggestionsBasedOnFrequency`).

None of these reach the person through the current client, which does not show this endpoint's results.

### Several lists per household

A household has any number of named lists. The one with the lowest id is the default; it cannot be deleted, and a new household gets one called "Default" (`backend/app/models/shoppinglist.py`, `backend/app/controller/household/household_controller.py`). Items are shared across lists, so categories, icons and learned order are the same everywhere. Adding a recipe's ingredients goes to the default list unless another is chosen (`kitchenowl/lib/cubits/recipe_cubit.dart`). Swiping between lists is an open request ([#507](https://github.com/TomBursch/kitchenowl/issues/507)).

## Recipes

### What a recipe holds

| Field | Type | Note |
|---|---|---|
| `name` | string, 128 characters | |
| `description` | text | the method, in Markdown; there is no separate steps field |
| `photo` | file name | one photo per recipe |
| `time`, `prep_time`, `cook_time` | integer minutes | |
| `yields` | integer | servings |
| `source` | string | a URL, or `kitchenowl:///recipe/<id>` for a copy of another KitchenOwl recipe |
| `visibility` | `PRIVATE`, `LINK` or `PUBLIC` | see sharing below |
| `server_curated`, `server_scrapes` | flag, counter | used by the discover page |
| `suggestion_score`, `suggestion_rank` | integers | written nightly for the planner |
| `items` | rows of `recipe_items` | item, `description`, `optional` |
| `tags` | rows of `recipe_tags` | household-wide tag names |
| `created_at`, `updated_at` | timestamps | on every table, from `DbModelTimestampMixin` |

Sources: `backend/app/models/recipe.py`, `backend/app/helpers/db_model_timestamp_mixin.py`.

### Ingredients

An ingredient is a row of `recipe_items` keyed by (recipe, item). It holds the household item, a free-text `description` for the amount and preparation, and an `optional` flag (`backend/app/models/recipe.py`). The written line from the source is not kept. Because the key is (recipe, item), one recipe cannot use the same item twice, for example butter in both dough and filling. Ingredients come back sorted by item name, not in the author's order (`order_by="RecipeItems._name"`). There are no ingredient sections. That is the most-upvoted discussion ([#160](https://github.com/TomBursch/kitchenowl/discussions/160), 8 upvotes, and [#922](https://github.com/TomBursch/kitchenowl/discussions/922)). The maintainer liked "the idea of recipe groups and having only one description (for now at least)" in May 2025 ([comment](https://github.com/TomBursch/kitchenowl/discussions/160#discussioncomment-13237403)).

Optional ingredients start unselected when adding a recipe to the list; the others start selected (`kitchenowl/lib/cubits/recipe_cubit.dart`).

### Ingredients inside the method

The method text can point at ingredients. `@ingredient_name` renders as a chip showing that ingredient and its amount, with underscores for spaces. `@ingredient_name{300g}` overrides the amount shown at that point only ([Markdown tips](https://docs.kitchenowl.org/latest/Tips-%26-Tricks/markdown/), `kitchenowl/lib/helpers/recipe_item_markdown_extension.dart`). The override arrived in v0.7.1 ([#808](https://github.com/TomBursch/kitchenowl/pull/808)). The client also recognises plain ingredient names in the text without the `@`, through `RecipeCubitItemMarkdownBuilder` in the same file. A reference that matches no ingredient stays plain text.

### Scaling and units

Scaling happens in the client. Choosing a different number of servings multiplies every number found in each description by the ratio, through `StringScaler` (`kitchenowl/lib/helpers/string_scaler.dart`). It converts unicode and `a/b` fractions to decimals, scales each number with the `fraction` package, and writes common fractions back as glyphs such as `½`. Every number in the string is scaled, including ones that are not amounts. There is no unit conversion and no unit list; a unit is just part of the text. A planned recipe keeps its chosen servings in `planner.yields` (`backend/app/models/planner.py`).

### Tags, photos, nutrition

Tags are household-wide names (`backend/app/models/tag.py`). The recipe list filters by tags with "any of" semantics (`Recipe.all_by_name_with_filter`). A recipe has one photo. Uploads accept `txt`, `pdf`, `png`, `jpg`, `jpeg`, `gif`, `webp` and `jxl` up to 32 MB, and each image gets a 4 by 3 blurhash for placeholders (`backend/app/config.py`, `backend/app/controller/upload/upload_controller.py`). There is no nutrition data anywhere in the backend or client; a search for "nutri" and "calorie" finds nothing. Calories and cost are an open request ([#638](https://github.com/TomBursch/kitchenowl/issues/638)).

## People, households and permissions

| Role | Where it lives | What it can do |
|---|---|---|
| Server admin | `user.admin` | everything in every household; manage users, read analytics and reports, curate recipes |
| Household owner | `household_member.owner` | as household admin; ownership passes to another member when the owner leaves |
| Household admin | `household_member.admin` | change household settings, add and remove members, delete the household, recalculate balances |
| Household member | a `household_member` row | read and write every list, recipe, plan and expense of the household |

Sources: `backend/app/models/household.py`, `backend/app/helpers/authorize_household.py`, `backend/app/helpers/db_model_authorize_mixin.py`, `backend/app/helpers/server_admin_required.py`.

A household is the unit of ownership. Items, categories, tags, recipes, lists, plans and expenses each carry a `household_id`, and authorisation is "is the user a member of that household" (`DbModelAuthorizeMixin.isAuthorized`). There are no per-recipe or per-list permissions. A server admin passes every household check (`authorize_household`). The first account is created through onboarding and is a server admin (`backend/app/controller/onboarding/onboarding_controller.py`).

A user can belong to several households, and the client switches between them. A household admin adds a person directly by user id with `PUT /api/household/<id>/member/<user_id>`; there is no invitation to accept (`household_controller.py`). To find that id, any signed-in user can search all users on the server by name or username, up to 15 results (`backend/app/controller/user/user_controller.py`, `User.search_name`). The `admin` column on a membership defaults to true, so a person added this way is a household admin unless changed (`HouseholdMember.admin`, `default=True`).

Deleting a household removes its memberships; the monthly job then deletes the empty household with its data (`deleteHouseholdById`, `deleteEmptyHouseholds`).

Accounts sign in by username and password, or by OIDC with Google, Apple or one custom issuer ([OIDC docs](https://docs.kitchenowl.org/latest/self-hosting/oidc/)). Registration is closed by default (`OPEN_REGISTRATION`). An OIDC sign-in with no linked account creates a new account, refused only when the email is already taken (`loginWithOIDC` in `auth_controller.py`), and that path does not consult `OPEN_REGISTRATION`. LDAP is an open request ([#28](https://github.com/TomBursch/kitchenowl/issues/28)).

## Sharing outside the household

A recipe's `visibility` decides who outside the household can read it (`RecipeVisibility` in `backend/app/models/recipe.py`):

- `PRIVATE`: members of the household only.
- `LINK`: anyone who asks for its id. `GET /api/recipe/<id>` takes an optional JWT, so this works without signing in (`getRecipeById` in `recipe_controller.py`).
- `PUBLIC`: as `LINK`, and also listed on the server-wide discover page and in public search and tag search.

Recipe ids are sequential integers, so `LINK` is not a secret link: any id can be tried. A non-member gets `obj_to_public_dict`, which is the full recipe with items and tags, minus planning fields, plus the household's public profile.

The discover page arrived in v0.7.1 ([#753](https://github.com/TomBursch/kitchenowl/pull/753)). It lists popular tags, curated recipes chosen by a server admin, popular recipes ranked by how often they were copied (`server_scrapes`), and newest, all optionally filtered by household language (`suggestedRecipes` in `recipe_controller.py`). Households have a public profile with name, photo, language, description, link and a `verified` flag (`Household.obj_to_public_dict`). A report mechanism lets users flag recipes or users to server admins ([#789](https://github.com/TomBursch/kitchenowl/pull/789), `backend/app/controller/report/report_controller.py`).

Sharing is by copying. "Add to household" on another household's recipe creates a new private recipe whose `source` is `kitchenowl:///recipe/<id>`, and increments the original's `server_scrapes` (`addRecipeToHousehold` in `kitchenowl/lib/cubits/recipe_cubit.dart`, `addRecipe` in `recipe_controller.py`). The copy does not follow later changes to the original. When viewing a recipe from another household, the client looks for a copy in the current household by that source and links to it.

Two privacy faults in this area are on record. v0.7.10 fixed "a vulnerability where a logged-in user was able to access all recipes (including private recipes) on the server", through the planner ([v0.7.10](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.10), [#1119](https://github.com/TomBursch/kitchenowl/pull/1119)). An IDOR on category ids was fixed on `main` after v0.7.10 ([#1155](https://github.com/TomBursch/kitchenowl/pull/1155), [#1154](https://github.com/TomBursch/kitchenowl/issues/1154)). A third is still in the code: `Recipe.search_name` calls `query.filter(cls.household_id == household_id)` without keeping the result, so `GET /api/household/<id>/recipe/search` matches recipe names across every household on the server and returns full recipes (`backend/app/models/recipe.py`). Open PR [#1108](https://github.com/TomBursch/kitchenowl/pull/1108) names it: "`search_name` scoped to current household (was leaking across households)". The client does not call that endpoint; it searches its local cache (see Search). There is no SECURITY.md; two open issues ask how to report security problems ([#918](https://github.com/TomBursch/kitchenowl/issues/918), [#1131](https://github.com/TomBursch/kitchenowl/issues/1131)).

## Recipe changes over time

There is no history of recipe content. A save overwrites the row; the only trace is `updated_at` (`updateRecipe` in `recipe_controller.py`). There are no versions, no undo and no record of who changed what. Concurrent edits are last-write-wins with no conflict check: an update that sends `items` deletes every ingredient whose name is not in the new list and upserts the rest. Edits cannot be made offline (see Offline and mobile). Copies are the only way to keep two variants, and a copy keeps only a `source` pointer to where it came from.

## Cooking

The cooking screen is a step-by-step view built from the Markdown method (`kitchenowl/lib/pages/recipe_cooking_page.dart`). It parses the description, makes each item of each ordered list a separate step, and keeps any text between lists with the step before. It shows a progress bar, a "step n / total" counter, back and next buttons, and below each step the ingredients that step references with `@name`, scaled to the chosen servings (`ExtractItemVisitor` in `kitchenowl/lib/helpers/markdown_extract_item.dart`). It keeps the screen on while open (`WakelockPlus.enable()`). The shopping list has its own keep-screen-on setting since v0.7.7 ([#1001](https://github.com/TomBursch/kitchenowl/pull/1001)).

There are no timers; nothing in the recipe pages starts or parses one. The 2023 request for a step-by-step view is still open with 14 upvotes ([#206](https://github.com/TomBursch/kitchenowl/issues/206)) and no maintainer reply.

## Planning

A plan entry is a row of `planner`: recipe, `cooking_date` and optional `yields`, with (recipe, date) as the primary key (`backend/app/models/planner.py`). A recipe can be planned once per day. An entry with no date sits in a "planned, any day" state stored as `date.min`, and giving it a date replaces it (`addPlannedRecipe` in `backend/app/controller/planner/planner_controller.py`). Only recipes of a household the user belongs to can be planned, after the v0.7.10 fix ([#1119](https://github.com/TomBursch/kitchenowl/pull/1119)). The household can switch the planner and expenses off (`planner_feature`, `expenses_feature`).

`recipe_history` is an event log: one row with status `ADDED` each time a recipe is planned and one with `DROPPED` each time a plan is removed, with no date of cooking and no user (`backend/app/models/recipe_history.py`). Removing a plan is the closest thing to "cooked". The planner shows **recent recipes**: the latest `DROPPED` recipes not currently planned, 10 per page (`RecipeHistory.get_recent`).

**Suggested recipes** come from that log (`backend/app/jobs/recipe_suggestions.py`, `Recipe.compute_suggestion_ranking`):

1. Nightly, each recipe's `suggestion_score` is set to the number of times it was planned between 182 and 7 days ago.
2. Recipes with a score above zero are then drawn at random without replacement, weighted by score, and numbered in draw order as `suggestion_rank`.
3. The planner lists unplanned recipes by rank, 10 at a time. A refresh button reruns the draw (`/refresh-suggested-recipes`).

So suggestions are recipes the household planned in the last six months but not in the last week, shuffled with a bias toward the most planned. A recipe never planned is never suggested. Requests here include a calendar view ([#127](https://github.com/TomBursch/kitchenowl/issues/127)) and meals that are not recipes ([#420](https://github.com/TomBursch/kitchenowl/issues/420)).

## Expenses

An expense has a name, an amount, a date, an optional category, photo and description, a payer, a flag to leave it out of statistics, and a list of people it was paid for, each with an integer weight called `factor` (`backend/app/models/expense.py`). Amounts are floats.

Each member's balance is stored on their membership as `expense_balance` (`backend/app/models/household.py`). It equals what they paid minus their weighted share of each expense they were part of: for each such expense, `factor / sum of factors * amount` (`backend/app/service/recalculate_balances.py`). Adding an expense updates balances incrementally (`addExpense` in `backend/app/controller/expense/expense_controller.py`). Deleting one, importing, or an admin's "recalculate" recomputes every balance from scratch. There is no settle-up suggestion; a member pays another back by entering an expense ([discussion #906](https://github.com/TomBursch/kitchenowl/discussions/906)). An overview endpoint sums by category and by day or month for charts (`getExpenseOverview`).

The currency symbol comes from a locale. Since v0.7.5 it follows the household language rather than the phone's ([#937](https://github.com/TomBursch/kitchenowl/pull/937); maintainer in [#594](https://github.com/TomBursch/kitchenowl/issues/594#issuecomment-3695005876): "The next version will prefer the household language to derive the currency symbol"). Choosing a currency directly is still requested ([#594](https://github.com/TomBursch/kitchenowl/issues/594), 8 upvotes).

## Getting recipes in and out

### Web import

`POST /api/household/<id>/recipe/scrape` takes a URL and returns a draft, not a saved recipe (`backend/app/service/recipe_scraping.py`). It fetches the page through `requests-hardened`, which blocks private and loopback addresses, with a 2 second connect and 10 second read timeout and a user agent naming recipe-scrapers. It then calls `scrape_html(html, url, supported_only=False)` from recipe-scrapers 15.12.0 (`backend/uv.lock`). That reads site-specific scrapers and falls back to schema.org data. A page with no title is "Unsupported website".

The draft gets title, the three times, the first number in the yield string, the site's description and instructions joined into the Markdown description, the image URL and the source URL. Each ingredient line goes through ingredient parsing (below). The parsed name is matched to an existing household item whose name starts with it (`Item.find_name_starts_with`). Unmatched lines come back empty, and the person picks or creates the item in the client before saving. The photo is downloaded when the recipe is saved (`file_has_access_or_download`). The mobile apps accept a shared link and open this flow ([Tips](https://docs.kitchenowl.org/latest/Tips-%26-Tricks/)).

### Ingredient parsing

The default parser is the local NLP model `ingredient-parser-nlp` 2.8.0. It takes the first name and the first amount as `quantity unit`; the comment, usually preparation, is dropped (`parseNLP` in `backend/app/service/ingredient_parsing.py`). The docs say this model is "trained on English ingredients" ([Ingredient Detection](https://docs.kitchenowl.org/latest/self-hosting/ingredient_parsing/)).

Setting `LLM_MODEL` switches to a language model through LiteLLM 1.102.1, so any LiteLLM provider works; the docs show OpenAI, Ollama and OpenRouter. The prompt asks for JSON `[{"name", "description"}]`, a singular name, and translation into the household's language (`parseLLM`). If the reply has a different number of entries, fails to parse, or the call errors, the NLP parser runs instead. This is the only built-in AI feature. A maintainer attempt at whole-page LLM scraping did not ship: "couldn't get the output to be reliably good" ([#670](https://github.com/TomBursch/kitchenowl/issues/670#issuecomment-2708802338)).

### From another KitchenOwl

The scrape endpoint recognises KitchenOwl recipe links (`scrape` in `recipe_scraping.py`):

- A link on the same server, `kitchenowl:///recipe/<id>` or `FRONT_URL/recipe/<id>`, copies that recipe with its items, if it is not private to another household.
- A link to `https://app.kitchenowl.org/recipe/<id>`, or to any page ending `/recipe/<id>` whose HTML holds `<title>KitchenOwl</title>`, fetches `<that server>/api/recipe/<id>` without credentials. Only `LINK` and `PUBLIC` recipes answer. Since [#1170](https://github.com/TomBursch/kitchenowl/pull/1170) that fetch also goes through the hardened client.

### Files

Export is per household. `GET /api/household/<id>/export` returns one JSON object with the household's name, language, settings, member usernames, list names, recipes, items and expenses (`Household.obj_to_export_dict`). Recipes export as name, description, times, yields, source, photo file name, items as name, description and optional flag, and tag names (`Recipe.obj_to_export_dict`). Photos are not included, only their server file names. Shopping list contents, plans, recipe history and item history are not exported.

Import takes that same JSON (`backend/app/controller/exportimport/import_controller.py`). A recipe whose name exists is imported with a numbered suffix such as "Soup (2)", or overwrites the existing one when `recipe_overwrite` is set (`backend/app/service/importServices/import_recipe.py`). There is no importer for any other app's format. The maintainer in 2023: "currently only a very specific JSON format is supported ... I wouldn't consider the import API stable" ([discussion #155](https://github.com/TomBursch/kitchenowl/discussions/155#discussioncomment-5840041)). Requests for Mealie, Tandoor, Nextcloud Cookbook, Thermomix and generic file import are all open ([#90](https://github.com/TomBursch/kitchenowl/issues/90), [#97](https://github.com/TomBursch/kitchenowl/issues/97), [#83](https://github.com/TomBursch/kitchenowl/issues/83), [#878](https://github.com/TomBursch/kitchenowl/issues/878), [#291](https://github.com/TomBursch/kitchenowl/issues/291)). The management script can also import files from the upload folder into the `file` table ([Server Management](https://docs.kitchenowl.org/latest/reference/management/), `backend/manage.py`).

### Backups

The self-hosted server has no backup feature. Nothing in the backend writes an archive; a backup means copying `/data`, which holds `database.db` and `upload/` (`backend/app/config.py`). The per-household JSON export is the only built-in way out.

## AI and agents

### API tokens

A user can mint a **long-lived token** with `POST /api/auth/llt` and a device name, and revoke it by id (`createLongLivedToken` in `auth_controller.py`). It is an access JWT with no expiry, stored in the `token` table with type `llt` (`Token.create_longlived_token`). It has no scopes: it carries every right of the user who made it, in every household. The settings page lists sessions and long-lived tokens (`kitchenowl/lib/pages/settings_user_sessions_page.dart`).

### The MCP server

v0.7.7 added an MCP server, "MCP tools for full management of household" ([#1020](https://github.com/TomBursch/kitchenowl/pull/1020) by w0s1nsk1, merged 2026-03-24), after the discussion "Is there an MCP Server?" ([#868](https://github.com/TomBursch/kitchenowl/discussions/868), 7 upvotes).

It is off by default. Setting `KITCHENOWL_MCP_ENABLED=true` registers it at `/mcp` (`backend/app/api/register_controller.py`), and the compose files carry that line commented out (`docker-compose.yml`, `docker-compose-single.yml`). The docs site does not mention it; the advanced configuration page has no "MCP" on it on 2026-10-22.

It is hand-written, with no MCP SDK (`backend/app/controller/mcp_controller.py`, 1,217 lines):

- **Transports.** Streamable HTTP as a stateless `POST /mcp` that answers with JSON; `GET` and `DELETE` return 405. Also the older HTTP+SSE pair, `GET /mcp/sse` and `POST /mcp/messages`, whose sessions live in one worker's memory. The SSE half was fixed after v0.7.10 to "make the SSE transport usable" ([#1121](https://github.com/TomBursch/kitchenowl/pull/1121)).
- **Protocol.** JSON-RPC with `initialize`, `ping`, `tools/list` and `tools/call`; batches accepted. It answers protocol versions `2025-06-18`, `2025-03-26` and `2024-11-05`, and sends server instructions telling the model to call `list_households` first.
- **Auth.** `@jwt_required()`, so any access token, including a long-lived token, as a bearer header. Tools check household membership; a server admin gets no extra reach here.
- **Tools.** A Python dict maps each name to a hand-written JSON Schema and a function. Each call's result is returned both as `structuredContent` and as JSON text, and the database transaction is committed once per call.

The 27 tools:

| Area | Tools |
|---|---|
| Households | `list_households` |
| Shopping lists | `list_shoppinglists`, `list_shoppinglist_items`, `add_item_by_name`, `remove_item_from_list`, `create_shoppinglist`, `delete_shoppinglist` |
| Items and tags | `list_items`, `list_tags`, `create_tag` |
| Recipes | `list_recipes`, `search_recipes`, `get_recipe`, `create_recipe`, `update_recipe`, `delete_recipe`, `add_recipe_item`, `remove_recipe_item`, `add_recipe_tag`, `remove_recipe_tag`, `scrape_recipe` |
| Planner | `list_planner`, `add_planner_entry`, `remove_planner_entry` |
| Expenses | `list_expenses`, `create_expense`, `delete_expense` |

Only `list_recipes` and `search_recipes` have written descriptions; every other tool is described as "KitchenOwl tool: <name>" (`TOOL_DESCRIPTIONS`). Those two page by `offset` and `limit` (1 to 100, default 50) and return only id and name unless `additional_fields` asks for more, a change made to cut response size ([#1145](https://github.com/TomBursch/kitchenowl/pull/1145)). `scrape_recipe` returns the same draft as the app and saves nothing.

The tools write rows directly instead of calling the REST handlers, so they skip some of what the app does on the same action:

- No Socket.IO events are emitted, so other devices see MCP changes only at their next 30-second refresh.
- `add_planner_entry` writes no `recipe_history` row, so MCP plans do not feed suggestions.
- `create_expense` records the caller as payer with no "paid for" rows and does not update balances; `delete_expense` does not recalculate them.
- `add_item_by_name` leaves an existing entry's description unchanged.

Before this server, the project had a Python client and a Home Assistant integration, both separate repositories (`README.md`).

## Search

The client searches recipes in its own cache: it fuzzy-matches the query against recipe names with the `fuzzywuzzy` package and a cutoff of 50 (`TransactionRecipeSearchRecipes` in `kitchenowl/lib/services/transactions/recipe.dart`). It searches names only, not ingredients, tags or method text. Tags work as a filter.

Item search goes to the server (`TransactionShoppingListSearchItem`). On PostgreSQL it ranks by Levenshtein distance between the query and the start of each name, up to 3 edits, then by `support`. On SQLite it collects, in order, names starting with the query, names containing it, and names matching with any one character wrong, ordered by `support`, up to 11 results (`Item.search_name`). `*` and `?` work as wildcards. Since [#1172](https://github.com/TomBursch/kitchenowl/pull/1172), sorting and searching on the list ignore diacritics. A slow item search is an open bug ([#1016](https://github.com/TomBursch/kitchenowl/issues/1016), 6 upvotes).

Public recipe search across the server uses the same name-only matching, limited to `PUBLIC` recipes (`searchAllRecipeByName`).

## Offline and mobile

Native apps exist for Android (Google Play, F-Droid, APK), iOS and macOS (App Store), Windows, Linux (deb, rpm, tarball) and the web; the v0.7.10 release attaches builds for all but iOS (`README.md`, release assets).

The README promises "partial offline support". In code, every read and write is a `Transaction` with `runOnline` and `runLocal` (`kitchenowl/lib/services/transaction.dart`). When the server answers, results are written to a per-household JSON cache in the app's cache folder: user, households, shopping lists, recipes, categories and tags (`kitchenowl/lib/services/storage/temp_storage.dart`). Offline, reads come from that cache.

Seven write types are queued when offline and replayed on reconnect, from `transactions.json` (`kitchenowl/lib/services/transaction_handler.dart`, `saveTransaction => true`):

- add item, remove item, remove several items, update item description, add recipe items to a list;
- add a recipe to the planner, remove it.

Everything else, including any recipe or expense edit, needs the server. Replay sends each queued call as is; there is no merge beyond the server's own rules. The queue is meant to drop entries older than three days, but the check computes `timestamp.difference(now).inDays < 3`, which is negative for any past entry, so as written nothing is dropped.

The web build caches nothing and queues nothing: both storage classes return early under `kIsWeb`. Android runs a background fetch that replays the queue and refreshes the last household's lists (`kitchenowl/lib/services/background_task.dart`). It came in v0.7.5 ([#898](https://github.com/TomBursch/kitchenowl/pull/898)), was disabled on iOS in v0.7.10 ([#1120](https://github.com/TomBursch/kitchenowl/pull/1120)) and re-enabled on `main` on 2026-08-25. Login and session loss on mobile recur in the tracker: [#400](https://github.com/TomBursch/kitchenowl/issues/400), [#626](https://github.com/TomBursch/kitchenowl/issues/626), [#1130](https://github.com/TomBursch/kitchenowl/issues/1130), and the iOS logout fix in v0.7.7 ([#1042](https://github.com/TomBursch/kitchenowl/pull/1042)). Home screen widgets ([#287](https://github.com/TomBursch/kitchenowl/issues/287)), voice assistants ([#117](https://github.com/TomBursch/kitchenowl/issues/117), [#245](https://github.com/TomBursch/kitchenowl/issues/245)) and notifications ([#679](https://github.com/TomBursch/kitchenowl/issues/679)) are open requests.

## Languages and translation

The client ships 49 ARB translation files (`kitchenowl/lib/l10n/`), translated on Hosted Weblate (`README.md`). The backend lists 44 household languages, from Arabic to Blissymbols (`SUPPORTED_LANGUAGES` in `backend/app/config.py`), and ships 44 item template files (`backend/templates/l10n/`). A household's language picks its default items and categories, its currency format, and the target language of LLM ingredient parsing. It can be set once: `updateHousehold` only accepts a language when none is set ([discussion #919](https://github.com/TomBursch/kitchenowl/discussions/919) asks how to change it). Recipe content is not translated. Choosing the interface language separately from the system is an open request ([#235](https://github.com/TomBursch/kitchenowl/issues/235)).

## Deployment and operations

| Topic | What the project does | Source |
|---|---|---|
| Images | `tombursch/kitchenowl` all-in-one on port 8080; or `tombursch/kitchenowl-web` on port 80 plus `tombursch/kitchenowl-backend` speaking uwsgi on 5000 | [Getting Started](https://docs.kitchenowl.org/latest/self-hosting/), `Dockerfile` |
| Tags | `latest`, `beta`, `dev` and one per version | [Advanced](https://docs.kitchenowl.org/latest/self-hosting/advanced/) |
| User | root; the `Dockerfile` sets no `USER` | `Dockerfile`, [#684](https://github.com/TomBursch/kitchenowl/issues/684) |
| Data | one volume, `/data`, with `database.db` and `upload/` | `Dockerfile`, `backend/app/config.py` |
| Database | SQLite, or PostgreSQL through `DB_DRIVER` and `DB_*` variables | [Advanced](https://docs.kitchenowl.org/latest/self-hosting/advanced/) |
| Secrets | `JWT_SECRET_KEY` with a placeholder default; most secrets also accept a `_FILE` variant | `Dockerfile`, `backend/app/config.py` |
| Sign-in | username and password, OIDC with Google, Apple or one custom issuer; password login can be turned off | [OIDC](https://docs.kitchenowl.org/latest/self-hosting/oidc/) |
| Email | optional SMTP for password reset and email verification | [Advanced](https://docs.kitchenowl.org/latest/self-hosting/advanced/) |
| Upgrades | `docker compose pull` then `up -d`; the entrypoint runs `flask db upgrade` and refreshes template items on every start | [Getting Started](https://docs.kitchenowl.org/latest/self-hosting/), `backend/entrypoint.sh` |
| Scaling | override the uWSGI command for more greenlets; a RabbitMQ broker for jobs | `docker-compose-postgres.yml`, `docker-compose-rabbitmq.yml` |
| Admin tasks | `python manage.py` inside the container: users, households, files, run all jobs | [Server Management](https://docs.kitchenowl.org/latest/reference/management/) |
| Reverse proxy | recommended with HTTPS, HSTS and websockets | [Getting Started](https://docs.kitchenowl.org/latest/self-hosting/) |

On running as root, the maintainer said: "Running the containers rootless has been implemented for a while now (#50). But changing the default user would be a breaking change, which I want to avoid" ([#684](https://github.com/TomBursch/kitchenowl/issues/684#issuecomment-2734618579)). The backend image builds NumPy, pandas and mlxtend for the suggestion jobs, and NLTK data for the NLP parser (`Dockerfile`). One uWSGI worker at about 400 MB of memory prompted [discussion #818](https://github.com/TomBursch/kitchenowl/discussions/818), and NumPy's CPU baseline broke older machines ([#961](https://github.com/TomBursch/kitchenowl/issues/961)).

## What users ask for and complain about

Most-upvoted open issues on 2026-10-22, by thumbs-up:

| Issue | Thumbs up | Opened |
|---|---|---|
| [#123](https://github.com/TomBursch/kitchenowl/issues/123) Images on shopping list items | 18 | 2023-01 |
| [#90](https://github.com/TomBursch/kitchenowl/issues/90) Mealie integration, to sync shopping lists | 18 | 2022-11 |
| [#206](https://github.com/TomBursch/kitchenowl/issues/206) A step-by-step option for recipes | 14 | 2023-07 |
| [#293](https://github.com/TomBursch/kitchenowl/issues/293) Custom icons for list entries | 14 | 2023-11 |
| [#10](https://github.com/TomBursch/kitchenowl/issues/10) An integrated pantry list | 10 | 2022-02 |
| [#97](https://github.com/TomBursch/kitchenowl/issues/97), [#83](https://github.com/TomBursch/kitchenowl/issues/83) Tandoor and Nextcloud Cookbook integration | 9 each | 2022 |
| [#127](https://github.com/TomBursch/kitchenowl/issues/127) Calendar; [#117](https://github.com/TomBursch/kitchenowl/issues/117) Google Assistant; [#287](https://github.com/TomBursch/kitchenowl/issues/287) Widgets | 9 each | 2023 |
| [#594](https://github.com/TomBursch/kitchenowl/issues/594) Different currencies for balances | 8 | 2024-12 |

Most-commented open issues are bugs: connection loss ([#239](https://github.com/TomBursch/kitchenowl/issues/239), 24 comments), user-installed certificates on Android ([#236](https://github.com/TomBursch/kitchenowl/issues/236), 22), logged out after a browser refresh ([#400](https://github.com/TomBursch/kitchenowl/issues/400), 19) and ARM 32-bit images ([#46](https://github.com/TomBursch/kitchenowl/issues/46), 19). OIDC trouble with Authentik and Authelia recurs ([#391](https://github.com/TomBursch/kitchenowl/issues/391), [#855](https://github.com/TomBursch/kitchenowl/issues/855)).

The top discussions are ingredient sections ([#160](https://github.com/TomBursch/kitchenowl/discussions/160), 8 upvotes), an MCP server ([#868](https://github.com/TomBursch/kitchenowl/discussions/868), 7, since answered by #1020), currency options ([#141](https://github.com/TomBursch/kitchenowl/discussions/141)) and importing from other apps ([#155](https://github.com/TomBursch/kitchenowl/discussions/155)).

The maintainer rarely comments on the most-upvoted requests. Of the top ten open issues, only #293 and #594 have a reply from him. On #293 he explained that a predefined icon list exists and the issue is about uploading new icons ([comment](https://github.com/TomBursch/kitchenowl/issues/293#issuecomment-3227306230)).

## Recent direction

The last releases, newest first:

- **v0.7.10, 2026-07-26:** private-recipe exposure through the planner fixed, SQLite WAL, iOS background sync turned off ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.10)).
- **v0.7.9, 2026-06-04:** past recipes hidden when adding items, past plans shown faded ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.9)).
- **v0.7.8, 2026-04-26:** the return after the account suspension, an OIDC launch parameter, fixes ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.8)).
- **v0.7.7, 2026-04-03:** the MCP server, keep-screen-on for the list, 32 MB uploads, Android sync and iOS session fixes ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.7)).
- **v0.7.5, 2026-01-10:** background fetch, currency from the household language, a minimalist list style ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.5)).
- **v0.7.1, 2025-06-29:** the discover page, per-occurrence ingredient amounts in the method, reporting ([release](https://github.com/TomBursch/kitchenowl/releases/tag/v0.7.1)).

Since v0.7.10, `main` has gained a usable MCP SSE transport, a smaller MCP recipe list, long-press to open a URL in an item description ([#1132](https://github.com/TomBursch/kitchenowl/pull/1132)), the category IDOR fix, diacritic-insensitive list sorting and a Socket.IO origin fix ([#1166](https://github.com/TomBursch/kitchenowl/pull/1166)). Releases have slowed from 30 in 2022 to 6 so far in 2026, and recent work is mostly fixes, security fixes and outside contributions. The open PR [#1108](https://github.com/TomBursch/kitchenowl/pull/1108) proposes paginated "slim" recipe lists with a delta-sync endpoint and tombstones for large collections.

The README and docs point to a public roadmap on a GitHub project board ([users/TomBursch/projects/1](https://github.com/users/TomBursch/projects/1)). The GraphQL API reports it as "KitchenOwl Project", last updated 2026-08-25, with 0 items visible, and the page itself showed "This project failed to load" on 2026-10-22.

## Not established

- The roadmap board's contents. The API shows no items to this account and the page did not load, so it may be empty, private in part, or broken.
- Whether the household recipe search leak, the cross-household item suggestions and the association table wipe behave as read. None was run live.
- Whether the Socket.IO room list updates when a user joins a household while connected. The code joins rooms only on connect, which suggests a reconnect is needed.
- How the client renders a plain ingredient name in the method without `@`, in detail; `RecipeCubitItemMarkdownBuilder` was read only in part.
- The size and use of the hosted service at `app.kitchenowl.org`: no figures are published.
- Total Docker pulls and app store install counts; the badges were not read.
- When the cooking screen was added; the release notes read here do not name it, and #206 stays open beside it.
- Whether `app.kitchenowl.org` runs the same code as `main`, an earlier release, or something else.
