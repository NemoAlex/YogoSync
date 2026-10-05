use crate::{
    art::Frame,
    config_store::{ConfigStore, Preferences},
    device_service::DeviceService,
    events::consume_events,
    now_ms,
    task_states::{PetState, TaskSnapshot, TaskStates},
    themes::{Theme, ThemeLibrary, ThemeOption},
};
use fs2::FileExt;
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: u64,
    pub level: String,
    pub message: String,
}
#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    pub phase: String,
    pub error: String,
    pub model: String,
    pub preferences: Preferences,
    pub tasks: TaskSnapshot,
    pub state: PetState,
    pub frame: Frame,
    pub theme_name: String,
    pub active_theme_id: String,
    pub theme_options: Vec<ThemeOption>,
    pub theme_previews: std::collections::BTreeMap<String, Frame>,
    pub demo: bool,
    pub logs: Vec<LogEntry>,
    pub data_dir: String,
    pub recovery_pending: bool,
}
enum Action {
    Start,
    Stop,
    Demo(PetState),
    Auto,
    Save(Preferences),
    PreviewCover(String),
    ClearLogs,
    SaveTheme(Theme),
    ActivateTheme(String),
    Shutdown,
}
struct Request {
    action: Action,
    reply: Sender<Result<(), String>>,
}
#[derive(Clone)]
pub struct ServiceRuntime {
    sender: Sender<Request>,
    snapshot: Arc<Mutex<DesktopSnapshot>>,
}
struct Worker {
    store: ConfigStore,
    snapshot: Arc<Mutex<DesktopSnapshot>>,
    local: DesktopSnapshot,
    tasks: TaskStates,
    device: Option<DeviceService>,
    demo: Option<(PetState, u64)>,
    last_frame: Option<Frame>,
    themes: ThemeLibrary,
    animation_start: Instant,
    next_frame: Instant,
    on_change: Box<dyn Fn(DesktopSnapshot) + Send>,
    _lock: File,
}
impl ServiceRuntime {
    pub fn spawn(
        dir: PathBuf,
        on_change: impl Fn(DesktopSnapshot) + Send + 'static,
    ) -> Result<Self, String> {
        let store = ConfigStore::open(dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(store.dir.join("desktop.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive()
            .map_err(|_| "YogoSync 桌面服务已在运行".to_string())?;
        let preferences = store.load()?;
        let themes = ThemeLibrary::load(&store.dir)?;
        let local = DesktopSnapshot {
            phase: "stopped".into(),
            error: String::new(),
            model: String::new(),
            preferences,
            tasks: TaskStates::default().snapshot(now_ms(), 8),
            state: PetState::Idle,
            frame: themes.active().sample(PetState::Idle, 0).0,
            theme_name: themes.active().name.clone(),
            active_theme_id: themes.active_id.clone(),
            theme_options: themes.options(),
            theme_previews: themes.active().previews(),
            demo: false,
            logs: vec![],
            data_dir: store.dir.display().to_string(),
            recovery_pending: store.dir.join("device-backup.json").exists(),
        };
        let snapshot = Arc::new(Mutex::new(local.clone()));
        let (sender, receiver) = mpsc::channel();
        let mut worker = Worker {
            store,
            snapshot: snapshot.clone(),
            local,
            tasks: TaskStates::default(),
            device: None,
            demo: None,
            last_frame: None,
            themes,
            animation_start: Instant::now(),
            next_frame: Instant::now(),
            on_change: Box::new(on_change),
            _lock: lock,
        };
        thread::Builder::new()
            .name("yogo-device-service".into())
            .spawn(move || worker.run(receiver))
            .map_err(|e| e.to_string())?;
        Ok(Self { sender, snapshot })
    }
    fn request(&self, action: Action) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Request { action, reply: tx })
            .map_err(|_| "服务已关闭".to_string())?;
        rx.recv_timeout(Duration::from_secs(60))
            .map_err(|_| "设备操作超时".to_string())?
    }
    pub fn snapshot(&self) -> DesktopSnapshot {
        self.snapshot.lock().unwrap().clone()
    }
    pub fn start(&self) -> Result<(), String> {
        self.request(Action::Start)
    }
    pub fn stop(&self) -> Result<(), String> {
        self.request(Action::Stop)
    }
    pub fn demo(&self, state: PetState) -> Result<(), String> {
        self.request(Action::Demo(state))
    }
    pub fn automatic(&self) -> Result<(), String> {
        self.request(Action::Auto)
    }
    pub fn save(&self, p: Preferences) -> Result<(), String> {
        self.request(Action::Save(p))
    }
    pub fn set_preview_cover(&self, color: String) -> Result<(), String> {
        self.request(Action::PreviewCover(color))
    }
    pub fn save_theme(&self, theme: Theme) -> Result<(), String> {
        self.request(Action::SaveTheme(theme))
    }
    pub fn activate_theme(&self, id: String) -> Result<(), String> {
        self.request(Action::ActivateTheme(id))
    }
    pub fn clear_logs(&self) -> Result<(), String> {
        self.request(Action::ClearLogs)
    }
    pub fn shutdown(&self) -> Result<(), String> {
        self.request(Action::Shutdown)
    }
}
impl Worker {
    fn record(&mut self, level: &str, message: impl Into<String>) {
        self.local.logs.push(LogEntry {
            at: now_ms(),
            level: level.into(),
            message: message.into(),
        });
        if self.local.logs.len() > 250 {
            self.local.logs.remove(0);
        }
    }
    fn publish(&mut self) {
        self.local.theme_name = self.themes.active().name.clone();
        self.local.active_theme_id = self.themes.active_id.clone();
        self.local.theme_options = self.themes.options();
        self.local.theme_previews = self.themes.active().previews();
        self.local.recovery_pending = self.store.dir.join("device-backup.json").exists();
        let mut shared = self.snapshot.lock().unwrap();
        if *shared != self.local {
            *shared = self.local.clone();
            drop(shared);
            (self.on_change)(self.local.clone());
        }
    }
    fn start(&mut self) -> Result<(), String> {
        if self.device.is_some() {
            return Ok(());
        }
        self.local.phase = "starting".into();
        self.local.error.clear();
        self.publish();
        let mut d = DeviceService::open(self.store.dir.clone())?;
        if let Err(e) = d.begin() {
            let recovery = d.restore();
            return Err(match recovery {
                Ok(_) => e,
                Err(r) => format!("{e}；恢复待重试：{r}"),
            });
        }
        self.local.model = d.model.clone();
        self.device = Some(d);
        self.last_frame = None;
        self.animation_start = Instant::now();
        self.local.phase = "running".into();
        self.record("success", "接收器已连接，原灯效已备份");
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.local.phase = "stopping".into();
        self.publish();
        if let Some(mut d) = self.device.take() {
            d.restore()?;
            self.record("info", "已恢复原灯效，设备已断开");
        } else if self.store.dir.join("device-backup.json").exists() {
            let mut d = DeviceService::open(self.store.dir.clone())?;
            d.restore()?;
            self.record("success", "已恢复上次中断前的灯效");
        }
        self.local.phase = "stopped".into();
        self.local.model.clear();
        self.local.error.clear();
        self.last_frame = None;
        Ok(())
    }
    fn fail(&mut self, error: String) {
        self.local.error = error.clone();
        self.local.phase = "error".into();
        self.record("error", error);
    }
    fn tick(&mut self) -> Result<(), String> {
        let now = now_ms();
        // Do not race the earlier Node prototype for the same event spool.
        if !self.store.dir.join("server.lock").exists() {
            for event in consume_events(&self.store.dir)? {
                let name = event.event.clone();
                if self.tasks.apply(event, now) {
                    self.record("info", format!("Codex · {name}"));
                }
            }
        }
        self.local.tasks = self
            .tasks
            .snapshot(now, self.local.preferences.completed_seconds);
        if self.demo.is_some_and(|(_, until)| now >= until) {
            self.demo = None
        }
        self.local.demo = self.demo.is_some();
        let previous_state = self.local.state;
        self.local.state = self.demo.map(|(s, _)| s).unwrap_or(self.local.tasks.state);
        if previous_state != self.local.state {
            self.animation_start = Instant::now();
        }
        let (frame, remaining) = self.themes.active().sample(
            self.local.state,
            self.animation_start.elapsed().as_millis() as u64,
        );
        self.local.frame = frame;
        self.local.theme_name = self.themes.active().name.clone();
        self.next_frame = Instant::now() + Duration::from_millis(remaining);
        if let Some(d) = self.device.as_mut() {
            if self.last_frame.as_ref() != Some(&self.local.frame) {
                d.write_frame(&self.local.frame)?;
                self.last_frame = Some(self.local.frame);
            }
        }
        Ok(())
    }
    fn run(&mut self, receiver: Receiver<Request>) {
        self.record("info", "桌面服务就绪 · 仅接收任务状态，不读取对话正文");

        loop {
            match receiver.recv_timeout(
                self.next_frame
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(100)),
            ) {
                Ok(req) => {
                    let shutdown = matches!(req.action, Action::Shutdown);
                    let device_action =
                        matches!(req.action, Action::Start | Action::Stop | Action::Shutdown);
                    let result = match req.action {
                        Action::Start => self.start(),
                        Action::Stop | Action::Shutdown => self.stop(),
                        Action::Demo(state) => {
                            self.animation_start = Instant::now();
                            self.demo = Some((state, now_ms() + 5000));
                            Ok(())
                        }
                        Action::Auto => {
                            self.animation_start = Instant::now();
                            self.demo = None;
                            Ok(())
                        }
                        Action::Save(mut p) => {
                            p.preview_cover = self.local.preferences.preview_cover.clone();
                            self.store.save(&p).map(|_| {
                                self.local.preferences = p;
                                self.record("info", "设置已保存");
                            })
                        }
                        Action::PreviewCover(color) => {
                            let mut p = self.local.preferences.clone();
                            p.preview_cover = color;
                            self.store.save(&p).map(|_| self.local.preferences = p)
                        }
                        Action::SaveTheme(theme) => {
                            let updates_active = theme.id == self.themes.active_id;
                            self.themes.upsert(theme, &self.store.dir).map(|_| {
                                if updates_active {
                                    self.animation_start = Instant::now();
                                }
                            })
                        }
                        Action::ActivateTheme(id) => {
                            self.themes.activate(id, &self.store.dir).map(|_| {
                                self.animation_start = Instant::now();
                            })
                        }
                        Action::ClearLogs => {
                            self.local.logs.clear();
                            Ok(())
                        }
                    };
                    if let Err(e) = &result {
                        if device_action {
                            self.fail(e.clone())
                        } else {
                            self.record("error", e.clone())
                        }
                    }
                    self.publish();
                    let successful = result.is_ok();
                    let _ = req.reply.send(result);
                    // Keep the app alive if restoring failed so the user can retry.
                    if shutdown && successful {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = self.stop();
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if let Err(e) = self.tick() {
                self.device = None;
                self.local.model.clear();
                        if self.local.error != e {
                    self.fail(e)
                }
            }
            self.publish();
        }
    }
}
