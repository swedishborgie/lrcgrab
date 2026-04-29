use std::path::PathBuf;

/// Fetch and embed synchronized lyrics into audio files
#[derive(clap::Parser, Debug)]
#[command(name = "lrcgrab", version, about, long_about = None)]
pub struct Cli {
    /// Directory to recursively scan for audio files
    #[arg(required = true)]
    pub directory: PathBuf,

    /// Embed lyrics into audio files instead of writing sidecar .lrc files
    #[arg(short, long)]
    pub embed: bool,

    /// Include instrumental tracks
    #[arg(short, long)]
    pub instrumental: bool,

    /// Base URL for the lrclib API
    #[arg(short = 'u', long, default_value = "https://lrclib.net")]
    pub lrclib_url: String,

    /// Preview matches without writing any files
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Force embed, overwriting existing lyrics
    #[arg(short, long)]
    pub force: bool,

    /// Use ffmpeg to remux MP3 files that lofty can't write to
    #[arg(long)]
    pub ffmpeg_remux_fallback: bool,
}
