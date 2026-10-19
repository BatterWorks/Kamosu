//! Reading the text out of a recipe PDF (#176).
//!
//! **Only the text.** A PDF that a browser or a word processor printed holds
//! real text, and what comes out of it goes to the paste reader
//! ([`crate::pasting::read_printed`]) like anything pasted. A scan or a
//! photograph of a page holds a picture of text and no text at all, and is
//! refused with a sentence saying so. Reading a picture (OCR) is a separate
//! and much larger piece of work, and none of it is here.
//!
//! **The file is a stranger's**, so it is read in a program of its own
//! (#373): this one, started again as `kamosu read-pdf`, held to
//! [`MOST_MEMORY`] and stopped outright at [`LONGEST_READ`]. A crafted file
//! 218 KB long, a page that unpacks to 112 MB of drawing, grew Kamosu to
//! 10 GB in seconds when it was read in place, and a time limit on a thread
//! cannot stop a thread. `pdf-extract` answers most damaged files with an
//! error and panics on some others, and the reader ending any of those ways
//! becomes one refusal, so a bad file costs its sender one sentence and
//! nobody else anything.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::core::OpError;
use crate::slow_work::off_the_request_workers;

/// Past this many bytes a file is not one recipe. A two-page recipe printed
/// from a web page is 41 KB; a cookbook's worth of photographs is not what
/// this reads.
pub const LARGEST_PDF: u64 = 20 * 1024 * 1024;

/// The longest a PDF is read before its reader is stopped (#373). The recipe
/// printed from a web page is read in a fraction of a second; a file that
/// takes longer than this was built to keep its reader busy, or is no recipe.
const LONGEST_READ: Duration = Duration::from_secs(10);

/// The most memory a PDF's reader may take for its own (#373). Measured on
/// 19 October 2026: the two-page recipe printed from a web page took 23 MB
/// at its peak, and an 18 MB PDF of eighteen pages, each with a photograph,
/// took 58 MB. The 218 KB file above reached this cap in two seconds and
/// was stopped.
pub const MOST_MEMORY: u64 = 512 * 1000 * 1000;

/// PDFs one instance reads at once, so a run of them costs at most this many
/// cores and this many times [`MOST_MEMORY`]. One more is refused as busy
/// (#373).
const MOST_AT_ONCE: usize = 2;

/// The most text a reader hands back. Far past what a recipe holds, which
/// the caller refuses on its own terms; this only bounds what is held.
const MOST_TEXT: u64 = 16 * 1024 * 1024;

/// The processor time a reader may use, past which the system stops it. The
/// time limit above is kept by the Kamosu that started the reader, and this
/// stops one that Kamosu left behind by dying mid-read.
const MOST_PROCESSOR_SECONDS: libc::rlim_t = 15;

/// The exit code of a reader that read the file and found it unreadable.
const UNREADABLE: i32 = 2;

const DAMAGED: &str = "that PDF could not be read: it looks damaged";

/// The program a PDF is read in: the one running, unless
/// [`read_pdfs_with`] named another.
static READER: OnceLock<PathBuf> = OnceLock::new();

/// Read PDFs with this program rather than the one running. A test of the
/// library runs inside a test harness, which is not Kamosu, so it names
/// Kamosu's binary here before reading one. Only the first call counts.
pub fn read_pdfs_with(program: PathBuf) {
    let _ = READER.set(program);
}

/// The `reason` a PDF with no text is refused with, which the interface says
/// in its own words and three Languages (#176).
pub const NO_TEXT: &str = "pdf_has_no_text";

/// The refusal a scanned PDF gets. The wording was chosen on #176: it
/// says why, and what to do instead.
const NO_TEXT_SENTENCE: &str = "This PDF is a scan or a photo of a page, so it has no text to \
                                read. If it came from a website, use From a link instead, or copy \
                                the recipe's text and use From pasted text.";

/// Whether these bytes say they are a PDF: the marker in the first kilobyte,
/// where the format allows some leading junk before it.
pub fn looks_like_a_pdf(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    head.windows(5).any(|window| window == b"%PDF-")
}

/// The PDFs an instance is reading now, of the [`MOST_AT_ONCE`] it may.
#[derive(Default)]
pub struct Readers {
    reading: AtomicUsize,
}

impl Readers {
    /// **The text a PDF holds, or why it holds none.**
    pub fn text_of(&self, bytes: &[u8]) -> Result<String, OpError> {
        self.text_within(bytes, LONGEST_READ)
    }

    /// [`Readers::text_of`] with the time limit a test can reach.
    pub fn text_within(&self, bytes: &[u8], limit: Duration) -> Result<String, OpError> {
        // A PDF says so in its first kilobyte, where the format allows some
        // leading junk before the marker. Anything else is refused before the
        // parser sees it.
        if !looks_like_a_pdf(bytes) {
            return Err(OpError::bad_request("that file is not a PDF"));
        }
        let took = self
            .reading
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reading| {
                (reading < MOST_AT_ONCE).then_some(reading + 1)
            });
        if took.is_err() {
            return Err(OpError::busy());
        }
        let _place = Place(&self.reading);
        let text = off_the_request_workers(|| read_in_its_own_program(bytes, limit))?;
        if text.trim().is_empty() {
            return Err(no_text());
        }
        Ok(text)
    }
}

/// One of the [`MOST_AT_ONCE`] places, given back however the read ends.
struct Place<'a>(&'a AtomicUsize);

impl Drop for Place<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Start the reader, hand it the file, and wait for its text until `limit`.
fn read_in_its_own_program(bytes: &[u8], limit: Duration) -> Result<String, OpError> {
    // The running program by the kernel's own link to it, which still
    // starts it after its file on disk was replaced by an upgrade.
    let program = READER
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("/proc/self/exe"));
    let mut reader = Command::new(program)
        .arg("read-pdf")
        // It needs nothing Kamosu was started with.
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OpError::internal(format!("cannot start a PDF's reader: {e}")))?;
    let started = Instant::now();
    let (Some(mut file), Some(text_pipe), Some(said_pipe)) = (
        reader.stdin.take(),
        reader.stdout.take(),
        reader.stderr.take(),
    ) else {
        let _ = reader.kill();
        let _ = reader.wait();
        return Err(OpError::internal(
            "a PDF's reader started without its pipes",
        ));
    };
    std::thread::scope(|scope| {
        // Each pipe has a thread of its own, so a reader that stops reading
        // the file, or fills one pipe while the other waits, holds nothing up.
        scope.spawn(move || {
            let _ = file.write_all(bytes);
        });
        let text = scope.spawn(move || {
            let mut out = Vec::new();
            let _ = text_pipe.take(MOST_TEXT + 1).read_to_end(&mut out);
            out
        });
        let said = scope.spawn(move || {
            let mut out = Vec::new();
            let _ = said_pipe.take(4096).read_to_end(&mut out);
            String::from_utf8_lossy(&out).into_owned()
        });
        let ended = loop {
            match reader.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if started.elapsed() < limit => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                // Out of time, or no longer watchable: stopped either way.
                _ => {
                    let _ = reader.kill();
                    let _ = reader.wait();
                    break None;
                }
            }
        };
        let text = text.join().unwrap_or_default();
        let said = said.join().unwrap_or_default();
        refusal_or_text(ended, text, &said, limit)
    })
}

/// What a reader's ending means for the person who sent the file.
fn refusal_or_text(
    ended: Option<std::process::ExitStatus>,
    text: Vec<u8>,
    said: &str,
    limit: Duration,
) -> Result<String, OpError> {
    use std::os::unix::process::ExitStatusExt;
    let Some(status) = ended else {
        return Err(OpError::bad_request(format!(
            "that PDF could not be read in {} seconds, which is longer than a recipe takes",
            limit.as_secs_f32().ceil()
        )));
    };
    // Past the most text, the pipe was closed on the reader mid-write, so it
    // ended however a refused write ends it.
    if text.len() as u64 > MOST_TEXT {
        return Err(OpError::bad_request(format!(
            "a recipe PDF holds at most {} bytes of text, and that one holds more than {}",
            crate::pasting::LONGEST_PASTE,
            MOST_TEXT
        )));
    }
    if status.success() {
        return String::from_utf8(text).map_err(|_| OpError::bad_request(DAMAGED));
    }
    match (status.code(), status.signal()) {
        (Some(UNREADABLE), _) => Err(OpError::bad_request(format!(
            "that PDF could not be read: {}",
            said.trim()
        ))),
        // Rust aborts when memory runs out, and the cap is how it runs out.
        (_, Some(libc::SIGABRT)) => Err(OpError::bad_request(
            "that PDF could not be read: it needs far more memory than a recipe does",
        )),
        _ => Err(OpError::bad_request(DAMAGED)),
    }
}

/// `kamosu read-pdf`: read the PDF on standard input and write its text to
/// standard output, held to [`MOST_MEMORY`] before a byte is read. Only
/// Kamosu starts it, and it answers with its exit code: 0 with the text,
/// [`UNREADABLE`] with the reason on standard error.
pub fn read_here() -> i32 {
    // RLIMIT_DATA counts the heap and every private mapping, which is where
    // a parser's memory goes, and not the program's own file. A reader that
    // runs out of it aborts, and leaves no core file the size of the cap on
    // the disk. A machine that already holds a process to less keeps its
    // own, lower limit.
    for (resource, most) in [
        (libc::RLIMIT_DATA, MOST_MEMORY as libc::rlim_t),
        (libc::RLIMIT_CPU, MOST_PROCESSOR_SECONDS),
        (libc::RLIMIT_CORE, 0),
    ] {
        let mut cap = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: getrlimit(2) and setrlimit(2) touch only the struct they
        // are handed, which lives on this stack for both calls.
        let held = unsafe {
            libc::getrlimit(resource, &mut cap) == 0 && {
                let most = most.min(cap.rlim_max);
                cap.rlim_cur = most;
                cap.rlim_max = most;
                libc::setrlimit(resource, &cap) == 0
            }
        };
        if !held {
            eprintln!("Kamosu could not hold its PDF reader to its limits");
            return UNREADABLE;
        }
    }
    // A reader stopped at its cap is no crash worth reporting, and a host
    // that sends crashes to a program of its own would otherwise be handed
    // every one, at a second or so each.
    // SAFETY: prctl(2) with PR_SET_DUMPABLE takes plain integers.
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
    let mut bytes = Vec::new();
    if std::io::stdin()
        .take(LARGEST_PDF + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        eprintln!("the file did not arrive whole");
        return UNREADABLE;
    }
    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => match std::io::stdout().write_all(text.as_bytes()) {
            Ok(()) => 0,
            Err(_) => UNREADABLE,
        },
        Err(error) => {
            eprintln!("{error}");
            UNREADABLE
        }
    }
}

/// The refusal for a PDF with no text to read: a scan, a photograph, or a
/// page whose only marks are list bullets.
pub fn no_text() -> OpError {
    OpError::refused_because(NO_TEXT, NO_TEXT_SENTENCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused_before_it_is_read() {
        let refused = Readers::default()
            .text_of(b"PK\x03\x04 a zip file")
            .unwrap_err();
        assert_eq!(refused.message, "that file is not a PDF");
        assert_eq!(refused.reason, None);
    }
}
