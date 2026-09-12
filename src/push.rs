//! Sending one screen update, and remembering that it happened.

use chrono::{DateTime, Utc};
use portaki_sdk::host;
use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{load_config, ModuleConfig};
use crate::payload::{build_payload, StaySnapshot};

const STATE_KEY: &str = "push_state";

/// TRMNL's free plan allows roughly twelve webhook calls an hour; this stays one under.
///
/// A screen that refreshes every fifteen minutes at best gains nothing from a thirteenth push,
/// and a module that trips the plan limit breaks the twelve that mattered.
const MAX_PUSHES_PER_HOUR: usize = 11;

const CONNECTOR_ID: &str = "trmnl";
const CONNECTOR_OP: &str = "push";

/// The ledger of what left this module, kept so a host can see it and a limit can be enforced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushState {
    /// Epoch seconds of the pushes that went out, oldest first, pruned to the last hour.
    #[serde(default)]
    pub recent: Vec<i64>,
    #[serde(default)]
    pub last_push_at: Option<String>,
    /// `ok`, `rate_limited`, `not_configured`, or the runtime's own failure reason.
    #[serde(default)]
    pub last_status: Option<String>,
}

impl PushState {
    /// Whether one more push fits in the trailing hour, pruning what has aged out.
    pub fn allows(&mut self, now: DateTime<Utc>) -> bool {
        let cutoff = now.timestamp() - 3600;
        self.recent.retain(|stamp| *stamp >= cutoff);
        self.recent.len() < MAX_PUSHES_PER_HOUR
    }

    pub fn record(&mut self, now: DateTime<Utc>, status: &str, counted: bool) {
        if counted {
            self.recent.push(now.timestamp());
        }
        self.last_push_at = Some(now.to_rfc3339());
        self.last_status = Some(status.to_string());
    }
}

/// What a push did, in terms a host surface can show without inventing its own vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushOutcome {
    pub ok: bool,
    pub status: String,
}

impl PushOutcome {
    fn ok() -> Self {
        PushOutcome {
            ok: true,
            status: "ok".to_string(),
        }
    }

    fn refused(status: &str) -> Self {
        PushOutcome {
            ok: false,
            status: status.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct PushArgs {
    /// Consumed by the connector path template; never reaches the body.
    plugin_id: String,
    merge_variables: Value,
}

/// Reads the plugin id out of a pasted webhook URL.
///
/// The host copies the whole URL from TRMNL — asking them to extract a UUID by hand is asking
/// for a support thread. Anything that is not a TRMNL custom-plugin URL returns `None` rather
/// than a guess: a wrong id pushes someone else's screen.
pub fn plugin_id_from_webhook_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let (_, tail) = trimmed.split_once("/api/custom_plugins/")?;
    let id = tail
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim();
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
}

/// Pushes the current state of this property to the screen.
///
/// Never fails the caller: a booking is confirmed whether or not a screen in a hallway heard
/// about it. The reason is logged, stored, and shown on the host surface instead.
pub fn push_now(ctx: &Context, trigger: &str) -> PushOutcome {
    match try_push(ctx, trigger) {
        Ok(outcome) => outcome,
        Err(error) => {
            let status = error.to_string();
            log_failure(trigger, &status);
            let _ = record(&status, false);
            PushOutcome::refused(&status)
        }
    }
}

fn try_push(ctx: &Context, trigger: &str) -> Result<PushOutcome> {
    let config = load_config()?;
    let Some(plugin_id) = plugin_id_from_webhook_url(&config.webhook_url) else {
        return finish(&config, "not_configured", false);
    };

    let now = host::time::now()?;
    let mut state = load_state()?;
    if !state.allows(now) {
        state.record(now, "rate_limited", false);
        save_state(&state)?;
        return Ok(PushOutcome::refused("rate_limited"));
    }

    let stay = ctx.stay.as_ref().map(|stay| StaySnapshot {
        checkin_at: stay.checkin_at,
        checkout_at: stay.checkout_at,
        booking_channel: stay.booking_channel.clone(),
    });
    let payload = build_payload(&config, &ctx.property.name, stay.as_ref(), now);
    let args = PushArgs {
        plugin_id,
        merge_variables: payload,
    };

    match host::connectors::call::<PushArgs, Value>(CONNECTOR_ID, CONNECTOR_OP, &args) {
        Ok(_) => {
            state.record(now, "ok", true);
            save_state(&state)?;
            Ok(PushOutcome::ok())
        }
        Err(error) => {
            let status = error.to_string();
            log_failure(trigger, &status);
            state.record(now, &status, false);
            save_state(&state)?;
            Ok(PushOutcome::refused(&status))
        }
    }
}

fn finish(_config: &ModuleConfig, status: &str, counted: bool) -> Result<PushOutcome> {
    record(status, counted)?;
    Ok(PushOutcome::refused(status))
}

fn record(status: &str, counted: bool) -> Result<()> {
    let now = host::time::now()?;
    let mut state = load_state()?;
    state.record(now, status, counted);
    save_state(&state)
}

fn log_failure(trigger: &str, status: &str) {
    let mut fields = host::log::Fields::new();
    fields.insert("trigger", &trigger.to_string());
    fields.insert("status", &status.to_string());
    let _ = host::log::warn("trmnl_push_failed", &fields);
}

pub fn load_state() -> Result<PushState> {
    let Some(bytes) = host::kv::get(STATE_KEY)? else {
        return Ok(PushState::default());
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| PortakiError::Storage(format!("invalid push state JSON: {error}")))
}

fn save_state(state: &PushState) -> Result<()> {
    let bytes = serde_json::to_vec(state)
        .map_err(|error| PortakiError::Storage(format!("push state serialize: {error}")))?;
    host::kv::set(STATE_KEY, &bytes, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn the_plugin_id_is_read_out_of_whatever_trmnl_gave_the_host() {
        assert_eq!(
            plugin_id_from_webhook_url("https://usetrmnl.com/api/custom_plugins/7f1c-42"),
            Some("7f1c-42".to_string())
        );
        // Trailing slash, query string, and stray whitespace all come from a copy-paste.
        assert_eq!(
            plugin_id_from_webhook_url("  https://usetrmnl.com/api/custom_plugins/7f1c/  "),
            Some("7f1c".to_string())
        );
        assert_eq!(
            plugin_id_from_webhook_url("https://usetrmnl.com/api/custom_plugins/7f1c?merge=1"),
            Some("7f1c".to_string())
        );
    }

    #[test]
    fn anything_that_is_not_a_plugin_webhook_is_refused_rather_than_guessed() {
        assert_eq!(plugin_id_from_webhook_url(""), None);
        assert_eq!(
            plugin_id_from_webhook_url("https://usetrmnl.com/api/custom_plugins/"),
            None
        );
        assert_eq!(
            plugin_id_from_webhook_url("https://example.com/webhook/7f1c"),
            None
        );
    }

    #[test]
    fn the_hourly_budget_refuses_the_twelfth_push_and_refills_as_it_ages() {
        let mut state = PushState::default();
        let now = at("2026-09-12T10:00:00Z");
        for _ in 0..MAX_PUSHES_PER_HOUR {
            assert!(state.allows(now));
            state.record(now, "ok", true);
        }
        assert!(!state.allows(now));

        // An hour later, the whole batch has aged out.
        assert!(state.allows(at("2026-09-12T11:00:01Z")));
    }

    #[test]
    fn a_refused_push_does_not_spend_the_budget() {
        let mut state = PushState::default();
        let now = at("2026-09-12T10:00:00Z");
        state.record(now, "connector_credential_missing", false);

        assert!(state.recent.is_empty());
        assert_eq!(
            state.last_status.as_deref(),
            Some("connector_credential_missing")
        );
        assert_eq!(
            state.last_push_at.as_deref(),
            Some("2026-09-12T10:00:00+00:00")
        );
    }
}
