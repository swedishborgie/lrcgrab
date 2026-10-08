use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

/// Extracted metadata from an audio file.
#[derive(Debug)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
}

/// File extensions that lofty can read.
const SUPPORTED_EXTENSIONS: &[&str] = &["flac", "mp3", "m4a", "ogg", "opus", "wav", "aac", "mp4"];

pub fn is_supported(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.iter().any(|&s| s.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn supported_lowercase_extensions() {
        for ext in &["flac", "mp3", "m4a", "ogg", "opus", "wav", "aac"] {
            let filename = format!("track.{ext}");
            let path = std::path::Path::new(&filename);
            assert!(is_supported(path), "{ext} should be supported");
        }
    }

    #[test]
    fn supported_uppercase_extensions() {
        for ext in &["FLAC", "MP3", "M4A", "OGG", "OPUS", "WAV", "AAC"] {
            let filename = format!("track.{ext}");
            let path = std::path::Path::new(&filename);
            assert!(is_supported(path), "{ext} (uppercase) should be supported");
        }
    }

    #[test]
    fn unsupported_extensions() {
        for ext in &["txt", "pdf", "exe", "lrc", "jpg"] {
            let filename = format!("file.{ext}");
            let path = std::path::Path::new(&filename);
            assert!(!is_supported(path), "{ext} should not be supported");
        }
    }

    #[test]
    fn no_extension_returns_false() {
        assert!(!is_supported(Path::new("noextension")));
    }
}

/// Extract metadata from an audio file using lofty.
pub fn extract(path: &std::path::Path) -> anyhow::Result<TrackInfo> {
    let tagged_file = Probe::open(path)?.guess_file_type()?.read()?;

    let props = tagged_file.properties();
    let duration_secs = props.duration().as_secs_f64();

    let mut title = String::new();
    let mut artist = String::new();
    let mut album = None::<String>;

    for tag in tagged_file.tags() {
        if tag.title().is_some() && title.is_empty() {
            title = tag.title().map(|s| s.to_string()).unwrap_or_default();
        }
        if tag.artist().is_some() && artist.is_empty() {
            artist = tag.artist().map(|s| s.to_string()).unwrap_or_default();
        }
        if tag.album().is_some() && album.is_none() {
            album = tag.album().map(|s| s.to_string());
        }
    }

    Ok(TrackInfo {
        title,
        artist,
        album,
        duration_secs,
    })
}

/// Check if the file already has lyrics embedded.
pub fn has_lyrics(tagged_file: &lofty::file::TaggedFile) -> bool {
    use lofty::tag::ItemKey;

    // Minimum content length to consider it real lyrics (avoids false positives
    // from USLT frames that only store a language code like "eng").
    const MIN_LYRICS_LEN: usize = 100;

    for tag in tagged_file.tags() {
        for lyrics_str in tag.get_strings(ItemKey::Lyrics) {
            if lyrics_str.len() >= MIN_LYRICS_LEN {
                return true;
            }
        }
        for lyrics_str in tag.get_strings(ItemKey::UnsyncLyrics) {
            if lyrics_str.len() >= MIN_LYRICS_LEN {
                return true;
            }
        }
    }

    // Also check for SYLT in ID3v2 (MP3 synced lyrics)
    if tagged_file.contains_tag_type(lofty::tag::TagType::Id3v2) {
        let generic = tagged_file
            .tag(lofty::tag::TagType::Id3v2)
            .map(|t| t.clone());
        if let Some(g) = generic {
            let id3_tag: lofty::id3::v2::Id3v2Tag = g.into();
            for frame in &id3_tag {
                if frame.id_str() == "SYLT" {
                    return true;
                }
            }
        }
    }

    false
}
