use sha2::{Digest, Sha256};
use e_moderation_sdk::crypto::signature::{PrivateKey, PublicKey, Signature};

use crate::identity::IdentityError;
use crate::types::{MembershipRecord, RoomConfig};

#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Room already exists: {0}")]
    RoomAlreadyExists(&'static str),
    #[error("Room not found")]
    RoomNotFound,
    #[error("Invalid room config: {0}")]
    InvalidConfig(&'static str),
    #[error("Member already joined")]
    MemberAlreadyJoined,
    #[error("Member not found in room")]
    MemberNotFound,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),
}

/// In-memory room registry that tracks rooms and memberships.
pub struct RoomRegistry {
    rooms: Vec<RoomConfig>,
    memberships: Vec<MembershipRecord>,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            memberships: Vec::new(),
        }
    }

    /// Create a new room.
    ///
    /// `room_id` is SHA256(admin_commitment || creation_index || n_mod || m_mod).
    /// Returns the created RoomConfig.
    pub fn create_room(
        &mut self,
        admin_commitment: [u8; 32],
        n_mod_threshold: u32,
        m_mod_total: u32,
        moderator_pubkeys: Vec<[u8; 32]>,
        creation_index: u64,
        min_members_for_maturity: u32,
    ) -> Result<RoomConfig, RoomError> {
        if n_mod_threshold == 0 || m_mod_total == 0 {
            return Err(RoomError::InvalidConfig(
                "Moderator counts must be greater than zero",
            ));
        }
        if n_mod_threshold > m_mod_total {
            return Err(RoomError::InvalidConfig(
                "N_mod threshold cannot exceed M_mod total",
            ));
        }
        if moderator_pubkeys.len() != m_mod_total as usize {
            return Err(RoomError::InvalidConfig(
                "Number of moderator pubkeys must equal M_mod",
            ));
        }

        // Derive deterministic room_id
        let mut hasher = Sha256::new();
        hasher.update(&admin_commitment);
        hasher.update(creation_index.to_le_bytes());
        hasher.update(n_mod_threshold.to_le_bytes());
        hasher.update(m_mod_total.to_le_bytes());
        let room_id: [u8; 32] = hasher.finalize().into();

        if self.rooms.iter().any(|r| r.room_id == room_id) {
            return Err(RoomError::RoomAlreadyExists(
                "Room with this configuration already exists",
            ));
        }

        let room = RoomConfig {
            room_id,
            admin_commitment,
            n_mod_threshold,
            m_mod_total,
            moderator_pubkeys,
            creation_index,
            min_members_for_maturity,
        };

        self.rooms.push(room.clone());
        Ok(room)
    }

    /// Join a room with signed consent.
    ///
    /// The member signs SHA256(room_id || member_commitment) with their NSK-derived
    /// Schnorr key to prove they consent to join this specific room.
    pub fn join_room(
        &mut self,
        room_id: &[u8; 32],
        member_commitment: [u8; 32],
        member_pubkey: &[u8; 32],
        join_signature: [u8; 64],
        join_index: u64,
    ) -> Result<(), RoomError> {
        if !self.rooms.iter().any(|r| r.room_id == *room_id) {
            return Err(RoomError::RoomNotFound);
        }

        // Check not already an active member
        if self.memberships.iter().any(|m| {
            m.room_id == *room_id && m.member_commitment == member_commitment && m.is_active
        }) {
            return Err(RoomError::MemberAlreadyJoined);
        }

        // Verify join consent signature: SHA256(room_id || member_commitment)
        let mut hasher = Sha256::new();
        hasher.update(room_id);
        hasher.update(&member_commitment);
        let message: [u8; 32] = hasher.finalize().into();

        let pubkey = PublicKey::try_new(*member_pubkey)
            .map_err(|_| RoomError::SignatureVerificationFailed)?;
        let sig = Signature {
            value: join_signature,
        };
        if !sig.is_valid_for(&message, &pubkey) {
            return Err(RoomError::SignatureVerificationFailed);
        }

        self.memberships.push(MembershipRecord {
            room_id: *room_id,
            member_commitment,
            join_signature,
            join_index,
            is_active: true,
        });

        Ok(())
    }

    /// Prepare a join consent signature (helper for client-side).
    pub fn sign_join_consent(
        room_id: &[u8; 32],
        member_commitment: &[u8; 32],
        nsk: &[u8; 32],
    ) -> Result<[u8; 64], RoomError> {
        let mut hasher = Sha256::new();
        hasher.update(room_id);
        hasher.update(member_commitment);
        let message: [u8; 32] = hasher.finalize().into();

        let private_key =
            PrivateKey::try_new(*nsk).map_err(|_| RoomError::SignatureVerificationFailed)?;
        let signature = Signature::new(&private_key, &message);
        Ok(signature.value)
    }

    /// Remove a member from a room (set is_active = false).
    pub fn leave_room(
        &mut self,
        room_id: &[u8; 32],
        member_commitment: &[u8; 32],
    ) -> Result<(), RoomError> {
        let record = self
            .memberships
            .iter_mut()
            .find(|m| m.room_id == *room_id && m.member_commitment == *member_commitment && m.is_active)
            .ok_or(RoomError::MemberNotFound)?;
        record.is_active = false;
        Ok(())
    }

    /// Lookup a room by its ID.
    pub fn get_room(&self, room_id: &[u8; 32]) -> Option<&RoomConfig> {
        self.rooms.iter().find(|r| r.room_id == *room_id)
    }

    /// Count active members in a room.
    pub fn active_member_count(&self, room_id: &[u8; 32]) -> u32 {
        self.memberships
            .iter()
            .filter(|m| m.room_id == *room_id && m.is_active)
            .count() as u32
    }

    /// Check if a commitment has an active membership in a room.
    pub fn has_active_membership(
        &self,
        room_id: &[u8; 32],
        member_commitment: &[u8; 32],
    ) -> bool {
        self.memberships
            .iter()
            .any(|m| m.room_id == *room_id && m.member_commitment == *member_commitment && m.is_active)
    }

    /// Get all rooms.
    pub fn rooms(&self) -> &[RoomConfig] {
        &self.rooms
    }

    /// Get all memberships for a room.
    pub fn memberships_for_room(&self, room_id: &[u8; 32]) -> Vec<&MembershipRecord> {
        self.memberships
            .iter()
            .filter(|m| m.room_id == *room_id)
            .collect()
    }
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self::new()
    }
}
