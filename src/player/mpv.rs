use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;

use crate::core::action::{Action, PlayerEvent, Track};
use crate::player::Player;

#[allow(dead_code)]
pub struct MpvPlayer {
    _ipc_path: String,
    _action_tx: Sender<Action>,
    child: Arc<Mutex<Child>>,
    ipc_tx: mpsc::Sender<serde_json::Value>,
}

impl MpvPlayer {
    pub async fn spawn(action_tx: Sender<Action>) -> Result<Self> {
        let ipc_path = "/tmp/mpv-socket-audiplayer";
        let _ = tokio::fs::remove_file(ipc_path).await;

        let child = Command::new("mpv")
            .arg("--idle")
            .arg(format!("--input-ipc-server={}", ipc_path))
            .arg("--no-video")
            .arg("--audio-display=no")
            .arg("--no-config")
            .arg("--terminal=yes")
            .arg("--force-window=no")
            .arg("--no-osc")
            .arg("--no-osd-bar")
            .arg("--ytdl=no") 
            .arg("--msg-level=all=status")
            .arg("--term-playing-msg=A: ${time-pos} / ${duration} (${percent-pos}%) Cache: ${demuxer-cache-duration}s")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let (ipc_tx, mut ipc_rx) = mpsc::channel::<serde_json::Value>(32);

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let tx = action_tx.clone();
        let path = ipc_path.to_string();

        tokio::spawn(async move {
            loop {
                match UnixStream::connect(&path).await {
                    Ok(stream) => {
                        let (read_half, mut write_half) = stream.into_split();
                        let mut reader = BufReader::new(read_half).lines();
                        let tx_inner = tx.clone();
                        
                        let read_task = tokio::spawn(async move {
                            while let Ok(Some(line)) = reader.next_line().await {
                                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                                    handle_mpv_event(event, &tx_inner).await;
                                }
                            }
                        });

                        // Observe properties for progress bar
                        let _ = write_half.write_all(b"{\"command\": [\"observe_property\", 1, \"time-pos\"]}\n").await;
                        let _ = write_half.write_all(b"{\"command\": [\"observe_property\", 2, \"duration\"]}\n").await;
                        let _ = write_half.write_all(b"{\"command\": [\"observe_property\", 3, \"percent-pos\"]}\n").await;

                        while let Some(cmd) = ipc_rx.recv().await {
                            let mut msg = cmd.to_string();
                            msg.push('\n');
                            if write_half.write_all(msg.as_bytes()).await.is_err() {
                                break;
                            }
                        }
                        read_task.abort();
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        });

        Ok(Self {
            _ipc_path: ipc_path.to_string(),
            _action_tx: action_tx,
            child: Arc::new(Mutex::new(child)),
            ipc_tx,
        })
    }

    async fn send_command(&self, command: Vec<serde_json::Value>) -> Result<()> {
        let msg = serde_json::json!({ "command": command });
        self.ipc_tx.send(msg).await?;
        Ok(())
    }

    pub async fn stop_mpv(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        let _ = child.kill().await;
        Ok(())
    }
}

async fn handle_mpv_event(event: serde_json::Value, action_tx: &Sender<Action>) {
    if let Some(event_name) = event.get("event").and_then(|e| e.as_str()) {
        match event_name {
            "end-file" => {
                let reason = event.get("reason").and_then(|r| r.as_str()).unwrap_or("unknown");
                // Only auto-advance on natural end (eof).
                // "stop" fires on loadfile replace — ignore it to prevent cascade.
                if reason == "eof" {
                    let _ = action_tx.send(Action::PlayerEvent(PlayerEvent::TrackEnded)).await;
                } else if reason != "quit" && reason != "stop" && reason != "redirect" {
                    eprintln!("mpv playback ended with reason: {}", reason);
                }
            }
            "property-change" => {
                if let (Some(name), Some(value)) = (event.get("name").and_then(|n| n.as_str()), event.get("data")) {
                    match name {
                        "time-pos" => {
                            if let Some(pos_f) = value.as_f64() {
                                let _ = action_tx.send(Action::PlayerEvent(PlayerEvent::TimePosChanged(Duration::from_secs_f64(pos_f)))).await;
                            }
                        }
                        "duration" => {
                            if let Some(dur_f) = value.as_f64() {
                                let _ = action_tx.send(Action::PlayerEvent(PlayerEvent::DurationChanged(Duration::from_secs_f64(dur_f)))).await;
                            }
                        }
                        "percent-pos" => {
                            if let Some(per_f) = value.as_f64() {
                                let _ = action_tx.send(Action::PlayerEvent(PlayerEvent::PercentChanged(per_f as u8))).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

#[async_trait]
impl Player for MpvPlayer {
    async fn play(&self, track: &Track) -> Result<()> {
        self.send_command(vec![
            "loadfile".into(),
            track.url.clone().into(),
            "replace".into(),
        ]).await?;
        // Ensure playback starts even if it was paused
        self.resume().await?;
        Ok(())
    }

    async fn pause(&self) -> Result<()> {
        self.send_command(vec!["set_property".into(), "pause".into(), true.into()]).await?;
        Ok(())
    }

    async fn resume(&self) -> Result<()> {
        self.send_command(vec!["set_property".into(), "pause".into(), false.into()]).await?;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.send_command(vec!["stop".into()]).await?;
        Ok(())
    }

    async fn seek(&self, seconds: u64) -> Result<()> {
        self.send_command(vec!["seek".into(), seconds.into(), "absolute".into()]).await?;
        Ok(())
    }

    async fn seek_relative(&self, seconds: i64) -> Result<()> {
        self.send_command(vec!["seek".into(), seconds.into(), "relative".into()]).await?;
        Ok(())
    }

    async fn set_volume(&self, volume: u8) -> Result<()> {
        self.send_command(vec!["set_property".into(), "volume".into(), volume.into()]).await?;
        Ok(())
    }

    async fn set_mute(&self, mute: bool) -> Result<()> {
        self.send_command(vec!["set_property".into(), "mute".into(), mute.into()]).await?;
        Ok(())
    }
}
