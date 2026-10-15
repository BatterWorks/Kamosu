# Setting up a Runner

By itself, Kamosu reads a recipe out of a web page's recipe data, and splits
pasted text or a PDF into a draft that someone sorts by hand. A photo of a
cookbook page, a scanned PDF, a video or a page with no recipe data needs a
model to read it, and Kamosu calls none. A **Runner**
is how an instance gets them read. It is a small program that runs beside
Kamosu, asks it for work, and starts an agent of your choosing for each piece
of work. The agent reads the thing and saves the recipe into Kamosu.

An instance without a Runner is complete. It reads what it always read, and
nothing in it waits for one. This guide is for the Operator who wants more read.
The decision behind all of it is
[ADR 0048](adr/0048-an-agent-beside-kamosu-reads-what-kamosu-cannot.md).

## What Kamosu ships, and what you supply

**Kamosu ships the Runner and nothing else of the agent.** The Runner is the
same `kamosu` program as the server, started as `kamosu runner`. You supply
three things:

- **The agent**: Claude Code, Codex, or any program that can follow
  instructions and call Kamosu over MCP or HTTP.
- **Its tools**: whatever it needs to read what it is handed. A video needs a
  downloader and something to cut frames; a picture needs a model that reads
  pictures.
- **Its instructions**: what to do with a Job, written for an agent working
  alone. See [Writing the agent's instructions](#writing-the-agents-instructions).

Kamosu ships no agent, no instructions and no image with an agent in it. Do not
look for one. Kamosu's own image holds one program and no shell, so the agent
cannot live in it either.

## How it works

The Runner asks Kamosu for a **Job**: one thing to read for one person. When
Kamosu has one, the Runner makes a fresh folder, writes the Job down in it as
`job.json`, and starts your agent command in that folder. The Job comes with a
**Job Key**, a Secret that lets the agent act for the person who asked, on that
one recipe, until the Job ends. The agent reads the thing, saves the recipe
with the Job Key, leaves `report.json` in the folder and exits. The Runner
reports the Job to Kamosu with that report, removes the folder, and asks again.

The Runner always opens the connection. Kamosu never calls it and stores no
address for it, so the Runner needs no open port. While the agent works, the
Runner keeps telling Kamosu it still holds the Job. If it goes quiet, Kamosu
lets the Job wait for another Runner.

These are the Jobs that go to an agent, once a Runner Key exists:

- A link Kamosu cannot read: a page with no recipe data, a video, or a page that
  refused Kamosu's fetch. The recipe lands bare at once, and the agent fills it
  in.
- Pasted text, a PDF (a scan included) or a photo of a page, from the **+**
  beside the Recipes search box. The recipe appears when the agent creates it.
- A recipe a Co-author asks to have read again, from its page.

## What you need

- **A Kamosu you are an Operator of**, reached at an address the Runner's
  machine can open. A private address such as `http://192.0.2.10:5266` is fine.
- **A Linux machine for the Runner**, amd64 or arm64. The program needs glibc
  2.38 or later and the C++ library of GCC 13 or later: Debian 13 and Ubuntu
  24.04 have both, Debian 12 and Alpine have neither. It reaches an `https`
  address through the machine's own certificates, so install `ca-certificates`
  where it is missing.
- **The Runner of the same version as the Kamosu it asks.** Kamosu refuses a
  Runner of any other version. Upgrade both together.
- **An agent that runs unattended** on that machine, started by one command.

## 1. Make a Runner Key

The Runner signs in with a **Runner Key**. It can ask for work, keep hold of a
Job and report on it, and nothing else. It acts as no person and cannot touch a
recipe.

1. Sign in to Kamosu as an Operator, open **Settings**, then **Administer this
   instance**.
2. Open the **Instance** tab and find **Runner**.
3. Give the key a name that tells this Runner from another, such as
   `kitchen laptop`, and press **Make a Runner Key**.
4. Copy the key. Kamosu shows it once. If you lose it, revoke it and make
   another.

From this moment the instance **has a Runner**, whether or not one is running:
the + offers photos and PDFs for the agent, and links Kamosu cannot read wait
for one. Start the Runner soon after, or revoke the key until you are ready.

Keep the key out of shell history and out of any file the agent can read. The
examples below keep it in a file of its own, readable by the Runner's user
alone. `read -rs` takes the key as you paste it, without showing it and
without it landing in your shell's history:

```sh
install -m 600 /dev/null ~/.config/kamosu-runner.env
read -rs key && printf 'KAMOSU_RUNNER_KEY=%s\n' "$key" >> ~/.config/kamosu-runner.env; unset key
```

## 2. Put the program beside the agent

The program is `/app/kamosu` in Kamosu's image. Take it from the image of the
same version as your Kamosu.

**On a machine of its own**, copy it out. The image has no shell, so make a
container from it without starting it, and copy from that:

```sh
docker create --name kamosu-copy battermanz/kamosu:<version>
docker cp kamosu-copy:/app/kamosu ./kamosu
docker rm kamosu-copy
./kamosu runner --help
```

With Podman, the same three commands start with `podman`, and the image is
`docker.io/battermanz/kamosu:podman-<version>`.

**In an image with your agent**, copy it in from Kamosu's image as one stage of
the build. The Runner and the agent then run in one container, which you start
and restart like any other:

```dockerfile
# The version of the Kamosu this Runner asks.
ARG KAMOSU_VERSION
FROM docker.io/battermanz/kamosu:${KAMOSU_VERSION} AS kamosu

FROM docker.io/library/debian:13-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*
# Install your agent, its tools and its instructions here.
COPY --from=kamosu /app/kamosu /usr/local/bin/kamosu
COPY stand-in.py /opt/runner/stand-in.py
RUN useradd --create-home runner
USER runner
ENTRYPOINT ["kamosu", "runner"]
```

```sh
docker build --build-arg KAMOSU_VERSION=<version> -t kamosu-runner .
```

`python3` and `stand-in.py` are for the check in the next step. Take them out
once your own agent is in.

## 3. Check it with a stand-in agent

Before the real agent, check the path from Kamosu to the agent and back with a
stand-in that calls no model. This one makes a recipe titled with the first
line of pasted text, and reports anything else as unreadable. Save it as
`stand-in.py`:

```python
#!/usr/bin/env python3
# A stand-in agent for checking a Runner. It calls no model: it titles a
# recipe with the first line of pasted text, and reads nothing else.
import json, urllib.parse, urllib.request

handed = json.load(open("job.json"))
job = handed["job"]

def call(path, body=None):
    request = urllib.request.Request(
        urllib.parse.urljoin(handed["kamosu"], path),
        data=None if body is None else json.dumps(body).encode(),
        headers={"Authorization": "Bearer " + job["job_key"],
                 "Content-Type": "application/json"},
    )
    return urllib.request.urlopen(request).read().decode()

original = job["original"]
if original and original["kind"] == "pasted_text":
    first_line = call(original["address"]).strip().splitlines()[0]
    call("api/op/create_recipe", {"title": first_line})
    report = {"outcome": "completed",
              "report": {"left_undone": ["The stand-in read the first line and nothing else."]}}
else:
    report = {"outcome": "could_not_be_read",
              "reason": "The stand-in reads pasted text and nothing else."}

with open("report.json", "w") as f:
    json.dump(report, f)
```

Start the Runner with it. The agent command comes after `--`, and the Runner
starts it directly, with no shell. It starts it inside the Job's folder, so
**name the script by its full path**. The parentheses load the key in a shell
of its own, so it is gone from yours when the Runner stops:

```sh
(set -a; . ~/.config/kamosu-runner.env; set +a
 ./kamosu runner --kamosu https://recipes.kamosu.example -- python3 "$PWD/stand-in.py")
```

In the image from step 2, pass the key's file to the container instead:

```sh
docker run -d --name kamosu-runner --restart on-failure:3 \
  --env-file ~/.config/kamosu-runner.env \
  kamosu-runner --kamosu https://recipes.kamosu.example -- python3 /opt/runner/stand-in.py
```

The Runner says `Kamosu Runner <version> asking https://recipes.kamosu.example/
for work.` and nothing more while there is none. Then, in Kamosu:

1. Press the **+** beside the Recipes search box and choose pasted text.
2. Paste two lines, such as `Runner check soup` and `1 onion`, and send it.
3. Within about ten seconds the Runner says `Working Job j_…`, then
   `Reported Job j_… as completed.`
4. The Import opens on its own. A recipe titled **Runner check soup** is in
   your Cookbook, and the Import Report lists what the stand-in left undone.

Delete that recipe afterwards. If any step does not happen, see
[When nothing happens](#when-nothing-happens).

## 4. Start it with your agent

Replace the stand-in with your agent's command, and keep the Runner running the
way you keep anything running on that machine.

The Runner rides out Kamosu being down by itself: it keeps asking, more slowly
each time. It exits only when Kamosu refuses it for good, for a revoked key or
another version, and starting it again cannot mend either. So limit how often
it is restarted, as `on-failure:3` does above. **Never give it `always` or
`unless-stopped`**: a refused Runner exits at once, and those start it again
for ever, each time asking Kamosu with a key it refuses. Podman did so several
times a second when this guide was tested.

```sh
kamosu runner --kamosu https://recipes.kamosu.example -- /opt/runner/read-a-recipe
```

The command is usually a short script of your own that reads `job.json`,
connects the agent to Kamosu's MCP door with the Job Key, starts it with your
instructions, and leaves `report.json` if the agent did not. The two sections
after the next one are its contract.

## Where to run it

Anywhere that reaches Kamosu's address: the same machine, another one at home,
or a server elsewhere. It opens no port, so nothing has to reach it.

- **Where it runs decides where the agent's fetching comes from.** A recipe
  site or a video site sees the Runner's machine asking, not Kamosu's. Some
  sites answer a home connection and refuse a data centre.
- **Give it Kamosu's own address**, the one that answers without a redirect. The
  Runner follows no redirect and refuses an address holding a name or a
  password, because the agent is handed that address. Beside Kamosu in one
  Compose project, `http://kamosu:5266` is that address.
- **Run it as a user of its own.** The agent runs as the Runner's user, so that
  user should not be able to read Kamosu's data folder, your home folder, or
  anything else an agent tricked by a web page should not reach. Never run it
  as root.

The Runner keeps its own key from the agent. The key is in no file of the
Job's folder and not in the agent's environment, and the Runner blanks it in
its own, where another program of the same user could read it. Two things this
does not cover:

- A process the agent starts with `setsid` leaves the Job's process group, so
  it is not stopped with the agent.
- On a machine where `/proc/sys/kernel/yama/ptrace_scope` is `0`, a program can
  read the memory of another of the same user, the Runner's included.

Running the agent as another user than the Runner closes both: start it through
`sudo -u` or `setpriv` in your command, or give the Runner a container of its
own.

## What the agent is handed

For each Job, the Runner makes a folder under the temporary directory
(`$TMPDIR`, or `/tmp`), readable by its user alone. It starts the agent command
there with:

- **The folder as its working directory**, holding one file, `job.json`.
- **The Runner's environment**, less `KAMOSU_RUNNER_KEY`.
- **No input**: stdin is closed. What it prints goes to the Runner's own output.
- **A process group of its own**, so that stopping it stops everything it
  started.

`job.json` holds:

```json
{
  "kamosu": "https://recipes.kamosu.example/",
  "mcp": "https://recipes.kamosu.example/mcp",
  "job": {
    "job_id": "j_…",
    "branch_id": "b_…",
    "link": "https://…",
    "original": null,
    "job_key": "…"
  }
}
```

- `kamosu` is the address the Runner was given, and `mcp` is Kamosu's MCP door
  there.
- `job_key` is the agent's Credential. Send it as `Authorization: Bearer
  <job_key>`, at the MCP door or at the web door (`POST <kamosu>api/op/<name>`).
- `link` is the page to read, or null.
- `original` is what to read when that is not a link, or null. It holds `kind`,
  one of `pasted_text`, `pdf` or `picture`, and `address`, a path such as
  `/api/originals/j_…`. Fetch it with a GET to that path on Kamosu's address,
  with the Job Key.
- `branch_id` is the recipe the Job is for. It is null when the Job reads an
  Original and no recipe exists yet: the agent creates it with `create_recipe`.

Kamosu may add fields to `job` later. The Runner passes them on unread.

## What the agent leaves

Before it exits with status 0, the agent leaves `report.json` in the same
folder:

```json
{
  "outcome": "completed",
  "report": {
    "left_undone": ["The page gives no cooking time."],
    "unsure": ["Step 2 may mean 180 °C or 160 °C."],
    "likely_duplicates": ["b_…"],
    "unused_foods": ["caster sugar"]
  }
}
```

- `outcome` is `completed`; `failed`, when something went wrong on the way; or
  `could_not_be_read`, when the thing holds no recipe the agent can read.
- `reason` is one sentence, which the cook reads in the Import Report. The last
  two outcomes need it, and `completed` takes none.
- `report` is optional, with any outcome, and so is each list in it:
  - `left_undone`: sentences saying what the agent did not do.
  - `unsure`: sentences saying what it wrote and was unsure of.
  - `likely_duplicates`: the `branch_id` of each recipe already in the library
    that may be the same dish. The Import Report links to each.
  - `unused_foods`: the names of Foods its corrections left with nothing
    pointing at them.

  Each list takes up to 100 entries of up to 2,000 characters, none blank.
  Kamosu refuses any other field.

The Runner reports the Job failed, with a reason the cook reads, when the
command exits with another status, leaves no `report.json`, leaves one that is
not a JSON object in a plain file of 4 MB or less, or leaves one Kamosu refuses.

A Job reported `failed` waits to be tried again, keeping what the agent saved.
`could_not_be_read` ends it at once.

## What a Job Key can do

A Job Key acts as the person who asked for the Job, under their Hand, and only
for that Job. It is spent when the Job ends. Whoever writes the agent's
instructions should know its edges, because everything else is refused.

It can:

- **Read the library as that person sees it**: `get_recipe`, `get_thread`,
  `branch_point`, `divergence`, `changed_since`, `search_recipes`, `list_tags`,
  `list_foods` and `get_food`.
- **Fetch the Job's Original**, at the address in `job.json`.
- **Write the Job's one recipe**: `edit_recipe` for a partial change,
  `save_recipe_version` for the whole recipe, and `create_recipe` once, for a
  Job that has no recipe yet.
- **Correct that recipe's Readings** with `set_reading`.
- **Put a Tag the Cookbook already has** on that recipe, with `set_recipe_tag`.
- **Store a Photograph** with `upload_photograph`, or by sending the picture's
  bytes as the whole body of `POST <kamosu>api/photographs`.

It cannot make a Translation, a Related Recipe or a Tag, rename or merge a
Food, delete anything, touch another recipe or reach an account. Any other
recipe answers as one that is not there. At the MCP door, it is shown the
fifteen tools above and no other.

Three more edges:

- **A Job leaves one new Version.** However many times the agent saves, its
  saves shape that one Version, and the History keeps what was there before.
- **If someone else saves the recipe while the agent works**, the agent's next
  save is refused and the Job fails, keeping what the agent had saved.
- **It never makes a Copy.** If the person who asked has left the Cookbook that
  holds the recipe, the write is refused and the Job fails with that reason.

## Writing the agent's instructions

The agent works alone, with nobody to ask. Instructions written for an agent
with a person beside it have to say what to do instead of asking:

- **Leave a thing undone and say so** in `left_undone`, rather than guess.
  Write what you were unsure of in `unsure`.
- **Read the saved recipe back and check it against the source**, each number
  and each unit, then correct it. Nobody approves the recipe before it is
  saved: it is in the person's Cookbook as soon as the agent writes it.
- **Keep the recipe in the language of its source.**
- **Search the library for the same dish**, and name each likely one in
  `likely_duplicates`. Save the recipe anyway.
- **Name the Foods a correction leaves unused** in `unused_foods`. The agent
  cannot delete a Food; an Operator tidies them.
- **Use `could_not_be_read`** when the thing holds no recipe, and say why in
  one sentence.

The Kamosu MCP server describes its own tools, and the rules for saving a
recipe are in its instructions.

## Time, tries and cost

- **Thirty minutes.** An agent command still running after 30 minutes is
  stopped, with everything it started, and the Job is reported failed.
- **Three tries.** A Job that is taken and does not finish three times ends as
  failed, and the Import Report says so.
- **Cancelling.** The person who asked can stop a Job from the recipe's page.
  The Runner hears it within a minute, stops the agent, and asks for the next
  Job. What the agent saved stays.
- **Stopping the Runner** (Ctrl-C or SIGTERM) stops the agent too. Its Job
  waits again once the hold lapses, about two minutes later, and goes to the
  next Runner that asks.
- **Kamosu cannot see what a Job cost.** The agent's model bill, and any limit
  on it, belong to whoever runs the Runner. Kamosu sets no limit on how often
  people send things to the agent.

## When nothing happens

Start with the Runner's own output, then the **Runner** section of the Operator
screen, which says when a Runner last asked for work. A Runner with nothing to
do asks every ten seconds.

- **"Kamosu refused this Runner for its version"**: the Runner and Kamosu are
  of different versions. Take the program from the image of Kamosu's version.
- **"Kamosu does not know this Runner Key"**: the key was revoked, or was made
  on another Kamosu. Make a new one.
- **"Kamosu at … is out of reach"**: the Runner cannot open the address. It
  keeps asking, waiting longer each time, up to every two minutes. Check the
  address from the Runner's machine, with `curl` or a browser.
- **"answered with a redirect"**: the address sends the Runner elsewhere, such
  as `http` to `https`. Give it the address the redirect leads to.
- **"No Runner has asked for work yet"** on the Operator screen: the Runner is
  not running, or not asking this Kamosu. Read its output.
- **"Kamosu's AI agent isn't answering right now"** on a recipe: a Job
  waits, and no Runner has asked for work in the last five minutes. The
  Operator sees a link to the Runner Keys there.
- **The Job fails with "the agent command could not be started"**, or with
  "ended in an error" straight after it starts: the command, or a script it
  names, is not where it was said to be when seen from inside the Job's
  folder. Name both by their full path.
- **The Job fails with "the agent left no report"**: the agent exited without
  writing `report.json` in its working directory. Run the agent command by hand
  in a folder holding a copy of a `job.json` to see what it does.
