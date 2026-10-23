# Mealie exports

Real files Mealie wrote, which the Mealie reader (`src/mealie.rs`, #413) and the
Mealie backup reader (#418) are tested against. None was edited after Mealie
wrote it. A shape of Mealie's file is supported exactly when a file of it is
here.

## Borrowed

| File | What it is | Where it comes from |
|---|---|---|
| `pre-v1-export.zip` | An export from before Mealie 1, December 2021: three recipes copied from websites, with their pictures, made on a Mac | Mealie's own `tests/data/migrations/mealie.zip`, at commit `cb59fad8f0187b1972ac040bf06aca0a34967c44`. © the Mealie contributors, [AGPL-3.0](https://github.com/mealie-recipes/mealie/blob/mealie-next/LICENSE) |
| `v2-export.zip` | An export of August 2025: ten recipes in French, German, Czech, English and Spanish, with pictures, step headings and a picture in a description | vanilla-cookbook's `src/lib/data/import/mealie.zip`, at commit `e842601e109adf63667bae84a5ff28c96a1b4345`. © the vanilla-cookbook contributors, [GPL-3.0](https://github.com/jt196/vanilla-cookbook/blob/main/LICENSE) |

GPL-3.0 and AGPL-3.0 combine under section 13 of each. Mealie's sample holds
recipes copied from websites, and it is kept here as Mealie publishes it.

## Made for Kamosu

Made on 23 October 2026 with Mealie v3.29.0
(`ghcr.io/mealie-recipes/mealie:v3.29.0`), run once in a throwaway container.
The recipes were entered through Mealie's own API, the calls its screens make,
and the files were made by Mealie's own export and backup.

- Sam Cook administers it, in the household Mealie made first, *Family*. Alex
  Baker cooks in a second household, *Cabin*.
- *Pizza dough*: parsed lines, a yield of 2 balls, a category, a tag, a tool
  and a note. No picture.
- *Margherita pizza*: its first line points at the Pizza dough (`1 batch`),
  section titles on lines and on Steps, a Step heading, a picture dropped into
  its second Step, a second picture and a text file among its assets, two
  tools, a rating and a favourite, a comment from each cook, calories, a
  source link, an "I Made This" by each cook (Sam's with a photo) and a meal
  plan entry.
- *Tarte aux pommes*: French, with lines typed and never parsed.
- *Lemon drizzle cake*: Alex's, in the Cabin, with an "I Made This" with a
  photo.

| File | What Mealie made |
|---|---|
| `v3-export.zip` | Data Management › Recipes › Export, by Sam, of every recipe of the Family: the dough, the pizza and the tart. The export holds its own household alone |
| `v3-export-pizza-alone.zip` | The same export of the pizza alone, whose dough line still carries Mealie's copy of the dough |
| `v3-export-pizza-changed.zip` | The export of the Family again, after the pizza's last Step was changed to bake for 7 minutes instead of 8 |
| `v3-backup.zip` | The administrator's backup of the whole instance, both households and their cooking history included. Its passwords, tokens and secrets are those of the throwaway instance, which no longer exists |

The pictures are from the Sample Library (`sample-library/CREDITS.md`): the
pizza's photo is the Weeknight Tomato Pasta (public domain), its second picture
and the cake's photo the Orange cake (Andy Li, CC0 1.0), and the dough in the
Step, the tart, Sam's "I Made This" and Alex's are the maintainer's own Beef
bourguignon, Torrijas, T-bone steak and Cold salmon salad, under this
repository's licence.
