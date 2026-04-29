mod cli;
mod embed;
mod inject;
mod lrclib;
mod matcher;
mod metadata;
mod sidecar;

use cli::{Cli, Commands};
use inject::{InjectOptions, InjectOutcome};
use lrclib::Client;
use lofty::probe::Probe;
use tracing::{info, warn};

use clap::Parser;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Grab(args) => run_grab(args),
        Commands::InjectSidecars(args) => run_inject_sidecars(args),
    }
}

fn run_grab(args: cli::GrabArgs) -> anyhow::Result<()> {
    if !args.directory.exists() {
        anyhow::bail!("directory '{}' does not exist", args.directory.display());
    }

    let lrclib_client = Client::new(&args.lrclib_url);
    let mut found: u64 = 0;
    let mut matched_count: u64 = 0;
    let mut skipped: u64 = 0;

    for entry in walkdir::WalkDir::new(&args.directory) {
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
        if args.embed && !args.force && embed::can_embed(path) {
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
                    if args.ffmpeg_remux_fallback {
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
        if match_result.lyrics.instrumental && !args.instrumental {
            info!("skip (instrumental): {}", path.display());
            skipped += 1;
            continue;
        }

        let lyrics_content = match_result
            .lyrics
            .content()
            .expect("matched lyrics must have content");

        // Dry run
        if args.dry_run {
            info!(
                "dry-run match (score {:.1}): {} - {}",
                match_result.score,
                path.display(),
                match_result.lyrics.track_name
            );
            matched_count += 1;
            continue;
        }

        let is_synced = match_result.lyrics.has_synced();
        let options = InjectOptions {
            embed: args.embed,
            ffmpeg_remux_fallback: args.ffmpeg_remux_fallback,
        };

        match inject::inject(path, lyrics_content, is_synced, &options)? {
            InjectOutcome::Embedded => info!(
                "embedded: {} - {} (score {:.1})",
                path.display(),
                match_result.lyrics.track_name,
                match_result.score
            ),
            InjectOutcome::Sidecar => info!(
                "sidecar: {} - {} (score {:.1})",
                path.display(),
                match_result.lyrics.track_name,
                match_result.score
            ),
        }

        matched_count += 1;
    }

    println!(
        "done: {} files found, {} matched, {} skipped",
        found, matched_count, skipped
    );

    Ok(())
}

fn run_inject_sidecars(args: cli::InjectSidecarsArgs) -> anyhow::Result<()> {
    if !args.directory.exists() {
        anyhow::bail!("directory '{}' does not exist", args.directory.display());
    }

    let mut found: u64 = 0;
    let mut injected: u64 = 0;
    let mut skipped: u64 = 0;

    for entry in walkdir::WalkDir::new(&args.directory) {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if !metadata::is_supported(path) {
            continue;
        }

        // Check if a sidecar exists (either naming format)
        let sidecar_path = match sidecar::find_sidecar(path) {
            Some(p) => p,
            None => continue,
        };

        found += 1;

        if !embed::can_embed(path) {
            warn!(
                "skip (format does not support embedding): {}",
                path.display()
            );
            skipped += 1;
            continue;
        }

        // Check for already-embedded lyrics unless --force
        if !args.force {
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
                    if args.ffmpeg_remux_fallback {
                        info!("probe failed but ffmpeg-remux-fallback enabled, proceeding: {}: {}", path.display(), e);
                    } else {
                        warn!("skip (can't read tags): {}: {}", path.display(), e);
                        skipped += 1;
                        continue;
                    }
                }
            }
        }

        let lyrics_content = match std::fs::read_to_string(&sidecar_path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "skip (can't read sidecar {}): {}",
                    sidecar_path.display(),
                    e
                );
                skipped += 1;
                continue;
            }
        };

        let is_synced = inject::is_synced_lrc(&lyrics_content);

        if args.dry_run {
            info!(
                "dry-run: would embed {} → {}",
                sidecar_path.display(),
                path.display()
            );
            injected += 1;
            continue;
        }

        let options = InjectOptions {
            embed: true,
            ffmpeg_remux_fallback: args.ffmpeg_remux_fallback,
        };

        match inject::inject(path, &lyrics_content, is_synced, &options) {
            Ok(InjectOutcome::Embedded) => {
                info!(
                    "embedded: {} → {}",
                    sidecar_path.display(),
                    path.display()
                );
                if args.delete_sidecar {
                    if let Err(e) = std::fs::remove_file(&sidecar_path) {
                        warn!("failed to delete sidecar {}: {}", sidecar_path.display(), e);
                    } else {
                        info!("deleted sidecar: {}", sidecar_path.display());
                    }
                }
                injected += 1;
            }
            Ok(InjectOutcome::Sidecar) => {
                warn!(
                    "skip (format does not support embedding, sidecar unchanged): {}",
                    path.display()
                );
                skipped += 1;
            }
            Err(e) => {
                warn!("embed failed: {}: {}", path.display(), e);
                skipped += 1;
            }
        }
    }

    println!(
        "done: {} sidecars found, {} embedded, {} skipped",
        found, injected, skipped
    );

    Ok(())
}

