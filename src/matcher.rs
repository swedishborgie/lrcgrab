use std::collections::HashSet;

use super::lrclib::Lyrics;
use super::metadata::TrackInfo;

const MIN_SCORE: f64 = 8.0;
const DURATION_TOLERANCE: f64 = 5.0;

/// Result of matching a lyrics entry against track metadata.
#[derive(Debug)]
pub struct MatchResult {
    pub lyrics: Lyrics,
    pub score: f64,
}

/// Score and select the best lyrics match from search results.
/// If any result has synced lyrics, synced results always win over unsynced.
pub fn match_lyrics(info: &TrackInfo, results: Vec<Lyrics>) -> Option<MatchResult> {
    let mut best: Option<MatchResult> = None;

    for lyrics in results {
        let score = score_entry(info, &lyrics);
        if score >= MIN_SCORE {
            let is_synced = lyrics.has_synced();
            if let Some(ref b) = best {
                let should_replace = if is_synced && !b.lyrics.has_synced() {
                    true // synced always beats unsynced
                } else if !is_synced && b.lyrics.has_synced() {
                    false // unsynced never beats synced
                } else {
                    score > b.score
                };
                if should_replace {
                    best = Some(MatchResult {
                        lyrics,
                        score,
                    });
                }
            } else {
                best = Some(MatchResult {
                    lyrics,
                    score,
                });
            }
        }
    }

    best
}

fn score_entry(info: &TrackInfo, lyrics: &Lyrics) -> f64 {
    // Hard gate: duration must be within tolerance
    if (info.duration_secs - lyrics.duration).abs() > DURATION_TOLERANCE {
        return 0.0;
    }

    let mut score = 0.0;

    // Track name (max 10.0)
    score += string_score(&info.title, &lyrics.track_name) * 10.0;

    // Artist name (max 8.0)
    score += string_score(&info.artist, &lyrics.artist_name) * 8.0;

    // Album name (max 5.0) — only if we have album info
    if let Some(ref album) = info.album {
        if let Some(ref lyrics_album) = lyrics.album_name {
            score += string_score(album, lyrics_album) * 5.0;
        }
    }

    // Synced bonus (2.0)
    if lyrics.has_synced() {
        score += 2.0;
    }

    score
}

/// Score two strings between 0.0 and 1.0.
fn string_score(a: &str, b: &str) -> f64 {
    let lower_a = a.to_lowercase();
    let lower_b = b.to_lowercase();

    // Exact match
    if lower_a == lower_b {
        return 1.0;
    }

    // Contains match
    if lower_a.contains(&lower_b) || lower_b.contains(&lower_a) {
        return 0.6;
    }

    // Fuzzy match via Jaccard word overlap
    let score = jaccard_similarity(&lower_a, &lower_b);
    // Scale to max 0.5
    score * 0.5
}

/// Jaccard similarity on whitespace-delimited word sets.
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}
