use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::broadcast;

use crate::config::{NtfyPriority, PushConfig, PushRule};
use crate::db::Db;
use crate::events::{AgentEvent, Event, EventBus};
use crate::session::SessionState;

pub struct Notification {
    pub title: String,
    pub body: String,
    pub priority: NtfyPriority,
    pub click_url: String,
    pub tags: String,
}

pub struct NtfyClient {
    http: reqwest::Client,
    url: String,
    topic: String,
}

impl NtfyClient {
    pub fn new(url: &str, topic: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client");
        NtfyClient {
            http,
            url: url.to_string(),
            topic: topic.to_string(),
        }
    }

    pub async fn send(&self, notif: Notification) {
        let endpoint = format!("{}/{}", self.url, self.topic);
        let result = self
            .http
            .post(&endpoint)
            .header("X-Title", &notif.title)
            .header("X-Priority", notif.priority.as_ntfy_str())
            .header("X-Click", &notif.click_url)
            .header("X-Tags", &notif.tags)
            .body(notif.body.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                tracing::debug!(
                    title = %notif.title,
                    status = %resp.status(),
                    "ntfy push sent"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, title = %notif.title, "ntfy push failed");
            }
        }
    }
}

pub struct RuleEngine {
    rules: Vec<PushRule>,
    base_url: String,
    context_fired: HashSet<String>,
}

impl RuleEngine {
    pub fn new(config: &PushConfig) -> Self {
        RuleEngine {
            rules: config.rules.clone(),
            base_url: config.base_url.clone(),
            context_fired: HashSet::new(),
        }
    }

    fn rule_enabled(&self, name: &str) -> Option<&NtfyPriority> {
        self.rules
            .iter()
            .find(|r| r.name == name && r.enabled)
            .map(|r| &r.priority)
    }

    pub fn evaluate(&mut self, session_id: &str, event: &Event) -> Option<Notification> {
        let click_url = format!("{}/session/{}", self.base_url, session_id);

        match event {
            Event::StateChanged {
                from: SessionState::Streaming,
                to: SessionState::Awaiting,
            } => {
                let priority = self.rule_enabled("awaiting_permission")?.clone();
                Some(Notification {
                    title: "Permission required".to_string(),
                    body: format!("Session {} waiting for approval", session_id),
                    priority,
                    click_url,
                    tags: "bell".to_string(),
                })
            }

            Event::AgentEvent {
                event: AgentEvent::Error { message },
                ..
            } => {
                let priority = self.rule_enabled("agent_error")?.clone();
                let body: String = message.chars().take(200).collect();
                Some(Notification {
                    title: "Agent error".to_string(),
                    body,
                    priority,
                    click_url,
                    tags: "warning".to_string(),
                })
            }

            Event::StateChanged {
                to: SessionState::Exited,
                ..
            } => {
                let priority = self.rule_enabled("session_exited_nonzero")?.clone();
                Some(Notification {
                    title: "Session exited".to_string(),
                    body: format!("Session {} exited — check session for details", session_id),
                    priority,
                    click_url,
                    tags: "skull".to_string(),
                })
            }

            Event::AgentEvent {
                event: AgentEvent::ContextWindowSizeChanged { pct_used, .. },
                ..
            } if *pct_used >= 0.80 => {
                let priority = self.rule_enabled("context_window_80pct")?.clone();
                if self.context_fired.contains(session_id) {
                    return None;
                }
                self.context_fired.insert(session_id.to_string());
                Some(Notification {
                    title: "Context window 80%".to_string(),
                    body: format!("Session {} at {:.0}% context", session_id, pct_used * 100.0),
                    priority,
                    click_url,
                    tags: "hourglass".to_string(),
                })
            }

            _ => None,
        }
    }

    pub fn clear_session(&mut self, session_id: &str) {
        self.context_fired.remove(session_id);
    }
}

pub async fn run(event_bus: Arc<EventBus>, db: Db, config: PushConfig) -> Result<()> {
    if !config.enabled {
        tracing::info!("push disabled");
        return Ok(());
    }

    let client = NtfyClient::new(&config.ntfy_url, &config.ntfy_topic);
    let mut engine = RuleEngine::new(&config);
    let mut rx = event_bus.subscribe();

    loop {
        match rx.recv().await {
            Ok((session_id, event)) => {
                let is_exit = matches!(
                    &event,
                    Event::StateChanged {
                        to: SessionState::Exited,
                        ..
                    }
                );

                if let Some(notif) = engine.evaluate(&session_id, &event) {
                    client.send(notif).await;
                    increment_push_metric(&db).await;
                }

                if is_exit {
                    engine.clear_session(&session_id);
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("push subscriber lagged, skipped {} events", n);
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::info!("event bus closed, push task exiting");
                break;
            }
        }
    }

    Ok(())
}

async fn increment_push_metric(db: &Db) {
    let day = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_per_day = 86400u64;
        let days = now / secs_per_day;
        time_from_epoch_days(days)
    };

    let result = sqlx::query(
        "INSERT INTO metrics_daily (day, tokens_total, sessions_created, push_sent) \
         VALUES (?, 0, 0, 1) \
         ON CONFLICT(day) DO UPDATE SET push_sent = push_sent + 1",
    )
    .bind(&day)
    .execute(db.pool())
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, "failed to increment push_sent metric");
    }
}

fn time_from_epoch_days(days: u64) -> String {
    // Compute YYYY-MM-DD from days since Unix epoch (1970-01-01)
    // Using the algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PushConfig;

    fn default_engine() -> RuleEngine {
        RuleEngine::new(&PushConfig::default())
    }

    fn engine_with_rule_disabled(rule_name: &str) -> RuleEngine {
        let mut config = PushConfig::default();
        for rule in config.rules.iter_mut() {
            if rule.name == rule_name {
                rule.enabled = false;
            }
        }
        RuleEngine::new(&config)
    }

    fn streaming_to_awaiting() -> Event {
        Event::StateChanged {
            from: SessionState::Streaming,
            to: SessionState::Awaiting,
        }
    }

    fn idle_to_awaiting() -> Event {
        Event::StateChanged {
            from: SessionState::Idle,
            to: SessionState::Awaiting,
        }
    }

    fn agent_error(msg: &str) -> Event {
        Event::AgentEvent {
            id: crate::session::SessionId::new(),
            event: AgentEvent::Error {
                message: msg.to_string(),
            },
        }
    }

    fn context_window(pct: f32) -> Event {
        Event::AgentEvent {
            id: crate::session::SessionId::new(),
            event: AgentEvent::ContextWindowSizeChanged {
                pct_used: pct,
                tokens: 1000,
            },
        }
    }

    fn state_exited(from: SessionState) -> Event {
        Event::StateChanged {
            from,
            to: SessionState::Exited,
        }
    }

    #[test]
    fn test_awaiting_permission_rule_fires() {
        let mut engine = default_engine();
        let notif = engine.evaluate("sess1", &streaming_to_awaiting());
        assert!(notif.is_some());
        let notif = notif.unwrap();
        assert!(matches!(notif.priority, NtfyPriority::High));
    }

    #[test]
    fn test_awaiting_permission_wrong_transition_ignored() {
        let mut engine = default_engine();
        let notif = engine.evaluate("sess1", &idle_to_awaiting());
        assert!(notif.is_none());
    }

    #[test]
    fn test_agent_error_rule_fires() {
        let mut engine = default_engine();
        let notif = engine.evaluate("sess1", &agent_error("boom")).unwrap();
        assert!(matches!(notif.priority, NtfyPriority::High));
        assert!(notif.body.contains("boom"));
    }

    #[test]
    fn test_context_window_below_threshold_ignored() {
        let mut engine = default_engine();
        assert!(engine.evaluate("sess1", &context_window(0.5)).is_none());
    }

    #[test]
    fn test_context_window_fires_once_per_session() {
        let mut engine = default_engine();
        let first = engine.evaluate("sess1", &context_window(0.85));
        let second = engine.evaluate("sess1", &context_window(0.85));
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn test_context_window_different_sessions() {
        let mut engine = default_engine();
        let a = engine.evaluate("sessA", &context_window(0.85));
        let b = engine.evaluate("sessB", &context_window(0.85));
        assert!(a.is_some());
        assert!(b.is_some());
    }

    #[test]
    fn test_context_window_resets_on_exit() {
        let mut engine = default_engine();
        let _ = engine.evaluate("sessA", &context_window(0.85));
        engine.clear_session("sessA");
        let second = engine.evaluate("sessA", &context_window(0.85));
        assert!(second.is_some());
    }

    #[test]
    fn test_disabled_rule_skipped() {
        let mut engine = engine_with_rule_disabled("awaiting_permission");
        let notif = engine.evaluate("sess1", &streaming_to_awaiting());
        assert!(notif.is_none());
    }

    #[test]
    fn test_exited_rule_fires() {
        let mut engine = default_engine();
        let notif = engine
            .evaluate("sess1", &state_exited(SessionState::Streaming))
            .unwrap();
        assert!(matches!(notif.priority, NtfyPriority::Normal));
    }

    #[test]
    fn test_deep_link_format() {
        let config = PushConfig {
            base_url: "https://optiplex.example.com".to_string(),
            ..PushConfig::default()
        };
        let mut engine = RuleEngine::new(&config);
        let notif = engine
            .evaluate("sess123", &streaming_to_awaiting())
            .unwrap();
        assert_eq!(
            notif.click_url,
            "https://optiplex.example.com/session/sess123"
        );
    }

    #[test]
    fn test_date_from_epoch_days() {
        // 1970-01-01 = day 0
        assert_eq!(time_from_epoch_days(0), "1970-01-01");
        // 2024-01-01 = day 19723
        assert_eq!(time_from_epoch_days(19723), "2024-01-01");
    }
}
