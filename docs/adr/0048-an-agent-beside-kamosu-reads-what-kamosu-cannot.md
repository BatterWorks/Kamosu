# An agent beside Kamosu reads what Kamosu cannot

Kamosu's importer reads a recipe out of a page's structured data and out of nothing else. A page without that data, a picture, a scanned PDF and a video all need a model to read them, and Kamosu calls none. Settled on 2026-10-13: Kamosu ships a **Runner**, a program that runs beside it, asks it for work and has an agent do each **Job**. Kamosu itself calls no model and holds no provider's key.

The survey behind this is in [`docs/research/ai-recipe-import-in-other-apps.md`](../research/ai-recipe-import-in-other-apps.md).

## What was decided

- **Kamosu ships the Runner and nothing else of the agent.** The Runner is Kamosu's own program started in another mode. Whoever runs it supplies the agent it starts, the tools that agent needs and the instructions it follows. Kamosu's image stays one program ([ADR 0028](./0028-kamosu-is-one-file-and-one-directory.md)).
- **The Runner asks, and Kamosu never calls it.** The Runner asks for a Job, keeps renewing its hold on it while the agent works, and hands back the result. Kamosu owns the deadline, so a Job whose Runner goes quiet returns to the line. Cancelling travels in Kamosu's reply to a call the Runner was already making. Kamosu stores no address for a Runner and makes no new outbound call, which leaves [ADR 0033](./0033-kamosu-assumes-the-proxy-did-nothing-but-carry-the-bytes.md) as it was.
- **A Runner Key is a third source of a Credential.** An Operator mints it, and it is a **Secret** like the others: shown once, stored hashed, revoked and never on a clock ([ADR 0031](./0031-a-secret-is-spent-or-revoked-never-on-a-clock.md)). For one Job it acts as the Person who asked for that Job, under their Hand. Between Jobs it acts as nobody and can only ask for work.
- **For a Job, a Runner Key may do six things.** Read the library as the asker sees it. Write and correct the one recipe the Job is for. Correct that recipe's Readings. Upload its Photographs. Put existing Tags on it. Delete a Food the Job itself created and then left unused.
- **It may do nothing else.** No Translation, no Related Recipe, no Food name, no Merge, no new Tag, no deleting a recipe, no other recipe, no account.
- **Every check that binds the asker binds the Job.** A write is refused wherever the asker's own would be: after they left the Cookbook, or after their account was disabled.
- **Who may ask.** A fresh Import lands in the asker's own Cookbook, as every Import does. Reading an existing recipe again is offered only to a Co-author of the Cookbook that holds it. Nobody holding only a Share Link can cause a Job for a Runner ([ADR 0032](./0032-a-stranger-may-cause-work-never-work-that-scales-with-them.md)).
- **Which Imports go to a Runner.** With no Runner, nothing changes. With one, a page that carries structured data is still read by Kamosu, and goes to the agent only when a Co-author asks for it to be read again. A page without structured data, a video link, pasted text, a PDF and a picture go to the agent. For a link, the bare recipe lands at once as it does today, and the agent fills it in.
- **Any recipe with something to read again can be sent to the agent**, whichever way it arrived. That something is its Source's link or its **Original**. The result is a new Version.
- **An Original is kept for 30 days.** It is the PDF, picture or pasted text an Import was read from. It belongs to the Import and never to the recipe ([ADR 0025](./0025-an-imported-recipe-is-an-ordinary-recipe.md)), and it is in no Backup. After 30 days, reading again asks for the file.
- **What the agent makes is an ordinary recipe, saved at once.** There is no approval step. The Import Report says an agent made it, and lists what the agent left undone or was unsure of.
- **The job is one good recipe, checked against its source and corrected.** A likely duplicate is saved and flagged, not blocked. The recipe stays in the language of its source.
- **Kamosu sets no caps.** A Runner does one Job at a time and the rest wait. When no Runner has asked for work lately, Kamosu says so, and a waiting Job can be cancelled.

## Why

- **The standard was set by an agent, and only an agent meets it.** The maintainer's library is built by an agent following a written skill: it reads the source, saves, reads the save back, compares each number and corrects. Video adds steps that are judgement made in the middle of the task, such as checking that a written recipe found elsewhere is the same dish, or going back for a sharper frame of a label.
- **Kamosu's image cannot carry the tools.** Video needs a downloader and a frame cutter, and the image is one program with no shell. The survey found that the downloader's stable build lags site changes by weeks, that the tools which survive update themselves, and that where the fetch runs decides whether a site answers at all. All of that belongs outside the binary.
- **The worker asking for work is the established contract.** GitHub Actions, GitLab Runner, the Forgejo runner and Woodpecker all have the worker open every connection, hold a lease the server can let lapse, and hear "cancel" inside a reply. The one surveyed case of the app calling the worker had no authentication on that call.
- **The key is narrow because the agent reads strangers' text with nobody watching.** [ADR 0034](./0034-kamosu-names-what-it-does-not-defend.md) says an agent does what it is told, including by text it reads. An Access Key acts as its Person everywhere, so an unattended agent holding one could empty a library on a web page's say-so. With a Runner Key the worst outcome of a successful trick is one wrong recipe.
- **Acting as the asker keeps authorisation in one place.** The Core's single check still decides everything ([ADR 0001](./0001-both-doors-generated-from-one-operation-catalogue.md)). A Runner Key adds a list of what it may attempt, and the asker's own permissions decide the rest.
- **Saved at once, because a recipe is visible and reversible.** A wrong recipe sits in its Co-author's own Cookbook with its history. What guards quantities is the agent's own check against the source, which a tap of approval on a phone would not repeat.
- **"Any provider" is met by the Runner starting any agent.** Kamosu does not need to speak to a model for an Operator to choose one.

## Considered options

- **Kamosu calls a model itself.** This is what all six surveyed recipe apps do, with a base URL, a key and a model name. It was designed in full under the name "Scribe" and tested on 2026-10-13 (part 5 of the research note). It read faithfully: across seven pages no number in its recipes was absent from the page. Set aside, and not for reading badly. It was about half of the work, it would have put a provider's key in Kamosu for the first time, and an instance that runs an agent gains only seconds from it. It is the option to reopen for instances that will never run a Runner.
- **An agent holding an ordinary Access Key.** Rejected for its reach, and because every recipe would land under the key's own Person whoever asked.
- **Kamosu calls the agent.** Rejected: it needs the agent's address, a server in the agent's container, and an exception to ADR 0033 for a private address.
- **A fetcher with no model beside Kamosu, feeding a model call inside it.** Rejected with the option above, and because it loses the steps that are judgement.
- **A provider's hosted agent.** Rejected: it ties the feature to one provider.
- **Asking a connected agent through MCP.** Not available. In revision 2026-07-28 a server may only speak inside a request the client already sent, and sampling is deprecated.
- **A complete companion image published by Kamosu.** Deferred. It needs a public set of instructions and one agent chosen for everybody, which the maintainer's own setup should inform first.
- **Holding the recipe until someone approves it.** Rejected: a waiting state for a whole recipe is something every list and search would have to respect.

## Consequences

- **An instance without a Runner gains nothing from this and loses nothing.**
- **An agent's Import takes minutes**, where a page Kamosu reads itself takes seconds.
- **A fourth kind of caller exists.** Until now every Credential named a Person from the start. The glossary's entries for Credential, Access Key and Secret change with this record.
- **The Runner Key's list is the security boundary.** Adding to it is a decision of this weight, never a convenience.
- **One agent process per Job, in a fresh folder, inside the Runner's own container.** Vendor guidance prefers a fresh container per task. The Runner Key's reach is what makes the lighter arrangement acceptable, and giving the Runner control of the container engine would cost more than it buys.
- **Kamosu cannot see what a Job cost.** The model's bill belongs to whoever runs the Runner, and so does any limit on it.
- **Instructions written for an agent with a person present have to learn to work alone.** Where they would stop and ask, the agent leaves the thing undone and says so in the Import Report.
- **Two things are not measured yet:** what an agent's Job costs and how long it takes, and whether the video sites answer the machine a given Runner is on.
