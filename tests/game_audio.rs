//! Integration coverage for shipped audio under `game/assets/{music,sfx}`.
//!
//! Decoding is already proven in-crate by
//! `audio::tests::every_shipped_audio_file_decodes`; this file does not
//! repeat it. Instead it checks the packaging contract around decode:
//!
//! - the extension allowlist is *derived* from the bevy features enabled
//!   for both targets in `game/Cargo.toml` (so a dropped feature fails here);
//! - every file under the audio roots is allowlisted (or a known doc file),
//!   non-empty, bounded in size, and its header bytes match its extension;
//! - every file the game references exists and passes, and every shipped
//!   audio file is referenced (no orphans bloating the APK).
//!
//! Policy for bad files is skip-and-report, matching the game's own decode
//! guard: the scanner records a typed `AudioError` per rejected file and
//! keeps going. Only a missing/unreadable root aborts the scan.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use light_show::audio::{
    menu_track, outage_track, playing_track, results_track, sfx_path, SfxKind,
};

/// Largest shipped track is ~8.8 MB; 32 MiB leaves headroom while still
/// catching an accidental uncompressed master.
const AUDIO_FILE_BYTES_MAX: u64 = 32 * 1024 * 1024;
/// Directory walk bounds.
const SCAN_ENTRIES_MAX: usize = 1024;
const SCAN_DEPTH_MAX: usize = 4;
/// How far past an ID3v2 tag to look for the first MPEG frame sync.
const MP3_SYNC_SCAN_BYTES_MAX: usize = 64 * 1024;
/// RIFF chunks walked before giving up on finding `fmt ` and `data`.
const WAV_CHUNKS_MAX: usize = 64;
/// Non-audio files allowed to live beside the audio (attribution docs).
const DOC_EXTENSIONS: [&str; 2] = ["md", "txt"];

const SFX_KINDS: [SfxKind; 8] = [
    SfxKind::Click,
    SfxKind::Pick,
    SfxKind::Place,
    SfxKind::Alarm,
    SfxKind::Tick,
    SfxKind::Win,
    SfxKind::Lose,
    SfxKind::Dialogue,
];

/// Container formats the game can ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AudioFormat {
    Mp3,
    OggVorbis,
    Wav,
}

/// Why a file was rejected (or the scan aborted).
#[derive(Debug, PartialEq, Eq)]
enum AudioError {
    DirMissing,
    Io(String),
    TooManyEntries,
    UnknownExtension(String),
    Empty,
    TooLarge,
    /// Fewer bytes than the format's fixed header.
    TooShort,
    /// Header bytes belong to a different container than the extension.
    HeaderMismatch {
        expected: AudioFormat,
    },
    /// Header declares more bytes than the file holds.
    Truncated,
    /// Right container, codec the build cannot decode (e.g. Opus in Ogg).
    WrongCodec(&'static str),
    Malformed(&'static str),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirMissing => write!(f, "audio root missing"),
            Self::Io(message) => write!(f, "I/O: {message}"),
            Self::TooManyEntries => write!(f, "more than {SCAN_ENTRIES_MAX} entries"),
            Self::UnknownExtension(ext) => write!(f, "extension {ext:?} not allowlisted"),
            Self::Empty => write!(f, "zero-byte file"),
            Self::TooLarge => write!(f, "larger than {AUDIO_FILE_BYTES_MAX} bytes"),
            Self::TooShort => write!(f, "shorter than the format header"),
            Self::HeaderMismatch { expected } => write!(f, "header is not {expected:?}"),
            Self::Truncated => write!(f, "header declares more bytes than present"),
            Self::WrongCodec(codec) => write!(f, "undecodable codec {codec}"),
            Self::Malformed(what) => write!(f, "malformed header: {what}"),
        }
    }
}

/// Outcome of scanning an audio root: accepted files, docs, rejections.
#[derive(Debug, Default)]
struct ScanReport {
    accepted: Vec<(PathBuf, AudioFormat)>,
    docs: Vec<PathBuf>,
    rejected: Vec<(PathBuf, AudioError)>,
}

// ---------------------------------------------------------------------------
// Allowlist derived from game/Cargo.toml bevy features.
// ---------------------------------------------------------------------------

/// Quoted feature names in each `bevy = { ... features = [ ... ] }` block,
/// one set per target table. Line-based: matches the manifest's layout of
/// one feature per line, which the validation test pins.
fn bevy_feature_blocks(cargo_toml: &str) -> Vec<BTreeSet<String>> {
    let mut blocks = Vec::new();
    let mut current: Option<BTreeSet<String>> = None;
    for line in cargo_toml.lines().map(str::trim) {
        if line.starts_with("bevy = {") && line.contains("features") {
            current = Some(BTreeSet::new());
        } else if let Some(features) = current.as_mut() {
            if line.starts_with(']') {
                blocks.extend(current.take());
            } else if let Some(name) = line.strip_prefix('"').and_then(|l| l.split('"').next()) {
                features.insert(name.to_string());
            }
        }
    }
    blocks
}

/// Formats bevy can decode given one block's features.
fn decodable_formats(features: &BTreeSet<String>) -> BTreeSet<AudioFormat> {
    let has = |name: &str| features.contains(name);
    let mut formats = BTreeSet::new();
    if has("mp3") || has("symphonia-all") {
        formats.insert(AudioFormat::Mp3);
    }
    if has("vorbis") || has("symphonia-vorbis") || has("symphonia-all") {
        formats.insert(AudioFormat::OggVorbis);
    }
    if has("wav") || has("symphonia-wav") || has("symphonia-all") {
        formats.insert(AudioFormat::Wav);
    }
    formats
}

fn extension_format(extension: &str) -> Option<AudioFormat> {
    match extension {
        "mp3" => Some(AudioFormat::Mp3),
        "ogg" => Some(AudioFormat::OggVorbis),
        "wav" => Some(AudioFormat::Wav),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Header sniffers: pure functions over the file bytes.
// ---------------------------------------------------------------------------

fn u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Which container the leading bytes look like, if any.
fn detect_container(bytes: &[u8]) -> Option<AudioFormat> {
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        Some(AudioFormat::Wav)
    } else if bytes.starts_with(b"OggS") {
        Some(AudioFormat::OggVorbis)
    } else if bytes.starts_with(b"ID3") || is_mpeg_sync(bytes) {
        Some(AudioFormat::Mp3)
    } else {
        None
    }
}

/// MPEG audio frame header at the start of `bytes`: 11-bit sync, a
/// non-reserved version, Layer III, valid bitrate and sample-rate indices.
fn is_mpeg_sync(bytes: &[u8]) -> bool {
    let Some(&[b0, b1, b2, _]) = bytes.get(..4).and_then(|h| <&[u8; 4]>::try_from(h).ok()) else {
        return false;
    };
    let version_reserved = (b1 >> 3) & 0b11 == 0b01;
    let layer_iii = (b1 >> 1) & 0b11 == 0b01;
    let bitrate_index = b2 >> 4;
    let sample_rate_index = (b2 >> 2) & 0b11;
    b0 == 0xFF
        && b1 & 0xE0 == 0xE0
        && !version_reserved
        && layer_iii
        && bitrate_index != 0b1111
        && sample_rate_index != 0b11
}

fn sniff_wav(bytes: &[u8]) -> Result<(), AudioError> {
    const HEADER_BYTES: usize = 12;
    if bytes.len() < HEADER_BYTES {
        return Err(AudioError::TooShort);
    }
    let riff_bytes = u32_le(bytes, 4).ok_or(AudioError::TooShort)?;
    let declared_total = u64::from(riff_bytes) + 8;
    if declared_total > u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(AudioError::Truncated);
    }
    let (mut offset, mut saw_fmt) = (HEADER_BYTES, false);
    for _ in 0..WAV_CHUNKS_MAX {
        let id = bytes
            .get(offset..offset + 4)
            .ok_or(AudioError::Malformed("no data chunk"))?;
        let size = u32_le(bytes, offset + 4).ok_or(AudioError::Truncated)?;
        let body = offset + 8;
        let size = usize::try_from(size).map_err(|_| AudioError::Truncated)?;
        let end = body.checked_add(size).ok_or(AudioError::Truncated)?;
        if end > bytes.len() {
            return Err(AudioError::Truncated);
        }
        match id {
            b"fmt " => {
                check_wav_fmt(&bytes[body..end])?;
                saw_fmt = true;
            }
            b"data" if saw_fmt => return Ok(()),
            b"data" => return Err(AudioError::Malformed("data before fmt")),
            _ => {}
        }
        // RIFF chunks are word-aligned.
        offset = end + (size & 1);
    }
    Err(AudioError::Malformed("too many chunks"))
}

/// `fmt ` chunk: PCM/float/extensible, 1-8 channels, 8-192 kHz, sane depth.
fn check_wav_fmt(chunk: &[u8]) -> Result<(), AudioError> {
    let field = |offset: usize| -> Result<u16, AudioError> {
        let pair = chunk.get(offset..offset + 2).ok_or(AudioError::Truncated)?;
        Ok(u16::from_le_bytes([pair[0], pair[1]]))
    };
    let format_tag = field(0)?;
    let channels = field(2)?;
    let sample_rate_hz = u32_le(chunk, 4).ok_or(AudioError::Truncated)?;
    let bits_per_sample = field(14)?;
    if ![1, 3, 0xFFFE].contains(&format_tag) {
        return Err(AudioError::WrongCodec("non-PCM WAV"));
    }
    if !(1..=8).contains(&channels) || !(8_000..=192_000).contains(&sample_rate_hz) {
        return Err(AudioError::Malformed("WAV channels or sample rate"));
    }
    if ![8, 16, 24, 32].contains(&bits_per_sample) {
        return Err(AudioError::Malformed("WAV bit depth"));
    }
    Ok(())
}

/// First Ogg page must be a beginning-of-stream page whose first packet is
/// a Vorbis identification header.
fn sniff_ogg(bytes: &[u8]) -> Result<(), AudioError> {
    const PAGE_HEADER_BYTES: usize = 27;
    const BOS_FLAG: u8 = 0x02;
    if bytes.len() < PAGE_HEADER_BYTES {
        return Err(AudioError::TooShort);
    }
    if bytes[4] != 0 || bytes[5] & BOS_FLAG == 0 {
        return Err(AudioError::Malformed("first Ogg page is not version-0 BOS"));
    }
    let segment_count = usize::from(bytes[26]);
    let packet_start = PAGE_HEADER_BYTES + segment_count;
    let packet = bytes.get(packet_start..).ok_or(AudioError::Truncated)?;
    if packet.starts_with(b"\x01vorbis") {
        Ok(())
    } else if packet.starts_with(b"OpusHead") {
        Err(AudioError::WrongCodec("Opus"))
    } else if packet.starts_with(b"\x7fFLAC") {
        Err(AudioError::WrongCodec("FLAC-in-Ogg"))
    } else if packet.len() < 7 {
        Err(AudioError::Truncated)
    } else {
        Err(AudioError::WrongCodec("unknown Ogg codec"))
    }
}

/// Optional ID3v2 tag (syncsafe size, in bounds), then an MPEG Layer III
/// frame sync within `MP3_SYNC_SCAN_BYTES_MAX`.
fn sniff_mp3(bytes: &[u8]) -> Result<(), AudioError> {
    const ID3_HEADER_BYTES: usize = 10;
    const ID3_FOOTER_FLAG: u8 = 0x10;
    let mut audio_start = 0;
    if bytes.starts_with(b"ID3") {
        let header = bytes.get(..ID3_HEADER_BYTES).ok_or(AudioError::TooShort)?;
        let size_bytes = &header[6..10];
        if size_bytes.iter().any(|b| b & 0x80 != 0) {
            return Err(AudioError::Malformed("ID3 size is not syncsafe"));
        }
        let tag_bytes = size_bytes
            .iter()
            .fold(0usize, |acc, b| (acc << 7) | usize::from(*b));
        let footer = if header[5] & ID3_FOOTER_FLAG != 0 {
            ID3_HEADER_BYTES
        } else {
            0
        };
        audio_start = ID3_HEADER_BYTES + tag_bytes + footer;
        if audio_start >= bytes.len() {
            return Err(AudioError::Truncated);
        }
    }
    let scan_end = bytes
        .len()
        .min(audio_start.saturating_add(MP3_SYNC_SCAN_BYTES_MAX));
    let found = (audio_start..scan_end).any(|offset| is_mpeg_sync(&bytes[offset..]));
    if found {
        Ok(())
    } else if bytes.len() < audio_start + 4 {
        Err(AudioError::TooShort)
    } else {
        Err(AudioError::Malformed("no MPEG Layer III frame sync"))
    }
}

/// Checks `bytes` against the container its extension promises.
fn sniff(expected: AudioFormat, bytes: &[u8]) -> Result<(), AudioError> {
    if bytes.is_empty() {
        return Err(AudioError::Empty);
    }
    match detect_container(bytes) {
        Some(found) if found != expected => return Err(AudioError::HeaderMismatch { expected }),
        None if bytes.len() >= 4 => return Err(AudioError::HeaderMismatch { expected }),
        _ => {}
    }
    match expected {
        AudioFormat::Wav => sniff_wav(bytes),
        AudioFormat::OggVorbis => sniff_ogg(bytes),
        AudioFormat::Mp3 => sniff_mp3(bytes),
    }
}

// ---------------------------------------------------------------------------
// Bounded filesystem scan.
// ---------------------------------------------------------------------------

/// Reads at most `AUDIO_FILE_BYTES_MAX` bytes; larger files are rejected
/// from metadata before any read.
fn read_audio(path: &Path) -> Result<Vec<u8>, AudioError> {
    let io_err = |e: io::Error| AudioError::Io(e.to_string());
    let file = fs::File::open(path).map_err(io_err)?;
    if file.metadata().map_err(io_err)?.len() > AUDIO_FILE_BYTES_MAX {
        return Err(AudioError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(AUDIO_FILE_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(io_err)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > AUDIO_FILE_BYTES_MAX {
        return Err(AudioError::TooLarge);
    }
    Ok(bytes)
}

fn classify_file(path: &Path, allowed: &BTreeSet<AudioFormat>, report: &mut ScanReport) {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if DOC_EXTENSIONS.contains(&extension) {
        report.docs.push(path.to_path_buf());
        return;
    }
    let Some(format) = extension_format(extension).filter(|f| allowed.contains(f)) else {
        let error = AudioError::UnknownExtension(extension.to_string());
        report.rejected.push((path.to_path_buf(), error));
        return;
    };
    match read_audio(path).and_then(|bytes| sniff(format, &bytes)) {
        Ok(()) => report.accepted.push((path.to_path_buf(), format)),
        Err(error) => report.rejected.push((path.to_path_buf(), error)),
    }
}

/// Walks `root` (explicit stack, depth- and entry-bounded), classifying
/// every regular file. Per-file problems go into the report; only a
/// missing root or an exceeded bound aborts.
fn scan_audio_root(root: &Path, allowed: &BTreeSet<AudioFormat>) -> Result<ScanReport, AudioError> {
    if !root.is_dir() {
        return Err(AudioError::DirMissing);
    }
    let mut report = ScanReport::default();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut entries_seen = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| AudioError::Io(e.to_string()))?;
        for entry in entries {
            entries_seen += 1;
            if entries_seen > SCAN_ENTRIES_MAX {
                return Err(AudioError::TooManyEntries);
            }
            let path = entry.map_err(|e| AudioError::Io(e.to_string()))?.path();
            if path.is_dir() {
                if depth + 1 > SCAN_DEPTH_MAX {
                    report
                        .rejected
                        .push((path, AudioError::Malformed("too deep")));
                } else {
                    stack.push((path, depth + 1));
                }
            } else {
                classify_file(&path, allowed, &mut report);
            }
        }
    }
    report.accepted.sort();
    report.docs.sort();
    Ok(report)
}

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

fn game_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../game")
}

fn shipped_allowlist() -> BTreeSet<AudioFormat> {
    let manifest = fs::read_to_string(game_dir().join("Cargo.toml")).expect("read game/Cargo.toml");
    let blocks = bevy_feature_blocks(&manifest);
    assert_eq!(
        blocks.len(),
        2,
        "expected desktop + android bevy feature blocks"
    );
    let desktop = decodable_formats(&blocks[0]);
    let android = decodable_formats(&blocks[1]);
    // A format ships only if it decodes on every target.
    desktop.intersection(&android).copied().collect()
}

fn scan_shipped() -> ScanReport {
    let allowed = shipped_allowlist();
    let mut combined = ScanReport::default();
    for root in ["music", "sfx"] {
        let report = scan_audio_root(&game_dir().join("assets").join(root), &allowed)
            .unwrap_or_else(|e| panic!("assets/{root}: {e}"));
        combined.accepted.extend(report.accepted);
        combined.docs.extend(report.docs);
        combined.rejected.extend(report.rejected);
    }
    combined
}

/// Every asset path the game can request, relative to `game/assets/`.
fn referenced_audio_paths() -> BTreeSet<&'static str> {
    let mut paths: BTreeSet<&str> = SFX_KINDS.iter().map(|k| sfx_path(*k)).collect();
    paths.extend([menu_track(), results_track(true), results_track(false)]);
    for world in 0..=5 {
        for level_index in 0..2 {
            paths.insert(playing_track(level_index, world));
        }
    }
    paths.extend((0..2).map(outage_track));
    paths
}

fn shipped_bytes(relative: &str) -> Vec<u8> {
    read_audio(&game_dir().join("assets").join(relative)).unwrap_or_else(|e| panic!("{e}"))
}

/// Per-test scratch directory, removed (best-effort) on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(test_name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("light-show-audio-{}-{test_name}", process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear stale scratch dir");
        }
        fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create scratch subdir");
        }
        fs::write(&path, bytes).expect("write scratch file");
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _cleanup_is_best_effort = fs::remove_dir_all(&self.0);
    }
}

fn rejection_for<'a>(report: &'a ScanReport, name: &str) -> Option<&'a AudioError> {
    report
        .rejected
        .iter()
        .find(|(path, _)| path.file_name().is_some_and(|n| n == name))
        .map(|(_, error)| error)
}

// ---------------------------------------------------------------------------
// Validation tests.
// ---------------------------------------------------------------------------

#[test]
fn allowlist_matches_the_bevy_audio_features_on_every_target() {
    let expected: BTreeSet<_> = [AudioFormat::Mp3, AudioFormat::OggVorbis, AudioFormat::Wav]
        .into_iter()
        .collect();
    assert_eq!(shipped_allowlist(), expected);
    let manifest = fs::read_to_string(game_dir().join("Cargo.toml")).expect("read game/Cargo.toml");
    for block in bevy_feature_blocks(&manifest) {
        // lewton (`vorbis`) rejects a shipped track; symphonia must decode Ogg.
        assert!(!block.contains("vorbis"), "lewton `vorbis` feature is back");
        assert!(block.contains("symphonia-vorbis"));
    }
}

#[test]
fn every_shipped_audio_file_is_allowlisted_nonempty_and_well_formed() {
    let report = scan_shipped();
    assert!(
        report.rejected.is_empty(),
        "rejected shipped audio: {:#?}",
        report.rejected
    );
    assert_eq!(
        report.accepted.len(),
        referenced_audio_paths().len(),
        "{:#?}",
        report.accepted
    );
    for doc in &report.docs {
        assert!(
            doc.starts_with(game_dir().join("assets/music")),
            "stray doc {}",
            doc.display()
        );
    }
}

#[test]
fn every_referenced_path_ships_and_every_shipped_file_is_referenced() {
    let assets = game_dir().join("assets");
    let shipped: BTreeSet<String> = scan_shipped()
        .accepted
        .iter()
        .map(|(path, _)| {
            let relative = path
                .strip_prefix(&assets)
                .expect("scan stays under assets/");
            relative.to_string_lossy().replace('\\', "/")
        })
        .collect();
    let referenced: BTreeSet<String> = referenced_audio_paths()
        .into_iter()
        .map(str::to_string)
        .collect();
    let missing: Vec<_> = referenced.difference(&shipped).collect();
    let orphans: Vec<_> = shipped.difference(&referenced).collect();
    assert!(
        missing.is_empty(),
        "referenced but not shipped/valid: {missing:?}"
    );
    assert!(orphans.is_empty(), "shipped but never played: {orphans:?}");
}

#[test]
fn sfx_are_wav_and_music_is_compressed() {
    for (path, format) in scan_shipped().accepted {
        let in_sfx = path.components().any(|c| c.as_os_str() == "sfx");
        let expected_wav = format == AudioFormat::Wav;
        assert_eq!(in_sfx, expected_wav, "{} is {format:?}", path.display());
    }
}

// ---------------------------------------------------------------------------
// Adversarial tests.
// ---------------------------------------------------------------------------

#[test]
fn zero_byte_files_are_rejected_for_every_format() {
    for format in [AudioFormat::Mp3, AudioFormat::OggVorbis, AudioFormat::Wav] {
        assert_eq!(sniff(format, &[]), Err(AudioError::Empty), "{format:?}");
    }
}

#[test]
fn truncated_headers_are_rejected_without_panicking() {
    let wav = shipped_bytes(sfx_path(SfxKind::Click));
    let ogg = shipped_bytes(outage_track(1));
    let mp3 = shipped_bytes(menu_track());
    for cut in [1, 3, 4, 11, 12, 20, 36, 44] {
        assert!(
            sniff(AudioFormat::Wav, &wav[..cut]).is_err(),
            "wav cut at {cut}"
        );
    }
    for cut in [1, 4, 26, 27, 30] {
        assert!(
            sniff(AudioFormat::OggVorbis, &ogg[..cut]).is_err(),
            "ogg cut at {cut}"
        );
    }
    for cut in [1, 3, 9, 10, 64] {
        assert!(
            sniff(AudioFormat::Mp3, &mp3[..cut]).is_err(),
            "mp3 cut at {cut}"
        );
    }
    // Lop the tail off a real WAV: RIFF size now overstates the file.
    assert_eq!(
        sniff(AudioFormat::Wav, &wav[..wav.len() - 1]),
        Err(AudioError::Truncated)
    );
}

#[test]
fn content_under_the_wrong_extension_is_a_header_mismatch() {
    let wav = shipped_bytes(sfx_path(SfxKind::Win));
    let ogg = shipped_bytes(outage_track(1));
    let mp3 = shipped_bytes(results_track(true));
    let cases = [
        (AudioFormat::Mp3, &wav),
        (AudioFormat::OggVorbis, &wav),
        (AudioFormat::Wav, &ogg),
        (AudioFormat::Mp3, &ogg),
        (AudioFormat::Wav, &mp3),
        (AudioFormat::OggVorbis, &mp3),
    ];
    for (expected, bytes) in cases {
        assert_eq!(
            sniff(expected, bytes),
            Err(AudioError::HeaderMismatch { expected })
        );
    }
    let text = b"#!/bin/sh\necho not audio\n";
    assert_eq!(
        sniff(AudioFormat::Wav, text),
        Err(AudioError::HeaderMismatch {
            expected: AudioFormat::Wav
        })
    );
}

#[test]
fn right_container_with_undecodable_codec_is_rejected() {
    let mut opus = shipped_bytes(outage_track(1));
    let packet_start = 27 + usize::from(opus[26]);
    opus[packet_start..packet_start + 8].copy_from_slice(b"OpusHead");
    assert_eq!(
        sniff(AudioFormat::OggVorbis, &opus),
        Err(AudioError::WrongCodec("Opus"))
    );

    let mut adpcm = shipped_bytes(sfx_path(SfxKind::Tick));
    let fmt_body = 12 + 8;
    assert_eq!(&adpcm[12..16], b"fmt ", "fixture: fmt chunk expected first");
    adpcm[fmt_body..fmt_body + 2].copy_from_slice(&2u16.to_le_bytes()); // MS ADPCM
    assert_eq!(
        sniff(AudioFormat::Wav, &adpcm),
        Err(AudioError::WrongCodec("non-PCM WAV"))
    );
}

#[test]
fn malformed_id3_and_missing_frame_sync_are_rejected() {
    let not_syncsafe = b"ID3\x04\x00\x00\x00\x00\x00\x80rest-of-file";
    assert_eq!(
        sniff(AudioFormat::Mp3, not_syncsafe),
        Err(AudioError::Malformed("ID3 size is not syncsafe"))
    );
    let tag_past_eof = b"ID3\x04\x00\x00\x00\x00\x7f\x7fshort";
    assert_eq!(
        sniff(AudioFormat::Mp3, tag_past_eof),
        Err(AudioError::Truncated)
    );
    let mut no_sync = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
    no_sync.extend(std::iter::repeat_n(0u8, 4096));
    assert_eq!(
        sniff(AudioFormat::Mp3, &no_sync),
        Err(AudioError::Malformed("no MPEG Layer III frame sync"))
    );
    // Reserved MPEG version bits must not count as a sync.
    assert!(!is_mpeg_sync(&[0xFF, 0xEB, 0x90, 0x00]));
    assert!(is_mpeg_sync(&[0xFF, 0xFB, 0x90, 0x00]));
}

#[test]
fn scanner_skips_and_reports_bad_files_and_keeps_going() {
    let scratch = ScratchDir::new("mixed");
    let good = shipped_bytes(sfx_path(SfxKind::Click));
    scratch.write("good.wav", &good);
    scratch.write("nested/deeper/also_good.wav", &good);
    scratch.write("empty.wav", b"");
    scratch.write("track.flac", b"fLaC\0\0\0\x22");
    scratch.write("SHOUT.WAV", &good);
    scratch.write("noext", &good);
    scratch.write("wav_as.mp3", &good);
    scratch.write("CREDITS.md", b"docs are allowed");
    let report = scan_audio_root(&scratch.0, &shipped_allowlist()).expect("scratch root exists");
    assert_eq!(report.accepted.len(), 2, "{report:#?}");
    assert_eq!(report.docs.len(), 1);
    assert_eq!(
        rejection_for(&report, "empty.wav"),
        Some(&AudioError::Empty)
    );
    let unknown = |ext: &str| Some(AudioError::UnknownExtension(ext.to_string()));
    assert_eq!(
        rejection_for(&report, "track.flac"),
        unknown("flac").as_ref()
    );
    assert_eq!(rejection_for(&report, "SHOUT.WAV"), unknown("WAV").as_ref());
    assert_eq!(rejection_for(&report, "noext"), unknown("").as_ref());
    let mismatch = AudioError::HeaderMismatch {
        expected: AudioFormat::Mp3,
    };
    assert_eq!(rejection_for(&report, "wav_as.mp3"), Some(&mismatch));
    assert_eq!(report.rejected.len(), 5);
}

#[test]
fn missing_root_oversized_file_and_excessive_depth_are_typed() {
    let missing = std::env::temp_dir().join(format!("light-show-no-audio-{}", process::id()));
    assert_eq!(
        scan_audio_root(&missing, &shipped_allowlist()).err(),
        Some(AudioError::DirMissing)
    );

    let scratch = ScratchDir::new("bounds");
    let huge = scratch.write("huge.wav", b"");
    // Sparse file: costs no disk, but metadata reports > the cap.
    fs::File::options()
        .write(true)
        .open(&huge)
        .and_then(|file| file.set_len(AUDIO_FILE_BYTES_MAX + 1))
        .expect("extend sparse file");
    scratch.write("a/b/c/d/e/too_deep.wav", b"");
    let report = scan_audio_root(&scratch.0, &shipped_allowlist()).expect("scratch root exists");
    assert_eq!(
        rejection_for(&report, "huge.wav"),
        Some(&AudioError::TooLarge)
    );
    assert_eq!(
        rejection_for(&report, "e"),
        Some(&AudioError::Malformed("too deep"))
    );
    assert!(report.accepted.is_empty());
}

#[test]
fn dropping_a_feature_removes_its_format_from_the_allowlist() {
    let manifest = fs::read_to_string(game_dir().join("Cargo.toml")).expect("read game/Cargo.toml");
    let without_wav = manifest.replacen("\"wav\",", "", 1);
    assert_ne!(
        without_wav, manifest,
        "fixture: \"wav\" feature line not found"
    );
    let blocks = bevy_feature_blocks(&without_wav);
    let desktop = decodable_formats(&blocks[0]);
    let android = decodable_formats(&blocks[1]);
    let allowed: BTreeSet<_> = desktop.intersection(&android).copied().collect();
    assert!(
        !allowed.contains(&AudioFormat::Wav),
        "WAV must drop if one target loses it"
    );
    assert!(decodable_formats(&BTreeSet::new()).is_empty());
}
