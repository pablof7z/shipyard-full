mod client;
mod commands;
mod config;
mod output;

use clap::Parser;
use commands::{run, Cli};
use output::print_output;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json_output = cli.json;
    match run(cli).await {
        Ok(output) => print_output(json_output, &output),
        Err(error) => {
            if json_output {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "code": "cli_error",
                        "message": error.to_string()
                    }))
                    .unwrap_or_else(|_| "{\"code\":\"cli_error\"}".to_string())
                );
            } else {
                eprintln!("Shipyard CLI: {error}");
            }
            std::process::exit(1);
        }
    }
}
