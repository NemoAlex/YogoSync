use crate::{
    art::{frame_for, Frame},
    events::atomic_write,
    task_states::PetState,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const STATES: [(PetState, &str); 6] = [
    (PetState::Idle, "idle"),
    (PetState::Thinking, "thinking"),
    (PetState::Working, "working"),
    (PetState::Waiting, "waiting"),
    (PetState::Done, "done"),
    (PetState::Interrupted, "interrupted"),
];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeFrame {
    pub duration_ms: u64,
    pub pixels: Frame,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Theme {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub states: BTreeMap<String, Vec<ThemeFrame>>,
}
impl Theme {
    pub fn builtin() -> Self {
        let dim = |mut pixels: Frame, rows: &[usize], percent: u16| {
            for &y in rows {
                for pixel in &mut pixels[y] {
                    for c in pixel {
                        *c = (*c as u16 * percent / 100) as u8;
                    }
                }
            }
            pixels
        };
        let frame = |pixels, duration_ms| ThemeFrame {
            pixels,
            duration_ms,
        };
        let mut states = BTreeMap::new();
        let idle = frame_for(PetState::Idle);
        let mut blink = idle;
        for y in [1, 2] {
            for x in [1, 4] {
                blink[y][x] = [0, 7, 10];
            }
        }
        states.insert("idle".into(), vec![frame(idle, 2600), frame(blink, 160)]);
        // Alternate the three-dot group by one column to balance its odd width.
        let mut thinking_frames = Vec::new();
        for offset in [0, 1] {
            for active in 0..3 {
                let mut pixels = [[[0; 3]; 6]; 6];
                for dot in 0..3 {
                    let color = if dot == active {
                        [85, 45, 130]
                    } else {
                        [13, 6, 20]
                    };
                    pixels[2 + offset][dot * 2 + offset] = color;
                }
                thinking_frames.push(frame(pixels, 350));
            }
        }
        states.insert("thinking".into(), thinking_frames);
        let mut working_frames = Vec::new();
        // Equal one-pixel steps around a centered square, with opposing lights.
        let path = [
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (4, 2),
            (4, 3),
            (4, 4),
            (3, 4),
            (2, 4),
            (1, 4),
            (1, 3),
            (1, 2),
        ];
        for index in 0..path.len() / 2 {
            let mut pixels = [[[0; 3]; 6]; 6];
            for opposite in [0, path.len() / 2] {
                for trail in 0..2 {
                    let (x, y) = path[(index + opposite + path.len() - trail) % path.len()];
                    let percent = [100, 20][trail];
                    pixels[y][x] = [125u16, 95, 10].map(|v| (v * percent / 100) as u8);
                }
            }
            working_frames.push(frame(pixels, 220));
        }
        states.insert("working".into(), working_frames);
        let waiting = frame_for(PetState::Waiting);
        states.insert(
            "waiting".into(),
            vec![
                frame(waiting, 1100),
                frame(dim(waiting, &[0, 1, 2, 5], 45), 650),
            ],
        );
        for (key, state) in [
            ("done", PetState::Done),
            ("interrupted", PetState::Interrupted),
        ] {
            let pixels = frame_for(state);
            states.insert(
                key.into(),
                [(100, 1600), (80, 250), (60, 400), (80, 250)]
                    .into_iter()
                    .map(|(brightness, duration)| {
                        frame(dim(pixels, &[0, 1, 2, 3, 4, 5], brightness), duration)
                    })
                    .collect(),
            );
        }
        Self {
            version: 1,
            id: "builtin".into(),
            name: "机器人与符号".into(),
            states,
        }
    }

    pub fn previews(&self) -> BTreeMap<String, Frame> {
        self.states
            .iter()
            .map(|(key, frames)| {
                let frame = frames.iter().max_by_key(|f| f.duration_ms).unwrap();
                (key.clone(), frame.pixels)
            })
            .collect()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("不支持此主题版本".into());
        }
        if self.id != "builtin" && uuid::Uuid::parse_str(&self.id).is_err() {
            return Err("主题标识无效".into());
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 40 {
            return Err("主题名称应为 1–40 个字符".into());
        }
        if self.states.len() != 6 {
            return Err("主题必须包含全部 6 种状态".into());
        }
        for (_, key) in STATES {
            let frames = self
                .states
                .get(key)
                .ok_or_else(|| format!("缺少状态：{key}"))?;
            if frames.is_empty() || frames.len() > 60 {
                return Err("每种状态需要 1–60 帧".into());
            }
            if frames
                .iter()
                .any(|f| !(100..=60000).contains(&f.duration_ms))
            {
                return Err("每帧时长应为 100–60000 毫秒".into());
            }
        }
        Ok(())
    }
    // The elapsed clock skips overdue frames rather than queueing device writes.
    pub fn sample(&self, state: PetState, elapsed_ms: u64) -> (Frame, u64) {
        let key = STATES.iter().find(|(s, _)| *s == state).unwrap().1;
        let frames = &self.states[key];
        let total: u64 = frames.iter().map(|f| f.duration_ms).sum();
        let mut position = elapsed_ms % total;
        for frame in frames {
            if position < frame.duration_ms {
                return (frame.pixels, frame.duration_ms - position);
            }
            position -= frame.duration_ms;
        }
        unreachable!()
    }
}
#[derive(Clone, Serialize, PartialEq)]
pub struct ThemeOption {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeLibrary {
    pub active_id: String,
    pub themes: Vec<Theme>,
}
impl Default for ThemeLibrary {
    fn default() -> Self {
        Self {
            active_id: "builtin".into(),
            themes: vec![Theme::builtin()],
        }
    }
}
impl ThemeLibrary {
    pub fn options(&self) -> Vec<ThemeOption> {
        self.themes
            .iter()
            .map(|t| ThemeOption {
                id: t.id.clone(),
                name: t.name.clone(),
            })
            .collect()
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        let bytes = match fs::read(dir.join("themes.json")) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut library: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("主题库无效：{e}"))?;
        if library.themes.len() > 100 {
            return Err("主题库最多保存 100 个主题".into());
        }
        let mut ids = std::collections::HashSet::new();
        for theme in &library.themes {
            theme.validate()?;
            if !ids.insert(&theme.id) {
                return Err("主题标识重复".into());
            }
        }
        library.themes.retain(|t| t.id != "builtin");
        library.themes.insert(0, Theme::builtin());
        if !library.themes.iter().any(|t| t.id == library.active_id) {
            return Err("找不到当前主题".into());
        }
        Ok(library)
    }
    pub fn active(&self) -> &Theme {
        self.themes.iter().find(|t| t.id == self.active_id).unwrap()
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        atomic_write(
            &dir.join("themes.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn upsert(&mut self, mut theme: Theme, dir: &Path) -> Result<(), String> {
        theme.validate()?;
        if theme.id == "builtin" {
            return Err("请将内置主题另存为新主题".into());
        }
        theme.name = theme.name.trim().into();
        let mut next = self.clone();
        if let Some(old) = next.themes.iter_mut().find(|t| t.id == theme.id) {
            *old = theme;
        } else {
            if next.themes.len() >= 100 {
                return Err("主题库已满".into());
            }
            next.themes.push(theme);
        }
        next.save(dir)?;
        *self = next;
        Ok(())
    }
    pub fn activate(&mut self, id: String, dir: &Path) -> Result<(), String> {
        if !self.themes.iter().any(|t| t.id == id) {
            return Err("主题不存在，请先保存".into());
        }
        let mut next = self.clone();
        next.active_id = id;
        next.save(dir)?;
        *self = next;
        Ok(())
    }
}
