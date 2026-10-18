# An agent beside Kamosu reads what Kamosu cannot

Kamosu's importer reads a recipe out of a page's structured data and out of nothing else. A page without that data, a picture, a scanned PDF and a video all need a model to read them, and Kamosu calls none. Settled on 2026-10-13: Kamosu ships a **Runner**, a program that runs beside it, asks it for work and has an agent do each **Job**. Kamosu itself calls no model and holds no provider's key.

The survey behind this is in [`docs/research/ai-recipe-import-in-other-apps.md`](../research/ai-recipe-import-in-other-apps.md).

## What was decided

### The Runner

- **Kamosu ships the Runner and nothing else of the agent.** The Runner is Kamosu's own program started in another mode. Whoever runs it supplies the agent it starts, the tools that agent needs and the instructions it follows. Kamosu's image stays one program ([ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md)). _(Amended by [#345](https://github.com/BatterWorks/Kamosu/issues/345): **every release also publishes the agent's image Kamosu's repository builds, as `battermanz/kamosu-agent`.** It holds opencode, its tools and the instructions in `agent-image/`, and it is a second image beside Kamosu's, which stays one program. Whoever runs a Runner still chooses: that image or an agent of their own, the provider and the model, and, by mounting a file over the image's, the instructions. Nothing in the image names a provider or a model.)_ _(Amended by [#344](https://github.com/BatterWorks/Kamosu/issues/344): **Kamosu holds one part of the instructions, an Operator's House Notes.** They go on the end of whatever instructions the agent has, and every Job carries them to the Runner, which passes them on unread. Only an Operator writes them, because the agent's container holds the provider's key and what the instructions say is what the agent obeys. They never travel by the Job Key, whose reach stays as below. The image's own instructions stay the image's, and a release improves them under the notes.)_
- **The Runner asks, and Kamosu never calls it.** The Runner asks for a Job, keeps renewing its hold on it while the agent works, and reports how it ended. Kamosu owns the deadline, so a Job whose Runner goes quiet goes back to waiting. Cancelling travels in Kamosu's reply to a call the Runner was already making, and the Runner stops its agent. Kamosu stores no address for a Runner.
- **A Runner holds one Job at a time,** and a Job is held by one Runner at a time.

### Two keys

- **The Runner Key is the Runner's own.** An Operator mints it. It can ask for work, renew a hold and report on a Job, and nothing else. It never acts as a Person and it cannot touch a recipe.
- **Each Job comes with a Job Key.** Kamosu hands it to the Runner with the Job, and the Runner hands it to the agent. It acts as the Person who asked for that Job, under their Hand, and only for that Job. It is spent when the Job ends.
- **The agent never holds the Runner Key.** So an agent that a page has tricked can reach the one Job it was given and no other.
- **Both are Secrets like the rest:** stored hashed, revoked or spent, never on a clock ([ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md)).

### What a Job Key may do

- **Five things.** Read the library as the asker sees it, and the Job's own **Original**. Write and correct the one recipe the Job is for, creating it first where the Job reads an Original and so has none. Correct that recipe's Readings. Upload Photographs. Put existing Tags on that recipe.
- **Nothing else.** No Translation, no Related Recipe, no Food name, no Merge, no deleting a Food, no new Tag, no deleting a recipe, no other recipe, no account.
- **It can do no more than the asker could.** The Core's one check asks two things of every call: is this on the Job Key's list, and could the asker do it themselves right now.
- **It never makes a Copy.** If the asker has stopped being a Co-author of the Cookbook, their own edit would start a Copy in their own Cookbook. A Job's write is refused there, and the Job fails and says why.
- **The MCP door shows a Job Key only what it may call,** as it already hides writing from a read-only Access Key.

### Which Imports go to an agent

- **An instance has a Runner when it has a Runner Key that is not revoked.** Without one, nothing changes.
- **A page that yields a structured recipe is read by Kamosu, as before.** It goes to the agent only when a Co-author asks for it to be read again.
- **A link Kamosu cannot read lands as a bare recipe, and a Job for the agent follows.** That covers a page with no structured data, a video, and a page whose fetch was refused, which today lands nothing at all. _(Amended by [#349](https://github.com/BatterWorks/Kamosu/issues/349): **a video's link sent from the + list, or as `link` in `import_by_agent`, goes to the agent unread**, and no bare recipe lands before the agent's. A video's link given as an ordinary link still lands bare first.)_
- **Pasted text, a PDF and a picture go to the agent.** The recipe appears when the Job is done.

### Reading again

- **Any recipe with something to read again can be sent to the agent,** whichever way it arrived. That something is its Source's link or its Original.
- **Only a Co-author of the Cookbook that holds the recipe may ask.** A fresh Import lands in the asker's own Cookbook, as every Import does. Nobody holding only a Share Link can cause a Job for a Runner ([ADR 0032](./0032-a-stranger-may-cause-work-never-work-that-scales-with-them.md)).
- **A Job leaves at most one new Version.** The agent's saves within a Job shape that one Version. It never folds into a Version that was there before the Job, and a person's later save never folds into it.

### Originals

- **An Original is what a Job reads when that is not a link:** a PDF, a picture or pasted text.
- **It is kept for 30 days and then deleted.** After that, reading again asks for the file. The days are counted from the Job that last read it: reading a recipe again from its Original makes that Job keep it too, for 30 days of its own (#273).
- **It is never part of the recipe** ([ADR 0025](./0025-an-imported-recipe-is-an-ordinary-recipe.md)), and it is in no Backup.

### What lands

- **An ordinary recipe, saved at once.** There is no approval step. The Import Report says an agent made it, and lists what the agent left undone or was unsure of.
- **One good recipe, checked against its source and corrected.** A likely duplicate is saved and flagged, not blocked. The recipe stays in the language of its source.
- **A Job that keeps failing stops and says so.** Kamosu sets no other limit on who may ask or how often. When no Runner has asked for work lately, Kamosu says so, and a Job can be cancelled whether it is waiting or being worked. _(Amended by [#279](https://github.com/BatterWorks/Kamosu/issues/279): **a recipe is read again once.** After a reading that finished, or one stopped while a Runner held it, `read_recipe_again` refuses with reason `already_read_again`. One that failed, or was stopped while it waited, gives the turn back, and a variation has a reading of its own.)_

## Why

- **The standard was set by an agent, and only an agent meets it.** The maintainer's library is built by an agent following a written skill: it reads the source, saves, reads the save back, compares each number and corrects. Video adds steps that are judgement made in the middle of the task, such as checking that a written recipe found elsewhere is the same dish, or going back for a sharper frame of a label.
- **Kamosu's image cannot carry the tools.** Video needs a downloader and a frame cutter, and the image is one program with no shell. The survey found that the downloader's stable build lags site changes by weeks, that the tools which survive update themselves, and that where the fetch runs decides whether a site answers at all. All of that belongs outside the binary.
- **The worker asking for work is the established contract.** GitHub Actions, GitLab Runner, the Forgejo runner and Woodpecker all have the worker open every connection, hold a lease the server can let lapse, and hear "cancel" inside a reply. The one surveyed case of the app calling the worker had no authentication on that call.
- **A key for each Job, because the agent reads strangers' text with nobody watching.** [ADR 0034](./0034-kamosu-names-what-it-does-not-defend.md) says an agent does what it is told, including by text it reads. An Access Key acts as its Person everywhere, so an unattended agent holding one could empty a library on a web page's say-so. With a Job Key the worst outcome of a successful trick is one wrong recipe. The job runners surveyed give each job a credential of its own for the same reason.
- **Acting as the asker keeps authorisation in one place.** The Core's single check still decides everything ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)). No Operation learns about Jobs to decide what is allowed. The Job travels with the key. A save does write down which Job wrote a Version, which is how a Job leaves one.
- **Saved at once, because a recipe is visible and reversible.** A wrong recipe sits in its Co-author's own Cookbook with every earlier Version behind it. What guards quantities is the agent's own check against the source, which a tap of approval on a phone would not repeat.
- **"Any provider" is met by the Runner starting any agent.** Kamosu does not need to speak to a model for an Operator to choose one.

## Considered options

- **Kamosu calls a model itself.** This is what all six surveyed recipe apps do, with a base URL, a key and a model name. It was designed in full under the name "Scribe" and tested on 2026-10-13 (part 5 of the research note). It read faithfully: across seven pages no number in its recipes was absent from the page. Set aside, and not for reading badly. It was about half of the work, it would have put a provider's key in Kamosu for the first time, and an instance that runs an agent gains only seconds from it. It is the option to reopen for instances that will never run a Runner.
- **One key for the Runner and the agent alike.** This was the first design. Rejected after review: an agent holding the Runner's key could take every waiting Job, every Operation would have had to learn which Job a call was for, and a key that "acts as nobody" between Jobs has no place in how Kamosu knows a caller.
- **An agent holding an ordinary Access Key.** Rejected for its reach, and because every recipe would land under the key's own Person whoever asked.
- **Letting the agent delete a Food its Job created and left unused.** Rejected. Only an Operator deletes a Food, and nothing records which save created one. The agent names the strays in its report.
- **Kamosu calls the agent.** Rejected: it needs the agent's address and a server in the agent's container.
- **A fetcher with no model beside Kamosu, feeding a model call inside it.** Rejected with the first option, and because it loses the steps that are judgement.
- **A provider's hosted agent.** Rejected: it ties the feature to one provider.
- **Asking a connected agent through MCP.** Not available. In revision 2026-07-28 a server may only speak inside a request the client already sent, and sampling is deprecated.
- **A complete image with an agent in it, published by Kamosu.** Deferred. It needs a public set of instructions and one agent chosen for everybody, which the maintainer's own setup should inform first. _(Amended by [#345](https://github.com/BatterWorks/Kamosu/issues/345): taken up on 2026-10-17. Both conditions are met: opencode is the agent, because it speaks to any provider, and `agent-image/instructions.md` is public, tested on real sources in #324. The image carries Kamosu's version and is built again only when `agent-image/` changes, or when a release moves its rebuild line for Alpine's security fixes.)_
- **Holding the recipe until someone approves it.** Rejected: a waiting state for a whole recipe is something every list and search would have to respect.

## Consequences

- **An instance without a Runner loses nothing, and keeps one small gain of its own (#301).** A link to a page with no recipe data is read from the page's own article text, the way a paste is, and shown on the screen a paste is checked on, with the page as its Source. Nothing is saved until a person makes the recipe from there. A page whose text holds no Ingredient Line lands bare, as before. Beside a Runner the same page goes to the agent instead: an agent reads better than the paste reader, and the check would only stand in its way.
- **An agent's Import takes minutes**, where a page Kamosu reads itself takes seconds.
- **Two new kinds of caller exist.** Until now every Credential named a Person from the start. The glossary's entries for Credential, Access Key and Secret change with this record.
- **The Job Key's list is the security boundary.** Adding to it is a decision of this weight, never a convenience.
- **The Runner speaks to one address, the Kamosu its Operator named.** That is a second place the program makes a request from, beside the guarded client of [ADR 0033](./0033-kamosu-assumes-the-proxy-did-nothing-but-carry-the-bytes.md), which is for addresses a stranger can point Kamosu at. The Runner's never is one, and it is often a private address.
- **One agent process per Job, in a fresh folder, inside the Runner's own container.** Vendor guidance prefers a fresh container per task. The Job Key's reach is what makes the lighter arrangement acceptable, and giving the Runner control of the container engine would cost more than it buys. _(Amended by [#317](https://github.com/BatterWorks/Kamosu/issues/317): **a Runner can instead hand each Job to an agent in a container of its own**, through a Unix socket in a volume the two share. That container is read-only, takes one Job and starts afresh for the next, so the Runner Key is in another container altogether. The Runner still has no control of the container engine. The published agent image works this way.)_
- **Kamosu cannot see what a Job cost.** The model's bill belongs to whoever runs the Runner, and so does any limit on it.
- **Stray Foods stay until someone tidies them.** An agent that corrects a Reading can leave the Food it replaced with nothing pointing at it.
- **Instructions written for an agent with a person present have to learn to work alone.** Where they would stop and ask, the agent leaves the thing undone and says so in the Import Report.
- **Two things are not measured yet:** what an agent's Job costs and how long it takes, and whether the video sites answer the machine a given Runner is on. _(Amended by [#318](https://github.com/BatterWorks/Kamosu/issues/318) and [#325](https://github.com/BatterWorks/Kamosu/issues/325): the first is measured. The agent's log gives each Job's time and cost, and `docs/runner.md`, under "Time, tries and cost", records both across six model settings.)_
