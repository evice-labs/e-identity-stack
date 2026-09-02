//! C-ABI FFI bindings for `e_identity_sdk`.
//!
//! All functions follow these conventions:
//! - Opaque handle structs wrap inner Rust types (created/freed via `_new`/`_free`)
//! - Complex data is exchanged as JSON strings (`*mut c_char`), freed via `ffi_identity_free_string`
//! - 32-byte keys/commitments are passed as raw `*const u8` pointers
//! - All functions perform null-pointer checks before dereferencing

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use crate::identity::blacklist::Blacklist;
use crate::identity::registration::RegistrationClient;
use crate::identity::username::UsernameRegistry;
use crate::moderation::anti_sybil::AntiSybilConfig;
use crate::moderation::release_share::ReleaseShareValidator;
use crate::moderation::strike;
use crate::room::management::RoomRegistry;
use crate::room::moderator_registry::ModeratorRegistry;
use crate::types::StrikeCertificate;

// 1. Helpers

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn ok_json(s: &str) -> *mut c_char {
    to_c_string(s)
}

fn error_json(msg: &str) -> *mut c_char {
    to_c_string(&format!("{{\"error\":\"{}\"}}", msg))
}

fn success_json() -> *mut c_char {
    to_c_string("{\"ok\":true}")
}

unsafe fn read_32(ptr: *const u8) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf.copy_from_slice(slice::from_raw_parts(ptr, 32));
    buf
}

unsafe fn read_64(ptr: *const u8) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf.copy_from_slice(slice::from_raw_parts(ptr, 64));
    buf
}

/// Free a JSON string returned by any `ffi_identity_*` function.
#[no_mangle]
pub extern "C" fn ffi_identity_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

// 2. RegistrationClient

pub struct FfiRegistrationClient {
    inner: RegistrationClient,
}

/// Create a new RegistrationClient with a random NSK.
#[no_mangle]
pub extern "C" fn ffi_registration_new() -> *mut FfiRegistrationClient {
    Box::into_raw(Box::new(FfiRegistrationClient {
        inner: RegistrationClient::new(),
    }))
}

/// Create a RegistrationClient from an existing 32-byte NSK.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_from_nsk(
    nsk_ptr: *const u8,
) -> *mut FfiRegistrationClient {
    if nsk_ptr.is_null() {
        return ptr::null_mut();
    }
    let nsk = read_32(nsk_ptr);
    Box::into_raw(Box::new(FfiRegistrationClient {
        inner: RegistrationClient::from_nsk(nsk),
    }))
}

/// Free a RegistrationClient handle.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_free(ptr: *mut FfiRegistrationClient) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Copy the 32-byte commitment into the provided output buffer.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_commitment(
    handle: *const FfiRegistrationClient,
    out_ptr: *mut u8,
) {
    if handle.is_null() || out_ptr.is_null() {
        return;
    }
    let commitment = (*handle).inner.commitment();
    ptr::copy_nonoverlapping(commitment.as_ptr(), out_ptr, 32);
}

/// Copy the 32-byte NSK into the provided output buffer.
/// WARNING: NSK is sensitive material — handle with care.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_nsk(
    handle: *const FfiRegistrationClient,
    out_ptr: *mut u8,
) {
    if handle.is_null() || out_ptr.is_null() {
        return;
    }
    let nsk = (*handle).inner.nsk();
    ptr::copy_nonoverlapping(nsk.as_ptr(), out_ptr, 32);
}

/// Prepare a registration payload. Returns JSON string.
///
/// `node_pubkeys_ptr`: flat array of `node_count * 32` bytes.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_prepare(
    handle: *const FfiRegistrationClient,
    username: *const c_char,
    node_pubkeys_ptr: *const u8,
    node_count: u32,
    k_sss_threshold: u32,
) -> *mut c_char {
    if handle.is_null() || username.is_null() || node_pubkeys_ptr.is_null() {
        return error_json("null pointer");
    }

    let username_str = match CStr::from_ptr(username).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8 in username"),
    };

    let all_keys = slice::from_raw_parts(node_pubkeys_ptr, (node_count * 32) as usize);
    let node_pubkeys: Vec<[u8; 32]> = all_keys
        .chunks_exact(32)
        .map(|chunk| {
            let mut key = [0u8; 32];
            key.copy_from_slice(chunk);
            key
        })
        .collect();

    match (*handle)
        .inner
        .prepare_registration(username_str, &node_pubkeys, k_sss_threshold)
    {
        Ok(payload) => match serde_json::to_string(&payload) {
            Ok(json) => ok_json(&json),
            Err(e) => error_json(&format!("serialize: {}", e)),
        },
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Prepare a username change payload. Returns JSON string.
#[no_mangle]
pub unsafe extern "C" fn ffi_registration_prepare_username_change(
    handle: *const FfiRegistrationClient,
    new_username: *const c_char,
) -> *mut c_char {
    if handle.is_null() || new_username.is_null() {
        return error_json("null pointer");
    }

    let name_str = match CStr::from_ptr(new_username).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };

    match (*handle).inner.prepare_username_change(name_str) {
        Ok(payload) => match serde_json::to_string(&payload) {
            Ok(json) => ok_json(&json),
            Err(e) => error_json(&format!("serialize: {}", e)),
        },
        Err(e) => error_json(&format!("{}", e)),
    }
}

// 3. UsernameRegistry

pub struct FfiUsernameRegistry {
    inner: UsernameRegistry,
}

#[no_mangle]
pub extern "C" fn ffi_username_registry_new() -> *mut FfiUsernameRegistry {
    Box::into_raw(Box::new(FfiUsernameRegistry {
        inner: UsernameRegistry::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_username_registry_free(ptr: *mut FfiUsernameRegistry) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Register a username for a commitment. Returns JSON `{"ok":true}` or `{"error":"..."}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_username_registry_register(
    handle: *mut FfiUsernameRegistry,
    commitment_ptr: *const u8,
    username: *const c_char,
) -> *mut c_char {
    if handle.is_null() || commitment_ptr.is_null() || username.is_null() {
        return error_json("null pointer");
    }
    let commitment = read_32(commitment_ptr);
    let name = match CStr::from_ptr(username).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };

    match (*handle).inner.register(commitment, name.to_string()) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Lookup username by commitment. Returns JSON `{"username":"..."}` or `{"username":null}`.
#[no_mangle]
pub unsafe extern "C" fn ffi_username_registry_lookup_by_commitment(
    handle: *const FfiUsernameRegistry,
    commitment_ptr: *const u8,
) -> *mut c_char {
    if handle.is_null() || commitment_ptr.is_null() {
        return error_json("null pointer");
    }
    let commitment = read_32(commitment_ptr);
    match (*handle).inner.lookup_by_commitment(&commitment) {
        Some(name) => ok_json(&format!("{{\"username\":\"{}\"}}", name)),
        None => ok_json("{\"username\":null}"),
    }
}

/// Lookup commitment by username. Returns hex commitment or null.
#[no_mangle]
pub unsafe extern "C" fn ffi_username_registry_lookup_by_username(
    handle: *const FfiUsernameRegistry,
    username: *const c_char,
) -> *mut c_char {
    if handle.is_null() || username.is_null() {
        return error_json("null pointer");
    }
    let name = match CStr::from_ptr(username).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };
    match (*handle).inner.lookup_by_username(name) {
        Some(commitment) => ok_json(&format!("{{\"commitment\":\"{}\"}}", hex::encode(commitment))),
        None => ok_json("{\"commitment\":null}"),
    }
}

// 4. Blacklist

pub struct FfiBlacklist {
    inner: Blacklist,
}

#[no_mangle]
pub extern "C" fn ffi_blacklist_new() -> *mut FfiBlacklist {
    Box::into_raw(Box::new(FfiBlacklist {
        inner: Blacklist::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_blacklist_free(ptr: *mut FfiBlacklist) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Returns 1 if commitment is revoked, 0 if not, -1 on null pointer.
#[no_mangle]
pub unsafe extern "C" fn ffi_blacklist_is_revoked(
    handle: *const FfiBlacklist,
    commitment_ptr: *const u8,
) -> i32 {
    if handle.is_null() || commitment_ptr.is_null() {
        return -1;
    }
    let commitment = read_32(commitment_ptr);
    if (*handle).inner.is_revoked(&commitment) {
        1
    } else {
        0
    }
}

/// Add a commitment to the blacklist. Returns JSON result.
#[no_mangle]
pub unsafe extern "C" fn ffi_blacklist_revoke(
    handle: *mut FfiBlacklist,
    commitment_ptr: *const u8,
) -> *mut c_char {
    if handle.is_null() || commitment_ptr.is_null() {
        return error_json("null pointer");
    }
    let commitment = read_32(commitment_ptr);
    match (*handle).inner.revoke(commitment) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Returns the number of revoked commitments, or -1 on null.
#[no_mangle]
pub unsafe extern "C" fn ffi_blacklist_len(handle: *const FfiBlacklist) -> i32 {
    if handle.is_null() {
        return -1;
    }
    (*handle).inner.len() as i32
}

// 5. RoomRegistry

pub struct FfiRoomRegistry {
    inner: RoomRegistry,
}

#[no_mangle]
pub extern "C" fn ffi_room_registry_new() -> *mut FfiRoomRegistry {
    Box::into_raw(Box::new(FfiRoomRegistry {
        inner: RoomRegistry::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_free(ptr: *mut FfiRoomRegistry) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Create a room. Returns JSON with the created `RoomConfig` (including derived `room_id`).
///
/// `mod_pubkeys_ptr`: flat array of `m_mod_total * 32` bytes.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_create_room(
    handle: *mut FfiRoomRegistry,
    admin_commitment_ptr: *const u8,
    n_mod_threshold: u32,
    m_mod_total: u32,
    mod_pubkeys_ptr: *const u8,
    creation_index: u64,
    min_members_for_maturity: u32,
) -> *mut c_char {
    if handle.is_null() || admin_commitment_ptr.is_null() || mod_pubkeys_ptr.is_null() {
        return error_json("null pointer");
    }

    let admin_commitment = read_32(admin_commitment_ptr);
    let all_keys = slice::from_raw_parts(mod_pubkeys_ptr, (m_mod_total * 32) as usize);
    let mod_pubkeys: Vec<[u8; 32]> = all_keys
        .chunks_exact(32)
        .map(|chunk| {
            let mut key = [0u8; 32];
            key.copy_from_slice(chunk);
            key
        })
        .collect();

    match (*handle).inner.create_room(
        admin_commitment,
        n_mod_threshold,
        m_mod_total,
        mod_pubkeys,
        creation_index,
        min_members_for_maturity,
    ) {
        Ok(config) => match serde_json::to_string(&config) {
            Ok(json) => ok_json(&json),
            Err(e) => error_json(&format!("serialize: {}", e)),
        },
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Join a room with signed consent. Returns JSON result.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_join_room(
    handle: *mut FfiRoomRegistry,
    room_id_ptr: *const u8,
    member_commitment_ptr: *const u8,
    member_pubkey_ptr: *const u8,
    join_signature_ptr: *const u8,
    join_index: u64,
) -> *mut c_char {
    if handle.is_null()
        || room_id_ptr.is_null()
        || member_commitment_ptr.is_null()
        || member_pubkey_ptr.is_null()
        || join_signature_ptr.is_null()
    {
        return error_json("null pointer");
    }

    let room_id = read_32(room_id_ptr);
    let member_commitment = read_32(member_commitment_ptr);
    let member_pubkey = read_32(member_pubkey_ptr);
    let join_signature = read_64(join_signature_ptr);

    match (*handle).inner.join_room(
        &room_id,
        member_commitment,
        &member_pubkey,
        join_signature,
        join_index,
    ) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Sign a join consent message. Returns 64-byte signature as hex JSON.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_sign_join_consent(
    room_id_ptr: *const u8,
    member_commitment_ptr: *const u8,
    nsk_ptr: *const u8,
) -> *mut c_char {
    if room_id_ptr.is_null() || member_commitment_ptr.is_null() || nsk_ptr.is_null() {
        return error_json("null pointer");
    }

    let room_id = read_32(room_id_ptr);
    let member_commitment = read_32(member_commitment_ptr);
    let nsk = read_32(nsk_ptr);

    match RoomRegistry::sign_join_consent(&room_id, &member_commitment, &nsk) {
        Ok(sig) => ok_json(&format!("{{\"signature\":\"{}\"}}", hex::encode(sig))),
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Leave a room. Returns JSON result.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_leave_room(
    handle: *mut FfiRoomRegistry,
    room_id_ptr: *const u8,
    member_commitment_ptr: *const u8,
) -> *mut c_char {
    if handle.is_null() || room_id_ptr.is_null() || member_commitment_ptr.is_null() {
        return error_json("null pointer");
    }

    let room_id = read_32(room_id_ptr);
    let member_commitment = read_32(member_commitment_ptr);

    match (*handle).inner.leave_room(&room_id, &member_commitment) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Get active member count for a room.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_active_member_count(
    handle: *const FfiRoomRegistry,
    room_id_ptr: *const u8,
) -> i32 {
    if handle.is_null() || room_id_ptr.is_null() {
        return -1;
    }
    let room_id = read_32(room_id_ptr);
    (*handle).inner.active_member_count(&room_id) as i32
}

/// Check if a commitment has active membership in a room. Returns 1/0/-1.
#[no_mangle]
pub unsafe extern "C" fn ffi_room_registry_has_active_membership(
    handle: *const FfiRoomRegistry,
    room_id_ptr: *const u8,
    member_commitment_ptr: *const u8,
) -> i32 {
    if handle.is_null() || room_id_ptr.is_null() || member_commitment_ptr.is_null() {
        return -1;
    }
    let room_id = read_32(room_id_ptr);
    let commitment = read_32(member_commitment_ptr);
    if (*handle).inner.has_active_membership(&room_id, &commitment) {
        1
    } else {
        0
    }
}

// 6. ModeratorRegistry

pub struct FfiModeratorRegistry {
    inner: ModeratorRegistry,
}

#[no_mangle]
pub extern "C" fn ffi_moderator_registry_new() -> *mut FfiModeratorRegistry {
    Box::into_raw(Box::new(FfiModeratorRegistry {
        inner: ModeratorRegistry::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_moderator_registry_free(ptr: *mut FfiModeratorRegistry) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Register moderators from a room config JSON string.
#[no_mangle]
pub unsafe extern "C" fn ffi_moderator_registry_register_from_config(
    handle: *mut FfiModeratorRegistry,
    room_config_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || room_config_json.is_null() {
        return error_json("null pointer");
    }
    let json_str = match CStr::from_ptr(room_config_json).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };
    let config: crate::types::RoomConfig = match serde_json::from_str(json_str) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("parse config: {}", e)),
    };
    (*handle).inner.register_from_config(&config);
    success_json()
}

/// Check if a pubkey is an active moderator for a room. Returns 1/0/-1.
#[no_mangle]
pub unsafe extern "C" fn ffi_moderator_registry_is_moderator(
    handle: *const FfiModeratorRegistry,
    room_id_ptr: *const u8,
    pubkey_ptr: *const u8,
) -> i32 {
    if handle.is_null() || room_id_ptr.is_null() || pubkey_ptr.is_null() {
        return -1;
    }
    let room_id = read_32(room_id_ptr);
    let pubkey = read_32(pubkey_ptr);
    if (*handle).inner.is_moderator(&room_id, &pubkey) {
        1
    } else {
        0
    }
}

/// Get active moderator count for a room.
#[no_mangle]
pub unsafe extern "C" fn ffi_moderator_registry_active_count(
    handle: *const FfiModeratorRegistry,
    room_id_ptr: *const u8,
) -> i32 {
    if handle.is_null() || room_id_ptr.is_null() {
        return -1;
    }
    let room_id = read_32(room_id_ptr);
    (*handle).inner.active_count(&room_id) as i32
}

// 7. Strike Operations

/// Sign a strike as a moderator. Returns JSON `ModeratorSig`.
#[no_mangle]
pub unsafe extern "C" fn ffi_strike_sign(
    room_id_ptr: *const u8,
    target_commitment_ptr: *const u8,
    evidence_hash_ptr: *const u8,
    strike_index: u64,
    moderator_nsk_ptr: *const u8,
) -> *mut c_char {
    if room_id_ptr.is_null()
        || target_commitment_ptr.is_null()
        || evidence_hash_ptr.is_null()
        || moderator_nsk_ptr.is_null()
    {
        return error_json("null pointer");
    }

    let room_id = read_32(room_id_ptr);
    let target_commitment = read_32(target_commitment_ptr);
    let evidence_hash = read_32(evidence_hash_ptr);
    let moderator_nsk = read_32(moderator_nsk_ptr);

    match strike::sign_strike(
        &room_id,
        &target_commitment,
        &evidence_hash,
        strike_index,
        &moderator_nsk,
    ) {
        Ok(mod_sig) => {
            let json = format!(
                "{{\"pubkey\":\"{}\",\"signature\":\"{}\"}}",
                hex::encode(mod_sig.pubkey),
                hex::encode(mod_sig.signature)
            );
            ok_json(&json)
        }
        Err(e) => error_json(&format!("{}", e)),
    }
}

/// Validate a strike certificate against a moderator registry.
/// `certificate_json`: JSON-encoded `StrikeCertificate`.
#[no_mangle]
pub unsafe extern "C" fn ffi_strike_validate(
    certificate_json: *const c_char,
    n_mod_threshold: u32,
    moderator_registry_handle: *const FfiModeratorRegistry,
) -> *mut c_char {
    if certificate_json.is_null() || moderator_registry_handle.is_null() {
        return error_json("null pointer");
    }

    let json_str = match CStr::from_ptr(certificate_json).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };

    let cert: StrikeCertificate = match serde_json::from_str(json_str) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("parse cert: {}", e)),
    };

    match strike::validate_strike_certificate(
        &cert,
        n_mod_threshold,
        &(*moderator_registry_handle).inner,
    ) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}

// 8. ReleaseShareValidator

pub struct FfiReleaseShareValidator {
    inner: ReleaseShareValidator,
}

/// Create a ReleaseShareValidator with anti-Sybil config.
#[no_mangle]
pub extern "C" fn ffi_release_share_validator_new(
    k_strikes: u32,
    k_rooms_min: u32,
    min_room_age_indexes: u64,
    min_room_members: u32,
) -> *mut FfiReleaseShareValidator {
    let config = AntiSybilConfig {
        k_rooms_min,
        min_room_age_indexes,
        min_room_members,
        require_signed_join_consent: true,
    };
    Box::into_raw(Box::new(FfiReleaseShareValidator {
        inner: ReleaseShareValidator::new(k_strikes, config),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn ffi_release_share_validator_free(ptr: *mut FfiReleaseShareValidator) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Validate a ReleaseShare transaction.
///
/// `release_share_json`: JSON-encoded `ReleaseShareTx`.
/// `used_strike_indexes_ptr`: flat array of `count * 32` bytes (previously used strike IDs).
#[no_mangle]
pub unsafe extern "C" fn ffi_release_share_validate(
    handle: *const FfiReleaseShareValidator,
    release_share_json: *const c_char,
    room_registry_handle: *const FfiRoomRegistry,
    moderator_registry_handle: *const FfiModeratorRegistry,
    current_index: u64,
    used_strike_indexes_ptr: *const u8,
    used_strike_count: u32,
) -> *mut c_char {
    if handle.is_null()
        || release_share_json.is_null()
        || room_registry_handle.is_null()
        || moderator_registry_handle.is_null()
    {
        return error_json("null pointer");
    }

    let json_str = match CStr::from_ptr(release_share_json).to_str() {
        Ok(s) => s,
        Err(_) => return error_json("invalid UTF-8"),
    };

    let tx: crate::types::ReleaseShareTx = match serde_json::from_str(json_str) {
        Ok(t) => t,
        Err(e) => return error_json(&format!("parse tx: {}", e)),
    };

    let used_indexes: Vec<[u8; 32]> = if used_strike_indexes_ptr.is_null() || used_strike_count == 0
    {
        Vec::new()
    } else {
        let raw = slice::from_raw_parts(used_strike_indexes_ptr, (used_strike_count * 32) as usize);
        raw.chunks_exact(32)
            .map(|chunk| {
                let mut id = [0u8; 32];
                id.copy_from_slice(chunk);
                id
            })
            .collect()
    };

    match (*handle).inner.validate(
        &tx,
        &(*room_registry_handle).inner,
        &(*moderator_registry_handle).inner,
        current_index,
        &used_indexes,
    ) {
        Ok(()) => success_json(),
        Err(e) => error_json(&format!("{}", e)),
    }
}
