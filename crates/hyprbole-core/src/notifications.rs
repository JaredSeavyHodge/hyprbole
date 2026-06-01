use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    pub dnd_enabled: bool,
    pub history_limit: usize,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            dnd_enabled: false,
            history_limit: 50,
        }
    }
}

impl NotificationSettings {
    pub fn effective_history_limit(&self) -> usize {
        self.history_limit.max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

impl Default for NotificationUrgency {
    fn default() -> Self {
        Self::Normal
    }
}

impl std::str::FromStr for NotificationUrgency {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "low" => Ok(Self::Low),
            "normal" | "default" => Ok(Self::Normal),
            "critical" | "urgent" => Ok(Self::Critical),
            value => Err(format!("invalid notification urgency `{value}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: u64,
    pub summary: String,
    pub body: String,
    pub urgency: NotificationUrgency,
    pub read: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationSnapshot {
    pub dnd_enabled: bool,
    pub unread_count: usize,
    pub history_limit: usize,
    pub recent: Vec<NotificationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NotificationAction {
    SetDnd {
        enabled: bool,
    },
    ToggleDnd,
    ClearHistory,
    Dismiss {
        id: u64,
    },
    Push {
        summary: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        urgency: NotificationUrgency,
    },
}

pub fn snapshot(
    settings: &NotificationSettings,
    history: &[NotificationRecord],
) -> NotificationSnapshot {
    NotificationSnapshot {
        dnd_enabled: settings.dnd_enabled,
        unread_count: history.iter().filter(|record| !record.read).count(),
        history_limit: settings.effective_history_limit(),
        recent: history.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_action_round_trips_json() {
        let action = NotificationAction::SetDnd { enabled: true };
        let json = serde_json::to_string(&action).unwrap();
        let decoded: NotificationAction = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn notification_snapshot_counts_unread() {
        let settings = NotificationSettings::default();
        let snapshot = snapshot(
            &settings,
            &[
                NotificationRecord {
                    id: 1,
                    summary: "a".to_string(),
                    body: String::new(),
                    urgency: NotificationUrgency::Normal,
                    read: false,
                },
                NotificationRecord {
                    id: 2,
                    summary: "b".to_string(),
                    body: String::new(),
                    urgency: NotificationUrgency::Low,
                    read: true,
                },
            ],
        );
        assert_eq!(snapshot.unread_count, 1);
    }

    #[test]
    fn notification_snapshot_reports_effective_history_limit() {
        let settings = NotificationSettings {
            history_limit: 0,
            ..NotificationSettings::default()
        };
        let snapshot = snapshot(&settings, &[]);
        assert_eq!(snapshot.history_limit, 1);
    }
}
