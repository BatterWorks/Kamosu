# The Dutch test set

`lines.json` holds real Dutch Ingredient Lines and Step sentences, each with what
Kamosu should read of it: a line's amount, Unit and Food, and a sentence's lines,
timer and oven. Each recipe names the page it came from. `tests/dutch_lines.rs`
reads every one of them, in `just test`.

The pages come from Dutch and Flemish recipe sites, chosen for their range. A line
is one page's whole list where it is a list of food. A method is cut to a few
sentences that each hold something to read, at most six a page, so this is a test
and no copy of anybody's recipe. Lists of equipment, brand names, and lines a site
printed broken are left out.

A misread line reported by a cook goes in here, under the recipe it came from, or
under a new one with its page, with what Kamosu should have read. Then fix the
word in `src/words/nl.toml` or the grammar in `src/dutch.rs`.
