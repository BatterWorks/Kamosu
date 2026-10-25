# The Sample Library

A Demo holds this library (ADR 0052). `story.json` is a story of what its cooks
did, step by step, and a Demo plays it through Kamosu's real Operations each
time it starts. Then it pours the result into its own database, at startup and
every day at 04:00 UTC, with every date moved forward to today. The photographs
and files the story names sit beside it, and all of it is built into the binary.

Build it on its own with `just sample-library`. It prints what the library holds
and how long it took, and stops at the first step that fails, naming it.

## A step

Each step happened some days before the library is built, at a time of day in
UTC:

```json
{ "days_ago": 58, "at": "19:45", "as": "Camille", "op": "create_recipe", "input": { "title": "Tarte Tatin" }, "keep": "tarte" }
```

- `days_ago` is one at least, so nothing in the story is later than the day
  it is told. `at` is `HH:MM`.
- Steps are in order. A step dated before the one above it fails the build.
- `as` is who acts. It is a cook's name, or `{ "$ref": "…" }` naming a Secret
  an earlier step kept, such as a Runner Key's or a Job Key's.
- `keep` names the step's answer, so a later step can use what is in it.

A step does exactly one of these things:

| Field | What happens |
|---|---|
| `"first_person": "Camille"` | The first Person signs up, an Operator. The story starts here, and a visitor arrives as the cook named Camille, so the story must have a Camille. |
| `"person": "Mum"` | Another Person signs up through an Invite that the `as` cook mints. |
| `"op": "<name>", "input": {…}` | The `as` cook calls an Operation, as either Door would. The build has no Job lanes, so it cannot call a Job such as `import_web_link`. |
| `"photograph": "photographs/<file>"` | The `as` cook uploads a picture from this folder. Its `photograph_id` goes into a recipe's `main_photo`. |
| `"published_key": "<name>"` | The one Access Key the Demo publishes: read-only, the `as` cook's, and always the same Secret, `DEMO_ACCESS_KEY` in `src/core/demo.rs`. |

## Naming what an earlier step answered

Anywhere in `input`, `{ "$ref": "name.path" }` stands for a value from the
answer a step kept as `name`. Each dot goes one field or one list position
deeper: `{ "$ref": "tarte.branch_id" }`, `{ "$ref": "work.job.job_key" }`.
`{ "$ref": "kamosu.version" }` is this Kamosu's own version, which a Runner in
the story reports when it asks for work.

Two more stand for a file in this folder, so a step can hand a cook's file to
the agent:

- `{ "$file": "originals/orange-cake-card.jpg" }` is the file's bytes in
  base64, as `import_by_agent` takes a picture or a PDF in `data`.
- `{ "$text": "originals/sunny-side-shakshuka.txt" }` is the file's words, as
  `import_by_agent` takes pasted text in `text`.

## Time in the story

Every Operation stamps the present. Once a step has run, everything it stamped
moves back to the step's own moment, and the times keep their distance from
one another. Two things follow from that:

- **Saves in the story never fold into one another.** A real save within an
  hour of the cook's last one joins that Version. In the build, the step before
  already sits days back, so the next save starts a new Version. Write one step
  for each Version you want in the Thread.
- **Never send a time of your own**, such as `written_at` or `started_at`. No
  step moves it back, so it stays on the date you wrote rather than on the
  step's own day. The build does refuse a library that keeps any time from the
  day it was built.

A deadline a step sets ahead of itself, such as a Runner's hold on a Job, runs
in the build's own time, so the steps after it still find it ahead. When the
story ends, any deadline still ahead moves back with the last step.

## The agent's work

An Import Report comes from a real agent Job:

1. The cook mints a Runner Key (`mint_runner_key`) and hands something to the
   agent (`import_by_agent`).
2. The Runner Key asks for work (`ask_for_work`). Keep the answer: it holds the
   Job's id and its Job Key.
3. The Job Key makes the recipe (`create_recipe`).
4. The Runner Key ends the Job with the agent's report (`report_on_job`).

Between the last two, the Job Key uploads the frames or the page's picture
with a `photograph` step, as an agent sends them to `POST /api/photographs`,
and corrects the Readings it checked with `set_reading`. A
Job that cannot be read ends with `outcome` `could_not_be_read` and a `reason`,
and makes no recipe.

A Job reading a file keeps it as its Original for 30 days after it ends. A
recipe read from one is read again from it while it is kept, and asks for the
file again once it is gone. A Demo moves every date forward by whole days, so
an Original stays as old as the story made it: hand files over in the story's
last four weeks. A visitor, who is Camille, reads only the Jobs that Camille or
Lucía asked for, and sees the Read Again card only on their recipes, since it
is for the recipe's Co-authors.

## What the library holds

About 40 recipes, written as the Demo's cooks would have written them:

- **Camille**, the Operator, a visitor's own self. **Mum** cooks with Camille
  in the Kitchen called Family. **Lucía**, a friend in a Kitchen of her own,
  joined her Cookbook to Camille's, so the two write one Cookbook together.
- Twelve recipes come from a real household library, with their history, the
  names taken out. The rest were written for the Demo.
- Twelve the agent read from videos, four each from YouTube, Instagram and
  TikTok, tagged with their platform. Their photographs are frames of the
  video, and each recipe's Source links the video and, where the agent found
  one, names its creator. Four old
  recipe cards, a magazine page and two pasted texts were read by the agent
  too. Every one of those recipes, its frames and its Import Report is
  what Kamosu's own agent made of that source, run through a Runner and
  copied into the story as it was, the Readings it corrected included. No
  recipe here comes from behind a paywall.
- About a third have a Translation into French or Spanish. The wet-wet sauce is
  a Component of the garlic butter noodles and the wontons.
- Waiting for a visitor: a Share Link to the Tarte Tatin, a Kitchen Invite, a
  Cookbook Invite, and the pizza dough's cooking with changed lines that were
  never kept.

## The examples a Demo replays

A Demo shows the agent at work on four of the agent's Jobs (#406). The story
names them in `replays`, after its steps, and the + offers them in that order:

```json
{ "kind": "picture", "job": { "$ref": "hand_orange_cake.job_id" }, "picture": { "$ref": "replay_card.photograph_id" } }
```

- `kind` is what was handed over: `picture`, `pdf`, `pasted_text` or `video`.
  The interface names each example by its kind, and says which dish it holds.
  So an example swapped for another dish changes the words in
  `ui/messages/` too.
- `job` is the `job_id` of an `import_by_agent` step. Its Job is one Camille
  asked for, whose agent made its recipe. A visitor is Camille, so it is one
  they can read. Anything else fails the build.
- `picture` is a Photograph of what was handed over, or `excerpt` is the
  top of the text as a visitor would want to see it: the post's own title
  and first lines, not the site's menu above them. One of the two, never
  both. The card and the
  magazine page are uploaded as Photographs of their own for this, since an
  Original is the agent's alone to read.

| Example | What was handed over | Recipe |
|---|---|---|
| A handwritten card | `originals/orange-cake-card.jpg` | Orange Cake |
| A blog post, pasted | `originals/sunny-side-shakshuka.txt` | Sunny Side Shakshuka |
| A magazine page | `originals/the-weekend-table-ratatouille.pdf`, printed from the `.html` beside it | Summer ratatouille, sliced and baked |
| A YouTube video | the pizza dough's link | Easiest Actually Good Pizza Dough |

`CREDITS.md` names who made every picture here that is not the project's own,
and under which licence. A picture added to the library adds its line there.
