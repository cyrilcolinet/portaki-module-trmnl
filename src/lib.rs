//! Portaki TRMNL module — pushes stay information to a TRMNL e-ink screen.
//!
//! The screen is fed by a TRMNL *Private Plugin* webhook: one POST carrying a flat map of
//! merge variables, which the plugin's Liquid template lays out. The module owns the payload,
//! never the layout — a screen layout lives in the TRMNL dashboard, where its author can change
//! it without republishing anything here.

mod commands;
mod config;
mod connectors;
mod events;
mod host;
mod i18n;
mod ids;
mod payload;
mod push;
mod queries;
mod tasks;

pub use commands::{on_config_updated, push_now, ConfigUpdatedArgs};
pub use config::{DisplayMode, ModuleConfig};
pub use events::on_booking_confirmed;
pub use host::render_host_main;
pub use ids::module_id;
pub use payload::{build_payload, StaySnapshot};
pub use push::{plugin_id_from_webhook_url, PushState};
pub use queries::get_status;
pub use tasks::timeline_tasks;

use portaki_sdk::prelude::*;

portaki_sdk::portaki_module!(
    id = "trmnl",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "Cyril Colinet",
);

/// Settings and the push ledger both live in the module's own KV namespace.
#[portaki_sdk::capability(required, id = "core.storage")]
pub const STORAGE: CapabilityId = capability::core::STORAGE;
