//! Platform events this module reacts to.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::push;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingConfirmedEvent {
    pub id: Uuid,
    pub property_id: Uuid,
}

// The wire string must match `ids::BOOKING_CONFIRMED`: macros need the literal at expand time.
/// A confirmed booking is the one moment a screen is certainly stale.
#[portaki_sdk::event_handler(event_type = "core.booking.confirmed")]
pub fn on_booking_confirmed(ctx: Context, _event: BookingConfirmedEvent) -> Result<()> {
    let _ = push::push_now(&ctx, "core.booking.confirmed");
    Ok(())
}
