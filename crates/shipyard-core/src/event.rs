use chrono::{DateTime, TimeZone, Utc};
use secp256k1::{Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::Pubkey;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NostrEvent {
    pub id: Option<String>,
    pub pubkey: Pubkey,
    pub created_at: i64,
    pub kind: u64,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventValidationError {
    #[error("event pubkey must match owner pubkey")]
    OwnerMismatch,
    #[error("signed event must include id")]
    MissingId,
    #[error("signed event must include signature")]
    MissingSignature,
    #[error("event created_at must match publish time")]
    CreatedAtMismatch,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventSigningError {
    #[error("private key must be 32-byte hex")]
    InvalidPrivateKey,
    #[error("event id hash is invalid")]
    InvalidEventHash,
}

impl NostrEvent {
    pub fn unsigned(
        pubkey: Pubkey,
        created_at: i64,
        kind: u64,
        tags: Vec<Vec<String>>,
        content: String,
    ) -> Self {
        Self {
            id: None,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig: None,
        }
    }

    pub fn created_at_datetime(&self) -> Option<DateTime<Utc>> {
        Utc.timestamp_opt(self.created_at, 0).single()
    }

    pub fn validate_signed_for_owner(
        &self,
        owner_pubkey: &Pubkey,
        publish_time: Option<DateTime<Utc>>,
    ) -> Result<(), EventValidationError> {
        if &self.pubkey != owner_pubkey {
            return Err(EventValidationError::OwnerMismatch);
        }
        if self.id.as_deref().unwrap_or_default().is_empty() {
            return Err(EventValidationError::MissingId);
        }
        if self.sig.as_deref().unwrap_or_default().is_empty() {
            return Err(EventValidationError::MissingSignature);
        }
        if let Some(publish_time) = publish_time {
            if self.created_at != publish_time.timestamp() {
                return Err(EventValidationError::CreatedAtMismatch);
            }
        }
        Ok(())
    }

    pub fn calculate_id(&self) -> String {
        let serialized = serde_json::json!([
            0,
            self.pubkey.as_str(),
            self.created_at,
            self.kind,
            self.tags,
            self.content
        ]);
        hex::encode(Sha256::digest(serialized.to_string().as_bytes()))
    }

    pub fn sign_with_secret_hex(&mut self, secret_hex: &str) -> Result<(), EventSigningError> {
        let secret_bytes =
            hex::decode(secret_hex).map_err(|_| EventSigningError::InvalidPrivateKey)?;
        let secret_key = SecretKey::from_slice(&secret_bytes)
            .map_err(|_| EventSigningError::InvalidPrivateKey)?;
        let secp = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
        self.pubkey = Pubkey::parse(hex::encode(public_key.serialize()))
            .map_err(|_| EventSigningError::InvalidPrivateKey)?;

        let id = self.calculate_id();
        let id_bytes = hex::decode(&id).map_err(|_| EventSigningError::InvalidEventHash)?;
        let message = Message::from_digest_slice(&id_bytes)
            .map_err(|_| EventSigningError::InvalidEventHash)?;
        let signature = secp.sign_schnorr_no_aux_rand(&message, &keypair);
        self.id = Some(id);
        self.sig = Some(signature.to_string());
        Ok(())
    }
}

pub fn pubkey_from_secret_hex(secret_hex: &str) -> Result<Pubkey, EventSigningError> {
    let secret_bytes = hex::decode(secret_hex).map_err(|_| EventSigningError::InvalidPrivateKey)?;
    let secret_key =
        SecretKey::from_slice(&secret_bytes).map_err(|_| EventSigningError::InvalidPrivateKey)?;
    let secp = Secp256k1::new();
    let keypair = Keypair::from_secret_key(&secp, &secret_key);
    let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
    Pubkey::parse(hex::encode(public_key.serialize()))
        .map_err(|_| EventSigningError::InvalidPrivateKey)
}
