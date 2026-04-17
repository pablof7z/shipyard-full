use std::path::PathBuf;

use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use uuid::Uuid;

use crate::client::ApiClient;

use super::read_json_file;

#[derive(Debug, Subcommand)]
pub(crate) enum DvmCommand {
    /// List DVM requests for the authenticated owner
    Requests,
    /// Submit a signed Nostr event for DVM-based scheduling
    Schedule {
        /// Path to a signed Nostr event JSON file
        #[arg(long)]
        event_json: PathBuf,
    },
    /// Show the status of a specific DVM request
    Status {
        /// DVM request UUID
        id: Uuid,
    },
}

pub(crate) async fn run(client: &ApiClient, command: DvmCommand) -> anyhow::Result<Value> {
    match command {
        DvmCommand::Requests => {
            client
                .request(Method::GET, "/v1/dvm/requests", None, true, true)
                .await
        }
        DvmCommand::Schedule { event_json } => {
            let signed_event = read_json_file(event_json)?;
            client
                .request(
                    Method::POST,
                    "/v1/dvm/schedule",
                    Some(serde_json::json!({ "signed_event": signed_event })),
                    true,
                    false,
                )
                .await
        }
        DvmCommand::Status { id } => {
            client
                .request(
                    Method::GET,
                    &format!("/v1/dvm/requests/{id}"),
                    None,
                    true,
                    true,
                )
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::DvmCommand;

    #[test]
    fn dvm_requests_variant_exists() {
        let cmd = DvmCommand::Requests;
        assert!(matches!(cmd, DvmCommand::Requests));
    }

    #[test]
    fn dvm_schedule_variant_holds_event_json_path() {
        let path = PathBuf::from("/tmp/event.json");
        let cmd = DvmCommand::Schedule {
            event_json: path.clone(),
        };
        match cmd {
            DvmCommand::Schedule { event_json } => assert_eq!(event_json, path),
            _ => panic!("expected Schedule variant"),
        }
    }

    #[test]
    fn dvm_status_variant_holds_uuid() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let cmd = DvmCommand::Status { id };
        match cmd {
            DvmCommand::Status { id: stored_id } => assert_eq!(stored_id, id),
            _ => panic!("expected Status variant"),
        }
    }

    #[test]
    fn dvm_schedule_and_status_are_distinct_from_requests() {
        // Ensure none of the variants accidentally collapse
        let requests = DvmCommand::Requests;
        let schedule = DvmCommand::Schedule {
            event_json: PathBuf::from("/tmp/e.json"),
        };
        let status = DvmCommand::Status { id: Uuid::new_v4() };
        assert!(!matches!(requests, DvmCommand::Schedule { .. }));
        assert!(!matches!(requests, DvmCommand::Status { .. }));
        assert!(!matches!(schedule, DvmCommand::Requests));
        assert!(!matches!(schedule, DvmCommand::Status { .. }));
        assert!(!matches!(status, DvmCommand::Requests));
        assert!(!matches!(status, DvmCommand::Schedule { .. }));
    }
}
