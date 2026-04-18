use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum DvmCommand {
    /// List DVM requests for the authenticated owner
    Requests,
}

pub(crate) async fn run(client: &ApiClient, command: DvmCommand) -> anyhow::Result<Value> {
    match command {
        DvmCommand::Requests => {
            client
                .request(Method::GET, "/v1/dvm/requests", None, true, true)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::DvmCommand;

    #[derive(Debug, Parser)]
    struct Harness {
        #[command(subcommand)]
        command: DvmCommand,
    }

    #[test]
    fn dvm_requests_variant_exists() {
        let cmd = DvmCommand::Requests;
        assert!(matches!(cmd, DvmCommand::Requests));
    }

    #[test]
    fn dvm_schedule_is_not_accepted() {
        let result =
            Harness::try_parse_from(["shipyard", "schedule", "--event-json", "/tmp/event.json"]);
        assert!(result.is_err());
    }

    #[test]
    fn dvm_status_is_not_accepted() {
        let result =
            Harness::try_parse_from(["shipyard", "status", "550e8400-e29b-41d4-a716-446655440000"]);
        assert!(result.is_err());
    }
}
