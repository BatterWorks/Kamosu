//! The release checklist's library check (`docs/releasing.md`, ADR 0047): a
//! library left by the previous release opens under this build.
//!
//! `tests/behaviour.rs` carries a database it made itself forward from an
//! earlier schema. This does the same to a copy of a real one, which is where
//! a migration meets rows no test thought to write. The folder is one a Backup
//! unzips to, holding `kamosu.db`. Only a copy is opened, so the folder is
//! left as it was.
//!
//! It is `#[ignore]`d and kept out of `just test`, since the library is
//! somebody's own. `just release-library <folder>` runs it.

use kamosu::db::{DATABASE_FILE, Db};

#[test]
#[ignore = "needs KAMOSU_RELEASE_LIBRARY, the folder of a library to carry forward"]
fn a_library_left_by_the_previous_release_opens_under_this_one() {
    let source = std::env::var("KAMOSU_RELEASE_LIBRARY")
        .expect("KAMOSU_RELEASE_LIBRARY names the folder holding kamosu.db");
    let source = std::path::Path::new(&source);
    assert!(
        source.join(DATABASE_FILE).is_file(),
        "{} holds no {DATABASE_FILE}",
        source.display()
    );

    // The database and, where a running instance left them, its two
    // companions. The photographs play no part in a migration.
    let copy = tempfile::tempdir().unwrap();
    for suffix in ["", "-wal", "-shm"] {
        let name = format!("{DATABASE_FILE}{suffix}");
        if source.join(&name).is_file() {
            std::fs::copy(source.join(&name), copy.path().join(&name)).unwrap();
        }
    }

    let db = Db::open(copy.path()).expect("every migration runs on the copy");
    db.with_conn(|conn| {
        let (total, wrong): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COUNT(*) FILTER (WHERE id <> version_fingerprint(content)) \
                 FROM versions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        println!("{total} Versions carried forward, {wrong} under the wrong id");
        assert_eq!(
            wrong, 0,
            "every Version's id is the fingerprint of its content"
        );
        Ok(())
    })
    .unwrap();
}
