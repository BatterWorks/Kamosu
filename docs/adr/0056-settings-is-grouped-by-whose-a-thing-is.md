# Settings is grouped by whose a thing is

Settings is a menu of four groups by whose a thing is, *You*, *Your household*, *Add recipes* and *This instance*, with one short page for each job, and *About Kamosu* and *Sign out* at the foot. The Operator's screen is no longer a second place: its two rooms are pages of the *This instance* group, which only an Operator is shown. On the wide layout the menu stays in a column beside the page that is open. Settled on 2026-10-26 in a design session and on its prototype (#469). This amends [ADR 0044](./0044-wide-screens-are-the-same-app-laid-out-by-room-and-designed-by-role.md), which made Settings and the Operator screen one column at reading width, and [ADR 0034](./0034-kamosu-names-what-it-does-not-defend.md), whose list Settings no longer carries.

People got lost in Settings, cooks and Operators both. Its seven pages, made one ticket at a time, mixed what was the reader's own, what was the household's and what was the instance's. *This instance* showed to everybody and held a cook's imports beside an Operator's Meaning Search and the only link to the Operator screen. The Security Record was half the reader's and half the instance's on one page. Measures followed the account but sat on *This device*. Account could show about 950 words, and Import opened with a paragraph of 46.

## What was decided

- **Four groups by whose a thing is.**
  - *You*: your name as a card at the top of the menu, then Signing in (password, passkeys, devices signed in), Keys for agents, Language and measures, This device and Your security record.
  - *Your household*: Kitchens, Your Cookbook, Tags and Foods.
  - *Add recipes*: Import from another app, Save from your phone and What has been brought in.
  - *This instance*: People, Backups, Runner and House Notes, Public address, Meaning Search, Upkeep and the Instance security record. Only an Operator is shown it, as every Operator surface is left out for anybody else ([ADR 0040](./0040-a-refusal-answers-as-if-the-thing-were-not-here.md)).
- **One short page per job.** A page whose name is exactly what someone looks for may hold one control. Kitchens and Your Cookbook are separate pages, so the invite that shares your recipes and the one that lets someone change them are never side by side.
- **Each menu row is a name and a short line under it** saying what is inside (option C of the prototype, over the name alone and the name with its current value).
- **One short line at most under a heading, and a warning where the act is.** A consequence such as "this cannot be undone" or "other devices are signed out" is said in the confirm dialog that opens when the button is pressed, and nowhere before. Making an Operator key and changing a public address that passkeys depend on each ask first now, with the sentence that used to stand under the choice.
- **On the wide layout the menu stays beside the page**, as Foods keeps its list beside a Food ([#199](https://github.com/BatterWorks/Kamosu/issues/199)). `/settings` opens Signing in there. A page of Settings carries no back arrow on the wide layout, since the menu is beside it. On the phone each page is a screen of its own, as before, and the back arrow returns to the menu.
- **Foods and What has been brought in keep their own screens** at `/foods` and `/imports`, each a list with what is open beside it. Their back arrow leads to Settings.
- **"What Kamosu does not protect" leaves Settings.** No app shows its users such a list. It stays in `SECURITY.md`, which the README links, and `tests/defences.rs` checks that one place.
- **Save from your phone** is the new name of *Share from your phone*: "share" read as sending a recipe out, the way a Share Link does.
- **Every old address still arrives.** `/settings/account` opens Signing in, `/settings/instance` About Kamosu, `/operator` People and `/operator#runner-keys` Runner and House Notes.

## Why

- **Whose a thing is, because that was the confusion.** A cook could not tell which part of Settings was theirs, and an Operator found instance things on two screens. Grouping by task ("Run Kamosu", "Cook with others") was considered and would have mixed the reader's own and the instance's again in each row.
- **Short pages over a short menu.** The menu grows from 7 rows to about 13 for a cook and 21 for an Operator. Group headings make a long menu easy to scan, while a long page has to be read to learn whether the thing wanted is on it.
- **The menu beside the page on the wide layout.** With 21 rows, an Operator moving from Backups to People would otherwise go back to the menu every time. It is how a tablet's and a computer's own settings work.
- **Warnings at the act.** Every confirm dialog already carried the consequence that matters when someone acts, so most paragraphs said it twice, once too early. Text nobody reads still costs three languages to keep true.

## Considered options

- **Grouped by task.** Rejected above.
- **The seven pages kept, with the misplaced parts moved and the words cut.** Rejected: *This instance* would stay a drawer of unrelated things, and the Operator screen a second place behind a link.
- **The same one column on every layout.** Rejected above.
- **A "More" link opening each page's longer explanation, or a help page.** Rejected: the long text would still be written, translated and kept true, for a reader who rarely opens it.

## Consequences

- **A Kitchen Invite is a link now**, in the same change: it opens a page showing the Kitchen as its members see it, and an Operator's Invite can name one of the Operator's own Kitchens for its newcomer. The "Join a Kitchen" field and its code are gone.
- **The published iPhone Shortcut's install question still names "Share from your phone"** until it is built again from `docs/sharing-from-an-iphone.md`, which names the page as it is now.
