# UX research: reading, crowding and finding

**Purpose:** the published research Kamosu's UX audit judges every screen against, with the conditions each finding was measured under, a look at how five other recipe apps lay out finding, opening and sharing a recipe on a phone, and the ten measures the audit counts, each with its threshold. Gathered for the audit charted in #627 ("the map" below): its measuring tool `just ux-measure`, its walkthrough, and the rules ticket that follows them.

**Method:** every source in section 1 was read at its own page on 2026-11-17, through DonSeTch. Pages that are built by scripts were read another way, and the finding says which. Apple's guidelines came from the JSON Apple serves the same pages from (`developer.apple.com/tutorials/data/design/human-interface-guidelines/<page>.json`), Material Design's through Tavily. Section 2 rests on Mealie's source at `04ba6b7` (`mealie-next`, 2026-11-17) and Tandoor's at `d9282c0` (`develop`, 2026-11-16), both shallow clones, on the iPhone screenshots in each native app's App Store listing (fetched from Apple's lookup API the same day), on Paprika 4's user guide, and on Mela's help page. Mealie's public demo (version "nightly") was opened in Chrome at 402 × 874 CSS pixels, three dots to the pixel, as a phone with touch, and counted by a scratch script that applies the rules of section 3 roughly. Nothing was written to the demo. A fact is marked **inferred** or **unchecked** where it is one.

**Words used here.** A *study* measured people doing something. A *guideline* is advice, sometimes drawn from studies, that measured nothing itself. An *estimate* fits a formula to data gathered for another purpose. A *CSS pixel* is the unit a page is laid out in. On an iPhone one point (Apple's unit) is one CSS pixel, and in Chrome on Android one dp (Material's unit) is one CSS pixel.

## 1. The findings

Each finding gives the claim, where it comes from, the conditions it was measured under, what kind of evidence it is, and what it means for Kamosu. **Not a phone app** marks a finding whose conditions differ from Kamosu's, used with care. The measures named here (Words, Room, Controls and the rest) are defined in section 3.

### How little people read

- **Claim.** On an average web page people have time to read at most 28% of the words, and about 20% is more likely. Each extra 100 words adds about 4.4 seconds of time on the page, enough to read 18% of them.
- **Source.** Nielsen Norman Group (NN/g), [How Little Do Users Read?](https://www.nngroup.com/articles/how-little-do-users-read/), Jakob Nielsen, 2008-05-05, sections "Percentage of Text Read" and "The Research Study".
- **Conditions.** Browser logs of 25 people, gathered in 2005 by Weinreich, Obendorf, Herder and Mayer (*ACM Transactions on the Web*, 2008): 59,573 page views on desktop browsers. Pages averaged 593 words, and the formula was fitted between 30 and 1,250 words. The article notes the people were more educated than average, several of them university staff.
- **Kind.** An estimate, from time spent on a page and an assumed 250 words a minute. Nobody's reading was observed.
- **Not a phone app.** Desktop web pages, read in 2005.
- **For Kamosu.** It supports counting words at all: a sentence added to a screen is mostly not read. It gives no number for a phone screen, so the Words threshold comes from the survey.

### Concise, scannable, objective

- **Claim.** The same site scored 58% higher in measured usability when its text was cut to about half, 47% higher when written to be scanned (lists, bold keywords, more headings), 27% higher without promotional language, and 124% higher with all three.
- **Source.** NN/g, [Concise, SCANNABLE, and Objective](https://www.nngroup.com/articles/concise-scannable-and-objective-how-to-write-for-the-web/), John Morkes and Jakob Nielsen, 1997-01-01, "Study 3: Measurement Study". Measured usability combines task time, task errors, memory, time to draw the site map, and satisfaction.
- **Conditions.** 51 web users, each given one of five versions of "Travel Nebraska", a seven-page travel site built for the study. The concise version kept the information the tasks needed and cut the rest. The same article cites an earlier rewrite of pages from Sun's site that scored 159% higher.
- **Kind.** A study, between groups.
- **Not a phone app.** Desktop web, 1997, an information site rather than an app.
- **For Kamosu.** Halving the words is the single largest effect any source here measured. It is why Words counts interface words separately from the recipe, so a cut shows.

### Flat signifiers

- **Claim.** Looking for the control to click took 22% longer, with 25% more eye fixations, when actions looked like plain text than when they looked clickable. Plain-looking actions cost little where the page held little, was laid out conventionally, and put its important actions where they stood out. Coloured links inside text were recognised without an underline.
- **Source.** NN/g, [Flat UI Elements Attract Less Attention and Cause Uncertainty](https://www.nngroup.com/articles/flat-ui-less-attention-cause-uncertainty/), Kate Moran, 2017-09-03, sections "Methodology", "Number of Fixations and Time on Page", "Page Pairs with Nearly Identical User Gaze Patterns" and "When Flat Designs Can Work".
- **Conditions.** Eyetracking on a desktop computer, 71 general web users, 9 sites each in a strong and a weak version, one task each. Timing stopped when the person said "I found it". The article warns that a whole task slows by less than 22%, since finding the control is a small part of it.
- **Kind.** A study.
- **Not a phone app.** Desktop only.
- **For Kamosu.** Kamosu's quiet actions are indigo on paper, so the coloured-link finding partly covers them. Unmarked actions counts the rest, and Controls tests whether a screen is sparse enough for the study's exemption to apply.

### Hidden navigation

- **Claim.** With navigation hidden behind a menu, people found content significantly less often on both desktop and phone, a drop of more than 20% in the article's overall figure. They were at least 39% slower on desktop and 15% slower on a phone, and rated tasks harder. On a phone they used hidden navigation in 57% of tasks and partly visible navigation in 86%.
- **Source.** NN/g, [Hamburger Menus and Hidden Navigation Hurt UX Metrics](https://www.nngroup.com/articles/hamburger-menus/), Kara Pernice and Raluca Budiu, 2016-06-26, sections "Why Study Hidden and Visible Navigation", "Measures" and "Hidden Navigation Leads to Worse User Experience".
- **Conditions.** 179 people doing tasks on 6 websites, remotely, on desktops and smartphones. On phones only hidden and "combo" (partly visible) navigation were tested. Fully visible navigation was not.
- **Kind.** A study.
- **For Kamosu.** Kamosu's five places sit in a visible, labelled tab bar, which the study supports. Applying it to anything else behind a menu (the + menu, a ⋯ menu) is an extension: the study tested a site's main navigation, not a menu of actions.

### Banner blindness

- **Claim.** People skip what sits where ads usually go, and what looks like an ad: a small rectangle set into the text, fancy formatting, a coloured background, text inside a picture. On phones they also scroll past large, striking pictures they take for ads.
- **Source.** NN/g, [Banner Blindness Revisited](https://www.nngroup.com/articles/banner-blindness-old-and-new-findings/), Kara Pernice, 2018-04-22, sections "1. Location", "2. Visual Treatment" and "Faux Ads".
- **Conditions.** Eyetracking on desktop and phones, 26 people on each of the tasks described. First documented by NN/g in 1997 and repeated in 2007.
- **Kind.** A study, reported as observations and gaze plots rather than rates.
- **Not a phone app.** Phones were in the study, but what it observed is people avoiding ads on websites. Kamosu has no ads.
- **For Kamosu.** A notice card is a boxed rectangle with a coloured edge, set between the header and the page, which is the shape the study says people learn to skip. That is the case for counting Notices and keeping them few, and an extension of the study.

### Reading on phones

- **Claim.** People understood articles as well on a phone as on a computer. Hard articles, long and in harder language, took longer to read on a phone for the same understanding.
- **Source.** NN/g, [Reading Content on Mobile Devices](https://www.nngroup.com/articles/mobile-content/), Kate Moran, 2016-12-11, sections "Methodology and Analysis" and "Speed-Accuracy Tradeoff on Mobile".
- **Conditions.** 276 people, each reading easy and hard articles on a phone or a computer, then answering questions. Difficulty was set by length and by the Flesch-Kincaid reading level.
- **Kind.** A study.
- **For Kamosu.** It matches Kamosu's device. A long explanation costs more time on a phone than on a laptop, which is why Words counts sentences of explanation.

### Touch targets

- **Claim.** A touch target needs to be at least 1 cm × 1 cm to be hit quickly and accurately, and targets need space between them. A target must first be big enough, then spaced, and spacing does not rescue a target that is too small.
- **Source.** NN/g, [Touch Targets on Touchscreens](https://www.nngroup.com/articles/touch-target-size/), Aurora Harley, 2019-05-05, sections "Size Matters" and "Mind the Gap". The 1 cm comes from a study by Parhi, Karlson and Bederson. The article also cites MIT's Touch Lab: fingertips 1.6 to 2 cm wide, a thumb's contact about 2.5 cm.
- **Conditions.** Touchscreens in general. The article does not give the conditions of the studies it rests on.
- **Kind.** A guideline resting on studies.
- **For Kamosu.** 1 cm is about 60 CSS px on the iPhone 17 (460 dots per inch, three dots per CSS pixel, Apple's specification) and about 63 on the Pixel 10 (1080 × 2424 at 422 dots per inch, laid out 412 wide, Google's specification). Recorded, not adopted: Kamosu's rule is 48 (ADR 0044), 7.6 to 8 mm on these two phones.

### WCAG 2.2 target size

- **Claim.** Level AA (2.5.8, Target Size Minimum): a target is at least 24 × 24 CSS px, or spaced so that a 24 px circle centred on it touches no other target and no other small target's circle. Level AAA (2.5.5, Target Size Enhanced): at least 44 × 44. Both exempt a link inside a sentence, a target that another full-size control on the page duplicates, and a control the browser draws unchanged.
- **Source.** W3C, [Understanding SC 2.5.8](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html), sections "Exceptions" and "Spacing", and [Understanding SC 2.5.5](https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html), section "Success Criterion".
- **Conditions.** All web content on any device, with any pointer.
- **Kind.** A standard.
- **For Kamosu.** Kamosu's 48 is above both, so neither is adopted. The inline exception carries over: a link in a sentence is counted apart under Tap size.

### Apple's Human Interface Guidelines

- **Claim.** A button needs a hit region of at least 44 × 44 pt. Keep prominent buttons to one or two per view, and tell the preferred choice by style, not size. Allow about 12 pt of padding around a control with a visible edge and about 24 pt around one without. Put the most important items near the top. A tab bar is for navigation, not actions: it stays visible, its tabs carry labels of one word where possible, and its buttons are never hidden or disabled. Show one sheet at a time.
- **Source.** Apple, [Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons) ("Best practices", "Style"), [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility) ("Mobility": iOS default control size 44 × 44 pt, minimum 28 × 28), [Layout](https://developer.apple.com/design/human-interface-guidelines/layout) ("Visual hierarchy"), [Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars) ("Best practices"), [Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets) ("Best practices"). The pages render with scripts and carry no date. They were read from Apple's JSON of the same pages.
- **Conditions.** Apps on Apple's platforms, the iPhone among them. Read as it stood on 2026-11-17.
- **Kind.** A platform guideline. It cites no studies.
- **For Kamosu.** "One or two prominent buttons per view" is the threshold for main buttons under Controls. The padding figures are recorded under Gaps.

### Android and Material Design

- **Claim.** Touch targets of at least 48 × 48 dp, which Google gives as about 9 mm, separated by 8 dp or more. Pointer targets 44 dp. A navigation bar holds three to five destinations of equal importance. Keep essential interactions, primary navigation among them, in a reachable part of the screen, space like elements alike, and do not "overwhelm your user with too many actions per view".
- **Source.** Google, [Touch target size](https://support.google.com/accessibility/android/answer/7101858) (Android Accessibility Help, "Design"); Material Design 3, [Accessibility designing: structure](https://m3.material.io/foundations/designing/structure) ("Touch and pointer target sizes", "Target spacing", read through Tavily since the page renders with scripts); Android Developers, [Navigation bar](https://developer.android.com/develop/ui/compose/components/navigation-bar) and [Layout basics](https://developer.android.com/design/ui/mobile/guides/layout-and-content/layout-basics) ("Takeaways").
- **Conditions.** Android apps, phones first. Read as it stood on 2026-11-17.
- **Kind.** A platform guideline. It gives no number for "too many actions".
- **For Kamosu.** 48 matches ADR 0044. The 8 dp spacing is the Gaps threshold. Kamosu's five tabs sit at the top of the three-to-five range.

### Progressive disclosure

- **Claim.** Show up front everything people need often, and few enough options to keep attention on what matters. Designs with more than two levels of disclosure "typically have low usability because users often get lost when moving between the levels". The way to the second level must be obvious and labelled.
- **Source.** NN/g, [Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/), Jakob Nielsen, 2006-12-03, "Usability Criteria for Progressive Disclosure".
- **Conditions.** Written in 2006 for applications and websites in general.
- **Kind.** A guideline from experience. The article names no study.
- **Not a phone app.** Its examples are desktop applications and a hotel booking site.
- **For Kamosu.** The two-level limit is the Depth threshold.

### Visual hierarchy

- **Claim.** Use no more than three sizes (small, medium, large), and no more than three contrast variations "for complex designs". In uncomplicated designs, two primary and two secondary colours.
- **Source.** NN/g, [Visual Hierarchy in UX: Definition](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/), Kelley Gordon, 2021-01-17, sections "1. Color and contrast" and "2. Scale".
- **Conditions.** Written in 2021 for web pages.
- **Kind.** A guideline.
- **Not a phone app.** Its example sizes (14 to 16 px body, 18 to 22 subheads, up to 32 for headers) are for web pages.
- **For Kamosu.** The figure behind Type and colour, which flags and never fails, since the cooking screen differs on purpose (the map's decision 4).

### Cognitive load, heuristics and evaluation

- [Minimize Cognitive Load](https://www.nngroup.com/articles/minimize-cognitive-load/) (Kathryn Whitenton, 2013-12-22, "Minimizing Cognitive Load"): avoid visual clutter, build on the layouts people know from elsewhere, and take reading, remembering and deciding off the person. A guideline for websites.
- [10 Usability Heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/) (Jakob Nielsen, 1994-04-24, reviewed 2024-01-30): number 8, aesthetic and minimalist design, "Every extra unit of information in an interface competes with the relevant units of information". Also number 6, recognition rather than recall, and number 1, visibility of system status. A guideline for any interface. The list the walkthrough's evaluators judge against.
- [Severity Ratings](https://www.nngroup.com/articles/how-to-rate-the-severity-of-usability-problems/) (Jakob Nielsen, 1994-11-01): a scale from 0 (not a problem) to 4 (catastrophe), weighing frequency, impact and persistence. A method, from experience. One evaluator's ratings are "too unreliable to be trusted", and the mean of three "is satisfactory for many practical purposes". The report's scale.
- [How to Conduct a Heuristic Evaluation](https://www.nngroup.com/articles/how-to-conduct-a-heuristic-evaluation/) (Kate Moran and Kelley Gordon, 2023-06-25, "Choose and Train Your Team"): three to five people evaluate the same interface independently. A method. The audit uses three (the map's decision 6).
- [Why You Only Need to Test with 5 Users](https://www.nngroup.com/articles/why-you-only-need-to-test-with-5-users/) (Jakob Nielsen, 2000-03-18): one person finds about 31% of the problems, so five find about 85%, a figure averaged across NN/g's projects. Test again after fixing, and test more people when groups of users differ sharply. Behind the later session with people (the map's decision 8).

### GOV.UK

- **Claim.** Check any sentence over 25 words for a split. Keep paragraphs to five sentences. People skim in an F shape. Plain English is mandatory, and specialists prefer it too: in research on legal documents, 80% preferred sentences in clear English, more so as the issue grew complex. Say "select", not "click".
- **Source.** GOV.UK, [Use clear language](https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/writing-guidelines/clear-language/) ("Keep paragraphs and sentences short", "Write clearly for specialists too"), [Create a clear structure](https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/writing-guidelines/clear-structure/), and the [A to Z style guide](https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/style-guides/a-to-z-style-guide/) ("Sentence length", "click", "plain English"). The style guide and "Writing for GOV.UK" have moved from `gov.uk/guidance` to these addresses.
- **Kind.** A guideline. The 80% figure is the one number it gives, and it does not cite its source.
- **Conditions.** Government information pages for everyone in the UK, on any device.
- **For Kamosu.** The 25-word check is used under Words, as a flag.

### Kamosu's own rule

Tapped areas are 48 px (ADR 0044), and a control drawn smaller grows an invisible 48 px area round itself, with two side by side keeping a gap at least as wide as their two overhangs. Kamosu's rule outranks every general one here. NN/g's 1 cm and WCAG's 44 px are recorded above and not adopted.

## 2. Recipe apps on a phone

How the five apps lay out the three starting tasks. Kamosu's notes on how each app works are in [how-mealie-works.md](how-mealie-works.md) and [how-tandoor-works.md](how-tandoor-works.md), and sending a recipe *into* an app from a phone is in [sharing-a-recipe-in-from-a-phone.md](sharing-a-recipe-in-from-a-phone.md). Sharing here means sending one of your recipes to someone else. [how-kitchenowl-works.md](how-kitchenowl-works.md) was read too: KitchenOwl is one Flutter app on every platform, outside the five, and that note says nothing of its phone layout.

| | Paprika 4 | Mela | Crouton | Mealie | Tandoor |
|---|---|---|---|---|---|
| Kind | native | native | native | web, PWA | web, PWA |
| Evidence | guide, 4 screenshots | screenshots, help page | 4 screenshots | source, demo measured | source only |
| Places | behind ☰ (a sidebar), 7 sections | tab bar, 5 labelled | tab bar, 4 labelled | behind ☰ (a drawer) | bottom bar, 4 icons, no labels, the 4th a ☰ sheet |
| Finding | search field at the bottom of the recipe list | **unchecked** | **unchecked** | search field at the top of Home, 4 filter buttons and sort under it | search in the top bar, or Search in the ☰ sheet |
| Recipe page, top | back, ♥, add to grocery list, start cooking, ⋯, edit (guide) | back, ⋯ | back, timers, ⋯ | 4 round buttons: ♥, timeline, edit, ⋮ | ⋮ beside the name |
| Recipe page, main action | Ingredients / Directions switch | Cook, Groceries, Adjust in red text | Start, a pale green pill | none stands out (Cook Mode is orange text) | none found in source |
| Share | ⋯ → Share → Source Link, Paprika File or Recipe Text → share sheet | **unchecked**; in ⋯ (**inferred**) | paper-plane button on the recipe page (**inferred** from its icon) | ⋮ → Share (8th of 9 entries) → link dialog, or the share sheet when the recipe is public | ⋮ → Share (6th of 7 entries) |

### Paprika 4

Paprika's [user guide](https://www.paprikaapp.com/help/v4/ios/all/) (read as one page) says the sidebar on iPhone "is hidden until you need it: tap the menu button in the upper left corner". The recipe list fills the screen. Its search field "sits at the bottom of the screen, within easy reach of your thumb". On a recipe, the toolbar holds the heart, add to grocery list, start cooking and edit buttons. Less common actions sit under a menu button beside them (an icon in the guide, ⋯ in the screenshot), and Share is one of them: "A submenu with Source Link …, Paprika File and Recipe Text. Each opens the share sheet". The recipe screenshot shows one filled control, the selected half of the Ingredients / Directions switch.

### Mela

The App Store screenshots (version 2.6.2, 2025-06-08) show a tab bar of five labelled tabs (Recipes, Feeds, Browser, Calendar, Groceries). The recipe page has a round back and ⋯ button over a full-width photograph, then the title, times, and three actions in the accent red with an icon and no fill or border: Cook, Groceries, Adjust. Mela's [help page](https://mela.recipes/help/) covers sharing a whole library with other iCloud users and adding recipes from other apps. It says nothing about sending one recipe to someone.

### Crouton

The App Store screenshots (version 2026.5.1, though the pictures show an older iPhone) show a tab bar of four labelled tabs (Plan, Recipes, Groceries, Settings). The recipe page has "‹ All Recipes", a timers button and ⋯ at the top, a photograph with the title on it, then a row of three: Start (a pale green pill with a play icon), a round grocery button and a round paper-plane button. The paper plane is read here as send or share (**inferred**). The App Store text says "Share your recipes with friends and family using iCloud". Crouton's site renders with scripts and gave nothing.

### Mealie

On a phone (`DefaultLayout.vue`) the sidebar opens only on large screens. Below that, a ☰ button in the compact orange header opens it. The header also holds the logo, the name, a search button and a log-out button (`AppHeader.vue`). Home (`RecipeExplorerPage.vue`) opens with a search field, four filter buttons (Categories, Tags, Tools, Foods), sort and settings, then recipe cards. On a recipe (`RecipeActionMenu.vue`) four round buttons sit in a row. The ⋮ menu opened on the demo lists Download, Download PDF, Duplicate, Add to Plan, Add to List, Print, Print Preferences, Share and Delete. Share opens the device's share sheet when the recipe, its group and household are all public, and otherwise a dialog that makes an expiring link (`RecipeContextMenuContent.vue`).

### Tandoor

On a phone (`Tandoor.vue`) a top bar holds the logo, a search dialog, a + menu (create, import) and the account menu. Below the large-screen breakpoint, a bottom bar holds four icon buttons with no labels: the start page, meal plan, shopping, and ☰, which opens a sheet with Settings, Import, Database, Search, Pantry, Books and more (`useNavigation.ts`). The recipe view puts a ⋮ beside the name. Its menu holds Edit, Add to Plan, Add to Shopping, Property Editor, Pantry, Share and Duplicate (`RecipeContextMenu.vue`). Tandoor's demo link now leads to a sign-in page, so nothing was measured.

### Counted on Mealie's demo

Counted by the scratch script at 402 × 874, on the demo's Home and on the first recipe with a photograph, which on the last run was "Klassische Rinderroulade". The demo's recipes change as visitors add them, so a later count of "the first recipe" may land on another. The script follows section 3's rules roughly. It did not set aside links in sentences, controls holding a picture or rows of a list, and it probed tapped areas on the first screenful only. The tool will refine all of that, so these numbers anchor thresholds rather than settle them.

| Measure | Home | Recipe |
|---|---|---|
| Words: interface on the first screenful / whole page | 5 / 24 | 19 / 82 |
| Words: sentences of explanation | 0 | 0 |
| Room | 5% | 11% |
| Controls on the first screenful / main buttons | 16 / 0 | 14 / 4 (the round buttons, 40 × 40) |
| Unmarked actions | 4 | 5 |
| Tap size: under 48 on the first screenful | 7 of 16 (filter buttons 28 tall) | 5 of 14 (the round buttons) |
| Gaps: smallest between controls drawn under 48 / tapped areas overlapping | 10 px / 0 | 4 px / 0 |
| Type and colour: sizes / text colours | 7 / 5 | 7 / 5 |
| Length: screenfuls / first main button at | 2.7 / none | 6.8 / 0.92 |
| Notices | 0 | 0 |

### Counted by eye on the screenshots

Marketing screenshots, possibly older than the app, and only the recipe page is shown in all three. Counted by eye, so each count could be one or two out.

| On the recipe page's first screenful | Paprika 4 | Mela | Crouton |
|---|---|---|---|
| Controls, not counting ingredient lines | about 11 | about 5 | about 9 |
| Filled controls | 1 | 0 | 0 to 1 (Start's fill is pale) |
| Sentences of explanation | 0 | 0 | 0 |
| Notices | 0 | 0 | 0 |
| Share | 2 levels down | **unchecked** | on the screen |

What the survey shows: none of the screens seen (Tandoor's were not) carries a sentence of explanation or a notice. The native apps draw at most one filled control on the recipe page. Two of the five hide all their places behind ☰, Tandoor all but three, and the three whose share is known from their own guide or source (Paprika, Mealie, Tandoor) put it behind ⋯ or ⋮. Crouton's paper plane, if it is share, is the exception, and Kamosu's Share tile is another.

## 3. The ten measures

Every measure is counted on one screen, loaded alone, after its answers have arrived and nothing has moved for two seconds, with no sheet open. The **first screenful** is the window scrolled to the top at that moment: 402 × 874 and 412 × 924 CSS pixels, the map's two comfort sizes (its decision 3). A **control** is a visible element that is a link with an address, a button, a field, a select, a summary, or an element with role button, link, tab, checkbox, switch, menuitem or option. Where controls nest, only the outermost counts. Text hidden from sight (`display: none`, invisible, or a box of one pixel or less, as `.sr-only` is) counts nowhere. A **bar** is the page's header (a `header` element outside `main`, or an element with role `banner`) and any element positioned `fixed` or `sticky` that is at least half the window wide and less than nine tenths of its height. Kamosu's header scrolls with the page and its tab bar is fixed, and both are bars. The **accent colour** is `--color-accent` in `ui/src/app.css` (#1d2b4c today), and `--color-cook-accent` on the cooking screen.

Each measure has a threshold and the finding behind it. **Judgement** marks a threshold chosen within the range the evidence allows, which the rules ticket may move.

### 1. Words

- **Counts.** Interface words on the first screenful and on the whole page. Sentences of explanation on the first screenful and on the whole page. Interface sentences over 25 words on the whole page.
- **Interface or content.** The tool keeps every string and number in each JSON answer the page receives while it loads (in Kamosu, the answers to `POST /api/op/<name>`). A text node is *content* when its whitespace-normalised text equals one of those values exactly, or when it holds three words or more and appears inside one with no letter or digit touching either end. A single word or two therefore counts as content only on an exact match, so a tab's "Plan" is not swallowed by a recipe that mentions "Plantain". Everything else is *interface*, a field's placeholder included. Words the server writes itself, a refusal or a server's notice, arrive in an answer and so land on the content side: Words does not count them, and the walkthrough reads them instead. Kamosu's own notice cards (`ui/src/lib/offline/Notices.svelte`) draw the interface's phrases and count as interface.
- **A word** is a run of characters between spaces that holds a letter or a digit.
- **A sentence** is interface text split after each full stop, question mark, exclamation mark or ellipsis, within the nearest enclosing element that is not inline. **A sentence of explanation** is one that ends in one of those marks and holds six words or more.
- **Threshold.** No sentence of explanation on the first screenful of Home, Recipes, a recipe or the share screen, and at most one anywhere else on a screen (judgement: the survey found none, and NN/g's reading estimate says most are not read). A sentence over 25 words is flagged for a split (GOV.UK, a guideline). At most 20 interface words on the first screenful and 80 on the whole page (judgement: Mealie's highest counts were 19 and 82). The 1997 study's halving is the effect to aim for when a screen is over.

### 2. Room

- **Counts.** The share of the first screenful's height covered by bars and notice cards (defined under Notices). Bands that overlap are counted once.
- **Threshold.** At most 15% (judgement: Mealie's bars take 5% to 11%, and the October 2026 audit found Kamosu's header and tab bar at 19%). Hidden navigation is found less (NN/g, 2016) and Apple says a tab bar stays visible, which the rules ticket weighs when it decides what gives.

### 3. Controls

- **Counts.** Controls whose box reaches into the first screenful, and how many of them are main buttons. A **main button** is a control drawn at least 40 px tall whose background colour has an opacity above one half and a contrast of at least 3:1 against the colour behind it, which in Kamosu is a filled indigo button or tile. A control showing a held state (`aria-pressed`, `aria-selected` or `aria-checked` true, or any `aria-current`) is not a main button, and nor is a chip under 40 px, such as Recipes' filter groups filled while a filter is held. A tile drawn filled to say it is done (`act-tile-done`) counts, since it weighs as much as one.
- **Threshold.** At most 16 controls (judgement: Mealie's highest count, and the 2017 study's "low information density" condition). At most two main buttons (Apple: "Keep the number of prominent buttons to one or two per view"). The native apps drew none or one on the recipe page, and Mealie four.

### 4. Unmarked actions

- **Counts.** Controls on the first screenful that are not a field, and have no fill (a background with opacity above 0.1 that differs from the colour behind it), no border (any side wider than zero in a colour that differs from the colour behind it), no underline on the control or inside it, and no text or icon in the accent colour. Three kinds are not counted, because a convention marks them (NN/g's "build on existing mental models"): a control inside a bar, a control holding an image (`img`, `picture` or a background image, not an icon drawn in `svg`), and a row of a list of three or more alike controls stacked one above the other. These three are this note's addition to the definition the map gives.
- **Threshold.** None (judgement). Flat actions cost little only on sparse, conventional pages (NN/g, 2017), and with this definition indigo text is already marked, as the study's coloured links were.

### 5. Tap size

- **Counts.** Every control on the whole page whose tapped area is under 48 px across or down. The tool scrolls each control until its centre sits halfway between the bars at the top and bottom of the window, then asks the browser which element lies under a point (`elementFromPoint`), from the centre outwards one pixel at a time, left, right, up and down, while the answer is the control or something inside it. A control inside a bar is probed with the page at the top. That measures the area a finger hits, including the invisible layer that grows Kamosu's small buttons to 48 (`tap-out`, `quiet-button`, `shelf-arrow`), rather than the box that is drawn. A walk that reaches a bar stops there, and the control is reported as cut short rather than small. A control whose centre answers with something else, because it is covered, is reported apart. So is a link inside a sentence, which WCAG exempts.
- **Threshold.** None under 48 (ADR 0044).

### 6. Gaps

- **Counts.** Two numbers, over the same probes as Tap size. First, how many pairs of controls have tapped areas that overlap. Second, the smallest distance between the drawn boxes of two neighbouring controls where at least one is drawn under 48 px in either direction. The map words this measure as the gap between tap targets. It takes two numbers because in Kamosu a finger hits the tapped area and an eye sees the drawn box, and they differ for every small button. Controls drawn 48 or larger may touch, as the rows of a list do.
- **Threshold.** No overlapping tapped areas (ADR 0044: neither swallows the other's tap). At least 8 px between a small control and its neighbour (Material's 8 dp, judgement over Apple's 12 to 24 pt, which is padding round a control rather than the gap between two). Mealie's smallest was 4.

### 7. Type and colour

- **Counts.** The distinct font sizes, rounded to whole pixels, and the distinct text colours of all visible text on the screen.
- **Threshold.** Flagged above three sizes or three colours (NN/g's visual hierarchy guideline, the colours standing in for its "contrast variations"). A guideline only, never a failure: the walkthrough judges a flagged screen, and the cooking screen differs on purpose. Mealie showed 7 sizes and 5 colours, so the flag will be common.

### 8. Length

- **Counts.** The page's height in screenfuls, and where the screen's main action starts, in screenfuls from the top. The main action is named per screen in the walkthrough. Where none is named, the first main button stands in.
- **Threshold.** The main action starts within the first screenful (Apple: "place the most important items near the top"). A screen whose body is not a list or a recipe is at most two screenfuls long (judgement: such a screen is mostly interface words).

### 9. Notices

- **Counts.** Notice cards anywhere on the screen. A notice card is a visible element with role `status` or `alert`, at least half the window wide, that is drawn as a box: a border on at least one side, or a background that differs from the colour behind it. In Kamosu that takes in every `ui/src/lib/offline/Notice.svelte` and a boxed status line such as the recipe page's "read again" line. A plain paragraph with role `alert` or `status`, an error under a field, is not a box and not counted.
- **Threshold.** At most one at a time on any screen, and none on the cooking screen (judgement: the survey found none on any screen seen, and banner blindness says a boxed, coloured card is the shape people learn to skip).

### 10. Depth

- **Counts.** For each starting task (find a recipe, open it, share it) and each task the walkthrough adds: the taps from Home to the end of the task, and for each tap whether its control was visible on the screen or behind a menu (☰, ⋯, ⋮, +). Loading one page cannot count it, so `just ux-walk` counts it from each task's recorded path (#639).
- **Threshold.** Every tap on the way is on a visible control, and no task goes more than two levels of disclosure deep (NN/g, progressive disclosure, and the hidden-navigation study, extended from main navigation to menus of actions). Search is at most one tap from Home, a recipe one tap from a list or result, and the share link or share sheet at most two taps from the recipe (judgement: Mealie's and Paprika's search fields need none, and Crouton's share takes one tap to the button).

## Sources not read

- Material Design 3's [navigation bar guidelines](https://m3.material.io/components/navigation-bar/guidelines) failed through both DonSeTch and Tavily. Android's Compose navigation bar page gave the three-to-five rule instead.
- GOV.UK's "Writing for GOV.UK" now redirects to a [tone of voice](https://guidance.publishing.service.gov.uk/writing-to-gov-uk-standards/tone-of-voice/) page that renders with scripts and gave no text. The writing guideline pages and the style guide named above cover the same rules.
- Tandoor's demo (`app.tandoor.dev/e/demo-auto-login/`) now leads to a sign-in page, so no Tandoor screen was counted.
- Crouton's site renders with scripts and gave no text. Mela's help page says nothing about sending one recipe to someone, so Mela's share is unchecked.
- How search is reached in Mela and Crouton: neither first-party source says, and no screenshot shows it.
- Parhi, Karlson and Bederson's touch target study, behind NN/g's 1 cm, was not read at its source.
