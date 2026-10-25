//! The Paprika reader against a whole real library (#414): vanilla-cookbook's
//! Paprika export of 746 recipes, sent the way a browser sends it — staged at
//! `POST /api/uploads`, then `import_from_another_app` naming it — and landed
//! in one Job.
//!
//! The file lives at `samples/paprika/` and is gitignored, at 22 MB;
//! `tests/fixtures/paprika/README.md` says where it comes from. Run
//! deliberately: `cargo test --test paprika_corpus -- --ignored`.

mod support;

use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn corpus_path() -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/paprika/vanilla-cookbook.paprikarecipes");
    path.exists().then_some(path)
}

fn wait(app: &support::TestApp, key: &str, job_id: &str) -> Value {
    let started = Instant::now();
    let body = json!({ "job_id": job_id }).to_string();
    loop {
        let (_, answer) = app.post_op("get_job", Some(key), &body);
        let job = answer["result"].clone();
        if ["completed", "failed", "cancelled"].contains(&job["status"].as_str().unwrap_or("")) {
            return job;
        }
        assert!(
            started.elapsed() < Duration::from_secs(900),
            "the import is still running after 15 minutes: {job}"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn import(app: &support::TestApp, key: &str, file: &[u8]) -> Value {
    let (status, staged) = app.post_bytes("/api/uploads", Some(key), "application/zip", file);
    assert_eq!(status, 200, "{staged}");
    let (status, asked) = app.post_op(
        "import_from_another_app",
        Some(key),
        &json!({ "upload_id": staged["result"]["upload_id"] }).to_string(),
    );
    assert_eq!(status, 200, "{asked}");
    let job = wait(app, key, asked["result"]["job_id"].as_str().unwrap());
    assert_eq!(job["status"], json!("completed"), "{job}");
    job["result"].clone()
}

/// How many recipes the Report says left `what` behind, and how many in all.
fn left_out(report: &Value, what: &str) -> (usize, u64) {
    let rows: Vec<&Value> = report["left_out"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["what"] == what)
        .collect();
    let count = rows.iter().map(|row| row["count"].as_u64().unwrap()).sum();
    (rows.len(), count)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs the gitignored samples/paprika export on disk"]
async fn a_whole_paprika_library_arrives_in_one_job() {
    let Some(path) = corpus_path() else {
        eprintln!("skipping: no samples/paprika/vanilla-cookbook.paprikarecipes on this machine");
        return;
    };
    let app = support::spawn_app();
    let person = app.core.create_person("Stéphane").expect("person");
    let key = app
        .core
        .mint_access_key(&person, "corpus", false)
        .unwrap()
        .secret;
    let file = std::fs::read(&path).expect("read the export");
    let report = import(&app, &key, &file);

    assert_eq!(report["source_kind"], json!("paprika"));
    let arrived = report["arrived"].as_array().unwrap();
    assert_eq!(arrived.len(), 746, "{}", report["unreadable"]);
    assert_eq!(report["unreadable"], json!([]));
    assert!(arrived.iter().all(|row| row["status"] == json!("created")));

    // Every one of the 670 photos inside the file arrives, and nothing is
    // downloaded: every recipe with a web address for its picture holds its
    // own photo too.
    let photos: Vec<&str> = arrived
        .iter()
        .filter_map(|row| row["main_photo"].as_str())
        .collect();
    assert_eq!(photos.len(), 670);
    // Three pictures are each the photo of two recipes, and are kept once.
    assert_eq!(photos.iter().collect::<HashSet<_>>().len(), 667);
    assert_eq!(left_out(&report, "photos_not_downloaded"), (0, 0));
    assert_eq!(left_out(&report, "unreadable_photos"), (0, 0));

    // What stayed in Paprika, as the file holds it.
    assert_eq!(left_out(&report, "extra_photos"), (12, 13));
    assert_eq!(left_out(&report, "ratings"), (25, 25));
    assert_eq!(left_out(&report, "favourites"), (0, 0));
    // The free-text nutrition is written into the Note, so none stays (#434).
    assert_eq!(left_out(&report, "nutrition"), (0, 0));

    // What no field holds is in the Note, a paragraph of its own (#434).
    let mut paragraphs: std::collections::BTreeMap<&str, usize> = Default::default();
    let leads = [
        "Total time:",
        "Prep time:",
        "Cook time:",
        "Servings:",
        "Difficulty:",
        "Nutrition:",
    ];
    let mut blueberry_pie = None;
    for row in arrived {
        let (status, recipe) = app.post_op(
            "get_recipe",
            Some(&key),
            &json!({ "branch_id": row["branch_id"] }).to_string(),
        );
        assert_eq!(status, 200, "{recipe}");
        let content = &recipe["result"]["versions"][0]["content"];
        let note = content["note"].as_str().unwrap_or("");
        for lead in leads {
            if note.split("\n\n").any(|p| p.starts_with(lead)) {
                *paragraphs.entry(lead).or_default() += 1;
            }
        }
        if content["title"] == "Blueberry Pie - Aeri’s Kitchen" {
            blueberry_pie = Some(content.clone());
        }
    }
    // 141 nutrition texts are led by the line, and the other 19 of the 160
    // already name themselves on their first line. Most prep and cook times
    // kept landed too but say more, `35 to 40 mins`. Most servings
    // kept are a range, `Serves 4 to 6`, whose upper end no Yield holds.
    assert_eq!(
        paragraphs.into_iter().collect::<Vec<_>>(),
        [
            ("Cook time:", 29),
            ("Difficulty:", 83),
            ("Nutrition:", 141),
            ("Prep time:", 8),
            ("Servings:", 89),
            ("Total time:", 30),
        ]
    );
    // Its prep and cook times landed, so only the total and the difficulty
    // are said again.
    let pie = blueberry_pie.expect("the blueberry pie arrived");
    assert_eq!(pie["prep_time_minutes"], json!(30));
    assert_eq!(pie["cook_time_minutes"], json!(45));
    assert_eq!(
        pie["note"],
        json!("Total time: 3 hours\n\nDifficulty: Medium")
    );

    // Importing it again shelves nothing twice.
    let again = import(&app, &key, &file);
    let again = again["arrived"].as_array().unwrap();
    assert_eq!(again.len(), 746);
    assert!(again.iter().all(|row| row["status"] == json!("unchanged")));
}
