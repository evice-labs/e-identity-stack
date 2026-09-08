# Basecamp Integration: Anonymous DM & Group Chat

Comprehensive architecture and integration specification for the **`anon_chat_core`** (Qt C++ Plugin) and **`anon_chat_ui`** (QML Interface) modules into the **Logos Basecamp** host application.

---

## 1. Component Architecture

Phase 3 integration adopts the official Logos Basecamp Plugin architectural pattern (*derived and specialized from `anonymous_forum_core` / `anonymous_forum_ui`*):

```
┌────────────────────────────────────────────────────────────────────────┐
│                          Logos Basecamp Host                           │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    anon_chat_ui (QML Frontend)                   │  │
│  │  - IdentityView.qml            - RoomList.qml                    │  │
│  │  - ChatRoomView.qml            - ModerationView.qml              │  │
│  │  - MessageComposer.qml         - StrikeModal.qml                 │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
│                                     │ Qt Meta-Object (Q_INVOKABLE)     │
│  ┌──────────────────────────────────▼───────────────────────────────┐  │
│  │                   anon_chat_core (Qt C++ Plugin)                 │  │
│  │  - AnonChatPlugin (QObject, PluginInterface)                     │  │
│  │  - AnonChatInterface (Pure virtual interface)                    │  │
│  │  - Local Caching & Asynchronous Worker Threads                   │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
└─────────────────────────────────────┼──────────────────────────────────┘
                                      │ C-ABI FFI (`extern "C"`)
┌─────────────────────────────────────▼──────────────────────────────────┐
│                           E-Identity Stack                             │
│  ┌────────────────────────────────────┐ ┌───────────────────────────┐  │
│  │           e_identity_sdk           │ │     e_moderation_sdk      │  │
│  │  - RegistrationClient (NSK/Commit) │ │  - MemberClient (SSS)     │  │
│  │  - UsernameRegistry (Schnorr)      │ │  - ModeratorClient (ECDH) │  │
│  │  - RoomRegistry (Threshold Setup)  │ │  - SlashAggregator (GF2⁸) │  │
│  │  - Blacklist & Revocation Check    │ │  - Lagrange Slashing      │  │
│  └────────────────────────────────────┘ └───────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Project Directory Structure

### A. `anon_chat_core`
**Path**: `/home/namauser/logos-ecosystem/project/anon_chat_core`

```
anon_chat_core/
├── CMakeLists.txt              # Build configuration based on LogosModule CMake helper
├── metadata.json               # Basecamp Core Plugin Manifest
├── flake.nix                   # Nix packaging with logos-module-builder
├── README.md                   # Core module technical documentation & API specification
├── lib/                        # Vendor FFI binaries and C headers
│   ├── e_identity_sdk.h        # C-ABI header for identity & room operations
│   ├── e_moderation_sdk.h      # C-ABI header for SSS, ECDH, & aggregator
│   ├── libe_identity_sdk.so    # Compiled release shared library
│   └── libe_moderation_sdk.so  # Compiled release shared library
├── cmake/                      # Custom CMake configuration helper directory
└── src/
    ├── anon_chat_interface.h   # Qt Plugin Interface definition (Q_INVOKABLE)
    ├── anon_chat_plugin.h      # Qt plugin implementation class header
    └── anon_chat_plugin.cpp    # Rust FFI <-> Qt/QML bridge implementation
```

### B. `anon_chat_ui`
**Path**: `/home/namauser/logos-ecosystem/project/anon_chat_ui`

```
anon_chat_ui/
├── metadata.json               # Basecamp QML UI Manifest (depends on anon_chat_core)
├── flake.nix                   # Nix packaging with mkLogosQmlModule
├── README.md                   # User interface guide & navigation docs
├── assets/                     # Visual assets and icons directory
└── qml/
    ├── Main.qml                # Primary navigation controller & status bar
    ├── components/
    │   ├── IdentityBanner.qml  # Identity & commitment status bar
    │   ├── RoomList.qml        # Room directory browser & room creation dialog
    │   ├── ChatView.qml        # Chat message feed & tracing tag viewer
    │   ├── MessageComposer.qml # Message input with automatic SSS encryption
    │   └── StrikeModal.qml     # Moderator strike issuance dialog
    └── views/
        ├── IdentityView.qml    # NSK, commitment, & username management
        ├── ChatRoomView.qml    # Active conversation room view
        └── ModerationView.qml  # Moderator dashboard & Lagrange slashing
```

---

## 3. C++ API Functional Specification (`AnonChatInterface`)

The C++ plugin exposes the following invocable methods directly to QML and the Basecamp runtime:

| Category | Method | Description |
| :--- | :--- | :--- |
| **Identity** | `createIdentity(nskHex)` | Generates or imports an NSK, returning commitment and NSK JSON |
| | `getCommitment()` | Retrieves the active 32-byte commitment as hex |
| | `prepareRegistration(...)` | Prepares the identity registration payload for on-chain submission |
| | `registerUsername(...)` | Registers an off-chain pseudonym (BIP-340 Schnorr signed) |
| | `lookupUsername(...)` | Queries registered pseudonym by commitment |
| **Room** | `createRoom(...)` | Creates a decentralized room with $N$-of-$M$ threshold |
| | `joinRoom(...)` | Joins a room by generating a signed consent signature |
| | `leaveRoom(...)` | Leaves an active room |
| | `getRoomMemberCount(...)`| Retrieves active member count for a room |
| **Messaging**| `preparePost(...)` | Splits NSK via 2-tier SSS and ECDH-encrypts shares to room moderators |
| **Moderation**| `createModerator(...)` | Initializes moderator private key |
| | `getModeratorPubkey()` | Retrieves moderator public key as hex |
| | `issueStrike(...)` | Issues a BIP-340 Schnorr-signed strike certificate |
| | `validateStrike(...)` | Validates strike certificate threshold & signatures |
| **Slashing** | `reconstructStrike(...)`| Reconstructs Tier-1 strike share from $N$ moderator shares over $\text{GF}(2^8)$ |
| | `reconstructNsk(...)` | Reconstructs Tier-2 full NSK from $K$ accumulated strikes |
| | `isRevoked(...)` | Checks identity revocation status against blacklist |

---

## 4. FFI Memory Safety & Memory Management Protocol

All data exchanged across the C-ABI between Rust and C++ adheres to strict zero-leakage conventions:

1. **String Allocation**: Any `char*` returned by `e_identity_sdk` or `e_moderation_sdk` is dynamically allocated in Rust via `CString::into_raw`.
2. **String Conversion & Deallocation**:
   ```cpp
   // Safe pattern implemented in anon_chat_plugin.cpp
   char* raw_json = identity_sdk_create_identity(nsk_ptr);
   if (!raw_json) return QString();
   QString result = QString::fromUtf8(raw_json);
   identity_sdk_free_string(raw_json); // Explicitly returns ownership to Rust deallocator
   return result;
   ```
3. **Struct Memory Safety**: Opaque handles (e.g., cryptographic contexts) are managed via designated constructor and destructor FFI functions. No raw pointers cross boundaries without explicit lifecycle management.

---

## 5. End-to-End Cryptographic Data Flow

### A. Message Preparation (Sender Flow)
1. Sender types message in `anon_chat_ui` (`MessageComposer.qml`).
2. `AnonChatPlugin::preparePost` is called with message body and room ID.
3. In Rust (`MemberClient`):
   - Generates a unique point $(x_{post}, y_{post})$ on the sender's Tier-2 NSK polynomial.
   - Splits $y_{post}$ into $M$ Tier-1 shares using an $N$-of-$M$ Shamir Secret Sharing polynomial over $\text{GF}(2^8)$.
   - Performs ECDH shared secret derivation using each moderator's public key and encrypts each share.
   - Computes deterministic message ciphertext and `tracing_tag`.
4. Returns serialized JSON payload to UI for broadcast over the transport layer (Waku).

### B. Accountability & Slashing (Moderator & Aggregator Flow)
1. Flagged message triggers moderator investigation in `ModerationView.qml`.
2. $N$ moderators decrypt their respective shares and issue BIP-340 Schnorr-signed strike certificates.
3. `AnonChatPlugin::reconstructStrike` combines $N$ shares using Lagrange interpolation over $\text{GF}(2^8)$ to recover the Tier-2 point $S_{post}$.
4. When $K$ strikes accumulate for the same commitment, `AnonChatPlugin::reconstructNsk` computes the full 32-byte NSK.
5. The reconstructed NSK is published to LEZ via the on-chain `slash-member` instruction, permanently revoking the identity commitment and confiscating collateral.

---

## 6. Standalone Build & Packaging Instructions

Both modules can be built using Nix with the official Logos Module Builder:

```bash
# 1. Build Core Module (C++ Qt Plugin)
cd /home/namauser/logos-ecosystem/project/anon_chat_core
nix build .#

# 2. Build UI Module (QML Module)
cd /home/namauser/logos-ecosystem/project/anon_chat_ui
nix build .#
```
