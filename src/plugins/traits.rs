use async_trait::async_trait;
use anyhow::Result;
use crate::core::action::Track;

#[allow(dead_code)]
#[async_trait]
pub trait Provider: Send + Sync {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>>;
    async fn get_stream_url(&self, track: &Track) -> Result<String>;
    async fn get_album_tracks(&self, _track: &Track) -> Result<Vec<Track>> { Ok(vec![]) }
}
