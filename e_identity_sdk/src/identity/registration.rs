use e_moderation_sdk::crypto::ecdh;
use e_moderation_sdk::crypto::sss::split_secret;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sha2::{Digest, Sha256};

use crate::types::EncryptedShare;

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("SSS split failed: {0}")]
    SssSplitFailed(&'static str),
    #[error("ECDH encryption failed: {0}")]
    EcdhFailed(&'static str),
    #[error("Invalid username: {0}")]
    InvalidUsername(&'static str),
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
}

/// Payload returned by `prepare_registration()` for on-chain submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationPayload {
    pub commitment: [u8; 32],
    pub username: String,
    pub encrypted_shares: Vec<EncryptedShare>,
}

/// Payload returned by `prepare_username_change()` for on-chain submission.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernameChangePayload {
    pub commitment: [u8; 32],
    pub new_username: String,
    /// Schnorr signature over SHA256(commitment || new_username) proving ownership.
    #[serde_as(as = "[_; 64]")]
    pub ownership_signature: [u8; 64],
}

/// Client for managing user identity lifecycle.
///
/// Holds the NSK and derives the commitment.
/// Reuses `e_moderation_sdk::crypto::sss::split_secret()` for SSS
/// and `e_moderation_sdk::crypto::ecdh::compute_shared_secret()` for
/// encrypting shares to node public keys.
pub struct RegistrationClient {
    nsk: [u8; 32],
    commitment: [u8; 32],
}

impl RegistrationClient {
    /// Generate a new identity with a random NSK.
    /// Commitment = SHA256(NSK).
    pub fn new() -> Self {
        let mut nsk = [0u8; 32];
        rand::rng().fill_bytes(&mut nsk);
        let commitment: [u8; 32] = Sha256::digest(&nsk).into();
        Self { nsk, commitment }
    }

    /// Create a RegistrationClient from an existing NSK (e.g. restored from backup).
    pub fn from_nsk(nsk: [u8; 32]) -> Self {
        let commitment: [u8; 32] = Sha256::digest(&nsk).into();
        Self { nsk, commitment }
    }

    /// Returns the public commitment (can be shared freely).
    pub fn commitment(&self) -> &[u8; 32] {
        &self.commitment
    }

    /// Returns a reference to the NSK (sensitive, handle with care).
    pub fn nsk(&self) -> &[u8; 32] {
        &self.nsk
    }

    /// Prepare registration payload with SSS shares encrypted to node pubkeys.
    ///
    /// Flow:
    /// 1. Split NSK into M shares with threshold K via Shamir (GF(256))
    /// 2. For each share, generate ephemeral ECDH keypair
    /// 3. Encrypt share to corresponding node pubkey via XOR keystream
    /// 4. Return payload for on-chain submission
    pub fn prepare_registration(
        &self,
        username: &str,
        node_pubkeys: &[[u8; 32]],
        k_sss_threshold: u32,
    ) -> Result<RegistrationPayload, IdentityError> {
        if username.is_empty() || username.len() > 64 {
            return Err(IdentityError::InvalidUsername(
                "Username must be between 1 and 64 characters",
            ));
        }

        let total_nodes = node_pubkeys.len() as u32;
        let shares = split_secret(&self.nsk, k_sss_threshold, total_nodes)
            .map_err(IdentityError::SssSplitFailed)?;

        let mut encrypted_shares = Vec::with_capacity(shares.len());

        for (i, share) in shares.iter().enumerate() {
            let ephemeral_sk = ecdh::generate_ephemeral_scalar();
            let ephemeral_pk = ecdh::derive_xonly_pubkey(&ephemeral_sk);

            let shared_secret = ecdh::compute_shared_secret(&ephemeral_sk, &node_pubkeys[i])
                .map_err(IdentityError::EcdhFailed)?;

            let mut ciphertext = share.clone();
            ecdh::xor_encrypt(&mut ciphertext, &shared_secret, 0);

            encrypted_shares.push(EncryptedShare {
                node_pubkey: node_pubkeys[i],
                ephemeral_pk,
                ciphertext,
            });
        }

        Ok(RegistrationPayload {
            commitment: self.commitment,
            username: username.to_string(),
            encrypted_shares,
        })
    }

    /// Prepare username change proof.
    ///
    /// Signs SHA256(commitment || new_username) with NSK-derived Schnorr key to prove commitment ownership.
    pub fn prepare_username_change(
        &self,
        new_username: &str,
    ) -> Result<UsernameChangePayload, IdentityError> {
        if new_username.is_empty() || new_username.len() > 64 {
            return Err(IdentityError::InvalidUsername(
                "Username must be between 1 and 64 characters",
            ));
        }

        // Construct message: SHA256(commitment || new_username)
        let mut hasher = Sha256::new();
        hasher.update(&self.commitment);
        hasher.update(new_username.as_bytes());
        let message: [u8; 32] = hasher.finalize().into();

        // Sign with BIP-340 Schnorr using NSK as the private key
        let private_key = e_moderation_sdk::crypto::signature::PrivateKey::try_new(self.nsk)
            .map_err(|_| IdentityError::InvalidUsername("Invalid NSK private key"))?;
        let signature = e_moderation_sdk::crypto::signature::Signature::new(&private_key, &message);

        Ok(UsernameChangePayload {
            commitment: self.commitment,
            new_username: new_username.to_string(),
            ownership_signature: signature.value,
        })
    }
}
