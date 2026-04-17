use chrono::{DateTime, Duration, Utc};

use crate::model::Queue;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueueSlotError {
    #[error("queue cadence must be positive")]
    NonPositiveCadence,
    #[error("queue is archived")]
    Archived,
}

pub fn next_queue_slot(
    queue: &Queue,
    now: DateTime<Utc>,
    latest_queue_slot: Option<DateTime<Utc>>,
) -> Result<DateTime<Utc>, QueueSlotError> {
    if queue.archived_at.is_some() {
        return Err(QueueSlotError::Archived);
    }

    if queue.cadence_seconds <= 0 {
        return Err(QueueSlotError::NonPositiveCadence);
    }

    let cadence = Duration::seconds(queue.cadence_seconds);
    let mut candidate = latest_queue_slot
        .map(|slot| slot + cadence)
        .unwrap_or(queue.start_at);

    let lower_bound = now.max(candidate);
    if candidate >= lower_bound {
        return Ok(candidate);
    }

    let elapsed = lower_bound
        .signed_duration_since(queue.start_at)
        .num_seconds()
        .max(0);
    let cadence_seconds = queue.cadence_seconds;
    let slots_elapsed = (elapsed + cadence_seconds - 1) / cadence_seconds;
    candidate = queue.start_at + Duration::seconds(slots_elapsed * cadence_seconds);

    if candidate < now {
        candidate += cadence;
    }

    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Pubkey, Queue};
    use chrono::TimeZone;
    use uuid::Uuid;

    fn queue() -> Queue {
        Queue {
            id: Uuid::nil(),
            owner_pubkey: Pubkey::parse(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
            name: "Daily".to_string(),
            description: None,
            cadence_seconds: 86_400,
            start_at: Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap(),
            archived_at: None,
        }
    }

    #[test]
    fn uses_start_when_no_items_and_start_is_future() {
        let queue = queue();
        let now = Utc.with_ymd_and_hms(2026, 4, 17, 9, 0, 0).unwrap();
        assert_eq!(next_queue_slot(&queue, now, None).unwrap(), queue.start_at);
    }

    #[test]
    fn advances_to_next_cadence_aligned_slot() {
        let queue = queue();
        let now = Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap();
        assert_eq!(next_queue_slot(&queue, now, None).unwrap(), expected);
    }

    #[test]
    fn follows_latest_queue_slot_when_later_than_now() {
        let queue = queue();
        let now = Utc.with_ymd_and_hms(2026, 4, 17, 9, 0, 0).unwrap();
        let latest = Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2026, 4, 21, 10, 0, 0).unwrap();
        assert_eq!(
            next_queue_slot(&queue, now, Some(latest)).unwrap(),
            expected
        );
    }
}
