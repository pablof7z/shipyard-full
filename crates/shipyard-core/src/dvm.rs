use serde::{Deserialize, Serialize};

use crate::event::{EventSigningError, NostrEvent};
use crate::model::Pubkey;

pub const DVM_SCHEDULE_KIND: u64 = 5905;
pub const DVM_FEEDBACK_KIND: u64 = 7000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DvmRequestEvent {
    pub id: String,
    pub pubkey: Pubkey,
    pub created_at: i64,
    pub kind: u64,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDvmRequest {
    pub request_event_id: String,
    pub request_pubkey: Pubkey,
    pub encrypted: bool,
    pub relay_targets: Vec<String>,
    pub scheduled_events: Vec<NostrEvent>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DvmParseError {
    #[error("DVM request must use kind 5905")]
    WrongKind,
    #[error("DVM request has no signed input events")]
    MissingInputEvents,
    #[error("DVM request input event JSON is invalid")]
    InvalidInputEvent,
    #[error("DVM request input event must be signed")]
    UnsignedInputEvent,
}

pub fn parse_dvm_request(event: &DvmRequestEvent) -> Result<ParsedDvmRequest, DvmParseError> {
    parse_dvm_request_tags(event, &event.tags, false)
}

pub fn parse_encrypted_dvm_request(
    event: &DvmRequestEvent,
    decrypted_tags: Vec<Vec<String>>,
) -> Result<ParsedDvmRequest, DvmParseError> {
    parse_dvm_request_tags(event, &decrypted_tags, true)
}

fn parse_dvm_request_tags(
    event: &DvmRequestEvent,
    tags: &[Vec<String>],
    encrypted: bool,
) -> Result<ParsedDvmRequest, DvmParseError> {
    if event.kind != DVM_SCHEDULE_KIND {
        return Err(DvmParseError::WrongKind);
    }

    let mut scheduled_events = Vec::new();
    let mut relay_targets = Vec::new();

    for tag in tags {
        match tag.first().map(String::as_str) {
            Some("i") => {
                let raw_event = tag.get(1).ok_or(DvmParseError::InvalidInputEvent)?;
                let parsed = serde_json::from_str::<NostrEvent>(raw_event)
                    .map_err(|_| DvmParseError::InvalidInputEvent)?;
                if parsed.id.as_deref().unwrap_or_default().is_empty()
                    || parsed.sig.as_deref().unwrap_or_default().is_empty()
                {
                    return Err(DvmParseError::UnsignedInputEvent);
                }
                scheduled_events.push(parsed);
            }
            Some("relays") => {
                relay_targets.extend(
                    tag.iter()
                        .skip(1)
                        .filter(|value| value.starts_with("wss://"))
                        .cloned(),
                );
            }
            _ => {}
        }
    }

    if scheduled_events.is_empty() {
        return Err(DvmParseError::MissingInputEvents);
    }

    Ok(ParsedDvmRequest {
        request_event_id: event.id.clone(),
        request_pubkey: event.pubkey.clone(),
        encrypted,
        relay_targets,
        scheduled_events,
    })
}

pub fn feedback_tags(
    status: &str,
    request_event_id: &str,
    message: Option<&str>,
) -> Vec<Vec<String>> {
    let mut tags = vec![
        vec!["e".to_string(), request_event_id.to_string()],
        vec!["status".to_string(), status.to_string()],
    ];

    if let Some(message) = message {
        tags.push(vec!["alt".to_string(), message.to_string()]);
    }

    tags
}

pub fn build_signed_feedback_event(
    secret_hex: &str,
    status: &str,
    request_event_id: &str,
    message: Option<&str>,
    created_at: i64,
) -> Result<NostrEvent, EventSigningError> {
    let mut event = NostrEvent::unsigned(
        Pubkey::parse("0".repeat(64)).map_err(|_| EventSigningError::InvalidPrivateKey)?,
        created_at,
        DVM_FEEDBACK_KIND,
        feedback_tags(status, request_event_id, message),
        String::new(),
    );
    event.sign_with_secret_hex(secret_hex)?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pubkey() -> Pubkey {
        Pubkey::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap()
    }

    fn signed_event_json() -> String {
        serde_json::to_string(&NostrEvent {
            id: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into()),
            pubkey: pubkey(),
            created_at: 1_776_432_000,
            kind: 1,
            tags: vec![],
            content: "Scheduled through DVM".into(),
            sig: Some("cccc".into()),
        })
        .unwrap()
    }

    #[test]
    fn parses_kind_5905_input_events_and_relays() {
        let event = DvmRequestEvent {
            id: "request".into(),
            pubkey: pubkey(),
            created_at: 1_776_431_900,
            kind: DVM_SCHEDULE_KIND,
            tags: vec![
                vec!["i".into(), signed_event_json(), "event".into()],
                vec!["relays".into(), "wss://relay.example".into()],
            ],
            content: String::new(),
            sig: Some("sig".into()),
        };

        let parsed = parse_dvm_request(&event).unwrap();
        assert_eq!(parsed.scheduled_events.len(), 1);
        assert_eq!(parsed.relay_targets, vec!["wss://relay.example"]);
    }

    #[test]
    fn parses_decrypted_encrypted_tags() {
        let event = DvmRequestEvent {
            id: "request".into(),
            pubkey: pubkey(),
            created_at: 1_776_431_900,
            kind: DVM_SCHEDULE_KIND,
            tags: vec![vec!["encrypted".into()]],
            content: "encrypted".into(),
            sig: Some("sig".into()),
        };

        let parsed = parse_encrypted_dvm_request(
            &event,
            vec![
                vec!["i".into(), signed_event_json(), "event".into()],
                vec!["relays".into(), "wss://relay.example".into()],
            ],
        )
        .unwrap();

        assert!(parsed.encrypted);
        assert_eq!(parsed.scheduled_events.len(), 1);
    }

    #[test]
    fn rejects_unsigned_input_event() {
        let mut raw = serde_json::from_str::<serde_json::Value>(&signed_event_json()).unwrap();
        raw["sig"] = serde_json::Value::Null;
        let event = DvmRequestEvent {
            id: "request".into(),
            pubkey: pubkey(),
            created_at: 1_776_431_900,
            kind: DVM_SCHEDULE_KIND,
            tags: vec![vec!["i".into(), raw.to_string()]],
            content: String::new(),
            sig: Some("sig".into()),
        };

        assert_eq!(
            parse_dvm_request(&event).unwrap_err(),
            DvmParseError::UnsignedInputEvent
        );
    }

    #[test]
    fn builds_legacy_feedback_tags() {
        assert_eq!(
            feedback_tags("scheduled", "abc", Some("Scheduled.")),
            vec![
                vec!["e".to_string(), "abc".to_string()],
                vec!["status".to_string(), "scheduled".to_string()],
                vec!["alt".to_string(), "Scheduled.".to_string()]
            ]
        );
    }

    #[test]
    fn signs_feedback_event() {
        let secret = "1111111111111111111111111111111111111111111111111111111111111111";
        let feedback = build_signed_feedback_event(
            secret,
            "scheduled",
            "abc",
            Some("Scheduled."),
            1_700_000_000,
        )
        .unwrap();
        assert_eq!(feedback.kind, DVM_FEEDBACK_KIND);
        assert!(feedback.id.is_some());
        assert!(feedback.sig.is_some());
        assert!(feedback
            .tags
            .iter()
            .any(|tag| tag == &vec!["status", "scheduled"]));
    }
}
