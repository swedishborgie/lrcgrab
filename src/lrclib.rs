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
