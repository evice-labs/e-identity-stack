use e_chat_bridge::ffi::*;
use e_chat_bridge::{EChatSession, ProtectedPayload};
use e_identity_sdk::identity::RegistrationClient;
use std::ffi::{CStr, CString};

#[test]
fn test_end_to_end_mls_with_two_tier_sss() {
    // 1. Setup Alice's ZK Identity via e_identity_sdk
    let alice_reg = RegistrationClient::new();
    let alice_comm = *alice_reg.commitment();
    let alice_nsk = *alice_reg.nsk();

    // 2. Setup Bob's ZK Identity
    let bob_reg = RegistrationClient::new();
    let bob_comm = *bob_reg.commitment();

    // 3. Setup Moderator keys for Two-Tier SSS
    let mod_sk = e_moderation_sdk::crypto::ecdh::generate_ephemeral_scalar();
    let mod_pk = e_moderation_sdk::crypto::ecdh::derive_xonly_pubkey(&mod_sk);
    let moderator_pubkeys = vec![mod_pk];
    let moderator_threshold = 1;

    let room_id = "test-e2ee-room-001";

    // 4. Alice creates the MLS room
    let mut alice_session =
        EChatSession::create_room(room_id, alice_comm, "alice_anon").expect("Alice creates room");

    // 5. Bob generates a KeyPackage to join the room
    let (bob_kp, bob_provider, bob_signer) =
        EChatSession::generate_joiner_key_package(bob_comm).expect("Bob generates KeyPackage");

    // 6. Alice adds Bob to the room, advancing the MLS epoch and generating a Welcome message
    let welcome_bytes = alice_session
        .add_member(&bob_kp)
        .expect("Alice adds Bob to room");

    // 7. Bob joins the room using the Welcome message
    let mut bob_session = EChatSession::join_from_welcome(
        room_id,
        bob_comm,
        "bob_anon",
        bob_provider,
        bob_signer,
        &welcome_bytes,
    )
    .expect("Bob joins from Welcome");

    // 8. Alice prepares an anonymous post protected by Two-Tier SSS + MLS E2EE
    alice_session.set_member_client(alice_nsk, 3);
    let post_salt = [42u8; 32];
    let chat_message =
        "Hello Bob! This message is end-to-end encrypted with MLS and protected by Two-Tier SSS.";

    let ciphertext = alice_session
        .send_protected_message(
            chat_message,
            &post_salt,
            &moderator_pubkeys,
            moderator_threshold,
        )
        .expect("Alice sends protected message");

    // Verify ciphertext is NOT plaintext (it is MLS encrypted)
    assert!(!ciphertext.is_empty());
    assert!(!ciphertext
        .windows(chat_message.len())
        .any(|w| w == chat_message.as_bytes()));

    // 9. Bob receives and decrypts the MLS message
    let decrypted_opt = bob_session
        .receive_protected_message(&ciphertext)
        .expect("Bob decrypts message");

    assert!(decrypted_opt.is_some(), "Expected decrypted message");
    let decrypted: ProtectedPayload = decrypted_opt.unwrap();

    // 10. Verify message content and cryptographic accountability fields
    assert_eq!(
        String::from_utf8(decrypted.plaintext).unwrap(),
        chat_message
    );
    assert_eq!(decrypted.sender_commitment, alice_comm);
    assert_eq!(decrypted.sender_username, "alice_anon");
    assert_eq!(decrypted.encrypted_shares.len(), 1);
    assert_ne!(decrypted.tracing_tag, [0u8; 32]);
}

#[test]
fn test_1_on_1_direct_message_mls() {
    let alice_comm = [1u8; 32];
    let bob_comm = [2u8; 32];
    let dm_room_id = "dm-alice-bob";

    // Alice creates DM session
    let mut alice_session =
        EChatSession::create_room(dm_room_id, alice_comm, "alice").expect("create DM room");

    // Bob creates joiner package
    let (bob_kp, bob_prov, bob_sign) =
        EChatSession::generate_joiner_key_package(bob_comm).expect("bob key package");

    let welcome = alice_session.add_member(&bob_kp).expect("add bob to DM");

    let mut bob_session =
        EChatSession::join_from_welcome(dm_room_id, bob_comm, "bob", bob_prov, bob_sign, &welcome)
            .expect("bob joins DM");

    // Alice sends DM
    let dm_text = "Hey Bob, private 1-on-1 direct message!";
    let ciphertext = alice_session
        .send_dm_message(dm_text)
        .expect("Alice sends DM");

    // Bob receives DM
    let decrypted = bob_session
        .receive_protected_message(&ciphertext)
        .expect("Bob decrypts DM")
        .expect("Message received");

    assert_eq!(String::from_utf8(decrypted.plaintext).unwrap(), dm_text);
    assert_eq!(decrypted.sender_commitment, alice_comm);
    assert_eq!(decrypted.encrypted_shares.len(), 0); // No SSS for DMs
}

#[test]
fn test_c_abi_ffi_full_lifecycle() {
    unsafe {
        let alice_reg = RegistrationClient::new();
        let alice_comm = *alice_reg.commitment();
        let alice_nsk = *alice_reg.nsk();

        let bob_reg = RegistrationClient::new();
        let bob_comm = *bob_reg.commitment();

        let room_id = CString::new("ffi-room-42").unwrap();
        let alice_name = CString::new("alice_ffi").unwrap();
        let bob_name = CString::new("bob_ffi").unwrap();

        // 1. Alice creates room via FFI
        let alice_session = ffi_chat_session_create_room(
            room_id.as_ptr(),
            alice_comm.as_ptr(),
            alice_name.as_ptr(),
        );
        assert!(!alice_session.is_null());

        // 2. Alice initializes Two-Tier SSS moderation
        let init_res = ffi_chat_session_init_moderation(alice_session, alice_nsk.as_ptr(), 3);
        let init_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(init_res).to_str().unwrap()).unwrap();
        assert_eq!(init_json["ok"], true);
        ffi_chat_free_string(init_res);

        // 3. Bob creates joiner via FFI
        let bob_joiner = ffi_chat_joiner_new(bob_comm.as_ptr(), bob_name.as_ptr());
        assert!(!bob_joiner.is_null());

        let kp_res = ffi_chat_joiner_get_key_package(bob_joiner);
        let kp_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(kp_res).to_str().unwrap()).unwrap();
        let kp_hex = kp_json["key_package_hex"].as_str().unwrap().to_string();
        ffi_chat_free_string(kp_res);

        // 4. Alice adds Bob via FFI
        let kp_c_hex = CString::new(kp_hex).unwrap();
        let add_res = ffi_chat_session_add_member(alice_session, kp_c_hex.as_ptr());
        let add_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(add_res).to_str().unwrap()).unwrap();
        assert_eq!(add_json["ok"], true);
        let welcome_hex = add_json["welcome_hex"].as_str().unwrap().to_string();
        ffi_chat_free_string(add_res);

        // 5. Bob completes join via FFI
        let welcome_c_hex = CString::new(welcome_hex).unwrap();
        let bob_session =
            ffi_chat_joiner_complete_join(bob_joiner, room_id.as_ptr(), welcome_c_hex.as_ptr());
        assert!(!bob_session.is_null());

        // 6. Alice sends protected room message via FFI
        let mod_sk = e_moderation_sdk::crypto::ecdh::generate_ephemeral_scalar();
        let mod_pk = e_moderation_sdk::crypto::ecdh::derive_xonly_pubkey(&mod_sk);
        let post_salt = [99u8; 32];
        let chat_text = CString::new("Hello from C-ABI FFI!").unwrap();

        let send_res = ffi_chat_session_send_protected_message(
            alice_session,
            chat_text.as_ptr(),
            post_salt.as_ptr(),
            mod_pk.as_ptr(),
            1,
            1,
        );
        let send_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(send_res).to_str().unwrap()).unwrap();
        assert_eq!(send_json["ok"], true);
        let cipher_hex = send_json["ciphertext_hex"].as_str().unwrap().to_string();
        ffi_chat_free_string(send_res);

        // 7. Bob receives and decrypts protected message via FFI
        let cipher_c_hex = CString::new(cipher_hex).unwrap();
        let recv_res =
            ffi_chat_session_receive_protected_message(bob_session, cipher_c_hex.as_ptr());
        let recv_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(recv_res).to_str().unwrap()).unwrap();
        assert_eq!(recv_json["ok"], true);
        assert_eq!(recv_json["has_message"], true);
        assert_eq!(recv_json["plaintext"], "Hello from C-ABI FFI!");
        assert_eq!(recv_json["sender_username"], "alice_ffi");
        ffi_chat_free_string(recv_res);

        // 8. 1-on-1 DM test via FFI
        let dm_text = CString::new("Private DM via FFI").unwrap();
        let dm_send_res = ffi_chat_session_send_dm(alice_session, dm_text.as_ptr());
        let dm_send_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(dm_send_res).to_str().unwrap()).unwrap();
        assert_eq!(dm_send_json["ok"], true);
        let dm_cipher_hex = dm_send_json["ciphertext_hex"].as_str().unwrap().to_string();
        ffi_chat_free_string(dm_send_res);

        let dm_cipher_c_hex = CString::new(dm_cipher_hex).unwrap();
        let dm_recv_res = ffi_chat_session_receive_dm(bob_session, dm_cipher_c_hex.as_ptr());
        let dm_recv_json: serde_json::Value =
            serde_json::from_str(CStr::from_ptr(dm_recv_res).to_str().unwrap()).unwrap();
        assert_eq!(dm_recv_json["ok"], true);
        assert_eq!(dm_recv_json["has_message"], true);
        assert_eq!(dm_recv_json["plaintext"], "Private DM via FFI");
        ffi_chat_free_string(dm_recv_res);

        // 9. Free sessions
        ffi_chat_session_free(alice_session);
        ffi_chat_session_free(bob_session);
    }
}
