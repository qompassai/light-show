//! Image asset integration tests.
//!
//! Validation: every markdown image referenced by `README.md` exists inside
//! the repo and decodes with the `image` crate (non-zero dimensions, GIFs have
//! at least one frame); every file under `game/assets/{sprites,fonts}` is
//! non-empty with an extension that belongs in that directory.
//!
//! Adversarial: the same decoder and validators are fed zero-byte, truncated,
//! garbage, mislabeled, and path-escaping inputs synthesized in a temp dir.
//! Each must come back as a typed error, never a panic and never `Ok`.

use std::fmt;
use std::fs;
use std::io::{self, Cursor, Read};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use image::codecs::gif::{GifDecoder, GifEncoder};
use image::{
    AnimationDecoder, Delay, DynamicImage, Frame, ImageDecoder, ImageError, ImageFormat, Rgba,
    RgbaImage,
};

/// README is prose; anything larger is not the file we think it is.
const README_BYTES_MAX: u64 = 1024 * 1024;
/// Largest committed README image is ~3.3 MB; leave headroom, refuse beyond.
const IMAGE_BYTES_MAX: u64 = 16 * 1024 * 1024;
/// Upper bound on GIF frames decoded per file.
const GIF_FRAME_COUNT_MAX: usize = 4096;
/// Upper bound on image references accepted from one markdown document.
const IMAGE_REF_COUNT_MAX: usize = 256;
/// Upper bound on directory entries visited by one asset walk.
const WALK_ENTRY_COUNT_MAX: usize = 10_000;
/// Upper bound on directory nesting below an asset root.
const WALK_DEPTH_MAX: usize = 8;

const SPRITE_EXTENSIONS: &[&str] = &["png", "aseprite", "md"];
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "txt"];

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

// ---------------------------------------------------------------------------
// Markdown image references
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum RefParseError {
    /// `![alt](` with no closing `)`; offset is the byte index of `![`.
    Unterminated {
        offset: usize,
    },
    /// `![alt]()` — nothing to resolve.
    EmptyTarget {
        offset: usize,
    },
    TooMany {
        count_max: usize,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum RefPathError {
    /// `scheme://` or `data:` target; not a repo file.
    Remote,
    Absolute,
    /// Contains `..`; rejected outright rather than normalized.
    EscapesRoot,
}

/// Extracts the targets of `![alt](target "title")` references, in order.
///
/// Bounded by `IMAGE_REF_COUNT_MAX`. The alt text must not contain `]`.
/// Code fences are not special-cased.
fn parse_image_refs(markdown: &str) -> Result<Vec<String>, RefParseError> {
    let mut targets = Vec::new();
    let mut cursor = 0;
    while let Some(found) = markdown[cursor..].find("![") {
        let start = cursor + found;
        let Some(alt_len) = markdown[start..].find(']') else {
            break;
        };
        let open = start + alt_len + 1;
        if !markdown[open..].starts_with('(') {
            cursor = open;
            continue;
        }
        let Some(target_len) = markdown[open + 1..].find(')') else {
            return Err(RefParseError::Unterminated { offset: start });
        };
        let inner = markdown[open + 1..open + 1 + target_len].trim();
        let target = inner.split_whitespace().next().unwrap_or("");
        let target = target.trim_start_matches('<').trim_end_matches('>');
        if target.is_empty() {
            return Err(RefParseError::EmptyTarget { offset: start });
        }
        if targets.len() == IMAGE_REF_COUNT_MAX {
            return Err(RefParseError::TooMany {
                count_max: IMAGE_REF_COUNT_MAX,
            });
        }
        targets.push(target.to_owned());
        cursor = open + 1 + target_len + 1;
    }
    Ok(targets)
}

/// Resolves a reference target to a path inside `root`, by path components.
fn resolve_ref(root: &Path, target: &str) -> Result<PathBuf, RefPathError> {
    if target.contains("://") || target.starts_with("data:") {
        return Err(RefPathError::Remote);
    }
    for component in Path::new(target).components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(RefPathError::EscapesRoot),
            Component::RootDir | Component::Prefix(_) => return Err(RefPathError::Absolute),
        }
    }
    Ok(root.join(target))
}

// ---------------------------------------------------------------------------
// Image decoding
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum DecodeError {
    Io(io::Error),
    TooLarge {
        bytes: u64,
    },
    Empty,
    /// Extension does not name a format the `image` crate knows.
    UnknownFormat(ImageError),
    Decode(ImageError),
    ZeroDimensions,
    NoFrames,
    TooManyFrames,
    /// The decoder panicked; always a bug, surfaced instead of propagated.
    Panicked,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "I/O: {source}"),
            Self::TooLarge { bytes } => write!(f, "{bytes} bytes > {IMAGE_BYTES_MAX}"),
            Self::Empty => write!(f, "zero-byte file"),
            Self::UnknownFormat(source) => write!(f, "unknown format: {source}"),
            Self::Decode(source) => write!(f, "decode: {source}"),
            Self::ZeroDimensions => write!(f, "zero width or height"),
            Self::NoFrames => write!(f, "GIF has no frames"),
            Self::TooManyFrames => write!(f, "GIF exceeds {GIF_FRAME_COUNT_MAX} frames"),
            Self::Panicked => write!(f, "decoder panicked"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodedImage {
    format: ImageFormat,
    width: u32,
    height: u32,
    frame_count: usize,
}

/// Decodes `path` fully, trusting its extension (not its magic bytes) for
/// the format so a mislabeled file is an error. Reads at most
/// `IMAGE_BYTES_MAX` bytes; GIFs decode at most `GIF_FRAME_COUNT_MAX` frames.
fn decode_image_file(path: &Path) -> Result<DecodedImage, DecodeError> {
    let format = ImageFormat::from_path(path).map_err(DecodeError::UnknownFormat)?;
    let bytes_len = fs::metadata(path).map_err(DecodeError::Io)?.len();
    if bytes_len > IMAGE_BYTES_MAX {
        return Err(DecodeError::TooLarge { bytes: bytes_len });
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(IMAGE_BYTES_MAX + 1).read_to_end(&mut bytes))
        .map_err(DecodeError::Io)?;
    // The file can grow between stat and read; the take() bound still holds.
    if bytes.len() as u64 > IMAGE_BYTES_MAX {
        return Err(DecodeError::TooLarge {
            bytes: bytes.len() as u64,
        });
    }
    decode_image_bytes(&bytes, format)
}

/// Decodes `bytes` as `format`, converting a decoder panic into an error.
fn decode_image_bytes(bytes: &[u8], format: ImageFormat) -> Result<DecodedImage, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Empty);
    }
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| match format {
        ImageFormat::Gif => decode_gif(bytes),
        _ => decode_still(bytes, format),
    }));
    let decoded = outcome.unwrap_or(Err(DecodeError::Panicked))?;
    if decoded.width == 0 || decoded.height == 0 {
        return Err(DecodeError::ZeroDimensions);
    }
    assert!(
        decoded.frame_count >= 1,
        "decoders return Err rather than zero frames"
    );
    Ok(decoded)
}

fn decode_still(bytes: &[u8], format: ImageFormat) -> Result<DecodedImage, DecodeError> {
    let image = image::load_from_memory_with_format(bytes, format).map_err(DecodeError::Decode)?;
    Ok(DecodedImage {
        format,
        width: image.width(),
        height: image.height(),
        frame_count: 1,
    })
}

fn decode_gif(bytes: &[u8]) -> Result<DecodedImage, DecodeError> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(DecodeError::Decode)?;
    let (width, height) = decoder.dimensions();
    let mut frame_count = 0;
    for frame in decoder.into_frames().take(GIF_FRAME_COUNT_MAX + 1) {
        frame.map_err(DecodeError::Decode)?;
        frame_count += 1;
    }
    if frame_count == 0 {
        return Err(DecodeError::NoFrames);
    }
    if frame_count > GIF_FRAME_COUNT_MAX {
        return Err(DecodeError::TooManyFrames);
    }
    Ok(DecodedImage {
        format: ImageFormat::Gif,
        width,
        height,
        frame_count,
    })
}

// ---------------------------------------------------------------------------
// Asset directory walk
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum AssetIssue {
    MissingDir(PathBuf),
    Unreadable { path: PathBuf, reason: String },
    EmptyFile(PathBuf),
    BadExtension(PathBuf),
    TooDeep(PathBuf),
    TooManyEntries(PathBuf),
}

/// Walks `dir` (explicit stack, bounded by `WALK_DEPTH_MAX` and
/// `WALK_ENTRY_COUNT_MAX`) and reports every file that is empty or whose
/// lowercase extension is not in `allowed`. Symlinks are not followed.
fn check_asset_dir(dir: &Path, allowed: &[&str]) -> Vec<AssetIssue> {
    let mut issues = Vec::new();
    if !dir.is_dir() {
        issues.push(AssetIssue::MissingDir(dir.to_path_buf()));
        return issues;
    }
    let mut stack = vec![(dir.to_path_buf(), 0_usize)];
    let mut entry_count = 0_usize;
    while let Some((current, depth)) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(source) => {
                let reason = source.to_string();
                issues.push(AssetIssue::Unreadable {
                    path: current,
                    reason,
                });
                continue;
            }
        };
        for entry in entries {
            entry_count += 1;
            if entry_count > WALK_ENTRY_COUNT_MAX {
                issues.push(AssetIssue::TooManyEntries(dir.to_path_buf()));
                return issues;
            }
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(source) => {
                    let reason = source.to_string();
                    issues.push(AssetIssue::Unreadable {
                        path: current.clone(),
                        reason,
                    });
                    continue;
                }
            };
            check_asset_entry(path, depth, allowed, &mut stack, &mut issues);
        }
    }
    issues
}

fn check_asset_entry(
    path: PathBuf,
    depth: usize,
    allowed: &[&str],
    stack: &mut Vec<(PathBuf, usize)>,
    issues: &mut Vec<AssetIssue>,
) {
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(source) => {
            issues.push(AssetIssue::Unreadable {
                path,
                reason: source.to_string(),
            });
            return;
        }
    };
    if metadata.is_dir() {
        if depth + 1 > WALK_DEPTH_MAX {
            issues.push(AssetIssue::TooDeep(path));
        } else {
            stack.push((path, depth + 1));
        }
        return;
    }
    if metadata.len() == 0 {
        issues.push(AssetIssue::EmptyFile(path));
        return;
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let extension_ok = extension.is_some_and(|ext| allowed.contains(&ext.as_str()));
    if !extension_ok {
        issues.push(AssetIssue::BadExtension(path));
    }
}

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/// Scratch directory under the OS temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        static SEQUENCE: AtomicU32 = AtomicU32::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let name = format!(
            "light-show-assets-{label}-{}-{nanos}-{sequence}",
            std::process::id()
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).expect("temp dir must be creatable for adversarial fixtures");
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent dir must be creatable");
        }
        fs::write(&path, bytes).expect("fixture file must be writable");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best-effort cleanup: a leftover dir in /tmp must not fail a test.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn gradient(width: u32, height: u32, phase: u8) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let red = (x * 4) as u8 ^ phase;
        let green = (y * 4) as u8;
        Rgba([red, green, phase, 255])
    })
}

fn encode_still(format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(gradient(64, 48, 7)).to_rgb8());
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, format)
        .expect("in-memory encode of a valid image cannot fail");
    bytes.into_inner()
}

fn encode_gif(frame_count: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        let frames = (0..frame_count).map(|index| {
            Frame::from_parts(
                gradient(64, 48, index * 40),
                0,
                0,
                Delay::from_numer_denom_ms(80, 1),
            )
        });
        encoder
            .encode_frames(frames)
            .expect("in-memory GIF encode cannot fail");
    }
    bytes
}

/// Deterministic xorshift bytes: garbage that is identical on every run.
fn garbage_bytes(len: usize, seed: u64) -> Vec<u8> {
    assert!(seed != 0, "xorshift state must be non-zero");
    let mut state = seed;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn assert_rejected(path: &Path) -> DecodeError {
    match decode_image_file(path) {
        Ok(decoded) => panic!(
            "{} must be rejected, decoded as {decoded:?}",
            path.display()
        ),
        Err(DecodeError::Panicked) => panic!("{}: decoder panicked", path.display()),
        Err(error) => error,
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn readme_refs() -> Vec<String> {
    let path = repo_root().join("README.md");
    let bytes_len = fs::metadata(&path)
        .expect("README.md must exist at repo root")
        .len();
    assert!(
        bytes_len <= README_BYTES_MAX,
        "README.md is {bytes_len} bytes"
    );
    let markdown = fs::read_to_string(&path).expect("README.md must be UTF-8 text");
    parse_image_refs(&markdown).expect("README.md image references must be well-formed")
}

#[test]
fn readme_has_local_image_refs() {
    let refs = readme_refs();
    let local_count = refs
        .iter()
        .filter(|target| resolve_ref(&repo_root(), target).is_ok())
        .count();
    // Guards against a parser regression silently making the decode test vacuous.
    assert!(
        local_count >= 1,
        "README.md has no local image references: {refs:?}"
    );
    assert!(
        refs.iter()
            .any(|target| target == "assets/art/hero_header.jpg"),
        "{refs:?}"
    );
}

#[test]
fn readme_image_refs_exist_inside_repo() {
    let root = repo_root();
    let mut failures = Vec::new();
    for target in readme_refs() {
        match resolve_ref(&root, &target) {
            Ok(path) if path.is_file() => {}
            Ok(_) => failures.push(format!("{target}: file not found")),
            Err(RefPathError::Remote) => println!("skipped remote image: {target}"),
            Err(error) => failures.push(format!("{target}: {error:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "README image refs broken:\n{}",
        failures.join("\n")
    );
}

#[test]
fn readme_images_decode() {
    let root = repo_root();
    let mut failures = Vec::new();
    for target in readme_refs() {
        let Ok(path) = resolve_ref(&root, &target) else {
            continue; // Rejections are reported by readme_image_refs_exist_inside_repo.
        };
        match decode_image_file(&path) {
            Ok(decoded) => println!(
                "{target}: {:?} {}x{} frames={}",
                decoded.format, decoded.width, decoded.height, decoded.frame_count
            ),
            Err(error) => failures.push(format!("{target}: {error}")),
        }
    }
    assert!(
        failures.is_empty(),
        "README images failed to decode:\n{}",
        failures.join("\n")
    );
}

#[test]
fn sprite_assets_nonempty_with_sane_extensions() {
    let issues = check_asset_dir(&repo_root().join("game/assets/sprites"), SPRITE_EXTENSIONS);
    assert!(issues.is_empty(), "sprite asset issues: {issues:#?}");
}

#[test]
fn font_assets_nonempty_with_sane_extensions() {
    let issues = check_asset_dir(&repo_root().join("game/assets/fonts"), FONT_EXTENSIONS);
    assert!(issues.is_empty(), "font asset issues: {issues:#?}");
}

#[test]
fn parser_handles_multiple_refs_titles_and_plain_links() {
    let markdown = "| ![a (b)](x/one.gif)<br>**A** | ![c](<x/two.png> \"Title\") |\n\
                    [not an image](x/link.md) ![dangling] text ![d](three.jpg)";
    let refs = parse_image_refs(markdown).expect("well-formed refs parse");
    assert_eq!(refs, ["x/one.gif", "x/two.png", "three.jpg"]);
}

#[test]
fn synthesized_valid_images_decode() {
    let temp = TempDir::new("valid");
    for (name, bytes, format) in [
        ("ok.png", encode_still(ImageFormat::Png), ImageFormat::Png),
        ("ok.jpg", encode_still(ImageFormat::Jpeg), ImageFormat::Jpeg),
        ("ok.gif", encode_gif(3), ImageFormat::Gif),
    ] {
        let decoded = decode_image_file(&temp.write(name, &bytes))
            .unwrap_or_else(|error| panic!("{name} must decode: {error}"));
        assert_eq!(
            (decoded.format, decoded.width, decoded.height),
            (format, 64, 48)
        );
    }
    let gif = decode_image_file(&temp.0.join("ok.gif")).expect("ok.gif decoded above");
    assert_eq!(gif.frame_count, 3);
}

// ---------------------------------------------------------------------------
// Adversarial
// ---------------------------------------------------------------------------

#[test]
fn zero_byte_images_rejected() {
    let temp = TempDir::new("empty");
    for name in ["empty.png", "empty.gif", "empty.jpg"] {
        let error = assert_rejected(&temp.write(name, b""));
        assert!(matches!(error, DecodeError::Empty), "{name}: {error}");
    }
}

#[test]
fn truncated_png_rejected() {
    let temp = TempDir::new("trunc-png");
    let bytes = encode_still(ImageFormat::Png);
    let error = assert_rejected(&temp.write("half.png", &bytes[..bytes.len() / 2]));
    assert!(matches!(error, DecodeError::Decode(_)), "{error}");
}

#[test]
fn truncated_gif_rejected() {
    let temp = TempDir::new("trunc-gif");
    let bytes = encode_gif(4);
    let error = assert_rejected(&temp.write("half.gif", &bytes[..bytes.len() / 2]));
    assert!(matches!(error, DecodeError::Decode(_)), "{error}");
}

#[test]
fn truncated_jpeg_header_rejected() {
    let temp = TempDir::new("trunc-jpg");
    let bytes = encode_still(ImageFormat::Jpeg);
    // JPEG decoders may zero-fill a cut scan, so cut inside the headers instead.
    let error = assert_rejected(&temp.write("cut.jpg", &bytes[..64]));
    assert!(matches!(error, DecodeError::Decode(_)), "{error}");
}

#[test]
fn garbage_bytes_with_image_extensions_rejected() {
    let temp = TempDir::new("garbage");
    for (seed, name) in [
        (0x9E37_79B9_7F4A_7C15, "noise.png"),
        (0xD1B5_4A32, "noise.gif"),
    ] {
        let error = assert_rejected(&temp.write(name, &garbage_bytes(4096, seed)));
        assert!(matches!(error, DecodeError::Decode(_)), "{name}: {error}");
    }
    let error = assert_rejected(&temp.write("noise.jpg", &garbage_bytes(4096, 0xC0FF_EE11)));
    assert!(
        matches!(error, DecodeError::Decode(_)),
        "noise.jpg: {error}"
    );
}

#[test]
fn text_file_renamed_png_rejected() {
    let temp = TempDir::new("text");
    let error = assert_rejected(&temp.write("notes.png", b"# TODO: draw the sprite\n"));
    assert!(matches!(error, DecodeError::Decode(_)), "{error}");
}

#[test]
fn valid_png_with_gif_extension_rejected() {
    let temp = TempDir::new("mislabel");
    let error = assert_rejected(&temp.write("liar.gif", &encode_still(ImageFormat::Png)));
    assert!(matches!(error, DecodeError::Decode(_)), "{error}");
}

#[test]
fn missing_unknown_and_oversized_files_rejected() {
    let temp = TempDir::new("io");
    let missing = assert_rejected(&temp.0.join("absent.png"));
    assert!(matches!(missing, DecodeError::Io(_)), "{missing}");
    let unknown = assert_rejected(&temp.write("sprite.notanimage", b"data"));
    assert!(
        matches!(unknown, DecodeError::UnknownFormat(_)),
        "{unknown}"
    );
    // Sparse file: exercises the size bound without writing 16 MiB.
    let huge = temp.write("huge.png", b"");
    fs::File::options()
        .write(true)
        .open(&huge)
        .and_then(|file| file.set_len(IMAGE_BYTES_MAX + 1))
        .expect("sparse fixture must be extendable");
    let oversized = assert_rejected(&huge);
    assert!(
        matches!(oversized, DecodeError::TooLarge { .. }),
        "{oversized}"
    );
}

#[test]
fn malformed_and_escaping_refs_rejected() {
    assert_eq!(
        parse_image_refs("ok ![a](x.png) then ![b](never-closed.png"),
        Err(RefParseError::Unterminated { offset: 20 })
    );
    assert_eq!(
        parse_image_refs("![a]()"),
        Err(RefParseError::EmptyTarget { offset: 0 })
    );
    let flood = "![a](x.png)".repeat(IMAGE_REF_COUNT_MAX + 1);
    assert_eq!(
        parse_image_refs(&flood),
        Err(RefParseError::TooMany {
            count_max: IMAGE_REF_COUNT_MAX
        })
    );
    let root = repo_root();
    let rejected = [
        ("../../etc/passwd", RefPathError::EscapesRoot),
        ("assets/../../outside.png", RefPathError::EscapesRoot),
        ("/etc/passwd", RefPathError::Absolute),
        ("https://example.com/badge.svg", RefPathError::Remote),
        ("data:image/png;base64,AAAA", RefPathError::Remote),
    ];
    for (target, expected) in rejected {
        assert_eq!(resolve_ref(&root, target), Err(expected), "{target}");
    }
}

#[test]
fn asset_dir_issues_reported_distinctly() {
    let temp = TempDir::new("assetdir");
    let empty = temp.write("sprites/empty.png", b"");
    let bad = temp.write("sprites/sub/payload.exe", b"MZ");
    let no_ext = temp.write("sprites/LICENSE", b"text");
    temp.write(
        "sprites/ok.png",
        b"not decoded here; only size and extension",
    );
    let mut issues = check_asset_dir(&temp.0.join("sprites"), SPRITE_EXTENSIONS);
    issues.sort_by_key(|issue| format!("{issue:?}"));
    assert_eq!(
        issues,
        [
            AssetIssue::BadExtension(no_ext),
            AssetIssue::BadExtension(bad),
            AssetIssue::EmptyFile(empty),
        ]
    );
    let missing = temp.0.join("fonts");
    assert_eq!(
        check_asset_dir(&missing, FONT_EXTENSIONS),
        [AssetIssue::MissingDir(missing)]
    );
}

#[test]
fn asset_dir_depth_bound_enforced() {
    let temp = TempDir::new("deep");
    let mut deep = PathBuf::from("sprites");
    for level in 0..=WALK_DEPTH_MAX {
        deep.push(format!("d{level}"));
    }
    temp.write(&deep.join("leaf.png").to_string_lossy(), b"x");
    let issues = check_asset_dir(&temp.0.join("sprites"), SPRITE_EXTENSIONS);
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert!(matches!(issues[0], AssetIssue::TooDeep(_)), "{issues:#?}");
}
