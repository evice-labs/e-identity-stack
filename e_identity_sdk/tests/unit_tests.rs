use e_identity_sdk::identity::{
    verify_username_change, Blacklist, 
    RegistrationClient, UsernameRegistry,
};
use e_moderation_sdk::crypto::{
    ecdh,
    signature::PrivateKey,
    sss::recover_secret,
};

#[test]
fn test_registration_client_lifecycle() {
    let client = RegistrationClient::new();
    assert_ne!(*client.commitment(), [0u8; 32]);

    // Generate node keypairs for 3 storage nodes
    let node_sks: Vec<[u8; 32]> = (0..3).map(|_| ecdh::generate_ephemeral_scalar()).collect();
    let node_pks: Vec<[u8; 32]> = node_sks.iter().map(ecdh::derive_xonly_pubkey).collect();

    // Prepare registration with K=2 SSS threshold out of 3 nodes
    let payload = client
        .prepare_registration("alice_basecamp", &node_pks, 2)
        .expect("Registration preparation should succeed");

    assert_eq!(payload.username, "alice_basecamp");
    assert_eq!(payload.commitment, *client.commitment());
    assert_eq!(payload.encrypted_shares.len(), 3);

    // Decrypt SSS shares on node side
    let mut decrypted_shares = Vec::new();
    for (i, enc_share) in payload.encrypted_shares.iter().enumerate() {
        let shared_secret =
            ecdh::compute_shared_secret(&node_sks[i], &enc_share.ephemeral_pk).unwrap();

        let mut share_buf = enc_share.ciphertext.clone();
        ecdh::xor_encrypt(&mut share_buf, &shared_secret, 0);
        decrypted_shares.push(share_buf);
    }

    // Recover NSK using 2 of 3 shares
    let recovered_nsk = recover_secret(&decrypted_shares[0..2], 2).expect("Reconstruction succeeds");
    assert_eq!(recovered_nsk, *client.nsk());
}

#[test]
fn test_username_change_verification() {
    // Generate identity from valid private key scalar
    let sk_bytes = ecdh::generate_ephemeral_scalar();
    let client = RegistrationClient::from_nsk(sk_bytes);

    let change_payload = client
        .prepare_username_change("alice_v2")
        .expect("Prepare username change succeeds");

    let priv_key = PrivateKey::try_new(sk_bytes).unwrap();
    let pubkey_bytes = *e_moderation_sdk::crypto::signature::PublicKey::new_from_private_key(&priv_key).value();

    // Verify valid signature
    let verify_res = verify_username_change(
        &change_payload.commitment,
        &change_payload.new_username,
        &change_payload.ownership_signature,
        &pubkey_bytes,
    );
    assert!(verify_res.is_ok());

    // Verify tampered signature fails
    let mut tampered_sig = change_payload.ownership_signature;
    tampered_sig[0] ^= 0xFF;
    let verify_failed = verify_username_change(
        &change_payload.commitment,
        &change_payload.new_username,
        &tampered_sig,
        &pubkey_bytes,
    );
    assert!(verify_failed.is_err());
}

#[test]
fn test_username_registry() {
    let mut registry = UsernameRegistry::new();

    let comm_1 = [1u8; 32];
    let comm_2 = [2u8; 32];

    assert!(registry.register(comm_1, "bob".to_string()).is_ok());
    assert!(registry.register(comm_2, "bob".to_string()).is_err()); // Duplicate username

    assert_eq!(registry.lookup_by_commitment(&comm_1), Some("bob"));
    assert_eq!(registry.lookup_by_username("bob"), Some(&comm_1));

    assert!(registry.update(&comm_1, "bob_new".to_string()).is_ok());
    assert_eq!(registry.lookup_by_commitment(&comm_1), Some("bob_new"));
}

#[test]
fn test_blacklist() {
    let mut blacklist = Blacklist::new();
    let comm = [9u8; 32];

    assert!(!blacklist.is_revoked(&comm));
    assert!(blacklist.revoke(comm).is_ok());
    assert!(blacklist.is_revoked(&comm));
    assert!(blacklist.revoke(comm).is_err()); // Double revoke
}
