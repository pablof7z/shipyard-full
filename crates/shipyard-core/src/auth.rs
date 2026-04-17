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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use secp256k1::{Keypair, SecretKey, XOnlyPublicKey};

    const SECRET: &str = "3333333333333333333333333333333333333333333333333333333333333333";
    const DOMAIN: &str = "api.shipyard.example";
    const METHOD: &str = "GET";
    const URL: &str = "https://api.shipyard.example/v1/session";

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap()
    }

    fn auth_tags() -> Vec<Vec<String>> {
        vec![
            vec!["domain".to_string(), DOMAIN.to_string()],
            vec!["method".to_string(), "get".to_string()],
            vec!["u".to_string(), URL.to_string()],
        ]
    }

    fn signed_auth_event(created_at: i64, kind: u64, tags: Vec<Vec<String>>) -> AuthEvent {
        let secp = Secp256k1::new();
        let secret_bytes = hex::decode(SECRET).unwrap();
        let secret_key = SecretKey::from_slice(&secret_bytes).unwrap();
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let (public_key, _) = XOnlyPublicKey::from_keypair(&keypair);
        let pubkey = hex::encode(public_key.serialize());
        let content = String::new();
        let serialized = serde_json::json!([0, pubkey, created_at, kind, tags, content]);
        let id = hex::encode(Sha256::digest(serialized.to_string().as_bytes()));
        let id_bytes = hex::decode(&id).unwrap();
        let message = Message::from_digest_slice(&id_bytes).unwrap();
        let sig = secp
            .sign_schnorr_no_aux_rand(&message, &keypair)
            .to_string();

        AuthEvent {
            id,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        }
    }

    #[test]
    fn verifies_valid_kind_27235_auth_proof_and_returns_delegate_pubkey() {
        let event = signed_auth_event(now().timestamp(), 27_235, auth_tags());
        let proof = AuthProof {
            event: event.clone(),
            expected_domain: DOMAIN.to_string(),
            expected_method: METHOD.to_string(),
            expected_url: URL.to_string(),
        };

        let delegate_pubkey = proof.verify(now()).unwrap();

        assert_eq!(delegate_pubkey.as_str(), event.pubkey);
    }

    #[test]
    fn rejects_wrong_kind_events() {
        let event = signed_auth_event(now().timestamp(), 1, auth_tags());

        assert_eq!(
            event
                .verify_http_auth(DOMAIN, METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::WrongKind
        );
    }

    #[test]
    fn rejects_stale_timestamps() {
        let stale_past = signed_auth_event(
            (now() - Duration::seconds(181)).timestamp(),
            27_235,
            auth_tags(),
        );
        assert_eq!(
            stale_past
                .verify_http_auth(DOMAIN, METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::StaleTimestamp
        );

        let stale_future = signed_auth_event(
            (now() + Duration::seconds(181)).timestamp(),
            27_235,
            auth_tags(),
        );
        assert_eq!(
            stale_future
                .verify_http_auth(DOMAIN, METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::StaleTimestamp
        );
    }

    #[test]
    fn validates_domain_method_and_url_tags() {
        let event = signed_auth_event(now().timestamp(), 27_235, auth_tags());

        assert_eq!(
            event
                .verify_http_auth("other.example", METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::DomainMismatch
        );
        assert_eq!(
            event
                .verify_http_auth(DOMAIN, "POST", URL, now())
                .unwrap_err(),
            AuthProofError::MethodMismatch
        );
        assert_eq!(
            event
                .verify_http_auth(DOMAIN, METHOD, "https://other.example/session", now())
                .unwrap_err(),
            AuthProofError::UrlMismatch
        );
    }

    #[test]
    fn rejects_event_id_mismatches_and_invalid_signatures() {
        let event = signed_auth_event(now().timestamp(), 27_235, auth_tags());

        let mut wrong_id = event.clone();
        wrong_id.id = "f".repeat(64);
        assert_eq!(
            wrong_id
                .verify_http_auth(DOMAIN, METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::EventIdMismatch
        );

        let mut wrong_signature = event;
        wrong_signature.sig = "00".repeat(64);
        assert_eq!(
            wrong_signature
                .verify_http_auth(DOMAIN, METHOD, URL, now())
                .unwrap_err(),
            AuthProofError::InvalidSignature
        );
    }
}
