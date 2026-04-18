/// NIP-44 v2 encryption: ChaCha20 stream cipher + HMAC-SHA256 authentication,
/// with keys derived via HKDF-SHA256 from a secp256k1 ECDH shared secret.
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20::{
    cipher::{KeyIvInit, StreamCipher},
    ChaCha20,
};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use secp256k1::{ecdh, PublicKey, SecretKey};
use sha2::Sha256;

use crate::model::Pubkey;

type HmacSha256 = Hmac<Sha256>;
type MessageKeys = ([u8; 32], [u8; CHACHA_NONCE_LEN], [u8; MAC_LEN]);

const VERSION: u8 = 0x02;
const NONCE_LEN: usize = 32;
const MAC_LEN: usize = 32;
const CHACHA_NONCE_LEN: usize = 12;
const MESSAGE_KEYS_LEN: usize = 32 + CHACHA_NONCE_LEN + MAC_LEN; // 76

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Nip44Error {
    #[error("NIP-44 private key is invalid")]
    InvalidPrivateKey,
    #[error("NIP-44 peer pubkey is invalid")]
    InvalidPeerPubkey,
    #[error("NIP-44 payload is invalid base64")]
    InvalidBase64,
    #[error("NIP-44 payload version is unsupported (expected 0x02)")]
    UnsupportedVersion,
    #[error("NIP-44 payload is too short")]
    PayloadTooShort,
    #[error("NIP-44 MAC verification failed")]
    MacVerificationFailed,
    #[error("NIP-44 plaintext is not valid UTF-8")]
    InvalidUtf8,
    #[error("NIP-44 plaintext length prefix is out of range")]
    InvalidLengthPrefix,
}

// ── Key derivation ───────────────────────────────────────────────────────────

/// Derive a 32-byte conversation key via HKDF-Extract(salt="nip44-v2", IKM=ecdh_shared_x).
fn conversation_key(secret_hex: &str, peer_pubkey: &Pubkey) -> Result<[u8; 32], Nip44Error> {
    let secret_bytes = hex::decode(secret_hex).map_err(|_| Nip44Error::InvalidPrivateKey)?;
    let secret_key =
        SecretKey::from_slice(&secret_bytes).map_err(|_| Nip44Error::InvalidPrivateKey)?;

    let peer_bytes =
        hex::decode(peer_pubkey.as_str()).map_err(|_| Nip44Error::InvalidPeerPubkey)?;
    if peer_bytes.len() != 32 {
        return Err(Nip44Error::InvalidPeerPubkey);
    }
    let mut compressed = [0u8; 33];
    compressed[0] = 0x02;
    compressed[1..].copy_from_slice(&peer_bytes);
    let peer_pub = PublicKey::from_slice(&compressed).map_err(|_| Nip44Error::InvalidPeerPubkey)?;

    let shared_point = ecdh::shared_secret_point(&peer_pub, &secret_key);
    let shared_x = &shared_point[..32];

    // HKDF-Extract: salt = b"nip44-v2", IKM = shared_x
    let (prk, _) = Hkdf::<Sha256>::extract(Some(b"nip44-v2"), shared_x);
    Ok(prk.into())
}

/// Derive per-message keys: HKDF-Expand(prk=conversation_key, info=nonce, L=76).
/// Returns (chacha_key[32], chacha_nonce[12], hmac_key[32]).
fn message_keys(conv_key: &[u8; 32], nonce: &[u8; NONCE_LEN]) -> Result<MessageKeys, Nip44Error> {
    let hk = Hkdf::<Sha256>::from_prk(conv_key).map_err(|_| Nip44Error::InvalidPrivateKey)?;
    let mut okm = [0u8; MESSAGE_KEYS_LEN];
    hk.expand(nonce, &mut okm)
        .map_err(|_| Nip44Error::InvalidPrivateKey)?;

    let mut chacha_key = [0u8; 32];
    let mut chacha_nonce = [0u8; CHACHA_NONCE_LEN];
    let mut hmac_key = [0u8; MAC_LEN];
    chacha_key.copy_from_slice(&okm[..32]);
    chacha_nonce.copy_from_slice(&okm[32..44]);
    hmac_key.copy_from_slice(&okm[44..76]);
    Ok((chacha_key, chacha_nonce, hmac_key))
}

// ── Padding ──────────────────────────────────────────────────────────────────

/// Calculate the padded length for the plaintext (excluding the 2-byte length prefix).
fn calc_padded_len(unpadded: usize) -> usize {
    if unpadded <= 32 {
        return 32;
    }
    // next_power = smallest power-of-2 >= unpadded
    let next_power = (unpadded - 1).next_power_of_two();
    let chunk = if next_power <= 256 {
        32
    } else {
        next_power / 8
    };
    chunk * unpadded.div_ceil(chunk)
}

/// Pads plaintext: [u16-BE length][plaintext][zero-padding to calc_padded_len].
fn pad(plaintext: &[u8]) -> Vec<u8> {
    let padded_len = calc_padded_len(plaintext.len());
    let mut buf = Vec::with_capacity(2 + padded_len);
    let len_u16 = plaintext.len() as u16;
    buf.extend_from_slice(&len_u16.to_be_bytes());
    buf.extend_from_slice(plaintext);
    buf.resize(2 + padded_len, 0);
    buf
}

/// Unpads and returns the original plaintext bytes.
fn unpad(padded: &[u8]) -> Result<&[u8], Nip44Error> {
    if padded.len() < 2 {
        return Err(Nip44Error::InvalidLengthPrefix);
    }
    let len = u16::from_be_bytes([padded[0], padded[1]]) as usize;
    if len == 0 || 2 + len > padded.len() {
        return Err(Nip44Error::InvalidLengthPrefix);
    }
    Ok(&padded[2..2 + len])
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Encrypt `plaintext` from `secret_hex` to `peer_pubkey` using NIP-44 v2.
/// `nonce` must be 32 random bytes; provide `[0u8; 32]` only in tests.
pub fn nip44_encrypt(
    secret_hex: &str,
    peer_pubkey: &Pubkey,
    plaintext: &str,
    nonce: [u8; NONCE_LEN],
) -> Result<String, Nip44Error> {
    let conv = conversation_key(secret_hex, peer_pubkey)?;
    let (chacha_key, chacha_nonce, hmac_key) = message_keys(&conv, &nonce)?;

    let mut buf = pad(plaintext.as_bytes());
    ChaCha20::new(chacha_key.as_slice().into(), chacha_nonce.as_slice().into())
        .apply_keystream(&mut buf);

    let ciphertext = buf;

    // HMAC-SHA256(key=hmac_key, message=nonce || ciphertext)
    let mut mac =
        HmacSha256::new_from_slice(&hmac_key).map_err(|_| Nip44Error::InvalidPrivateKey)?;
    mac.update(&nonce);
    mac.update(&ciphertext);
    let mac_bytes: [u8; 32] = mac.finalize().into_bytes().into();

    // Encode: 0x02 || nonce || ciphertext || mac
    let mut payload = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len() + MAC_LEN);
    payload.push(VERSION);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);
    payload.extend_from_slice(&mac_bytes);

    Ok(STANDARD.encode(&payload))
}

/// Decrypt a NIP-44 v2 payload produced by `nip44_encrypt`.
pub fn nip44_decrypt(
    secret_hex: &str,
    peer_pubkey: &Pubkey,
    payload: &str,
) -> Result<String, Nip44Error> {
    let raw = STANDARD
        .decode(payload)
        .map_err(|_| Nip44Error::InvalidBase64)?;

    // Minimum: 1 (version) + 32 (nonce) + 32 (min cipher block) + 32 (mac)
    if raw.len() < 1 + NONCE_LEN + 32 + MAC_LEN {
        return Err(Nip44Error::PayloadTooShort);
    }

    if raw[0] != VERSION {
        return Err(Nip44Error::UnsupportedVersion);
    }

    let nonce: [u8; NONCE_LEN] = raw[1..1 + NONCE_LEN].try_into().unwrap();
    let ciphertext = &raw[1 + NONCE_LEN..raw.len() - MAC_LEN];
    let expected_mac = &raw[raw.len() - MAC_LEN..];

    let conv = conversation_key(secret_hex, peer_pubkey)?;
    let (chacha_key, chacha_nonce, hmac_key) = message_keys(&conv, &nonce)?;

    // Verify MAC before decrypting (authenticate-then-decrypt)
    let mut mac =
        HmacSha256::new_from_slice(&hmac_key).map_err(|_| Nip44Error::InvalidPrivateKey)?;
    mac.update(&nonce);
    mac.update(ciphertext);
    mac.verify_slice(expected_mac)
        .map_err(|_| Nip44Error::MacVerificationFailed)?;

    let mut buf = ciphertext.to_vec();
    ChaCha20::new(chacha_key.as_slice().into(), chacha_nonce.as_slice().into())
        .apply_keystream(&mut buf);

    let plaintext_bytes = unpad(&buf)?;
    String::from_utf8(plaintext_bytes.to_vec()).map_err(|_| Nip44Error::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey_from_secret_hex;

    const ALICE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const BOB: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn nonce() -> [u8; NONCE_LEN] {
        [42u8; NONCE_LEN]
    }

    #[test]
    fn round_trips_short_plaintext() {
        let bob_pubkey = pubkey_from_secret_hex(BOB).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(ALICE).unwrap();

        let encrypted = nip44_encrypt(ALICE, &bob_pubkey, "hello", nonce()).unwrap();
        let decrypted = nip44_decrypt(BOB, &alice_pubkey, &encrypted).unwrap();

        assert_eq!(decrypted, "hello");
    }

    #[test]
    fn round_trips_long_plaintext() {
        let bob_pubkey = pubkey_from_secret_hex(BOB).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(ALICE).unwrap();
        let msg = "x".repeat(1000);

        let encrypted = nip44_encrypt(ALICE, &bob_pubkey, &msg, nonce()).unwrap();
        let decrypted = nip44_decrypt(BOB, &alice_pubkey, &encrypted).unwrap();

        assert_eq!(decrypted, msg);
    }

    #[test]
    fn symmetric_encryption() {
        // Alice encrypts to Bob, Bob can decrypt with Alice's pubkey
        let bob_pubkey = pubkey_from_secret_hex(BOB).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(ALICE).unwrap();

        let ct_ab = nip44_encrypt(ALICE, &bob_pubkey, "test", nonce()).unwrap();
        let ct_ba = nip44_encrypt(BOB, &alice_pubkey, "test", nonce()).unwrap();

        // Symmetric ECDH means same conversation key, same ciphertext given same nonce
        assert_eq!(ct_ab, ct_ba);
    }

    #[test]
    fn rejects_tampered_mac() {
        let bob_pubkey = pubkey_from_secret_hex(BOB).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(ALICE).unwrap();

        let encrypted = nip44_encrypt(ALICE, &bob_pubkey, "secret", nonce()).unwrap();
        let mut raw = STANDARD.decode(&encrypted).unwrap();
        // flip a byte in the MAC area
        let last = raw.len() - 1;
        raw[last] ^= 0xff;
        let tampered = STANDARD.encode(&raw);

        assert_eq!(
            nip44_decrypt(BOB, &alice_pubkey, &tampered).unwrap_err(),
            Nip44Error::MacVerificationFailed
        );
    }

    #[test]
    fn rejects_wrong_version() {
        let bob_pubkey = pubkey_from_secret_hex(BOB).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(ALICE).unwrap();

        let encrypted = nip44_encrypt(ALICE, &bob_pubkey, "secret", nonce()).unwrap();
        let mut raw = STANDARD.decode(&encrypted).unwrap();
        raw[0] = 0x01; // wrong version
        let bad = STANDARD.encode(&raw);

        assert_eq!(
            nip44_decrypt(BOB, &alice_pubkey, &bad).unwrap_err(),
            Nip44Error::UnsupportedVersion
        );
    }

    #[test]
    fn calc_padded_len_boundaries() {
        assert_eq!(calc_padded_len(1), 32);
        assert_eq!(calc_padded_len(32), 32);
        assert_eq!(calc_padded_len(33), 64);
        assert_eq!(calc_padded_len(64), 64);
        assert_eq!(calc_padded_len(65), 96);
    }
}
