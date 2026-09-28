use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

pub const SUPPORTED: [&str; 8] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
    "Interrupt",
    "SessionEnd",
];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event: String,
    pub session: String,
    #[serde(default)]
    pub turn: String,
    #[serde(default)]
    pub call: String,
    pub at: u64,
}
impl TaskEvent {
    pub fn from_hook(input: &serde_json::Value, at: u64) -> Option<Self> {
        let event = input.get("hook_event_name")?.as_str()?;
        if !SUPPORTED.contains(&event) {
            return None;
        }
        let session = input.get("session_id")?.as_str()?;
        if session.is_empty() {
            return None;
        }
        let bounded = |s: &str| s.chars().take(160).collect::<String>();
        Some(Self {
            event: event.into(),
            session: bounded(session),
            turn: bounded(input["turn_id"].as_str().unwrap_or("")),
            call: bounded(input["tool_use_id"].as_str().unwrap_or("")),
            at,
        })
    }
}
pub fn private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut f = options.open(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
pub fn write_event(dir: &Path, event: &TaskEvent) -> Result<(), String> {
    let folder = dir.join("events");
    private_dir(&folder)?;
    atomic_write(
        &folder.join(format!("{}-{}.json", event.at, Uuid::new_v4())),
        &serde_json::to_vec(event).map_err(|e| e.to_string())?,
    )
}
pub fn consume_events(dir: &Path) -> Result<Vec<TaskEvent>, String> {
    let mut files = fs::read_dir(dir.join("events"))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|f| {
            f.file_type().map(|t| t.is_file()).unwrap_or(false)
                && f.path().extension().is_some_and(|s| s == "json")
        })
        .map(|f| f.path())
        .collect::<Vec<_>>();
    files.sort();
    let mut events = Vec::new();
    for file in files.into_iter().take(500) {
        // Atomic rename by writers prevents observing partially written JSON.
        if fs::metadata(&file).map(|m| m.len()).unwrap_or(0) <= 4096 {
            if let Ok(bytes) = fs::read(&file) {
                if let Ok(event) = serde_json::from_slice(&bytes) {
                    events.push(event);
                }
            }
        }
        fs::remove_file(file).map_err(|e| e.to_string())?;
    }
    events.sort_by_key(|e: &TaskEvent| e.at);
    Ok(events)
}
