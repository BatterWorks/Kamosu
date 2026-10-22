# The Sample Library

A Demo holds this library (ADR 0052). `story.json` is a story of what its cooks
did, step by step, and a Demo plays it through Kamosu's real Operations each
time it starts. Then it pours the result into its own database, at startup and
every day at 04:00 UTC, with every date moved forward to today. The photographs
the story names sit beside it, and both are built into the binary.

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

`story.json` does this once, for the lemon vinaigrette.
