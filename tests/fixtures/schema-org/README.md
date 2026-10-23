# Recipe files in schema.org's format

Real files that apps wrote in schema.org's Recipe format, which the recipe-file
reader (`src/schema_org.rs`) and the Bar Assistant reader
(`src/bar_assistant.rs`) are tested against (#416). Tandoor's two files in this
format are in `tests/fixtures/tandoor/`. A shape is supported exactly when a
file of it is here.

## Borrowed

| File | What it is | Where it comes from |
|---|---|---|
| `nextcloud-mealie-sample.zip` | Three Nextcloud Cookbook folders of January 2021, each a `recipe.json` copied from a website, two with `full.jpg` and `thumb.jpg` beside it | Mealie's own `tests/data/migrations/nextcloud.zip`, at commit `cb59fad8f0187b1972ac040bf06aca0a34967c44`. © the Mealie contributors, [AGPL-3.0](https://github.com/mealie-recipes/mealie/blob/mealie-next/LICENSE) |
| `nextcloud-flutter-recipes.zip` | Nineteen Nextcloud Cookbook folders of December 2021, in English and German, copied from websites, one an app's test recipe with an empty step | nextcloud-cookbook-flutter's `docker/data/Recipes/`, at commit `0e290f4a93184e9090672e6af495851148264938`, [CC0 1.0](https://github.com/Teifun2/nextcloud-cookbook-flutter/blob/develop/LICENSE) |

The flutter app keeps its folders unzipped. They were zipped for Kamosu as
Nextcloud's own folder download does, under `Recipes/`, and their pictures
were left out to keep the repository small: Mealie's sample has the pictures.
The folder names are as that repository spells them, some with their
umlauts garbled. Mealie's sample holds recipes copied from websites, and is
kept here as Mealie publishes it.

## Made for Kamosu

Made on 23 October 2026, each app run once from its current release: in a
throwaway container on this machine, or from its release binary. Each was
handed the same few recipes, and the files are what each app's own export
wrote. The photographs are from the Sample Library
(`sample-library/CREDITS.md`): the Orange cake by Andy Li (CC0 1.0), and the
Vietnamese coffee by Simon Law (CC BY-SA 2.0).

| File | App | How it was made |
|---|---|---|
| `recipesage-4.0.13.json` | RecipeSage 4.0.13, the self-host release (`julianpoy/recipesage-selfhost:api-v4.0.13`) | Two recipes entered through its API, the calls its screens make: an *Orange cake* with a photo, two labels, a source, a link, notes, a rating, nutrition and its lines and steps split by `[Cake]` and `[Syrup]` headings, and *Pâtes à la tomate*, in French, with none of those. Exported with Settings › Export Recipe Data › Export to JSON-LD. It is `{"recipes": […]}`, and a self-hosted RecipeSage writes the photo into the file as a `data:` address |
| `recipya-1.2.2.zip` | Recipya 1.2.2 (`reaper99/recipya:v1.2.2`) | The same two recipes through its *Add manually* form, the cake with a photo, a category, two keywords, a tool, a source link and nutrition. Exported with Settings › Data › Export data, as JSON. It holds no picture: each recipe names its photo by an address on the Recipya server, and Recipya left the tool out |
| `cookcli-0.38.1.zip` | CookCLI 0.38.1, its `x86_64-unknown-linux-musl` release binary | Three Cooklang recipes, *Orange cake* with sections, a photo's address, tags, a source and an author, *Pâtes à la tomate* in French, and *Lemonade*, each written out with `cook recipe <file> -f schema -o <name>.json` and the three files zipped together. CookCLI writes no photo into a file, only the address the recipe names |
| `bar-assistant-6.9.1-json-ld.zip` | Bar Assistant 6.9.1 (`barassistant/server:6.9.1`) | A bar with its starter ingredients, and two cocktails made through its API: *Vietnamese iced coffee* with a photo and its credit, a source, a ranged optional amount, a garnish and a glass, and a *Negroni*. Exported with `php artisan bar:export-recipes 1 --type=json-ld`, the export its Settings › Exports screen starts, as *Schema.org Recipe (JSON-LD)* |
| `bar-assistant-6.9.1-schema.zip` | Bar Assistant 6.9.1 | The same bar, exported with `--type=schema`: Bar Assistant's own cocktail format, *JSON (Schema draft 2)* on its screen and its default |

Bar Assistant's other export types, Datapack, Markdown, XML and YAML, are not
read.
