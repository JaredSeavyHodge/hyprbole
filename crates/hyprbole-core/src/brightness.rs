use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrightnessAction {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrightnessSnapshot {
    pub available: bool,
    pub percent: Option<u8>,
    pub summary: String,
    pub error: Option<String>,
}

pub fn dispatch(action: BrightnessAction) -> Result<(), String> {
    let value = match action {
        BrightnessAction::Increase => "5%+",
        BrightnessAction::Decrease => "5%-",
    };
    let output = Command::new("brightnessctl")
        .args(["set", value])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn summary() -> Option<String> {
    let snapshot = snapshot();
    snapshot.available.then_some(snapshot.summary)
}

pub fn snapshot() -> BrightnessSnapshot {
    let output = Command::new("brightnessctl").arg("info").output();
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            return BrightnessSnapshot {
                available: false,
                summary: "Brightness status unavailable".to_string(),
                error: Some(format!("failed to run brightnessctl: {err}")),
                ..BrightnessSnapshot::default()
            };
        }
    };
    if !output.status.success() {
        return BrightnessSnapshot {
            available: false,
            summary: "Brightness status unavailable".to_string(),
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            ..BrightnessSnapshot::default()
        };
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let summary = output
        .lines()
        .find(|line| line.contains("Current brightness"))
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "Brightness status unavailable".to_string());
    BrightnessSnapshot {
        available: summary != "Brightness status unavailable",
        percent: parse_percent(&summary),
        summary,
        error: None,
    }
}

pub fn parse_percent(input: &str) -> Option<u8> {
    let start = input.find('(')? + 1;
    let end = input[start..].find('%')? + start;
    input[start..end].trim().parse::<u8>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brightness_percent() {
        assert_eq!(parse_percent("Current brightness: 512 (50%)"), Some(50));
    }

    #[test]
    fn missing_brightness_percent_returns_none() {
        assert_eq!(parse_percent("Brightness status unavailable"), None);
    }
}
