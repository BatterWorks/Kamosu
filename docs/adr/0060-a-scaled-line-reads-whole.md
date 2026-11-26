# A scaled line reads whole

_Amends [ADR 0016](./0016-kamosu-converts-to-the-kitchen-not-to-the-arithmetic.md). Built for [#680](https://github.com/BatterWorks/Kamosu/issues/680), 26 November 2026._

The maintainer cooked a tomato soup written for four at six servings. Since [#109](https://github.com/BatterWorks/Kamosu/issues/109) the cooking screen leads with the scaled amount, and the scaled amount was the short line ADR 0016 wrote to sit beside the written one: *about 1½*, with the onion only in the small grey line beneath. The cherry tomatoes, *500-700 g*, were not scaled at all, the olive oil said *kept as written* although it had no amount, and every figure said *about*, 1 × 1.5 included. The maintainer's verdict: "It's unusable as is."

## What was decided

- **At another Yield, the line beneath an Ingredient Line is the whole line at the new amount.** The written line with its amount scaled, and its Unit too where the reader's measures converted it: `1 head of garlic` at half again is `1½ heads of garlic`, `½ cup coconut milk` for a metric reader is `about 180 ml coconut milk`. In English the counted word follows the new number: the Unit's own word where the line spells it out (`head`, `tablespoon`, never `tbsp`), and otherwise the Food's last word (`onion`). Other Languages keep their words as written. A measure the line only restates after a Unit of the closed set, `1 cup (240 ml) milk` or `400 g / 0.9 lb`, is left out, since it would now give a second, unscaled amount; the size of an open Unit, `1 can (14 oz)`, stays. At the Yield as written, a conversion is still the short figure beside the line, and a Step's conversions stay short in every case, since they sit inside the Step's own sentence.
- **A range scales and converts at both ends.** `500-700 g` at half again is `750-1050 g`, both ends in the Unit the larger one is worded in. It is still never added up: a Shopping Row carries the Food with no number, and a Component line standing for a range gets no factor, exactly as ADR 0016 has it.
- **"About" is said where Kamosu rounded or converted, and nowhere else.** `1½ cups` from `1 cup` is exact and says none; `about 1¼ tsp` from `1 tsp` at 1.3 was rounded. An oven temperature is converted and keeps it, and a Shopping Row's sum keeps it as before.
- **"Kept as written" is for a line with an amount Kamosu could not scale.** A line with no amount, *olive oil* or *salt and pepper*, had nothing to scale, and says nothing.

## Why

- **The scaled line is now read on its own.** ADR 0016 wrote the subordinate line as an addition to a line still on screen above it. The cooking screen made it the main line, so it has to say what it is an amount of. One line from the Core, worded for every Door, is still the rule: the screen, the printed Sheet and an agent at the MCP Door all read the same words.
- **A range is a real amount.** ADR 0016 declined range arithmetic because nobody had asked for it and the fault in #167 was the Food. Scaling one beneath the line is arithmetic on each end, the same rounding as any amount, and the cook asked for it. Adding ranges up on a shopping list still has no rule anyone has asked for, so it stays out.
- **ADR 0016's argument for an unconditional "about" was that Kamosu always rounded.** That held for conversions, which it was written about. Plain scaling often lands on an exact figure, and *about 1½* for exactly one and a half is the false mark ADR 0016 wanted to avoid, turned the other way. Tying "about" to whether rounding or conversion happened keeps its absence a promise that is true.

## Considered options

- **A second field beside `measured`, holding the whole line.** Rejected: every reader of a scaled recipe wants the whole line, and two answers for one line are two things for a screen to choose between.
- **Pluralising in every Language.** Rejected for now: French, Spanish and Dutch plurals need more than a suffix, and a wrong plural is worse than the word as written.
