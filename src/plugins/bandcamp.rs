use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::core::action::{Track, ResultType};
use crate::plugins::traits::{Capability, Provider};

pub struct BandcampProvider {
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct BandcampResult {
    #[serde(rename = "type")]
    _result_type: String, // "t" for track, "a" for album
    id: i64,
    name: String,
    band_name: String,
    album_name: Option<String>,
    url: String,
}

impl BandcampProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn fix_url(url: &str) -> String {
        if let Some(second_https) = url[8..].find("https://") {
            return url[8 + second_https..].to_string();
        }
        url.to_string()
    }
}

#[async_trait]
impl Provider for BandcampProvider {
    fn id(&self) -> &str { "bandcamp" }
    fn display_name(&self) -> &str { "Bandcamp" }
    fn capabilities(&self) -> Vec<Capability> { vec![Capability::Search] }

    async fn search(&self, query: &str, _limit: usize) -> Result<Vec<Track>> {
        let url = format!(
            "https://bandcamp.com/api/fuzzysearch/1/app_autocomplete?q={}&param_with_locations=true",
            urlencoding::encode(query)
        );

        let resp = self.client.get(url).send().await?;
        let text = resp.text().await?;
        
        let results: Vec<BandcampResult> = if let Ok(v) = serde_json::from_str::<Vec<BandcampResult>>(&text) {
            v
        } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(arr) = v.get("results").and_then(|r| r.as_array()) {
                arr.iter().filter_map(|item| serde_json::from_value(item.clone()).ok()).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        let tracks = results.into_iter().map(|r| {
            let result_type = if r._result_type == "a" { ResultType::Album } else { ResultType::Track };
            Track {
                id: r.id.to_string(),
                title: r.name,
                artist: r.band_name,
                album: r.album_name,
                url: Self::fix_url(&r.url),
                stream_url: None,
                provider: "bandcamp".to_string(),
                duration: None,
                bitrate: Some(128),
                result_type,
            }
        }).collect();

        Ok(tracks)
    }

    async fn get_stream_url(&self, _track_id: &str) -> Result<String> { Ok(String::new()) }
}
