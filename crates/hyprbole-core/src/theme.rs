use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
        }
    }
}

impl std::str::FromStr for ThemeMode {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "system" | "auto" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            value => Err(format!("invalid theme mode `{value}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    pub mode: ThemeMode,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeSnapshot {
    pub mode: ThemeMode,
    pub effective_mode: ThemeMode,
    pub tokens: ThemeTokens,
}

impl Default for ThemeSnapshot {
    fn default() -> Self {
        snapshot(&ThemeSettings::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub colors: ColorTokens,
    pub spacing: SpacingTokens,
    pub radii: RadiusTokens,
    pub typography: TypographyTokens,
    pub density: DensityTokens,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorTokens {
    pub background: String,
    pub surface: String,
    pub surface_muted: String,
    pub text: String,
    pub text_muted: String,
    pub accent: String,
    pub accent_text: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub border: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpacingTokens {
    pub xs: u32,
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
    pub xl: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadiusTokens {
    pub sm: u32,
    pub md: u32,
    pub lg: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypographyTokens {
    pub body_size: u32,
    pub title_size: u32,
    pub mono_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DensityTokens {
    pub bar_height: u32,
    pub control_row_height: u32,
    pub quick_button_height: u32,
}

pub fn snapshot(settings: &ThemeSettings) -> ThemeSnapshot {
    let effective_mode = match settings.mode {
        ThemeMode::System => ThemeMode::Dark,
        mode => mode,
    };
    ThemeSnapshot {
        mode: settings.mode,
        effective_mode,
        tokens: tokens_for_mode(effective_mode),
    }
}

pub fn tokens_for_mode(mode: ThemeMode) -> ThemeTokens {
    match mode {
        ThemeMode::Light => ThemeTokens {
            colors: ColorTokens {
                background: "#f4efe6".to_string(),
                surface: "#fff8ec".to_string(),
                surface_muted: "#eadfce".to_string(),
                text: "#211b14".to_string(),
                text_muted: "#6f6254".to_string(),
                accent: "#8f5f2a".to_string(),
                accent_text: "#fff8ec".to_string(),
                success: "#2f7d32".to_string(),
                warning: "#9a6700".to_string(),
                error: "#b3261e".to_string(),
                border: "#d5c7b7".to_string(),
            },
            spacing: default_spacing(),
            radii: default_radii(),
            typography: default_typography(),
            density: default_density(),
        },
        ThemeMode::System | ThemeMode::Dark => ThemeTokens {
            colors: ColorTokens {
                background: "#111318".to_string(),
                surface: "#1a1d24".to_string(),
                surface_muted: "#252a33".to_string(),
                text: "#f1f3f6".to_string(),
                text_muted: "#a7afbd".to_string(),
                accent: "#8fb4ff".to_string(),
                accent_text: "#0d1117".to_string(),
                success: "#7ee787".to_string(),
                warning: "#f2cc60".to_string(),
                error: "#ff7b72".to_string(),
                border: "#343a46".to_string(),
            },
            spacing: default_spacing(),
            radii: default_radii(),
            typography: default_typography(),
            density: default_density(),
        },
    }
}

fn default_spacing() -> SpacingTokens {
    SpacingTokens {
        xs: 4,
        sm: 8,
        md: 12,
        lg: 18,
        xl: 24,
    }
}

fn default_radii() -> RadiusTokens {
    RadiusTokens {
        sm: 6,
        md: 10,
        lg: 16,
    }
}

fn default_typography() -> TypographyTokens {
    TypographyTokens {
        body_size: 14,
        title_size: 20,
        mono_size: 12,
    }
}

fn default_density() -> DensityTokens {
    DensityTokens {
        bar_height: 36,
        control_row_height: 34,
        quick_button_height: 38,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_mode_parses_aliases() {
        assert_eq!("auto".parse::<ThemeMode>().unwrap(), ThemeMode::System);
        assert_eq!("light".parse::<ThemeMode>().unwrap(), ThemeMode::Light);
        assert!("blue".parse::<ThemeMode>().is_err());
    }

    #[test]
    fn theme_snapshot_round_trips_json() {
        let snapshot = snapshot(&ThemeSettings {
            mode: ThemeMode::Light,
        });
        let json = serde_json::to_string(&snapshot).unwrap();
        let decoded: ThemeSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.mode, ThemeMode::Light);
        assert_eq!(decoded.tokens.colors.background, "#f4efe6");
    }

    #[test]
    fn system_mode_currently_resolves_to_dark_tokens() {
        let snapshot = snapshot(&ThemeSettings {
            mode: ThemeMode::System,
        });
        assert_eq!(snapshot.effective_mode, ThemeMode::Dark);
        assert_eq!(snapshot.tokens.colors.background, "#111318");
    }
}
