use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn actual_hook_binary_is_fail_open_and_filters_sensitive_fields() {
    let dir = std::env::temp_dir().join(format!("yogo-hook-test-{}", std::process::id()));
    let input = r#"{"hook_event_name":"PreToolUse","session_id":"test","tool_use_id":"call","tool_input":{"secret":"SHOULD_NOT_APPEAR"},"prompt":"SHOULD_NOT_APPEAR"}"#;
    for raw in [input, "not json"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_yogosync-hook"))
            .env("YOGOSYNC_HOME", &dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(raw.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"{}\n");
    }
    let files = std::fs::read_dir(dir.join("events"))
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    let text = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert!(!text.contains("SHOULD_NOT_APPEAR"));
    let event: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(event["event"], "PreToolUse");
    assert_eq!(event.as_object().unwrap().len(), 5);
    std::fs::remove_dir_all(dir).unwrap();
}
