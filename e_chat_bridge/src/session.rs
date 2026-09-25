use de_mls::mls_crypto::{MlsCommitInput, MlsService};
use de_mls::protos::de_mls::messages::v1::{app_message::Payload, AppMessage, ConversationMessage};
use e_moderation_sdk::MemberClient;
use openmls::credentials::{BasicCredential, CredentialWithKey};
use openmls::group::MlsGroupCreateConfig;
use openmls::key_packages::KeyPackage;
use openmls::prelude::tls_codec::Serialize as _;
use openmls::prelude::Ciphersuite;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use prost::Message;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{BridgeError, ProtectedPayload};

pub const DEFAULT_CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// An end-to-end encrypted chat session integrating OpenMLS / de-MLS ratcheting
/// with e-identity-stack Zero-Knowledge identity and Two-Tier SSS accountability.
pub struct EChatSession {
    pub room_id: String,
    pub sender_commitment: [u8; 32],
    pub sender_username: String,
    pub provider: OpenMlsRustCrypto,
    pub signer: SignatureKeyPair,
    pub mls_service: MlsService,
    pub member_client: Option<MemberClient>,
}

impl EChatSession {
    /// Create a new protected chat room as the room creator / admin.
    pub fn create_room(
        room_id: &str,
        sender_commitment: [u8; 32],
        sender_username: &str,
    ) -> Result<Self, BridgeError> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(DEFAULT_CIPHERSUITE.signature_algorithm())
            .map_err(|e| BridgeError::Mls(format!("Failed to create signer: {e:?}")))?;

        let credential = CredentialWithKey {
            credential: BasicCredential::new(sender_commitment.to_vec()).into(),
            signature_key: signer.to_public_vec().into(),
        };

        let group_config = MlsGroupCreateConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

        let mls_service = MlsService::new_as_creator(
            room_id.to_string(),
            &provider,
            credential,
            &group_config,
            &signer,
        )
        .map_err(|e| BridgeError::Mls(format!("Failed to create MLS group: {e:?}")))?;

        Ok(Self {
            room_id: room_id.to_string(),
            sender_commitment,
            sender_username: sender_username.to_string(),
            provider,
            signer,
            mls_service,
            member_client: None,
        })
    }

    /// Initialize the Two-Tier SSS moderation client using the user's Nullifier Secret Key.
    pub fn set_member_client(&mut self, nsk: [u8; 32], k_strikes: u32) {
        self.member_client = Some(MemberClient::new(nsk, k_strikes));
    }

    /// Generate an MLS KeyPackage for a new member wanting to join this room.
    pub fn generate_joiner_key_package(
        member_commitment: [u8; 32],
    ) -> Result<(Vec<u8>, OpenMlsRustCrypto, SignatureKeyPair), BridgeError> {
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(DEFAULT_CIPHERSUITE.signature_algorithm())
            .map_err(|e| BridgeError::Mls(format!("Failed to create signer: {e:?}")))?;

        let credential = CredentialWithKey {
            credential: BasicCredential::new(member_commitment.to_vec()).into(),
            signature_key: signer.to_public_vec().into(),
        };

        let key_package_bytes = KeyPackage::builder()
            .build(DEFAULT_CIPHERSUITE, &provider, &signer, credential)
            .map_err(|e| BridgeError::KeyPackage(format!("Failed to build key package: {e:?}")))?
            .key_package()
            .tls_serialize_detached()
            .map_err(|e| {
                BridgeError::KeyPackage(format!("Failed to serialize key package: {e:?}"))
            })?;

        Ok((key_package_bytes, provider, signer))
    }

    /// Add a new member to the room using their KeyPackage, advancing the MLS epoch
    /// and returning the serialized Welcome message to send to the joiner.
    pub fn add_member(&mut self, key_package_bytes: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let _ = de_mls::mls_crypto::validate_key_package(&self.provider, key_package_bytes)
            .map_err(|e| {
                BridgeError::KeyPackage(format!("Key package validation failed: {e:?}"))
            })?;

        let artifacts = self
            .mls_service
            .create_commit_candidate(
                &self.provider,
                &self.signer,
                &[MlsCommitInput::Add(key_package_bytes.to_vec())],
            )
            .map_err(|e| BridgeError::Mls(format!("Failed to create commit candidate: {e:?}")))?;

        self.mls_service
            .merge_own_commit(&self.provider)
            .map_err(|e| BridgeError::Mls(format!("Failed to merge commit: {e:?}")))?;

        let welcome = artifacts.welcome.ok_or_else(|| {
            BridgeError::Mls("No welcome message generated for Add commit".to_string())
        })?;

        Ok(welcome)
    }

    /// Join an existing room using a received Welcome message.
    pub fn join_from_welcome(
        room_id: &str,
        sender_commitment: [u8; 32],
        sender_username: &str,
        provider: OpenMlsRustCrypto,
        signer: SignatureKeyPair,
        welcome_bytes: &[u8],
    ) -> Result<Self, BridgeError> {
        let mls_service = MlsService::new_from_welcome(&provider, welcome_bytes)
            .map_err(|e| BridgeError::Mls(format!("Failed to parse welcome: {e:?}")))?
            .ok_or_else(|| {
                BridgeError::Mls("Welcome message was not addressed to our key package".to_string())
            })?;

        Ok(Self {
            room_id: room_id.to_string(),
            sender_commitment,
            sender_username: sender_username.to_string(),
            provider,
            signer,
            mls_service,
            member_client: None,
        })
    }

    /// Prepare, protect with Two-Tier SSS, and encrypt an outgoing chat post using internal member_client.
    pub fn send_protected_message(
        &mut self,
        text: &str,
        post_salt: &[u8; 32],
        moderator_pubkeys: &[[u8; 32]],
        moderator_threshold: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        let member_client = self.member_client.as_mut().ok_or_else(|| {
            BridgeError::Moderation(
                "Member client not initialized. Call set_member_client first.".to_string(),
            )
        })?;

        Self::prepare_and_encrypt(
            &mut self.mls_service,
            &self.provider,
            &self.signer,
            &self.room_id,
            self.sender_commitment,
            &self.sender_username,
            text,
            member_client,
            post_salt,
            moderator_pubkeys,
            moderator_threshold,
        )
    }

    /// Prepare, protect with Two-Tier SSS, and encrypt an outgoing chat post using an explicit member_client reference.
    pub fn send_protected_message_with_client(
        &mut self,
        text: &str,
        member_client: &mut MemberClient,
        post_salt: &[u8; 32],
        moderator_pubkeys: &[[u8; 32]],
        moderator_threshold: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        Self::prepare_and_encrypt(
            &mut self.mls_service,
            &self.provider,
            &self.signer,
            &self.room_id,
            self.sender_commitment,
            &self.sender_username,
            text,
            member_client,
            post_salt,
            moderator_pubkeys,
            moderator_threshold,
        )
    }

    fn prepare_and_encrypt(
        mls_service: &mut MlsService,
        provider: &OpenMlsRustCrypto,
        signer: &SignatureKeyPair,
        room_id: &str,
        sender_commitment: [u8; 32],
        sender_username: &str,
        text: &str,
        member_client: &mut MemberClient,
        post_salt: &[u8; 32],
        moderator_pubkeys: &[[u8; 32]],
        moderator_threshold: u32,
    ) -> Result<Vec<u8>, BridgeError> {
        let text_bytes = text.as_bytes();

        // 1. Generate Two-Tier SSS shares and tracing tag via e_moderation_sdk
        let post_payload = member_client
            .prepare_post(
                text_bytes,
                post_salt,
                moderator_pubkeys,
                moderator_threshold,
            )
            .map_err(|e| BridgeError::Moderation(e.to_string()))?;

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 2. Package into ProtectedPayload
        let protected = ProtectedPayload {
            plaintext: text_bytes.to_vec(),
            tracing_tag: post_payload.tracing_tag,
            x_index: post_payload.x_index,
            encrypted_shares: post_payload.encrypted_shares,
            sender_commitment,
            sender_username: sender_username.to_string(),
            timestamp_ms,
        };

        // 3. Serialize to JSON bytes
        let payload_bytes = protected.to_bytes()?;

        // 4. Wrap in Protobuf AppMessage
        let app_msg = AppMessage {
            payload: Some(Payload::ConversationMessage(ConversationMessage {
                message: payload_bytes,
                conversation_id: room_id.to_string(),
            })),
        };

        // 5. Encrypt with OpenMLS ratchet tree
        let ciphertext = mls_service
            .build_message(provider, signer, &app_msg)
            .map_err(|e| BridgeError::Mls(format!("Failed to build MLS message: {e:?}")))?;

        Ok(ciphertext)
    }

    /// Decrypt an incoming MLS message, verify its structure, and extract the ProtectedPayload.
    /// Returns Ok(None) if the message was not an application message or belonged to a different epoch.
    pub fn receive_protected_message(
        &mut self,
        ciphertext: &[u8],
    ) -> Result<Option<ProtectedPayload>, BridgeError> {
        let decrypted = self
            .mls_service
            .decrypt_application_only(&self.provider, ciphertext)
            .map_err(|e| {
                BridgeError::Mls(format!("Failed to decrypt application message: {e:?}"))
            })?;

        let decrypted_msg = match decrypted {
            Some(msg) => msg,
            None => return Ok(None),
        };

        // Parse Protobuf AppMessage from decrypted payload bytes
        let app_msg = AppMessage::decode(&decrypted_msg.payload[..]).map_err(|e| {
            BridgeError::Deserialization(format!("Failed to decode AppMessage: {e:?}"))
        })?;

        let payload = match app_msg.payload {
            Some(Payload::ConversationMessage(conv_msg)) => conv_msg,
            _ => return Ok(None),
        };

        // Check Room ID
        if !payload.conversation_id.is_empty() && payload.conversation_id != self.room_id {
            return Err(BridgeError::RoomIdMismatch {
                expected: self.room_id.clone(),
                got: payload.conversation_id,
            });
        }

        // Deserialize ProtectedPayload
        let protected = ProtectedPayload::from_bytes(&payload.message)?;

        Ok(Some(protected))
    }

    /// Send a Direct Message (1-on-1 DM) encrypted via MLS ratchet tree.
    /// For DMs, Two-Tier SSS is bypassed (no moderator oversight in private 1-on-1 chat).
    pub fn send_dm_message(&mut self, text: &str) -> Result<Vec<u8>, BridgeError> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let protected = ProtectedPayload {
            plaintext: text.as_bytes().to_vec(),
            tracing_tag: [0u8; 32],
            x_index: 0,
            encrypted_shares: Vec::new(),
            sender_commitment: self.sender_commitment,
            sender_username: self.sender_username.clone(),
            timestamp_ms,
        };

        let payload_bytes = protected.to_bytes()?;

        let app_msg = AppMessage {
            payload: Some(Payload::ConversationMessage(ConversationMessage {
                message: payload_bytes,
                conversation_id: self.room_id.clone(),
            })),
        };

        let ciphertext = self
            .mls_service
            .build_message(&self.provider, &self.signer, &app_msg)
            .map_err(|e| BridgeError::Mls(format!("Failed to encrypt DM message: {e:?}")))?;

        Ok(ciphertext)
    }
}
