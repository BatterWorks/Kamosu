# syntax=docker.io/docker/dockerfile:1.7

# The agent's container (#315, #317): opencode, the tools a Job needs and the
# instructions it follows, beside a Runner in a container of its own. The
# Runner is Kamosu's own image started as `kamosu runner`, and hands each Job
# over through a socket in a volume the two share, so the Runner Key is never
# in this container (`docs/runner.md`, "The agent image Kamosu publishes"). The
# model provider's key stays with the Runner too, which relays the agent's
# model requests through a socket in a second volume (#372). The same image,
# started as `fence`, keeps the agent's container off the host's network.
#
# Alpine, as the thinnest base that carries every tool: a distroless image
# has no shell, and the agent's work is running tools through one. One user,
# never root. Nothing here names a provider or a model: the Operator gives
# both when the container starts.
#
# A release publishes this image as battermanz/kamosu-agent, and builds it
# again only when something in this folder changed since the commit the
# published one was built from (#345). Alpine's security fixes reach the image
# only through a build, so a release that finds some since the last one moves
# the date in the rebuild line, the first line of the packages' RUN below, on
# its bump branch (docs/releasing.md, the checklist).

FROM docker.io/library/alpine:3.24 AS tools
ARG TARGETARCH
# Held, as a dependency is: taken at their current release when this image was
# first written, and moved deliberately. yt-dlp is not held here, because it
# updates itself each time the container starts.
# opencode 2 is published on npm as @opencode/cli, one package for each
# platform, and not on its GitHub releases page, which carries 1.x alone.
ARG OPENCODE_VERSION=2.0.30
RUN apk add --no-cache curl
WORKDIR /fetch
# Each download is checked against the hash its publisher gives: npm's
# integrity field for opencode, the release's checksum file for yt-dlp.
RUN case "${TARGETARCH}" in \
        amd64) opencode_arch=x64 ;; \
        arm64) opencode_arch=arm64 ;; \
        *) echo "no build for ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && package="cli-linux-${opencode_arch}-musl" \
    && opencode="https://registry.npmjs.org/@opencode/${package}" \
    && curl -fsSL -o opencode.tgz "${opencode}/-/${package}-${OPENCODE_VERSION}.tgz" \
    && integrity="$(curl -fsSL "${opencode}/${OPENCODE_VERSION}" \
        | grep -o '"integrity":"sha512-[^"]*"' | cut -d- -f2- | tr -d '"')" \
    && echo "$(printf '%s' "${integrity}" | base64 -d | od -An -v -tx1 | tr -d ' \n')  opencode.tgz" \
        | sha512sum -c - \
    && tar -xzf opencode.tgz --strip-components=2 package/bin/opencode \
    && curl -fsSL -O https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp \
    && curl -fsSL -O https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS \
    && grep ' yt-dlp$' SHA2-256SUMS | sha256sum -c - \
    && chmod 755 opencode yt-dlp

# The two build arguments, written to a file the image copies in above its
# labels, as Kamosu's own Dockerfile does and for the same reason (see the foot
# of this file). Nothing is built on this stage, so a changed argument reruns
# one `printf` and no download.
FROM docker.io/library/alpine:3.24 AS stamp
ARG VERSION=""
ARG GIT_SHA=""
RUN printf '%s\n%s\n' "$VERSION" "$GIT_SHA" > /stamp

FROM docker.io/library/alpine:3.24
# What reading needs, whatever the model: ffmpeg cuts a video's frames and
# sound, python3 runs yt-dlp and this container's own command, deno runs
# yt-dlp's YouTube support, poppler turns a PDF's pages into pictures for a
# model that takes no PDF, and curl fetches a page. opencode's musl build
# needs libstdc++. These are held by the Alpine release above, which takes
# security fixes alone: deno is Alpine's own build, since deno publishes none
# for musl. nftables is the fence's, which sets the agent's firewall rules.
#
# The rebuild line is part of this RUN and not a comment, because a build
# reuses a step whose text has not changed: an edited comment would rebuild
# the image with last time's packages. A new date reruns this step, which
# upgrades what the base image holds and installs the rest at Alpine's
# current versions.
RUN : "Rebuilt for Alpine's fixes as of 2026-10-29" \
    && apk upgrade --no-cache \
    && apk add --no-cache \
        ca-certificates curl deno ffmpeg libstdc++ nftables poppler-utils python3

RUN adduser -D -u 1000 -s /bin/sh agent

COPY --from=tools /fetch/opencode /usr/local/bin/opencode
COPY --from=tools /fetch/yt-dlp /opt/yt-dlp/yt-dlp
COPY --chmod=755 agent fence upload-photograph /opt/kamosu-agent/
RUN ln -s /opt/kamosu-agent/upload-photograph /usr/local/bin/upload-photograph
COPY --chmod=644 instructions.md /opt/kamosu-agent/
# Where the socket goes, the agent's own, so that a fresh named volume
# mounted here starts out writable by it.
RUN install -d -o agent -g agent -m 755 /run/kamosu-agent

USER agent
WORKDIR /tmp
ENTRYPOINT ["/opt/kamosu-agent/agent"]
# The release that built the image and the commit it was built from (#345),
# empty unless the build passes them: `--build-arg VERSION=0.3.0 --build-arg
# GIT_SHA=<commit>`. An image reused for a later release keeps both, so the
# revision is what says which agent-image/ the image holds.
# Declared last, so a changed argument throws away only the copy and the
# labels. The copy is what makes Podman write the labels again: Podman 4.9
# reuses a LABEL step by its text, the argument unread, and compares a copied
# file by what it holds. tests/image_labels.rs holds this file to it.
ARG VERSION=""
ARG GIT_SHA=""
COPY --from=stamp /stamp /opt/kamosu-agent/stamp
LABEL org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${GIT_SHA}"
