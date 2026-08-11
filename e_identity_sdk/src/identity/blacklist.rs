use crate::identity::registration::IdentityError;

/// Global blacklist of revoked commitments.
///
/// When a user accumulates K_strikes and their NSK is exposed via ReleaseShare,
/// their commitment is added to this blacklist. Blacklisted commitments are
/// rejected from all future operations (registration, room join, messaging).
pub struct Blacklist {
    revoked_commitments: Vec<[u8; 32]>,
}

impl Blacklist {
    pub fn new() -> Self {
        Self {
            revoked_commitments: Vec::new(),
        }
    }

    /// Load blacklist from a list of already-revoked commitments (e.g. from on-chain state).
    pub fn from_revoked(commitments: Vec<[u8; 32]>) -> Self {
        Self {
            revoked_commitments: commitments,
        }
    }

    /// Add a commitment to the blacklist.
    /// Returns error if already blacklisted.
    pub fn revoke(&mut self, commitment: [u8; 32]) -> Result<(), IdentityError> {
        if self.is_revoked(&commitment) {
            return Err(IdentityError::InvalidUsername("Commitment already revoked"));
        }
        self.revoked_commitments.push(commitment);
        Ok(())
    }

    /// Check if a commitment is blacklisted.
    pub fn is_revoked(&self, commitment: &[u8; 32]) -> bool {
        self.revoked_commitments.contains(commitment)
    }

    /// Returns the full list of revoked commitments.
    pub fn revoked_commitments(&self) -> &[[u8; 32]] {
        &self.revoked_commitments
    }

    /// Number of revoked identities.
    pub fn len(&self) -> usize {
        self.revoked_commitments.len()
    }

    /// Whether the blacklist is empty.
    pub fn is_empty(&self) -> bool {
        self.revoked_commitments.is_empty()
    }
}

impl Default for Blacklist {
    fn default() -> Self {
        Self::new()
    }
}
