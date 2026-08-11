use sha2::{Digest, Sha256};

use crate::state::ForumInstance;

/// NSK type alias (same as lee_core::NullifierSecretKey = [u8; 32]).
pub type NullifierSecretKey = [u8; 32];

/// Derive the commitment bytes from NSK using the same derivation as lee_core:
/// NPK = SHA256("LEE/keys" || NSK || [7] || [0; 23])
/// Commitment = SHA256(NPK) — simplified for on-chain identity.
fn derive_commitment_from_nsk(nsk: &NullifierSecretKey) -> [u8; 32] {
    let mut npk_input = Vec::new();
    npk_input.extend_from_slice(b"LEE/keys");
    npk_input.extend_from_slice(nsk);
    npk_input.push(7);
    npk_input.extend_from_slice(&[0u8; 23]);

    let npk: [u8; 32] = Sha256::digest(&npk_input).into();

    Sha256::digest(npk).into()
}

pub fn process_slash(
    forum: &mut ForumInstance,
    slashed_nsk: &NullifierSecretKey,
) -> Result<u64, &'static str> {
    let comm_bytes = derive_commitment_from_nsk(slashed_nsk);

    if !forum.registered_commitments.contains(&comm_bytes) {
        return Err("Slashing failed: NSK does not correspond to any registered member.");
    }

    if forum.revoked_commitments.contains(&comm_bytes) {
        return Err("Slashing failed: This member's access has already been revoked.");
    }

    let confiscated = forum
        .member_stakes
        .iter()
        .find(|(c, _)| c == &comm_bytes)
        .map(|(_, s)| *s)
        .unwrap_or(0);

    forum.revoked_commitments.push(comm_bytes);

    if forum.total_staked >= confiscated {
        forum.total_staked -= confiscated;
    }

    Ok(confiscated)
}
