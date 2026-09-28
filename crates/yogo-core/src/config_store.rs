use crate::events::{atomic_write, private_dir};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub auto_connect: bool,
    pub completed_seconds: u64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_connect: false,
            completed_seconds: 8,
        }
    }
}
pub struct ConfigStore {
    pub dir: PathBuf,
}
impl ConfigStore {
    pub fn open(dir: PathBuf) -> Result<Self, String> {
        private_dir(&dir)?;
        private_dir(&dir.join("events"))?;
        Ok(Self { dir })
    }
    pub fn load(&self) -> Result<Preferences, String> {
        match fs::read(self.dir.join("preferences.json")) {
            Ok(bytes) => {
                let p: Preferences =
                    serde_json::from_slice(&bytes).map_err(|e| format!("设置文件无效：{e}"))?;
                Self::validate(&p)?;
                Ok(p)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn validate(p: &Preferences) -> Result<(), String> {
        if !(2..=60).contains(&p.completed_seconds) {
            Err("完成图标停留时间应为 2–60 秒".into())
        } else {
            Ok(())
        }
    }
    pub fn save(&self, p: &Preferences) -> Result<(), String> {
        Self::validate(p)?;
        atomic_write(
            &self.dir.join("preferences.json"),
            &serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?,
        )
    }
}
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("YOGO_PET_HOME") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_else(|| ".".into());
    PathBuf::from(home).join(".local/share/yogo-pet")
}
