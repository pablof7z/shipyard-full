use chrono::{DateTime, Utc};
use shipyard_core::{
    assert_transition, next_queue_slot, Actor, EventValidationError, NostrEvent, Pubkey,
    PublishItem, PublishItemInvariantError, PublishState, PublishTrigger, Queue, QueueSlotError,
    StateTransitionError,
};

use crate::api::{
    CreateProposalRequest, QueueNextSlotPreviewRequest, QueueNextSlotResponse,
    RegisterDeviceRequest, ScheduleSignedEventRequest, UpdateDeviceRequest,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MobileValidationError {
    #[error("device token is required")]
    DeviceTokenRequired,
    #[error("publish time is required for timed scheduling")]
    PublishTimeRequired,
    #[error("queue id is required for queued scheduling")]
    QueueRequired,
    #[error("queue id is not allowed for this trigger")]
    QueueNotAllowed,
    #[error(transparent)]
    Event(#[from] EventValidationError),
    #[error(transparent)]
    PublishItem(#[from] PublishItemInvariantError),
    #[error(transparent)]
    StateTransition(#[from] StateTransitionError),
}

pub fn validate_device_registration(
    request: &RegisterDeviceRequest,
) -> Result<(), MobileValidationError> {
    if request.token.trim().is_empty() {
        return Err(MobileValidationError::DeviceTokenRequired);
    }

    Ok(())
}

pub fn validate_device_update(_request: &UpdateDeviceRequest) -> Result<(), MobileValidationError> {
    Ok(())
}

pub fn preview_next_queue_slot(
    queue: &Queue,
    now: DateTime<Utc>,
    latest_queue_slot: Option<DateTime<Utc>>,
) -> Result<QueueNextSlotResponse, QueueSlotError> {
    let next_slot = next_queue_slot(queue, now, latest_queue_slot)?;

    Ok(QueueNextSlotResponse {
        queue_id: queue.id,
        owner_pubkey: queue.owner_pubkey.clone(),
        next_slot,
        latest_queue_slot,
        now,
    })
}

pub fn preview_next_queue_slot_request(
    request: &QueueNextSlotPreviewRequest,
) -> Result<QueueNextSlotResponse, QueueSlotError> {
    preview_next_queue_slot(&request.queue, request.now, request.latest_queue_slot)
}

pub fn validate_signed_schedule(
    owner_pubkey: &Pubkey,
    request: &ScheduleSignedEventRequest,
) -> Result<(), MobileValidationError> {
    validate_trigger_inputs(request.trigger, request.publish_time, request.queue_id)?;
    request
        .signed_event
        .validate_signed_for_owner(owner_pubkey, request.publish_time)?;
    Ok(())
}

pub fn validate_proposal_request(
    request: &CreateProposalRequest,
) -> Result<(), MobileValidationError> {
    validate_trigger_inputs(request.trigger, request.publish_time, request.queue_id)
}

pub fn validate_signed_event_for_owner(
    event: &NostrEvent,
    owner_pubkey: &Pubkey,
    publish_time: Option<DateTime<Utc>>,
) -> Result<(), EventValidationError> {
    event.validate_signed_for_owner(owner_pubkey, publish_time)
}

pub fn validate_publish_item(item: &PublishItem) -> Result<(), PublishItemInvariantError> {
    item.validate_state_invariants()
}

pub fn validate_state_transition(
    actor: Actor,
    from: PublishState,
    to: PublishState,
) -> Result<(), StateTransitionError> {
    assert_transition(actor, from, to)
}

pub fn validate_trigger_inputs(
    trigger: PublishTrigger,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<uuid::Uuid>,
) -> Result<(), MobileValidationError> {
    match trigger {
        PublishTrigger::Time => {
            if publish_time.is_none() {
                return Err(MobileValidationError::PublishTimeRequired);
            }
            if queue_id.is_some() {
                return Err(MobileValidationError::QueueNotAllowed);
            }
        }
        PublishTrigger::Queue => {
            if queue_id.is_none() {
                return Err(MobileValidationError::QueueRequired);
            }
        }
        PublishTrigger::SendNow => {
            if queue_id.is_some() {
                return Err(MobileValidationError::QueueNotAllowed);
            }
        }
        PublishTrigger::Dvm => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DevicePlatform;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn pubkey() -> Pubkey {
        Pubkey::parse("a".repeat(64)).unwrap()
    }

    fn queue() -> Queue {
        Queue {
            id: Uuid::nil(),
            owner_pubkey: pubkey(),
            name: "Daily".to_string(),
            description: None,
            cadence_seconds: 86_400,
            start_at: Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap(),
            archived_at: None,
        }
    }

    #[test]
    fn validates_device_registration_tokens() {
        let request = RegisterDeviceRequest {
            platform: DevicePlatform::Android,
            token: "   ".to_string(),
            owner_pubkey: None,
            enabled: None,
        };

        assert_eq!(
            validate_device_registration(&request).unwrap_err(),
            MobileValidationError::DeviceTokenRequired
        );
    }

    #[test]
    fn previews_next_slot_with_core_queue_logic() {
        let now = Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap();
        let preview = preview_next_queue_slot(&queue(), now, None).unwrap();

        assert_eq!(
            preview.next_slot,
            Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap()
        );

        let request = QueueNextSlotPreviewRequest {
            queue: queue(),
            now,
            latest_queue_slot: None,
        };
        assert_eq!(
            preview_next_queue_slot_request(&request).unwrap().next_slot,
            preview.next_slot
        );
    }

    #[test]
    fn enforces_trigger_shape_before_api_submission() {
        assert_eq!(
            validate_trigger_inputs(PublishTrigger::Time, None, None).unwrap_err(),
            MobileValidationError::PublishTimeRequired
        );
        assert_eq!(
            validate_trigger_inputs(PublishTrigger::Queue, None, None).unwrap_err(),
            MobileValidationError::QueueRequired
        );
        assert_eq!(
            validate_trigger_inputs(PublishTrigger::SendNow, None, Some(Uuid::nil())).unwrap_err(),
            MobileValidationError::QueueNotAllowed
        );
    }
}
