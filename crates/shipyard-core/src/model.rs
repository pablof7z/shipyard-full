use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pubkey(String);

impl Pubkey {
    pub fn parse(value: impl Into<String>) -> Result<Self, PubkeyError> {
        let value = value.into();
        let is_hex = value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit());
        let is_npub = value.starts_with("npub1") && value.len() >= 10;

        if is_hex || is_npub {
            Ok(Self(value))
        } else {
            Err(PubkeyError::InvalidFormat)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PubkeyError {
    #[error("pubkey must be 64 hex characters or npub format")]
    InvalidFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishState {
    Proposed,
    Rejected,
    NeedsSignature,
    Signed,
    Scheduled,
    Publishing,
    Published,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishTrigger {
    SendNow,
    Time,
    Queue,
    Dvm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Queue {
    pub id: Uuid,
    pub owner_pubkey: Pubkey,
    pub name: String,
    pub description: Option<String>,
    pub cadence_seconds: i64,
    pub start_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublishItem {
    pub id: Uuid,
    pub owner_pubkey: Pubkey,
    pub created_by_pubkey: Pubkey,
    pub state: PublishState,
    pub trigger: PublishTrigger,
    pub unsigned_event_json: Option<serde_json::Value>,
    pub signed_event_json: Option<serde_json::Value>,
    pub event_id: Option<String>,
    pub publish_time: Option<DateTime<Utc>>,
    pub queue_id: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub published_to: Vec<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PublishItemInvariantError {
    #[error("{state:?} publish items must include signed event JSON")]
    MissingSignedEvent { state: PublishState },
    #[error("{state:?} publish items must include publish time")]
    MissingPublishTime { state: PublishState },
    #[error("failed publish items must include failure details")]
    MissingFailureDetails,
    #[error("cancelled publish items cannot be marked published")]
    CancelledItemPublished,
}

impl PublishItem {
    pub fn validate_state_invariants(&self) -> Result<(), PublishItemInvariantError> {
        if matches!(
            self.state,
            PublishState::Scheduled | PublishState::Publishing | PublishState::Published
        ) && self.signed_event_json.is_none()
        {
            return Err(PublishItemInvariantError::MissingSignedEvent { state: self.state });
        }

        if matches!(
            self.state,
            PublishState::Scheduled | PublishState::Publishing | PublishState::Published
        ) && self.publish_time.is_none()
        {
            return Err(PublishItemInvariantError::MissingPublishTime { state: self.state });
        }

        if self.state == PublishState::Failed
            && (self.failure_code.is_none() || self.failure_message.is_none())
        {
            return Err(PublishItemInvariantError::MissingFailureDetails);
        }

        if self.state == PublishState::Cancelled && self.published_at.is_some() {
            return Err(PublishItemInvariantError::CancelledItemPublished);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizedAccount {
    pub owner_pubkey: Pubkey,
    pub relationship: AccountRelationship,
    pub can_propose: bool,
    pub can_sign: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRelationship {
    Owner,
    Delegate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn pubkey() -> Pubkey {
        Pubkey::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap()
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap()
    }

    fn publish_item(state: PublishState) -> PublishItem {
        PublishItem {
            id: Uuid::nil(),
            owner_pubkey: pubkey(),
            created_by_pubkey: pubkey(),
            state,
            trigger: PublishTrigger::Time,
            unsigned_event_json: Some(json!({ "kind": 1, "content": "draft" })),
            signed_event_json: Some(json!({ "id": "event" })),
            event_id: Some("event".to_string()),
            publish_time: Some(now()),
            queue_id: None,
            published_at: None,
            published_to: vec![],
            failure_code: if state == PublishState::Failed {
                Some("relay_error".to_string())
            } else {
                None
            },
            failure_message: if state == PublishState::Failed {
                Some("Relay rejected event".to_string())
            } else {
                None
            },
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn parses_valid_hex_and_npub_pubkeys() {
        let hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(Pubkey::parse(hex).unwrap().as_str(), hex);

        let uppercase_hex = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert_eq!(
            Pubkey::parse(uppercase_hex).unwrap().as_str(),
            uppercase_hex
        );

        let npub = "npub1shipyarddelegate";
        assert_eq!(Pubkey::parse(npub).unwrap().as_str(), npub);
    }

    #[test]
    fn rejects_invalid_pubkeys_with_typed_error() {
        for value in [
            String::new(),
            "abc".to_string(),
            "a".repeat(63),
            "g".repeat(64),
            "npub1".to_string(),
        ] {
            assert_eq!(
                Pubkey::parse(value).unwrap_err(),
                PubkeyError::InvalidFormat
            );
        }
    }

    #[test]
    fn serializes_domain_enums_without_stringly_internal_state() {
        assert_eq!(
            serde_json::to_value(PublishState::NeedsSignature).unwrap(),
            json!("NEEDS_SIGNATURE")
        );
        assert_eq!(
            serde_json::from_value::<PublishState>(json!("NEEDS_SIGNATURE")).unwrap(),
            PublishState::NeedsSignature
        );
        assert!(serde_json::from_value::<PublishState>(json!("needs_signature")).is_err());

        assert_eq!(
            serde_json::to_value(PublishTrigger::SendNow).unwrap(),
            json!("SEND_NOW")
        );
        assert_eq!(
            serde_json::to_value(AccountRelationship::Delegate).unwrap(),
            json!("delegate")
        );
    }

    #[test]
    fn validates_publish_item_state_invariants_with_typed_errors() {
        let mut missing_signed_event = publish_item(PublishState::Scheduled);
        missing_signed_event.signed_event_json = None;
        assert_eq!(
            missing_signed_event
                .validate_state_invariants()
                .unwrap_err(),
            PublishItemInvariantError::MissingSignedEvent {
                state: PublishState::Scheduled
            }
        );

        let mut missing_publish_time = publish_item(PublishState::Publishing);
        missing_publish_time.publish_time = None;
        assert_eq!(
            missing_publish_time
                .validate_state_invariants()
                .unwrap_err(),
            PublishItemInvariantError::MissingPublishTime {
                state: PublishState::Publishing
            }
        );

        let mut missing_failure_details = publish_item(PublishState::Failed);
        missing_failure_details.failure_code = None;
        assert_eq!(
            missing_failure_details
                .validate_state_invariants()
                .unwrap_err(),
            PublishItemInvariantError::MissingFailureDetails
        );

        let mut cancelled_after_publish = publish_item(PublishState::Cancelled);
        cancelled_after_publish.published_at = Some(now());
        assert_eq!(
            cancelled_after_publish
                .validate_state_invariants()
                .unwrap_err(),
            PublishItemInvariantError::CancelledItemPublished
        );

        assert!(publish_item(PublishState::Scheduled)
            .validate_state_invariants()
            .is_ok());
    }
}
