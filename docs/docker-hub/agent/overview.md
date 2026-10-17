# Kamosu's agent

**The agent a [Kamosu](https://hub.docker.com/r/battermanz/kamosu) Runner hands its Jobs to. It reads a recipe Kamosu cannot read itself, such as a photo of a cookbook page, a scanned PDF, a page with no recipe data or a cooking video, and writes it into your library.**

You do not need this image to run Kamosu. Kamosu reads a recipe page's own data, a pasted recipe and a recipe PDF on its own. This image is for the rest, and it needs a model from a provider of your choosing.

- **Source:** https://github.com/BatterWorks/Kamosu/tree/main/agent-image
- **Setup guide:** https://github.com/BatterWorks/Kamosu/blob/main/docs/runner.md

## What is in it

- **[opencode](https://opencode.ai) 2**, the agent. It speaks to several dozen model providers, and to any service with an OpenAI-compatible API. Nothing in the image names a provider or a model: you give both when the container starts.
- **The tools a Job needs:** yt-dlp, which takes its nightly build each time the container starts, ffmpeg, deno for yt-dlp's YouTube support, poppler for a PDF's pages, curl and python3, on Alpine.
- **The instructions it follows**, written for an agent reading one recipe alone. You can replace them with your own (below).

It runs as its own user, never as root, in a read-only container that takes one Job and ends. Its restart policy starts it again, empty, for the next one. It never holds the Runner Key: the Runner, which is Kamosu's own image, holds it and hands each Job over through a socket in a volume the two share.

## Tags

- `latest` and `<version>`: images for Docker
- `podman-latest` and `podman-<version>`: images for Podman

**Use the same version as your Kamosu.** A Kamosu refuses a Runner of any other version, and the agent is made for the Runner of its own. A release that did not change the agent gives its new tag to the image already published, so nothing new is downloaded. Every tag is built for amd64 and arm64.

## Quick start

Beside Kamosu in one Compose project, with `<version>` the version of your Kamosu:

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

With Podman, use the `podman-<version>` tags of both images.

The Runner's settings file holds its key, Kamosu's address and the agent's socket:

```sh
KAMOSU_RUNNER_KEY=...
KAMOSU_RUNNER_INSTANCE=http://kamosu:5266
KAMOSU_RUNNER_AGENT_SOCKET=/run/kamosu-agent/agent.sock
```

An Operator makes the key in Kamosu: **Settings**, then **Administer this instance**, then **Runner** on the **Instance** tab. The agent's settings file holds the rest:

| Variable | What it holds |
| --- | --- |
| `KAMOSU_AGENT_MODEL` | The model, as opencode names it: `provider/model`, such as `openrouter/the-model-id`. |
| `KAMOSU_AGENT_API_KEY` | The key for that model's provider, whichever it is. A provider's own variable, such as `OPENAI_API_KEY`, works too. |
| `KAMOSU_AGENT_BASE_URL` | Optional. A service opencode does not know that offers an OpenAI-compatible API. The model there must take pictures. |
| `KAMOSU_AGENT_EFFORT` | Optional. How hard the model thinks, in opencode's words for that model, such as `low`, `high` or `max`. |

An agent given `KAMOSU_RUNNER_KEY`, or started without `KAMOSU_AGENT_MODEL`, says so in its log, waits a minute and exits.

## Your own instructions

The instructions are one file in the image, `/opt/kamosu-agent/instructions.md`. To change them, copy [the file of your version](https://github.com/BatterWorks/Kamosu/blob/main/agent-image/instructions.md), edit it, and mount your copy over it, read-only. It must be readable by the agent's user, uid 1000:

```yaml
    volumes:
      - agent-socket:/run/kamosu-agent
      - ./my-instructions.md:/opt/kamosu-agent/instructions.md:ro
```

The agent reads the file at the start of every Job. A new version of the image may bring better instructions, which your copy does not follow until you bring the changes into it.

## License

The image is built from Kamosu's repository, AGPL-3.0-only. The programs in it keep their own licences, which [`THIRD_PARTY_NOTICES.md`](https://github.com/BatterWorks/Kamosu/blob/main/THIRD_PARTY_NOTICES.md) names with where each one's source is.
