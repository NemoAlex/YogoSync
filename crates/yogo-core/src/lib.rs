pub mod art;
pub mod config_store;
pub mod device_service;
pub mod events;
pub mod protocol;
pub mod service_runtime;
pub mod task_states;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub mod themes;
