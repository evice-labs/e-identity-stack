use e_moderation_sdk::types::EncryptedSharePerPost;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    #[error("Moderation SDK error: {0}")]
    Moderation(String),
    #[error("Identity SDK error: {0}")]
    Identity(String),
    #[error("MLS error: {0}")]
    Mls(String),
    #[error("Invalid payload format or corrupted message")]
    InvalidPayload,
    #[error("Tracing tag mismatch")]
    TracingTagMismatch,
    #[error("Room ID mismatch: expected {expected}, got {got}")]
    RoomIdMismatch { expected: String, got: String },
    #[error("Key package error: {0}")]
    KeyPackage(String),
}

/// A protected message container combining MLS End-to-End Encryption with
/// e-identity-stack Two-Tier SSS accountability.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProtectedPayload {
    pub plaintext: Vec<u8>,
    pub tracing_tag: [u8; 32],
    pub x_index: u8,
    pub encrypted_shares: Vec<EncryptedSharePerPost>,
    pub sender_commitment: [u8; 32],
    pub sender_username: String,
    pub timestamp_ms: u64,
}

impl ProtectedPayload {
    /// Serialize the protected payload into JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, BridgeError> {
        serde_json::to_vec(self).map_err(|e| BridgeError::Serialization(e.to_string()))
    }

    /// Deserialize a protected payload from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BridgeError> {
        serde_json::from_slice(bytes).map_err(|e| BridgeError::Deserialization(e.to_string()))
    }
}
