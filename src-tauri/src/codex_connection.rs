//! Read-only Codex discovery. Trust is always reviewed in Codex itself.
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct ConnectionProbe(Mutex<Option<(Instant, Value)>>);

fn status(state: &str, detail: &str) -> Value {
    json!({"state":state,"detail":detail})
}

fn cli_path() -> Option<PathBuf> {
    let mut candidates = vec![];
    #[cfg(target_os = "macos")]
    {
        for root in [
            Some(PathBuf::from("/Applications")),
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications")),
        ]
        .into_iter()
        .flatten()
        {
            for app in ["ChatGPT.app", "Codex.app"] {
                // Prefer the CLI bundled with the client that owns Hook review.
                // Recent desktop builds moved it under codex-cli/bin.
                for relative in [
                    "Contents/Resources/codex-cli/bin/codex",
                    "Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
                    "Contents/Resources/codex",
                ] {
                    candidates.push(root.join(app).join(relative));
                }
            }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(
            std::env::split_paths(&path)
                .map(|dir| dir.join(if cfg!(windows) { "codex.exe" } else { "codex" })),
        );
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn classify(response: &Value) -> Value {
    let Some(entries) = response.pointer("/result/data").and_then(Value::as_array) else {
        return status(
            "error",
            "无法读取 Codex 授权状态，请确认 Codex 已更新后重试。",
        );
    };
    if entries.is_empty()
        || entries
            .iter()
            .any(|e| e["errors"].as_array().is_some_and(|v| !v.is_empty()))
    {
        return status(
            "error",
            "Codex 插件配置读取失败，请在 Codex 设置中检查 Hooks。",
        );
    }
    let hooks: Vec<_> = entries
        .iter()
        .filter_map(|e| e["hooks"].as_array())
        .flatten()
        .filter(|h| {
            h["pluginId"]
                .as_str()
                .is_some_and(|id| id.split('@').next() == Some("yogo-pet"))
        })
        .collect();
    if hooks.is_empty() {
        return status(
            "missing",
            "未检测到启用的 YOGO Pet 插件，请先在 Codex 中安装并启用。",
        );
    }
    if hooks.iter().any(|h| h["trustStatus"] == "modified") {
        return status("modified", "插件事件定义已变更，请重新审阅并授权。");
    }
    if hooks
        .iter()
        .any(|h| matches!(h["trustStatus"].as_str(), Some("untrusted")))
    {
        return status(
            "unauthorized",
            "在 Codex 设置 → Hooks 中，找到 YOGO Pet，审阅并信任待授权的事件。",
        );
    }
    if hooks.iter().any(|h| h["enabled"] != true) {
        return status(
            "disabled",
            "YOGO Pet 的部分 Hooks 已停用，请在 Codex 设置 → Hooks 中启用。",
        );
    }
    let expected = [
        "sessionStart",
        "userPromptSubmit",
        "preToolUse",
        "postToolUse",
        "permissionRequest",
        "stop",
        "interrupt",
        "sessionEnd",
    ];
    if expected
        .iter()
        .any(|event| !hooks.iter().any(|h| h["eventName"] == *event))
        || hooks
            .iter()
            .any(|h| !matches!(h["trustStatus"].as_str(), Some("trusted" | "managed")))
    {
        return status(
            "error",
            "YOGO Pet 的事件配置不完整或版本不兼容，请更新插件。",
        );
    }
    status(
        "authorized",
        "已授权。继续任意 Codex 任务即可验证事件连接；若仍无事件，请重启 Codex 后继续原任务。",
    )
}

fn classify_installation(response: &Value) -> Value {
    let Some(markets) = response
        .pointer("/result/marketplaces")
        .and_then(Value::as_array)
    else {
        return status("error", "无法检测插件安装状态，请更新 Codex 后重试。");
    };
    if response
        .pointer("/result/marketplaceLoadErrors")
        .and_then(Value::as_array)
        .is_some_and(|v| !v.is_empty())
    {
        return status("error", "Codex 插件市场读取失败，请检查 Codex 设置。");
    }
    let plugins: Vec<_> = markets
        .iter()
        .filter_map(|m| m["plugins"].as_array())
        .flatten()
        .filter(|p| p["name"] == "yogo-pet" && p["installed"] == true)
        .collect();
    if plugins.iter().any(|p| p["enabled"] == true) {
        status(
            "error",
            "插件已启用，但未加载任务事件。请检查 Codex 的 Hooks 设置或重启 Codex。",
        )
    } else if !plugins.is_empty() {
        status(
            "plugin_disabled",
            "YOGO Pet 插件已安装，请在 Codex 插件页启用。",
        )
    } else {
        status("missing", "安装 YOGO Pet 插件以接收任务状态。")
    }
}

fn query(cli: PathBuf) -> Result<Value, String> {
    let mut command = Command::new(cli);
    command
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法检测 Codex：{e}"))?;
    let mut input = child.stdin.take().ok_or("无法打开 Codex 查询通道")?;
    let output = child.stdout.take().ok_or("无法读取 Codex 查询结果")?;
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                if tx.send(value).is_err() {
                    break;
                }
            }
        }
    });
    let result = (|| {
        writeln!(input, "{}", json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"yogo-pet-status","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}})).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let response = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| "Codex 检测超时或连接中断，请重试。".to_string())?;
            match response["id"].as_u64() {
                Some(1) => {
                    if response.get("error").is_some() {
                        return Err("当前 Codex 版本不支持连接检测，请更新 Codex。".into());
                    }
                    writeln!(input, "{}", json!({"method":"initialized"}))
                        .map_err(|e| e.to_string())?;
                    writeln!(
                        input,
                        "{}",
                        json!({"id":2,"method":"hooks/list","params":{"cwds":[]}})
                    )
                    .map_err(|e| e.to_string())?;
                }
                Some(2) => {
                    let result = classify(&response);
                    if result["state"] != "missing" {
                        return Ok(result);
                    }
                    writeln!(
                        input,
                        "{}",
                        json!({"id":3,"method":"plugin/installed","params":{}})
                    )
                    .map_err(|e| e.to_string())?;
                }
                Some(3) => return Ok(classify_installation(&response)),
                _ => {}
            }
        }
    })();
    drop(input);
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    result
}

impl ConnectionProbe {
    pub fn check(&self) -> Value {
        let Ok(mut cache) = self.0.lock() else {
            return status("error", "连接检测暂不可用，请重启 YOGO Pet。");
        };
        if let Some((at, value)) = &*cache {
            if at.elapsed() < Duration::from_secs(5) {
                return value.clone();
            }
        }
        let value = match cli_path() {
            None => status(
                "unavailable",
                "未找到 Codex，请安装 Codex 桌面版，或将 Codex CLI 加入 PATH。",
            ),
            Some(cli) => query(cli).unwrap_or_else(|e| status("error", &e)),
        };
        *cache = Some((Instant::now(), value.clone()));
        value
    }
}

#[tauri::command]
pub async fn get_codex_connection(
    probe: tauri::State<'_, std::sync::Arc<ConnectionProbe>>,
) -> Result<Value, String> {
    let probe = probe.inner().clone();
    tauri::async_runtime::spawn_blocking(move || probe.check())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_codex_authorization() -> Result<(), String> {
    // Codex currently supports opening settings, but not a Hooks-specific deep link.
    // Never write hooks.state or automatically approve trust on the user's behalf.
    tauri::async_runtime::spawn_blocking(|| open_link("codex://settings"))
        .await
        .map_err(|e| e.to_string())?
}

pub fn open_link(link: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(link).status();
    #[cfg(windows)]
    let result = Command::new("explorer.exe").arg(link).status();
    #[cfg(not(any(target_os = "macos", windows)))]
    let result = Command::new("xdg-open").arg(link).status();
    match result {
        Ok(s) if s.success() => Ok(()),
        _ => {
            Err("无法打开 Codex。请手动打开 Codex 设置 → Hooks，审阅并信任 YOGO Pet。".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn response() -> Value {
        let hooks: Vec<_> = ["sessionStart", "userPromptSubmit", "preToolUse", "postToolUse", "permissionRequest", "stop", "interrupt", "sessionEnd"].iter().map(|e| json!({"pluginId":"yogo-pet@personal","eventName":e,"enabled":true,"trustStatus":"trusted"})).collect();
        json!({"result":{"data":[{"hooks":hooks,"errors":[]}]}})
    }
    #[test]
    fn authorization_requires_every_hook_and_current_trust() {
        let mut r = response();
        assert_eq!(classify(&r)["state"], "authorized");
        r["result"]["data"][0]["hooks"][3]["trustStatus"] = json!("modified");
        assert_eq!(classify(&r)["state"], "modified");
        r["result"]["data"][0]["hooks"][3]["trustStatus"] = json!("trusted");
        r["result"]["data"][0]["hooks"][3]["enabled"] = json!(false);
        assert_eq!(classify(&r)["state"], "disabled");
        r["result"]["data"][0]["hooks"]
            .as_array_mut()
            .unwrap()
            .remove(3);
        assert_eq!(classify(&r)["state"], "error");
    }
    #[test]
    fn distinguish_missing_disabled_and_unloaded_plugin() {
        assert_eq!(
            classify_installation(&json!({"result":{"marketplaces":[]}}))["state"],
            "missing"
        );
        let mut r = json!({"result":{"marketplaces":[{"plugins":[{"name":"yogo-pet","installed":true,"enabled":false}]}]}});
        assert_eq!(classify_installation(&r)["state"], "plugin_disabled");
        r["result"]["marketplaces"][0]["plugins"][0]["enabled"] = json!(true);
        assert_eq!(classify_installation(&r)["state"], "error");
        assert_eq!(
            classify_installation(&json!({"error":{}}))["state"],
            "error"
        );
    }
    #[test]
    fn errors_and_other_plugins_do_not_mean_authorized() {
        assert_eq!(
            classify(&json!({"error":{"message":"unsupported"}}))["state"],
            "error"
        );
        assert_eq!(classify(&json!({"result":{"data":[]}}))["state"], "error");
        let mut r = response();
        for h in r["result"]["data"][0]["hooks"].as_array_mut().unwrap() {
            h["pluginId"] = json!("other@personal");
        }
        assert_eq!(classify(&r)["state"], "missing");
    }
}
