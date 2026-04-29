mod cli;
mod embed;
mod lrclib;
mod matcher;
mod metadata;
mod sidecar;

use cli::Cli;
use lrclib::Client;
use lofty::probe::Probe;
use tracing::{info, warn};

use clap::Parser;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if !cli.directory.exists() {
        anyhow::bail!("directory '{}' does not exist", cli.directory.display());
    }

    let lrclib_client = Client::new(&cli.lrclib_url);
    let mut found: u64 = 0;
    let mut matched_count: u64 = 0;
    let mut skipped: u64 = 0;

    for entry in walkdir::WalkDir::new(&cli.directory) {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if !metadata::is_supported(path) {
            continue;
        }

        found += 1;

        // Check for existing sidecar
        if sidecar::sidecar_exists(path) {
            info!("skip (sidecar exists): {}", path.display());
            skipped += 1;
            continue;
        }

        // Extract metadata
        let track_info = match metadata::extract(path) {
            Ok(info) => info,
            Err(e) => {
                warn!("skip (extract error): {}: {}", path.display(), e);
                skipped += 1;
                continue;
            }
        };

        // Skip if no title or artist
        if track_info.title.is_empty() || track_info.artist.is_empty() {
            info!(
                "skip (missing metadata): {} (title='{}', artist='{}')",
                path.display(),
                track_info.title,
                track_info.artist
            );
            skipped += 1;
            continue;
        }

        // In embed mode, check if file already has lyrics (unless --force)
        if cli.embed && !cli.force && embed::can_embed(path) {
            match Probe::open(path)
                .and_then(|p| Ok(p.guess_file_type()?))
                .and_then(|p| Ok(p.read()?))
            {
                Ok(tagged_file) => {
                    if metadata::has_lyrics(&tagged_file) {
                        info!("skip (has lyrics): {}", path.display());
                        skipped += 1;
                        continue;
                    }
                }
                Err(e) => {
                    if cli.ffmpeg_remux_fallback {
                        info!("probe failed but ffmpeg-remux-fallback enabled, proceeding: {}: {}", path.display(), e);
                    } else {
                        warn!(
                            "skip (can't read tags): {}: {}",
                            path.display(),
                            e
                        );
                        skipped += 1;
                        continue;
                    }
                }
            }
        }

        // Search lrclib
        let results = match lrclib_client.search(
            &track_info.title,
            &track_info.artist,
            track_info.album.as_deref(),
            track_info.duration_secs,
        ) {
            Ok(r) => r,
            Err(e) => {
                warn!("skip (search error): {}: {}", path.display(), e);
                skipped += 1;
                continue;
            }
        };

        // Match results
        let match_result = match matcher::match_lyrics(&track_info, results) {
            Some(m) => m,
            None => {
                info!("skip (no match): {}", path.display());
                skipped += 1;
                continue;
            }
        };

        // Check instrumental
        if match_result.lyrics.instrumental && !cli.instrumental {
            info!("skip (instrumental): {}", path.display());
            skipped += 1;
            continue;
        }

        let lyrics_content = match_result
            .lyrics
            .content()
            .expect("matched lyrics must have content");

        // Dry run
        if cli.dry_run {
            info!(
                "dry-run match (score {:.1}): {} - {}",
                match_result.score,
                path.display(),
                match_result.lyrics.track_name
            );
            matched_count += 1;
            continue;
        }

        // Store lyrics
        let is_synced = match_result.lyrics.has_synced();

        if cli.embed && embed::can_embed(path) {
            match embed::embed_lyrics(path, lyrics_content, is_synced, cli.ffmpeg_remux_fallback) {
                Ok(()) => {
                    info!(
                        "embedded: {} - {} (score {:.1})",
                        path.display(),
                        match_result.lyrics.track_name,
                        match_result.score
                    );
                }
                Err(e) => {
                    warn!(
                        "embed failed, falling back to sidecar: {}: {}",
                        path.display(),
                        e
                    );
                    sidecar::write_sidecar(path, lyrics_content)?;
                    info!(
                        "sidecar fallback: {} - {}",
                        path.display(),
                        match_result.lyrics.track_name
                    );
                }
            }
        } else {
            sidecar::write_sidecar(path, lyrics_content)?;
            info!(
                "sidecar: {} - {} (score {:.1})",
                path.display(),
                match_result.lyrics.track_name,
                match_result.score
            );
        }

        matched_count += 1;
    }

    println!(
        "done: {} files found, {} matched, {} skipped",
        found, matched_count, skipped
    );

    Ok(())
}
