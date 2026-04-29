use std::path::Path;
use tracing::{info, warn};

use crate::{embed, sidecar};

/// Options controlling how lyrics are injected into an audio file.
pub struct InjectOptions {
    /// Embed lyrics into the audio file's tags when the format supports it.
    pub embed: bool,
    /// Allow ffmpeg remux fallback for MP3 files lofty cannot write to.
    pub ffmpeg_remux_fallback: bool,
}

/// Outcome of a single inject operation.
pub enum InjectOutcome {
    Embedded,
    Sidecar,
}

/// Inject lyrics into an audio file.
///
/// If `options.embed` is true and the format supports embedding, the lyrics are
/// written into the audio file's tags (with a fallback to sidecar on error).
/// Otherwise a `.lrc` sidecar is written next to the file.
pub fn inject(
    path: &Path,
    lyrics_content: &str,
    is_synced: bool,
    options: &InjectOptions,
) -> anyhow::Result<InjectOutcome> {
    if options.embed && embed::can_embed(path) {
        match embed::embed_lyrics(path, lyrics_content, is_synced, options.ffmpeg_remux_fallback) {
            Ok(()) => return Ok(InjectOutcome::Embedded),
            Err(e) => {
                warn!(
                    "embed failed, falling back to sidecar: {}: {}",
                    path.display(),
                    e
                );
                sidecar::write_sidecar(path, lyrics_content)?;
                info!("sidecar fallback: {}", path.display());
                return Ok(InjectOutcome::Sidecar);
            }
        }
    }

    sidecar::write_sidecar(path, lyrics_content)?;
    Ok(InjectOutcome::Sidecar)
}

/// Returns true if the lyrics content appears to be time-synced LRC format.
pub fn is_synced_lrc(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.starts_with('[') && line.contains(':') && line.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synced_lrc_detects_timestamp_line() {
        assert!(is_synced_lrc("[00:01.23]Hello world"));
    }

    #[test]
    fn synced_lrc_detects_in_multiline() {
        let content = "plain line\n[01:23.45]synced line\nanother plain";
        assert!(is_synced_lrc(content));
    }

    #[test]
    fn synced_lrc_plain_text_returns_false() {
        assert!(!is_synced_lrc("These are just plain lyrics\nno timestamps here"));
    }

    #[test]
    fn synced_lrc_empty_returns_false() {
        assert!(!is_synced_lrc(""));
    }

    #[test]
    fn synced_lrc_bracket_without_timestamp_returns_false() {
        // Has '[' and ':' but no '.'
        assert!(!is_synced_lrc("[no:timestamp]text"));
    }
}
