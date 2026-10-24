# A Step's lines may be corrected by hand

Which Ingredient Lines a Step uses is still worked out of the Step's words on every read, as [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md) and [ADR 0019](./0019-an-ingredient-line-has-no-name-of-its-own.md) decided. A person may now correct it for one Step, with the Operation `set_step_uses` ([#439](https://github.com/BatterWorks/Kamosu/issues/439)). The correction is the whole list of lines the Step uses. It stands in place of what was read, and `get_recipe` marks the Step `by_hand`. This amends both ADRs, which said nobody types a link.

## What was decided

- **A correction belongs to one Step of one Version**, kept beside the Version as a Reading is, and like a corrected Reading it mints no Version and appears in no history ([ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md)).
- **A save carries it onto the new Version while the Step's words and every line it names stay word for word as they were**, a photograph put on the Step changing nothing, wherever they now sit, paired as a save pairs the lines whose Readings it carries. Change the Step's words or one of those lines and the correction is dropped, and the Step is read again. What a Step uses may have changed with its words, and a kept link to a line that is now something else would be wrong with no sign of it.
- **The lines are named by their place in the Version**, which is all a correction needs to be about a Version. Nothing is added to a line or a Step, so the fingerprint of a recipe, its Bundle and its Vault copy are untouched, and two people holding the same Version still hold the same Version.
- **Anyone who can see the recipe may correct a Step**, as with a Reading. A Job Key may not, though it may correct a Reading (`JobKey::Never`, where `set_reading` is `CorrectsItsReadings`), and a Demo refuses it.
- **A correction does not travel.** A Bundle carries no correction, as it does not say which of its Readings were corrected either, so a received recipe's Steps are read. A Translation's Steps are other words, so the correction is not carried there either.
- **An empty list is a correction too**: the Step adds nothing. A correction taken away (`uses: null`) leaves the Step to the reading.

## Why

- **The reading cannot be right every time, and a wrong link on the cooking screen puts the wrong amount beside a Step.** The fourteen recipes imported on 2026-10-23 showed it plainly. The reading was made better on the same ticket, but a Step such as *add the ground meat* beside *ground lamb* is beyond any rule the reader can be given.
- **ADR 0019's reasons were against ids on lines, and this adds none.** A line's place in one Version never travels and never enters the fingerprint, and a correction ends when its words change rather than claiming a line it no longer knows.
- **It is the same bargain a corrected Reading already strikes** ([ADR 0002](./0002-the-written-ingredient-line-is-the-truth.md), [ADR 0021](./0021-a-reading-is-kamosus-reading-not-the-recipe.md)): what was written stays the truth, what Kamosu read from it can be corrected, and the correction lasts as long as the words it was about.

## Considered options

- **No correction at all**, leaving the reading to be improved. Rejected: some Steps no rule reads, and until now the only way out was rewording the recipe to suit the reader.
- **A link stored in the recipe's content**, on each Step. Rejected for ADR 0019's reasons: it would move every Version id, travel in every Bundle and outlive the words it was about.
- **A correction beside the Version, kept while the words are** (chosen).

## Consequences

- **Agents can correct a Step today**, at the MCP Door. A control on the edit screen is a ticket of its own, with a prototype to choose from.
- **A correction is lost on any change to its Step or its lines**, a fixed typo included. That is the accepted cost of never keeping a link the words no longer support.
