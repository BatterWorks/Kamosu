# Type follows the reader's text size, up to 135%

Kamosu's type grows with the text size a person has set on their phone or tablet, from the sizes it was drawn at up to 135% of them, and stops there. At the system's default setting nothing changes. Settled on 2026-10-11 on [#228](https://github.com/BattermanZ/Kamosu/issues/228). Until then every size was a fixed number of px and the setting did nothing.

## What was decided

- **Every type size is a count of one unit**, `--type-unit` in `ui/src/app.css`. The unit is 1px until the reader's text size moves it, so the sizes the identity record fitted to real content are still the sizes at rest.
- **Only type follows.** Spacing, pictures, the width of a column and the height of a control stay in px. The 48px a thumb needs does not shrink or grow with someone's reading.
- **It stops at 135%**, the top of the iPhone's ordinary Text Size slider. Someone who has turned on Larger Accessibility Sizes gets 135% and no more.
- **It never goes under the sizes it was drawn at.** A field whose text is under 16px makes iOS zoom the page when it is tapped, and the smallest slider settings would put every field there.
- **The system is asked in the two ways there are.** Chrome on Android sets the root's font size from the system's once a page carries `<meta name="text-scale" content="scale">`. iOS has no such switch, so the root there is given the system's body font, whose size is the setting's. The Share Link page asks the same way, since a stranger's phone has a text size too.

## Why

- **About a third of phone users have moved the setting** (the CSS Working Group's explainer for the `text-scale` meta gives 37% on Android and 34% on iOS), and an app that ignores it is the one that looks wrong to them.
- **At 135% the screens held as they were.** The recipe page and the cooking screen were shown at 100%, 135%, 200% and 310% with nothing changed but the type. At 135% lines wrap sooner and a long Step scrolls, which is the trade someone who asked for larger text has made. At 200% and 310% words overlap on the cooking screen and the navigation's labels collide.

## Considered options

- **Stay in px.** Rejected: the setting is the reader's, and following it part of the way costs no screen anything.
- **Follow it all the way to 310%.** Rejected for now. Every screen would have to be reworked to reflow at those sizes. It is a ticket of its own if someone in the household needs it, and the 135% ceiling is one number to raise when it is done.

## Consequences

- **#88's fit is a fit at rest.** The share of Steps that show without scrolling was measured at 26px. A reader at 135% sees fewer, by their own choice.
- **The root's font size is no longer 16px everywhere**, so a length written in rem would move with the reader's type. The stylesheet holds none but the unit's own, and `ui/src/lib/type-unit.spec.ts` fails if one comes back.
- **Browsers with neither way leave Kamosu as it was**: Firefox, and Chrome before 146. A person who raised a desktop browser's default font size gets the same following, to the same ceiling.
- **The story's drawings (#158) do not follow.** They are pictures sized to their own frame. The words around them, its language buttons and its button to sign in, do.
