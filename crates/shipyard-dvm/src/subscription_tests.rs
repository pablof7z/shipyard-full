use super::*;
use shipyard_core::Pubkey;

fn request_event() -> DvmRequestEvent {
    DvmRequestEvent {
        id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        pubkey: Pubkey::parse("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .unwrap(),
        created_at: 1_776_432_000,
        kind: DVM_SCHEDULE_KIND,
        tags: vec![],
        content: String::new(),
        sig: Some("sig".into()),
    }
}

#[test]
fn parses_relay_event_for_matching_subscription() {
    let raw = serde_json::json!(["EVENT", "sub", request_event()]).to_string();
    let parsed = parse_relay_event_message(&raw, "sub").unwrap().unwrap();
    assert_eq!(parsed.kind, DVM_SCHEDULE_KIND);
}

#[test]
fn ignores_other_subscription_ids() {
    let raw = serde_json::json!(["EVENT", "other", request_event()]).to_string();
    assert!(parse_relay_event_message(&raw, "sub").unwrap().is_none());
}

#[test]
fn parses_nip65_kind_5905_relay_list_message() {
    let raw = serde_json::json!([
        "EVENT",
        "nip65",
        NostrEvent {
            id: Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()),
            pubkey: Pubkey::parse(
                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            )
            .unwrap(),
            created_at: 1_776_432_000,
            kind: NIP65_RELAY_LIST_KIND,
            tags: vec![
                vec!["k".into(), "5905".into()],
                vec!["r".into(), "wss://relay-a.example".into(), "read".into()],
            ],
            content: String::new(),
            sig: Some("sig".into()),
        }
    ])
    .to_string();

    assert_eq!(
        parse_nip65_relay_list_message(&raw, "nip65").unwrap(),
        vec!["wss://relay-a.example".to_string()]
    );
}

#[test]
fn parses_nip65_kind_5905_relay_list_event() {
    let event = NostrEvent {
        id: Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()),
        pubkey: Pubkey::parse("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
            .unwrap(),
        created_at: 1_776_432_000,
        kind: NIP65_RELAY_LIST_KIND,
        tags: vec![
            vec!["k".into(), "5905".into()],
            vec!["r".into(), "wss://relay-a.example".into(), "read".into()],
            vec!["r".into(), "ws://relay-b.example".into()],
            vec!["r".into(), "https://not-a-relay.example".into()],
        ],
        content: String::new(),
        sig: Some("sig".into()),
    };

    assert_eq!(
        parse_nip65_relay_list_event(&event),
        vec![
            "wss://relay-a.example".to_string(),
            "ws://relay-b.example".to_string()
        ]
    );
}

#[test]
fn ignores_nip65_relay_lists_without_kind_5905_tag() {
    let event = NostrEvent {
        id: Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()),
        pubkey: Pubkey::parse("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
            .unwrap(),
        created_at: 1_776_432_000,
        kind: NIP65_RELAY_LIST_KIND,
        tags: vec![
            vec!["k".into(), "1".into()],
            vec!["r".into(), "wss://relay-a.example".into()],
        ],
        content: String::new(),
        sig: Some("sig".into()),
    };

    assert!(parse_nip65_relay_list_event(&event).is_empty());
}
