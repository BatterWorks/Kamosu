//! One guard, run before the crate compiles: a release build must carry a built
//! interface.
//!
//! Kamosu is one executable (ADR 0028), so a release binary without `ui/build`
//! is not a smaller Kamosu — it is a Kamosu with no screens, and it would only
//! be discovered by whoever opened the port. `rust-embed` is happy to embed
//! nothing, so the loudness has to be added here.
//!
//! Debug builds are exempt on purpose: development runs vite on 5174 beside the
//! binary on 5266, and `cargo run` must not require a frontend build first.

use std::path::Path;

fn main() {
    // `rust-embed` copies `ui/build` into the binary only where debug
    // assertions are off, as in a release build; anywhere else it reads the
    // folder from disk while running. Only a build that copies it compiles
    // again when it changes. `just test` rebuilds `ui/build` on every run, so
    // watching it in a debug build recompiled Kamosu and linked every test
    // program again each time, with nothing changed (#615).
    if std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some() {
        println!("cargo:rerun-if-changed=build.rs");
    } else {
        println!("cargo:rerun-if-changed=ui/build");
    }

    let built = Path::new("ui/build");
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    if release && !built.join("index.html").exists() {
        panic!(
            "ui/build/index.html is missing, so this release binary would ship no \
             interface.\n       Build it first: `just ui-build` (or `npm run build` in ui/)."
        );
    }

    // A fresh clone has never run the frontend build. An empty folder is honest
    // in a debug build, where the app is served by vite on 5174, and the check
    // above has already ruled it out for a release. `src/interface.rs` also
    // allows the folder to be missing, since a debug build no longer runs this
    // again when it is deleted.
    std::fs::create_dir_all(built).expect("ui/build must be creatable");
}
