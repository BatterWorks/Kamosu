//! A Photograph: known by its own contents, remade once at the door before it
//! has an identity (#45, ADR 0017). This module does the remaking and the
//! Display Copy work; [`crate::core::Core`] owns where the bytes live and the
//! `photographs` table that records what exists.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use std::sync::{Condvar, Mutex};

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, imageops::FilterType};
use sha2::{Digest, Sha256};

use crate::core::OpError;
use crate::slow_work::off_the_request_workers;

/// A Photograph's long edge, capped in pixels — the one dimension every
/// remade picture is measured against (ADR 0017). Never upscaled: a picture
/// already smaller than this is left at its own size.
pub const MAX_LONG_EDGE: u32 = 2560;

/// The quality WebP is encoded at. Chosen and measured in ADR 0017; free to
/// change later since a Photograph, once made, never changes, and only a
/// future upload is affected.
const WEBP_QUALITY: f32 = 80.0;

/// An Original picture's long edge (#269). An agent reads small print off it,
/// where a Photograph is only looked at. A phone's 4032-pixel photograph of a
/// two-page spread, 40 cm across, comes down to about 200 pixels an inch,
/// where the small letters of 8-point print are still a dozen pixels tall.
/// One page alone is above 300. A Photograph's 2560 would leave the spread
/// near 160.
pub const ORIGINAL_LONG_EDGE: u32 = 3200;

/// The quality an Original picture is encoded at: above a Photograph's, since
/// lossy blocks blur the edges of letters before they blur a dish.
const ORIGINAL_WEBP_QUALITY: f32 = 90.0;

/// The most bytes a picture may arrive as, whichever way it arrives. ADR 0033
/// sets this number for a picture Kamosu fetches from a URL, and a picture
/// handed to it directly is the same thing with a shorter journey, so
/// `web_import` reads it from here rather than keeping a second copy.
///
/// Distinct from `/api/uploads`' own 2 GB cap, which is sized for a Bundle
/// carrying a whole library. A Bundle may be enormous; one picture inside it
/// may not.
pub const MAX_PICTURE_BYTES: usize = 25 * 1024 * 1024;

/// The most pixels a picture may *claim*, checked against the header before
/// anything is decoded. A decompression bomb is small on disk and vast once
/// unpacked. 60 KB declaring 50,000 × 50,000 is 2.5 billion pixels, which is
/// several gigabytes of memory the moment it is decoded and an instance that
/// falls over. The number a file declares is the one thing available before
/// paying that cost, so it is what Kamosu decides on.
///
/// 100 megapixels is far above any camera a household owns (a 2560-pixel long
/// edge is what a Photograph is stored at in any case). It is not enough on
/// its own: what decoding costs is the pixels times the bytes each takes,
/// which [`MOST_UNPACKED`] holds.
const MAX_MEGAPIXELS: u64 = 100;

/// The most bytes a picture may unpack to, read off its header as width ×
/// height × the bytes its colours take per pixel, before anything is decoded
/// (#538). A 778 KB PNG of 100 megapixels at 16 bits a colour with
/// transparency unpacks to 800 MB, and remaking it took Kamosu to 1.3 GB.
///
/// 200 MB is a 50-megapixel picture at 8 bits a colour with transparency, so
/// every camera photograph passes: a 50-megapixel JPEG unpacks to 150 MB, and
/// a phone's 48-megapixel one is usually saved at 12. This bounds the decode,
/// not the whole remake: turning the picture upright copies it, and the
/// resize holds a working copy of its own. Measured on 6 November 2026, two
/// of the largest pictures let in, remade at once, took the instance to
/// 1.6 GB at its peak; four 16-bit pictures had taken it to 5 GB before.
const MOST_UNPACKED: u64 = 50 * 1_000_000 * 4;

/// Pictures one instance remakes at once, so a run of large ones costs at
/// most this many cores and this many times what one costs in memory (#538).
/// One more asked for by a person is answered busy, as a third PDF is (#373).
/// One in an import waits its turn, and takes a place only while that leaves
/// one free, so imports running never keep a person's picture out.
const MOST_AT_ONCE: usize = 2;

/// A Display Copy's own long edge — worked out from the Photograph, kept only
/// for convenience, and rebuildable, so these numbers are free to change
/// (ADR 0017 sets Card and Page from a real measured library; Print
/// has no Sheet consumer yet and is a reasonable placeholder until one exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplaySize {
    /// A shelf card: GLOSSARY.md, "Display Copy".
    Card,
    /// A recipe page on a phone.
    Page,
    /// A print-sized rendering for a Sheet.
    Print,
}

impl DisplaySize {
    pub fn long_edge(self) -> u32 {
        match self {
            DisplaySize::Card => 600,
            DisplaySize::Page => 1400,
            DisplaySize::Print => 2000,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DisplaySize::Card => "card",
            DisplaySize::Page => "page",
            DisplaySize::Print => "print",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "card" => Some(DisplaySize::Card),
            "page" => Some(DisplaySize::Page),
            "print" => Some(DisplaySize::Print),
            _ => None,
        }
    }
}

/// Where a Photograph's own bytes live under `/data` (ADR 0017, ADR 0028).
pub fn photographs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("photographs")
}

/// Where Display Copies live under `/data` — rebuildable, so a separate
/// directory that can be emptied without touching a Photograph.
pub fn display_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("display")
}

pub fn photograph_path(data_dir: &Path, hash: &str) -> PathBuf {
    photographs_dir(data_dir).join(format!("{hash}.webp"))
}

pub fn display_path(data_dir: &Path, hash: &str, size: DisplaySize) -> PathBuf {
    display_dir(data_dir).join(format!("{hash}-{}.webp", size.as_str()))
}

/// A Photograph's identity: the contents alone (ADR 0017), distinguished from
/// a Version's `v_` fingerprint by its own prefix.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hashing = HashingBytes::default();
    hashing.add(bytes);
    hashing.finish()
}

/// [`hash_bytes`] of bytes that arrive a piece at a time, so a picture can be
/// checked against its name without being held whole (#296).
#[derive(Clone, Default)]
pub struct HashingBytes(Sha256);

impl HashingBytes {
    pub fn add(&mut self, piece: &[u8]) {
        self.0.update(piece);
    }

    pub fn finish(self) -> String {
        format!("p_{}", hex::encode(self.0.finalize()))
    }
}

/// The pictures an instance is remaking now, of the [`MOST_AT_ONCE`] it may
/// (#538). Every picture handed to Kamosu is remade through one of these, and
/// the Core holds the one there is.
#[derive(Default)]
pub struct Remaking {
    at_once: Mutex<usize>,
    freed: Condvar,
}

/// Whether a picture asked for while every place is taken waits for one.
#[derive(Clone, Copy)]
enum Turn {
    /// Answered busy at once: a person is waiting on the answer, and can
    /// send it again in a moment.
    Now,
    /// Waits until a place is free with another left beside it: an
    /// import's picture, which nobody is waiting on and which would
    /// otherwise be left out of what landed.
    Waiting,
}

impl Remaking {
    /// Remake whatever arrived into the one agreed form: WebP, quality 80, at
    /// most 2560 pixels on the long edge, camera metadata stripped (ADR
    /// 0017). Checks the picture's own header before decoding anything, and
    /// refuses SVG outright — decoding is the one place an untrusted upload
    /// gets to run arbitrary-looking code, so nothing here decodes bytes it
    /// hasn't first recognised as a real picture. Busy when
    /// [`MOST_AT_ONCE`] are being remade.
    pub fn remake(&self, bytes: &[u8]) -> Result<Vec<u8>, OpError> {
        self.remake_at(bytes, MAX_LONG_EDGE, WEBP_QUALITY, Turn::Now)
    }

    /// [`Remaking::remake`] for a picture an import brings, which waits for
    /// a place rather than being answered busy.
    pub fn remake_in_turn(&self, bytes: &[u8]) -> Result<Vec<u8>, OpError> {
        self.remake_at(bytes, MAX_LONG_EDGE, WEBP_QUALITY, Turn::Waiting)
    }

    /// Remake a picture handed to an agent as its Original (#269): checked
    /// and stripped as a Photograph is, the place it was taken above all, but
    /// kept larger and sharper so print on it stays legible. It is no
    /// Photograph and is stored as none.
    pub fn remake_original(&self, bytes: &[u8]) -> Result<Vec<u8>, OpError> {
        self.remake_at(bytes, ORIGINAL_LONG_EDGE, ORIGINAL_WEBP_QUALITY, Turn::Now)
    }

    /// Remake at a given long edge and quality: the one path every picture
    /// handed to Kamosu is decoded through. Decoding, resizing and encoding
    /// a camera's picture holds a thread far longer than a request should,
    /// so it is done off the request workers whichever Door it came through
    /// (#373), and waiting for a place with it.
    fn remake_at(
        &self,
        bytes: &[u8],
        long_edge: u32,
        quality: f32,
        turn: Turn,
    ) -> Result<Vec<u8>, OpError> {
        off_the_request_workers(|| {
            // Refused before it takes a place: a picture that will not be
            // decoded should not make another wait. Reading a header can
            // itself unpack a little, so it is off the request workers too.
            checked_decoder(bytes)?;
            let _place = self.take_place(turn)?;
            remade(bytes, long_edge, quality)
        })
    }

    /// Test-only, compiled only under the `test-jobs` feature: take one
    /// place as a remake would and hold it until the answer is dropped, so a
    /// test can say what a picture meets while every place is taken.
    #[cfg(feature = "test-jobs")]
    pub fn hold_a_place(&self) -> Result<Place<'_>, OpError> {
        self.take_place(Turn::Now)
    }

    /// One of the [`MOST_AT_ONCE`] places, or busy.
    fn take_place(&self, turn: Turn) -> Result<Place<'_>, OpError> {
        let mut at_once = self.at_once.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            match turn {
                Turn::Now if *at_once < MOST_AT_ONCE => break,
                Turn::Now => return Err(OpError::busy()),
                Turn::Waiting if *at_once + 1 < MOST_AT_ONCE => break,
                Turn::Waiting => {
                    at_once = self.freed.wait(at_once).unwrap_or_else(|e| e.into_inner());
                }
            }
        }
        *at_once += 1;
        Ok(Place(self))
    }
}

/// One of the [`MOST_AT_ONCE`] places, given back however the remake ends.
pub struct Place<'a>(&'a Remaking);

impl Drop for Place<'_> {
    fn drop(&mut self) {
        let mut at_once = self.0.at_once.lock().unwrap_or_else(|e| e.into_inner());
        *at_once -= 1;
        // Every import waiting looks again: which of them may go depends on
        // how many places are taken, not only on this one coming free.
        self.0.freed.notify_all();
    }
}

/// The remaking itself, on a picture [`checked_decoder`] passes. Its header
/// is read a second time here, which costs nothing beside the decode.
fn remade(bytes: &[u8], long_edge: u32, quality: f32) -> Result<Vec<u8>, OpError> {
    let mut decoder = checked_decoder(bytes)?;
    // Cameras stamp rotation into Exif rather than the pixels (eight of
    // the sixty in the real export do). Reading it now and baking it in
    // before the metadata is stripped below is the only chance to keep
    // it upright.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|e| OpError::bad_request(format!("cannot read this picture: {e}")))?;
    image.apply_orientation(orientation);
    encode_at(&downscale(image, long_edge), quality)
}

/// Whether these bytes start the way a picture Kamosu reads does. Says
/// nothing about whether the rest of it decodes.
pub fn looks_like_a_picture(bytes: &[u8]) -> bool {
    image::guess_format(bytes).is_ok()
}

/// Work out one Display Copy from an already-remade Photograph. No
/// orientation handling needed: a Photograph's own bytes already carry it
/// correctly and nothing else remains in them to strip. Made when a screen
/// first asks for it, so off the request workers as a remake is.
pub fn display_copy(photograph_bytes: &[u8], long_edge: u32) -> Result<Vec<u8>, OpError> {
    off_the_request_workers(|| {
        let image = image::load_from_memory_with_format(photograph_bytes, ImageFormat::WebP)
            .map_err(|e| OpError::internal(format!("cannot read stored Photograph: {e}")))?;
        encode(&downscale(image, long_edge))
    })
}

/// Scale down to fit within `max_long_edge`, preserving aspect ratio, and
/// never upscale a picture already smaller than the cap (ADR 0017).
fn downscale(image: DynamicImage, max_long_edge: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_long_edge {
        image
    } else {
        image.resize(max_long_edge, max_long_edge, FilterType::Lanczos3)
    }
}

/// Encode as lossy WebP. Starting from a freshly decoded pixel buffer with no
/// path back to the source bytes is what strips every camera metadata field
/// as a side effect, the GPS tag above all (ADR 0017) — there is no metadata
/// chunk here to carry one.
fn encode(image: &DynamicImage) -> Result<Vec<u8>, OpError> {
    encode_at(image, WEBP_QUALITY)
}

fn encode_at(image: &DynamicImage, quality: f32) -> Result<Vec<u8>, OpError> {
    let rgb = on_white(image);
    let encoder = webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height());
    Ok(encoder.encode(quality).to_vec())
}

/// The picture as a page shows it, its clear parts white. WebP is encoded
/// here without transparency, and dropping it outright leaves each clear pixel
/// the colour it stores, which is black in most PNGs: a recipe card of dark
/// text on nothing became a black square.
fn on_white(image: &DynamicImage) -> image::RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }
    let rgba = image.to_rgba8();
    image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over_white = |c: u8| {
            let (c, a) = (u32::from(c), u32::from(a));
            ((c * a + 255 * (255 - a) + 127) / 255) as u8
        };
        image::Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

/// Check a picture that is being kept exactly as it arrived, without remaking
/// it. A Photograph travelling in a Bundle is stored byte-for-byte, because
/// the bytes are its identity and re-encoding on receipt would report you as
/// diverged from the sender over a picture neither of you touched (ADR 0017).
/// Byte-for-byte is not the same as unexamined: what arrives still has to be
/// a picture, and still has to be one of a size Kamosu will later agree to
/// decode when it draws a Display Copy from it.
pub fn check(bytes: &[u8]) -> Result<(), OpError> {
    checked_decoder(bytes).map(|_| ())
}

/// A decoder over bytes that have passed every check Kamosu makes from a
/// header: small enough to hold, a raster format Kamosu reads, and claiming no
/// more pixels than Kamosu will unpack.
///
/// The order is the point. Each check is cheaper than the one after it, and
/// all of them happen before any pixel is decoded, which is the single place
/// an untrusted upload gets to make Kamosu do arbitrary work. Returning the
/// decoder rather than the format is what keeps that true of the caller too:
/// there is no second header parse, and no way to reach a decode that skipped
/// this function.
fn checked_decoder(bytes: &[u8]) -> Result<impl ImageDecoder + '_, OpError> {
    if bytes.len() > MAX_PICTURE_BYTES {
        return Err(OpError::bad_request(format!(
            "this picture is larger than {} MB",
            MAX_PICTURE_BYTES / (1024 * 1024)
        )));
    }
    let format = sniff(bytes)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    // The crate's own limits, a second guard behind the checks below. They
    // reach only what a decoder chooses to count: the PNG decoder counts the
    // chunks it unpacks while reading the header, the JPEG decoder nothing.
    // The checks below, on what the header claims, are the real limit.
    let mut limits = Limits::default();
    limits.max_alloc = Some(MOST_UNPACKED);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|e| OpError::bad_request(format!("cannot read this picture: {e}")))?;
    // Reading the header is all this has cost: `into_decoder` parsed the
    // dimensions out of it and no pixel data has been touched.
    let (width, height) = decoder.dimensions();
    let claimed = u64::from(width) * u64::from(height);
    if claimed > MAX_MEGAPIXELS * 1_000_000 {
        return Err(OpError::bad_request(format!(
            "this picture says it is {width} × {height}, which is more than the {MAX_MEGAPIXELS} megapixels Kamosu will open"
        )));
    }
    if decoder.total_bytes() > MOST_UNPACKED {
        return Err(OpError::bad_request(format!(
            "this picture says it is {width} × {height} in {} bytes a pixel, which unpacks to more than the {} MB Kamosu will open",
            decoder.color_type().bytes_per_pixel(),
            MOST_UNPACKED / 1_000_000
        )));
    }
    Ok(decoder)
}

/// Recognise a picture by its own header before anything tries to decode it.
/// SVG is refused outright rather than folded into "unrecognised": it is XML,
/// not a raster picture, and Kamosu has no renderer for one.
fn sniff(bytes: &[u8]) -> Result<ImageFormat, OpError> {
    let sample_len = bytes.len().min(64);
    let sample = String::from_utf8_lossy(&bytes[..sample_len]);
    let trimmed = sample.trim_start();
    if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") {
        return Err(OpError::bad_request(
            "SVG is not a picture Kamosu can store — only raster formats are accepted",
        ));
    }
    image::guess_format(bytes)
        .map_err(|_| OpError::bad_request("this does not look like a picture Kamosu can read"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remake(bytes: &[u8]) -> Result<Vec<u8>, OpError> {
        Remaking::default().remake(bytes)
    }

    /// A real, freshly encoded PNG of the given size — generated rather than
    /// hand-typed, so the fixture itself cannot be wrong the way a hand-typed
    /// CRC could be.
    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgb8(image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encodes");
        bytes
    }

    #[test]
    fn remaking_the_same_picture_twice_is_byte_identical() {
        let png = make_png(40, 30);
        let a = remake(&png).expect("remakes");
        let b = remake(&png).expect("remakes");
        assert_eq!(a, b, "remaking must be deterministic for dedup to work");
        assert_eq!(hash_bytes(&a), hash_bytes(&b));
    }

    #[test]
    fn remaking_gives_a_photograph_that_decodes_as_webp() {
        let remade = remake(&make_png(40, 30)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!((decoded.width(), decoded.height()), (40, 30));
    }

    #[test]
    fn a_picture_already_smaller_than_the_cap_is_never_upscaled() {
        let remade = remake(&make_png(40, 30)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!((decoded.width(), decoded.height()), (40, 30));
    }

    #[test]
    fn a_picture_bigger_than_the_cap_is_downscaled_to_it() {
        let remade = remake(&make_png(4000, 2000)).expect("remakes");
        let decoded =
            image::load_from_memory_with_format(&remade, ImageFormat::WebP).expect("valid webp");
        assert_eq!(decoded.width(), MAX_LONG_EDGE);
        assert_eq!(decoded.height(), 1280);
    }

    /// A PNG with an opaque black square in its middle and a clear border,
    /// each clear pixel storing black, as most PNGs with clear parts do.
    fn see_through_png(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(width, height, |x, y| {
            let inside = (width / 4..width * 3 / 4).contains(&x)
                && (height / 4..height * 3 / 4).contains(&y);
            image::Rgba([0, 0, 0, if inside { 255 } else { 0 }])
        }));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encodes");
        bytes
    }

    #[test]
    fn a_picture_with_clear_parts_is_laid_on_white() {
        // A recipe card of dark text on nothing arrived as a black square: the
        // clear pixels kept the black they stored, and the text was lost in it.
        let remade = remake(&see_through_png(40, 40)).expect("remakes");
        let decoded = image::load_from_memory_with_format(&remade, ImageFormat::WebP)
            .expect("valid webp")
            .to_rgb8();
        let clear = decoded.get_pixel(0, 0).0;
        assert!(
            clear.iter().all(|&c| c > 240),
            "a clear pixel should come out white, got {clear:?}"
        );
        let opaque = decoded.get_pixel(20, 20).0;
        assert!(
            opaque.iter().all(|&c| c < 15),
            "an opaque pixel should keep its own colour, got {opaque:?}"
        );
    }

    /// CRC-32, as PNG defines it over a chunk's type and data. Written out
    /// here rather than pulled in, because the only thing in this file that
    /// needs one is the bomb below, and a fixture that computes its own
    /// checksum cannot be wrong the way a hand-typed one could.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// A real PNG whose header *claims* an enormous picture while the file
    /// itself stays tiny, which is the shape of a decompression bomb. The pixel data
    /// is left as the small picture's, which is exactly the point: nothing
    /// may get far enough to notice it disagrees.
    fn claiming_to_be(width: u32, height: u32) -> Vec<u8> {
        let mut png = make_png(4, 4);
        // PNG is an 8-byte signature then chunks of length, type, data, CRC.
        // IHDR is always first, 13 bytes of data, and opens with the
        // dimensions, so they sit at a fixed place and the CRC follows them.
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        png
    }

    #[test]
    fn a_picture_claiming_more_pixels_than_kamosu_opens_is_refused_by_its_header() {
        let bomb = claiming_to_be(50_000, 50_000);
        assert!(
            bomb.len() < 1024,
            "the whole point is that it is small on disk: {} bytes",
            bomb.len()
        );
        let err = remake(&bomb).expect_err("a declared 2.5-gigapixel picture must be refused");
        assert!(
            err.message.contains("50000 × 50000"),
            "the refusal should say what the picture claimed, got: {}",
            err.message
        );
    }

    /// A PNG of 16 bits a colour with transparency, which unpacks to eight
    /// bytes a pixel, claiming the given size by its header.
    fn deep_png_claiming_to_be(width: u32, height: u32) -> Vec<u8> {
        let small = DynamicImage::ImageRgba16(image::ImageBuffer::new(4, 4));
        let mut png = Vec::new();
        small
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .expect("encodes");
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        png
    }

    #[test]
    fn a_picture_that_unpacks_past_the_limit_is_refused_by_its_header() {
        // 100 megapixels is within the count, and eight bytes each is 800 MB.
        let err = check(&deep_png_claiming_to_be(10_000, 10_000))
            .expect_err("a picture unpacking to 800 MB must be refused");
        assert!(
            err.message.contains("8 bytes a pixel") && err.message.contains("200 MB"),
            "the refusal should say why, got: {}",
            err.message
        );
        // A 50-megapixel camera photograph, three bytes a pixel, still passes.
        check(&claiming_to_be(8_660, 5_773)).expect("a camera's photograph passes");
    }

    #[test]
    fn a_third_picture_while_two_are_remade_is_busy_and_an_import_waits() {
        let remaking = Remaking::default();
        let first = remaking.take_place(Turn::Now).expect("a place");
        let second = remaking.take_place(Turn::Now).expect("a place");
        let busy = remaking
            .remake(&make_png(40, 30))
            .expect_err("both places are taken");
        assert_eq!(busy.kind, crate::core::ErrorKind::Busy);
        std::thread::scope(|scope| {
            let waiting = scope.spawn(|| remaking.remake_in_turn(&make_png(40, 30)));
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(!waiting.is_finished(), "an import's picture waits its turn");
            drop(first);
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(
                !waiting.is_finished(),
                "with one place taken, an import leaves the other for a person"
            );
            drop(second);
            waiting
                .join()
                .unwrap()
                .expect("remade once both places are free");
        });
        remaking
            .remake(&make_png(40, 30))
            .expect("each place is given back");
    }

    #[test]
    fn a_large_but_ordinary_picture_is_still_accepted() {
        // Comfortably under the cap, and bigger than any Photograph is kept
        // at. The check must not be a second, quieter size limit.
        remake(&make_png(5000, 4000)).expect("an ordinary large picture must still be accepted");
    }

    #[test]
    fn a_picture_larger_than_the_byte_cap_is_refused_without_being_read() {
        let too_big = vec![0u8; MAX_PICTURE_BYTES + 1];
        let err = remake(&too_big).expect_err("a picture over the byte cap must be refused");
        assert!(
            err.message.contains("25 MB"),
            "the refusal should name the cap, got: {}",
            err.message
        );
    }

    #[test]
    fn a_picture_kept_as_it_arrived_is_checked_just_the_same() {
        // The Bundle path stores bytes verbatim (ADR 0017); `check` is what
        // stops "verbatim" meaning "unexamined".
        check(&make_png(40, 30)).expect("an ordinary picture passes");
        check(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>")
            .expect_err("SVG must be refused even where nothing is re-encoded");
        check(&claiming_to_be(50_000, 50_000))
            .expect_err("a bomb must be refused even where nothing is re-encoded");
    }

    #[test]
    fn svg_is_refused_before_any_decode_is_attempted() {
        let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let err = remake(svg).expect_err("SVG must be refused");
        assert!(err.message.to_ascii_lowercase().contains("svg"));
    }

    #[test]
    fn garbage_bytes_are_refused_rather_than_panicking() {
        let garbage = vec![0u8; 128];
        assert!(remake(&garbage).is_err());
    }

    #[test]
    fn display_size_names_round_trip() {
        for size in [DisplaySize::Card, DisplaySize::Page, DisplaySize::Print] {
            assert_eq!(DisplaySize::parse(size.as_str()), Some(size));
        }
        assert_eq!(DisplaySize::parse("thumbnail"), None);
    }
}
