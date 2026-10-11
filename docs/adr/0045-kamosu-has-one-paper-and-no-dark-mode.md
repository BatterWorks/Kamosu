# Kamosu has one paper, and no dark mode

Kamosu is indigo ink on unbleached paper whatever the device is set to. It does not follow the system's dark setting, and `ui/src/app.css` holds no `prefers-color-scheme` rule on purpose. Settled on 2026-10-11 on [#228](https://github.com/BattermanZ/Kamosu/issues/228), after the audit of 9 October 2026 found that nothing recorded the answer.

## What was decided

- **One palette.** The ground is kinari and the ink is sumi on every screen but one, at every hour.
- **The cooking screen stays the one dark room.** The identity record of 26 August 2026 turns it indigo so that standing at the stove is another room. That is also the screen a dim kitchen at ten at night is spent on.

## Why

- **Every screen would be designed and checked twice from then on.** A dark paper is a second palette, and each new control needs a place in it. Kamosu is built by one person choosing from prototypes, and that would double what is put in front of them.
- **Indigo stops being what you tap.** The identity gives indigo one job, and indigo cannot be seen on a sumi ground. The dark prototype had to lift it to a pale indigo and give filled buttons dark words, which is a second identity more than a second ground.
- **The cooking screen would stop being another room.** Beside a sumi page the indigo room is one dark screen among many.
- **The case for it is already met.** The audit named an iPad in a dim kitchen. What is open there is the cooking screen.

## Considered options

- **A dark paper that follows the system**, sumi for the ground and kinari for the ink, shown on Home, Recipes and a recipe page at every size. The navigation, Covers and photographs kept their colours. It read well, and was turned down for the costs above.
- **A switch in Settings.** Not offered. It carries the same second palette and adds a setting.

## Consequences

- **A browser's own forced dark mode is not Kamosu's.** Nothing here opts in or out of one.
- **Reopening this is a design ticket with prototypes**, since the cost is in every screen. The pages this was judged on were a prototype and are not kept. The palette they used is on #228.
