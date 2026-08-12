use crate::moderation::anti_sybil::{check_room_diversity, AntiSybilConfig, ModerationError};
use crate::moderation::strike::validate_strike_certificate;
use crate::room::management::RoomRegistry;
use crate::room::maturity::is_room_mature;
use crate::room::moderator_registry::ModeratorRegistry;
use crate::types::ReleaseShareTx;

/// Validates a ReleaseShare transaction against all anti-Sybil rules.
///
/// This is the core validation logic that determines whether accumulated strikes
/// are sufficient to trigger NSK reconstruction and identity revocation.
///
/// Validation steps:
/// 1. Check K_strikes certificates present
/// 2. Check all certificates target same commitment
/// 3. Check >= K_rooms_min distinct room_ids
/// 4. Check each room meets maturity (age + member count)
/// 5. Check target has signed membership in each room
/// 6. Check no certificate replay (anti-replay via strike_index)
/// 7. Verify N_mod signatures per certificate
pub struct ReleaseShareValidator {
    pub k_strikes: u32,
    pub anti_sybil_config: AntiSybilConfig,
}

impl ReleaseShareValidator {
    pub fn new(k_strikes: u32, anti_sybil_config: AntiSybilConfig) -> Self {
        Self {
            k_strikes,
            anti_sybil_config,
        }
    }

    /// Validate a ReleaseShare transaction.
    pub fn validate(
        &self,
        tx: &ReleaseShareTx,
        room_registry: &RoomRegistry,
        moderator_registry: &ModeratorRegistry,
        current_index: u64,
        used_strike_indexes: &[[u8; 32]],
    ) -> Result<(), ModerationError> {
        // 1. Check K_strikes certificates present
        if tx.certificates.len() < self.k_strikes as usize {
            return Err(ModerationError::InsufficientStrikes {
                required: self.k_strikes,
                provided: tx.certificates.len() as u32,
            });
        }

        // 2. Check all certificates target same commitment
        for cert in &tx.certificates {
            if cert.target_commitment != tx.target_commitment {
                return Err(ModerationError::InconsistentTarget);
            }
        }

        // 3. Check >= K_rooms_min distinct room_ids
        check_room_diversity(&tx.certificates, &self.anti_sybil_config)?;

        for cert in &tx.certificates {
            // 4. Check each room meets maturity
            let room = room_registry
                .get_room(&cert.room_id)
                .ok_or(ModerationError::RoomNotMature("Room not found"))?;

            let member_count = room_registry.active_member_count(&cert.room_id);

            if !is_room_mature(room, current_index, member_count, &self.anti_sybil_config) {
                return Err(ModerationError::RoomNotMature(
                    "Room does not meet maturity requirements",
                ));
            }

            // 5. Check target has signed membership in each room
            if !room_registry.has_active_membership(&cert.room_id, &tx.target_commitment) {
                return Err(ModerationError::NoMembershipInRoom);
            }

            // 6. Anti-replay: check strike_index hasn't been used
            let mut strike_id_hasher = sha2::Sha256::new();
            use sha2::Digest;
            strike_id_hasher.update(&cert.room_id);
            strike_id_hasher.update(cert.strike_index.to_le_bytes());
            let strike_id: [u8; 32] = strike_id_hasher.finalize().into();

            if used_strike_indexes.contains(&strike_id) {
                return Err(ModerationError::CertificateReplay);
            }

            // 7. Verify N_mod signatures per certificate
            validate_strike_certificate(cert, room.n_mod_threshold, moderator_registry)?;
        }

        Ok(())
    }
}
