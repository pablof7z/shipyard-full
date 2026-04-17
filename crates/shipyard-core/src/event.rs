use chrono::{DateTime, TimeZone, Utc};
use secp256k1::{schnorr::Signature, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
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
    #[error("event id must match event contents")]
    EventIdMismatch,
    #[error("event pubkey is invalid")]
    InvalidPubkey,
    #[error("event signature is invalid")]
    InvalidSignature,
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
        let id = self.id.as_deref().unwrap_or_default();
        if id.is_empty() {
            return Err(EventValidationError::MissingId);
        }
        let sig = self.sig.as_deref().unwrap_or_default();
        if sig.is_empty() {
            return Err(EventValidationError::MissingSignature);
        }
        if let Some(publish_time) = publish_time {
            if self.created_at != publish_time.timestamp() {
                return Err(EventValidationError::CreatedAtMismatch);
            }
        }

        if self.calculate_id() != id {
            return Err(EventValidationError::EventIdMismatch);
        }

        let id_bytes = hex::decode(id).map_err(|_| EventValidationError::EventIdMismatch)?;
        let pubkey_bytes =
            hex::decode(self.pubkey.as_str()).map_err(|_| EventValidationError::InvalidPubkey)?;
        let signature_bytes =
            hex::decode(sig).map_err(|_| EventValidationError::InvalidSignature)?;
        let message = Message::from_digest_slice(&id_bytes)
            .map_err(|_| EventValidationError::EventIdMismatch)?;
        let pubkey = XOnlyPublicKey::from_slice(&pubkey_bytes)
            .map_err(|_| EventValidationError::InvalidPubkey)?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|_| EventValidationError::InvalidSignature)?;

        Secp256k1::verification_only()
            .verify_schnorr(&signature, &message, &pubkey)
            .map_err(|_| EventValidationError::InvalidSignature)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use secp256k1::{schnorr::Signature, Message, Secp256k1, XOnlyPublicKey};

    const SECRET: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const OTHER_SECRET: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn placeholder_pubkey() -> Pubkey {
        Pubkey::parse("0".repeat(64)).unwrap()
    }

    fn fixture_tags() -> Vec<Vec<String>> {
        vec![
            vec![
                "e".to_string(),
                "b".repeat(64),
                "wss://relay.example".to_string(),
                "root".to_string(),
            ],
            vec!["t".to_string(), "shipyard".to_string()],
        ]
    }

    fn signed_event() -> NostrEvent {
        let mut event = NostrEvent::unsigned(
            placeholder_pubkey(),
            1_700_000_000,
            1,
            fixture_tags(),
            "Ahoy".to_string(),
        );
        event.sign_with_secret_hex(SECRET).unwrap();
        event
    }

    fn verify_schnorr_signature(event: &NostrEvent) -> bool {
        let id_bytes = hex::decode(event.id.as_deref().unwrap()).unwrap();
        let signature_bytes = hex::decode(event.sig.as_deref().unwrap()).unwrap();
        let pubkey_bytes = hex::decode(event.pubkey.as_str()).unwrap();
        let message = Message::from_digest_slice(&id_bytes).unwrap();
        let signature = Signature::from_slice(&signature_bytes).unwrap();
        let pubkey = XOnlyPublicKey::from_slice(&pubkey_bytes).unwrap();

        Secp256k1::verification_only()
            .verify_schnorr(&signature, &message, &pubkey)
            .is_ok()
    }

    #[test]
    fn calculates_event_id_from_core_fields() {
        let event = NostrEvent::unsigned(
            Pubkey::parse("a".repeat(64)).unwrap(),
            1_700_000_000,
            1,
            fixture_tags(),
            "Ahoy".to_string(),
        );

        assert_eq!(
            event.calculate_id(),
            "59d88ffc6e1d6af6c85d0a26bef9ae2c92c037704fa29188316689a640663672"
        );
    }

    #[test]
    fn signs_and_verifies_schnorr_signature() {
        let event = signed_event();
        let calculated_id = event.calculate_id();

        assert_eq!(event.id.as_deref(), Some(calculated_id.as_str()));
        assert!(verify_schnorr_signature(&event));
    }

    #[test]
    fn validates_required_owner_id_signature_and_publish_time() {
        let event = signed_event();
        let owner = event.pubkey.clone();
        let other_owner = pubkey_from_secret_hex(OTHER_SECRET).unwrap();

        assert_eq!(
            event
                .validate_signed_for_owner(&other_owner, None)
                .unwrap_err(),
            EventValidationError::OwnerMismatch
        );

        let mut missing_id = event.clone();
        missing_id.id = None;
        assert_eq!(
            missing_id
                .validate_signed_for_owner(&owner, None)
                .unwrap_err(),
            EventValidationError::MissingId
        );

        let mut missing_signature = event.clone();
        missing_signature.sig = Some(String::new());
        assert_eq!(
            missing_signature
                .validate_signed_for_owner(&owner, None)
                .unwrap_err(),
            EventValidationError::MissingSignature
        );

        let wrong_publish_time = Utc
            .timestamp_opt(event.created_at + 60, 0)
            .single()
            .unwrap();
        assert_eq!(
            event
                .validate_signed_for_owner(&owner, Some(wrong_publish_time))
                .unwrap_err(),
            EventValidationError::CreatedAtMismatch
        );
    }

    #[test]
    fn validate_signed_for_owner_checks_event_hash_and_signature() {
        let event = signed_event();
        let owner = event.pubkey.clone();

        let mut tampered_content = event.clone();
        tampered_content.content.push('!');
        assert_eq!(
            tampered_content
                .validate_signed_for_owner(&owner, None)
                .unwrap_err(),
            EventValidationError::EventIdMismatch
        );

        let mut bad_signature = event.clone();
        bad_signature.sig = Some("00".repeat(64));
        assert_eq!(
            bad_signature
                .validate_signed_for_owner(&owner, None)
                .unwrap_err(),
            EventValidationError::InvalidSignature
        );
    }

    #[test]
    fn reports_typed_signing_error_variants() {
        let mut invalid_hex_event = NostrEvent::unsigned(
            placeholder_pubkey(),
            1_700_000_000,
            1,
            vec![],
            String::new(),
        );
        assert_eq!(
            invalid_hex_event
                .sign_with_secret_hex("not-hex")
                .unwrap_err(),
            EventSigningError::InvalidPrivateKey
        );

        let mut invalid_scalar_event = NostrEvent::unsigned(
            placeholder_pubkey(),
            1_700_000_000,
            1,
            vec![],
            String::new(),
        );
        assert_eq!(
            invalid_scalar_event
                .sign_with_secret_hex(&"0".repeat(64))
                .unwrap_err(),
            EventSigningError::InvalidPrivateKey
        );

        let invalid_hash_error = EventSigningError::InvalidEventHash;
        assert!(matches!(
            invalid_hash_error,
            EventSigningError::InvalidEventHash
        ));
        assert_ne!(
            EventSigningError::InvalidEventHash,
            EventSigningError::InvalidPrivateKey
        );
    }

    #[test]
    fn signs_unsigned_event_workflow() {
        let expected_pubkey = pubkey_from_secret_hex(SECRET).unwrap();
        let mut event = NostrEvent::unsigned(
            placeholder_pubkey(),
            1_700_000_000,
            30_023,
            vec![vec!["d".to_string(), "shipyard".to_string()]],
            "draft".to_string(),
        );

        assert!(event.id.is_none());
        assert!(event.sig.is_none());

        event.sign_with_secret_hex(SECRET).unwrap();

        assert_eq!(event.pubkey, expected_pubkey);
        assert_eq!(event.id.as_deref(), Some(event.calculate_id().as_str()));
        assert_eq!(event.sig.as_ref().unwrap().len(), 128);
        assert!(verify_schnorr_signature(&event));
        assert!(event
            .validate_signed_for_owner(
                &expected_pubkey,
                Some(Utc.timestamp_opt(1_700_000_000, 0).single().unwrap())
            )
            .is_ok());
    }
}
