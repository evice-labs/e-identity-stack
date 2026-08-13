use borsh::{BorshDeserialize, BorshSerialize};
use spel_framework::prelude::account_type;

#[account_type]
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct ForumInstance {
    pub admin_pubkey: [u8; 32],
    pub k_strikes: u32,
    pub n_moderators: u32,
    pub m_moderators: u32,
    pub registered_commitments: Vec<[u8; 32]>,
    pub revoked_commitments: Vec<[u8; 32]>,
    pub total_staked: u64,
    pub member_stakes: Vec<([u8; 32], u64)>,
    pub used_tracing_tags: Vec<[u8; 32]>,
    pub rooms: Vec<OnChainRoom>,
    pub room_memberships: Vec<OnChainMembership>,
    pub recorded_strikes: Vec<OnChainStrike>,
    /// Monotonic index counter for ordering.
    pub current_index: u64,
}

/// On-chain room configuration (mirrors e_identity_sdk::types::RoomConfig).
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct OnChainRoom {
    pub room_id: [u8; 32],
    pub admin_commitment: [u8; 32],
    pub n_mod_threshold: u32,
    pub m_mod_total: u32,
    pub moderator_pubkeys: Vec<[u8; 32]>,
    pub creation_index: u64,
    pub min_members_for_maturity: u32,
}

/// On-chain room membership record.
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct OnChainMembership {
    pub room_id: [u8; 32],
    pub member_commitment: [u8; 32],
    pub join_index: u64,
    pub is_active: bool,
}

/// On-chain recorded strike.
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct OnChainStrike {
    pub room_id: [u8; 32],
    pub target_commitment: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub strike_index: u64,
    pub n_valid_sigs: u32,
}
