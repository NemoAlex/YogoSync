use yogo_core::{
    service_runtime::ServiceRuntime,
    task_states::PetState,
    themes::{Theme, ThemeFrame, ThemeLibrary},
};
#[test]
fn durations_loop_and_skip_late_frames() {
    let mut theme = Theme::builtin();
    let a = [[[1; 3]; 6]; 6];
    let b = [[[2; 3]; 6]; 6];
    theme.states.insert(
        "idle".into(),
        vec![
            ThemeFrame {
                duration_ms: 100,
                pixels: a,
            },
            ThemeFrame {
                duration_ms: 350,
                pixels: b,
            },
        ],
    );
    theme.validate().unwrap();
    assert_eq!(theme.sample(PetState::Idle, 0), (a, 100));
    assert_eq!(theme.sample(PetState::Idle, 99), (a, 1));
    assert_eq!(theme.sample(PetState::Idle, 100), (b, 350));
    assert_eq!(theme.sample(PetState::Idle, 450), (a, 100));
    assert_eq!(theme.sample(PetState::Idle, 450_120), (b, 330));
}
#[test]
fn reject_invalid_imports_and_preserve_all_pixels() {
    let original = Theme::builtin();
    assert_eq!(
        serde_json::from_str::<Theme>(&serde_json::to_string(&original).unwrap()).unwrap(),
        original
    );
    let mut t = original.clone();
    t.version = 2;
    assert!(t.validate().is_err());
    t = original.clone();
    t.states.remove("idle");
    assert!(t.validate().is_err());
    t = original.clone();
    t.states.get_mut("idle").unwrap().clear();
    assert!(t.validate().is_err());
    for duration in [0, 99, 60001] {
        t = original.clone();
        t.states.get_mut("idle").unwrap()[0].duration_ms = duration;
        assert!(t.validate().is_err());
    }
    let mut v = serde_json::to_value(&original).unwrap();
    v["states"]["idle"][0]["pixels"][0][0][0] = serde_json::json!(256);
    assert!(serde_json::from_value::<Theme>(v).is_err());
}
#[test]
fn library_persists_selection_and_failed_save_is_non_destructive() {
    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let mut lib = ThemeLibrary::load(&dir).unwrap();
    let mut t = Theme::builtin();
    assert!(lib.upsert(t.clone(), &dir).is_err());
    t.id = uuid::Uuid::new_v4().to_string();
    t.name = "我的主题".into();
    lib.upsert(t.clone(), &dir).unwrap();
    lib.activate(t.id.clone(), &dir).unwrap();
    assert_eq!(ThemeLibrary::load(&dir).unwrap().active(), &t);
    t.states.clear();
    assert!(lib.upsert(t, &dir).is_err());
    assert_eq!(ThemeLibrary::load(&dir).unwrap().active().name, "我的主题");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn runtime_plays_saved_theme_and_restores_it_after_restart() {
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };
    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let output = seen.clone();
    let runtime =
        ServiceRuntime::spawn(dir.clone(), move |s| output.lock().unwrap().push(s.frame)).unwrap();
    let mut t = Theme::builtin();
    t.id = uuid::Uuid::new_v4().to_string();
    t.name = "播放测试".into();
    let a = [[[1; 3]; 6]; 6];
    let b = [[[2; 3]; 6]; 6];
    t.states.insert(
        "idle".into(),
        vec![
            ThemeFrame {
                duration_ms: 100,
                pixels: a,
            },
            ThemeFrame {
                duration_ms: 100,
                pixels: b,
            },
        ],
    );
    runtime.save_theme(t.clone()).unwrap();
    assert_eq!(runtime.snapshot().active_theme_id, "builtin");
    assert!(runtime
        .snapshot()
        .theme_options
        .iter()
        .any(|option| option.id == t.id && option.name == t.name));
    runtime.activate_theme(t.id.clone()).unwrap();
    assert_eq!(runtime.snapshot().active_theme_id, t.id);
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        let s = seen.lock().unwrap();
        if s.contains(&a) && s.contains(&b) {
            break;
        }
        drop(s);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(seen.lock().unwrap().contains(&a));
    assert!(seen.lock().unwrap().contains(&b));
    runtime.shutdown().unwrap();
    drop(runtime);
    // Wait for the worker to release the single-instance lock after acknowledging shutdown.
    let next = loop {
        match ServiceRuntime::spawn(dir.clone(), |_| {}) {
            Ok(r) => break r,
            Err(_) if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => panic!("{e}"),
        }
    };
    assert_eq!(next.snapshot().theme_name, "播放测试");
    next.shutdown().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn builtin_symbols_keep_exclamation_centered_and_animations_valid() {
    let theme = Theme::builtin();
    theme.validate().unwrap();
    for frame in &theme.states["waiting"] {
        for row in frame.pixels {
            assert_eq!(row[2], row[3]);
            for x in [0, 1, 4, 5] {
                assert_eq!(row[x], [0; 3]);
            }
        }
        assert_eq!(frame.pixels[3], [[0; 3]; 6]);
        assert_eq!(frame.pixels[4], [[0; 3]; 6]);
    }
    for state in ["thinking", "working"] {
        let frames = &theme.states[state];
        assert!(frames
            .windows(2)
            .all(|pair| pair[0].pixels != pair[1].pixels));
    }
}

#[test]
fn working_animation_has_balanced_opposing_lights() {
    let theme = Theme::builtin();
    assert!(theme.states.values().all(|frames| frames.len() > 1));
    for frame in &theme.states["working"] {
        for y in 0..6 {
            for x in 0..6 {
                assert_eq!(frame.pixels[y][x], frame.pixels[5 - y][5 - x]);
            }
        }
    }
}

#[test]
fn preview_cover_persists_and_settings_cannot_overwrite_it() {
    use yogo_core::config_store::ConfigStore;
    let dir = std::env::temp_dir().join(format!("yogo-cover-{}", uuid::Uuid::new_v4()));
    let runtime = ServiceRuntime::spawn(dir.clone(), |_| {}).unwrap();
    let mut stale_settings = runtime.snapshot().preferences;
    runtime.set_preview_cover("yellow".into()).unwrap();
    stale_settings.completed_seconds = 12;
    runtime.save(stale_settings).unwrap();
    assert!(runtime.set_preview_cover("invalid".into()).is_err());
    runtime.shutdown().unwrap();
    let persisted = ConfigStore::open(dir.clone()).unwrap().load().unwrap();
    assert_eq!(persisted.preview_cover, "yellow");
    assert_eq!(persisted.completed_seconds, 12);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let next = loop {
        match ServiceRuntime::spawn(dir.clone(), |_| {}) {
            Ok(r) => break r,
            Err(_) if std::time::Instant::now() < until => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            Err(e) => panic!("{e}"),
        }
    };
    assert_eq!(next.snapshot().preferences.preview_cover, "yellow");
    next.shutdown().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
