pub mod mpv;

use async_trait::async_trait;
use crate::core::action::Track;
use anyhow::Result;

#[async_trait]
pub trait Player: Send + Sync {
    async fn play(&self, track: &Track) -> Result<()>;
    async fn pause(&self) -> Result<()>;
    async fn resume(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn seek(&self, seconds: u64) -> Result<()>;
}
