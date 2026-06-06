use std::env;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipMode {
    RespectUserConfig,
    RuntimeOwned,
    PersistedOwned,
}

impl OwnershipMode {
    pub fn controls_runtime(self) -> bool {
        matches!(self, Self::RuntimeOwned | Self::PersistedOwned)
    }
}

impl fmt::Display for OwnershipMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RespectUserConfig => write!(f, "respect_user_config"),
            Self::RuntimeOwned => write!(f, "runtime_owned"),
            Self::PersistedOwned => write!(f, "persisted_owned"),
        }
    }
}

impl std::str::FromStr for OwnershipMode {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "respect" | "respect_user_config" | "user" => Ok(Self::RespectUserConfig),
            "runtime" | "runtime_owned" | "hyprbole" => Ok(Self::RuntimeOwned),
            "persisted" | "persisted_owned" => Ok(Self::PersistedOwned),
            value => Err(SettingsError::InvalidOwnershipMode(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellSubsystem {
    Keybindings,
    Monitors,
    Workspaces,
    Audio,
    Brightness,
    Appearance,
    Notifications,
    Power,
}

impl fmt::Display for ShellSubsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keybindings => write!(f, "keybindings"),
            Self::Monitors => write!(f, "monitors"),
            Self::Workspaces => write!(f, "workspaces"),
            Self::Audio => write!(f, "audio"),
            Self::Brightness => write!(f, "brightness"),
            Self::Appearance => write!(f, "appearance"),
            Self::Notifications => write!(f, "notifications"),
            Self::Power => write!(f, "power"),
        }
    }
}

impl std::str::FromStr for ShellSubsystem {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "keybindings" | "keys" | "binds" => Ok(Self::Keybindings),
            "monitors" | "monitor" => Ok(Self::Monitors),
            "workspaces" | "workspace" => Ok(Self::Workspaces),
            "audio" => Ok(Self::Audio),
            "brightness" | "backlight" => Ok(Self::Brightness),
            "appearance" | "theme" => Ok(Self::Appearance),
            "notifications" | "notification" => Ok(Self::Notifications),
            "power" | "session" => Ok(Self::Power),
            value => Err(SettingsError::InvalidSubsystem(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellSettings {
    pub ownership: OwnershipSettings,
    pub autostart: AutostartSettings,
    pub ui: UiSettings,
    pub notifications: crate::notifications::NotificationSettings,
    pub theme: crate::theme::ThemeSettings,
}

impl Default for ShellSettings {
    fn default() -> Self {
        Self {
            ownership: OwnershipSettings::default(),
            autostart: AutostartSettings::default(),
            ui: UiSettings::default(),
            notifications: crate::notifications::NotificationSettings::default(),
            theme: crate::theme::ThemeSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub bar: BarSettings,
    pub osd: OsdSettings,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            bar: BarSettings::default(),
            osd: OsdSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OsdSettings {
    pub enabled: bool,
    pub edge: BarEdge,
    pub width: u32,
    pub height: u32,
    pub margin: u32,
    pub timeout_ms: u32,
    pub font_size: u32,
    pub radius: u32,
    pub opacity: u32,
}

impl Default for OsdSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            edge: BarEdge::Top,
            width: 420,
            height: 96,
            margin: 64,
            timeout_ms: 1800,
            font_size: 12,
            radius: 14,
            opacity: 94,
        }
    }
}

impl OsdSettings {
    pub const MIN_WIDTH: u32 = 240;
    pub const MAX_WIDTH: u32 = 720;
    pub const MIN_HEIGHT: u32 = 48;
    pub const MAX_HEIGHT: u32 = 180;
    pub const MAX_MARGIN: u32 = 160;
    pub const MIN_TIMEOUT_MS: u32 = 500;
    pub const MAX_TIMEOUT_MS: u32 = 10_000;
    pub const MIN_FONT_SIZE: u32 = 8;
    pub const MAX_FONT_SIZE: u32 = 20;
    pub const MAX_RADIUS: u32 = 40;
    pub const MIN_OPACITY: u32 = 40;
    pub const MAX_OPACITY: u32 = 100;

    pub fn validate(&self) -> Result<(), SettingsError> {
        if !(Self::MIN_WIDTH..=Self::MAX_WIDTH).contains(&self.width) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.width must be {}..={}",
                Self::MIN_WIDTH,
                Self::MAX_WIDTH
            )));
        }
        if !(Self::MIN_HEIGHT..=Self::MAX_HEIGHT).contains(&self.height) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.height must be {}..={}",
                Self::MIN_HEIGHT,
                Self::MAX_HEIGHT
            )));
        }
        if self.margin > Self::MAX_MARGIN {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.margin must be <= {}",
                Self::MAX_MARGIN
            )));
        }
        if !(Self::MIN_TIMEOUT_MS..=Self::MAX_TIMEOUT_MS).contains(&self.timeout_ms) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.timeout_ms must be {}..={}",
                Self::MIN_TIMEOUT_MS,
                Self::MAX_TIMEOUT_MS
            )));
        }
        if !(Self::MIN_FONT_SIZE..=Self::MAX_FONT_SIZE).contains(&self.font_size) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.font_size must be {}..={}",
                Self::MIN_FONT_SIZE,
                Self::MAX_FONT_SIZE
            )));
        }
        if self.radius > Self::MAX_RADIUS {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.radius must be <= {}",
                Self::MAX_RADIUS
            )));
        }
        if !(Self::MIN_OPACITY..=Self::MAX_OPACITY).contains(&self.opacity) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "osd.opacity must be {}..={}",
                Self::MIN_OPACITY,
                Self::MAX_OPACITY
            )));
        }
        Ok(())
    }

    pub fn set_field(&mut self, field: &str, value: &str) -> Result<(), SettingsError> {
        match field {
            "osd.enabled" | "enabled" => self.enabled = parse_bool(value)?,
            "osd.edge" | "edge" => self.edge = value.parse()?,
            "osd.width" | "width" => self.width = parse_u32_field("osd.width", value)?,
            "osd.height" | "height" => self.height = parse_u32_field("osd.height", value)?,
            "osd.margin" | "margin" => self.margin = parse_u32_field("osd.margin", value)?,
            "osd.timeout_ms" | "timeout_ms" | "timeout" => {
                self.timeout_ms = parse_u32_field("osd.timeout_ms", value)?
            }
            "osd.font_size" | "font_size" => {
                self.font_size = parse_u32_field("osd.font_size", value)?
            }
            "osd.radius" | "radius" => self.radius = parse_u32_field("osd.radius", value)?,
            "osd.opacity" | "opacity" => self.opacity = parse_u32_field("osd.opacity", value)?,
            other => {
                return Err(SettingsError::InvalidUiSetting(format!(
                    "unknown OSD setting `{other}`"
                )));
            }
        }
        self.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarSettings {
    pub enabled: bool,
    pub monitor: BarMonitor,
    pub edge: BarEdge,
    pub height: u32,
    pub margin: u32,
    pub padding: u32,
    pub font_size: u32,
    pub radius: u32,
    pub opacity: u32,
    pub widgets: Vec<BarWidget>,
}

impl Default for BarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor: BarMonitor::Focused,
            edge: BarEdge::Top,
            height: 36,
            margin: 0,
            padding: 12,
            font_size: 11,
            radius: 10,
            opacity: 96,
            widgets: BarSettings::DEFAULT_WIDGETS.to_vec(),
        }
    }
}

impl BarSettings {
    pub const MIN_HEIGHT: u32 = 24;
    pub const MAX_HEIGHT: u32 = 96;
    pub const MAX_MARGIN: u32 = 64;
    pub const MAX_PADDING: u32 = 48;
    pub const MIN_FONT_SIZE: u32 = 8;
    pub const MAX_FONT_SIZE: u32 = 18;
    pub const MAX_RADIUS: u32 = 32;
    pub const MIN_OPACITY: u32 = 40;
    pub const MAX_OPACITY: u32 = 100;
    pub const DEFAULT_WIDGETS: &'static [BarWidget] = &[
        BarWidget::Workspaces,
        BarWidget::FocusedWindow,
        BarWidget::DaemonStatus,
        BarWidget::Audio,
        BarWidget::Brightness,
        BarWidget::Layout,
        BarWidget::QuickToggle,
    ];

    pub fn validate(&self) -> Result<(), SettingsError> {
        if !(Self::MIN_HEIGHT..=Self::MAX_HEIGHT).contains(&self.height) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.height must be {}..={}",
                Self::MIN_HEIGHT,
                Self::MAX_HEIGHT
            )));
        }
        if self.margin > Self::MAX_MARGIN {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.margin must be <= {}",
                Self::MAX_MARGIN
            )));
        }
        if self.padding > Self::MAX_PADDING {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.padding must be <= {}",
                Self::MAX_PADDING
            )));
        }
        if !(Self::MIN_FONT_SIZE..=Self::MAX_FONT_SIZE).contains(&self.font_size) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.font_size must be {}..={}",
                Self::MIN_FONT_SIZE,
                Self::MAX_FONT_SIZE
            )));
        }
        if self.radius > Self::MAX_RADIUS {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.radius must be <= {}",
                Self::MAX_RADIUS
            )));
        }
        if !(Self::MIN_OPACITY..=Self::MAX_OPACITY).contains(&self.opacity) {
            return Err(SettingsError::InvalidUiSetting(format!(
                "bar.opacity must be {}..={}",
                Self::MIN_OPACITY,
                Self::MAX_OPACITY
            )));
        }
        if let BarMonitor::Named(name) = &self.monitor
            && name.trim().is_empty()
        {
            return Err(SettingsError::InvalidUiSetting(
                "bar.monitor cannot be empty".to_string(),
            ));
        }
        if self.widgets.is_empty() {
            return Err(SettingsError::InvalidUiSetting(
                "bar.widgets cannot be empty".to_string(),
            ));
        }
        let mut seen = Vec::new();
        for widget in &self.widgets {
            if seen.contains(widget) {
                return Err(SettingsError::InvalidUiSetting(format!(
                    "bar.widgets contains duplicate `{widget}`"
                )));
            }
            seen.push(*widget);
        }
        Ok(())
    }

    pub fn set_field(&mut self, field: &str, value: &str) -> Result<(), SettingsError> {
        match field {
            "bar.enabled" | "enabled" => self.enabled = parse_bool(value)?,
            "bar.monitor" | "monitor" => self.monitor = value.parse()?,
            "bar.edge" | "edge" => self.edge = value.parse()?,
            "bar.height" | "height" => self.height = parse_u32_field("bar.height", value)?,
            "bar.margin" | "margin" => self.margin = parse_u32_field("bar.margin", value)?,
            "bar.padding" | "padding" => self.padding = parse_u32_field("bar.padding", value)?,
            "bar.font_size" | "font_size" => {
                self.font_size = parse_u32_field("bar.font_size", value)?
            }
            "bar.radius" | "radius" => self.radius = parse_u32_field("bar.radius", value)?,
            "bar.opacity" | "opacity" => self.opacity = parse_u32_field("bar.opacity", value)?,
            "bar.widgets" | "widgets" => self.widgets = parse_bar_widgets(value)?,
            other => {
                return Err(SettingsError::InvalidUiSetting(format!(
                    "unknown bar setting `{other}`"
                )));
            }
        }
        self.validate()
    }

    pub fn widget_enabled(&self, widget: BarWidget) -> bool {
        self.widgets.contains(&widget)
    }

    pub fn widgets_csv(&self) -> String {
        self.widgets
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn reset_widgets(&mut self) {
        self.widgets = Self::DEFAULT_WIDGETS.to_vec();
    }

    pub fn move_widget(&mut self, widget: BarWidget, direction: WidgetMove) -> bool {
        let Some(index) = self.widgets.iter().position(|existing| *existing == widget) else {
            return false;
        };
        let target = match direction {
            WidgetMove::Up if index > 0 => index - 1,
            WidgetMove::Down if index + 1 < self.widgets.len() => index + 1,
            _ => return false,
        };
        self.widgets.swap(index, target);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetMove {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode", content = "name")]
pub enum BarMonitor {
    Focused,
    Primary,
    Named(String),
}

impl Default for BarMonitor {
    fn default() -> Self {
        Self::Focused
    }
}

impl fmt::Display for BarMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Focused => write!(f, "focused"),
            Self::Primary => write!(f, "primary"),
            Self::Named(name) => write!(f, "{name}"),
        }
    }
}

impl std::str::FromStr for BarMonitor {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        match input {
            "focused" => Ok(Self::Focused),
            "primary" => Ok(Self::Primary),
            "" => Err(SettingsError::InvalidUiSetting(
                "bar.monitor cannot be empty".to_string(),
            )),
            name => Ok(Self::Named(name.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarEdge {
    Top,
    Bottom,
}

impl Default for BarEdge {
    fn default() -> Self {
        Self::Top
    }
}

impl fmt::Display for BarEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Top => write!(f, "top"),
            Self::Bottom => write!(f, "bottom"),
        }
    }
}

impl std::str::FromStr for BarEdge {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "top" => Ok(Self::Top),
            "bottom" => Ok(Self::Bottom),
            value => Err(SettingsError::InvalidUiSetting(format!(
                "invalid bar.edge `{value}`"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarWidget {
    Workspaces,
    FocusedWindow,
    Audio,
    Brightness,
    Layout,
    Notifications,
    DaemonStatus,
    Clock,
    QuickToggle,
}

impl fmt::Display for BarWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspaces => write!(f, "workspaces"),
            Self::FocusedWindow => write!(f, "focused_window"),
            Self::Audio => write!(f, "audio"),
            Self::Brightness => write!(f, "brightness"),
            Self::Layout => write!(f, "layout"),
            Self::Notifications => write!(f, "notifications"),
            Self::DaemonStatus => write!(f, "daemon_status"),
            Self::Clock => write!(f, "clock"),
            Self::QuickToggle => write!(f, "quick_toggle"),
        }
    }
}

impl std::str::FromStr for BarWidget {
    type Err = SettingsError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "workspaces" | "workspace" | "ws" => Ok(Self::Workspaces),
            "focused_window" | "window" | "win" => Ok(Self::FocusedWindow),
            "audio" => Ok(Self::Audio),
            "brightness" | "backlight" => Ok(Self::Brightness),
            "layout" => Ok(Self::Layout),
            "notifications" | "notification" | "dnd" => Ok(Self::Notifications),
            "daemon_status" | "status" | "daemon" => Ok(Self::DaemonStatus),
            "clock" | "time" => Ok(Self::Clock),
            "quick_toggle" | "quick" => Ok(Self::QuickToggle),
            value => Err(SettingsError::InvalidUiSetting(format!(
                "invalid bar widget `{value}`"
            ))),
        }
    }
}

fn parse_bool(value: &str) -> Result<bool, SettingsError> {
    match value.trim() {
        "true" | "yes" | "on" | "1" | "enabled" => Ok(true),
        "false" | "no" | "off" | "0" | "disabled" => Ok(false),
        value => Err(SettingsError::InvalidUiSetting(format!(
            "invalid boolean `{value}`"
        ))),
    }
}

fn parse_u32_field(field: &str, value: &str) -> Result<u32, SettingsError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|err| SettingsError::InvalidUiSetting(format!("invalid {field} `{value}`: {err}")))
}

fn parse_bar_widgets(value: &str) -> Result<Vec<BarWidget>, SettingsError> {
    let widgets = value
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if widgets.is_empty() {
        return Err(SettingsError::InvalidUiSetting(
            "bar.widgets cannot be empty".to_string(),
        ));
    }
    Ok(widgets)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AutostartSettings {
    pub apps: Vec<AutostartApp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutostartApp {
    pub name: String,
    pub command: String,
    pub enabled: bool,
}

impl Default for AutostartApp {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            enabled: true,
        }
    }
}

impl ShellSettings {
    pub fn safe_fallback() -> Self {
        let mut settings = Self::default();
        settings.ownership.keybindings = OwnershipMode::RespectUserConfig;
        settings
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        self.ui.bar.validate()?;
        self.ui.osd.validate()
    }

    pub fn ownership(&self, subsystem: ShellSubsystem) -> OwnershipMode {
        match subsystem {
            ShellSubsystem::Keybindings => self.ownership.keybindings,
            ShellSubsystem::Monitors => self.ownership.monitors,
            ShellSubsystem::Workspaces => self.ownership.workspaces,
            ShellSubsystem::Audio => self.ownership.audio,
            ShellSubsystem::Brightness => self.ownership.brightness,
            ShellSubsystem::Appearance => self.ownership.appearance,
            ShellSubsystem::Notifications => self.ownership.notifications,
            ShellSubsystem::Power => self.ownership.power,
        }
    }

    pub fn set_ownership(&mut self, subsystem: ShellSubsystem, mode: OwnershipMode) {
        match subsystem {
            ShellSubsystem::Keybindings => self.ownership.keybindings = mode,
            ShellSubsystem::Monitors => self.ownership.monitors = mode,
            ShellSubsystem::Workspaces => self.ownership.workspaces = mode,
            ShellSubsystem::Audio => self.ownership.audio = mode,
            ShellSubsystem::Brightness => self.ownership.brightness = mode,
            ShellSubsystem::Appearance => self.ownership.appearance = mode,
            ShellSubsystem::Notifications => self.ownership.notifications = mode,
            ShellSubsystem::Power => self.ownership.power = mode,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OwnershipSettings {
    pub keybindings: OwnershipMode,
    pub monitors: OwnershipMode,
    pub workspaces: OwnershipMode,
    pub audio: OwnershipMode,
    pub brightness: OwnershipMode,
    pub appearance: OwnershipMode,
    pub notifications: OwnershipMode,
    pub power: OwnershipMode,
}

impl Default for OwnershipSettings {
    fn default() -> Self {
        Self {
            keybindings: OwnershipMode::RuntimeOwned,
            monitors: OwnershipMode::RespectUserConfig,
            workspaces: OwnershipMode::RespectUserConfig,
            audio: OwnershipMode::RuntimeOwned,
            brightness: OwnershipMode::RuntimeOwned,
            appearance: OwnershipMode::RespectUserConfig,
            notifications: OwnershipMode::RespectUserConfig,
            power: OwnershipMode::RuntimeOwned,
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    MissingEnvironment(&'static str),
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    Decode {
        path: PathBuf,
        source: serde_json::Error,
    },
    Encode(serde_json::Error),
    InvalidOwnershipMode(String),
    InvalidSubsystem(String),
    InvalidUiSetting(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironment(name) => write!(f, "{name} is not set"),
            Self::CreateDir { path, source } => write!(f, "create {}: {source}", path.display()),
            Self::Read { path, source } => write!(f, "read {}: {source}", path.display()),
            Self::Write { path, source } => write!(f, "write {}: {source}", path.display()),
            Self::Decode { path, source } => write!(f, "decode {}: {source}", path.display()),
            Self::Encode(source) => write!(f, "encode settings: {source}"),
            Self::InvalidOwnershipMode(value) => write!(f, "invalid ownership mode `{value}`"),
            Self::InvalidSubsystem(value) => write!(f, "invalid subsystem `{value}`"),
            Self::InvalidUiSetting(value) => write!(f, "invalid UI setting: {value}"),
        }
    }
}

impl std::error::Error for SettingsError {}

pub fn settings_path_from_env() -> Result<PathBuf, SettingsError> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or(SettingsError::MissingEnvironment("XDG_CONFIG_HOME or HOME"))?;
    Ok(config_home.join("hyprbole").join("settings.json"))
}

pub trait SettingsStore {
    fn load_or_default(&self) -> Result<ShellSettings, SettingsError>;
    fn save(&self, settings: &ShellSettings) -> Result<(), SettingsError>;
    fn path(&self) -> &std::path::Path;
}

#[derive(Debug, Clone)]
pub struct FileSettingsStore {
    path: PathBuf,
}

impl FileSettingsStore {
    pub fn from_env() -> Result<Self, SettingsError> {
        Ok(Self::new(settings_path_from_env()?))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl SettingsStore for FileSettingsStore {
    fn load_or_default(&self) -> Result<ShellSettings, SettingsError> {
        load_or_default_from_path(self.path.clone())
    }

    fn save(&self, settings: &ShellSettings) -> Result<(), SettingsError> {
        save_to_path(self.path.clone(), settings)
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

pub fn load_or_default() -> Result<ShellSettings, SettingsError> {
    FileSettingsStore::from_env()?.load_or_default()
}

pub fn load_or_default_from_path(path: PathBuf) -> Result<ShellSettings, SettingsError> {
    if !path.exists() {
        return Ok(ShellSettings::default());
    }
    let contents = std::fs::read_to_string(&path).map_err(|source| SettingsError::Read {
        path: path.clone(),
        source,
    })?;
    match serde_json::from_str::<ShellSettings>(&contents) {
        Ok(settings) => match settings.validate() {
            Ok(()) => Ok(settings),
            Err(source) => {
                eprintln!(
                    "failed to validate {}; using safe fallback settings: {source}",
                    path.display()
                );
                Ok(ShellSettings::safe_fallback())
            }
        },
        Err(source) => {
            eprintln!(
                "failed to decode {}; using safe fallback settings: {source}",
                path.display()
            );
            Ok(ShellSettings::safe_fallback())
        }
    }
}

pub fn save(settings: &ShellSettings) -> Result<(), SettingsError> {
    FileSettingsStore::from_env()?.save(settings)
}

pub fn save_to_path(path: PathBuf, settings: &ShellSettings) -> Result<(), SettingsError> {
    settings.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SettingsError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let contents = serde_json::to_string_pretty(settings).map_err(SettingsError::Encode)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, contents + "\n").map_err(|source| SettingsError::Write {
        path: tmp_path.clone(),
        source,
    })?;
    std::fs::rename(&tmp_path, &path).map_err(|source| SettingsError::Write { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hyprbole-settings-{name}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn ownership_modes_parse_aliases() {
        assert!(matches!(
            "respect".parse::<OwnershipMode>(),
            Ok(OwnershipMode::RespectUserConfig)
        ));
        assert!(matches!(
            "runtime".parse::<OwnershipMode>(),
            Ok(OwnershipMode::RuntimeOwned)
        ));
        assert!(matches!(
            "persisted".parse::<OwnershipMode>(),
            Ok(OwnershipMode::PersistedOwned)
        ));
    }

    #[test]
    fn settings_save_and_load_roundtrip() {
        let path = test_path("roundtrip");
        let mut settings = ShellSettings::default();
        settings.set_ownership(ShellSubsystem::Monitors, OwnershipMode::RuntimeOwned);
        settings.ui.bar.edge = BarEdge::Bottom;
        settings.ui.osd.edge = BarEdge::Bottom;
        save_to_path(path.clone(), &settings).unwrap();
        let loaded = load_or_default_from_path(path.clone()).unwrap();
        assert_eq!(loaded.ownership.monitors, OwnershipMode::RuntimeOwned);
        assert_eq!(loaded.ui.bar.edge, BarEdge::Bottom);
        assert_eq!(loaded.ui.osd.edge, BarEdge::Bottom);
        assert_eq!(loaded.notifications.history_limit, 50);
        assert_eq!(loaded.theme.mode, crate::theme::ThemeMode::Dark);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn file_settings_store_reports_path_and_roundtrips() {
        let path = test_path("store-roundtrip");
        let store = FileSettingsStore::new(path.clone());
        let mut settings = ShellSettings::default();
        settings.ui.bar.height = 44;

        assert_eq!(store.path(), path.as_path());
        store.save(&settings).unwrap();
        let loaded = store.load_or_default().unwrap();
        assert_eq!(loaded.ui.bar.height, 44);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn atomic_save_failure_leaves_existing_settings_intact() {
        let path = test_path("atomic-failure");
        let tmp_path = path.with_extension("json.tmp");
        let mut original = ShellSettings::default();
        original.ui.bar.height = 36;
        save_to_path(path.clone(), &original).unwrap();
        std::fs::create_dir(&tmp_path).unwrap();

        let mut updated = original.clone();
        updated.ui.bar.height = 56;
        let result = save_to_path(path.clone(), &updated);

        assert!(result.is_err());
        let loaded = load_or_default_from_path(path.clone()).unwrap();
        assert_eq!(loaded.ui.bar.height, 36);
        let _ = std::fs::remove_dir(tmp_path);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bar_settings_validate_bounds_and_widgets() {
        let mut bar = BarSettings::default();
        bar.height = BarSettings::MIN_HEIGHT - 1;
        assert!(bar.validate().is_err());
        bar.height = BarSettings::MIN_HEIGHT;
        bar.font_size = BarSettings::MAX_FONT_SIZE + 1;
        assert!(bar.validate().is_err());
        bar.font_size = BarSettings::MIN_FONT_SIZE;
        bar.opacity = BarSettings::MIN_OPACITY - 1;
        assert!(bar.validate().is_err());
        bar.opacity = BarSettings::MAX_OPACITY;
        bar.widgets = vec![BarWidget::Audio, BarWidget::Audio];
        assert!(bar.validate().is_err());
        bar.widgets.clear();
        assert!(bar.validate().is_err());
        bar.widgets = BarSettings::DEFAULT_WIDGETS.to_vec();
        bar.monitor = BarMonitor::Named(" ".to_string());
        assert!(bar.validate().is_err());
    }

    #[test]
    fn bar_settings_set_field_parses_values() {
        let mut bar = BarSettings::default();
        bar.set_field("bar.enabled", "off").unwrap();
        bar.set_field("bar.monitor", "primary").unwrap();
        bar.set_field("bar.margin", "4").unwrap();
        bar.set_field("bar.padding", "16").unwrap();
        bar.set_field("bar.edge", "bottom").unwrap();
        bar.set_field("bar.height", "40").unwrap();
        bar.set_field("bar.font_size", "12").unwrap();
        bar.set_field("bar.radius", "8").unwrap();
        bar.set_field("bar.opacity", "90").unwrap();
        bar.set_field("bar.widgets", "workspaces,audio,clock")
            .unwrap();
        assert!(!bar.enabled);
        assert_eq!(bar.monitor, BarMonitor::Primary);
        assert_eq!(bar.margin, 4);
        assert_eq!(bar.padding, 16);
        assert_eq!(bar.edge, BarEdge::Bottom);
        assert_eq!(bar.height, 40);
        assert_eq!(bar.font_size, 12);
        assert_eq!(bar.radius, 8);
        assert_eq!(bar.opacity, 90);
        assert_eq!(
            bar.widgets,
            vec![BarWidget::Workspaces, BarWidget::Audio, BarWidget::Clock]
        );
    }

    #[test]
    fn bar_settings_widget_order_helpers_mutate_csv() {
        let mut bar = BarSettings {
            widgets: vec![BarWidget::Audio, BarWidget::Brightness, BarWidget::Clock],
            ..BarSettings::default()
        };

        assert!(bar.move_widget(BarWidget::Clock, WidgetMove::Up));
        assert_eq!(bar.widgets_csv(), "audio,clock,brightness");
        assert!(bar.move_widget(BarWidget::Audio, WidgetMove::Down));
        assert_eq!(bar.widgets_csv(), "clock,audio,brightness");
        assert!(!bar.move_widget(BarWidget::Clock, WidgetMove::Up));
        bar.reset_widgets();
        assert_eq!(bar.widgets, BarSettings::DEFAULT_WIDGETS);
    }

    #[test]
    fn osd_settings_validate_bounds() {
        let mut osd = OsdSettings::default();
        osd.width = OsdSettings::MIN_WIDTH - 1;
        assert!(osd.validate().is_err());
        osd.width = OsdSettings::MIN_WIDTH;
        osd.height = OsdSettings::MAX_HEIGHT + 1;
        assert!(osd.validate().is_err());
        osd.height = OsdSettings::MIN_HEIGHT;
        osd.margin = OsdSettings::MAX_MARGIN + 1;
        assert!(osd.validate().is_err());
        osd.margin = 0;
        osd.timeout_ms = OsdSettings::MIN_TIMEOUT_MS - 1;
        assert!(osd.validate().is_err());
        osd.timeout_ms = OsdSettings::MIN_TIMEOUT_MS;
        osd.font_size = OsdSettings::MAX_FONT_SIZE + 1;
        assert!(osd.validate().is_err());
        osd.font_size = OsdSettings::MIN_FONT_SIZE;
        osd.opacity = OsdSettings::MIN_OPACITY - 1;
        assert!(osd.validate().is_err());
    }

    #[test]
    fn osd_settings_set_field_parses_values() {
        let mut osd = OsdSettings::default();
        osd.set_field("osd.enabled", "off").unwrap();
        osd.set_field("osd.edge", "bottom").unwrap();
        osd.set_field("osd.width", "480").unwrap();
        osd.set_field("osd.height", "120").unwrap();
        osd.set_field("osd.margin", "24").unwrap();
        osd.set_field("osd.timeout_ms", "2500").unwrap();
        osd.set_field("osd.font_size", "13").unwrap();
        osd.set_field("osd.radius", "18").unwrap();
        osd.set_field("osd.opacity", "88").unwrap();
        assert!(!osd.enabled);
        assert_eq!(osd.edge, BarEdge::Bottom);
        assert_eq!(osd.width, 480);
        assert_eq!(osd.height, 120);
        assert_eq!(osd.margin, 24);
        assert_eq!(osd.timeout_ms, 2500);
        assert_eq!(osd.font_size, 13);
        assert_eq!(osd.radius, 18);
        assert_eq!(osd.opacity, 88);
    }

    #[test]
    fn corrupt_settings_fall_back_safely() {
        let path = test_path("corrupt");
        std::fs::write(&path, "{").unwrap();
        let loaded = load_or_default_from_path(path.clone()).unwrap();
        assert_eq!(
            loaded.ownership.keybindings,
            OwnershipMode::RespectUserConfig
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_bar_settings_fall_back_safely() {
        let path = test_path("invalid-bar");
        std::fs::write(
            &path,
            r#"{
  "ui": {
    "bar": {
      "widgets": [],
      "monitor": { "mode": "named", "name": "   " }
    }
  }
}"#,
        )
        .unwrap();

        let loaded = load_or_default_from_path(path.clone()).unwrap();
        assert_eq!(loaded.ui.bar.widgets, BarSettings::DEFAULT_WIDGETS);
        assert_eq!(loaded.ui.bar.monitor, BarMonitor::Focused);
        let _ = std::fs::remove_file(path);
    }
}
