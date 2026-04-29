# lrcgrab

Fetch and embed synchronized lyrics into audio files using the [LRCLib](https://lrclib.net) API.

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

```
lrcgrab <COMMAND>
```

### Commands

| Command | Description |
|---------|-------------|
| `grab` | Search LRCLib for lyrics and inject them into audio files |
| `inject-sidecars` | Embed pre-existing `.lrc` sidecar files into audio file tags |

---

### `grab` — fetch and inject lyrics from LRCLib

Scans a directory recursively, extracts track metadata, searches LRCLib for matching lyrics, and either embeds them into the file tags or writes sidecar `.lrc` files.

```bash
lrcgrab grab [OPTIONS] <DIRECTORY>
```

| Flag | Description |
|------|-------------|
| `-e`, `--embed` | Embed lyrics into audio file tags instead of writing `.lrc` sidecars |
| `-i`, `--instrumental` | Include instrumental tracks |
| `-n`, `--dry-run` | Preview matches without writing any files |
| `-f`, `--force` | Overwrite existing embedded lyrics |
| `-u`, `--lrclib-url <URL>` | Base URL for the LRCLib API (default: `https://lrclib.net`) |
| `--ffmpeg-remux-fallback` | Remux problematic MP3s via ffmpeg before embedding |

**Examples**

```bash
# Preview what would be matched without making any changes
lrcgrab grab -n /path/to/music

# Write sidecar .lrc files next to each audio file (default)
lrcgrab grab /path/to/music

# Embed lyrics directly into audio file tags
lrcgrab grab -e /path/to/music

# Force overwrite existing embedded lyrics
lrcgrab grab -e -f /path/to/music

# Include instrumental tracks
lrcgrab grab -i /path/to/music

# Use a custom LRCLib instance
lrcgrab grab -u https://custom-lrclib.example.com /path/to/music
```

---

### `inject-sidecars` — embed existing `.lrc` files into audio tags

Scans a directory for audio files that have a companion `.lrc` sidecar and embeds the lyrics into the audio file's tags. Both `track.lrc` and `track.mp3.lrc` naming conventions are supported.

```bash
lrcgrab inject-sidecars [OPTIONS] <DIRECTORY>
```

| Flag | Description |
|------|-------------|
| `-n`, `--dry-run` | Preview without writing any changes |
| `-f`, `--force` | Overwrite existing embedded lyrics |
| `--ffmpeg-remux-fallback` | Remux problematic MP3s via ffmpeg before embedding |
| `--delete-sidecar` | Delete the `.lrc` file after successful embedding |

**Examples**

```bash
# Preview which sidecars would be embedded
lrcgrab inject-sidecars -n /path/to/music

# Embed all sidecar .lrc files into their paired audio files
lrcgrab inject-sidecars /path/to/music

# Embed and remove the .lrc files afterwards
lrcgrab inject-sidecars --delete-sidecar /path/to/music

# Force re-embed even if lyrics are already present
lrcgrab inject-sidecars -f /path/to/music
```

---

### Warning: Embed Mode

The `-e` / `--embed` flag (for `grab`) and the `inject-sidecars` command both modify audio files in place. If the process fails or encounters a malformed file, it can corrupt metadata. Always:

1. **Run a dry run first** with `-n` to verify what will change.
2. **Keep backups** of your music library before embedding.
3. Consider `--ffmpeg-remux-fallback` for problematic MP3 files.

## Sidecar Naming Conventions

When reading sidecars (`inject-sidecars`), both formats are recognised:

| Audio file | Sidecar (preferred) | Sidecar (alternate) |
|------------|---------------------|---------------------|
| `track.mp3` | `track.lrc` | `track.mp3.lrc` |
| `song.flac` | `song.lrc` | `song.flac.lrc` |

When writing sidecars (`grab`), the `track.lrc` format is always used.

## Supported Formats

Audio formats supported by [lofty](https://crates.io/crates/lofty), including MP3, FLAC, Ogg Vorbis, Opus, M4A, and WAV.

Lyrics embedding (tag writing) is supported for MP3, FLAC, Ogg Vorbis, Opus, and M4A. Other formats fall back to sidecar output.

## Dependencies

- [clap](https://crates.io/crates/clap) - CLI argument parsing
- [lofty](https://crates.io/crates/lofty) - Audio metadata reading/writing
- [reqwest](https://crates.io/crates/reqwest) - HTTP client for LRCLib API
- [serde](https://crates.io/crates/serde) - JSON deserialization
- [walkdir](https://crates.io/crates/walkdir) - Recursive directory traversal
- [tracing](https://crates.io/crates/tracing) - Structured logging
