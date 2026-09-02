#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct FfiBlacklist;

struct FfiModeratorRegistry;

struct FfiRegistrationClient;

struct FfiReleaseShareValidator;

struct FfiRoomRegistry;

struct FfiUsernameRegistry;

extern "C" {

/// Free a JSON string returned by any `ffi_identity_*` function.
void ffi_identity_free_string(char *ptr);

/// Create a new RegistrationClient with a random NSK.
FfiRegistrationClient *ffi_registration_new();

/// Create a RegistrationClient from an existing 32-byte NSK.
FfiRegistrationClient *ffi_registration_from_nsk(const uint8_t *nsk_ptr);

/// Free a RegistrationClient handle.
void ffi_registration_free(FfiRegistrationClient *ptr);

/// Copy the 32-byte commitment into the provided output buffer.
void ffi_registration_commitment(const FfiRegistrationClient *handle, uint8_t *out_ptr);

/// Copy the 32-byte NSK into the provided output buffer.
/// WARNING: NSK is sensitive material — handle with care.
void ffi_registration_nsk(const FfiRegistrationClient *handle, uint8_t *out_ptr);

/// Prepare a registration payload. Returns JSON string.
///
/// `node_pubkeys_ptr`: flat array of `node_count * 32` bytes.
char *ffi_registration_prepare(const FfiRegistrationClient *handle,
                               const char *username,
                               const uint8_t *node_pubkeys_ptr,
                               uint32_t node_count,
                               uint32_t k_sss_threshold);

/// Prepare a username change payload. Returns JSON string.
char *ffi_registration_prepare_username_change(const FfiRegistrationClient *handle,
                                               const char *new_username);

FfiUsernameRegistry *ffi_username_registry_new();

void ffi_username_registry_free(FfiUsernameRegistry *ptr);

/// Register a username for a commitment. Returns JSON `{"ok":true}` or `{"error":"..."}`.
char *ffi_username_registry_register(FfiUsernameRegistry *handle,
                                     const uint8_t *commitment_ptr,
                                     const char *username);

/// Lookup username by commitment. Returns JSON `{"username":"..."}` or `{"username":null}`.
char *ffi_username_registry_lookup_by_commitment(const FfiUsernameRegistry *handle,
                                                 const uint8_t *commitment_ptr);

/// Lookup commitment by username. Returns hex commitment or null.
char *ffi_username_registry_lookup_by_username(const FfiUsernameRegistry *handle,
                                               const char *username);

FfiBlacklist *ffi_blacklist_new();

void ffi_blacklist_free(FfiBlacklist *ptr);

/// Returns 1 if commitment is revoked, 0 if not, -1 on null pointer.
int32_t ffi_blacklist_is_revoked(const FfiBlacklist *handle, const uint8_t *commitment_ptr);

/// Add a commitment to the blacklist. Returns JSON result.
char *ffi_blacklist_revoke(FfiBlacklist *handle, const uint8_t *commitment_ptr);

/// Returns the number of revoked commitments, or -1 on null.
int32_t ffi_blacklist_len(const FfiBlacklist *handle);

FfiRoomRegistry *ffi_room_registry_new();

void ffi_room_registry_free(FfiRoomRegistry *ptr);

/// Create a room. Returns JSON with the created `RoomConfig` (including derived `room_id`).
///
/// `mod_pubkeys_ptr`: flat array of `m_mod_total * 32` bytes.
char *ffi_room_registry_create_room(FfiRoomRegistry *handle,
                                    const uint8_t *admin_commitment_ptr,
                                    uint32_t n_mod_threshold,
                                    uint32_t m_mod_total,
                                    const uint8_t *mod_pubkeys_ptr,
                                    uint64_t creation_index,
                                    uint32_t min_members_for_maturity);

/// Join a room with signed consent. Returns JSON result.
char *ffi_room_registry_join_room(FfiRoomRegistry *handle,
                                  const uint8_t *room_id_ptr,
                                  const uint8_t *member_commitment_ptr,
                                  const uint8_t *member_pubkey_ptr,
                                  const uint8_t *join_signature_ptr,
                                  uint64_t join_index);

/// Sign a join consent message. Returns 64-byte signature as hex JSON.
char *ffi_room_sign_join_consent(const uint8_t *room_id_ptr,
                                 const uint8_t *member_commitment_ptr,
                                 const uint8_t *nsk_ptr);

/// Leave a room. Returns JSON result.
char *ffi_room_registry_leave_room(FfiRoomRegistry *handle,
                                   const uint8_t *room_id_ptr,
                                   const uint8_t *member_commitment_ptr);

/// Get active member count for a room.
int32_t ffi_room_registry_active_member_count(const FfiRoomRegistry *handle,
                                              const uint8_t *room_id_ptr);

/// Check if a commitment has active membership in a room. Returns 1/0/-1.
int32_t ffi_room_registry_has_active_membership(const FfiRoomRegistry *handle,
                                                const uint8_t *room_id_ptr,
                                                const uint8_t *member_commitment_ptr);

FfiModeratorRegistry *ffi_moderator_registry_new();

void ffi_moderator_registry_free(FfiModeratorRegistry *ptr);

/// Register moderators from a room config JSON string.
char *ffi_moderator_registry_register_from_config(FfiModeratorRegistry *handle,
                                                  const char *room_config_json);

/// Check if a pubkey is an active moderator for a room. Returns 1/0/-1.
int32_t ffi_moderator_registry_is_moderator(const FfiModeratorRegistry *handle,
                                            const uint8_t *room_id_ptr,
                                            const uint8_t *pubkey_ptr);

/// Get active moderator count for a room.
int32_t ffi_moderator_registry_active_count(const FfiModeratorRegistry *handle,
                                            const uint8_t *room_id_ptr);

/// Sign a strike as a moderator. Returns JSON `ModeratorSig`.
char *ffi_strike_sign(const uint8_t *room_id_ptr,
                      const uint8_t *target_commitment_ptr,
                      const uint8_t *evidence_hash_ptr,
                      uint64_t strike_index,
                      const uint8_t *moderator_nsk_ptr);

/// Validate a strike certificate against a moderator registry.
/// `certificate_json`: JSON-encoded `StrikeCertificate`.
char *ffi_strike_validate(const char *certificate_json,
                          uint32_t n_mod_threshold,
                          const FfiModeratorRegistry *moderator_registry_handle);

/// Create a ReleaseShareValidator with anti-Sybil config.
FfiReleaseShareValidator *ffi_release_share_validator_new(uint32_t k_strikes,
                                                          uint32_t k_rooms_min,
                                                          uint64_t min_room_age_indexes,
                                                          uint32_t min_room_members);

void ffi_release_share_validator_free(FfiReleaseShareValidator *ptr);

/// Validate a ReleaseShare transaction.
///
/// `release_share_json`: JSON-encoded `ReleaseShareTx`.
/// `used_strike_indexes_ptr`: flat array of `count * 32` bytes (previously used strike IDs).
char *ffi_release_share_validate(const FfiReleaseShareValidator *handle,
                                 const char *release_share_json,
                                 const FfiRoomRegistry *room_registry_handle,
                                 const FfiModeratorRegistry *moderator_registry_handle,
                                 uint64_t current_index,
                                 const uint8_t *used_strike_indexes_ptr,
                                 uint32_t used_strike_count);

}  // extern "C"
