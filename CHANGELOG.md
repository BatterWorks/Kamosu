# Changelog

## Unreleased

### After upgrading

- An Operator presses "Read every line again" on the Operator page once. The reader of Ingredient Lines was fixed in four places, listed under Fixed, and a line already saved keeps its old Reading until it is read again. A Reading somebody set by hand is left alone. [#207] [#208] [#213] [#230]

### Added

- Kamosu has a layout for tablets and computers. A window at least 700 pixels wide and 560 tall gets a sidebar down the left edge with Home, Recipes, Shopping, Cooked and Settings, where the phone has its tab bar. The phone layout stays. [#193] [#194]
- On that layout a bottom sheet opens as a window in the middle of the screen. Escape closes every one, and Enter confirms one that asks a single thing. [#196]
- A recipe and its editor are two columns where the window is at least 1000 pixels wide: the Ingredients stay in view while the Method scrolls. [#198]
- The cooking screen is set larger on a tablet or computer and shows the next Step, small, under or beside the one being cooked. [#197]
- Foods keeps its list beside the open Food on a tablet or computer, and the up and down arrow keys move through it. Brought in keeps its list of imports beside the open Report the same way. [#199] [#200]
- Home, Recipes and Cooked fill the window on a tablet or computer, and Shopping is two columns there, with a − and a + on each recipe whose amount can be counted, for how much of it you are making. [#201] [#202]
- A round back arrow sits at the top of every page reached from another page, on both layouts. It never walks back out to another site. [#195] [#219]
- With a mouse, every link, button and list row changes shade under the pointer, and a shelf gets arrows for earlier and later. A touch screen shows none of it. [#203] [#223]
- `/` puts the caret in the recipe search from anywhere but a text field, an open bottom sheet, the cooking screen and the pages read before signing in. [#203]
- A recipe PDF, a Kamosu recipe file or a link dropped onto Recipes is imported, as its row under the + does. [#204]
- A photograph dropped on a recipe's photograph, a Step's or a cooking's sets it, and a Crouton export dropped on its import starts that import. A file dropped anywhere else does nothing, so the browser no longer opens it in place of Kamosu. [#205]
- An older Version opens from the History as a page laid out like the recipe, with every line that has changed since marked on it. You can cook from it, and whoever writes the recipe can carry a line back. Agents get the same comparison from a new operation, `changed_since`. [#211]
- A cooking in Cooked whose lines you changed at the stove lists them under "What you changed" and offers to keep them as a Version. `list_attempts` says the same in a new field, `unkept`. [#210]
- A rating on the recipe page says "on an older Version" or "on another Version" when it was not given on the recipe as it stands, and `get_recipe` says which Version each rating was given on. [#229]
- The Recipes shelf filters to one Cookbook. `search_recipes` and `list_tags` take a `cookbook_id`. [#231]
- Type follows the text size set on the phone or tablet, up to 135% of the size it was drawn at. [#228]
- An image built with `--build-arg VERSION=<version> --build-arg GIT_SHA=<commit>` carries both as its `org.opencontainers.image.version` and `org.opencontainers.image.revision` labels, so `docker inspect` says which release it is. Built with neither, both labels are empty. [#233]

### Changed

- The filters on Recipes are three headed lines of chips: Cookbooks, Kitchens and Tags. A tap holds a chip and a second tap lets it go, so the All chip is gone. "My recipes" is now a tick beside the count, "Cooked or written by me". [#231]
- On a recipe, Cook this stands alone under the servings. The other acts are a grid of small buttons at the foot of the page, and a button is filled once its act has happened: the recipe is on the shopping list, or its Sheet is ready to share. [#220]
- Settings is a menu and four pages: Account, This device, Kitchens and cookbooks, This instance. On the phone it opens from a gear in the header, the header scrolls away with the page, and the tab bar is indigo where it was paper. [#218] [#221]
- A phone turned sideways cooks in two columns, the amounts beside the Step. On every size the switch that keeps the screen awake and "Not really cooking" sit under one ⋯, and the chip saying how much you are making wraps where it was cut short. [#222]
- A Cover is drawn from sixteen pasta shapes where there were eight, so about half the recipes with no photograph change shape once. [#228]
- Words that acted when tapped are now drawn as buttons, and small labels are a size larger. In the editor the tools under each Ingredient Line are small marks beside it. A control reached with the keyboard wears a clear outline where it had the browser's blue ring. [#225]
- Dates are written the same way on every screen, in the interface's language, and every page has a title of its own in the browser tab. A count of one reads "1 recipe", and a tablet is no longer called a phone. [#224]
- The pages read before signing in draw no tab bar and no gear, and the card offering to install Kamosu waits until somebody is signed in. A signed-in person who opens a Cookbook Invite keeps their navigation. [#214] [#226]
- Enter saves a Food's names, a Food's cup weight, a line's Reading and the public address, each of which needed its button before. [#203]
- The source moved to `github.com/BatterWorks/Kamosu`. The old address redirects, and the image is still `battermanz/kamosu`. [#237]

### Fixed

- A line with the amount after the Food is read: "Oignon jaune : 2", "Eau : 800 ml" and "Pâtes orzo 200 g" each used to be stored whole as a Food with no amount. [#207]
- An amount written against its Unit is read, as in "100g Crème". `cs` and `c.s.` are tablespoons, `cc` and `c.c.` teaspoons, `pince` is a pinch, and after an amount a lone `t` is a teaspoon and a lone `T` a tablespoon. "Un peu de", "a little", "a bit of" and "un poco de" opening a line are dropped from the Food's name. [#208]
- French "thé", "dés" and "dé" are words of a Food's name. They were dropped as "the", "des" and "de", so "1 thé vert" named the Food "vert". The Quebec teaspoon, "cuillère à thé", still reads as a teaspoon. [#213]
- An accent typed as its own character stays on a Food's name, so one name typed on two keyboards is one Food. [#230]
- A Step links an ingredient only by the main word of its name. "Add the chicken" no longer shows the chicken broth, and "steamed rice" no longer shows the rice vinegar. A Step saying "le thé" now links the tea. [#212] [#230]
- A cooking kept as a Version shows, with its rating, under that Version in the History. It used to sit under the Version the cooking started from, and the Version that was eaten looked untried. `get_thread` answers `eaten_version_id` on every Attempt. [#209]
- A recipe on the phone's shopping list shows its whole title on one row. The title was squeezed beside "shopping for 4 servings" and wrapped to four lines. [#226]

[#193]: https://github.com/BatterWorks/Kamosu/issues/193
[#194]: https://github.com/BatterWorks/Kamosu/issues/194
[#195]: https://github.com/BatterWorks/Kamosu/issues/195
[#196]: https://github.com/BatterWorks/Kamosu/issues/196
[#197]: https://github.com/BatterWorks/Kamosu/issues/197
[#198]: https://github.com/BatterWorks/Kamosu/issues/198
[#199]: https://github.com/BatterWorks/Kamosu/issues/199
[#200]: https://github.com/BatterWorks/Kamosu/issues/200
[#201]: https://github.com/BatterWorks/Kamosu/issues/201
[#202]: https://github.com/BatterWorks/Kamosu/issues/202
[#203]: https://github.com/BatterWorks/Kamosu/issues/203
[#204]: https://github.com/BatterWorks/Kamosu/issues/204
[#205]: https://github.com/BatterWorks/Kamosu/issues/205
[#207]: https://github.com/BatterWorks/Kamosu/issues/207
[#208]: https://github.com/BatterWorks/Kamosu/issues/208
[#209]: https://github.com/BatterWorks/Kamosu/issues/209
[#210]: https://github.com/BatterWorks/Kamosu/issues/210
[#211]: https://github.com/BatterWorks/Kamosu/issues/211
[#212]: https://github.com/BatterWorks/Kamosu/issues/212
[#213]: https://github.com/BatterWorks/Kamosu/issues/213
[#214]: https://github.com/BatterWorks/Kamosu/issues/214
[#218]: https://github.com/BatterWorks/Kamosu/issues/218
[#219]: https://github.com/BatterWorks/Kamosu/issues/219
[#220]: https://github.com/BatterWorks/Kamosu/issues/220
[#221]: https://github.com/BatterWorks/Kamosu/issues/221
[#222]: https://github.com/BatterWorks/Kamosu/issues/222
[#223]: https://github.com/BatterWorks/Kamosu/issues/223
[#224]: https://github.com/BatterWorks/Kamosu/issues/224
[#225]: https://github.com/BatterWorks/Kamosu/issues/225
[#226]: https://github.com/BatterWorks/Kamosu/issues/226
[#228]: https://github.com/BatterWorks/Kamosu/issues/228
[#229]: https://github.com/BatterWorks/Kamosu/issues/229
[#230]: https://github.com/BatterWorks/Kamosu/issues/230
[#231]: https://github.com/BatterWorks/Kamosu/issues/231
[#233]: https://github.com/BatterWorks/Kamosu/issues/233
[#237]: https://github.com/BatterWorks/Kamosu/pull/237

## v0.1.0 - 2026-09-28

The first release. Kamosu has been cooked from in one household since August
2026. This is the first time anyone else can run it: it is now
public, licensed under the AGPL-3.0, and published as a ready-made image,
`battermanz/kamosu`, for both amd64 and arm64.

Nothing is listed here as added or fixed, since there was no earlier release to
compare with. What 0.1.0 contains:

- **Recipes that keep their history.** Every save is a new Version and none is
  ever lost. Variations and Translations are Branches of the same recipe, and
  the History screen shows where two of them parted and every line that
  differs.
- **Cooking mode.** Step by step, with the amounts each step uses, ticked
  ingredients, timers read out of the step text, and the recipe scaled to what
  you are making. It works offline, and a cooking started on the phone carries
  on on a tablet.
- **Attempts.** A diary of each cooking, with a rating, photos, notes and what
  was actually done, any of which can be promoted into the recipe.
- **Ingredient lines kept as written**, read into amount, unit and food for
  scaling, metric and US conversions, oven temperatures and the shopping list.
- **A shopping list** worked out from the recipes you picked, merging the same
  food across recipes, plus loose items you type.
- **Imports** from a web page's schema.org data, a pasted recipe, a recipe PDF,
  a Crouton export, and another Kamosu's recipe files and Share Links.
- **Sharing.** A Share Link per recipe for anyone who holds it, a printable PDF
  Sheet, and a recipe file carrying the full history.
- **People.** Invites instead of signup, a Cookbook per person that two people
  can join, Kitchens for cooking from each other's Cookbooks, and Operators who
  run the instance without being able to read anyone's recipes through the app.
- **Agents.** Every operation is offered over MCP at `/mcp` exactly as the web
  app gets it, with per-person Access Keys that can be read-only.
- **Search** by words from the start, and by meaning once an Operator turns on
  the optional EmbeddingGemma model, which Kamosu does not ship.
- **Backups** taken automatically, three kept (a day, a week and a month back),
  and a Snapshot before every upgrade migrates the database.
- **English, French and Spanish**, for the interface and for recipes.

The things Kamosu does not defend against are listed in
[SECURITY.md](SECURITY.md).
