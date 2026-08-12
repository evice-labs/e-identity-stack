use crate::room::management::RoomError;
use crate::types::RoomConfig;

/// Per-room moderator registry.
///
/// Tracks which public keys are authorized moderators for a specific room,
/// and enforces the N-of-M threshold configuration.
pub struct ModeratorRegistry {
    entries: Vec<ModeratorEntry>,
}

#[derive(Debug, Clone)]
pub struct ModeratorEntry {
    pub room_id: [u8; 32],
    pub moderator_pubkey: [u8; 32],
    pub is_active: bool,
}

impl ModeratorRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register moderators for a room from its RoomConfig.
    /// This should be called after room creation.
    pub fn register_from_config(&mut self, config: &RoomConfig) {
        for pubkey in &config.moderator_pubkeys {
            if !self.entries.iter().any(|e| {
                e.room_id == config.room_id && e.moderator_pubkey == *pubkey
            }) {
                self.entries.push(ModeratorEntry {
                    room_id: config.room_id,
                    moderator_pubkey: *pubkey,
                    is_active: true,
                });
            }
        }
    }

    /// Check if a pubkey is an active moderator for a given room.
    pub fn is_moderator(&self, room_id: &[u8; 32], pubkey: &[u8; 32]) -> bool {
        self.entries
            .iter()
            .any(|e| e.room_id == *room_id && e.moderator_pubkey == *pubkey && e.is_active)
    }

    /// Get all active moderator pubkeys for a room.
    pub fn active_moderators(&self, room_id: &[u8; 32]) -> Vec<[u8; 32]> {
        self.entries
            .iter()
            .filter(|e| e.room_id == *room_id && e.is_active)
            .map(|e| e.moderator_pubkey)
            .collect()
    }

    /// Deactivate a moderator for a room.
    pub fn deactivate(
        &mut self,
        room_id: &[u8; 32],
        pubkey: &[u8; 32],
    ) -> Result<(), RoomError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.room_id == *room_id && e.moderator_pubkey == *pubkey && e.is_active)
            .ok_or(RoomError::MemberNotFound)?;
        entry.is_active = false;
        Ok(())
    }

    /// Count active moderators for a room.
    pub fn active_count(&self, room_id: &[u8; 32]) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.room_id == *room_id && e.is_active)
            .count() as u32
    }
}

impl Default for ModeratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
