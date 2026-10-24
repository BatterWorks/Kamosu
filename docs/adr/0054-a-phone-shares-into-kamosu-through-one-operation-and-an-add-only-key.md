# A phone shares into Kamosu through one Operation and an add-only key

A cook on a phone can add a recipe from the phone's own Share menu: a web page, an Instagram or TikTok post, one or several photos, copied text or a PDF, then "Add to Kamosu", and a short message says it was sent. On an iPhone the way in is an Apple Shortcut that Kamosu publishes. On Android it is Kamosu itself, installed from Chrome. Both hand what was shared to one new Operation, `add_this`, and show the message it answers with. Settled on 2026-10-24 in a design session (#452). How other recipe apps do it is in `docs/research/sharing-a-recipe-in-from-a-phone.md`.

Until now a recipe found on a phone had to be carried into Kamosu by hand: open Kamosu, find the add box, paste. That works for whoever set Kamosu up and loses everyone else in the household, which is the person this is for.

## What was decided

- **An iPhone shares through a Shortcut, and Kamosu ships it.** iOS lists no web app in its Share menu (WebKit has never shipped Web Share Target), so every self-hosted recipe app without an iPhone app of its own relies on a Shortcut. The maintainer builds it once on an iPhone and publishes its iCloud link. The repository keeps a written list of its steps, so it can be built again if the link is ever lost, and Kamosu's Settings links to it.
- **The Shortcut holds as little as it can.** It sends what it was given to `add_this` and shows the message `add_this` answers with. Kamosu decides everything else: whether a link goes to the plain reader or the agent, what to say on an instance with no agent, the message in the asker's Reading Language. The Shortcut sends its own version number with every call, so Kamosu can say when a newer one is available, and an older one keeps working for as long as Kamosu can still serve it.
- **From Safari, the Shortcut sends the link and not the page.** Kamosu fetches the page itself, as it does from the browser. Where it cannot, because a site refuses it or the page is behind the cook's own login, the message says to copy the recipe's text and share that.
- **It answers at once.** `add_this` starts the import and answers with a message, never with the finished recipe. The recipe arrives on the shelf when it is read. One that fails shows on the Imports screen, as any import does.
- **The Shortcut carries an add-only key.** An **Add-only key** is a third kind of Access Key, beside the read-only key and the Operator key, chosen when it is minted. It may call `add_this`, stage the uploads `add_this` takes, and read how its own imports went. Nothing else. Each declaration in the Catalogue says whether an add-only key may call it, and `Core::execute` enforces that, as it does for a Job Key.
- **Setup is one paste.** A Settings screen mints the add-only key and shows a single setup code, once, holding both the instance's address and the key. The Shortcut asks for that code when it is installed and splits it itself.
- **Several photos shared together are the pages of one recipe.** An import now takes several pictures, and its Original is that set in order. The agent reads them as one recipe. The browser's add place takes several pictures too.
- **Everything to the agent, or only what Kamosu cannot read.** The setup screen asks which one, and keeps the answer with the add-only key. A recipe-card link then goes to the agent too if the cook chose that. It can be changed later in Kamosu, and the Shortcut never needs reinstalling for it.
- **The same link is not added twice.** `add_this` knows a link a recipe the cook can see already came from, or one still being read, and answers with that recipe's name instead of starting another import. Pictures and PDFs were already known by their contents.
- **Android shares into Kamosu itself.** The installed web app declares a share target, so Kamosu is listed in Android's Share menu with no Shortcut and no key, signed in as the browser already is. It calls the same `add_this`. The agent choice is kept with that phone's Session.
- **It is offered on every instance.** Without a Runner, a recipe-card link is added in full and a page with no recipe card waits on the Imports screen to be checked. Text, pictures and PDFs are refused with a message saying so. Where a Runner exists but none has asked for work lately, the message says the agent is not answering and will read it when it is back.

## Why

- **Shipping the Shortcut, not documenting how to build one.** A Shortcut each person builds from instructions is beyond the people this is for. The prior art shows the cost of the other road too: Mealie's, Norish's and Tandoor's Shortcuts were written outside their projects, and each broke when the server they called changed.
- **One Operation, so the Shortcut stays dumb.** The Shortcut is the one piece of Kamosu on someone else's phone that a release cannot update. Every rule written into it is frozen until that person reinstalls it, which most never do. A rule in `add_this` improves with every release, and an agent at the MCP Door gets the same "add this, whatever it is".
- **An add-only key, not an ordinary one.** The Shortcut keeps its key in plain view in the Shortcuts editor, and the key goes along if the Shortcut is shared from that phone. An ordinary key acts with the whole of its Person's reach, and in Kamosu that includes deleting a recipe for good. [ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md) answers a leaked key by visibility rather than restriction, because scoping a key to one Kitchen would be a wall at every Operation. A key kind with a short list of what it may call is no such wall: it is the shape of the read-only key and the Job Key, decided once per declaration. A leaked add-only key can only add recipes, which a cook can see and delete.
- **The link only, because a pop-up costs more than it buys.** Sending the page from Safari needs a setting buried in iOS and a permission prompt the first time the Shortcut meets each website. That lands on the person the one tap was for. What it would have reached, paid and bot-blocking sites, still has a way in through copied text.
- **Several pictures, not one stitched picture.** Mealie stitches photos into one image because a Shortcut sends one file per field, and its docs admit the text is read less well. A model shrinks a picture before reading it, so three phone photos in one strip lose the small print of a cookbook page. The cost is Kamosu's own, and it closes a gap the browser already had.
- **Offered everywhere.** Most instances have no Runner. Links are what a phone shares most, and a recipe-card link needs no agent.

## Considered options

- **Opening Kamosu's page in Safari with the link in the address**, as Recipe Filer recommends, with no key at all. Rejected: a picture or a PDF does not fit in an address, and Safari's sign-in is separate from the Home Screen app's.
- **A Shortcut that calls today's Operations one after another.** Rejected above: every rule in it is frozen on strangers' phones.
- **An ordinary Access Key in the Shortcut.** Rejected above.
- **Two published Shortcuts, one always using the agent.** Rejected: two links and two step lists to keep, and changing one's mind means reinstalling.
- **Waiting in the Shortcut until the recipe is read**, then offering to open it. Rejected: a read can take minutes, and iOS may stop a Shortcut that is left.
- **A clipboard fallback for Instagram**, run from the Home Screen. Not taken: Instagram's own Share button reaches the system Share menu through "Share to…", so the Shortcut is listed there.

## Consequences

- **The published Shortcut lives on the maintainer's iCloud account.** Apple publishes a Shortcut only from an Apple device, so it cannot be built from the repository. The step list is the way back if the link is lost, and a release that changes the Shortcut changes the link Kamosu shows.
- **`add_this` is a contract with phones Kamosu cannot reach.** What it takes and answers changes only in ways an older Shortcut survives, and the version number is how Kamosu says when one has to be replaced.
- **An Original can be several files.** Reading again and sending again handle the set, and the agent fetches each one.
- **Messages are written by the server.** `add_this` words its answer in the asker's Reading Language, in all three languages, as a Sheet and the Share Link page already do.
- **Android is tested through Chrome's Android emulation** until someone with an Android phone tries it.
