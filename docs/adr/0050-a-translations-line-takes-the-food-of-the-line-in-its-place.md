# A Translation's line takes the Food of the line in its place

When Kamosu reads a Translation's Ingredient Lines, a line takes the Food of the source line in the same place, if the two lists have the same shape and the two lines read the same amount and the same Unit. Only the Food is taken. Settled on 2026-10-18 on [#356](https://github.com/BatterWorks/Kamosu/issues/356). This is the one place Kamosu matches lines by their place, which [ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md) rejected for Pairing.

It started with a French Translation of a beef curry. Its English recipe read *beef*, *ginger*, *Red chilli* and *black pepper*, because somebody had corrected all four Readings by hand. The French lines were read on their own words and made four Foods of their own: *bœuf à braiser*, *gingembre frais râpé*, *piments rouges frais* and *poivre noir moulu*. The correction never crossed the Language, and every line the reader keeps a describing word on did the same.

## What was decided

- **A line is matched with the source line in its place.** Both lists must have the same shape: the same number of lines, with each section heading and each Ingredient Line in the same place. If they don't, no line is matched.
- **The two lines must read the same amount and the same Unit.** `2 tbsp` and `2 c. à soupe` agree. `2 cups` and `250 g` do not. A line with no amount has nothing to agree on, and a unit outside the closed set cannot be known to be another, so both decline.
- **Only the Food is taken.** The Reading's amount, unit and target stay the line's own words, and no Food learns a name from them. This holds even where the words already answer to another Food.
- **It happens when a Translation is made, and when a save brings it up to a newer source.** That is `start_translation`, and a save that names a new `translates_version_id`. An ordinary edit of a Translation reads the lines it wrote on their own words.
- **A line that declines is read by the reader, as before.** A source line read as a Component, or read as nothing, passes nothing on.

## Why

- **Pairing cannot cross a Language.** It goes by the words, read against a Branch Point ([ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md)), and a Translation shares no words and no Branch Point with its source. The place is the only thing left, and a translator keeps it far more often than not.
- **ADR 0019's reasons against place do not apply here.** Place breaks the moment a line is inserted, which is most edits. A Translation is not an edit. It is the whole list written again, and the same-shape test turns away the case where a line was added or taken out. The amount and unit test turns away most lines that were moved.
- **Taking only the Food keeps a wrong match to one line.** If the Food learnt the Translation's words as a name, a wrong match would be taught to the whole library. Every French recipe saying *sucre* would then read as *salt*, shopping lists included. A wrong Food on one line shows on that line and is put right with `set_reading`.
- **A hand edit is the Translation's own words changing.** A cook who swaps salt for sugar at the same amount keeps the line's place and its measure, and taking the source line's Food would read the sugar as salt. A save that renders a newer source is a fresh translation of it, and an agent catching a Translation up is the one that sends it.

## Considered options

- **Have the reader drop describing words beside a food** (`fresh`, `grated`, `à braiser`, `moulu`), in all three Languages and for every recipe. Rejected for now: it reverses the rule from [#163](https://github.com/BatterWorks/Kamosu/issues/163) that a describing word is dropped only when it is all that is left, and it needs a list of exceptions that are foods of their own: *ground beef*, *crème fraîche*, *dried apricots*, ground and whole pepper.
- **Match by place alone, without the amount and unit test.** Rejected: a reordered list would give lines the wrong Food without a sign.
- **Have the Food learn the Translation's words as a name**, as a Food learns an arriving name ([ADR 0022](./0022-doubt-makes-a-new-food-never-a-merge.md)). Rejected for the spreading above.
- **Match on every save of a Translation.** Rejected for the swapped ingredient above.

## Consequences

- **The Reading's target may not answer to its Food.** The French line reads *gingembre frais râpé* and points at *ginger*, which has no such name. The Correcting panel's link searches the Foods for the word and may find nothing, or another Food. A Bundle carries the Food by its own names, so it travels as it should.
- **Reading the library again leaves a taken Food alone** while the reader reads the line as before. If the reader changes how it reads that line, `reread_ingredient_lines` resolves the new word as it would anywhere and the taken Food is lost. The Operator-wide reads of unread lines do not match lines with a source either.
- **A hand correction on the source crosses over** whenever the Translation is made or caught up, since the source line's Reading is what names the Food.
