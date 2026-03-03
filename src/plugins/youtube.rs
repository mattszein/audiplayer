use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

use crate::core::action::{Track, ResultType};
use crate::plugins::traits::{Capability, Provider};

pub struct YouTubeProvider {}

impl YouTubeProvider {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Provider for YouTubeProvider {
    fn id(&self) -> &str { "youtube" }
    fn display_name(&self) -> &str { "YouTube" }
    fn capabilities(&self) -> Vec<Capability> { vec![Capability::Search] }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let search_query = format!("ytsearch{}:{}", limit, query);
        let output = tokio::process::Command::new("yt-dlp")
            .arg("--dump-json")
            .arg("--flat-playlist")
            .arg(&search_query)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("yt-dlp search failed"));
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in out_str.lines() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
                let uploader = v.get("uploader").and_then(|u| u.as_str()).unwrap_or("").to_string();
                let url = format!("https://www.youtube.com/watch?v={}", id);
                
                let duration_secs = v.get("duration").and_then(|d| d.as_f64());
                let duration = duration_secs.map(|s| Duration::from_secs_f64(s));
                
                let bitrate = v.get("abr").and_then(|b| b.as_u64()).or_else(|| {
                    v.get("tbr").and_then(|b| b.as_u64())
                }).map(|b| b as u32);

                if !id.is_empty() {
                    tracks.push(Track {
                        id,
                        title,
                        artist: uploader,
                        album: None,
                        url,
                        stream_url: None,
                        provider: "youtube".to_string(),
                        duration,
                        bitrate,
                        result_type: ResultType::Track,
                    });
                }
            }
        }

        Ok(tracks)
    }

    async fn get_stream_url(&self, _track_id: &str) -> Result<String> { Ok(String::new()) }
}
