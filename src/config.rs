//! Host configuration, held by the platform (`#[portaki_sdk::config]`).
//!
//! Before the platform held it, the module kept the same keys in KV (`config`): the generated
//! `load` reads that blob while no `moduleConfig` is sent, and the platform imports it once
//! (`legacyConfig`), so an install keeps pushing until the host publishes.

use serde::{Deserialize, Serialize};

/// What the screen is for: the host's own dashboard, or a display left in the property.
///
/// Two audiences, two payloads: the host wants to know what is coming, a guest wants to know
/// what applies to them right now. Same device, same module — only the merge variables differ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    HostDashboard,
    /// « Affichage logement », the spec's default (TRMNL §2.2). A saved mode is always stored,
    /// so only an install that never saved reads this.
    #[default]
    GuestDisplay,
}

impl DisplayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DisplayMode::HostDashboard => "host_dashboard",
            DisplayMode::GuestDisplay => "guest_display",
        }
    }
}

/// The keys are the names of the host form fields: the platform takes `updateConfig` itself.
#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// The Private Plugin webhook URL, as TRMNL hands it over. The plugin id in it is the whole
    /// credential: a secret, in the vault.
    #[field(required, secret, label = "host.webhookUrl.label")]
    pub webhook_url: String,
    #[field(
        kind = "select",
        options = ["host_dashboard", "guest_display"],
        label = "host.displayMode.label"
    )]
    pub display_mode: DisplayMode,
    /// Shown instead of the Portaki property name — a screen in a hallway says "Chez Marie",
    /// not "Villa Pathologie — logement 2".
    #[field(label = "host.propertyName.label")]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An install that never saved reads « Affichage logement »; a saved mode stays.
    #[test]
    fn the_mode_defaults_to_the_guest_display_and_a_saved_one_stays() {
        assert_eq!(
            ModuleConfig::default().display_mode,
            DisplayMode::GuestDisplay
        );
        let blank: ModuleConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(blank.display_mode, DisplayMode::GuestDisplay);
        let saved: ModuleConfig =
            serde_json::from_str(r#"{"display_mode":"host_dashboard"}"#).unwrap();
        assert_eq!(saved.display_mode, DisplayMode::HostDashboard);
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
