//! Module queries — what the dashboard reads back.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::ModuleConfig;
use crate::push::{load_state, PushState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    /// Whether a webhook URL is set at all.
    pub configured: bool,
    pub display_mode: String,
    #[serde(flatten)]
    pub push: PushState,
}

/// Configured or not, and what the last push did — the stats card reads this.
#[portaki_sdk::query(name = "getStatus")]
pub fn get_status(ctx: Context) -> Result<ModuleStatus> {
    let config = ModuleConfig::load(&ctx)?;
    Ok(ModuleStatus {
        configured: config.is_ready(),
        display_mode: config.display_mode.as_str().to_string(),
        push: load_state()?,
    })
}

/// Ce qui bloque la publication : une URL qui n'est pas celle d'un Private Plugin TRMNL, un nom
/// affiché trop long — chacun sur son champ (`config.<clé>`).
#[portaki_sdk::query(name = "publishReadiness")]
pub fn publish_readiness(
    ctx: Context,
) -> Result<portaki_sdk::contracts::publish::PublishReadiness> {
    use portaki_sdk::contracts::publish::{PublishCheck, PublishLevel, PublishReadiness};
    let bundle = crate::i18n::text;
    let items = ModuleConfig::load(&ctx)?
        .problems()
        .into_iter()
        .map(|(field, key)| PublishCheck {
            id: format!("config.{field}"),
            level: PublishLevel::Required,
            ok: false,
            label: bundle(match field {
                "webhook_url" => "host.webhookUrl.label",
                _ => "host.propertyName.label",
            }),
            hint: bundle(key),
        })
        .collect();
    Ok(PublishReadiness { items })
}
