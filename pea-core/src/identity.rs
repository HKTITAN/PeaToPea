//! Device identity and crypto: keypairs, device ID, session keys, wire encryption.

use chacha20poly1305::aead::{Aead, KeyInit};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

/// Device public key (32 bytes, X25519). Serializable for beacon and handshake.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "bytes_32")] [u8; 32]);

mod bytes_32 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error> {
        v.as_slice().serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let buf: Vec<u8> = Deserialize::deserialize(d)?;
        buf.try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create a `PublicKey` from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
}

/// Device ID: deterministic hash of public key. Used in discovery and peer list.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DeviceId(#[serde(with = "bytes_16")] [u8; 16]);

mod bytes_16 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error> {
        v.as_slice().serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 16], D::Error> {
        let buf: Vec<u8> = Deserialize::deserialize(d)?;
        buf.try_into()
            .map_err(|_| serde::de::Error::custom("expected 16 bytes"))
    }
}

impl DeviceId {
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// X25519 keypair. Keep secret key private; expose only public key and device ID.
pub struct Keypair {
    secret: StaticSecret,
    public: PublicKey,
    device_id: DeviceId,
}

impl DeviceId {
    /// Derive device ID from a public key (same as Keypair does).
    pub fn from_public_key(public: &[u8; 32]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(public);
        let digest = hasher.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest[..16]);
        DeviceId(id)
    }
}

impl Keypair {
    /// Generate a new random keypair and derive device ID from public key.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public_x = X25519PublicKey::from(&secret);
        let public = PublicKey(public_x.to_bytes());
        let device_id = DeviceId::from_public_key(public.as_bytes());
        Self {
            secret,
            public,
            device_id,
        }
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Shared secret with another device's public key. Used to derive session key.
    pub fn shared_secret(&self, other_public: &PublicKey) -> [u8; 32] {
        let other = X25519PublicKey::from(other_public.0);
        self.secret.diffie_hellman(&other).to_bytes()
    }
}

/// Derive a 32-byte session key from shared secret (e.g. for ChaCha20-Poly1305).
/// Pairwise: each pair of devices has its own session key.
pub fn derive_session_key(shared_secret: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"peapod-session-v1");
    hasher.update(shared_secret);
    hasher.finalize().into()
}

/// Wire encryption: ChaCha20-Poly1305. Nonce: 96-bit counter per direction; never reuse.
pub fn encrypt_wire(
    key: &[u8; 32],
    nonce: u64,
    plaintext: &[u8],
) -> Result<Vec<u8>, WireCryptoError> {
    let cipher = chacha20poly1305::ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| WireCryptoError::Key)?;
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[4..12].copy_from_slice(&nonce.to_le_bytes());
    let nonce_arr = chacha20poly1305::aead::Nonce::<chacha20poly1305::ChaCha20Poly1305>::from_slice(
        &nonce_bytes,
    );
    cipher
        .encrypt(nonce_arr, plaintext)
        .map_err(|_| WireCryptoError::Encrypt)
}

/// Wire decryption.
pub fn decrypt_wire(
    key: &[u8; 32],
    nonce: u64,
    ciphertext: &[u8],
) -> Result<Vec<u8>, WireCryptoError> {
    let cipher = chacha20poly1305::ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| WireCryptoError::Key)?;
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[4..12].copy_from_slice(&nonce.to_le_bytes());
    let nonce_arr = chacha20poly1305::aead::Nonce::<chacha20poly1305::ChaCha20Poly1305>::from_slice(
        &nonce_bytes,
    );
    cipher
        .decrypt(nonce_arr, ciphertext)
        .map_err(|_| WireCryptoError::Decrypt)
}

#[derive(Debug, thiserror::Error)]
pub enum WireCryptoError {
    #[error("invalid key")]
    Key,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_device_id_derivation() {
        let kp = Keypair::generate();
        let id = DeviceId::from_public_key(kp.public_key().as_bytes());
        assert_eq!(id, kp.device_id());
    }

    #[test]
    fn key_exchange_symmetric() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let secret_a = a.shared_secret(b.public_key());
        let secret_b = b.shared_secret(a.public_key());
        assert_eq!(secret_a, secret_b);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let plain = b"hello peapod";
        let cipher = encrypt_wire(&key, 0, plain).unwrap();
        let dec = decrypt_wire(&key, 0, &cipher).unwrap();
        assert_eq!(dec.as_slice(), plain);
    }
}
