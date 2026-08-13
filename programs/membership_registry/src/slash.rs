use std::collections::HashSet;

use crate::state::ForumInstance;

/// Process a slash identity instruction.
///
/// Validates that the target has accumulated enough strikes from diverse,
/// mature rooms, then revokes the identity and confiscates the stake.
///
/// Validation steps:
/// 1. Target must be registered and not already revoked
/// 2. Must have >= k_strikes recorded strikes against the target
/// 3. Strikes must come from >= k_rooms_min distinct rooms
/// 4. Each room must meet maturity requirements (age + member count)
/// 5. Revoke the commitment and confiscate stake
pub fn process_slash(
    forum: &mut ForumInstance,
    target_commitment: [u8; 32],
    k_rooms_min: u32,
    min_room_age_indexes: u64,
    min_room_members: u32,
) -> Result<u64, &'static str> {
    if !forum.registered_commitments.contains(&target_commitment) {
        return Err("Target commitment is not registered");
    }
    if forum.revoked_commitments.contains(&target_commitment) {
        return Err("Target identity is already revoked");
    }

    let target_strikes: Vec<_> = forum
        .recorded_strikes
        .iter()
        .filter(|s| s.target_commitment == target_commitment)
        .collect();

    if target_strikes.len() < forum.k_strikes as usize {
        return Err("Insufficient strikes for slashing");
    }

    let distinct_rooms: HashSet<[u8; 32]> = target_strikes.iter().map(|s| s.room_id).collect();
    if distinct_rooms.len() < k_rooms_min as usize {
        return Err("Strikes lack sufficient room diversity");
    }

    let current_index = forum.current_index;
    for room_id in &distinct_rooms {
        let room = forum
            .rooms
            .iter()
            .find(|r| r.room_id == *room_id)
            .ok_or("Strike references a non-existent room")?;

        let age = current_index.saturating_sub(room.creation_index);
        if age < min_room_age_indexes {
            return Err("Strike room does not meet age maturity requirement");
        }

        let active_members = forum
            .room_memberships
            .iter()
            .filter(|m| m.room_id == *room_id && m.is_active)
            .count() as u32;

        if active_members < min_room_members {
            return Err("Strike room does not meet member count maturity requirement");
        }
    }

    forum.revoked_commitments.push(target_commitment);

    let confiscated = forum
        .member_stakes
        .iter()
        .find(|(c, _)| c == &target_commitment)
        .map(|(_, s)| *s)
        .unwrap_or(0);

    if forum.total_staked >= confiscated {
        forum.total_staked -= confiscated;
    }

    // Deactivate all memberships for revoked identity
    for membership in forum.room_memberships.iter_mut() {
        if membership.member_commitment == target_commitment {
            membership.is_active = false;
        }
    }

    Ok(confiscated)
}
