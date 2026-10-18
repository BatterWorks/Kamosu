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

**Kamosu's own image holds the Runner and nothing else of the agent.** The
Runner is the same `kamosu` program as the server, started as `kamosu runner`.
Beside it, an agent needs three things:

- **The agent**: Claude Code, Codex, or any program that can follow
  instructions and call Kamosu over MCP or HTTP.
- **Its tools**: whatever it needs to read what it is handed. A video needs a
  downloader and something to cut frames; a picture needs a model that reads
  pictures.
- **Its instructions**: what to do with a Job, written for an agent working
  alone. See [Writing the agent's instructions](#writing-the-agents-instructions).

You can supply all three yourself, or take the agent's image Kamosu
publishes, `battermanz/kamosu-agent`, which holds opencode, the tools and a
set of instructions you can replace: see
[The agent image Kamosu publishes](#the-agent-image-kamosu-publishes).
Kamosu's own image holds one program and no shell, so the agent cannot live in
it.

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
  If it reaches Kamosu over MCP, its MCP client must speak revision
  2026-07-28, the only one Kamosu's MCP door speaks: stateless, with no
  `initialize` handshake. A client that opens with `initialize` is refused, and
  the agent is left without Kamosu's tools. opencode 1.x is one such client;
  opencode 2 speaks it when the server is set to `"protocol": "2026-07-28"`.

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

The same file holds Kamosu's address, so the commands below name only the
agent:

```sh
printf 'KAMOSU_RUNNER_INSTANCE=%s\n' https://recipes.kamosu.example >> ~/.config/kamosu-runner.env
```

`--kamosu <address>` on the command line does the same, and wins over the
file.

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
**name the script by its full path**. The parentheses load the settings in a
shell of their own, so the key is gone from yours when the Runner stops:

```sh
(set -a; . ~/.config/kamosu-runner.env; set +a
 ./kamosu runner -- python3 "$PWD/stand-in.py")
```

In the image from step 2, pass the settings file to the container instead:

```sh
docker run -d --name kamosu-runner --restart on-failure:3 \
  --env-file ~/.config/kamosu-runner.env \
  kamosu-runner -- python3 /opt/runner/stand-in.py
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
kamosu runner -- /opt/runner/read-a-recipe
```

The command is usually a short script of your own that reads `job.json`,
connects the agent to Kamosu's MCP door with the Job Key, starts it with your
instructions, and leaves `report.json` if the agent did not. Its contract is
[What the agent is handed](#what-the-agent-is-handed) and
[What the agent leaves](#what-the-agent-leaves).

## The agent image Kamosu publishes

Every release of Kamosu publishes an agent's image that does steps 2 to 4 for
you: an agent, its tools and its instructions, ready to start beside a Runner.
It is `docker.io/battermanz/kamosu-agent`, built from `agent-image/` in
Kamosu's repository, with the same tags as Kamosu's own image: `<version>` and
`latest` for Docker, `podman-<version>` and `podman-latest` for Podman, each
for amd64 and arm64. **Take the tag of the version your Kamosu runs.** A
release that did not change the agent gives its new tag to the image already
published, so a new version of Kamosu may bring nothing new to download here.

- **The agent is [opencode](https://opencode.ai) 2**, which speaks to any of
  several dozen model providers, or to any that offers an OpenAI-compatible
  API. Nothing in the image names a provider or a model: you give both when
  the container starts.
- **The tools**: yt-dlp, which takes its nightly build each time the container
  starts, ffmpeg, deno for yt-dlp's YouTube support, curl, python3, and
  poppler for turning a PDF's pages into pictures, on Alpine.
- **The instructions** are `agent-image/instructions.md`, in the image as
  `/opt/kamosu-agent/instructions.md`: one recipe, read alone, for each kind
  of Job. To add rules of your own, write [House Notes](#house-notes) in Kamosu.
  To change the instructions themselves, mount your own copy over that file,
  as [Your own instructions](#your-own-instructions) shows.

**The Runner and the agent run in two containers.** The Runner is Kamosu's
own image, which holds one program, no shell and no root, started as `kamosu
runner`. It holds the Runner Key. The agent's container never sees that key.
The Runner hands it each Job through a socket in a volume the two share, as
[An agent in a container of its own](#an-agent-in-a-container-of-its-own)
describes. No process in either container runs as root, and neither container
is given the container engine's socket or any added capability.

**Each Job starts in a fresh container.** The agent's image is read-only, with
`/tmp` in memory. The container takes one Job and ends with it, and its
restart policy starts it again, empty, for the next. Nothing one Job leaves
behind reaches the next.

To change more than the instructions, such as a tool, build the image
yourself from the repository of your version, and use the name you gave it in
place of the published one below:

```sh
git clone https://github.com/BatterWorks/Kamosu && cd Kamosu && git checkout v<version>
docker build -t kamosu-agent agent-image
```

With Podman, add `--format docker`.

Each container takes its settings from an environment file of its own, kept
as the Runner Key's file above is. The Runner's holds three:

| Variable | What it holds |
|---|---|
| `KAMOSU_RUNNER_KEY` | The Runner Key from step 1. The Runner reads it from here and nowhere else. |
| `KAMOSU_RUNNER_INSTANCE` | Kamosu's address. Beside Kamosu in one Compose project, `http://kamosu:5266`. `--kamosu` does the same, and wins when both are given. |
| `KAMOSU_RUNNER_AGENT_SOCKET` | The socket the agent listens on, `/run/kamosu-agent/agent.sock` in the Compose file below. `--agent-socket` does the same, and wins when both are given. Leave it out, or empty, when an agent command follows `--`. |

For the Compose file below, the Runner's settings file is this, with Kamosu's
address beside it in place of the one step 1 wrote:

```sh
KAMOSU_RUNNER_KEY=...
KAMOSU_RUNNER_INSTANCE=http://kamosu:5266
KAMOSU_RUNNER_AGENT_SOCKET=/run/kamosu-agent/agent.sock
```

The agent's holds the rest:

| Variable | What it holds |
|---|---|
| `KAMOSU_AGENT_MODEL` | The model, as opencode names it: `provider/model`. The command below lists them. With `KAMOSU_AGENT_BASE_URL`, the provider part is a name of your choosing and the model part is the model's id at that service. |
| `KAMOSU_AGENT_API_KEY` | The key for that model's provider, whichever it is. |
| `KAMOSU_AGENT_BASE_URL` | Optional. The address of a service opencode does not know that offers an OpenAI-compatible API, such as `https://llm.kamosu.example/v1`. The model is then told it takes pictures, so make it one that does: a photo of a page and a scanned PDF reach it as pictures. |
| `KAMOSU_AGENT_EFFORT` | Optional. How hard the model thinks before it answers, in opencode's words for that model, such as `low`, `high` or `max`. A model reached through `KAMOSU_AGENT_BASE_URL` takes `low`, `medium` or `high`. Without it, the model's own default. |

An agent's container given `KAMOSU_RUNNER_KEY` refuses to work.

A provider's own variable still works in place of `KAMOSU_AGENT_API_KEY`,
under the name opencode reads for it, such as `OPENAI_API_KEY`,
`OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY` or `GOOGLE_GENERATIVE_AI_API_KEY`.
When both are set, `KAMOSU_AGENT_API_KEY` is the one used. A provider that
needs more than a key, such as Amazon Bedrock, Google Vertex or Azure, takes
its own variables, which opencode's provider documentation names.

An agent's settings file for a provider opencode knows:

```sh
KAMOSU_AGENT_MODEL=openrouter/the-model-id
KAMOSU_AGENT_API_KEY=...
```

And for a service it does not, where the provider part, `my-service` here, is
any name:

```sh
KAMOSU_AGENT_MODEL=my-service/the-model-id
KAMOSU_AGENT_API_KEY=...
KAMOSU_AGENT_BASE_URL=https://llm.kamosu.example/v1
```

The key stays in the settings file and the container's environment. opencode
reads it from there for each Job, and no file the agent writes holds it.

To see the models a key opens, ask opencode in the agent's image. It lists
the providers whose own variable is set, so hand it the key under that name,
`OPENROUTER_API_KEY` here:

```sh
docker run --rm --env-file ~/.config/kamosu-agent.env --entrypoint sh \
  docker.io/battermanz/kamosu-agent:<version> -c 'OPENROUTER_API_KEY="${KAMOSU_AGENT_API_KEY:-$OPENROUTER_API_KEY}" opencode models --standalone'
```

A service reached through `KAMOSU_AGENT_BASE_URL` is not in that list. Its
own documentation names its models.

Beside Kamosu in one Compose project, the two containers are:

```yaml
services:
  kamosu-agent:
    image: docker.io/battermanz/kamosu-agent:<version>
    restart: always
    env_file: ~/.config/kamosu-agent.env
    read_only: true
    tmpfs:
      - /tmp:rw,exec,mode=1777,size=4g
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    volumes:
      - agent-socket:/run/kamosu-agent

  kamosu-runner:
    image: docker.io/battermanz/kamosu:<version>
    command: [runner]
    restart: on-failure:3
    env_file: ~/.config/kamosu-runner.env
    read_only: true
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    healthcheck:
      disable: true
    volumes:
      - agent-socket:/run/kamosu-agent

volumes:
  agent-socket:
    driver_opts:
      type: tmpfs
      device: tmpfs
      o: size=1m,uid=1000,gid=1000,mode=0755
```

- **Both images are of the version your Kamosu runs.** The Runner's is Kamosu's
  own, since a Kamosu refuses a Runner of any other, and the agent is made for
  the Runner of its own version. With Podman, use the `podman-<version>` tag of
  each. The Runner's health check asks for a Kamosu server, which a Runner is
  not, so it is turned off.
- **The socket's volume is in memory and belongs to the agent's user**, uid
  1000, which the Runner's user may call but not write in. The agent clears it
  each time it starts, before anything else, so nothing a Job leaves there
  reaches the next Job or keeps the Runner from calling.
- **`/tmp` holds a whole Job in memory**, a video's download and frames
  included, so give it room. 4 GB is a ceiling, not a reservation. `exec`
  lets the agent run the yt-dlp it updates there.
- **Restart the agent always, the Runner a few times.** The agent's container
  ends after every Job by design. A Runner that ends three times running has
  something to say in its log.

With Podman and no Compose, the same settings are `podman run` flags, as
`just runner-start` in Kamosu's repository writes them: `--read-only`,
`--tmpfs`, `--cap-drop all`, `--security-opt no-new-privileges`,
`--no-healthcheck` for the Runner, and `-v` with one named volume for both,
made by `podman volume create` with the options above.
Rootless Podman and Docker need no flag beyond these.

An agent's container started without `KAMOSU_AGENT_MODEL`, or given
`KAMOSU_RUNNER_KEY`, says so, waits a minute and exits, and its restart policy
starts it again until you mend its settings. Check it
with pasted text as in step 3, and read what the two say with `docker logs`.
In Kamosu's repository, `just agent-image` builds both images from the working
copy, and `just runner-start` starts them beside the development instance.

### Your own instructions

The agent reads `/opt/kamosu-agent/instructions.md` at the start of every Job.
To change what it does, copy `agent-image/instructions.md` from the repository
at your version, edit it, and mount your copy over that file, read-only. In the
Compose file above, the agent's volumes become:

```yaml
    volumes:
      - agent-socket:/run/kamosu-agent
      - ./my-instructions.md:/opt/kamosu-agent/instructions.md:ro
```

With `podman run`, the same is
`-v ./my-instructions.md:/opt/kamosu-agent/instructions.md:ro`. The file must be
readable by the agent's user, uid 1000. A new version of the image may bring
better instructions, which your copy does not follow until you bring the
changes into it. When you upgrade, compare `agent-image/instructions.md`
between the two versions.

### House Notes

For a change of one rule, such as "grams, never cups" or "put the Tag
Sweet on every dessert", there is no file to mount. An Operator writes **House
Notes** on the Operator screen, in the Runner section, or with
`set_house_notes` at either Door. Every Job carries them as `house_notes`, and
the published image adds them to the end of its instructions, under a heading
saying they are the Operator's and win where the two disagree. They sit on top
of a file mounted as above just the same, and a new version of the image
brings its improved instructions with your notes still on top.

An agent of your own follows them only if it reads `house_notes` from
`job.json` and gives them to its model. Only an Operator can read or write
them, and a Job Key reaches neither.

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

Putting the agent in a container of its own closes both, as
[the agent image Kamosu publishes](#the-agent-image-kamosu-publishes)
does. Running it as another user than the Runner, through `sudo -u` in your
command, closes them too, inside one machine. `sudo --preserve-env` carries the
Runner's `PWD` across, which names the Job's folder the agent cannot open; set
`PWD` to the agent's own folder before starting it, because opencode takes its
project from `PWD` and fails without one it can read.

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
    "job_key": "…",
    "house_notes": null
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
- `house_notes` is what the instance's Operator added to the agent's
  instructions, as it stands when the Runner took the Job, or null when there
  is none. The agent reads it after its own instructions, as the Operator's,
  and follows it where the two disagree. See [House Notes](#house-notes).

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

## An agent in a container of its own

Given `--agent-socket <path>`, or `KAMOSU_RUNNER_AGENT_SOCKET`, in place of a
command, the Runner hands each Job to an agent listening on that Unix socket,
which the image above does. That puts the Runner Key in another container
altogether. The agent listens and
the Runner calls it, one call for each Job:

1. The agent greets the Runner with one line: `{"kamosu_agent":1}`. The Runner
   asks Kamosu for work only while an agent waits on a call, so no Job is
   taken that no agent can work. It calls again every second while none
   answers, and says why.
2. The Runner sends one line: what `job.json` holds.
3. The agent sends what `report.json` holds, then hangs up. The Runner reads
   it as it reads the file, and fails the Job the same way when it is missing,
   not a JSON object or larger than 4 MB.

The Runner hanging up before the report is how it stops the agent: past the
time limit, when the Job is cancelled, and when the Runner itself is stopped.

An agent should take one call in its life and end with it, so that each Job
starts afresh. After a Job, the Runner waits two seconds before calling
again, so that the container which worked it has ended with everything the
Job started.

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
- **Read the page you were given, or a copy of it.** A page that refuses
  the agent can often be read from the Wayback Machine's copy of the same
  address. Another site's recipe for the same dish is another cook's recipe,
  with its own amounts: the run that measured this guide saw one read in a
  refused page's place, with another oven temperature and yield.
- **Say what one recipe means.** A Job writes one recipe. Say what to do
  with a source holding several (take the first and name the others), and
  with a recipe that calls for another of its own ("pastry, see below").
  Left to itself, an agent tested for this guide merged a page of three
  dressings into one recipe.
- **Say what the title is.** A recipe Kamosu lands bare carries the page's
  title, and a page's headline is often not the recipe's name. Tell the
  agent to use the name over the recipe itself.
- **Say who corrects a source.** An agent that is told the source decides
  every number will undo a cook's correction when it reads a recipe again,
  and one told to keep that correction began correcting sources on its own,
  at a first reading. Tell it
  to copy a plain mistake and flag it, to take the source's own correction
  (an author's reply in the comments), and to keep a person's.
- **Explain Readings and Foods.** Kamosu reads each line as it is saved,
  before the agent checks it, and a misread line makes a Food of the wrong
  words. Tell the agent to correct each Reading with a unit Kamosu converts
  (`cup`, not an old card's "c"), and to look up and name the Foods its
  corrections leave unused.
- **Send it to the captions.** A cooking video often names each ingredient
  on screen and says it nowhere else. Contact sheets small enough to see a
  whole video shrink that text past reading, so give the agent a way to
  read it at full size. It costs time: the video this was learnt on took
  six minutes and nine cents with its captions read, against four and a
  half minutes and three cents without.

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
  people send things to the agent. The agent image Kamosu publishes says
  in its log, after each Job, how long the agent worked and what opencode
  counts the Job cost at the provider's list prices:

  ```text
  Job j_… took 52 s and cost $0.0112: 22 tokens in, 5139 out, …
  Job j_… sent 14 requests, the largest 61204 tokens; 0 went past 100000, …
  ```

  The second line is there because an agent sends its whole conversation
  again at each step, and some providers charge more for a request past
  100,000 tokens, so a long Job pays more for each step after it crosses.

  Measured on one small model at medium effort, most Jobs took from 15
  seconds to 2 minutes and cost under three cents. A picture the agent went
  over again and again cost seven cents, and a video whose recipe had to be
  read off its frames took up to seven minutes and ten cents. At high effort
  that video took fourteen minutes and 36 cents.
  [The research note behind ADR 0048](research/ai-recipe-import-in-other-apps.md#part-6-the-agent-measured-on-the-development-instance)
  has the figures for each kind of Job, and
  [its Part 7](research/ai-recipe-import-in-other-apps.md#part-7-the-agent-on-sources-that-go-wrong)
  those for sources that go wrong.

  Measured across six settings, Claude Haiku 5.5 at high effort read as well
  as Claude Sonnet 5.5 and Claude Opus 5.5 for a fifteenth of Sonnet's price:
  about two cents and a little over a minute a Job. Higher efforts cost more
  for little better.
  [Part 8](research/ai-recipe-import-in-other-apps.md#part-8-the-agent-on-six-model-and-effort-settings)
  has each setting's figures. Nothing in Kamosu depends on that choice.

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
