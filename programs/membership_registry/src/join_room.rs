use crate::state::{ForumInstance, OnChainMembership};

/// Process a room join instruction. 
/// Validates that:
/// 1. The room exists on-chain
/// 2. The member commitment is registered and not revoked
/// 3. The member doesn't already have an active membership in this room
///
/// Then appends a new `OnChainMembership` record and increments the
/// monotonic index counter.
pub fn process_join_room(
    forum: &mut ForumInstance,
    room_id: [u8; 32],
    member_commitment: [u8; 32],
) -> Result<(), &'static str> {
    // 1. Room must exist
    if !forum.rooms.iter().any(|r| r.room_id == room_id) {
        return Err("Room not found");
    }

    // 2. Member must be registered and not revoked
    if !forum.registered_commitments.contains(&member_commitment) {
        return Err("Member commitment is not registered");
    }
    if forum.revoked_commitments.contains(&member_commitment) {
        return Err("Member identity has been revoked");
    }

    // 3. No duplicate active membership
    let already_member = forum
        .room_memberships
        .iter()
        .any(|m| m.room_id == room_id && m.member_commitment == member_commitment && m.is_active);
    if already_member {
        return Err("Member already has an active membership in this room");
    }

    // Add membership
    forum.room_memberships.push(OnChainMembership {
        room_id,
        member_commitment,
        join_index: forum.current_index,
        is_active: true,
    });

    forum.current_index += 1;

    Ok(())
}
