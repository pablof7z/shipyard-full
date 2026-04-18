use serde::{Deserialize, Serialize};
use shipyard_core::{NostrEvent, Pubkey};

pub const NIP37_DRAFT_KIND: u64 = 31_234;
pub const NIP37_PRIVATE_RELAY_LIST_KIND: u64 = 10_013;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nip37DraftMetadata {
    pub identifier: String,
    pub event_kind: u64,
    pub relay_hints: Vec<String>,
    pub subject: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nip37DraftDeleteMarker {
    pub metadata: Nip37DraftMetadata,
}

impl Nip37DraftMetadata {
    pub fn tags(&self) -> Vec<Vec<String>> {
        let mut tags = vec![
            vec!["d".to_string(), self.identifier.clone()],
            vec!["k".to_string(), self.event_kind.to_string()],
        ];

        tags.extend(
            self.relay_hints
                .iter()
                .map(|relay| vec!["relay".to_string(), relay.clone()]),
        );

        if let Some(subject) = &self.subject {
            tags.push(vec!["subject".to_string(), subject.clone()]);
        }

        tags
    }

    pub fn from_tags(tags: &[Vec<String>]) -> Option<Self> {
        let identifier = tag_value(tags, "d")?.to_string();
        let event_kind = tag_value(tags, "k")?.parse().ok()?;
        let relay_hints = tags
            .iter()
            .filter_map(|tag| match (tag.first(), tag.get(1)) {
                (Some(marker), Some(value)) if marker == "relay" => Some(value.clone()),
                _ => None,
            })
            .collect();
        let subject = tag_value(tags, "subject").map(str::to_string);

        Some(Self {
            identifier,
            event_kind,
            relay_hints,
            subject,
        })
    }
}

impl Nip37DraftDeleteMarker {
    pub fn to_unsigned_event(&self, pubkey: Pubkey, created_at: i64) -> NostrEvent {
        NostrEvent::unsigned(
            pubkey,
            created_at,
            NIP37_DRAFT_KIND,
            self.metadata.tags(),
            String::new(),
        )
    }
}

pub fn draft_wrap_event(
    pubkey: Pubkey,
    created_at: i64,
    metadata: &Nip37DraftMetadata,
    encrypted_content: String,
) -> NostrEvent {
    NostrEvent::unsigned(
        pubkey,
        created_at,
        NIP37_DRAFT_KIND,
        metadata.tags(),
        encrypted_content,
    )
}

pub fn is_draft_delete_marker(event: &NostrEvent) -> bool {
    event.kind == NIP37_DRAFT_KIND
        && event.content.is_empty()
        && Nip37DraftMetadata::from_tags(&event.tags).is_some()
}

fn tag_value<'a>(tags: &'a [Vec<String>], marker: &str) -> Option<&'a str> {
    tags.iter().find_map(|tag| match (tag.first(), tag.get(1)) {
        (Some(candidate), Some(value)) if candidate == marker => Some(value.as_str()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Nip37DraftMetadata {
        Nip37DraftMetadata {
            identifier: "draft-1".to_string(),
            event_kind: 1,
            relay_hints: vec!["wss://relay.example".to_string()],
            subject: Some("Launch note".to_string()),
        }
    }

    fn pubkey() -> Pubkey {
        Pubkey::parse("a".repeat(64)).unwrap()
    }

    #[test]
    fn round_trips_draft_metadata_tags() {
        let metadata = metadata();

        assert_eq!(
            Nip37DraftMetadata::from_tags(&metadata.tags()).unwrap(),
            metadata
        );
    }

    #[test]
    fn builds_draft_wrap_and_delete_marker_events() {
        let metadata = metadata();
        let wrap = draft_wrap_event(pubkey(), 1_776_000_000, &metadata, "encrypted".to_string());
        assert_eq!(wrap.kind, NIP37_DRAFT_KIND);
        assert!(!is_draft_delete_marker(&wrap));

        let marker = Nip37DraftDeleteMarker { metadata }.to_unsigned_event(pubkey(), 1_776_000_001);
        assert!(is_draft_delete_marker(&marker));
    }
}
