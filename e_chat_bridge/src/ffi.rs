//! C-ABI FFI bindings for `e_chat_bridge`.
//!
//! Provides a complete C interface for integrating MLS End-to-End Encryption
//! with e-identity-stack Zero-Knowledge identity and Two-Tier SSS accountability
//! into C++ modules (such as `el-anon-chat-core`).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use serde_json::json;

use crate::session::EChatSession;

pub struct FfiChatSession {
    inner: EChatSession,
}

pub struct FfiChatJoiner {
    commitment: [u8; 32],
    username: String,
    provider: OpenMlsRustCrypto,
    signer: SignatureKeyPair,
    key_package_bytes: Vec<u8>,
}

// HELPERS
fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn error_json(msg: &str) -> *mut c_char {
    to_c_string(&json!({ "error": msg }).to_string())
}

unsafe fn read_32(ptr: *const u8) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf.copy_from_slice(slice::from_raw_parts(ptr, 32));
    buf
}

unsafe fn c_to_string(ptr: *const c_char) -> Result<String, &'static str> {
    if ptr.is_null() {
        return Err("null pointer string");
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| "invalid UTF-8 string")
}

/// Free a C-string allocated by this library.
#[no_mangle]
pub extern "C" fn ffi_chat_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

// CHAT SESSION LIFECYCLE

/// Create a new chat session as the room creator/admin.
/// Returns a pointer to FfiChatSession, or null on failure.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_create_room(
    room_id_ptr: *const c_char,
    commitment_ptr: *const u8,
    username_ptr: *const c_char,
) -> *mut FfiChatSession {
    if room_id_ptr.is_null() || commitment_ptr.is_null() || username_ptr.is_null() {
        return ptr::null_mut();
    }

    let room_id = match c_to_string(room_id_ptr) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let username = match c_to_string(username_ptr) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let commitment = read_32(commitment_ptr);

    match EChatSession::create_room(&room_id, commitment, &username) {
        Ok(session) => Box::into_raw(Box::new(FfiChatSession { inner: session })),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a chat session.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_free(handle: *mut FfiChatSession) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Initialize Two-Tier SSS moderation with user's Nullifier Secret Key.
/// Returns JSON `{"ok":true}` or `{"error":"..."}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_init_moderation(
    handle: *mut FfiChatSession,
    nsk_ptr: *const u8,
    k_strikes: u32,
) -> *mut c_char {
    if handle.is_null() || nsk_ptr.is_null() {
        return error_json("null pointer");
    }

    let nsk = read_32(nsk_ptr);
    (*handle).inner.set_member_client(nsk, k_strikes);
    to_c_string(&json!({ "ok": true }).to_string())
}

// JOINER WORKFLOW

/// Create a new joiner client to generate a KeyPackage and later process a Welcome message.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_joiner_new(
    commitment_ptr: *const u8,
    username_ptr: *const c_char,
) -> *mut FfiChatJoiner {
    if commitment_ptr.is_null() || username_ptr.is_null() {
        return ptr::null_mut();
    }

    let commitment = read_32(commitment_ptr);
    let username = match c_to_string(username_ptr) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match EChatSession::generate_joiner_key_package(commitment) {
        Ok((kp_bytes, provider, signer)) => Box::into_raw(Box::new(FfiChatJoiner {
            commitment,
            username,
            provider,
            signer,
            key_package_bytes: kp_bytes,
        })),
        Err(_) => ptr::null_mut(),
    }
}

/// Get the hex-encoded KeyPackage for this joiner to share with the room admin.
/// Caller must free with `ffi_chat_free_string`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_joiner_get_key_package(
    handle: *mut FfiChatJoiner,
) -> *mut c_char {
    if handle.is_null() {
        return error_json("null pointer");
    }

    let hex_kp = hex::encode(&(*handle).key_package_bytes);
    to_c_string(&json!({ "ok": true, "key_package_hex": hex_kp }).to_string())
}

/// Complete the join process using the received Welcome message hex.
/// Consumes the joiner and returns a ready `*mut FfiChatSession`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_joiner_complete_join(
    handle: *mut FfiChatJoiner,
    room_id_ptr: *const c_char,
    welcome_hex_ptr: *const c_char,
) -> *mut FfiChatSession {
    if handle.is_null() || room_id_ptr.is_null() || welcome_hex_ptr.is_null() {
        return ptr::null_mut();
    }

    let joiner = Box::from_raw(handle);
    let room_id = match c_to_string(room_id_ptr) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let welcome_hex = match c_to_string(welcome_hex_ptr) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let welcome_bytes = match hex::decode(&welcome_hex) {
        Ok(b) => b,
        Err(_) => return ptr::null_mut(),
    };

    match EChatSession::join_from_welcome(
        &room_id,
        joiner.commitment,
        &joiner.username,
        joiner.provider,
        joiner.signer,
        &welcome_bytes,
    ) {
        Ok(session) => Box::into_raw(Box::new(FfiChatSession { inner: session })),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a joiner client if join was aborted.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_joiner_free(handle: *mut FfiChatJoiner) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

// ROOM MEMBER ADMINISTRATION

/// Add a new member to the room by their hex-encoded KeyPackage.
/// Advances the MLS epoch and returns JSON `{"ok":true, "welcome_hex":"..."}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_add_member(
    handle: *mut FfiChatSession,
    key_package_hex_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() || key_package_hex_ptr.is_null() {
        return error_json("null pointer");
    }

    let kp_hex = match c_to_string(key_package_hex_ptr) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };
    let kp_bytes = match hex::decode(&kp_hex) {
        Ok(b) => b,
        Err(e) => return error_json(&format!("hex decode error: {e}")),
    };

    match (*handle).inner.add_member(&kp_bytes) {
        Ok(welcome) => {
            to_c_string(&json!({ "ok": true, "welcome_hex": hex::encode(welcome) }).to_string())
        }
        Err(e) => error_json(&e.to_string()),
    }
}

// ROOM MESSAGES (MLS E2EE + Two-Tier SSS)

/// Send a protected room message.
/// Returns JSON `{"ok":true, "ciphertext_hex":"...", "tracing_tag":"...", "x_index":...}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_send_protected_message(
    handle: *mut FfiChatSession,
    text_ptr: *const c_char,
    post_salt_ptr: *const u8,
    mod_pubkeys_ptr: *const u8,
    mod_count: u32,
    threshold: u32,
) -> *mut c_char {
    if handle.is_null()
        || text_ptr.is_null()
        || post_salt_ptr.is_null()
        || mod_pubkeys_ptr.is_null()
    {
        return error_json("null pointer");
    }

    let text = match c_to_string(text_ptr) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    let post_salt = read_32(post_salt_ptr);

    let all_keys = slice::from_raw_parts(mod_pubkeys_ptr, (mod_count * 32) as usize);
    let mod_pubkeys: Vec<[u8; 32]> = all_keys
        .chunks_exact(32)
        .map(|chunk| {
            let mut key = [0u8; 32];
            key.copy_from_slice(chunk);
            key
        })
        .collect();

    match (*handle)
        .inner
        .send_protected_message(&text, &post_salt, &mod_pubkeys, threshold)
    {
        Ok(ciphertext) => to_c_string(
            &json!({
                "ok": true,
                "ciphertext_hex": hex::encode(ciphertext),
            })
            .to_string(),
        ),
        Err(e) => error_json(&e.to_string()),
    }
}

/// Receive and decrypt a protected room message.
/// Returns JSON with plaintext, sender_commitment, tracing_tag, etc.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_receive_protected_message(
    handle: *mut FfiChatSession,
    ciphertext_hex_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() || ciphertext_hex_ptr.is_null() {
        return error_json("null pointer");
    }

    let cipher_hex = match c_to_string(ciphertext_hex_ptr) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };
    let ciphertext = match hex::decode(&cipher_hex) {
        Ok(b) => b,
        Err(e) => return error_json(&format!("hex decode error: {e}")),
    };

    match (*handle).inner.receive_protected_message(&ciphertext) {
        Ok(Some(msg)) => {
            let plaintext = String::from_utf8_lossy(&msg.plaintext).to_string();
            let comm_hex = hex::encode(msg.sender_commitment);
            let tag_hex = hex::encode(msg.tracing_tag);

            to_c_string(
                &json!({
                    "ok": true,
                    "has_message": true,
                    "plaintext": plaintext,
                    "sender_commitment": comm_hex,
                    "sender_username": msg.sender_username,
                    "tracing_tag": tag_hex,
                    "x_index": msg.x_index,
                    "timestamp_ms": msg.timestamp_ms,
                    "encrypted_shares": msg.encrypted_shares,
                })
                .to_string(),
            )
        }
        Ok(None) => to_c_string(&json!({ "ok": true, "has_message": false }).to_string()),
        Err(e) => error_json(&e.to_string()),
    }
}

// DIRECT MESSAGES 1-ON-1 (MLS E2EE)

/// Send a 1-on-1 Direct Message encrypted with MLS ratchet tree.
/// Returns JSON `{"ok":true, "ciphertext_hex":"..."}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_send_dm(
    handle: *mut FfiChatSession,
    text_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() || text_ptr.is_null() {
        return error_json("null pointer");
    }

    let text = match c_to_string(text_ptr) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };

    match (*handle).inner.send_dm_message(&text) {
        Ok(ciphertext) => to_c_string(
            &json!({
                "ok": true,
                "ciphertext_hex": hex::encode(ciphertext),
            })
            .to_string(),
        ),
        Err(e) => error_json(&e.to_string()),
    }
}

/// Receive and decrypt a 1-on-1 Direct Message.
/// Returns JSON with plaintext, sender_commitment, and timestamp.
#[no_mangle]
pub unsafe extern "C" fn ffi_chat_session_receive_dm(
    handle: *mut FfiChatSession,
    ciphertext_hex_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() || ciphertext_hex_ptr.is_null() {
        return error_json("null pointer");
    }

    let cipher_hex = match c_to_string(ciphertext_hex_ptr) {
        Ok(s) => s,
        Err(e) => return error_json(e),
    };
    let ciphertext = match hex::decode(&cipher_hex) {
        Ok(b) => b,
        Err(e) => return error_json(&format!("hex decode error: {e}")),
    };

    match (*handle).inner.receive_protected_message(&ciphertext) {
        Ok(Some(msg)) => {
            let plaintext = String::from_utf8_lossy(&msg.plaintext).to_string();
            let comm_hex = hex::encode(msg.sender_commitment);

            to_c_string(
                &json!({
                    "ok": true,
                    "has_message": true,
                    "plaintext": plaintext,
                    "sender_commitment": comm_hex,
                    "sender_username": msg.sender_username,
                    "timestamp_ms": msg.timestamp_ms,
                })
                .to_string(),
            )
        }
        Ok(None) => to_c_string(&json!({ "ok": true, "has_message": false }).to_string()),
        Err(e) => error_json(&e.to_string()),
    }
}
