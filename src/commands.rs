//! Module commands — settings persistence and a manual push.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::{save_config, DisplayMode, ModuleConfig};
use crate::push::{self, PushOutcome};

/// The flat field map the modules sheet posts when the host presses Save.
///
/// Names match the inputs in `host::render_host_main` one for one — the shell merges field
/// values at click time, so a renamed input silently stops arriving.
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfigArgs {
    #[serde(default)]
    pub webhook_url: String,
    #[serde(default)]
    pub display_mode: String,
    #[serde(default)]
    pub property_name_override: String,
}

impl UpdateConfigArgs {
    fn to_config(&self) -> Result<ModuleConfig> {
        let webhook_url = self.webhook_url.trim().to_string();
        // An URL that is not a plugin webhook would be saved, never used, and never explained.
        if !webhook_url.is_empty() && push::plugin_id_from_webhook_url(&webhook_url).is_none() {
            return Err(PortakiError::Host(
                "webhook_url_not_a_trmnl_plugin".to_string(),
            ));
        }
        Ok(ModuleConfig {
            webhook_url,
            display_mode: DisplayMode::from_wire(&self.display_mode),
            property_name_override: self.property_name_override.trim().to_string(),
        })
    }
}

/// Saves the settings, then repaints the screen so Save is visibly what it claims to be.
#[portaki_sdk::command(name = "updateConfig")]
pub fn update_config(ctx: Context, args: UpdateConfigArgs) -> Result<()> {
    save_config(&args.to_config()?)?;
    let _ = push::push_now(&ctx, "updateConfig");
    Ok(())
}

/// The "send now" button on the host surface.
#[portaki_sdk::command(name = "pushNow")]
pub fn push_now(ctx: Context, _args: EmptyArgs) -> Result<PushOutcome> {
    Ok(push::push_now(&ctx, "pushNow"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_trimmed_and_the_mode_is_normalised() {
        let config = UpdateConfigArgs {
            webhook_url: "  https://usetrmnl.com/api/custom_plugins/7f1c  ".to_string(),
            display_mode: "guest_display".to_string(),
            property_name_override: "  Chez Marie ".to_string(),
        }
        .to_config()
        .expect("valid settings");

        assert_eq!(
            config.webhook_url,
            "https://usetrmnl.com/api/custom_plugins/7f1c"
        );
        assert_eq!(config.display_mode, DisplayMode::GuestDisplay);
        assert_eq!(config.property_name_override, "Chez Marie");
    }

    #[test]
    fn an_empty_webhook_is_allowed_so_a_host_can_pause_the_screen() {
        let config = UpdateConfigArgs::default()
            .to_config()
            .expect("empty settings");

        assert!(!config.is_ready());
        assert_eq!(config.display_mode, DisplayMode::HostDashboard);
    }

    #[test]
    fn a_url_that_is_not_a_plugin_webhook_is_refused_at_save_time() {
        let error = UpdateConfigArgs {
            webhook_url: "https://example.com/hook".to_string(),
            ..UpdateConfigArgs::default()
        }
        .to_config()
        .expect_err("refused");

        assert!(error.to_string().contains("webhook_url_not_a_trmnl_plugin"));
    }
}
