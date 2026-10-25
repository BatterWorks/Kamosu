# An imported Attempt names its cook, and was never In Progress

An Import from another app's file may now bring that app's cooking history: each cooking the app recorded lands as a finished **Attempt** of the Person who imported the file, dated as the app dated it. The first is Mealie's backup, whose "I Made This" entries become Attempts (#418). This amends [ADR 0005](./0005-an-attempt-is-a-version-you-cooked-but-did-not-keep.md) on whose an Attempt is, and [ADR 0010](./0010-a-cook-in-progress-is-an-unfinished-attempt.md) on how one begins.

Until now an Attempt was always written by the cook it belongs to, from the moment they started cooking. A household moving from Mealie has years of "I Made This" behind it, by several cooks, some of whom will never have an account here. Dropping that history was the one thing the move would cost them.

## What was decided

- **An imported Attempt belongs to the importing Person, whoever cooked it.** It is in their diary, on the recipe as it arrived, and they may edit or delete it as any Attempt of theirs. No Attempt is ever written under another Kamosu account, even where a Person here has the same name as the cook in the other app.
- **Its text names who cooked it and where**: it begins "Cooked by Sam Cook in Mealie." and then holds what the cook wrote. The importer's own cookings begin that way too, since Kamosu cannot tell which of the other app's cooks the importer was.
- **It was never In Progress.** It is written whole and finished, dated as the app recorded the cooking, and never through starting a cooking. Starting would hand back a cook's real Attempt In Progress on the recipe, and an import must leave that alone.
- **It is pinned to the Version that arrived**: the recipe as it landed, or the Version offered where the recipe was offered rather than written over. The cooking was of the recipe as the other app held it, which is that Version.
- **It carries the app's photo of the cooking, and nothing it did not record.** No rating, since the app's ratings are about the recipe rather than one cooking, and no As Cooked, since the app kept no changed lines.
- **The Import remembers which of the app's records made which Attempt**, so importing the same file again adds none twice. An Attempt its cook deleted stays deleted when the file is imported again. Forgetting the Import forgets this too, as it forgets the ledger.
- **Only a recorded cooking counts.** Mealie also writes an entry on a recipe's timeline when a meal plan names it. That says the recipe was planned, not cooked, and is counted on the Import Report rather than made an Attempt.

## Why

- **The history is the cook's to keep, and the names are not Kamosu's to guess.** Matching "Alex Baker" in Mealie to a Person called Alex here would put a record under an account its owner never wrote, which ADR 0005 exists to prevent. Naming the cook in the text keeps the whole history and says honestly who made it.
- **A record written whole cannot disturb a cooking under way.** ADR 0010 allows one Attempt In Progress per Person per Lineage, and starting one returns the one already there. An import written through that path would finish the cook's real cooking, or fail on it.
- **The Version offered is the one the cooking belongs to.** Pinning to the recipe's head would claim a cooking of a recipe the other app never held.

## Considered options

- **Leave cooking history in the old app**, counted on the Report. Rejected: it is the part of a move a household minds losing most, and the backup holds it whole.
- **Put the history into the recipe's Note.** Rejected: it would make the cookings text no count, date or diary reads, and every later import of the file would offer a changed recipe.
- **Write each cooking under a Person matched by name.** Rejected for the reason above.
- **The importer's finished Attempts, each naming its cook** (chosen).

## Consequences

- **A recipe's count of cookings and its last cooked date include the imported ones**, all under the importer's name. The text says who really cooked each.
- **Another app's reader can bring cookings the same way**: it hands them over with the app's id for each, and the landing does the rest.
