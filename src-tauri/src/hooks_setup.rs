//! Install user Hooks without changing trust or unrelated configuration.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Manager;
static SETUP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn config_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(|p| PathBuf::from(p).join(".codex"))
        .ok_or("无法定位用户目录".into())
}
fn binary_name() -> &'static str {
    if cfg!(windows) {
        "yogosync-hook.exe"
    } else {
        "yogosync-hook"
    }
}
fn command(dir: &Path) -> String {
    let path = dir.join("yogosync/bin").join(binary_name());
    if cfg!(windows) {
        format!("\"{}\"", path.display())
    } else {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
    }
}
pub fn owns_hook(hook: &Value) -> bool {
    hook["pluginId"].is_null()
        && config_dir().is_ok_and(|dir| {
            hook["command"] == command(&dir)
                && hook["sourcePath"].as_str().is_some_and(|source| {
                    let expected = dir.join("hooks.json");
                    Path::new(source) == expected
                        || std::fs::canonicalize(source)
                            .ok()
                            .zip(std::fs::canonicalize(expected).ok())
                            .is_some_and(|(a, b)| a == b)
                })
        })
}
fn merge(mut config: Value, cmd: &str) -> Result<Value, String> {
    let object = config.as_object_mut().ok_or("Hooks 配置必须是 JSON 对象")?;
    let hooks = object
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("hooks 必须是对象")?;
    // Validate before touching any file; preserve other handlers even in shared groups.
    for groups in hooks.values_mut() {
        for group in groups.as_array_mut().ok_or("事件配置必须是数组")? {
            let handlers = group["hooks"].as_array_mut().ok_or("事件缺少 hooks 数组")?;
            handlers.retain(|h| h["command"] != cmd);
        }
        groups
            .as_array_mut()
            .unwrap()
            .retain(|g| !g["hooks"].as_array().unwrap().is_empty());
    }
    let template: Value = serde_json::from_str(include_str!("../resources/hooks.json")).unwrap();
    for (event, groups) in template["hooks"].as_object().unwrap() {
        let mut groups = groups.clone();
        for group in groups.as_array_mut().unwrap() {
            for handler in group["hooks"].as_array_mut().unwrap() {
                handler["command"] = json!(cmd);
                handler.as_object_mut().unwrap().remove("commandWindows");
            }
        }
        hooks
            .entry(event.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .extend(groups.as_array().unwrap().clone());
    }
    Ok(config)
}
fn prepare(dir: &Path, resource: &Path) -> Result<(), String> {
    let path = dir.join("hooks.json");
    let original = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let config: Value = original
        .as_ref()
        .map(|b| serde_json::from_slice(b))
        .transpose()
        .map_err(|e| format!("Hooks 配置格式有误，未修改：{e}"))?
        .unwrap_or_else(|| json!({}));
    let merged = merge(config.clone(), &command(dir))?;
    let bin = dir.join("yogosync/bin");
    yogo_core::events::private_dir(&bin)?;
    let temp_binary = bin.join(format!(".helper-{}", uuid::Uuid::new_v4()));
    std::fs::copy(resource.join("resources").join(binary_name()), &temp_binary)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_binary, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(temp_binary, bin.join(binary_name())).map_err(|e| e.to_string())?;
    if merged == config {
        return Ok(());
    }
    if std::fs::read(&path).ok() != original {
        return Err("Hooks 配置已变更，请重试。".into());
    }
    if let Some(bytes) = &original {
        std::fs::write(
            dir.join(format!(
                "hooks.yogosync-backup-{}.json",
                uuid::Uuid::new_v4()
            )),
            bytes,
        )
        .map_err(|e| e.to_string())?;
    }
    let temporary = dir.join(format!(".hooks-{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(&merged).unwrap())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn configure_codex_hooks(
    app: tauri::AppHandle,
    probe: tauri::State<'_, std::sync::Arc<super::codex_connection::ConnectionProbe>>,
) -> Result<(), String> {
    let resource = app.path().resource_dir().map_err(|e| e.to_string())?;
    let probe = probe.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SETUP_LOCK.lock().map_err(|_| "配置暂不可用，请重启应用")?;
        let state = probe.check_fresh();
        if !matches!(
            state["state"].as_str(),
            Some(
                "missing" | "incomplete" | "authorized" | "unauthorized" | "modified" | "disabled"
            )
        ) {
            return Err(state["detail"]
                .as_str()
                .unwrap_or("请先检查连接状态")
                .to_string());
        }
        prepare(&config_dir()?, &resource)?;
        probe.invalidate();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_preserves_other_handlers_and_is_repeatable() {
        let original = json!({"description":"mine", "hooks":{"Stop":[{"matcher":"*","hooks":[{"command":"other"},{"command":"owned"}]}]}});
        let merged = merge(original, "owned").unwrap();
        assert_eq!(merged["description"], "mine");
        assert_eq!(
            merged["hooks"]["Stop"][0]["hooks"],
            json!([{"command":"other"}])
        );
        assert_eq!(merge(merged.clone(), "owned").unwrap(), merged);
        assert_eq!(merged["hooks"].as_object().unwrap().len(), 8);
        assert!(merge(json!({"hooks":[]}), "owned").is_err());
    }
    #[test]
    fn prepare_preserves_invalid_file_and_backs_up_valid_file() {
        let root = std::env::temp_dir().join(format!("yogosync-hooks-{}", uuid::Uuid::new_v4()));
        let resources = root.join("resources");
        std::fs::create_dir_all(&resources).unwrap();
        std::fs::write(resources.join(binary_name()), "fixture").unwrap();
        let path = root.join("hooks.json");
        std::fs::write(&path, "invalid").unwrap();
        assert!(prepare(&root, &root).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid");
        std::fs::write(&path, "{}").unwrap();
        prepare(&root, &root).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        prepare(&root, &root).unwrap();
        assert_eq!(bytes, std::fs::read(&path).unwrap());
        assert_eq!(
            std::fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e
                    .file_name()
                    .to_string_lossy()
                    .starts_with("hooks.yogosync-backup"))
                .count(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
pub fn test_command() -> String {
    command(&config_dir().unwrap())
}
