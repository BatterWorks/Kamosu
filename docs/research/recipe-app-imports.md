# Importing from other recipe apps: which apps, what their files hold, and where the test files come from

**Spec:** #411, with tickets #412 to #418.
**Purpose:** the evidence behind which apps Kamosu imports from, in what order, and how each reader is tested. Wave 2 lives here until it gets tickets.
**Method:** each project's own source code on its default branch, read through `gh api` on 2026-10-21, plus the selfh.st annual survey's raw data. Stars are a rough signal only; they were read together with activity and survey mentions.

## Which apps people run

| App | Stars | Last push | What it is |
|---|---|---|---|
| Mealie | 13,642 | 2026-10-21 | Recipe manager and meal planner. The only recipe app in the selfh.st survey's favourite-software list (#22 in 2024, #17 in 2025) |
| Grocy | 9,604 | 2026-09-16 | Household inventory; recipes are a side module tied to stock |
| Tandoor Recipes | 8,673 | 2026-10-21 | Recipes, meal plans, shopping. AGPL with the Commons Clause |
| KitchenOwl | 3,761 | 2026-10-18 | Shopping list first, recipes second |
| CookCLI (Cooklang) | 1,415 | 2026-10-20 | A folder of `.cook` text files with a command line and web server |
| Norish | 1,370 | 2026-10-18 | Real-time family recipe app |
| Bar Assistant | 1,109 | 2026-10-15 | Cocktails only |
| RecipeSage | 976 | 2026-10-20 | Recipe keeper, meal planner, shopping list |
| Nextcloud Cookbook | 646 | 2026-10-21 | Nextcloud add-on; each recipe is a file in the user's Nextcloud |
| Recipya | 411 | 2026-10-16 | Go recipe manager; last release December 2024, a Rust rewrite under way |

Discussion on r/selfhosted is about Mealie against Tandoor. Mealie leads every measure; Tandoor is a clear second.

## What people migrate from

Which formats the self-hosted apps themselves import is evidence of where their newcomers come from:

| Format | Mealie | Tandoor | RecipeSage | Norish | Recipya | Count |
|---|---|---|---|---|---|---|
| Paprika | yes | yes | yes | yes | yes | 5 |
| Recipe Keeper | yes | yes | yes | yes | | 4 |
| Mealie | own older format | yes | | yes | API | 3 |
| Tandoor | yes | | | yes | API | 3 |
| Nextcloud Cookbook | yes | yes | | | API | 3 |
| Copy Me That | yes | yes | yes | | | 3 |
| Mela | | yes | yes | yes | | 3 |
| schema.org JSON-LD file | | yes | yes | | yes | 3 |
| Plan to Eat, Chowdown, Cookmate, Pepperplate, CookBook Manager, Crouton | | | | | | 2 each |

Formats only one app imports: Cook'n, My Recipe Box (Mealie); MealMaster, RezKonv, Rezeptsuite.de, RecetteTek, Saffron, Domestica, OpenEats, ChefTap, Gourmet, Pestle, Cooklang (Tandoor); Living Cookbook, Evernote, Flavorish, CSV (RecipeSage); MasterCook (Recipya).

## What each export holds

**Mealie, ordinary export.** Data Management, Recipes, Export All; needs the organise permission, not admin. A zip of `recipes/<slug>/<slug>.json` plus that recipe's images and assets. Ingredients carry quantity, unit, food, note, a `display` line, an `original_text` (the wording first imported, not the current one) and a section `title`. A line can point at another recipe through `referenced_recipe`, which holds a full copy of it; the `display` line leaves that recipe's name out, and Mealie's screen adds it back. Step pictures are image links into the recipe's assets. Also tags, categories, tools, an averaged rating, comments, nutrition, notes. No cooking history.

**Mealie, admin backup.** A zip of `database.json` (every table, the whole server) plus the data folder. It holds `recipe_timeline_events`: "I Made This" entries are `comment` events with a chosen date, a message and an optional photo; meal-plan entries are `info` events. The subject is a translated sentence, so match on the event type. Recipes reach their household through their author's user row. The backup stores no `display` line; Mealie builds it on load from quantity, unit flags, food plurals and note. Also holds password hashes and tokens, which an importer has no reason to read.

**Tandoor.** Settings, Space, Export, all recipes, open to ordinary users. The default format is a zip of `<id>.zip`, each holding `recipe.json` and one image (named `image` with no extension in Tandoor 2). Ingredients live inside steps as amount, unit, food, note, header flag; the written line is not exported, nor is a step's link to another recipe, the cooking log or ratings. A second export format writes schema.org JSON-LD, lossy and without images.

**Paprika.** `.paprikarecipes` is a zip of gzipped JSON per recipe. Ingredients and directions are one string each. `photo_data` is the cook's photo in base64; `image_url` is the source website's picture, not a larger copy of the cook's. Also categories, rating, free-text nutrition, notes, source and link. No cooking history.

**Nextcloud Cookbook.** No export button: recipes are files, `Recipes/<name>/recipe.json` (schema.org Recipe) beside `full.jpg` and thumbnails, downloaded as a folder zip.

**RecipeSage.** Settings, Export, JSON-LD: one array of schema.org Recipes; images as addresses, not files.

**Recipya.** A zip of `<recipe>/recipe.json` in schema.org with images.

**CookCLI.** `cook recipe -f schema` converts one recipe to schema.org.

**Bar Assistant.** A zip export in several formats, schema.org and JSON-LD among them.

**KitchenOwl.** Household settings, Danger Zone, Export: one JSON file. A recipe has name, description (the steps, in markdown), times, yields, source, tags, and items as name plus free-text amount. No recipe id, and `photo` is only a file name on the server; the image is not in the file.

**Grocy.** No export, only a REST API.

**Common ground.** No shared format, but schema.org Recipe comes closest: Nextcloud Cookbook stores it, RecipeSage and Recipya export it, CookCLI, Bar Assistant and Tandoor can write it, and Mealie puts it on every recipe page. Mealie and Tandoor keep their structure only in their own formats.

## Real test files that already exist

| Format | Real file | Source and licence | Gaps |
|---|---|---|---|
| Mealie export, before v1 | 3 recipes, webp images | Mealie `tests/data/migrations/mealie.zip`, AGPL | sections, ratings |
| Mealie export, v2 | 10 recipes (French, German, Czech), images | vanilla-cookbook `src/lib/data/import/mealie.zip`, GPL-3.0 | `referenced_recipe`, sections, households |
| Mealie backup | every era from 2022 to 2026 | Mealie `tests/data/backups/`, AGPL; anonymised, text scrambled, no images | real text, images, readable event types, two households |
| Tandoor default | 1 recipe (1.x); 10 recipes (2.0) | Mealie `migrations/tandoor.zip`, AGPL; vanilla-cookbook `tandoor.zip`, GPL-3.0 | headers, nutrition |
| Paprika | 5 recipes (4 German), copied from websites | Mealie `migrations/paprika.zip`, AGPL | none for the reader |
| Paprika, large | 746 recipes, 670 photos, 22 MB | vanilla-cookbook `paprika.paprikarecipes`, GPL-3.0; one person's collection | kept out of git, run on request |
| Nextcloud Cookbook | 3 recipes with images; 19 without | Mealie `migrations/nextcloud.zip`, AGPL; nextcloud-cookbook-flutter `docker/data/Recipes/`, CC0 | recent images |
| RecipeSage | 10 recipes, no images | vanilla-cookbook `recipe_sage.json`, GPL-3.0 | images |
| KitchenOwl, Recipya, CookCLI, Bar Assistant | none anywhere | | everything |

GPL-3.0 and AGPL-3.0 combine under section 13 of each. The licence covers the files, not necessarily the recipe text: Mealie's samples include recipes copied from websites, and they are committed as Mealie publishes them. The gaps are filled by running each app's current release once in a throwaway container (Mealie, Tandoor, KitchenOwl, Recipya, Bar Assistant) or from its release binary (CookCLI).

## Wave 2

Not yet ticketed. Each starts by looking for real export files, since a format is supported only when a real file of it is in the tests.

- **Recipe Keeper**: in four importer lists.
- **Copy Me That**: in three.
- **Mela**: in three.
- **Norish**: its own documented, versioned `.norishrecipes` zip, with step images, ratings and favourites.
