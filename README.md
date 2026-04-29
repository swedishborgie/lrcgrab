# lrcgrab-rs

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
# Write sidecar .lrc files (default)
lrcgrab-rs /path/to/music

# Embed lyrics into audio file tags
lrcgrab-rs -e /path/to/music

# Dry run to preview matches
lrcgrab-rs -n /path/to/music

# Include instrumental tracks
lrcgrab-rs -i /path/to/music

# Force overwrite existing lyrics
lrcgrab-rs -e -f /path/to/music

# Use a custom lrclib instance
lrcgrab-rs -u https://custom-lrclib.example.com /path/to/music
```

## Supported Formats

Audio formats supported by [lofty](https://crates.io/crates/lofty), including MP3, FLAC, Ogg Vorbis, Opus, M4A, and WavPack.

## Dependencies

- [clap](https://crates.io/crates/clap) - CLI argument parsing
- [lofty](https://crates.io/crates/lofty) - Audio metadata reading/writing
- [reqwest](https://crates.io/crates/reqwest) - HTTP client for LRCLib API
- [serde](https://crates.io/crates/serde) - JSON deserialization
- [walkdir](https://crates.io/crates/walkdir) - Recursive directory traversal
- [tracing](https://crates.io/crates/tracing) - Structured logging
