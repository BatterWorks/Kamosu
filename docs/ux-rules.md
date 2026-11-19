# Kamosu's interface rules

Read this before writing or changing a screen. Each rule is something to do
while the screen is being written, and each names where it comes from: a finding
in [the UX research note](research/ux-reading-crowding-and-finding.md) ("the
note"), a problem in [the UX audit of 19 November 2026](design/2026-11-19-ux-audit.md)
by its code (S3, P2), or both.

**This file is kept current.** The dated records under `docs/design/` say what
was found on one day and are never updated. When a rule here changes, this file
changes, and a rule that gains a test says so under its own line.

**What a test already enforces is not repeated here.** The look under the
pointer, type sizes in `--type-unit` and the one frame every sheet is drawn by
are in `AGENTS.md` ("Design tokens", "The interface"). The 48 px tapped area of
the small-button styles is ADR 0044's, held by `ui/src/lib/tap-area.spec.ts`.
The em dash and length guard on phrases is `ui/check-messages.mjs`. Each fails
`just check` or `just test` on its own.

Each rule ends with how it is checked. **Review** means a reviewer reads the
screen against it (`CODING_STANDARDS.md` points there). **Measured** means
`just ux-measure` or `just ux-walk` counts it, and the reviewer compares the
numbers before and after.

## Words

**U1. Say it in the words a cook uses, not Kamosu's own.** A word Kamosu coined
or gave a meaning of its own (Kitchen, Cookbook, Version, Branch, shelf,
Operator, agent, Job, most of them in `GLOSSARY.md`) goes on a screen only where an everyday word could not do its job, and then the
screen says what it means in one short phrase. Every evaluator, starting with no
glossary, stopped on these words: 25 of the audit's 99 problems break "match
between system and the real world" first (G1, I1, S2, V5). Note: GOV.UK.
*Review.*

**U2. A control's label says what tapping it does.** A verb and its object
("Start cooking", "Copy the link", "Show older versions"), never a bare noun
("Language"), an object nobody can picture ("Print a sheet": a sheet of what?), a
state ("Turn on the link") or a word that fits two actions ("Change", "Cancel
it", "Changed it"). Audit: P4, P11, P17, S1, I8, C5. Note: Cognitive load,
heuristics and evaluation (recognition rather than recall). *Review.*

**U3. Cut explanation before adding it.** No sentence of explanation (six words
or more, ending in a full stop) on the first screenful of Home, Recipes, a
recipe or the share screen, and at most one on any other screen. A subtitle
under a page's title says what the page is for, or the page has none. A screen
holds at most 20 interface words on its first screenful and 80 in all. Audit:
S3 (six sentences on the share screen), H2, R1, and 49 of 55 screens over 20
words. Note: Words; How little people read; Concise, scannable, objective (text
cut to half scored 58% higher). *Measured*, and the subtitle is *Review*.

**U4. What a person must read before acting is never the faintest text on the
screen.** A warning or a consequence sits beside the button it concerns, in body
text at full contrast. Muted colour and the smallest size are for what can be
skipped. Audit: S3 (rated 3.33, the worst problem found), C4, V7, G2, L2. Note:
Banner blindness; Visual hierarchy. *Review.*

## What a screen shows first

**U5. The everyday actions are on the first screenful, the rare ones below or
behind a menu.** Nothing a frequent task needs sits after the content it serves.
Audit: P2 (share, shopping list and History only at the foot of the recipe,
rated 3.00), P1. Note: Length; Apple ("place the most important items near the
top"); Progressive disclosure. *Review*, and Length is *measured*.

**U6. One main button per screen, two at most.** The main button is filled, and
every other action is drawn lighter. At most 16 controls reach into the first
screenful. Note: Controls; Apple ("Keep the number of prominent buttons to one
or two per view"). Audit: R4, I3, P3, V9. *Measured*, and *test*: every screen
test fails when what it drew holds a third `main-button`, an open sheet counted
on its own (`ui/src/testing/main-buttons.ts`).

**U7. Bars and notice cards take at most 15% of the first screenful, and a
screen shows one notice card at a time.** The cooking screen shows none. A
notice never arrives on top of the confirmation of what the person just did.
Audit: Room at 16.9% on 48 of 55 screens, G9. Note: Room; Notices; Banner
blindness. *Measured.* The header and tab bar alone break the 15% today. #655
chooses what gives, from a prototype, weighing that hidden navigation is found
less often and that Apple keeps a tab bar visible (Note: Hidden navigation;
Apple's Human Interface Guidelines).

## Controls

**U8. Every action looks like one.** It has a fill, a border, an underline or
the accent colour, and a row that opens something shows that it does. A dim word
with none of these is not a control. An icon alone carries a word, beside it or
under it, unless it is a back arrow or the screen's close. Audit: V2, C5, H3,
R4 (the unlabelled +), H6 (the gear), C1 (the unlabelled ⋯). Note: Flat
signifiers; Unmarked actions; Hidden navigation. *Review*, and Unmarked actions
is *measured*.

**U9. Use the shared pieces.** A page title is `Screen`, a section heading is
`Section`, a sheet is `SheetFrame`. The screen's main act is `main-button`,
every other act `second-button` (with `loses` for one that deletes, ends or
leaves) or, for a small word beside something, `quiet-button`. A line of a list
that opens something is `ListRow`, and a notice is `NoticeCard`. The title of a
sheet or a card is not a section heading, and an empty list's title is
`Empty`'s. A busy button says so with `aria-busy="true"` and words that say
what is happening. Never draw a heading or a button style of your own, and draw
and word the same job the same way on every screen. 58 filled indigo controls
were drawn 13 ways and 42 headings bypassed `Section` when the audit was charted
(#627). The pieces are drawn on the tokens page (`/tokens`), as chosen on #632.
Audit: S8 (the share link and the invite link worded and drawn apart), V6
(dates written two ways). Note: Visual hierarchy. *Review*, and *test* for
buttons: `ui/src/lib/hand-styled.spec.ts` fails on a button or link given a
fill or a border of its own in a file not on its list, and the list only
shrinks.

**U10. Tapped areas never overlap, and a control drawn under 48 px keeps 8 px
from its neighbour.** Every control's tapped area is 48 px (ADR 0044) whatever
its drawn size. Audit: R3 (filter chips 33 px tall), T3, C6, PL2, and the
overlap on the Kitchens page. Note: Tap size; Gaps; Android and Material Design.
*Measured.* `tap-area.spec.ts` tests the small-button styles alone.

## What could go wrong

**U11. An action that loses something is apart from the everyday ones and
looks different.** Delete, end, leave and cancel never sit directly under or
beside the action they could be taken for, never share its size and fill, and
either ask once or offer to undo. Audit: P18, S6, I4, I8, S3 (a share turned on
in one tap). Note: Cognitive load, heuristics and evaluation (Nielsen's ten
heuristics, which it links: error prevention). *Review.*

**U12. After an action, say what happened and how to take it back.** In the
place the person is looking: what changed, where it went, and an undo where one
is possible. A choice the person made (a filter, a scale, a language) stays
visible while it holds and comes off with one tap. Audit: SH1, PL6, L8, G7, T4,
V8. Note: Cognitive load, heuristics and evaluation (Nielsen's ten heuristics:
visibility of system status, user control and freedom). *Review.*

## Finding the way

**U13. Every step of an everyday task is on a visible control, and nothing is
more than two levels deep.** A menu (+, ⋯) holds only what is rare. Search is
one tap from Home, a recipe one tap from a list, sharing at most two from the
recipe. Audit: the tag and link tasks each put a tap behind a menu, R7 (search
scrolls away), I2. Note: Depth; Hidden navigation; Progressive disclosure.
*Measured* by `just ux-walk`.

**U14. Every screen has a visible way back or out.** A full-screen view says what
it is and how to leave it. Audit: C1 (the cooking screen, rated 2.67), G3. Note:
Cognitive load, heuristics and evaluation (Nielsen's ten heuristics: user
control and freedom). *Review.*

## Every language

**U15. A screen is looked at in all four Languages, and at 360 px wide, before
it ships.** `just screens <folder> <path>… --locale <code>` draws it in each
Language from 390 px up. `just ux-measure` adds the 360 px breakage pass, in
English and French, the longest. Nothing may be cut off, overlap or scroll
sideways in any of them, unless the design cuts it on purpose, as a long recipe
title on Recipes. Audit: the header's name cut off on 51 of 55 screens at 360
px, G11 (French tab labels edge to edge), and French, Spanish and Dutch running
a few words over English. Source: the breakage pass, decision 3 of the audit's
map (#627). *Review*, and the 360 px pass is *measured*.

**U16. The cap on one phrase covers all four Languages, each at its own
length.** `ui/check-messages.mjs` caps an English phrase at 170 characters
(#141) and measures no other language. A French, Dutch or Spanish phrase then
runs as long as it likes, and a paragraph cut in English comes back in
translation. The cap extends to every Language at 200 characters for French,
Dutch and Spanish: on 19 November 2026 a phrase ran 18% longer in French than in
English on average, 14% in Dutch and 11% in Spanish, and 170 × 1.18 is 200. A
single 170 for every Language would leave a faithful translation no room, and so
cut the English below what #141 approved. A key allowed longer in English is
allowed longer in each Language, at a length set per Language. At 200, ten
phrases are over: five in French, two in Dutch, three in Spanish, and seven of
those ten are keys already allowed longer in English. The check goes on in the
fix ticket that cuts or allows the last of them, with wording the person who
approves the copy chooses, never before. *Test*, once it is on.
