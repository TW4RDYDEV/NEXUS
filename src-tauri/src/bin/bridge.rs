use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
fn main() {
    let root = std::env::var_os("NEXUS_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(".nexus/dev"));
    let mut api = nexus_core::core::Api::new(root).expect("Local data directory must be writable");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) => match serde_json::from_str::<Value>(&line) {
                Ok(v) => match api.dispatch(nexus_core::models::s(&v, "op"), v["args"].clone()) {
                    Ok(value) => json!({"id":v["id"],"result":value}),
                    Err(error) => json!({"id":v["id"],"error":error}),
                },
                Err(_) => json!({"error":"Invalid request JSON"}),
            },
            Err(_) => break,
        };
        if writeln!(stdout, "{response}")
            .and_then(|_| stdout.flush())
            .is_err()
        {
            break;
        }
    }
}
