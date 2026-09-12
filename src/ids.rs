//! Typed surface / operation catalogs for this module.

use portaki_sdk::prelude::*;

define_surface_ids! {
    HOST_MAIN = "main",
    HOST_STATUS = "status",
}

define_operation_names! {
    UPDATE_CONFIG = "updateConfig",
    PUSH_NOW = "pushNow",
    GET_CONFIG = "getConfig",
    GET_STATUS = "getStatus",
}

// Must stay aligned with `portaki_sdk::contracts::platform::BOOKING_CONFIRMED`.
define_event_types! {
    BOOKING_CONFIRMED = "core.booking.confirmed",
}

pub fn module_id() -> ModuleId {
    ModuleId::from_static("trmnl")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire string is the contract: the platform routes on it, not on the constant's name.
    #[test]
    fn booking_confirmed_matches_the_platform_contract() {
        assert_eq!(
            BOOKING_CONFIRMED.as_str(),
            portaki_sdk::contracts::platform::BOOKING_CONFIRMED.as_str()
        );
    }
}
