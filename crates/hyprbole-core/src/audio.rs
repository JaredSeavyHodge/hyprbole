use std::fmt;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioAction {
    VolumeUp,
    VolumeDown,
    ToggleMute,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioSnapshot {
    pub available: bool,
    pub volume_percent: Option<u8>,
    pub muted: bool,
    pub summary: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum AudioError {
    CommandFailed(String),
    Spawn(std::io::Error),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed(message) => write!(f, "{message}"),
            Self::Spawn(err) => write!(f, "failed to run wpctl: {err}"),
        }
    }
}

impl std::error::Error for AudioError {}

pub fn summary() -> Option<String> {
    let snapshot = snapshot();
    snapshot.available.then_some(snapshot.summary)
}

pub fn snapshot() -> AudioSnapshot {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output();
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            return AudioSnapshot {
                available: false,
                summary: "Audio status unavailable".to_string(),
                error: Some(format!("failed to run wpctl: {err}")),
                ..AudioSnapshot::default()
            };
        }
    };
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return AudioSnapshot {
            available: false,
            summary: "Audio status unavailable".to_string(),
            error: Some(error),
            ..AudioSnapshot::default()
        };
    }
    let summary = String::from_utf8_lossy(&output.stdout).trim().to_string();
    AudioSnapshot {
        available: true,
        volume_percent: parse_volume_percent(&summary),
        muted: summary.to_ascii_lowercase().contains("muted"),
        summary,
        error: None,
    }
}

fn parse_volume_percent(summary: &str) -> Option<u8> {
    let value = summary.split_whitespace().find_map(|part| {
        let value = part.parse::<f32>().ok()?;
        Some((value * 100.0).round().clamp(0.0, 255.0) as u8)
    })?;
    Some(value)
}

pub fn dispatch(action: AudioAction) -> Result<(), AudioError> {
    let args = match action {
        AudioAction::VolumeUp => ["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"],
        AudioAction::VolumeDown => ["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"],
        AudioAction::ToggleMute => ["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
    };
    let output = Command::new("wpctl")
        .args(args)
        .output()
        .map_err(AudioError::Spawn)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AudioError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}
