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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lrclib::Lyrics;
    use crate::metadata::TrackInfo;

    fn make_lyrics(track: &str, artist: &str, duration: f64, synced: Option<&str>, plain: Option<&str>) -> Lyrics {
        Lyrics {
            id: 1,
            name: String::new(),
            track_name: track.to_string(),
            artist_name: artist.to_string(),
            album_name: None,
            duration,
            instrumental: false,
            plain_lyrics: plain.map(|s| s.to_string()),
            synced_lyrics: synced.map(|s| s.to_string()),
        }
    }

    fn make_track(title: &str, artist: &str, duration: f64) -> TrackInfo {
        TrackInfo {
            title: title.to_string(),
            artist: artist.to_string(),
            album: None,
            duration_secs: duration,
        }
    }

    // --- jaccard_similarity ---

    #[test]
    fn jaccard_identical_sets() {
        assert!((jaccard_similarity("hello world", "hello world") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint_sets() {
        assert!((jaccard_similarity("foo bar", "baz qux") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_partial_overlap() {
        // "hello world" ∩ "hello there" = {"hello"}, ∪ = {"hello","world","there"} → 1/3
        let score = jaccard_similarity("hello world", "hello there");
        assert!((score - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn jaccard_both_empty() {
        assert!((jaccard_similarity("", "") - 0.0).abs() < f64::EPSILON);
    }

    // --- string_score ---

    #[test]
    fn string_score_exact_match() {
        assert!((string_score("Hello", "hello") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn string_score_contains_match() {
        assert!((string_score("Hello World", "hello") - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn string_score_contains_reverse() {
        assert!((string_score("hello", "Hello World") - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn string_score_jaccard_fallback() {
        // No exact or contains match: Jaccard scaled to max 0.5
        let score = string_score("foo bar", "foo baz");
        assert!(score > 0.0 && score <= 0.5);
    }

    #[test]
    fn string_score_no_overlap() {
        let score = string_score("apple orange", "banana grape");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    // --- match_lyrics ---

    #[test]
    fn match_lyrics_empty_returns_none() {
        let info = make_track("Song", "Artist", 200.0);
        assert!(match_lyrics(&info, vec![]).is_none());
    }

    #[test]
    fn match_lyrics_all_below_threshold() {
        let info = make_track("Song", "Artist", 200.0);
        // Duration mismatch → score 0 → below MIN_SCORE
        let bad = make_lyrics("Song", "Artist", 999.0, None, Some("lyrics"));
        assert!(match_lyrics(&info, vec![bad]).is_none());
    }

    #[test]
    fn match_lyrics_single_result_above_threshold() {
        let info = make_track("Song", "Artist", 200.0);
        let good = make_lyrics("Song", "Artist", 201.0, None, Some("lyrics"));
        let result = match_lyrics(&info, vec![good]);
        assert!(result.is_some());
    }

    #[test]
    fn match_lyrics_synced_beats_unsynced_lower_score() {
        let info = make_track("Song", "Artist", 200.0);
        // Unsynced with perfect title+artist match
        let unsynced = make_lyrics("Song", "Artist", 201.0, None, Some("plain lyrics text"));
        // Synced with slightly worse artist name — but synced bonus should still win
        let synced = make_lyrics("Song", "Artist", 201.0, Some("[00:01.00]line"), None);
        let result = match_lyrics(&info, vec![unsynced, synced]).unwrap();
        assert!(result.lyrics.has_synced(), "synced should win over unsynced");
    }

    #[test]
    fn match_lyrics_higher_score_wins_same_sync_type() {
        let info = make_track("Exact Match Title", "Exact Artist", 200.0);
        let worse = make_lyrics("Totally Different", "Exact Artist", 201.0, None, Some("plain"));
        let better = make_lyrics("Exact Match Title", "Exact Artist", 201.0, None, Some("plain"));
        let result = match_lyrics(&info, vec![worse, better]).unwrap();
        assert_eq!(result.lyrics.track_name, "Exact Match Title");
    }
}
