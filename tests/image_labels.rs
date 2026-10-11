//! The image says which release it is (#233): the `Dockerfile` writes the
//! `VERSION` and `GIT_SHA` build arguments to two labels, the only trace of a
//! build a registry lookup can see. This reads the file, so it needs no Docker.

const ARGUMENTS: [&str; 2] = ["VERSION", "GIT_SHA"];

/// Each label, with the argument it is written from.
const LABELS: [(&str, &str); 2] = [
    ("org.opencontainers.image.version", "VERSION"),
    ("org.opencontainers.image.revision", "GIT_SHA"),
];

/// The stage that writes the arguments to a file, and the one place besides
/// the image itself that may declare them.
const STAMP: &str = "stamp";

/// One stage of the `Dockerfile`: its `FROM` line and every line under it.
struct Stage {
    /// What follows `AS`, or nothing for the stage that ships.
    name: String,
    lines: Vec<String>,
}

/// Every stage in order. The last is the image that ships.
fn stages() -> Vec<Stage> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Dockerfile");
    let dockerfile = std::fs::read_to_string(path).expect("the Dockerfile is read");
    let mut stages: Vec<Stage> = Vec::new();
    for line in dockerfile.lines() {
        if let Some(from) = instruction(line, "FROM") {
            let words: Vec<&str> = from.split_whitespace().collect();
            let name = match words[..] {
                [_, word, name] if word.eq_ignore_ascii_case("AS") => name.to_string(),
                _ => String::new(),
            };
            stages.push(Stage {
                name,
                lines: Vec::new(),
            });
        } else {
            // Nothing above the first FROM may declare an argument either.
            assert!(
                !stages.is_empty() || !ARGUMENTS.iter().any(|one| declares(line, one)),
                "`{line}` is above every stage"
            );
        }
        if let Some(stage) = stages.last_mut() {
            stage.lines.push(line.to_string());
        }
    }
    stages
}

/// The words of a line that is the named instruction, written in either case.
fn instruction<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let (word, rest) = line.split_once(' ')?;
    word.eq_ignore_ascii_case(name).then_some(rest.trim())
}

/// Whether a line declares the build argument, with any default or none.
fn declares(line: &str, argument: &str) -> bool {
    instruction(line, "ARG").is_some_and(|declared| {
        declared
            .split_whitespace()
            .any(|one| one.split('=').next() == Some(argument))
    })
}

/// Where a stage declares the argument as empty unless the build passes it,
/// so a build that passes nothing works as it did.
fn declared_empty_at(stage: &Stage, argument: &str) -> usize {
    let declaration = format!("ARG {argument}=\"\"");
    stage
        .lines
        .iter()
        .position(|line| *line == declaration)
        .unwrap_or_else(|| panic!("`{}` declares `{declaration}`", stage.lines[0]))
}

/// A label as the image will carry it, with the line that writes it. A label
/// written twice keeps its last value, so this reads the last one. A `LABEL`
/// runs over continuation lines to the first that does not end in a backslash.
fn label(stage: &Stage, key: &str) -> Option<(usize, String)> {
    let mut found = None;
    let mut inside = false;
    for (at, line) in stage.lines.iter().enumerate() {
        let pairs = match instruction(line, "LABEL") {
            Some(pairs) => pairs,
            None if inside => line.trim(),
            None => continue,
        };
        inside = pairs.ends_with('\\');
        for pair in pairs.trim_end_matches('\\').split_whitespace() {
            if let Some((name, value)) = pair.split_once('=')
                && name == key
            {
                found = Some((at, value.trim_matches('"').to_string()));
            }
        }
    }
    found
}

#[test]
fn the_image_is_labelled_with_its_version_and_its_commit() {
    let stages = stages();
    let shipped = stages.last().expect("the Dockerfile has a stage");

    for (key, argument) in LABELS {
        let (written_at, value) =
            label(shipped, key).unwrap_or_else(|| panic!("the last stage writes {key}"));
        assert_eq!(value, format!("${{{argument}}}"), "{key}");

        // An argument is in scope only below its declaration, in its own
        // stage. Read anywhere else it is empty, and no build says so.
        assert!(
            declared_empty_at(shipped, argument) < written_at,
            "{argument} is declared above {key}"
        );
    }
}

#[test]
fn no_stage_that_compiles_takes_either_build_argument() {
    let stages = stages();
    let (_, above) = stages.split_last().expect("the Dockerfile has a stage");

    // A changed argument throws away every layer below its declaration, and
    // in these stages that is the compiled libraries or the binary itself.
    for stage in above.iter().filter(|stage| stage.name != STAMP) {
        let instructions = stage.lines.iter().filter(|line| !line.starts_with('#'));
        for line in instructions {
            for argument in ARGUMENTS {
                assert!(!declares(line, argument), "`{line}` in `{}`", stage.name);
            }
            // Built on the stamp, or copying from it, a stage would be thrown
            // away with it.
            assert!(
                !line.split(['=', ' ']).any(|word| word == STAMP),
                "`{line}` in `{}` reads the stamp",
                stage.name
            );
        }
    }
}

#[test]
fn a_changed_argument_reaches_the_labels_through_a_copied_file() {
    let stages = stages();
    let shipped = stages.last().expect("the Dockerfile has a stage");
    let stamp = stages
        .iter()
        .find(|stage| stage.name == STAMP)
        .expect("a `stamp` stage");

    // Podman 4.9 reuses a LABEL step by its text, the argument unread: with
    // the layers above unchanged, a new commit kept the last one's label. It
    // compares a copied file by what the file holds, so the stamp holds both
    // arguments and the image copies it in above its labels.
    let written = stamp
        .lines
        .iter()
        .position(|line| instruction(line, "RUN").is_some())
        .expect("the stamp stage writes its file");
    let run = &stamp.lines[written];
    assert!(run.ends_with(" > /stamp"), "the stamp is written to /stamp");
    for argument in ARGUMENTS {
        assert!(declared_empty_at(stamp, argument) < written);
        assert!(
            run.contains(&format!("\"${argument}\"")),
            "the stamp holds {argument}"
        );
    }

    let copied = shipped
        .lines
        .iter()
        .position(|line| {
            instruction(line, "COPY").is_some_and(|copy| copy.starts_with("--from=stamp /stamp "))
        })
        .expect("the last stage copies /stamp in from the stamp stage");
    for (key, _) in LABELS {
        let (written_at, _) = label(shipped, key).expect("the last stage writes the label");
        assert!(copied < written_at, "the stamp is copied in above {key}");
    }
}
