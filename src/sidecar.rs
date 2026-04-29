use std::path::{Path, PathBuf};
use std::fs;

/// Get the `stem.lrc` sidecar path for a given audio file (e.g. `test.mp3` → `test.lrc`).
pub fn lrc_path(audio_path: &Path) -> PathBuf {
    let mut lrc = audio_path
        .file_stem()
        .map(|s| PathBuf::from(s))
        .unwrap_or_default();
    lrc.set_extension("lrc");
    audio_path.parent().map(|p| p.join(&lrc)).unwrap_or(lrc)
}

/// Get the `filename.ext.lrc` sidecar path for a given audio file (e.g. `test.mp3` → `test.mp3.lrc`).
pub fn lrc_path_dotted(audio_path: &Path) -> PathBuf {
    let filename = audio_path
        .file_name()
        .map(|s| {
            let mut name = s.to_os_string();
            name.push(".lrc");
            PathBuf::from(name)
        })
        .unwrap_or_default();
    audio_path.parent().map(|p| p.join(&filename)).unwrap_or(filename)
}

/// Find the sidecar .lrc file for an audio file, checking both naming formats.
/// Prefers `stem.lrc` over `filename.ext.lrc` if both exist.
pub fn find_sidecar(audio_path: &Path) -> Option<PathBuf> {
    let stem = lrc_path(audio_path);
    if stem.exists() {
        return Some(stem);
    }
    let dotted = lrc_path_dotted(audio_path);
    if dotted.exists() {
        return Some(dotted);
    }
    None
}

/// Write lyrics content to a sidecar .lrc file (uses `stem.lrc` format).
pub fn write_sidecar(audio_path: &Path, lyrics: &str) -> anyhow::Result<()> {
    let path = lrc_path(audio_path);
    fs::write(&path, lyrics)?;
    Ok(())
}

/// Check if a sidecar .lrc file already exists for the given audio file (either format).
pub fn sidecar_exists(audio_path: &Path) -> bool {
    find_sidecar(audio_path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // --- lrc_path ---

    #[test]
    fn lrc_path_basic() {
        let result = lrc_path(Path::new("/music/track.mp3"));
        assert_eq!(result, Path::new("/music/track.lrc"));
    }

    #[test]
    fn lrc_path_flac() {
        let result = lrc_path(Path::new("/a/b/song.flac"));
        assert_eq!(result, Path::new("/a/b/song.lrc"));
    }

    // --- lrc_path_dotted ---

    #[test]
    fn lrc_path_dotted_basic() {
        let result = lrc_path_dotted(Path::new("/music/track.mp3"));
        assert_eq!(result, Path::new("/music/track.mp3.lrc"));
    }

    #[test]
    fn lrc_path_dotted_flac() {
        let result = lrc_path_dotted(Path::new("/a/b/song.flac"));
        assert_eq!(result, Path::new("/a/b/song.flac.lrc"));
    }

    // --- write_sidecar / sidecar_exists / find_sidecar ---

    #[test]
    fn write_and_detect_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.mp3");
        // Create a dummy audio file so the path exists in the right directory
        std::fs::write(&audio, b"dummy").unwrap();

        assert!(!sidecar_exists(&audio));

        write_sidecar(&audio, "[00:01.00]Hello").unwrap();
        assert!(sidecar_exists(&audio));

        // Should find stem.lrc format
        let found = find_sidecar(&audio).unwrap();
        assert_eq!(found, lrc_path(&audio));
    }

    #[test]
    fn find_sidecar_prefers_stem_over_dotted() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.mp3");
        std::fs::write(&audio, b"dummy").unwrap();

        let stem = lrc_path(&audio);
        let dotted = lrc_path_dotted(&audio);

        // Write both formats
        std::fs::write(&stem, "stem").unwrap();
        std::fs::write(&dotted, "dotted").unwrap();

        let found = find_sidecar(&audio).unwrap();
        assert_eq!(found, stem, "stem format should be preferred");
    }

    #[test]
    fn find_sidecar_falls_back_to_dotted() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.mp3");
        std::fs::write(&audio, b"dummy").unwrap();

        let dotted = lrc_path_dotted(&audio);
        std::fs::write(&dotted, "dotted only").unwrap();

        let found = find_sidecar(&audio).unwrap();
        assert_eq!(found, dotted);
    }

    #[test]
    fn find_sidecar_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.mp3");
        std::fs::write(&audio, b"dummy").unwrap();

        assert!(find_sidecar(&audio).is_none());
    }
}
