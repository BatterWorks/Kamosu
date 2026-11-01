# A Calendar Link is a key a calendar app reads the plan with

A cook can see their Meal Plan in their phone's own calendar, beside their appointments. In Settings they make a **Calendar Link**, a private address they give to Google Calendar or Apple Calendar, which then fetches the plan from Kamosu on its own every so often and shows each Planned Meal as an all-day event, linking back to its recipe. Built for [#491](https://github.com/BatterWorks/Kamosu/issues/491), a later part of the Meal Plan ([ADR 0058](./0058-a-meal-plan-is-a-persons-own-days-ahead.md)). How the calendar apps read a feed is in `docs/research/meal-planning-in-other-apps.md`, "How a calendar app reads a feed". This amends [ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md), whose list of Secrets gains one, and sits beside [ADR 0055](./0055-a-phone-shares-into-kamosu-through-one-operation-and-an-add-only-key.md)'s Add-only key.

A calendar app sends no password, no cookie and no Access Key. It fetches one address, again and again, for as long as the cook keeps the subscription, and understands one format, iCalendar. So the address itself has to be the permission.

## What was decided

- **A Calendar Link is a key of its own kind.** Its Secret is 256 bits, kept by its hash alone and shown once, when it is made (ADR 0031). It is good for one thing: reading its Person's plan as a calendar file at `/calendar/<secret>.ics`. It acts as nobody, changes nothing and reaches no recipe.
- **One per Person.** Making one ends the one before, so "the calendar link" is always one thing to give or to end, and a cook who lost track of it makes a new one.
- **Made, read and ended by Operations at both Doors.** `make_calendar_link` answers the Secret once; `get_calendar_link` says whether one is live, when it was made and when a calendar app last read it; `end_calendar_link` ends it. Making one is for a Person signed in, as minting an Access Key is, since an Access Key mints no key (ADR 0031). Making and ending each write a line in the Security Record (`calendar_link_made`, `calendar_link_ended`).
- **The file is served outside the Catalogue**, as the Share Link page is ([ADR 0026](./0026-a-recipe-is-seen-by-its-kitchen-or-by-anyone-with-the-link.md)). A live Secret is answered the plan from 90 days back on; an ended one, one never made, and one whose Person can no longer sign in are all answered nothing, a 404. Whether a Secret is live is answered by the Core. The offline worker leaves the address to the server, so a browser with Kamosu installed gets the file and never the app.
- **Each Planned Meal is an all-day event**, titled with its recipe's title or its written words, named by the meal's own id so a move moves the event, and marked as taking no time, so a dinner never makes its cook look busy. A recipe's event links to it at the instance's public address where one is set. The file is written by Kamosu's own code, with no new dependency.
- **A Demo refuses making one** ([ADR 0052](./0052-a-demo-refuses-what-a-stranger-could-misuse.md)): a Demo's visitors share one account, and a link one of them made would hand the next their week.

## Why

- **A key, not a Session or an Access Key.** Either of those acts as the Person, across every Operation they may call, and a calendar link sits in plain view in someone's calendar settings, synced to Google's or Apple's servers, for years. What leaks with a Calendar Link is a list of dinners. ADR 0055 made the same trade for the Add-only key, for the same reason: a key that can do one small thing is safe to leave lying in a phone.
- **Hashed, unlike a Share Link.** ADR 0031 keeps a Share Link readable because the owner sends the same recipe to several people, and ending the link to send it again breaks every copy. A Calendar Link has one reader, the cook's own calendar, given it once. Showing it again would need it kept readable in the database for nothing; a cook who needs it again makes a new one.
- **A route an agent cannot reach is no break in parity.** [ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md) promises that what a Person can do, an agent can do, by building both Doors from one Catalogue. Making, reading and ending the link are Operations at both Doors. An agent holding an Access Key reads and ends one; making one takes a Session, as minting an Access Key does, because an Access Key mints no key (ADR 0031), the one rule parity already gives way to. The file is not something a Person does: it is a format for a calendar app, as the Share Link page is a page for a stranger's browser, and an agent wanting the plan reads it with `get_meal_plan`, which says more than the file does.
- **Ninety days back**, as Mealie and Tandoor carry ([research](../research/meal-planning-in-other-apps.md)): last week's dinners stay on the phone, and the file stays small however long the plan grows.

## Considered options

- **The plan's address with no Secret**, as RecipeSage serves its feed with no sign-in at the plan's id. Rejected: the id would then be the Secret without being treated as one, with no way to end it.
- **An Access Key in the address.** Rejected above: it would act as the Person.
- **Several links per Person**, one per device or calendar. Rejected for now: one calendar subscription reaches every device on the same Google or Apple account, and a list of links is one more thing to understand.
- **Keeping the Secret readable**, so Settings can show the link again. Rejected above.
- **Answering an ended link with an empty calendar**, which would clear the meals from a subscribed calendar. Rejected: it would answer differently for a Secret that was once real, and a calendar app keeps showing the last file it read either way until the cook removes the subscription.

## Consequences

- **A change can take hours, or a day, to show.** Google and Apple each decide how often they fetch, and neither says. The file asks for an hour (`REFRESH-INTERVAL`), which neither promises to obey. Settings says so in plain words.
- **Google Calendar needs Kamosu on the internet.** Google fetches the file from its own servers, so an instance reached only at home or over a VPN can be subscribed in Apple Calendar on the phone but not in Google Calendar.
- **A recovery link leaves it alone**: it reads a plan and nothing more, and the cook ends it in Settings.
- **A Person's plan in a Demo has no link**, so its daily cleaning has nothing of one to put back.
