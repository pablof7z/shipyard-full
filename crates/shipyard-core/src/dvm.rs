use serde::{Deserialize, Serialize};

use crate::event::{EventSigningError, EventValidationError, NostrEvent};
use crate::model::Pubkey;

pub const DVM_SCHEDULE_KIND: u64 = 5905;
pub const DVM_FEEDBACK_KIND: u64 = 7000;
pub const DEFAULT_DVM_METHOD: &str = "publish";

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

impl From<&DvmRequestEvent> for NostrEvent {
    fn from(event: &DvmRequestEvent) -> Self {
        Self {
            id: Some(event.id.clone()),
            pubkey: event.pubkey.clone(),
            created_at: event.created_at,
            kind: event.kind,
            tags: event.tags.clone(),
            content: event.content.clone(),
            sig: event.sig.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDvmRequest {
    pub request_event_id: String,
    pub request_pubkey: Pubkey,
    pub encrypted: bool,
    pub method: String,
    pub params: Vec<Vec<String>>,
    pub target_event_id: String,
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
    #[error("DVM request input event signature is invalid")]
    InvalidInputEventSignature,
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

pub fn method_from_tags(tags: &[Vec<String>]) -> String {
    tags.iter()
        .find(|tag| tag.first().map(String::as_str) == Some("method"))
        .and_then(|tag| tag.get(1))
        .filter(|method| !method.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| DEFAULT_DVM_METHOD.to_string())
}

pub fn params_from_tags(tags: &[Vec<String>]) -> Vec<Vec<String>> {
    tags.iter()
        .filter(|tag| matches!(tag.first().map(String::as_str), Some("param" | "params")))
        .cloned()
        .collect()
}

pub fn relay_targets_from_tags(tags: &[Vec<String>]) -> Vec<String> {
    let mut relays = Vec::new();
    for value in tags
        .iter()
        .filter(|tag| tag.first().map(String::as_str) == Some("relays"))
        .flat_map(|tag| tag.iter().skip(1))
        .filter(|value| is_relay_url(value))
    {
        push_unique(&mut relays, value.clone());
    }
    relays
}

pub fn first_input_event_id_from_tags(tags: &[Vec<String>]) -> Option<String> {
    tags.iter()
        .filter(|tag| tag.first().map(String::as_str) == Some("i"))
        .find_map(|tag| input_event_id_from_tag(tag))
}

fn parse_dvm_request_tags(
    event: &DvmRequestEvent,
    tags: &[Vec<String>],
    encrypted: bool,
) -> Result<ParsedDvmRequest, DvmParseError> {
    if event.kind != DVM_SCHEDULE_KIND {
        return Err(DvmParseError::WrongKind);
    }

    let method = method_from_tags(tags);
    let params = params_from_tags(tags);
    let relay_targets = relay_targets_from_tags(tags);
    let mut scheduled_events = Vec::new();
    let mut target_event_id = None;

    for tag in tags {
        if tag.first().map(String::as_str) != Some("i") {
            continue;
        }
        let raw_event = tag.get(1).ok_or(DvmParseError::InvalidInputEvent)?;
        let parsed = serde_json::from_str::<NostrEvent>(raw_event)
            .map_err(|_| DvmParseError::InvalidInputEvent)?;
        parsed
            .validate_signed_for_owner(&parsed.pubkey, None)
            .map_err(|e| match e {
                EventValidationError::MissingId | EventValidationError::MissingSignature => {
                    DvmParseError::UnsignedInputEvent
                }
                _ => DvmParseError::InvalidInputEventSignature,
            })?;
        let event_id = parsed.id.as_deref().unwrap_or_default();
        target_event_id.get_or_insert_with(|| event_id.to_string());
        scheduled_events.push(parsed);
    }

    if scheduled_events.is_empty() {
        return Err(DvmParseError::MissingInputEvents);
    }

    Ok(ParsedDvmRequest {
        request_event_id: event.id.clone(),
        request_pubkey: event.pubkey.clone(),
        encrypted,
        method,
        params,
        target_event_id: target_event_id.ok_or(DvmParseError::MissingInputEvents)?,
        relay_targets,
        scheduled_events,
    })
}

fn input_event_id_from_tag(tag: &[String]) -> Option<String> {
    let raw_input = tag.get(1)?;
    if is_event_id(raw_input) {
        return Some(raw_input.clone());
    }
    serde_json::from_str::<NostrEvent>(raw_input)
        .ok()
        .and_then(|event| event.id)
        .filter(|event_id| is_event_id(event_id))
}

fn is_event_id(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_relay_url(value: &str) -> bool {
    value.starts_with("wss://") || value.starts_with("ws://")
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DvmFeedbackMetadata<'a> {
    pub status: &'a str,
    pub request_event_id: &'a str,
    pub recipient_pubkey: &'a Pubkey,
    pub method: &'a str,
    pub result: &'a str,
    pub message: Option<&'a str>,
}

pub fn feedback_tags(metadata: DvmFeedbackMetadata<'_>) -> Vec<Vec<String>> {
    let mut tags = vec![
        vec!["e".to_string(), metadata.request_event_id.to_string()],
        vec![
            "p".to_string(),
            metadata.recipient_pubkey.as_str().to_string(),
        ],
        vec!["method".to_string(), metadata.method.to_string()],
        vec!["status".to_string(), metadata.status.to_string()],
        vec!["result".to_string(), metadata.result.to_string()],
    ];

    if let Some(message) = metadata.message {
        tags.push(vec!["alt".to_string(), message.to_string()]);
    }

    tags
}

pub fn build_signed_feedback_event(
    secret_hex: &str,
    metadata: DvmFeedbackMetadata<'_>,
    created_at: i64,
) -> Result<NostrEvent, EventSigningError> {
    let mut event = NostrEvent::unsigned(
        Pubkey::parse("0".repeat(64)).map_err(|_| EventSigningError::InvalidPrivateKey)?,
        created_at,
        DVM_FEEDBACK_KIND,
        feedback_tags(metadata),
        String::new(),
    );
    event.sign_with_secret_hex(secret_hex)?;
    Ok(event)
}

#[cfg(test)]
mod tests;
