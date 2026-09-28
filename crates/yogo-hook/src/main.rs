use std::io::{self, Read};
fn main() {
    // Fail open: hook failures must never stop or modify the Codex task.
    let _ = run();
    println!("{{}}");
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    io::stdin().take(2_000_001).read_to_end(&mut bytes)?;
    if bytes.len() > 2_000_000 {
        return Ok(());
    }
    let input: serde_json::Value = serde_json::from_slice(&bytes)?;
    if let Some(event) = yogo_core::events::TaskEvent::from_hook(&input, yogo_core::now_ms()) {
        let _ = yogo_core::events::write_event(&yogo_core::config_store::data_dir(), &event);
    }
    Ok(())
}
