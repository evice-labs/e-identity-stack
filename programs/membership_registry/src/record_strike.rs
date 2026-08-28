use sha2::{Digest, Sha256};

use crate::state::{ForumInstance, OnChainStrike};

/// Process a strike recording instruction.
///
/// Validates that:
/// 1. The room exists on-chain
/// 2. The target commitment is registered and not revoked
/// 3. The target has an active membership in the specified room
/// 4. The strike hasn't been recorded before (anti-replay via room_id + strike_index)
/// 5. Moderator signature count meets the room's N_mod threshold
///
/// Note: BIP-340 signature verification is done off-chain by the SDK's
/// `validate_strike_certificate()`. The on-chain program trusts the
/// pre-validated `n_valid_sigs` count submitted by the sequencer.
pub fn process_record_strike(
    forum: &mut ForumInstance,
    room_id: [u8; 32],
    target_commitment: [u8; 32],
    evidence_hash: [u8; 32],
    n_valid_sigs: u32,
) -> Result<u64, &'static str> {
    let room = forum
        .rooms
        .iter()
        .find(|r| r.room_id == room_id)
        .ok_or("Room not found")?;

    let n_mod_threshold = room.n_mod_threshold;

    if !forum.registered_commitments.contains(&target_commitment) {
        return Err("Target commitment is not registered");
    }
    if forum.revoked_commitments.contains(&target_commitment) {
        return Err("Target identity is already revoked");
    }

    let has_membership = forum
        .room_memberships
        .iter()
        .any(|m| m.room_id == room_id && m.member_commitment == target_commitment && m.is_active);
    if !has_membership {
        return Err("Target has no active membership in this room");
    }

    // Anti-replay: SHA256(room_id || strike_index) must not already exist
    let strike_index = forum.current_index;
    let mut replay_hasher = Sha256::new();
    replay_hasher.update(&room_id);
    replay_hasher.update(strike_index.to_le_bytes());
    let strike_id: [u8; 32] = replay_hasher.finalize().into();

    if forum.used_tracing_tags.contains(&strike_id) {
        return Err("Strike already recorded (replay detected)");
    }

    if n_valid_sigs < n_mod_threshold {
        return Err("Insufficient moderator signatures for this room's threshold");
    }
    forum.recorded_strikes.push(OnChainStrike {
        room_id,
        target_commitment,
        evidence_hash,
        strike_index,
        n_valid_sigs,
    });

    forum.used_tracing_tags.push(strike_id);
    forum.current_index += 1;

    Ok(strike_index)
}
