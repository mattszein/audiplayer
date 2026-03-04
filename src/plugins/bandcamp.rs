use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::core::action::{Track, ResultType};
use crate::plugins::traits::Provider;

pub struct BandcampProvider {
    client: reqwest::Client,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BandcampResult {
    #[serde(rename = "type")]
    result_type: String, // "t" for track, "a" for album, "b" for artist
    id: i64,
    name: String,
    band_name: Option<String>,
    band_id: Option<i64>,
    album_name: Option<String>,
    album_id: Option<i64>,
    url: String,
    art_id: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct TralbumDetails {
    id: Option<i64>,
    title: String,
    band_id: Option<i64>,
    tralbum_artist: Option<String>,
    bandcamp_url: Option<String>,
    tracks: Option<Vec<BandcampTrack>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BandcampTrack {
    track_id: i64,
    title: String,
    duration: Option<f64>,
    streaming_url: Option<serde_json::Value>,
    track_num: Option<i32>,
}

impl BandcampProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Bandcamp/2.0.0 (Android; 34)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
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
    async fn search(&self, query: &str, _limit: usize) -> Result<Vec<Track>> {
        let (filter, search_query) = if query.starts_with("@") {
            let parts: Vec<&str> = query.splitn(2, ' ').collect();
            if parts.len() == 2 {
                (Some(parts[0]), parts[1])
            } else {
                (None, query)
            }
        } else {
            (None, query)
        };

        let url = format!(
            "https://bandcamp.com/api/fuzzysearch/1/app_autocomplete?q={}&param_with_locations=true",
            urlencoding::encode(search_query)
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

        let tracks = results.into_iter()
            .filter(|r| {
                if let Some(f) = filter {
                    match f {
                        "@track" | "@t" => r.result_type == "t",
                        "@album" | "@a" => r.result_type == "a",
                        "@artist" | "@b" => r.result_type == "b",
                        _ => true,
                    }
                } else {
                    true
                }
            })
            .map(|r| {
                let rt = match r.result_type.as_str() {
                    "a" => ResultType::Album,
                    "b" => ResultType::Artist,
                    _ => ResultType::Track,
                };

                let artist_id = if rt == ResultType::Artist {
                    Some(r.id.to_string())
                } else {
                    r.band_id.map(|id| id.to_string())
                };

                Track {
                    id: r.id.to_string(),
                    title: r.name,
                    artist: r.band_name.unwrap_or_else(|| "".to_string()),
                    album: r.album_name,
                    artist_id,
                    album_id: r.album_id.map(|id| id.to_string()),
                    url: Self::fix_url(&r.url),
                    stream_url: None,
                    provider: "bandcamp".to_string(),
                    duration: None,
                    bitrate: Some(128),
                    result_type: rt,
                }
            }).collect();

        Ok(tracks)
    }

    async fn get_stream_url(&self, track: &Track) -> Result<String> { 
        if let Some(stream_url) = &track.stream_url {
            return Ok(stream_url.clone());
        }

        eprintln!("Bandcamp: fetching stream URL for track_id={}, artist_id={:?}, result_type={:?}", track.id, track.artist_id, track.result_type);

        let album_tracks = self.get_album_tracks(track).await?;
        if let Some(t) = album_tracks.iter().find(|t| t.id == track.id) {
            if let Some(stream_url) = &t.stream_url {
                return Ok(stream_url.clone());
            }
        }
        
        eprintln!("Bandcamp: No stream URL found in album tracks");
        Ok(String::new())
    }

    async fn get_album_tracks(&self, track: &Track) -> Result<Vec<Track>> {
        let band_id = track.artist_id.as_ref().ok_or_else(|| anyhow::anyhow!("Missing artist_id"))?;
        let tralbum_id = &track.id;
        let tralbum_type = if track.result_type == ResultType::Album { "a" } else { "t" };

        let url = format!(
            "https://bandcamp.com/api/mobile/24/tralbum_details?band_id={}&tralbum_id={}&tralbum_type={}",
            band_id, tralbum_id, tralbum_type
        );

        let resp = self.client.get(url).send().await?;
        let text = resp.text().await?;
        
        let details: TralbumDetails = match serde_json::from_str(&text) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Bandcamp: Failed to decode tralbum_details: {}. Raw text: {}", e, text);
                return Err(anyhow::anyhow!("error decoding response body: {}", e));
            }
        };

        let mut tracks = Vec::new();
        if let Some(api_tracks) = details.tracks {
            for t in api_tracks {
                let stream_url = t.streaming_url.and_then(|v| {
                    v.get("mp3-128").and_then(|s| s.as_str()).map(|s| s.to_string())
                });

                tracks.push(Track {
                    id: t.track_id.to_string(),
                    title: t.title,
                    artist: details.tralbum_artist.clone().unwrap_or_else(|| track.artist.clone()),
                    album: Some(details.title.clone()),
                    artist_id: details.band_id.map(|id| id.to_string()),
                    album_id: details.id.map(|id| id.to_string()),
                    url: details.bandcamp_url.clone().unwrap_or_else(|| track.url.clone()),
                    stream_url,
                    provider: "bandcamp".to_string(),
                    duration: t.duration.map(|d| std::time::Duration::from_secs_f64(d)),
                    bitrate: Some(128),
                    result_type: ResultType::Track,
                });
            }
        }

        Ok(tracks)
    }
}
