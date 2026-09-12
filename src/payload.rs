//! The merge variables a TRMNL template reads.
//!
//! Flat, snake_case, and small: a Liquid template indexes by name, and the free TRMNL plan
//! caps a payload at about 2 kB. Every value here is one the module can actually see — a wasm
//! module reads the property it runs for and the stay it was handed, nothing else.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config::{DisplayMode, ModuleConfig};

/// What the module knows about the stay a push is about, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaySnapshot {
    pub checkin_at: Option<DateTime<Utc>>,
    pub checkout_at: Option<DateTime<Utc>>,
    /// `airbnb`, `booking`, `direct`… as the platform recorded it.
    pub booking_channel: Option<String>,
}

impl StaySnapshot {
    /// A stay counts as running when `now` sits between its two ends.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        let started = self.checkin_at.map(|at| at <= now).unwrap_or(false);
        let ended = self.checkout_at.map(|at| at <= now).unwrap_or(false);
        started && !ended
    }

    pub fn nights_until_checkin(&self, now: DateTime<Utc>) -> Option<i64> {
        let checkin = self.checkin_at?;
        let days = (checkin - now).num_days();
        Some(days.max(0))
    }
}

/// Builds the payload for one push.
///
/// Times go out as RFC 3339 in UTC plus a bare date, and the template does the rest: a wasm
/// module has no timezone database, and a date silently shifted by an hour is worse on a screen
/// nobody rereads than an explicit UTC stamp.
pub fn build_payload(
    config: &ModuleConfig,
    property_name: &str,
    stay: Option<&StaySnapshot>,
    now: DateTime<Utc>,
) -> Value {
    let mut map = Map::new();
    map.insert(
        "property_name".into(),
        Value::String(display_name(config, property_name)),
    );
    map.insert(
        "display_mode".into(),
        Value::String(config.display_mode.as_str().to_string()),
    );
    map.insert("updated_at".into(), Value::String(now.to_rfc3339()));
    map.insert(
        "updated_time".into(),
        Value::String(now.format("%H:%M").to_string()),
    );

    let stay_active = stay.map(|s| s.is_active_at(now)).unwrap_or(false);
    map.insert("stay_active".into(), Value::Bool(stay_active));

    if let Some(stay) = stay {
        insert_instant(&mut map, "checkin", stay.checkin_at);
        insert_instant(&mut map, "checkout", stay.checkout_at);
        if let Some(channel) = stay
            .booking_channel
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            map.insert("booking_channel".into(), Value::String(channel.to_string()));
        }
        if config.display_mode == DisplayMode::HostDashboard {
            if let Some(days) = stay.nights_until_checkin(now) {
                map.insert("next_stay_countdown_days".into(), Value::from(days));
            }
        }
    }

    Value::Object(map)
}

/// `checkin_at` (RFC 3339), `checkin_date` and `checkin_time` — a template picks what it needs.
fn insert_instant(map: &mut Map<String, Value>, prefix: &str, at: Option<DateTime<Utc>>) {
    let Some(at) = at else { return };
    map.insert(format!("{prefix}_at"), Value::String(at.to_rfc3339()));
    map.insert(
        format!("{prefix}_date"),
        Value::String(at.format("%Y-%m-%d").to_string()),
    );
    map.insert(
        format!("{prefix}_time"),
        Value::String(at.format("%H:%M").to_string()),
    );
}

fn display_name(config: &ModuleConfig, property_name: &str) -> String {
    let override_name = config.property_name_override.trim();
    if !override_name.is_empty() {
        return override_name.to_string();
    }
    let property_name = property_name.trim();
    if !property_name.is_empty() {
        return property_name.to_string();
    }
    "Portaki".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    fn config(mode: DisplayMode) -> ModuleConfig {
        ModuleConfig {
            webhook_url: "https://usetrmnl.com/api/custom_plugins/7f1c".to_string(),
            display_mode: mode,
            property_name_override: String::new(),
        }
    }

    #[test]
    fn the_override_wins_then_the_property_then_a_last_resort() {
        let mut cfg = config(DisplayMode::HostDashboard);
        cfg.property_name_override = "  Chez Marie  ".to_string();
        assert_eq!(display_name(&cfg, "Villa"), "Chez Marie");

        cfg.property_name_override = String::new();
        assert_eq!(display_name(&cfg, " Villa "), "Villa");
        assert_eq!(display_name(&cfg, "   "), "Portaki");
    }

    #[test]
    fn a_payload_without_a_stay_still_says_the_screen_is_idle() {
        let payload = build_payload(
            &config(DisplayMode::GuestDisplay),
            "Villa",
            None,
            at("2026-09-12T08:00:00Z"),
        );

        assert_eq!(payload["stay_active"], Value::Bool(false));
        assert_eq!(
            payload["display_mode"],
            Value::String("guest_display".into())
        );
        assert_eq!(payload["updated_time"], Value::String("08:00".into()));
        assert!(payload.get("checkin_at").is_none());
    }

    #[test]
    fn a_running_stay_carries_both_ends_three_ways() {
        let stay = StaySnapshot {
            checkin_at: Some(at("2026-09-11T14:00:00Z")),
            checkout_at: Some(at("2026-09-15T09:00:00Z")),
            booking_channel: Some("airbnb".into()),
        };
        let payload = build_payload(
            &config(DisplayMode::GuestDisplay),
            "Villa",
            Some(&stay),
            at("2026-09-12T08:00:00Z"),
        );

        assert_eq!(payload["stay_active"], Value::Bool(true));
        assert_eq!(payload["checkin_date"], Value::String("2026-09-11".into()));
        assert_eq!(payload["checkin_time"], Value::String("14:00".into()));
        assert_eq!(
            payload["checkout_at"],
            Value::String("2026-09-15T09:00:00+00:00".into())
        );
        assert_eq!(payload["booking_channel"], Value::String("airbnb".into()));
        // The countdown is host-dashboard furniture: a guest already arrived.
        assert!(payload.get("next_stay_countdown_days").is_none());
    }

    #[test]
    fn the_host_dashboard_counts_the_days_left_and_never_goes_negative() {
        let stay = StaySnapshot {
            checkin_at: Some(at("2026-09-15T15:00:00Z")),
            checkout_at: Some(at("2026-09-20T10:00:00Z")),
            booking_channel: None,
        };
        let payload = build_payload(
            &config(DisplayMode::HostDashboard),
            "Villa",
            Some(&stay),
            at("2026-09-12T08:00:00Z"),
        );
        assert_eq!(payload["next_stay_countdown_days"], Value::from(3));

        let past = build_payload(
            &config(DisplayMode::HostDashboard),
            "Villa",
            Some(&stay),
            at("2026-09-18T08:00:00Z"),
        );
        assert_eq!(past["next_stay_countdown_days"], Value::from(0));
        assert_eq!(past["stay_active"], Value::Bool(true));
    }
}
