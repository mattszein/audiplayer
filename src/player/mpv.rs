use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::core::action::{Action, PlayerEvent, Track};
use crate::player::Player;

pub struct MpvPlayer {
    ipc_path: String,
    action_tx: Sender<Action>,
    child: Arc<Mutex<Child>>,
    ipc_tx: mpsc::Sender<serde_json::Value>,
}

impl MpvPlayer {
    pub async fn spawn(action_tx: Sender<Action>) -> Result<Self> {
        let ipc_path = "/tmp/mpv-socket-audiplayer";
        let _ = tokio::fs::remove_file(ipc_path).await;

        let mut child = Command::new("mpv")
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
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to open mpv stdout");
        let (ipc_tx, mut ipc_rx) = mpsc::channel::<serde_json::Value>(32);

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let tx = action_tx.clone();
        let path = ipc_path.to_string();
        
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            let mut byte_buf = [0u8; 1];
            while let Ok(n) = reader.read_exact(&mut byte_buf).await {
                if n == 0 { break; }
                let b = byte_buf[0];
                if b == b'\r' || b == b'\n' {
                    let line = String::from_utf8_lossy(&buf).trim().to_string();
                    if line.starts_with("A:") {
                        let _ = tx.send(Action::MpvStdout(line)).await;
                    }
                    buf.clear();
                } else {
                    buf.push(b);
                }
            }
        });

        let tx = action_tx.clone();
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
            ipc_path: ipc_path.to_string(),
            action_tx,
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
}
