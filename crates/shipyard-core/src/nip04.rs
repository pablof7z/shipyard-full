use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::Aes256;
use base64::{engine::general_purpose::STANDARD, Engine};
use cbc::{Decryptor, Encryptor};
use secp256k1::{ecdh, PublicKey, SecretKey};

use crate::model::Pubkey;

type Aes256CbcDec = Decryptor<Aes256>;
type Aes256CbcEnc = Encryptor<Aes256>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Nip04Error {
    #[error("NIP-04 ciphertext must include ?iv=")]
    MissingIv,
    #[error("NIP-04 private key is invalid")]
    InvalidPrivateKey,
    #[error("NIP-04 peer pubkey is invalid")]
    InvalidPeerPubkey,
    #[error("NIP-04 payload is invalid base64")]
    InvalidBase64,
    #[error("NIP-04 payload could not be decrypted")]
    DecryptFailed,
    #[error("NIP-04 plaintext is not UTF-8")]
    InvalidUtf8,
}

pub fn nip04_decrypt(
    secret_hex: &str,
    peer_pubkey: &Pubkey,
    payload: &str,
) -> Result<String, Nip04Error> {
    let (ciphertext, iv) = payload.split_once("?iv=").ok_or(Nip04Error::MissingIv)?;
    let ciphertext = STANDARD
        .decode(ciphertext)
        .map_err(|_| Nip04Error::InvalidBase64)?;
    let iv = STANDARD.decode(iv).map_err(|_| Nip04Error::InvalidBase64)?;
    let key = nip04_shared_key(secret_hex, peer_pubkey)?;
    let plaintext = Aes256CbcDec::new_from_slices(&key, &iv)
        .map_err(|_| Nip04Error::DecryptFailed)?
        .decrypt_padded_vec_mut::<Pkcs7>(&ciphertext)
        .map_err(|_| Nip04Error::DecryptFailed)?;
    String::from_utf8(plaintext).map_err(|_| Nip04Error::InvalidUtf8)
}

pub fn nip04_encrypt(
    secret_hex: &str,
    peer_pubkey: &Pubkey,
    plaintext: &str,
    iv: [u8; 16],
) -> Result<String, Nip04Error> {
    let key = nip04_shared_key(secret_hex, peer_pubkey)?;
    let ciphertext = Aes256CbcEnc::new_from_slices(&key, &iv)
        .map_err(|_| Nip04Error::DecryptFailed)?
        .encrypt_padded_vec_mut::<Pkcs7>(plaintext.as_bytes());
    Ok(format!(
        "{}?iv={}",
        STANDARD.encode(ciphertext),
        STANDARD.encode(iv)
    ))
}

fn nip04_shared_key(secret_hex: &str, peer_pubkey: &Pubkey) -> Result<[u8; 32], Nip04Error> {
    let secret_bytes = hex::decode(secret_hex).map_err(|_| Nip04Error::InvalidPrivateKey)?;
    let secret_key =
        SecretKey::from_slice(&secret_bytes).map_err(|_| Nip04Error::InvalidPrivateKey)?;
    let peer_bytes =
        hex::decode(peer_pubkey.as_str()).map_err(|_| Nip04Error::InvalidPeerPubkey)?;
    if peer_bytes.len() != 32 {
        return Err(Nip04Error::InvalidPeerPubkey);
    }
    let mut compressed = [0u8; 33];
    compressed[0] = 0x02;
    compressed[1..].copy_from_slice(&peer_bytes);
    let peer_public_key =
        PublicKey::from_slice(&compressed).map_err(|_| Nip04Error::InvalidPeerPubkey)?;
    let shared_point = ecdh::shared_secret_point(&peer_public_key, &secret_key);
    let mut key = [0u8; 32];
    key.copy_from_slice(&shared_point[..32]);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey_from_secret_hex;

    #[test]
    fn round_trips_nip04_payload() {
        let alice = "1111111111111111111111111111111111111111111111111111111111111111";
        let bob = "2222222222222222222222222222222222222222222222222222222222222222";
        let bob_pubkey = pubkey_from_secret_hex(bob).unwrap();
        let alice_pubkey = pubkey_from_secret_hex(alice).unwrap();
        let iv = [7u8; 16];

        let encrypted = nip04_encrypt(alice, &bob_pubkey, "[[\"i\",\"x\"]]", iv).unwrap();
        let decrypted = nip04_decrypt(bob, &alice_pubkey, &encrypted).unwrap();

        assert_eq!(decrypted, "[[\"i\",\"x\"]]");
    }
}
