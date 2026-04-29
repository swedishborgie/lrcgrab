use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Lyrics {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    pub id: i64,
    #[serde(rename = "name")]
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    #[serde(rename = "duration")]
    pub duration: f64,
    #[serde(rename = "instrumental")]
    pub instrumental: bool,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}

impl Lyrics {
    /// Get the best available lyrics content (synced preferred, then plain).
    pub fn content(&self) -> Option<&str> {
        if let Some(ref synced) = self.synced_lyrics {
            if !synced.is_empty() {
                return Some(synced);
            }
        }
        if let Some(ref plain) = self.plain_lyrics {
            if !plain.is_empty() {
                return Some(plain);
            }
        }
        None
    }

    pub fn has_synced(&self) -> bool {
        self.synced_lyrics.as_ref().map_or(false, |s| !s.is_empty())
    }
}

#[derive(Debug)]
pub struct Client {
    base_url: String,
    http: reqwest::blocking::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub fn search(
        &self,
        track: &str,
        artist: &str,
        album: Option<&str>,
        duration: f64,
    ) -> anyhow::Result<Vec<Lyrics>> {
        let duration_str = (duration as i64).to_string();
        let url = format!("{}/api/search", self.base_url);

        let mut query = vec![
            ("track_name", track),
            ("artist_name", artist),
            ("duration", &duration_str),
        ];

        if let Some(album_name) = album {
            query.push(("album_name", album_name));
        }

        let resp = self
            .http
            .get(&url)
            .query(&query)
            .send()?
            .error_for_status()?;

        let results = resp.json::<Vec<Lyrics>>()?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lyrics(synced: Option<&str>, plain: Option<&str>) -> Lyrics {
        Lyrics {
            id: 1,
            name: String::new(),
            track_name: "Track".to_string(),
            artist_name: "Artist".to_string(),
            album_name: None,
            duration: 200.0,
            instrumental: false,
            plain_lyrics: plain.map(|s| s.to_string()),
            synced_lyrics: synced.map(|s| s.to_string()),
        }
    }

    // --- Lyrics::has_synced ---

    #[test]
    fn has_synced_none() {
        assert!(!make_lyrics(None, None).has_synced());
    }

    #[test]
    fn has_synced_empty_string() {
        assert!(!make_lyrics(Some(""), None).has_synced());
    }

    #[test]
    fn has_synced_with_content() {
        assert!(make_lyrics(Some("[00:01.00]line"), None).has_synced());
    }

    // --- Lyrics::content ---

    #[test]
    fn content_prefers_synced_over_plain() {
        let l = make_lyrics(Some("synced text"), Some("plain text"));
        assert_eq!(l.content(), Some("synced text"));
    }

    #[test]
    fn content_falls_back_to_plain() {
        let l = make_lyrics(None, Some("plain text"));
        assert_eq!(l.content(), Some("plain text"));
    }

    #[test]
    fn content_skips_empty_synced_uses_plain() {
        let l = make_lyrics(Some(""), Some("plain text"));
        assert_eq!(l.content(), Some("plain text"));
    }

    #[test]
    fn content_both_absent_returns_none() {
        let l = make_lyrics(None, None);
        assert!(l.content().is_none());
    }

    #[test]
    fn content_both_empty_returns_none() {
        let l = make_lyrics(Some(""), Some(""));
        assert!(l.content().is_none());
    }
}
