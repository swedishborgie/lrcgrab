use std::path::{Path, PathBuf};
use std::fs;

/// Get the sidecar .lrc path for a given audio file.
pub fn lrc_path(audio_path: &Path) -> PathBuf {
    let mut lrc = audio_path
        .file_stem()
        .map(|s| PathBuf::from(s))
        .unwrap_or_default();
    lrc.set_extension("lrc");
    audio_path.parent().map(|p| p.join(&lrc)).unwrap_or(lrc)
}

/// Write lyrics content to a sidecar .lrc file.
pub fn write_sidecar(audio_path: &Path, lyrics: &str) -> anyhow::Result<()> {
    let path = lrc_path(audio_path);
    fs::write(&path, lyrics)?;
    Ok(())
}

/// Check if a sidecar .lrc file already exists for the given audio file.
pub fn sidecar_exists(audio_path: &Path) -> bool {
    lrc_path(audio_path).exists()
}
