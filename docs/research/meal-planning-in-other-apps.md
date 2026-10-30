# Meal planning in other apps

**Purpose:** how the self-hosted recipe apps Kamosu is compared with plan meals: what a plan entry holds, whose plan it is, how it reaches the shopping list, whether it records cooking, and how it reaches a calendar. Gathered for the design of Kamosu's own meal plan.
**Method:** each app's source on its default branch and its committed docs, read on 2026-10-30 through `gh api`, `gh issue/pr/release view`, GitHub's GraphQL search and raw.githubusercontent.com. Kamosu's notes in `docs/research/how-*-works.md` were the starting point. Nothing was built or run, so where this note says the code does something, that is a reading of the code, not an observed run. A fact is marked **unconfirmed** or **inferred** where it is one.

Versions read: Mealie `mealie-next` (latest release v3.29.0, 2026-10-17; one feature below merged after it), Tandoor `develop` (2.7.2), KitchenOwl `main`, Norish `main` (v0.25.1), Grocy `master` (4.7.1), RecipeSage `master`.

## The headline finding

The most repeated request across four trackers is **"I know what I'll cook this week, not on which day."**

- Tandoor [#1151](https://github.com/TandoorRecipes/recipes/issues/1151) (2021): "we normally ... pick meals for the week, do a shop and then pick and choose what we're feeling that day".
- Norish [#542](https://github.com/norish-recipes/norish/issues/542) "Week view ... with one tile per week" and [#557](https://github.com/norish-recipes/norish/issues/557) "Meal plan Queue": "I need 3 or 4 recipes. But I don't yet know on what day".
- KitchenOwl already models it: a planned recipe with no date sits in a "planned, any day" state (`date.min`).
- Mealie shipped it on 2026-10-27, after its latest release: [PR #8836](https://github.com/mealie-recipes/mealie/pull/8836) "weekly meal planning". An entry gets an optional `end_date`, always `date + 6 days`. The planner is now viewed in whole weeks, each with a "week" band above its days. "Cooking It Today" moves a week entry onto today.

The second theme is that **a plan is an intention, not a record.** Mealie writes "made this" for every planned recipe each night, and [discussion #8040](https://github.com/mealie-recipes/mealie/discussions/8040) asks for a way to turn that off: "families rearrange dinners, eat leftovers, order food ... the automatic task creates inaccurate timeline and `lastMade` history".

## Comparison at a glance

| | Mealie | Tandoor | KitchenOwl | Norish | Grocy | RecipeSage |
|---|---|---|---|---|---|---|
| Slots | 7 fixed types | user-defined, per space | none | 4 fixed | user-defined sections | free text per plan |
| Several per slot | yes | yes | yes, but one recipe once per day | yes, ordered | yes | yes |
| Note-only entry | yes (title, no recipe) | yes (title, no recipe) | no (requested) | yes | yes, plus "product" entries | yes (title, no recipe) |
| Servings per entry | yes, since v3.29.0 | yes | yes (`yields`) | no | yes | no |
| Scope | household | creator + household | household | user rows, read by household | whole instance | named plan + collaborators |
| Shopping list | add day / week / all, scaled | checkbox on create, linked and rescaled | pick ingredients, scaled | panel after planning, or auto | missing items vs stock, per entry/day/week | items linked to entries |
| Random / rules | rules by day + type + filter | auto-plan by keywords, up to 14 days | suggestions from plan history | random by category | none found | none found |
| Views | weeks (was N days) | week or month, 1 to 4 | list by day | timeline (mobile and desktop) | agenda week / day | calendar |
| Calendar feed | iCal, token link | iCal, token scope | none | CalDAV push per user | iCal sharing link | iCal, no auth |
| Records cooking | auto "made this" nightly | no | plan history only | no | "done" flag, consumes stock | no |
| Copy / drag | drag (edit mode) | drag, Ctrl+drag copies | no | drag, duplicate | "Copy this day" | unconfirmed |

## How a planned meal shows in a calendar

Read in each app's feed code on 2026-10-30.

- **Mealie** (`mealie/services/household_services/mealplan_ical_service.py`): an all-day event, unless the feed's link sets a start time for that meal type (`breakfast=`, `dinner=` and so on), which makes it a one-hour event at that time in the link's timezone. A week entry is always an all-day event spanning its seven days. The title reads `Dinner: <recipe>`.
- **Tandoor** (`meal_plans_to_ical` in `cookbook/views/api.py`): always a timed event. An entry is stored as a start and end datetime, taking the meal type's time or noon when none is given, and an entry whose end is not after its start lasts one hour. The title reads `<meal type>: <recipe or title>`, and the entry's note is the event's description.
- **RecipeSage** (`packages/express/src/routes/mealPlans/ical.ts`): timed when the item has a `scheduled` timestamp, otherwise all-day on its `scheduledDate`.
- **Norish**: no feed. Each person's plan is written into their own CalDAV calendar, with a time range set per slot.
- **Grocy**: **unconfirmed**, since whether meal plan entries reach its iCal output at all is inferred.
- **KitchenOwl** and **Recipya**: no calendar.

## Mealie

**1. Data model.** `GroupMealPlan`, table `group_meal_plans` (`mealie/db/models/household/mealplan.py`): `date`, `end_date` (nullable, new in #8836), `entry_type`, `title`, `text`, `recipe_id` (nullable), `recipe_scale` (float, default 1.0, added by [#8701](https://github.com/mealie-recipes/mealie/pull/8701)), `group_id`, `user_id`. The household is read through the user. The schema needs either `recipe_id` or `title` (`mealie/schema/meal_plan/new_meal.py`, `id_or_title`).

**2. Slots.** Fixed enum `PlanEntryType`: breakfast, lunch, dinner, side, snack, drink, dessert (the last three since [#6149](https://github.com/mealie-recipes/mealie/pull/6149), 2025-12). Not user-definable. There is no uniqueness constraint, so several entries can share a slot.

**3. Scope.** The household (`/households/mealplans`). The controller is a plain CRUD controller with no per-entry owner check, so any household member can edit any entry (read in `controller_mealplan.py`, not tested live).

**4. Non-recipe entries.** A "Note" entry, meaning a title with no recipe. The docs say: "great for leftovers, or for ordering out" (`docs/docs/documentation/getting-started/features.md`).

**5. Shopping list.** Per day ("add day to list", or "add week to list" on a week band) and "add all to list". There is no per-entry add from the planner and nothing is automatic. Since v3.29.0 quantities follow each entry's `recipe_scale`, and the recipe opens at that scale from the planner. The scale is changed only in the planner; the recipe page never writes it back ([#8701](https://github.com/mealie-recipes/mealie/pull/8701)). The open request is [#1899](https://github.com/mealie-recipes/mealie/discussions/1899) (41 upvotes).

**6. Random and rules.** `GroupMealPlanRules` holds `day` (weekday or unset), `entry_type` (or unset) and a `query_filter_string` over tags, categories, foods, food labels, rating and time. `POST /households/mealplans/random` combines every matching rule and picks one recipe at random, from all households unless a rule says otherwise. The edit view has a "randomize" button per entry. Week entries match only rules that apply to all days.

**7. Views and feed.** Before #8836 the planner showed "the previous day and the next 6 days by default" (docs). Since #8836 it shows whole weeks, with a preference for how many. The iCal feed arrived in v3.29.0 ([#8461](https://github.com/mealie-recipes/mealie/pull/8461)) through a household token link that needs no sign-in. The query can filter by type, include or leave out notes, and set a timezone. A type with a start time becomes a one-hour event, otherwise an all-day one. The feed carries 90 days of history (`FEED_HISTORY_DAYS`, `mealplan_ical_service.py`). Week entries become multi-day all-day events.

**8. Recording.** A daily task (`create_timeline_events.py`) writes an `info` timeline event "made this for dinner" for each of today's recipe entries, credited to the entry's user, and bumps the household's `last_made`. Week entries are skipped. There is no opt-out ([#8040](https://github.com/mealie-recipes/mealie/discussions/8040)). Meal plan webhooks fire at a set time each day with that day's plan (`post_webhooks.py`).

**9. Recurring, copy, drag.** No recurring entries or copy-week was found. Drag and drop works in the edit view (`planner/edit.vue`, `VueDraggable`), including between a week band and its days.

**10. Mobile.** Not documented. `RecipeCardMobile` is used in the planner.

**11. Requests.** [#850](https://github.com/mealie-recipes/mealie/discussions/850) iCal (50 upvotes, shipped); [#1899](https://github.com/mealie-recipes/mealie/discussions/1899) shopping list from the plan (41, partly shipped); [#3743](https://github.com/mealie-recipes/mealie/discussions/3743) nutrition in the planner (37, open); [#1386](https://github.com/mealie-recipes/mealie/discussions/1386) turn off meal planning (18; services can be disabled since v3.24.0); per-entry servings, [#5241](https://github.com/mealie-recipes/mealie/discussions/5241) and [#2303](https://github.com/mealie-recipes/mealie/discussions/2303) (shipped); [#8040](https://github.com/mealie-recipes/mealie/discussions/8040) automatic "made this" opt-out; [#8059](https://github.com/mealie-recipes/mealie/discussions/8059) main and side dish combinations.

## Tandoor

**1. Data model.** `MealPlan` (`cookbook/models.py`): `recipe` (nullable), `servings` (decimal, default 1), `title` (64 characters), `note` (Markdown text), `meal_type`, `from_date` and `to_date`, `created_by`, `space`. The two dates are **datetimes**, and one entry can span several days. A `shared` many-to-many to users is still in the model, but the serializer no longer exposes it; it was replaced by households in 2.6.0. When no time is given, the entry takes the meal type's `time`, or noon (`MealPlanSerializer._apply_default_time`).

**2. Slots.** User-defined `MealType`: `name` (unique per space), `order`, `color`, `time`, `default`. Shared across the space since 2.6.0. Several entries per type and day.

**3. Scope.** Visible to the creator and the creator's household (`get_queryset`: `created_by` is the user or a household member; permission `CustomIsOwner | CustomIsHousehold`). Open bug [#4900](https://github.com/TandoorRecipes/recipes/issues/4900): household members can't add another member's plan to the shopping list. The 2.6.0 move to households made people's plans disappear until a household was set up ([#4570](https://github.com/TandoorRecipes/recipes/issues/4570)).

**4. Non-recipe entries.** A `title` without a recipe, plus a free `note` on any entry.

**5. Shopping list.** There are three routes:
- An "Add to shopping" checkbox when creating an entry (`addshopping`).
- A shopping tab inside the entry editor.
- A user preference to add automatically (`UserPreference.mealplan_autoadd_shopping`, default off).

A `ShoppingListRecipe` keeps a link to its `mealplan`. Editing the entry's servings rescales the shopping entries (`MealPlanSerializer.update`, `RecipeShoppingEditor.edit`). Deleting the entry cascades to that `ShoppingListRecipe` (`on_delete=CASCADE`); whether its items go too is **inferred, not traced**. Further preferences leave out foods on hand and pull in related recipes (`mealplan_autoexclude_onhand`, `mealplan_autoinclude_related`).

**6. Random.** `AutoPlanViewSet` takes a start and end date (capped at 14 days), one meal type, servings, and keywords with AND or OR. It picks random internal recipes visible to the user, with no repeat until the pool runs out, and can add them to the shopping list. A request to enrich it is open: [#4419](https://github.com/TandoorRecipes/recipes/issues/4419).

**7. Views and feed.** A `vue-simple-calendar` grid, by week or month, showing 1 to 4 periods, with a chosen first day and optional week numbers (`MealPlanDeviceSettings.vue`). These are device settings, not account settings. The iCal feed is at `/api/meal-plan/ical/` with a token limited to the `mealplan` scope ([2.5.0](https://github.com/TandoorRecipes/recipes/releases/tag/2.5.0)). It covers 90 days back and 360 forward by default.

**8. Recording.** None. `CookLog` (recipe, rating, servings, comment, created_by) has no link to `MealPlan`.

**9. Recurring, copy, drag.** Entries drag between dates and keep their span and time. Ctrl+drag copies (`MealPlanView.vue`, `dropCalendarItemOnDate`). There are no recurring entries. Bulk delete is requested in [#4492](https://github.com/TandoorRecipes/recipes/issues/4492).

**10. Mobile.** Same grid on a phone, with one-line items instead of detailed ones (`calendarItemHeight`, `detailed-items="lgAndUp"`). The code has a TODO "add hint about CTRL key while drag/drop", so copying has no touch equivalent (**inferred**).

**11. Requests.** All have few reactions: [#1151](https://github.com/TandoorRecipes/recipes/issues/1151) week-level planning; [#4419](https://github.com/TandoorRecipes/recipes/issues/4419) smarter auto-plan; [#336](https://github.com/TandoorRecipes/recipes/issues/336) and [#4348](https://github.com/TandoorRecipes/recipes/issues/4348) nutrients per day; servings bugs [#4830](https://github.com/TandoorRecipes/recipes/issues/4830) and [#3787](https://github.com/TandoorRecipes/recipes/issues/3787) (closed).

## KitchenOwl

**1. Data model.** `planner` (`backend/app/models/planner.py`) holds `recipe_id`, `cooking_date` (datetime) and `yields` (integer, nullable), with `household_id`. The primary key is (recipe, date), so a recipe can be planned once per day. A planned recipe without a date is stored as `date.min`, and giving it a date replaces that row (`planner_controller.py`).

**2. Slots.** None. Entries are grouped by day only.

**3. Scope.** The household. Anyone in it can plan or unplan.

**4. Non-recipe entries.** None. Requested in [#420](https://github.com/TomBursch/kitchenowl/issues/420) "Allow on-the-fly meals" (6 reactions, open).

**5. Shopping list.** A cart button on the planner opens an item selection page. It lists each planned recipe's ingredients scaled to its `yields` (`Planner.recipeWithYields`), with mandatory items preselected and optional ones offered, then adds the chosen ones to a list (`planner.dart`, `item_selection_page.dart`).

**6. Suggestions.** Nightly, recipes planned between 182 and 7 days ago are drawn at random, weighted by how often they were planned. A refresh button redraws. "Recent" shows recipes recently taken off the plan.

**7. Views.** A list grouped by day, labelled Today, Tomorrow, then weekday names. There is no calendar view; it is requested in [#127](https://github.com/TomBursch/kitchenowl/issues/127), the top planner request (9 reactions). No feed.

**8. Recording.** `recipe_history` logs ADDED and DROPPED. Tapping a planned recipe removes it, which counts as "done" and feeds Recent and the suggestions.

**9. Recurring, copy, drag.** None found.

**10. Mobile.** A native Flutter app. Tap removes a planned recipe and long-press opens it. [#1081](https://github.com/TomBursch/kitchenowl/issues/1081) asks to invert that, and [#959](https://github.com/TomBursch/kitchenowl/issues/959) calls the planner UX confusing.

## Norish

**1. Data model.** `planned_items` (`packages/db-schema/src/schema/planned-items.ts`): `user_id`, `date` (a date), `slot`, `sort_order`, `item_type` (`recipe` or `note`), `recipe_id` (nullable), `title`, plus a version column for concurrent edits. **No servings field.**

**2. Slots.** Fixed enum: Breakfast, Lunch, Dinner, Snack. Several items per slot, in an order that can be changed.

**3. Scope.** Each row belongs to a user. A list reads every user in the caller's household (`ctx.userIds`), and an update or delete passes `assertHouseholdAccess` on the item's owner (`planned-items-helpers.ts`). So the household shares one calendar, and anyone in it can change anyone's item. Changes reach every open screen in the household in real time (`publishCalendarItemEvent`).

**4. Non-recipe entries.** `item_type = note` with a `title`.

**5. Shopping list.** After planning, a per-account preference applies: Nothing, Open groceries panel (the default), or Add ingredients to groceries, leaving out what is in the pantry (`apps/docs/docs/groceries/planned-recipes.md`). How servings are chosen there is **unconfirmed**.

**6. Random.** Since v0.16.0, a random recipe from the slot's category (recipes carry the categories Breakfast, Lunch, Dinner and Snack), weighted "very minimally" toward liked and highly rated recipes ([release](https://github.com/norish-recipes/norish/releases/tag/v0.16.0-beta)). A generator over a date range is requested in [#289](https://github.com/norish-recipes/norish/issues/289).

**7. Views and feed.** Separate desktop and mobile timeline components (`apps/web/components/calendar/desktop|mobile`). The API offers today, week and month lists. Each person can push their plan to a CalDAV calendar, with a time range per slot. There is no ICS feed.

**8. Recording.** None. Planned items older than the start of the month three months back are deleted by a scheduled task (`old-calendar-cleanup.ts`).

**9. Recurring, copy, drag.** Drag on both layouts (`desktop-drag-overlay.tsx`, `timeline-drag-overlay.tsx`; `moveItem` takes a date, slot and index). "Duplicate" exists for planned items. No recurring entries.

**10. Mobile.** A dedicated mobile timeline, plus an Expo app with a calendar tab (`apps/mobile/src/app/(tabs)/calendar`). The v0.16.0 rebuild "permanently deleted" all calendar data.

**11. Requests.** [#542](https://github.com/norish-recipes/norish/issues/542) week view; [#557](https://github.com/norish-recipes/norish/issues/557) queue of undated planned meals; [#289](https://github.com/norish-recipes/norish/issues/289) generator.

## Grocy (brief)

- **Data model.** Table `meal_plan` (migrations `0096`, `0142`, `0149`, `0163`): `day`, `type` (`recipe`, `note` or `product`), `recipe_id`, `recipe_servings`, `note`, `product_id`, `product_amount`, `product_qu_id`, `done`, `section_id`.
- **Sections.** `meal_plan_sections` are user-defined: name, sort number, `time_info`.
- **Scope.** One shared household per instance, with no per-user plan.
- **Stock and shopping.** Triggers build an internal recipe for each day and each week, holding that period's planned recipes. "Put missing products on shopping list" therefore works per entry, per day and per week, and compares against stock. A week header shows the week's costs (`public/viewjs/mealplan.js`).
- **Recording.** "Consume all ingredients" takes them out of stock and marks the entry `done`; there is also a manual done or undone toggle.
- **Copy and views.** "Copy this day" exists. The view is a FullCalendar agenda: a week on a wide screen, a day under 768px.
- **Feed.** Meal plan entries appear in Grocy's calendar (`CalendarService.php`), which has an iCal sharing link (`/calendar/ical/sharing-link`). That the iCal output includes meal plan entries is **inferred** from the shared `GetEvents`.

## RecipeSage (brief)

- **Plans.** A **named** `MealPlan` with an owner and `MealPlanCollaborator` users. A person can have several plans, and `customMealOptions` lets each plan name its own meals.
- **Items.** `MealPlanItem` holds `title` (required), `notes`, `scheduledDate`, an optional `scheduled` timestamp, `meal` (free text), an optional `recipeId` and `userId`. There are no servings (`packages/prisma/src/prisma/schema.prisma`, `createMealPlanItemInput.ts`).
- **Shopping.** Shopping list items link back to the meal plan item that produced them.
- **Feed.** The iCal feed at the meal plan's id needs **no authentication** (`AuthenticationEnforcement.None`, `routes/mealPlans/ical.ts`), so the plan's UUID is the only secret. It carries 30 days of history.
- **Print.** A meal plan print view exists (`routes/print/printMealPlan.ts`).

## Recipya

No meal planner. Open request [#178](https://github.com/reaper47/recipya/issues/178) since 2024-01-03 lists several meals per day, double portions, labels like breakfast, sending meals to the grocery list, and calendar sync later.

## Not confirmed

- Whether Mealie's shopping-list add leaves out foods on hand.
- How Norish picks servings when it adds a planned recipe to groceries.
- Whether deleting a Tandoor plan entry removes its shopping items.
- Whether Grocy's iCal output carries meal plan entries.
- Copy and drag in RecipeSage.
- Mobile behaviour in Mealie and Tandoor beyond what the code shows. Nothing was tested live.
