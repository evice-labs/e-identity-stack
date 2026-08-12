use e_identity_sdk::identity::{Blacklist, RegistrationClient};
use e_identity_sdk::moderation::anti_sybil::AntiSybilConfig;
use e_identity_sdk::moderation::release_share::ReleaseShareValidator;
use e_identity_sdk::moderation::strike::sign_strike;
use e_identity_sdk::room::management::RoomRegistry;
use e_identity_sdk::room::moderator_registry::ModeratorRegistry;
use e_identity_sdk::types::{ReleaseShareTx, StrikeCertificate};
use e_moderation_sdk::crypto::ecdh;

/// Helper: generate a valid secp256k1 scalar keypair
fn gen_keypair() -> ([u8; 32], [u8; 32]) {
    let sk = ecdh::generate_ephemeral_scalar();
    let pk = ecdh::derive_xonly_pubkey(&sk);
    (sk, pk)
}

/// Full lifecycle: register → create 3 rooms → join → strike → validate release_share → blacklist
#[test]
fn test_multi_room_strike_to_slash_lifecycle() {
    // Target user (will be slashed)
    let (target_sk, target_pk) = gen_keypair();
    let target = RegistrationClient::from_nsk(target_sk);
    let target_commitment = *target.commitment();

    // 3 moderators per room (2-of-3 threshold)
    let (mod1_sk, _mod1_pk) = gen_keypair();
    let (mod2_sk, _mod2_pk) = gen_keypair();
    let (mod3_sk, _mod3_pk) = gen_keypair();

    // Derive moderator x-only pubkeys for registration
    let mod1_pk_xonly = ecdh::derive_xonly_pubkey(&mod1_sk);
    let mod2_pk_xonly = ecdh::derive_xonly_pubkey(&mod2_sk);
    let mod3_pk_xonly = ecdh::derive_xonly_pubkey(&mod3_sk);
    let mod_pubkeys = vec![mod1_pk_xonly, mod2_pk_xonly, mod3_pk_xonly];

    // Admin for rooms
    let (admin_sk, _admin_pk) = gen_keypair();
    let admin = RegistrationClient::from_nsk(admin_sk);
    let admin_commitment = *admin.commitment();

    // Anti-Sybil config: require 3 distinct rooms, low maturity for testing
    let anti_sybil = AntiSybilConfig {
        k_rooms_min: 3,
        min_room_age_indexes: 10,
        min_room_members: 2,
        require_signed_join_consent: true,
    };

    // Create 3 rooms
    let mut room_registry = RoomRegistry::new();
    let mut moderator_registry = ModeratorRegistry::new();

    let room1 = room_registry
        .create_room(admin_commitment, 2, 3, mod_pubkeys.clone(), 0, 2)
        .expect("Create room 1");
    moderator_registry.register_from_config(&room1);

    let room2 = room_registry
        .create_room(admin_commitment, 2, 3, mod_pubkeys.clone(), 1, 2)
        .expect("Create room 2");
    moderator_registry.register_from_config(&room2);

    let room3 = room_registry
        .create_room(admin_commitment, 2, 3, mod_pubkeys.clone(), 2, 2)
        .expect("Create room 3");
    moderator_registry.register_from_config(&room3);

    // Target joins all 3 rooms with signed consent
    for room in [&room1, &room2, &room3] {
        let join_sig = RoomRegistry::sign_join_consent(
            &room.room_id,
            &target_commitment,
            &target_sk,
        )
        .expect("Sign join consent");

        room_registry
            .join_room(&room.room_id, target_commitment, &target_pk, join_sig, 50)
            .expect("Join room");
    }

    // Admin also joins all rooms (to meet min_room_members = 2)
    let admin_pk = ecdh::derive_xonly_pubkey(&admin_sk);
    for room in [&room1, &room2, &room3] {
        let join_sig = RoomRegistry::sign_join_consent(
            &room.room_id,
            &admin_commitment,
            &admin_sk,
        )
        .expect("Admin sign join consent");

        room_registry
            .join_room(&room.room_id, admin_commitment, &admin_pk, join_sig, 51)
            .expect("Admin join room");
    }

    // Verify member counts
    assert_eq!(room_registry.active_member_count(&room1.room_id), 2);
    assert_eq!(room_registry.active_member_count(&room2.room_id), 2);
    assert_eq!(room_registry.active_member_count(&room3.room_id), 2);

    // Moderators issue strikes in each room
    let evidence_hash = [0xABu8; 32];

    let mut certificates = Vec::new();
    for (strike_idx, room) in [&room1, &room2, &room3].iter().enumerate() {
        // 2-of-3 moderators sign each strike
        let sig1 = sign_strike(
            &room.room_id,
            &target_commitment,
            &evidence_hash,
            strike_idx as u64,
            &mod1_sk,
        )
        .expect("Mod1 signs strike");

        let sig2 = sign_strike(
            &room.room_id,
            &target_commitment,
            &evidence_hash,
            strike_idx as u64,
            &mod2_sk,
        )
        .expect("Mod2 signs strike");

        let cert = StrikeCertificate {
            room_id: room.room_id,
            target_commitment,
            moderator_signatures: vec![sig1, sig2],
            strike_index: strike_idx as u64,
            evidence_hash,
        };

        certificates.push(cert);
    }

    assert_eq!(certificates.len(), 3);

    // Validate ReleaseShare
    let release_tx = ReleaseShareTx {
        target_commitment,
        certificates,
    };

    let validator = ReleaseShareValidator::new(3, anti_sybil.clone());

    // current_index = 100 (rooms created at 0,1,2 so age >= 10 is satisfied)
    let result = validator.validate(
        &release_tx,
        &room_registry,
        &moderator_registry,
        100,
        &[], // no previously used strike indexes
    );
    assert!(result.is_ok(), "ReleaseShare validation should pass: {:?}", result);

    // Blacklist the commitment
    let mut blacklist = Blacklist::new();
    assert!(!blacklist.is_revoked(&target_commitment));
    blacklist.revoke(target_commitment).expect("Revoke succeeds");
    assert!(blacklist.is_revoked(&target_commitment));
}

/// Test that ReleaseShare fails with insufficient room diversity
#[test]
fn test_release_share_insufficient_room_diversity() {
    let (target_sk, target_pk) = gen_keypair();
    let target = RegistrationClient::from_nsk(target_sk);
    let target_commitment = *target.commitment();

    let (mod1_sk, _) = gen_keypair();
    let (mod2_sk, _) = gen_keypair();
    let mod1_pk = ecdh::derive_xonly_pubkey(&mod1_sk);
    let mod2_pk = ecdh::derive_xonly_pubkey(&mod2_sk);

    let (admin_sk, _) = gen_keypair();
    let admin = RegistrationClient::from_nsk(admin_sk);
    let admin_commitment = *admin.commitment();
    let admin_pk = ecdh::derive_xonly_pubkey(&admin_sk);

    let anti_sybil = AntiSybilConfig {
        k_rooms_min: 3,
        min_room_age_indexes: 5,
        min_room_members: 2,
        require_signed_join_consent: true,
    };

    // Only create 1 room — strikes from same room should fail diversity check
    let mut room_registry = RoomRegistry::new();
    let mut moderator_registry = ModeratorRegistry::new();

    let room = room_registry
        .create_room(admin_commitment, 2, 2, vec![mod1_pk, mod2_pk], 0, 2)
        .expect("Create room");
    moderator_registry.register_from_config(&room);

    // Both users join
    let join_sig = RoomRegistry::sign_join_consent(&room.room_id, &target_commitment, &target_sk).unwrap();
    room_registry.join_room(&room.room_id, target_commitment, &target_pk, join_sig, 10).unwrap();

    let admin_join = RoomRegistry::sign_join_consent(&room.room_id, &admin_commitment, &admin_sk).unwrap();
    room_registry.join_room(&room.room_id, admin_commitment, &admin_pk, admin_join, 10).unwrap();

    // 3 strikes from same room
    let evidence = [0xBBu8; 32];
    let mut certs = Vec::new();
    for i in 0..3 {
        let sig1 = sign_strike(&room.room_id, &target_commitment, &evidence, i, &mod1_sk).unwrap();
        let sig2 = sign_strike(&room.room_id, &target_commitment, &evidence, i, &mod2_sk).unwrap();
        certs.push(StrikeCertificate {
            room_id: room.room_id,
            target_commitment,
            moderator_signatures: vec![sig1, sig2],
            strike_index: i,
            evidence_hash: evidence,
        });
    }

    let tx = ReleaseShareTx {
        target_commitment,
        certificates: certs,
    };

    let validator = ReleaseShareValidator::new(3, anti_sybil);
    let result = validator.validate(&tx, &room_registry, &moderator_registry, 100, &[]);

    assert!(result.is_err(), "Should fail due to insufficient room diversity");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("room diversity"), "Error should mention room diversity: {err_msg}");
}

/// Test that immature rooms are rejected
#[test]
fn test_release_share_immature_room_rejected() {
    let (target_sk, target_pk) = gen_keypair();
    let target = RegistrationClient::from_nsk(target_sk);
    let target_commitment = *target.commitment();

    let (mod1_sk, _) = gen_keypair();
    let (mod2_sk, _) = gen_keypair();
    let mod1_pk = ecdh::derive_xonly_pubkey(&mod1_sk);
    let mod2_pk = ecdh::derive_xonly_pubkey(&mod2_sk);

    let (admin_sk, _) = gen_keypair();
    let admin = RegistrationClient::from_nsk(admin_sk);
    let admin_commitment = *admin.commitment();
    let admin_pk = ecdh::derive_xonly_pubkey(&admin_sk);

    let anti_sybil = AntiSybilConfig {
        k_rooms_min: 1,
        min_room_age_indexes: 100, // High maturity requirement
        min_room_members: 2,
        require_signed_join_consent: true,
    };

    let mut room_registry = RoomRegistry::new();
    let mut moderator_registry = ModeratorRegistry::new();

    // Room created at index 95
    let room = room_registry
        .create_room(admin_commitment, 2, 2, vec![mod1_pk, mod2_pk], 95, 2)
        .unwrap();
    moderator_registry.register_from_config(&room);

    let join_sig = RoomRegistry::sign_join_consent(&room.room_id, &target_commitment, &target_sk).unwrap();
    room_registry.join_room(&room.room_id, target_commitment, &target_pk, join_sig, 96).unwrap();

    let admin_join = RoomRegistry::sign_join_consent(&room.room_id, &admin_commitment, &admin_sk).unwrap();
    room_registry.join_room(&room.room_id, admin_commitment, &admin_pk, admin_join, 96).unwrap();

    let evidence = [0xCCu8; 32];
    let sig1 = sign_strike(&room.room_id, &target_commitment, &evidence, 0, &mod1_sk).unwrap();
    let sig2 = sign_strike(&room.room_id, &target_commitment, &evidence, 0, &mod2_sk).unwrap();

    let cert = StrikeCertificate {
        room_id: room.room_id,
        target_commitment,
        moderator_signatures: vec![sig1, sig2],
        strike_index: 0,
        evidence_hash: evidence,
    };

    let tx = ReleaseShareTx {
        target_commitment,
        certificates: vec![cert],
    };

    let validator = ReleaseShareValidator::new(1, anti_sybil);
    // current_index = 100, room created at 95 → age = 5, but min_room_age_indexes = 100
    let result = validator.validate(&tx, &room_registry, &moderator_registry, 100, &[]);

    assert!(result.is_err(), "Should fail due to immature room");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("mature"), "Error should mention maturity: {err_msg}");
}
