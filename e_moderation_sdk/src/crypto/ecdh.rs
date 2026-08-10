use sha2::{Digest, Sha256};

/// Compute ECDH shared secret between a local secret key and a remote x-only public key.
/// Returns SHA256("EVICE/v1/ECDH/" || raw_shared_point).
pub fn compute_shared_secret(
    local_sk: &[u8; 32],
    remote_xonly_pk: &[u8; 32],
) -> Result<[u8; 32], &'static str> {
    let mut sec1_compressed = [0u8; 33];
    sec1_compressed[0] = 0x02;
    sec1_compressed[1..33].copy_from_slice(remote_xonly_pk);

    let remote_pubkey = k256::PublicKey::from_sec1_bytes(&sec1_compressed)
        .map_err(|_| "Invalid remote public key for ECDH")?;

    let local_secret = k256::SecretKey::from_bytes(&(*local_sk).into())
        .map_err(|_| "Invalid local secret key")?;

    let shared_point = k256::ecdh::diffie_hellman(
        local_secret.to_nonzero_scalar(),
        remote_pubkey.as_affine(),
    );

    let mut hasher = Sha256::new();
    hasher.update(b"EVICE/v1/ECDH/");
    hasher.update(shared_point.raw_secret_bytes());
    Ok(hasher.finalize().into())
}

/// XOR-based stream cipher using SHA256-derived keystream.
/// Used for encrypting SSS shares to moderator/node public keys.
pub fn xor_encrypt(buffer: &mut [u8], shared_secret: &[u8; 32], index: u32) {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(index.to_le_bytes());
    let keystream: [u8; 32] = hasher.finalize().into();

    for (i, byte) in buffer.iter_mut().enumerate() {
        *byte ^= keystream[i % 32];
    }
}

/// Generate a random ephemeral scalar valid for the secp256k1 curve.
pub fn generate_ephemeral_scalar() -> [u8; 32] {
    use rand::RngCore;
    loop {
        let mut sk = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut sk);
        if k256::SecretKey::from_bytes(&sk.into()).is_ok() {
            return sk;
        }
    }
}

/// Derive the x-only public key from a scalar.
pub fn derive_xonly_pubkey(scalar_bytes: &[u8; 32]) -> [u8; 32] {
    use k256::elliptic_curve::sec1::ToEncodedPoint as _;
    let sk = k256::SecretKey::from_bytes(&(*scalar_bytes).into())
        .expect("Scalar was already validated");
    let encoded = sk.public_key().to_encoded_point(false);
    let x_coord = encoded.x().expect("Valid EC point has x-coordinate");
    let mut pk = [0u8; 32];
    pk.copy_from_slice(x_coord);
    pk
}
