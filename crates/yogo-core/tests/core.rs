use serde_json::json;
use yogo_core::{
    art::frame_for,
    config_store::{ConfigStore, Preferences},
    events::{consume_events, write_event, TaskEvent},
    protocol::{normalize_reply, packet, restore_dot},
    service_runtime::ServiceRuntime,
    task_states::{PetState, TaskStates},
};
fn event(name: &str, session: &str, turn: &str, call: &str, at: u64) -> TaskEvent {
    TaskEvent {
        event: name.into(),
        session: session.into(),
        turn: turn.into(),
        call: call.into(),
        at,
    }
}
fn temp() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("yogo-test-{}", uuid::Uuid::new_v4()))
}
#[test]
fn native_protocol_matches_hardware_verified_js() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!("fixtures.json")).unwrap();
    for state in [
        PetState::Idle,
        PetState::Thinking,
        PetState::Working,
        PetState::Waiting,
        PetState::Done,
        PetState::Interrupted,
    ] {
        let name = serde_json::to_value(state)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let frame = frame_for(state);
        assert_eq!(
            serde_json::to_value(frame).unwrap(),
            fixtures[&name]["frame"]
        );
        let flat = frame
            .iter()
            .flatten()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        for (i, offset) in (0..108).step_by(24).enumerate() {
            let p = packet(
                0x3b,
                offset as u16,
                24,
                &flat[offset..(offset + 24).min(108)],
                (i + 1) as u16,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(p.to_vec()).unwrap(),
                fixtures[&name]["packets"][i]
            );
        }
    }
    assert!(packet(1, 0, 25, &[], 1).is_err());
    assert!(packet(1, 0, 0, &[], 0).is_err());
    assert_eq!(normalize_reply(&[0, 0xaa, 1]), [0xaa, 1]);
}
#[test]
fn restore_preserves_every_non_dot_byte() {
    let mut current = [99; 24];
    let original = std::array::from_fn(|i| i as u8);
    restore_dot(&mut current, &original);
    for i in 0..24 {
        assert_eq!(
            current[i],
            if (5..14).contains(&i) {
                original[i]
            } else {
                99
            }
        );
    }
}
#[test]
fn parallel_calls_remain_working_and_completed_turn_cannot_resurrect() {
    let mut tasks = TaskStates::default();
    let now = 10_000;
    for e in [
        event("UserPromptSubmit", "a", "t", "", now),
        event("PreToolUse", "a", "t", "1", now + 1),
        event("PreToolUse", "a", "t", "2", now + 2),
        event("PostToolUse", "a", "t", "1", now + 3),
    ] {
        assert!(tasks.apply(e, now + 10));
    }
    assert_eq!(tasks.snapshot(now + 10, 8).state, PetState::Working);
    tasks.apply(event("Stop", "a", "t", "", now + 11), now + 11);
    assert!(!tasks.apply(event("PostToolUse", "a", "t", "2", now + 12), now + 12));
    assert_eq!(tasks.snapshot(now + 13, 8).state, PetState::Done);
    assert_eq!(tasks.snapshot(now + 9000, 8).state, PetState::Idle);
}
#[test]
fn stale_turns_and_sessions_are_ignored_and_multi_task_priority_is_preserved() {
    let mut t = TaskStates::default();
    t.apply(event("UserPromptSubmit", "a", "new", "", 10000), 10000);
    assert!(!t.apply(event("Stop", "a", "old", "", 10001), 10001));
    t.apply(event("PermissionRequest", "a", "new", "tool", 10002), 10002);
    t.apply(event("PreToolUse", "b", "t", "1", 10003), 10003);
    assert_eq!(t.snapshot(10004, 8).state, PetState::Waiting);
    assert_eq!(t.snapshot(10004, 8).active_tasks, 2);
    t.apply(event("Stop", "b", "t", "", 10005), 10005);
    assert_eq!(t.snapshot(10006, 8).state, PetState::Waiting);
    assert!(!t.apply(event("Stop", "x", "t", "", 0), 100_000_000));
    assert_eq!(t.snapshot(100_000_000, 8).active_tasks, 0);
}
#[test]
fn hook_spool_is_private_atomic_and_metadata_only() {
    let dir = temp();
    let store = ConfigStore::open(dir.clone()).unwrap();
    let input = json!({"hook_event_name":"PreToolUse","session_id":"test","turn_id":"turn","tool_use_id":"call","prompt":"NEVER_PERSIST","tool_input":{"password":"NEVER_PERSIST"}});
    let e = TaskEvent::from_hook(&input, 100).unwrap();
    let threads = (0..12)
        .map(|_| {
            let dir = dir.clone();
            let e = e.clone();
            std::thread::spawn(move || write_event(&dir, &e).unwrap())
        })
        .collect::<Vec<_>>();
    for t in threads {
        t.join().unwrap();
    }
    for entry in std::fs::read_dir(dir.join("events")).unwrap() {
        let path = entry.unwrap().path();
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("NEVER_PERSIST"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    assert_eq!(consume_events(&store.dir).unwrap().len(), 12);
    assert!(consume_events(&store.dir).unwrap().is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn runtime_consumes_events_persists_settings_and_enforces_single_owner() {
    let dir = temp();
    let runtime = ServiceRuntime::spawn(dir.clone(), |_| {}).unwrap();
    assert!(ServiceRuntime::spawn(dir.clone(), |_| {}).is_err());
    let mut preferences = Preferences::default();
    preferences.completed_seconds = 10;
    runtime.save(preferences.clone()).unwrap();
    assert_eq!(
        ConfigStore::open(dir.clone()).unwrap().load().unwrap(),
        preferences
    );
    preferences.completed_seconds = 0;
    assert!(runtime.save(preferences).is_err());
    assert_eq!(runtime.snapshot().phase, "stopped");
    write_event(
        &dir,
        &event("UserPromptSubmit", "runtime", "t", "", yogo_core::now_ms()),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while runtime.snapshot().tasks.active_tasks == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(runtime.snapshot().tasks.active_tasks, 1);
    assert_eq!(runtime.snapshot().state, PetState::Thinking);
    runtime.shutdown().unwrap();
    drop(runtime);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn other_tool_completion_does_not_clear_pending_permission() {
    let mut t = TaskStates::default();
    for e in [
        event("PreToolUse", "a", "t", "1", 1),
        event("PermissionRequest", "a", "t", "2", 2),
        event("PostToolUse", "a", "t", "1", 3),
    ] {
        t.apply(e, 10);
    }
    assert_eq!(t.snapshot(10, 8).state, PetState::Waiting);
    t.apply(event("PreToolUse", "a", "t", "2", 11), 11);
    assert_eq!(t.snapshot(11, 8).state, PetState::Working);
}
