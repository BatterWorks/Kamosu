# Tandoor exports

Real files Tandoor wrote, which the Tandoor reader (`src/tandoor.rs`, #415) is
tested against, and the recipe-file reader (`src/schema_org.rs`, #416) the two
schema.org ones. None was edited
after Tandoor wrote it. A shape of Tandoor's file is supported exactly when a
file of it is here.

## Borrowed

| File | What it is | Where it comes from |
|---|---|---|
| `v1-mealie-sample.zip` | A Tandoor 1 export of June 2023: one recipe, copied from a website, with its picture as `image.jpeg`. Its file has no `source_url` | Mealie's own `tests/data/migrations/tandoor.zip`, at commit `cb59fad8f0187b1972ac040bf06aca0a34967c44`. © the Mealie contributors, [AGPL-3.0](https://github.com/mealie-recipes/mealie/blob/mealie-next/LICENSE) |
| `v2-schema-org.json` | The same ten recipes exported from Tandoor 2 as *Recipe Sage*, which is schema.org JSON-LD: a bare array, each line written as `1.0 Taza de harina`, the times as bare minutes. vanilla-cookbook calls it a RecipeSage export, but RecipeSage writes neither these lines nor this array | vanilla-cookbook's `src/lib/data/import/recipe_sage.json`, at commit `e842601e109adf63667bae84a5ff28c96a1b4345`. © the vanilla-cookbook contributors, [GPL-3.0](https://github.com/jt196/vanilla-cookbook/blob/main/LICENSE) |
| `v2-export.zip` | A Tandoor 2 export of August 2025: ten recipes in English, French, German, Czech and Spanish, most of them brought into Tandoor from Mealie, five with a picture named `image` with no extension | vanilla-cookbook's `src/lib/data/import/tandoor.zip`, at commit `e842601e109adf63667bae84a5ff28c96a1b4345`. © the vanilla-cookbook contributors, [GPL-3.0](https://github.com/jt196/vanilla-cookbook/blob/main/LICENSE) |

GPL-3.0 and AGPL-3.0 combine under section 13 of each. Mealie's sample holds a
recipe copied from a website, and it is kept here as Mealie publishes it.

## Made for Kamosu

Made on 23 October 2026 with Tandoor 2.6.15
(`docker.io/vabene1111/recipes:2.6.15`), run once in a throwaway container on
its own SQLite database. The recipes were entered through Tandoor's own API,
the calls its recipe editor makes, and the files were made by Tandoor's own
export, Settings › Space › Export, with All Recipes ticked. Tandoor 2's screens
no longer edit a recipe's nutrition, which a recipe brought in from another app
still carries, so the nutrition was set through the same API.

- *Orange cake*: in English, 8 slices, a source link, two keywords, nutrition,
  a working and a waiting time. Its first step is named *Cake* and holds two
  header rows, *Wet* and *Dry*, a line with a note, a food with a plural, and
  salt marked as having no amount. Its second step is unnamed, with no
  ingredients and a time of its own, 45 minutes. Its third is named *Syrup*.
- *Pâtes à la tomate*: in French, 2 servings with no word for them, two
  keywords and nutrition. Its first step is named *Sauce*, its second is
  unnamed with a line of its own and a time of 12 minutes, and its third has
  no ingredients.

| File | What Tandoor made |
|---|---|
| `v2.6-export.zip` | The export in Tandoor's default format: a zip of `1.zip` and `2.zip`, each holding `recipe.json` and `image.jpg` |
| `v2.6-schema-org.json` | The same two recipes exported as *Recipe Sage*, which is schema.org JSON-LD, for #416. Tandoor writes each line here as `225.0 g butter`, a header as `0.0 None None`, and no picture |

The pictures are from the Sample Library (`sample-library/CREDITS.md`): the
cake's is the Orange cake (Andy Li, CC0 1.0), the pasta's the Weeknight Tomato
Pasta (public domain).
