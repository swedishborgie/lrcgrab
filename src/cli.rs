use std::path::PathBuf;

/// Fetch and embed synchronized lyrics into audio files
#[derive(clap::Parser, Debug)]
#[command(name = "lrcgrab", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Search lrclib for lyrics and inject them into audio files
    Grab(GrabArgs),
    /// Embed pre-existing .lrc sidecar files into audio file tags
    InjectSidecars(InjectSidecarsArgs),
}

/// Arguments for the `grab` subcommand.
#[derive(clap::Args, Debug)]
pub struct GrabArgs {
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

/// Arguments for the `inject-sidecars` subcommand.
#[derive(clap::Args, Debug)]
pub struct InjectSidecarsArgs {
    /// Directory to recursively scan for audio files with .lrc sidecars
    #[arg(required = true)]
    pub directory: PathBuf,

    /// Preview without writing any changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Overwrite existing embedded lyrics
    #[arg(short, long)]
    pub force: bool,

    /// Use ffmpeg to remux MP3 files that lofty can't write to
    #[arg(long)]
    pub ffmpeg_remux_fallback: bool,

    /// Delete the .lrc sidecar file after successful embedding
    #[arg(long)]
    pub delete_sidecar: bool,
}

