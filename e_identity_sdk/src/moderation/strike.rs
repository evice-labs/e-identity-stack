use e_moderation_sdk::crypto::signature::{PublicKey, Signature};
use sha2::{Digest, Sha256};

use crate::moderation::anti_sybil::ModerationError;
use crate::room::moderator_registry::ModeratorRegistry;
use crate::types::{ModeratorSig, StrikeCertificate};

/// Validates a StrikeCertificate against the room's moderator registry.
///
/// Checks:
/// 1. The certificate has at least N_mod valid moderator signatures
/// 2. Each signing moderator is registered and active for the room
/// 3. Signatures are valid BIP-340 Schnorr over the strike message
/// 4. No duplicate moderator signatures
pub fn validate_strike_certificate(
    certificate: &StrikeCertificate,
    n_mod_threshold: u32,
    moderator_registry: &ModeratorRegistry,
) -> Result<(), ModerationError> {
    let mut valid_count = 0u32;
    let mut seen_pubkeys = std::collections::HashSet::new();

    for mod_sig in &certificate.moderator_signatures {
        if !seen_pubkeys.insert(mod_sig.pubkey) {
            continue;
        }
        if !moderator_registry.is_moderator(&certificate.room_id, &mod_sig.pubkey) {
            return Err(ModerationError::ModeratorNotRegistered);
        }

        // Verify BIP-340 Schnorr signature over strike message
        let message = build_strike_message(certificate);
        if verify_moderator_signature(&message, mod_sig)? {
            valid_count += 1;
        }
    }

    if valid_count < n_mod_threshold {
        return Err(ModerationError::InsufficientModeratorSigs {
            required: n_mod_threshold,
            provided: valid_count,
        });
    }

    Ok(())
}

/// Build the message that moderators sign for a strike.
///
/// Message = SHA256("EVICE/v1/Strike/" || room_id || target_commitment || evidence_hash || strike_index)
pub fn build_strike_message(certificate: &StrikeCertificate) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"EVICE/v1/Strike/");
    hasher.update(&certificate.room_id);
    hasher.update(&certificate.target_commitment);
    hasher.update(&certificate.evidence_hash);
    hasher.update(certificate.strike_index.to_le_bytes());
    hasher.finalize().into()
}

/// Create a moderator signature for a strike certificate.
///
/// Used by moderators to sign their approval of a strike.
pub fn sign_strike(
    room_id: &[u8; 32],
    target_commitment: &[u8; 32],
    evidence_hash: &[u8; 32],
    strike_index: u64,
    moderator_nsk: &[u8; 32],
) -> Result<ModeratorSig, ModerationError> {
    let cert_for_hash = StrikeCertificate {
        room_id: *room_id,
        target_commitment: *target_commitment,
        moderator_signatures: Vec::new(),
        strike_index,
        evidence_hash: *evidence_hash,
    };
    let message = build_strike_message(&cert_for_hash);

    let private_key = e_moderation_sdk::crypto::signature::PrivateKey::try_new(*moderator_nsk)
        .map_err(|_| ModerationError::InvalidModeratorSignature)?;
    let pubkey = e_moderation_sdk::crypto::signature::PublicKey::new_from_private_key(&private_key);
    let signature = Signature::new(&private_key, &message);

    Ok(ModeratorSig {
        pubkey: *pubkey.value(),
        signature: signature.value,
    })
}

/// Verify a single moderator's BIP-340 Schnorr signature.
fn verify_moderator_signature(
    message: &[u8; 32],
    mod_sig: &ModeratorSig,
) -> Result<bool, ModerationError> {
    let pubkey = PublicKey::try_new(mod_sig.pubkey)
        .map_err(|_| ModerationError::InvalidModeratorSignature)?;
    let sig = Signature {
        value: mod_sig.signature,
    };
    Ok(sig.is_valid_for(message, &pubkey))
}
