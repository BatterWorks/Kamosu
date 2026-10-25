# Paprika exports

Real files Paprika wrote, which the Paprika reader (`src/paprika.rs`, #414) is
tested against. None was edited after Paprika wrote it. A shape of Paprika's
file is supported exactly when a file of it is here or in the corpus check
below.

| File | What it is | Where it comes from |
|---|---|---|
| `mealie-sample.zip` | Five recipes as loose `.paprikarecipe` files in a `My Recipes` folder, zipped on a Mac in December 2021: four in German and one in English, copied from websites, each with its photo inside the file | Mealie's own `tests/data/migrations/paprika.zip`, at commit `cb59fad8f0187b1972ac040bf06aca0a34967c44`. © the Mealie contributors, [AGPL-3.0](https://github.com/mealie-recipes/mealie/blob/mealie-next/LICENSE) |

Mealie's sample holds recipes copied from websites, and it is kept here as
Mealie publishes it.

No real file holds a recipe whose only photo is a web address, a rating, a
favourite mark or nutrition, so the behaviour tests change one recipe of this
sample in memory to reach those paths, and the corpus check reaches the real
ones.

## The corpus check

`tests/paprika_corpus.rs` reads a whole library: vanilla-cookbook's
`src/lib/data/import/paprika.paprikarecipes`, at commit
`e842601e109adf63667bae84a5ff28c96a1b4345` (© the vanilla-cookbook
contributors, [GPL-3.0](https://github.com/jt196/vanilla-cookbook/blob/main/LICENSE)).
It is one person's collection of 746 recipes, 670 with a photo, exported from
Paprika in August 2023, and at 22 MB it stays out of git. Put it at
`samples/paprika/vanilla-cookbook.paprikarecipes` and run
`cargo test --test paprika_corpus -- --ignored`.
