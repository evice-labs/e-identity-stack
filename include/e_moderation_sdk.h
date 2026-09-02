#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct FfiMemberClient;

struct FfiModeratorClient;

struct FfiSlashAggregator;

extern "C" {

void ffi_free_string(char *ptr);

FfiMemberClient *ffi_member_new(const uint8_t *nsk_ptr, uint32_t k_strikes);

void ffi_member_free(FfiMemberClient *ptr);

/// Prepare a post. Returns JSON string with PostPayload.
char *ffi_member_prepare_post(FfiMemberClient *handle,
                              const uint8_t *message_ptr,
                              uint32_t message_len,
                              const uint8_t *post_salt_ptr,
                              const uint8_t *mod_pubkeys_ptr,
                              uint32_t mod_count,
                              uint32_t n_threshold);

FfiModeratorClient *ffi_moderator_new(const uint8_t *privkey_ptr);

void ffi_moderator_free(FfiModeratorClient *ptr);

void ffi_moderator_public_key(const FfiModeratorClient *handle, uint8_t *out_ptr);

char *ffi_moderator_issue_strike(const FfiModeratorClient *handle,
                                 const uint8_t *tracing_tag_ptr,
                                 const char *encrypted_share_json,
                                 uint32_t moderator_index);

FfiSlashAggregator *ffi_aggregator_new(uint32_t n_threshold,
                                       uint32_t k_strikes,
                                       const uint8_t *mod_pubkeys_ptr,
                                       uint32_t mod_count);

void ffi_aggregator_free(FfiSlashAggregator *ptr);

/// Reconstruct a per-post strike from N certificates.
char *ffi_aggregator_reconstruct_strike(const FfiSlashAggregator *handle,
                                        const uint8_t *tracing_tag_ptr,
                                        const char *certificates_json);

/// Reconstruct the NSK from K accumulated strikes.
char *ffi_aggregator_reconstruct_nsk(const FfiSlashAggregator *handle, const char *strikes_json);

}  // extern "C"
