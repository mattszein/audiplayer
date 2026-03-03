pub mod traits;
pub mod bandcamp;
pub mod youtube;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc::Sender, Semaphore};

pub use traits::{Provider, Capability};
pub use bandcamp::BandcampProvider;
pub use youtube::YouTubeProvider;
use crate::core::action::{Action, PluginResult, Track};

pub struct PluginManager {
    providers: HashMap<String, Box<dyn Provider>>,
    action_tx: Sender<Action>,
    semaphore: Arc<Semaphore>,
}

impl PluginManager {
    pub fn new(action_tx: Sender<Action>) -> Self {
        let mut providers: HashMap<String, Box<dyn Provider>> = HashMap::new();
        providers.insert("bandcamp".to_string(), Box::new(BandcampProvider::new()));
        providers.insert("youtube".to_string(), Box::new(YouTubeProvider::new()));
        Self {
            providers,
            action_tx,
            semaphore: Arc::new(Semaphore::new(3)),
        }
    }

    pub async fn handle_search(&self, provider_id: &str, query: String) {
        if let Some(provider) = self.providers.get(provider_id) {
            match provider.search(&query, 20).await {
                Ok(results) => {
                    let _ = self.action_tx.send(Action::PluginResponse {
                        id: provider_id.to_string(),
                        result: PluginResult::Search(results),
                    }).await;
                }
                Err(e) => {
                    let msg = format!("Search error ({}): {}", provider_id, e);
                    let _ = self.action_tx.send(Action::Log(msg.clone())).await;
                    let _ = self.action_tx.send(Action::PluginResponse {
                        id: provider_id.to_string(),
                        result: PluginResult::Error(msg),
                    }).await;
                }
            }
        }
    }

    pub async fn resolve_stream_url(&self, track: Track) {
        let url = track.url.clone();
        let track_id = track.id.clone();
        let tx = self.action_tx.clone();
        let sem = self.semaphore.clone();

        tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };

            // -f bestaudio/best + --format-sort abr,bitrate ensures we get the highest quality audio stream URL
            let output = tokio::process::Command::new("yt-dlp")
                .arg("--print")
                .arg("%(url)s")
                .arg("--print")
                .arg("%(duration_string)s")
                .arg("--print")
                .arg("%(abr)s")
                .arg("-f")
                .arg("bestaudio/best")
                .arg("--format-sort")
                .arg("abr,bitrate")
                .arg("--no-playlist")
                .arg(&url)
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() => {
                    let out_str = String::from_utf8_lossy(&out.stdout);
                    let mut lines = out_str.lines();
                    
                    let stream_url = lines.next().unwrap_or("").trim().to_string();
                    let duration_str = lines.next().unwrap_or("").trim();
                    let bitrate_str = lines.next().unwrap_or("").trim();
                    
                    let duration = parse_duration(duration_str);
                    // Handle cases where bitrate might be "NA" or non-numeric
                    let bitrate = bitrate_str.parse::<f64>().ok().map(|b| b as u32);

                    if !stream_url.is_empty() {
                        let _ = tx.send(Action::PluginResponse {
                            id: "resolve".to_string(),
                            result: PluginResult::StreamUrl {
                                track_id,
                                url: stream_url,
                                duration,
                                bitrate,
                            },
                        }).await;
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let _ = tx.send(Action::Log(format!("yt-dlp error for {}: {}", url, err))).await;
                }
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("Failed to run yt-dlp: {}", e))).await;
                }
            }
        });
    }
}

fn parse_duration(s: &str) -> Option<Duration> {
    if s == "NA" || s.is_empty() { return None; }
    let parts: Vec<&str> = s.split(':').collect();
    let mut secs = 0u64;
    match parts.len() {
        3 => {
            let h = parts[0].parse::<u64>().ok()?;
            let m = parts[1].parse::<u64>().ok()?;
            let s = parts[2].parse::<u64>().ok()?;
            secs = h * 3600 + m * 60 + s;
        }
        2 => {
            let m = parts[0].parse::<u64>().ok()?;
            let s = parts[1].parse::<u64>().ok()?;
            secs = m * 60 + s;
        }
        1 => {
            secs = parts[0].parse::<u64>().ok()?;
        }
        _ => return None,
    }
    Some(Duration::from_secs(secs))
}
