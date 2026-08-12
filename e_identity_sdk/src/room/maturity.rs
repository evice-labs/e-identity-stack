use crate::moderation::anti_sybil::AntiSybilConfig;
use crate::types::RoomConfig;

/// Check if a room qualifies as "mature" for strike purposes.
///
/// A room must meet both conditions:
/// 1. Age: current_index - creation_index >= min_room_age_indexes
/// 2. Population: current active member count >= min_room_members
pub fn is_room_mature(
    room: &RoomConfig,
    current_index: u64,
    current_member_count: u32,
    config: &AntiSybilConfig,
) -> bool {
    let age = current_index.saturating_sub(room.creation_index);
    age >= config.min_room_age_indexes && current_member_count >= config.min_room_members
}
