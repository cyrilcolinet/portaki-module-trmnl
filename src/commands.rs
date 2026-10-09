//! Module commands — a manual push, and one after each saved change.
//!
//! The platform takes `updateConfig` itself (`#[portaki_sdk::config]`): the URL check that ran
//! at save time is `publishReadiness` now.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::push::{self, PushOutcome};

/// What the platform sends after a save that changed keys: their names, never their values.
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdatedArgs {
    #[serde(default)]
    pub changed_keys: Vec<String>,
}

/// Repaints the screen after a save, so Save is visibly what it claims to be.
#[portaki_sdk::command(name = "onConfigUpdated")]
pub fn on_config_updated(ctx: Context, _args: ConfigUpdatedArgs) -> Result<()> {
    let _ = push::push_now(&ctx, "onConfigUpdated");
    Ok(())
}

/// The "send now" button on the host surface.
#[portaki_sdk::command(name = "pushNow")]
pub fn push_now(ctx: Context, _args: EmptyArgs) -> Result<PushOutcome> {
    Ok(push::push_now(&ctx, "pushNow"))
}
