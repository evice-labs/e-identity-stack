use e_moderation_sdk::crypto::signature::{PublicKey, Signature};
use sha2::{Digest, Sha256};

use crate::identity::registration::IdentityError;

/// Validate username syntax: 3..=32 characters, ASCII alphanumeric or underscore.
pub fn validate_username_format(username: &str) -> Result<(), IdentityError> {
    if username.len() < 3 || username.len() > 32 {
        return Err(IdentityError::InvalidUsername(
            "Username must be between 3 and 32 characters",
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(IdentityError::InvalidUsername(
            "Username can only contain alphanumeric characters and underscores",
        ));
    }
    Ok(())
}

/// Verifies that a username change request is legitimate.
///
/// The owner must provide a Schnorr signature over SHA256(commitment || new_username)
/// using the private key corresponding to their commitment.
pub fn verify_username_change(
    commitment: &[u8; 32],
    new_username: &str,
    ownership_signature: &[u8; 64],
    owner_pubkey: &[u8; 32],
) -> Result<(), IdentityError> {
    validate_username_format(new_username)?;

    // Reconstruct the signed message
    let mut hasher = Sha256::new();
    hasher.update(commitment);
    hasher.update(new_username.as_bytes());
    let message: [u8; 32] = hasher.finalize().into();

    let pubkey = PublicKey::try_new(*owner_pubkey)
        .map_err(|_| IdentityError::SignatureVerificationFailed)?;
    let sig = Signature {
        value: *ownership_signature,
    };

    if !sig.is_valid_for(&message, &pubkey) {
        return Err(IdentityError::SignatureVerificationFailed);
    }

    Ok(())
}

/// In-memory username registry for tracking commitment ↔ username mapping.
/// Used by validators and nodes; on-chain state is the source of truth.
pub struct UsernameRegistry {
    entries: Vec<UsernameEntry>,
}

#[derive(Debug, Clone)]
pub struct UsernameEntry {
    pub commitment: [u8; 32],
    pub username: String,
}

impl UsernameRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a new username for a commitment.
    /// Returns error if the username is already taken (case-insensitive).
    pub fn register(
        &mut self,
        commitment: [u8; 32],
        username: String,
    ) -> Result<(), IdentityError> {
        validate_username_format(&username)?;

        if self
            .entries
            .iter()
            .any(|e| e.username.eq_ignore_ascii_case(&username))
        {
            return Err(IdentityError::InvalidUsername("Username already taken"));
        }

        self.entries.push(UsernameEntry {
            commitment,
            username,
        });
        Ok(())
    }

    /// Update the username for an existing commitment.
    /// The caller is responsible for verifying ownership proof before calling this.
    pub fn update(
        &mut self,
        commitment: &[u8; 32],
        new_username: String,
    ) -> Result<(), IdentityError> {
        validate_username_format(&new_username)?;

        if self
            .entries
            .iter()
            .any(|e| e.username.eq_ignore_ascii_case(&new_username) && e.commitment != *commitment)
        {
            return Err(IdentityError::InvalidUsername("Username already taken"));
        }

        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.commitment == *commitment)
        {
            entry.username = new_username;
            Ok(())
        } else {
            Err(IdentityError::InvalidUsername(
                "Commitment not found in registry",
            ))
        }
    }

    /// Lookup username by commitment.
    pub fn lookup_by_commitment(&self, commitment: &[u8; 32]) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.commitment == *commitment)
            .map(|e| e.username.as_str())
    }

    /// Lookup commitment by username (case-insensitive).
    pub fn lookup_by_username(&self, username: &str) -> Option<&[u8; 32]> {
        self.entries
            .iter()
            .find(|e| e.username.eq_ignore_ascii_case(username))
            .map(|e| &e.commitment)
    }
}

impl Default for UsernameRegistry {
    fn default() -> Self {
        Self::new()
    }
}
