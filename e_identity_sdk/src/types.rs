use serde::{Deserialize, Serialize};

/// User identity record stored in the on-chain registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub commitment: [u8; 32],
    pub current_username: String,
    pub encrypted_sss_shares: Vec<EncryptedShare>,
    pub registration_index: u64,
    pub is_revoked: bool,
}

/// An SSS share encrypted to a specific node's public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedShare {
    pub node_pubkey: [u8; 32],
    pub ephemeral_pk: [u8; 32],
    pub ciphertext: Vec<u8>,
}

/// Room configuration stored in the on-chain registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub room_id: [u8; 32],
    pub admin_commitment: [u8; 32],
    pub n_mod_threshold: u32,
    pub m_mod_total: u32,
    pub moderator_pubkeys: Vec<[u8; 32]>,
    pub creation_index: u64,
    pub min_members_for_maturity: u32,
}

/// Room membership record with signed consent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipRecord {
    pub room_id: [u8; 32],
    pub member_commitment: [u8; 32],
    pub join_signature: [u8; 64],
    pub join_index: u64,
    /// False = left or removed. Record retained for audit.
    pub is_active: bool,
}

/// Strike certificate — per room, per user. Requires N-of-M moderator signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrikeCertificate {
    pub room_id: [u8; 32],
    pub target_commitment: [u8; 32],
    pub moderator_signatures: Vec<ModeratorSig>,
    pub strike_index: u64,
    pub evidence_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeratorSig {
    pub pubkey: [u8; 32],
    pub signature: [u8; 64],
}

/// ReleaseShare transaction — triggers NSK reconstruction when K_strikes is met.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseShareTx {
    pub target_commitment: [u8; 32],
    /// Must contain exactly K_strikes certificates.
    /// Validated: ≥ K_rooms_min distinct room_ids.
    /// Validated: each room meets maturity.
    /// Validated: target has signed membership in each room.
    pub certificates: Vec<StrikeCertificate>,
}
