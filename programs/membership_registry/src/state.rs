use borsh::{BorshDeserialize, BorshSerialize};
use spel_framework::prelude::account_type;

#[account_type]
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct ForumInstance {
    pub admin_pubkey: [u8; 32],
    pub k_strikes: u32,
    pub n_moderators: u32,
    pub m_moderators: u32,
    /// Commitment registry — replaces MerkleTree (not available in lee_core).
    /// For on-chain identity, commitments are tracked as a flat list.
    /// LEZ state machine handles Merkle commitment set at the sequencer level.
    pub registered_commitments: Vec<[u8; 32]>,
    pub revoked_commitments: Vec<[u8; 32]>,
    pub total_staked: u64,
    pub member_stakes: Vec<([u8; 32], u64)>,
    pub used_tracing_tags: Vec<[u8; 32]>,
}
