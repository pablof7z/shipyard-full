use serde_json::Value;

pub(crate) fn print_output(json_output: bool, output: &Value) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
        );
    } else if let Some(status) = output.get("status").and_then(Value::as_str) {
        println!("Shipyard CLI: {status}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
        );
    }
}
