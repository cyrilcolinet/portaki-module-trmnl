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

/// Le nom affiché, au plus (spec TRMNL §2.2) : l'en-tête d'un écran e-ink est étroit.
pub const DISPLAY_NAME_MAX: usize = 30;

impl ModuleConfig {
    /// Ce qui ne va pas, champ par champ — sous le champ dans le tiroir, et dans
    /// `publishReadiness` (spec TRMNL §2) : une URL de Private Plugin TRMNL en https, un nom
    /// affiché de 30 caractères au plus.
    pub fn problems(&self) -> Vec<(&'static str, &'static str)> {
        let mut problems = Vec::new();
        let url = self.webhook_url.trim();
        let plugin = url.starts_with("https://")
            && url
                .strip_prefix("https://")
                .and_then(|rest| rest.split('/').next())
                .is_some_and(|host| host == "usetrmnl.com" || host.ends_with(".usetrmnl.com"))
            && crate::push::plugin_id_from_webhook_url(url).is_some();
        if !plugin {
            problems.push(("webhook_url", "host.webhookUrl.invalid"));
        }
        if self.property_name_override.trim().chars().count() > DISPLAY_NAME_MAX {
            problems.push(("property_name_override", "host.propertyName.tooLong"));
        }
        problems
    }

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

    /// Une URL de Private Plugin TRMNL en https, un nom de 30 caractères au plus.
    #[test]
    fn problems_follow_the_spec() {
        let with = |url: &str, name: &str| ModuleConfig {
            webhook_url: url.into(),
            property_name_override: name.into(),
            ..ModuleConfig::default()
        };
        let fields = |config: ModuleConfig| -> Vec<&'static str> {
            config.problems().into_iter().map(|(f, _)| f).collect()
        };
        assert!(fields(with(
            "https://usetrmnl.com/api/custom_plugins/7f1c",
            "Chez Marie"
        ))
        .is_empty());
        assert_eq!(fields(with("", "")), ["webhook_url"]);
        assert_eq!(
            fields(with("http://usetrmnl.com/api/custom_plugins/7f1c", "")),
            ["webhook_url"]
        );
        assert_eq!(
            fields(with("https://evil.example/api/custom_plugins/7f1c", "")),
            ["webhook_url"]
        );
        assert_eq!(
            fields(with(
                "https://usetrmnl.com/api/custom_plugins/7f1c",
                &"x".repeat(31)
            )),
            ["property_name_override"]
        );
    }
}
