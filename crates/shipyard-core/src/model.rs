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
