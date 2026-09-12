//! Host configuration stored in KV (`config` key).

use portaki_sdk::host;
use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

const CONFIG_KEY: &str = "config";

/// What the screen is for: the host's own dashboard, or a display left in the property.
///
/// Two audiences, two payloads: the host wants to know what is coming, a guest wants to know
/// what applies to them right now. Same device, same module — only the merge variables differ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    /// The mode that needs no guest present, hence the default.
    #[default]
    HostDashboard,
    GuestDisplay,
}

impl DisplayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DisplayMode::HostDashboard => "host_dashboard",
            DisplayMode::GuestDisplay => "guest_display",
        }
    }

    /// Anything unknown reads as the host dashboard — the mode that needs no guest present.
    pub fn from_wire(raw: &str) -> Self {
        match raw.trim() {
            "guest_display" => DisplayMode::GuestDisplay,
            _ => DisplayMode::HostDashboard,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// The Private Plugin webhook URL, as TRMNL hands it over.
    ///
    /// Kept whole rather than reduced to its id: it is what the host copied, so it is what the
    /// field must show back when they return to check it.
    #[serde(default)]
    pub webhook_url: String,
    #[serde(default)]
    pub display_mode: DisplayMode,
    /// Shown instead of the Portaki property name — a screen in a hallway says "Chez Marie",
    /// not "Villa Pathologie — logement 2".
    #[serde(default)]
    pub property_name_override: String,
}

impl ModuleConfig {
    /// Whether a push can even be attempted.
    pub fn is_ready(&self) -> bool {
        !self.webhook_url.trim().is_empty()
    }
}

pub fn load_config() -> Result<ModuleConfig> {
    let Some(bytes) = host::kv::get(CONFIG_KEY)? else {
        return Ok(ModuleConfig::default());
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| PortakiError::Storage(format!("invalid config JSON: {error}")))
}

pub fn save_config(config: &ModuleConfig) -> Result<()> {
    let bytes = serde_json::to_vec(config)
        .map_err(|error| PortakiError::Storage(format!("config serialize: {error}")))?;
    host::kv::set(CONFIG_KEY, &bytes, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_display_modes_fall_back_to_the_host_dashboard() {
        assert_eq!(
            DisplayMode::from_wire("guest_display"),
            DisplayMode::GuestDisplay
        );
        assert_eq!(
            DisplayMode::from_wire(" host_dashboard "),
            DisplayMode::HostDashboard
        );
        assert_eq!(DisplayMode::from_wire("kiosk"), DisplayMode::HostDashboard);
    }

    #[test]
    fn a_module_without_a_webhook_is_not_ready() {
        assert!(!ModuleConfig::default().is_ready());
        assert!(ModuleConfig {
            webhook_url: "https://usetrmnl.com/api/custom_plugins/abc".to_string(),
            ..ModuleConfig::default()
        }
        .is_ready());
    }
}
