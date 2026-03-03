use async_trait::async_trait;
use anyhow::Result;
use crate::core::action::Track;

#[derive(Debug, Clone, PartialEq)]
pub enum Capability {
    Search,
    Playlists,
    Recommendations,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn capabilities(&self) -> Vec<Capability>;

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>>;
    async fn get_stream_url(&self, track_id: &str) -> Result<String>;
}
