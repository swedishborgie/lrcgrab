# lrcgrab

Fetch and embed synchronized lyrics into audio files using the [LRCLib](https://lrclib.net) API.

Scans a directory recursively, extracts track metadata from audio files, searches LRCLib for matching lyrics, and either embeds them into the file tags or writes sidecar `.lrc` files.

## Features

- Recursive directory scanning
- Synced and plain lyric support (synced preferred)
- Embed lyrics into audio file metadata or write sidecar `.lrc` files
- Smart matching with scoring based on track name, artist, album, and duration
- Dry-run mode to preview matches before writing
- Instrumental track filtering
- ffmpeg remux fallback for problematic MP3 files

## Installation

```bash
cargo install --path .
```

## Usage

```bash
lrcgrab [OPTIONS] <DIRECTORY>
```

### Examples

```bash
# Preview what would be matched without making any changes
lrcgrab -n /path/to/music

# Write sidecar .lrc files (default behavior)
lrcgrab /path/to/music

# Embed lyrics directly into audio file tags
lrcgrab -e /path/to/music

# Force overwrite existing embedded lyrics
lrcgrab -e -f /path/to/music

# Include instrumental tracks
lrcgrab -i /path/to/music

# Use a custom LRCLib instance
lrcgrab -u https://custom-lrclib.example.com /path/to/music
```

### Warning: Embed Mode

The `-e` / `--embed` flag modifies audio files in place. If the embedding process fails or encounters a malformed file, it can corrupt the audio file's metadata or the file itself. Always:

1. **Run a dry run first** with `-n` to verify matches before embedding.
2. **Keep backups** of your music library before using embed mode.
3. Consider the `--ffmpeg-remux-fallback` flag for problematic MP3 files, which attempts recovery by remuxing through ffmpeg.

## Supported Formats

Audio formats supported by [lofty](https://crates.io/crates/lofty), including MP3, FLAC, Ogg Vorbis, Opus, M4A, and WavPack.

## Dependencies

- [clap](https://crates.io/crates/clap) - CLI argument parsing
- [lofty](https://crates.io/crates/lofty) - Audio metadata reading/writing
- [reqwest](https://crates.io/crates/reqwest) - HTTP client for LRCLib API
- [serde](https://crates.io/crates/serde) - JSON deserialization
- [walkdir](https://crates.io/crates/walkdir) - Recursive directory traversal
- [tracing](https://crates.io/crates/tracing) - Structured logging
