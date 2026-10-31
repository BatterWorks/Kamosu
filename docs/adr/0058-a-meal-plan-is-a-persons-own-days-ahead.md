# A Meal Plan is a Person's own days ahead

Kamosu gets a **Meal Plan**: each Person's own calendar of what they will cook, in a place of its own between Recipes and Shopping. A **Planned Meal** is a recipe put on a day, one particular Branch at the Yield the cook means. Settled on 2026-10-30 in a design session ([#485](https://github.com/BatterWorks/Kamosu/issues/485)), with the research in `docs/research/meal-planning-in-other-apps.md`, and its first screens chosen on 2026-10-31 from a prototype ([#486](https://github.com/BatterWorks/Kamosu/issues/486)). This amends [ADR 0052](./0052-a-demo-refuses-what-a-stranger-could-misuse.md): in a Demo, planning saves beside cooking and shopping. It adds a fifth place to the navigation of [ADR 0011](./0011-cooking-shows-the-amounts-for-the-step-it-is-on.md) and [ADR 0044](./0044-wide-screens-are-the-same-app-laid-out-by-room-and-designed-by-role.md).

A cook could keep every recipe in Kamosu, cook from it and shop for it, but Kamosu had no answer to "what are we eating this week?". The Shopping List holds the recipes about to be bought for, with no day on any of them, and it is emptied after the shop. Attempts say what was cooked, after the fact. Every self-hosted recipe app Kamosu is compared with has a meal planner, and the README said Kamosu is not one.

## What was decided

- **The plan is a Person's own (a candidate).** One Meal Plan per Person, unnamed and always there, as the Shopping List is ([ADR 0024](./0024-a-shopping-list-is-a-choice-of-recipes-not-a-list-of-things.md)). Nobody in their Kitchens reads it, and an Operator's `as_operator` ([ADR 0049](./0049-an-operator-looks-after-every-cookbook.md)) does not reach it. This is a **candidate**, not final: a plan shared by a Cookbook's Co-authors is the open alternative, and stays open until the choice is named final.
- **A Planned Meal is one Branch on one day.** Always that Branch's newest Version and never a pinned one, for ADR 0024's reason: a plan describes what is about to be cooked, so a correction made on Monday is what Thursday shows. Choosing between Branches is how a cook picks a method. A day holds as many as the cook likes, in the order they were put there, with no breakfast, lunch or dinner.
- **Its Yield follows the Shopping List's rule**, multiplier included ([#109](https://github.com/BatterWorks/Kamosu/issues/109)). None stored means the recipe as written, so the plan follows a correction to the recipe's own Yield. A recipe whose Yield has no amount Kamosu can count from is planned by a multiplier.
- **A recipe that goes away stays on the plan.** Deleted, its sharing withdrawn, or in a Cookbook that left a Kitchen: it keeps the name it was known by and says it can no longer be read, as a Shopping List entry does.
- **It follows its recipe where the Shopping List does.** When two Cookbooks join and one Branch folds into another, and when a Co-author leaves with a Copy, a Planned Meal moves to the Branch kept in the same step that moves the Shopping List's choosing.
- **Its writes carry `written_at` and the last device wins**, as the Shopping List's do: a phone with no network holds them in its outbox and sends them later, and one older than the plan's last write changes nothing ([ADR 0013](./0013-offline-you-may-write-your-own-history-never-the-recipes.md)). Reading the plan asks the server first and works offline from what the phone last read. The design session had it phone-first like most reads; live acceptance showed a plan made on the phone missing from the iPad's Plan when it opened, which a plan read on two devices cannot have.
- **Nothing about it is part of a recipe.** It is in a Backup and in no Bundle, and goes with its Person.
- **The Plan is the fifth place**, between Recipes and Shopping, in the tab bar and the sidebar. A week runs Monday to Sunday.
- **What it looks like** (#486, 31 October 2026): a calendar week. On the phone each day is a row, its date in a block on the left and its recipes beside it; on the wide layout seven columns side by side, each recipe a card with its photograph. A recipe's page has *Plan this* beside *Cook this*, half the width each, which opens a sheet listing today to the Sunday of the week after next, a heading at each week, each day naming what it already holds, and the recipe page's own "how much" picker below.

## Why

- **A Person's, as Attempts and the Shopping List are.** All three say what one cook does, and a plan read by nobody else tells nobody anything. A Cookbook's plan was preferred at first in the design session and set aside for now. Reaching it would give the Cookbook a plan, read and written by every Co-author, and move each Person's Planned Meals into it.
- **A Branch at its newest Version, because the Shopping List already learnt that lesson.** The two hold the same kind of choice, and agreeing means a cook never has to ask which of them is out of date.
- **The plan keeps what it held.** The Shopping List is emptied after the shop; a plan is not, so last week's dinners have an answer.
- **A fifth tab.** ADR 0011 kept the tab bar to what is opened during a week of cooking, and the days ahead are exactly that, as the shopping is. Settings stays behind the gear.

## Considered options

- **A plan shared by a Cookbook's Co-authors.** The open alternative above.
- **A plan shared by a Kitchen.** Rejected: a Kitchen is who sees whose recipes, and a member who sees your recipes need not see your week.
- **The Shopping List with days on it.** Rejected: the list is emptied after the shop, and a plan must outlast it.
- **Meal slots (breakfast, lunch, dinner).** Mealie, Tandoor and Norish have them, KitchenOwl does not. Left out: a day's order says which comes first without naming meals, and leaves nothing to translate.

## Consequences

- **A Demo lets planning through**, and its daily cleaning puts the plan back with the Sample Library.
- **Later parts are their own tickets**: a week planned without a day, typed Planned Meals ("leftovers"), moving and planning again ([#487](https://github.com/BatterWorks/Kamosu/issues/487)), sending planned days to the Shopping List ([#488](https://github.com/BatterWorks/Kamosu/issues/488)), "cooked" ([#489](https://github.com/BatterWorks/Kamosu/issues/489)), a planned week in the Sample Library ([#490](https://github.com/BatterWorks/Kamosu/issues/490)), a Calendar Link ([#491](https://github.com/BatterWorks/Kamosu/issues/491)) and Mealie's plan ([#492](https://github.com/BatterWorks/Kamosu/issues/492)).
