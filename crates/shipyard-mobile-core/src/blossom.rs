use serde::{Deserialize, Serialize};
use shipyard_core::NostrEvent;

pub const BLOSSOM_SERVER_LIST_KIND: u64 = 10_063;
pub const DEFAULT_BLOSSOM_SERVER: &str = "https://blossom.primal.net";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlossomServerSelection {
    pub server: String,
    pub used_default: bool,
}

pub fn select_blossom_server(servers: &[String]) -> BlossomServerSelection {
    match servers.iter().find_map(|server| normalize_server(server)) {
        Some(server) => BlossomServerSelection {
            server,
            used_default: false,
        },
        None => BlossomServerSelection {
            server: DEFAULT_BLOSSOM_SERVER.to_string(),
            used_default: true,
        },
    }
}

pub fn select_blossom_server_from_event(event: &NostrEvent) -> BlossomServerSelection {
    let servers = blossom_servers_from_event(event);
    select_blossom_server(&servers)
}

pub fn blossom_servers_from_event(event: &NostrEvent) -> Vec<String> {
    if event.kind != BLOSSOM_SERVER_LIST_KIND {
        return Vec::new();
    }

    event
        .tags
        .iter()
        .filter_map(|tag| match (tag.first(), tag.get(1)) {
            (Some(marker), Some(url)) if marker == "server" => Some(url.clone()),
            _ => None,
        })
        .collect()
}

fn normalize_server(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    let valid_scheme = trimmed.starts_with("https://") || trimmed.starts_with("http://");
    let has_host = trimmed
        .split_once("://")
        .is_some_and(|(_, rest)| !rest.is_empty() && !rest.contains(char::is_whitespace));

    if valid_scheme && has_host {
        Some(trimmed.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_core::Pubkey;

    fn event(tags: Vec<Vec<String>>) -> NostrEvent {
        NostrEvent::unsigned(
            Pubkey::parse("a".repeat(64)).unwrap(),
            1_776_000_000,
            BLOSSOM_SERVER_LIST_KIND,
            tags,
            String::new(),
        )
    }

    #[test]
    fn selects_first_valid_server_without_defaulting() {
        let selected = select_blossom_server(&[
            "not a url".to_string(),
            " https://media.example/ ".to_string(),
            "https://backup.example".to_string(),
        ]);

        assert_eq!(selected.server, "https://media.example");
        assert!(!selected.used_default);
    }

    #[test]
    fn defaults_only_when_no_valid_server_exists() {
        let selected = select_blossom_server(&["".to_string(), "ftp://example".to_string()]);

        assert_eq!(selected.server, DEFAULT_BLOSSOM_SERVER);
        assert!(selected.used_default);
    }

    #[test]
    fn reads_kind_10063_server_tags() {
        let selected = select_blossom_server_from_event(&event(vec![
            vec!["relay".to_string(), "wss://relay.example".to_string()],
            vec!["server".to_string(), "https://blossom.example/".to_string()],
        ]));

        assert_eq!(selected.server, "https://blossom.example");
        assert!(!selected.used_default);
    }
}
