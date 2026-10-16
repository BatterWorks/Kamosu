# syntax=docker.io/docker/dockerfile:1.7

# The Runner beside an agent (#315): `kamosu runner`, opencode, the tools a
# Job needs and the instructions it follows, in one image of their own.
# Kamosu's image stays one program (ADR 0028); this one sits beside it.
#
# Nothing here names a provider or a model. The Operator gives both, with the
# provider's key and the effort, when the container starts (`docs/runner.md`,
# "The image Kamosu's repository builds").
#
# Two users, and neither is root. `runner` runs `kamosu runner` and holds the
# Runner Key. `agent` runs opencode and everything it starts, through one
# sudo rule, and cannot read anything of the Runner's.

# The Kamosu image to take `kamosu runner` from. It must be the version of the
# Kamosu this Runner asks, which refuses any other. `just runner-image` passes
# the image `just podman-build` makes from this repository.
ARG KAMOSU_IMAGE

FROM ${KAMOSU_IMAGE} AS kamosu

# The tools are fetched in a stage of their own, so the image carries no unzip
# and no download left behind.
FROM docker.io/library/debian:13-slim AS tools
ARG TARGETARCH
# Held, as a dependency is: taken at their current release when this image was
# first written, and moved deliberately. yt-dlp is not held here, because it
# updates itself each time the container starts.
# opencode 2 is published on npm as @opencode/cli, one package for each
# platform, and not on its GitHub releases page, which carries 1.x alone.
ARG OPENCODE_VERSION=2.0.30
ARG DENO_VERSION=2.9.7
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl unzip \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /fetch
# Each download is checked against the hash its publisher gives: npm's
# integrity field for opencode, the release's checksum files for deno and
# yt-dlp.
RUN case "${TARGETARCH}" in \
        amd64) arch=x86_64; opencode_arch=x64 ;; \
        arm64) arch=aarch64; opencode_arch=arm64 ;; \
        *) echo "no build for ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && opencode="https://registry.npmjs.org/@opencode/cli-linux-${opencode_arch}" \
    && curl -fsSL -o opencode.tgz \
        "${opencode}/-/cli-linux-${opencode_arch}-${OPENCODE_VERSION}.tgz" \
    && integrity="$(curl -fsSL "${opencode}/${OPENCODE_VERSION}" \
        | grep -o '"integrity":"sha512-[^"]*"' | cut -d- -f2- | tr -d '"')" \
    && echo "$(printf '%s' "${integrity}" | base64 -d | od -An -v -tx1 | tr -d ' \n')  opencode.tgz" \
        | sha512sum -c - \
    && tar -xzf opencode.tgz --strip-components=2 package/bin/opencode \
    && deno="deno-${arch}-unknown-linux-gnu.zip" \
    && curl -fsSL -O "https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/${deno}" \
    && curl -fsSL -O "https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/${deno}.sha256sum" \
    && sha256sum -c "${deno}.sha256sum" \
    && unzip -q "${deno}" deno \
    && curl -fsSL -O https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp \
    && curl -fsSL -O https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS \
    && grep ' yt-dlp$' SHA2-256SUMS | sha256sum -c - \
    && chmod 755 opencode deno yt-dlp

FROM docker.io/library/debian:13-slim
# What reading needs, whatever the model: ffmpeg cuts a video's frames and
# sound, python3 runs yt-dlp, poppler turns a PDF's pages into pictures for a
# model that takes no PDF, and curl fetches an Original. sudo hands each Job
# to the agent's user.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl ffmpeg poppler-utils python3 sudo \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin runner \
    && useradd --create-home --shell /bin/bash agent

COPY --from=kamosu /app/kamosu /usr/local/bin/kamosu
COPY --from=tools /fetch/opencode /fetch/deno /usr/local/bin/
# yt-dlp belongs to the Runner's user, who updates it as the container starts.
# The agent may run it and not change it.
RUN install -d -o runner -g runner /opt/yt-dlp
COPY --from=tools --chown=runner:runner /fetch/yt-dlp /opt/yt-dlp/yt-dlp
RUN ln -s /opt/yt-dlp/yt-dlp /usr/local/bin/yt-dlp

COPY --chmod=755 start read-a-recipe agent-job upload-photograph /opt/kamosu-runner/
RUN ln -s /opt/kamosu-runner/upload-photograph /usr/local/bin/upload-photograph
COPY --chmod=644 instructions.md /opt/kamosu-runner/
COPY --chmod=440 sudoers /etc/sudoers.d/kamosu-runner
RUN visudo -cf /etc/sudoers.d/kamosu-runner

USER runner
WORKDIR /home/runner
ENTRYPOINT ["/opt/kamosu-runner/start"]
