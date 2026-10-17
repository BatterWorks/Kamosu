# Third-party notices

Kamosu is licensed under the GNU Affero General Public License v3.0 only (see
[LICENSE](LICENSE)). This file records third-party material that is
**bundled in this repository**, **in the agent image** Kamosu publishes, or
**downloaded at runtime by an Operator**. Kamosu redistributes the first two.

Rust crates and npm packages compiled into the binary keep their own licences,
which their manifests declare. `just audit` checks those dependencies against
the advisory databases.

---

## Bundled in this repository

### Zen Old Mincho

Copyright 2021 The Zen Old Mincho Project Authors
(<https://github.com/googlefonts/zen-oldmincho>). Licensed under the SIL Open
Font License, Version 1.1. The full licence is in
[`assets/fonts/OFL-ZenOldMincho.txt`](assets/fonts/OFL-ZenOldMincho.txt).

### Zen Kaku Gothic New

Copyright 2022 The Zen Kaku Gothic Project Authors
(<https://github.com/googlefonts/zen-kakugothic>). Licensed under the SIL Open
Font License, Version 1.1. The full licence is in
[`assets/fonts/OFL-ZenKakuGothicNew.txt`](assets/fonts/OFL-ZenKakuGothicNew.txt).

**Modification notice.** Both families ship as Latin and Latin-ext subsets, in
woff2 for the browser and as decompressed TrueType for the server, which sets
text on Share Cards and Sheets. No glyph is altered. Neither family declares a
Reserved Font Name, so the subsets keep their names.
[`assets/fonts/README.md`](assets/fonts/README.md) says how they are made.

---

## In the agent image

Every release publishes the agent's image, `battermanz/kamosu-agent`, built
from [`agent-image/`](agent-image/) (#345). Kamosu's own files in it, the
program that takes a Job and the instructions, are under Kamosu's licence. The
rest are other people's programs, as their publishers build them:

| Program | Licence | Where its source is |
| --- | --- | --- |
| [opencode](https://opencode.ai), the agent, at the version `agent-image/Dockerfile` names | MIT | <https://github.com/anomalyco/opencode>, the npm package `@opencode/cli` |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp), its latest release when the image is built, and its nightly build each time the container starts | Unlicense | <https://github.com/yt-dlp/yt-dlp> |

Everything else comes from Alpine Linux, at the release the Dockerfile's
`FROM` lines name, as Alpine builds it. The packages the image asks for, with
the licences Alpine 3.24 gave them on 2026-10-17:

| Package | Licence |
| --- | --- |
| ffmpeg | GPL-2.0-or-later AND LGPL-2.1-or-later |
| poppler-utils | GPL-2.0-or-later |
| python3 | PSF-2.0 |
| deno | MIT |
| curl | curl |
| libstdc++ | GPL-2.0-or-later AND LGPL-2.1-or-later |
| ca-certificates | MPL-2.0 AND MIT |

They pull in the libraries they need, and the base image brings Alpine's own
system: busybox, apk-tools and alpine-baselayout under GPL-2.0-only, and musl
under MIT. The image lists every package it holds, with its version and
licence, in `/lib/apk/db/installed`.

**The source of every Alpine package** is Alpine's: the build recipe in
[aports](https://gitlab.alpinelinux.org/alpine/aports), on the branch of the
release the image is built on, such as `3.24-stable`, and the original source
each recipe names.

---

## Downloaded at runtime

**This is not distributed with Kamosu.** No model weights ship in the
repository, the container image or a Backup (ADR 0029). Meaning Search is off
until an Operator turns it on, and turning it on is where they accept the terms
below. The model is then fetched from Hugging Face onto their own machine.

### EmbeddingGemma 300M

Fetched from `onnx-community/embeddinggemma-300m-ONNX` at a pinned revision.
Governed by the **Gemma Terms of Use**, which is *not* an open-source licence
and carries a prohibited-use policy.

- Terms: <https://ai.google.dev/gemma/terms>
- Prohibited use policy: <https://ai.google.dev/gemma/prohibited_use_policy>

Anyone running Kamosu with Meaning Search on is bound by those terms. The
authoritative repository, revision, terms version and URLs are the constants in
`src/meaning.rs`; if they disagree with this file, the code wins.
