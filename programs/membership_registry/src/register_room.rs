use sha2::{Digest, Sha256};

use crate::state::{ForumInstance, OnChainRoom};

/// Process a room creation instruction.
///
/// Validates N_mod/M_mod parameters, derives a deterministic room_id,
/// and appends the room to on-chain state. Increments the monotonic index.
pub fn process_register_room(
    forum: &mut ForumInstance,
    admin_commitment: [u8; 32],
    n_mod_threshold: u32,
    m_mod_total: u32,
    moderator_pubkeys: Vec<[u8; 32]>,
    min_members_for_maturity: u32,
) -> Result<[u8; 32], &'static str> {
    if n_mod_threshold == 0 || m_mod_total == 0 {
        return Err("Moderator counts must be greater than zero");
    }
    if n_mod_threshold > m_mod_total {
        return Err("N_mod threshold cannot exceed M_mod total");
    }
    if moderator_pubkeys.len() != m_mod_total as usize {
        return Err("Number of moderator pubkeys must equal M_mod");
    }
    if !forum.registered_commitments.contains(&admin_commitment) {
        return Err("Admin commitment is not a registered identity");
    }
    if forum.revoked_commitments.contains(&admin_commitment) {
        return Err("Admin identity has been revoked");
    }

    let creation_index = forum.current_index;

    // Derive deterministic room_id
    let mut hasher = Sha256::new();
    hasher.update(&admin_commitment);
    hasher.update(creation_index.to_le_bytes());
    hasher.update(n_mod_threshold.to_le_bytes());
    hasher.update(m_mod_total.to_le_bytes());
    let room_id: [u8; 32] = hasher.finalize().into();

    if forum.rooms.iter().any(|r| r.room_id == room_id) {
        return Err("Room with this ID already exists");
    }

    forum.rooms.push(OnChainRoom {
        room_id,
        admin_commitment,
        n_mod_threshold,
        m_mod_total,
        moderator_pubkeys,
        creation_index,
        min_members_for_maturity,
    });

    forum.current_index += 1;

    Ok(room_id)
}
