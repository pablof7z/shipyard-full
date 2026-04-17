use chrono::{DateTime, Utc};
use secp256k1::{schnorr::Signature, Message, Secp256k1, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::model::Pubkey;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthProof {
    pub event: AuthEvent,
    pub expected_domain: String,
    pub expected_method: String,
    pub expected_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u64,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_pubkey: Pubkey,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthProofError {
    #[error("auth proof must use kind 27235")]
    WrongKind,
    #[error("auth proof timestamp is outside the allowed window")]
    StaleTimestamp,
    #[error("auth proof domain does not match")]
    DomainMismatch,
    #[error("auth proof method does not match")]
    MethodMismatch,
    #[error("auth proof url does not match")]
    UrlMismatch,
    #[error("auth proof event id does not match event contents")]
    EventIdMismatch,
    #[error("auth proof signature is invalid")]
    InvalidSignature,
    #[error("auth proof contains invalid hex")]
    InvalidHex,
    #[error("auth proof pubkey is invalid")]
    InvalidPubkey,
}

impl AuthProof {
    pub fn verify(&self, now: DateTime<Utc>) -> Result<Pubkey, AuthProofError> {
        self.event.verify_http_auth(
            &self.expected_domain,
            &self.expected_method,
            &self.expected_url,
            now,
        )
    }
}

impl AuthEvent {
    pub fn verify_http_auth(
        &self,
        expected_domain: &str,
        expected_method: &str,
        expected_url: &str,
        now: DateTime<Utc>,
    ) -> Result<Pubkey, AuthProofError> {
        if self.kind != 27_235 {
            return Err(AuthProofError::WrongKind);
        }

        let age_seconds = (now.timestamp() - self.created_at).abs();
        if age_seconds > 180 {
            return Err(AuthProofError::StaleTimestamp);
        }

        if tag_value(&self.tags, "domain") != Some(expected_domain) {
            return Err(AuthProofError::DomainMismatch);
        }

        if tag_value(&self.tags, "method")
            .map(str::to_ascii_uppercase)
            .as_deref()
            != Some(expected_method)
        {
            return Err(AuthProofError::MethodMismatch);
        }

        if tag_value(&self.tags, "u") != Some(expected_url) {
            return Err(AuthProofError::UrlMismatch);
        }

        let serialized = serde_json::json!([
            0,
            self.pubkey,
            self.created_at,
            self.kind,
            self.tags,
            self.content
        ]);
        let digest = Sha256::digest(serialized.to_string().as_bytes());
        let calculated_id = hex::encode(digest);
        if calculated_id != self.id {
            return Err(AuthProofError::EventIdMismatch);
        }

        let id_bytes = hex::decode(&self.id).map_err(|_| AuthProofError::InvalidHex)?;
        let pubkey_bytes = hex::decode(&self.pubkey).map_err(|_| AuthProofError::InvalidHex)?;
        let signature_bytes = hex::decode(&self.sig).map_err(|_| AuthProofError::InvalidHex)?;
        let message =
            Message::from_digest_slice(&id_bytes).map_err(|_| AuthProofError::InvalidHex)?;
        let pubkey =
            XOnlyPublicKey::from_slice(&pubkey_bytes).map_err(|_| AuthProofError::InvalidPubkey)?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| AuthProofError::InvalidSignature)?;

        Secp256k1::verification_only()
            .verify_schnorr(&signature, &message, &pubkey)
            .map_err(|_| AuthProofError::InvalidSignature)?;

        Pubkey::parse(self.pubkey.clone()).map_err(|_| AuthProofError::InvalidPubkey)
    }
}

fn tag_value<'a>(tags: &'a [Vec<String>], name: &str) -> Option<&'a str> {
    tags.iter().find_map(|tag| {
        if tag.first().map(String::as_str) == Some(name) {
            tag.get(1).map(String::as_str)
        } else {
            None
        }
    })
}
