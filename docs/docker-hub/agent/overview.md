# Kamosu's agent

**The agent a [Kamosu](https://hub.docker.com/r/battermanz/kamosu) Runner hands its Jobs to. It reads a recipe Kamosu cannot read itself, such as a photo of a cookbook page, a scanned PDF, a page with no recipe data or a cooking video, and writes it into your library.**

You do not need this image to run Kamosu. Kamosu reads a recipe page's own data, a pasted recipe and a recipe PDF on its own, and without a Runner it reads a page with no recipe data from the page's text. This image is for the rest, reads those better, and needs a model from a provider of your choosing.

- **Source:** https://github.com/BatterWorks/Kamosu/tree/main/agent-image
- **Setup guide:** https://github.com/BatterWorks/Kamosu/blob/main/docs/runner.md

## What is in it

- **[opencode](https://opencode.ai) 2**, the agent. It speaks to several dozen model providers, and to any service with an OpenAI-compatible API. Nothing in the image names a provider or a model: you give both when the container starts.
- **The tools a Job needs:** yt-dlp, which takes its nightly build each time the container starts, ffmpeg, deno for yt-dlp's YouTube support, poppler for a PDF's pages, curl and python3, on Alpine.
- **The fence**, the same image started as `/opt/kamosu-agent/fence`, which keeps the agent off your own network: it reaches the internet and Kamosu, and never the host or the rest of your LAN. It starts as root to set its rules, then keeps no root and no capability.
- **The instructions it follows**, written for an agent reading one recipe alone. You can replace them with your own (below).

It runs as its own user, never as root, in a read-only container that takes one Job and ends. Its restart policy starts it again, empty, for the next one. It never holds the Runner Key or the model provider's key: the Runner, which is Kamosu's own image, holds both. It hands each Job over through a socket in a volume the two share, and relays the agent's model requests through a socket in another, adding the key on the way, so a page that tricks the agent finds no key to send away.

## Tags

- `latest` and `<version>`: images for Docker
- `podman-latest` and `podman-<version>`: images for Podman

**Use the same version as your Kamosu.** A Kamosu refuses a Runner of any other version, and the agent is made for the Runner of its own. A release that did not change the agent gives its new tag to the image already published, so nothing new is downloaded. Every tag is built for amd64 and arm64.

## Quick start

Beside Kamosu in one Compose project, with `<version>` the version of your Kamosu:

```yaml
services:
  kamosu-agent-fence:
    image: docker.io/battermanz/kamosu-agent:<version>
    entrypoint: ["/opt/kamosu-agent/fence"]
    user: "0:0"
    restart: always
    environment:
      KAMOSU_AGENT_FENCE_ALLOW: kamosu:5266
    read_only: true
    cap_drop: [ALL]
    cap_add: [NET_ADMIN, SETUID, SETGID]
    security_opt: [no-new-privileges:true]

  kamosu-agent:
    image: docker.io/battermanz/kamosu-agent:<version>
    restart: always
    network_mode: service:kamosu-agent-fence
    depends_on: [kamosu-agent-fence]
    env_file: ~/.config/kamosu-agent.env
    read_only: true
    tmpfs:
      - /tmp:rw,exec,mode=1777,size=4g
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    volumes:
      - agent-socket:/run/kamosu-agent
      - relay-socket:/run/kamosu-relay

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
      - relay-socket:/run/kamosu-relay

volumes:
  agent-socket:
    driver_opts:
      type: tmpfs
      device: tmpfs
      o: size=1m,uid=1000,gid=1000,mode=0755
  relay-socket:
    driver_opts:
      type: tmpfs
      device: tmpfs
      o: size=1m,uid=65532,gid=65532,mode=0755
```

With Podman, use the `podman-<version>` tags of both images.

The Runner's settings file holds its key, Kamosu's address, the agent's socket, and the model provider's key and address with the relay's socket:

```sh
KAMOSU_RUNNER_KEY=...
KAMOSU_RUNNER_INSTANCE=http://kamosu:5266
KAMOSU_RUNNER_AGENT_SOCKET=/run/kamosu-agent/agent.sock
KAMOSU_RUNNER_MODEL_KEY=...
KAMOSU_RUNNER_MODEL_ADDRESS=https://openrouter.ai/api/v1
KAMOSU_RUNNER_MODEL_SOCKET=/run/kamosu-relay/model.sock
```

`KAMOSU_RUNNER_MODEL_ADDRESS` is the provider's API address: `https://openrouter.ai/api/v1`, `https://api.openai.com/v1`, `https://api.anthropic.com/v1` or `https://generativelanguage.googleapis.com/v1beta`, or the service's own address for one with an OpenAI-compatible API.

An Operator makes the key in Kamosu: **Settings**, then **Administer this instance**, then **Runner** on the **Instance** tab. The agent's settings file holds the rest:

| Variable | What it holds |
| --- | --- |
| `KAMOSU_AGENT_MODEL` | The model, as opencode names it: `provider/model`, such as `openrouter/the-model-id`. |
| `KAMOSU_AGENT_BASE_URL` | Optional. A service opencode does not know that offers an OpenAI-compatible API, the same address as `KAMOSU_RUNNER_MODEL_ADDRESS`. The model there must take pictures. |
| `KAMOSU_AGENT_EFFORT` | Optional. How hard the model thinks, in opencode's words for that model, such as `low`, `high` or `max`. |

An agent given `KAMOSU_RUNNER_KEY`, or started without `KAMOSU_AGENT_MODEL`, says so in its log, waits a minute and exits. An agent still given its provider's key, as `KAMOSU_AGENT_API_KEY` or the provider's own variable, works while the Runner holds none, and says to move it there.

## Your own instructions

To add rules of your own without replacing the instructions, an Operator writes House Notes in Kamosu, in the Runner section of the **Instance** tab. Every Job carries them, and the agent reads them after its own instructions and follows them where the two disagree.

The instructions are one file in the image, `/opt/kamosu-agent/instructions.md`. To change them, copy [the file of your version](https://github.com/BatterWorks/Kamosu/blob/main/agent-image/instructions.md), edit it, and mount your copy over it, read-only. It must be readable by the agent's user, uid 1000:

```yaml
    volumes:
      - agent-socket:/run/kamosu-agent
      - ./my-instructions.md:/opt/kamosu-agent/instructions.md:ro
```

The agent reads the file at the start of every Job. A new version of the image may bring better instructions, which your copy does not follow until you bring the changes into it.

## License

The image is built from Kamosu's repository, AGPL-3.0-only. The programs in it keep their own licences, which [`THIRD_PARTY_NOTICES.md`](https://github.com/BatterWorks/Kamosu/blob/main/THIRD_PARTY_NOTICES.md) names with where each one's source is.
