//! Module queries — what the dashboard reads back.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::{load_config, ModuleConfig};
use crate::push::{load_state, PushState};

/// The settings, as saved.
#[portaki_sdk::query(name = "getConfig")]
pub fn get_config(_ctx: Context) -> Result<ModuleConfig> {
    load_config()
}

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
pub fn get_status(_ctx: Context) -> Result<ModuleStatus> {
    let config = load_config()?;
    Ok(ModuleStatus {
        configured: config.is_ready(),
        display_mode: config.display_mode.as_str().to_string(),
        push: load_state()?,
    })
}
