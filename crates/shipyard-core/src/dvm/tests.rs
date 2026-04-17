use super::*;
use crate::event::pubkey_from_secret_hex;

const INPUT_SECRET: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn pubkey() -> Pubkey {
    Pubkey::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap()
}

fn make_signed_input_event() -> NostrEvent {
    let mut event = NostrEvent::unsigned(
        Pubkey::parse("0".repeat(64)).unwrap(),
        1_776_432_000,
        1,
        vec![],
        "Scheduled through DVM".into(),
    );
    event.sign_with_secret_hex(INPUT_SECRET).unwrap();
    event
}

fn signed_event_json() -> String {
    serde_json::to_string(&make_signed_input_event()).unwrap()
}

fn input_event_id() -> String {
    make_signed_input_event().id.unwrap()
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
            vec!["method".into(), "publish".into()],
            vec!["param".into(), "priority".into(), "normal".into()],
            vec!["relays".into(), "wss://relay.example".into()],
        ],
        content: String::new(),
        sig: Some("sig".into()),
    };

    let parsed = parse_dvm_request(&event).unwrap();
    assert_eq!(parsed.scheduled_events.len(), 1);
    assert_eq!(parsed.target_event_id, input_event_id());
    assert_eq!(parsed.method, "publish");
    assert_eq!(
        parsed.params,
        vec![vec![
            "param".to_string(),
            "priority".to_string(),
            "normal".to_string()
        ]]
    );
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
    assert_eq!(parsed.method, DEFAULT_DVM_METHOD);
    assert_eq!(parsed.scheduled_events.len(), 1);
}

#[test]
fn extracts_input_event_id_without_full_parse() {
    let tags = vec![vec![
        "i".to_string(),
        signed_event_json(),
        "event".to_string(),
    ]];
    assert_eq!(
        first_input_event_id_from_tags(&tags).as_deref(),
        Some(input_event_id().as_str())
    );
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
fn rejects_input_event_with_tampered_content() {
    let mut raw = serde_json::from_str::<serde_json::Value>(&signed_event_json()).unwrap();
    raw["content"] = serde_json::Value::String("tampered content".into());
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
        DvmParseError::InvalidInputEventSignature
    );
}

#[test]
fn rejects_input_event_with_wrong_id() {
    let mut raw = serde_json::from_str::<serde_json::Value>(&signed_event_json()).unwrap();
    raw["id"] = serde_json::Value::String("a".repeat(64));
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
        DvmParseError::InvalidInputEventSignature
    );
}

#[test]
fn rejects_input_event_with_invalid_signature() {
    let mut raw = serde_json::from_str::<serde_json::Value>(&signed_event_json()).unwrap();
    raw["sig"] = serde_json::Value::String("b".repeat(128));
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
        DvmParseError::InvalidInputEventSignature
    );
}

#[test]
fn builds_feedback_tags_with_recipient_method_status_and_result() {
    assert_eq!(
        feedback_tags(DvmFeedbackMetadata {
            status: "succeeded",
            request_event_id: "abc",
            recipient_pubkey: &pubkey(),
            method: "publish",
            result: "scheduled",
            message: Some("Scheduled."),
        }),
        vec![
            vec!["e".to_string(), "abc".to_string()],
            vec!["p".to_string(), pubkey().as_str().to_string()],
            vec!["method".to_string(), "publish".to_string()],
            vec!["status".to_string(), "succeeded".to_string()],
            vec!["result".to_string(), "scheduled".to_string()],
            vec!["alt".to_string(), "Scheduled.".to_string()]
        ]
    );
}

#[test]
fn signs_feedback_event() {
    let secret = "1111111111111111111111111111111111111111111111111111111111111111";
    let feedback = build_signed_feedback_event(
        secret,
        DvmFeedbackMetadata {
            status: "succeeded",
            request_event_id: "abc",
            recipient_pubkey: &pubkey(),
            method: "publish",
            result: "scheduled",
            message: Some("Scheduled."),
        },
        1_700_000_000,
    )
    .unwrap();
    assert_eq!(feedback.kind, DVM_FEEDBACK_KIND);
    assert!(feedback.id.is_some());
    assert!(feedback.sig.is_some());
    assert!(feedback
        .tags
        .iter()
        .any(|tag| tag == &vec!["status", "succeeded"]));
    assert_eq!(feedback.pubkey, pubkey_from_secret_hex(secret).unwrap());
}
