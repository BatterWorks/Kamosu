# The cooking screen shows tonight's amounts

_Amends [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md), [ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md) and [ADR 0060](./0060-a-scaled-line-reads-whole.md). Built for [#681](https://github.com/BatterWorks/Kamosu/issues/681), 26 November 2026._

The maintainer cooked a tomato soup written for four at six servings on a phone. On its second Step eight Ingredient Lines, most of them two lines tall, filled the screen, and the Step's own words were cut off after two lines: "It's unusable as is." The cooking screen was reworked over five rounds, each scored by two graders working through it as a cook at the stove would. Three of the choices it ended with go against what earlier ADRs recorded. The maintainer chose to keep them, and this records them.

## What was decided

- **At another Yield, the cooking screen shows the scaled line alone**, at full size: `1½ heads of garlic`, where it showed `1 head of garlic` with the scaled line beneath. The recipe's own line is one tap away, in *Note a change*, which holds the recipe's lines, and on the recipe page. A line Kamosu could not scale is shown as written and says so ([ADR 0060](./0060-a-scaled-line-reads-whole.md)). At the Yield as written nothing changes: the written line at full size, its conversion beneath. This is the cooking screen's alone; the recipe page, the printed Sheet and the MCP Door are as ADR 0060 has them.
- **At another Yield, a Step's figures are tonight's, drawn where the written ones stood**: at twice the recipe, for a cook reading in US measures, "melt about 2 oz butter", where it read "melt 30 g butter (about 2 oz)", the 30 g being for half the pot. Nothing is stored changed. The Step's words are the Step's, and the screen draws the Core's figure in place of the one it scaled, as it drew it after the figure before. At the Yield as written a conversion still follows what it converts ([ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md), "Amounts in a Step").
- **A Step that uses no Ingredient Line shows the Step alone.** *Nothing new to add, just the pot* is no longer said.

## Why

- **The scaled line now says what it is an amount of by itself.** ADR 0060 made it the whole line at the new amount, so the written line beneath it added only the food again with the figure not to use. Two lines per Ingredient doubled the panel, and on the soup's busy Step that was the difference between the Step on screen and the Step cut off.
- **The figure drawn largest is the one a cook measures.** At twice the recipe the written figure stood at the Step's size, with tonight's small and grey beside it: "the one place the screen can make a cook measure wrong", in a grader's words. Both graders of the untouched screen asked for tonight's figure in the written one's place, and the next round's asked again. ADR 0016 calls a list that says one amount and a Step that says another "a contradiction". With the list showing tonight's amount alone, a Step showing the recipe's large was that contradiction.
- **The words repeated on most Steps.** A Step names an Ingredient Line only 42% of the time across the real corpus, so *nothing new to add* stood over most Steps of most recipes. The graders of the untouched screen read it as a hint repeated on every such Step, and it said *just the pot* over a pizza dough with no pot. A Step with nothing above it already says it adds nothing.

## Considered options

- **The written line small beneath the scaled one**, as before. Rejected: it took the room #681 was about, for a figure the cook must not use.
- **The written figure in a scaled Step, tonight's after it**, as before. Rejected for the reason above.
- **A mark on a Step's figures that were scaled.** Not built. A figure Kamosu did not scale, a count such as *2 eggs* or a Unit outside the closed set, stays as written and looks no different from tonight's, so at twice the recipe a cook reads it as already doubled. A mark on some figures and not others is a mark a cook learns to look for and then trusts where it is missing. The scaled Ingredient Lines say *kept as written* where Kamosu could not scale; a Step does not yet.

## Consequences

- **[ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md)'s rule that the Ingredient Line is always shown at full size with its Reading subordinate holds at the Yield as written.** At another Yield, on the cooking screen, the scaled line stands in its place.
- **[ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md)'s "the Step's text is still never rewritten" holds for what is stored.** What is drawn at another Yield is the Step's words with tonight's figures in them.
