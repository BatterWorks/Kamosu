# KitchenOwl exports

Real files KitchenOwl wrote, which the KitchenOwl reader (`src/kitchenowl.rs`,
#417) is tested against. Neither was edited after KitchenOwl wrote it. A shape
of KitchenOwl's file is supported exactly when a file of it is here.

No real KitchenOwl export was published anywhere to borrow, so both were made
for Kamosu.

## Made for Kamosu

Made on 23 October 2026 with KitchenOwl 0.7.10
(`docker.io/tombursch/kitchenowl:v0.7.10`), run once in a throwaway container
on its own SQLite database. Everything was entered through KitchenOwl's own
API, the calls its app makes: a first account, a household named *Home* in
English, with the meal planner and expenses turned on, which brings in
KitchenOwl's 660 default items. The files are what the household's settings ›
Danger Zone › Export downloads, `GET /api/household/<id>/export`, which the
app saves as `Home_export.json`.

- *Orange cake*: in English, 8 for yields, 20 minutes of prep, 45 of cooking
  and 65 in all, a source link, two tags and a photo. Its description opens
  with a paragraph, then a heading *Cake* over a numbered list, one item
  carried onto a second line, a heading *Syrup* over the list's last two, and
  a paragraph after it. One item has no amount, one has more after its amount,
  `2, zest and juice`, and *Icing sugar* is optional, `to dust`.
- *Pâtes à la tomate*: in French, 2 for yields, 12 minutes of cooking and 20 in
  all, two tags and a photo. Its description is a paragraph, a bulleted list
  and a last paragraph. *Parmesan* is optional, `un peu`.
- *Lemonade*: in English, 4 for yields, 10 minutes in all and no other time, a
  source that is words and no link, one tag and no photo. Its description is
  three lines and no list. *Mint* is optional, `a few leaves`.
- The household's *Default* shopping list holds milk, and a second list,
  *Weekend market*, strawberries. One expense, *Groceries*, 23.50.

| File | What KitchenOwl made |
|---|---|
| `v0.7.10-export.json` | The export of the household as above |
| `v0.7.10-export-renamed.json` | The same household exported again after *Lemonade* was renamed *Fresh lemonade* in KitchenOwl |

KitchenOwl writes a recipe's items in order of their names, not in the order
they were entered, and each photo as the name of a file on its server, with no
picture in the export. The photos uploaded were from the Sample Library
(`sample-library/CREDITS.md`): the cake's is the Orange cake (Andy Li, CC0
1.0), the pasta's the Weeknight Tomato Pasta (public domain).
