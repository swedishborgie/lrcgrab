use lofty::{
    config::WriteOptions,
    file::{AudioFile, FileType, TaggedFileExt},
    id3::v2::{BinaryFrame, ExtendedTextFrame, Frame, Id3v2Tag, SynchronizedTextFrame, UnsynchronizedTextFrame},
    probe::Probe,
    tag::{ItemKey, Tag, TagType, TagExt},
    TextEncoding,
};
use std::borrow::Cow;

/// TXXX descriptions for keys lost in lofty's from_tag (mapped key > 4 chars).
const LOST_KEY_DESCS: [(ItemKey, &str); 3] = [
    (ItemKey::MusicBrainzReleaseId, "MusicBrainz Album Id"),
    (ItemKey::MusicBrainzReleaseGroupId, "MusicBrainz Release Group Id"),
    (ItemKey::MusicBrainzTrackId, "MusicBrainz Release Track Id"),
];

/// Check if an audio file format supports native lyrics embedding.
pub fn can_embed(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("flac" | "mp3" | "ogg" | "opus" | "m4a")
    )
}

/// Infer FileType from file extension.
fn file_type_from_ext(path: &std::path::Path) -> Option<FileType> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mp3") => Some(FileType::Mpeg),
        Some("flac") => Some(FileType::Flac),
        Some("ogg") => Some(FileType::Vorbis),
        Some("opus") => Some(FileType::Opus),
        Some("m4a" | "mp4") => Some(FileType::Mp4),
        _ => None,
    }
}

/// Read a TaggedFile, falling back to extension-based type detection on failure.
fn read_tagged_file(path: &std::path::Path) -> anyhow::Result<lofty::file::TaggedFile> {
    // Try auto-detection first. If guess_file_type or read fails, fall back
    // to extension-based detection.
    let result = (|| {
        Probe::open(path)?.guess_file_type()?.read()
    })();

    match result {
        Ok(tagged) => Ok(tagged),
        Err(e) => {
            tracing::debug!("auto-detect failed for {:?}: {:?}, trying fallback", path, e);
            if let Some(ft) = file_type_from_ext(path) {
                tracing::debug!("falling back to extension-based detection: {:?}", ft);
                Probe::with_file_type(std::fs::File::open(path)?, ft).read().map_err(|e| {
                    tracing::debug!("fallback read also failed: {:?}", e);
                    anyhow::Error::from(e)
                })
            } else {
                anyhow::bail!("cannot determine file type");
            }
        }
    }
}

/// Remux an MP3 file with ffmpeg to create a clean copy with working ID3 tags.
fn remux_mp3(path: &std::path::Path) -> anyhow::Result<()> {
    let tmp_path = path.parent().map(|p| {
        p.join(format!(
            ".lrcgrab-remux.{}",
            path.file_name().unwrap().to_string_lossy()
        ))
    }).unwrap_or_else(|| path.with_extension("lrcgrab-remux"));

    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i", path.to_str().ok_or_else(|| anyhow::anyhow!("invalid path"))?,
            "-map_metadata", "0",
            "-c", "copy",
            "-id3v2_version", "3",
            tmp_path.to_str().ok_or_else(|| anyhow::anyhow!("invalid path"))?,
        ])
        .output()?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        anyhow::bail!("ffmpeg remux failed: {}", stderr.trim());
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Embed lyrics into an audio file using lofty.
pub fn embed_lyrics(
    path: &std::path::Path,
    lyrics_content: &str,
    is_synced: bool,
    ffmpeg_remux: bool,
) -> anyhow::Result<()> {
    let tagged_file = read_tagged_file(path)?;

    let file_type = tagged_file.file_type();

    match file_type {
        lofty::file::FileType::Mpeg => {
            embed_mp3(path, &tagged_file, lyrics_content, is_synced, ffmpeg_remux)?;
        }
        _ => {
            // For non-MP3 formats, we need a mutable TaggedFile
            let mut tagged_file = tagged_file;
            embed_generic(&mut tagged_file, lyrics_content)?;

            let tmp_path = path.parent().map(|p| {
                p.join(format!(
                    ".lrcgrab-tmp.{}",
                    path.file_name().unwrap().to_string_lossy()
                ))
            }).unwrap_or_else(|| path.with_extension("lrcgrab-tmp"));
            std::fs::copy(path, &tmp_path)?;
            tagged_file.save_to_path(&tmp_path, WriteOptions::new())?;
            std::fs::rename(&tmp_path, path)?;
        }
    }

    Ok(())
}

/// Embed into non-MP3 formats using generic Tag API.
fn embed_generic(
    tagged_file: &mut lofty::file::TaggedFile,
    lyrics: &str,
) -> anyhow::Result<()> {
    if let Some(tag) = tagged_file.primary_tag_mut() {
        tag.insert_text(ItemKey::Lyrics, lyrics.to_string());
    } else {
        let mut tag = Tag::new(tagged_file.primary_tag_type());
        tag.insert_text(ItemKey::Lyrics, lyrics.to_string());
        tagged_file.insert_tag(tag);
    }
    Ok(())
}

/// Embed lyrics into MP3 — synced uses SYLT binary frame, unsynced uses USLT.
/// Writes the Id3v2Tag directly to avoid from_tag dropping 3 MusicBrainz TXXX keys.
fn embed_mp3(
    path: &std::path::Path,
    tagged_file: &lofty::file::TaggedFile,
    lyrics: &str,
    is_synced: bool,
    ffmpeg_remux: bool,
) -> anyhow::Result<()> {
    let original_tag = tagged_file.tag(TagType::Id3v2);

    // Save values for keys that from_tag cannot round-trip
    let saved: Vec<(&str, String)> = LOST_KEY_DESCS
        .iter()
        .filter_map(|(key, desc)| {
            original_tag.and_then(|t| t.get_string(*key)).map(|v| (*desc, v.to_string()))
        })
        .collect();

    let mut id3_tag: Id3v2Tag;
    if let Some(existing) = original_tag {
        id3_tag = Id3v2Tag::from(existing.clone());
    } else {
        id3_tag = Id3v2Tag::new();
    }
    // Re-inject lost TXXX frames directly into Id3v2Tag
    for (desc, value) in &saved {
        id3_tag.insert(Frame::UserText(ExtendedTextFrame::new(
            TextEncoding::UTF8,
            *desc,
            value.clone(),
        )));
    }

    if is_synced {
        let entries = parse_lrc_entries(lyrics);
        if !entries.is_empty() {
            let stf = SynchronizedTextFrame::new(
                TextEncoding::UTF8,
                [b'e', b'n', b'g'],
                lofty::id3::v2::TimestampFormat::MS,
                lofty::id3::v2::SyncTextContentType::Lyrics,
                None,
                entries,
            );

            let bytes = stf.as_bytes(WriteOptions::new())?;
            id3_tag.insert(Frame::Binary(BinaryFrame::new(
                lofty::id3::v2::FrameId::Valid(Cow::Borrowed("SYLT")),
                bytes,
            )));
        }
    } else {
        // For unsynced, also inject USLT frame directly
        id3_tag.insert(Frame::UnsynchronizedText(
            UnsynchronizedTextFrame::new(
                TextEncoding::UTF8,
                [b'e', b'n', b'g'],
                Cow::Borrowed(""),
                Cow::Owned(lyrics.to_string()),
            )
        ));
    }

    // Write Id3v2Tag directly, bypassing from_tag which drops the 3 keys
    let tmp_path = path.parent().map(|p| {
        p.join(format!(
            ".lrcgrab-tmp.{}",
            path.file_name().unwrap().to_string_lossy()
        ))
    }).unwrap_or_else(|| path.with_extension("lrcgrab-tmp"));
    std::fs::copy(path, &tmp_path)?;
    if let Err(e) = id3_tag.save_to_path(&tmp_path, WriteOptions::new()) {
        std::fs::remove_file(&tmp_path)?;
        if ffmpeg_remux && e.to_string().contains("No format could be determined") {
            tracing::info!("save failed, remuxing with ffmpeg and retrying");
            remux_mp3(path)?;
            // Re-read after remux
            let new_tagged_file = read_tagged_file(path)?;
            let new_original_tag = new_tagged_file.tag(TagType::Id3v2);
            let mut new_id3_tag: Id3v2Tag;
            if let Some(existing) = new_original_tag {
                new_id3_tag = Id3v2Tag::from(existing.clone());
            } else {
                new_id3_tag = Id3v2Tag::new();
            }
            // Re-inject saved TXXX keys
            for (desc, value) in &saved {
                new_id3_tag.insert(Frame::UserText(ExtendedTextFrame::new(
                    TextEncoding::UTF8,
                    *desc,
                    value.clone(),
                )));
            }
            if is_synced {
                let entries = parse_lrc_entries(lyrics);
                if !entries.is_empty() {
                    let stf = SynchronizedTextFrame::new(
                        TextEncoding::UTF8,
                        [b'e', b'n', b'g'],
                        lofty::id3::v2::TimestampFormat::MS,
                        lofty::id3::v2::SyncTextContentType::Lyrics,
                        None,
                        entries,
                    );
                    let bytes = stf.as_bytes(WriteOptions::new())?;
                    new_id3_tag.insert(Frame::Binary(BinaryFrame::new(
                        lofty::id3::v2::FrameId::Valid(Cow::Borrowed("SYLT")),
                        bytes,
                    )));
                }
            } else {
                new_id3_tag.insert(Frame::UnsynchronizedText(
                    UnsynchronizedTextFrame::new(
                        TextEncoding::UTF8,
                        [b'e', b'n', b'g'],
                        Cow::Borrowed(""),
                        Cow::Owned(lyrics.to_string()),
                    )
                ));
            }
            std::fs::copy(path, &tmp_path)?;
            new_id3_tag.save_to_path(&tmp_path, WriteOptions::new())?;
            std::fs::rename(&tmp_path, path)?;
        } else {
            return Err(e.into());
        }
    } else {
        std::fs::rename(&tmp_path, path)?;
    }
    Ok(())
}

/// Parse LRC format `[mm:ss.xx]text` into (milliseconds, text) pairs.
fn parse_lrc_entries(lrc: &str) -> Vec<(u32, String)> {
    let mut entries = Vec::new();
    for line in lrc.lines() {
        if let Some(bracket_end) = line.find(']') {
            let timestamp_str = &line[..bracket_end];
            let text = line[bracket_end + 1..].trim().to_string();
            if let Some(ts) = parse_lrc_timestamp(&timestamp_str[1..]) {
                if !text.is_empty() {
                    entries.push((ts, text));
                }
            }
        }
    }
    entries
}

/// Parse LRC timestamp `mm:ss.xx` into milliseconds.
fn parse_lrc_timestamp(ts: &str) -> Option<u32> {
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let mins: f64 = parts[0].parse().ok()?;
    let secs: f64 = parts[1].parse().ok()?;
    Some(((mins * 60.0 + secs) * 1000.0) as u32)
}
