use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::{ApiErrorBody, NostrEvent, Pubkey, PublishState, PublishTrigger, Queue};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevicePlatform {
    Ios,
    Android,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterDeviceRequest {
    pub platform: DevicePlatform,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_pubkey: Option<Pubkey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateDeviceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_pubkey: Option<Pubkey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTokenResponse {
    pub id: Uuid,
    pub user_pubkey: Pubkey,
    pub owner_pubkey: Option<Pubkey>,
    pub platform: DevicePlatform,
    pub token: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueNextSlotPreviewRequest {
    pub queue: Queue,
    pub now: DateTime<Utc>,
    pub latest_queue_slot: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueNextSlotResponse {
    pub queue_id: Uuid,
    pub owner_pubkey: Pubkey,
    pub next_slot: DateTime<Utc>,
    pub latest_queue_slot: Option<DateTime<Utc>>,
    pub now: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleSignedEventRequest {
    pub signed_event: NostrEvent,
    pub trigger: PublishTrigger,
    pub publish_time: Option<DateTime<Utc>>,
    pub queue_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProposalRequest {
    pub owner_pubkey: Pubkey,
    pub unsigned_event: NostrEvent,
    pub trigger: PublishTrigger,
    pub publish_time: Option<DateTime<Utc>>,
    pub queue_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditProposalRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned_event: Option<NostrEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<PublishTrigger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectProposalRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignProposalRequest {
    pub signed_event: NostrEvent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSignProposalRequest {
    pub items: Vec<BatchSignProposalItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSignProposalItem {
    pub proposal_id: Uuid,
    pub signed_event: NostrEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSignProposalResponse {
    pub results: Vec<BatchSignProposalResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSignProposalResult {
    pub proposal_id: Uuid,
    pub item: Option<PublishItemResponse>,
    pub error: Option<ApiErrorBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishItemResponse {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pubkey() -> Pubkey {
        Pubkey::parse("a".repeat(64)).unwrap()
    }

    fn event() -> NostrEvent {
        NostrEvent::unsigned(pubkey(), 1_776_000_000, 1, vec![], "hello".to_string())
    }

    #[test]
    fn serializes_device_registration_like_api_contract() {
        let request = RegisterDeviceRequest {
            platform: DevicePlatform::Ios,
            token: " token ".to_string(),
            owner_pubkey: Some(pubkey()),
            enabled: Some(true),
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "platform": "ios",
                "token": " token ",
                "owner_pubkey": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "enabled": true
            })
        );
    }

    #[test]
    fn serializes_schedule_payload_with_core_trigger_names() {
        let request = ScheduleSignedEventRequest {
            signed_event: event(),
            trigger: PublishTrigger::Queue,
            publish_time: None,
            queue_id: Some(Uuid::nil()),
        };

        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["trigger"], "QUEUE");
        assert_eq!(value["signed_event"]["kind"], 1);
    }
}
