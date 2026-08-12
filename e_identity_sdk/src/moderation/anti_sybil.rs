use std::collections::HashSet;

use crate::types::StrikeCertificate;

/// Anti-Sybil configuration for the identity accountability system.
///
/// These parameters prevent attackers from creating puppet rooms
/// to accumulate fake strikes against a target.
#[derive(Debug, Clone)]
pub struct AntiSybilConfig {
    pub k_rooms_min: u32,
    pub min_room_age_indexes: u64,
    pub min_room_members: u32,
    pub require_signed_join_consent: bool,
}

impl AntiSybilConfig {
    /// Default production configuration.
    pub fn default_production() -> Self {
        Self {
            k_rooms_min: 3,
            min_room_age_indexes: 100,
            min_room_members: 10,
            require_signed_join_consent: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModerationError {
    #[error("Insufficient strike certificates: need {required}, got {provided}")]
    InsufficientStrikes { required: u32, provided: u32 },
    #[error("Insufficient room diversity: need {required} distinct rooms, got {provided}")]
    InsufficientRoomDiversity { required: u32, provided: u32 },
    #[error("Room is not mature: {0}")]
    RoomNotMature(&'static str),
    #[error("Target has no membership in room")]
    NoMembershipInRoom,
    #[error("Certificate replay detected")]
    CertificateReplay,
    #[error("Certificates target different commitments")]
    InconsistentTarget,
    #[error("Insufficient moderator signatures: need {required}, got {provided}")]
    InsufficientModeratorSigs { required: u32, provided: u32 },
    #[error("Invalid moderator signature")]
    InvalidModeratorSignature,
    #[error("Moderator not registered for room")]
    ModeratorNotRegistered,
}

/// Check room diversity requirement across strike certificates.
///
/// Ensures that strikes originate from at least `k_rooms_min` distinct rooms
/// to prevent puppet-room Sybil attacks.
pub fn check_room_diversity(
    certificates: &[StrikeCertificate],
    config: &AntiSybilConfig,
) -> Result<(), ModerationError> {
    let distinct_rooms: HashSet<[u8; 32]> = certificates.iter().map(|c| c.room_id).collect();
    if distinct_rooms.len() < config.k_rooms_min as usize {
        return Err(ModerationError::InsufficientRoomDiversity {
            required: config.k_rooms_min,
            provided: distinct_rooms.len() as u32,
        });
    }
    Ok(())
}
