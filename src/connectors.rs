//! TRMNL connector declaration.
//!
//! A module cannot open a socket: the only egress from wasm is a connector declared here and
//! executed by the runtime. `base_url` is compile-time metadata, so the host configures the
//! *plugin*, never the host name — which is also what keeps this module from becoming a way to
//! POST anywhere.

/// `auth = "none"`: the plugin id in the path is the whole secret, TRMNL wants no header.
#[portaki_sdk::custom_connector(
    id = "trmnl",
    display_name_key = "connector.trmnl.name",
    base_url = "https://usetrmnl.com",
    credential_provider_id = "trmnl",
    auth = "none"
)]
#[allow(dead_code)] // metadata-only: the macro emits the manifest entry at compile time.
pub struct ModuleTrmnl;

#[allow(dead_code)]
impl ModuleTrmnl {
    /// Remaining args after `{plugin_id}` become the JSON body — here, `merge_variables`.
    #[portaki_sdk::connector_op(method = "POST", path = "/api/custom_plugins/{plugin_id}")]
    pub fn push() {}
}
