use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use flux_core::{PriorityEntry, ResultKind, SearchResult, Settings};
use windui::prelude::Signal;

use crate::i18n::I18nHub;
use crate::settings_view::SettingsUiState;

pub(crate) struct LauncherSettingsState {
    pub(crate) shared_settings: Arc<RwLock<Settings>>,
    pub(crate) query_history: Rc<RefCell<Vec<String>>>,
    pub(crate) priorities: Signal<Vec<PriorityEntry>>,
    pub(crate) i18n_hub: I18nHub,
    pub(crate) game_mode: Signal<bool>,
    pub(crate) game_mode_status: Signal<String>,
    pub(crate) settings_ui: SettingsUiState,
    pub(crate) language_preference: Signal<usize>,
}

impl LauncherSettingsState {
    pub(crate) fn from_settings(settings: &Settings) -> Self {
        let settings_ui = SettingsUiState::new(
            windui::prelude::signal(std::env::var_os("FLUX_OPEN_SETTINGS").is_some()),
            windui::prelude::signal(
                std::env::var("FLUX_SMOKE_SETTINGS_TAB")
                    .ok()
                    .and_then(|value| value.parse::<usize>().ok())
                    .filter(|tab| *tab < 4)
                    .unwrap_or(0),
            ),
        );
        Self {
            shared_settings: Arc::new(RwLock::new(settings.clone())),
            query_history: Rc::new(RefCell::new(settings.query_history.clone())),
            priorities: windui::prelude::signal(settings.priority_entries.clone()),
            i18n_hub: I18nHub::new(),
            game_mode: windui::prelude::signal(settings.game_mode),
            game_mode_status: windui::prelude::signal(crate::ui_helpers::game_mode_label(
                settings.game_mode,
            )),
            settings_ui,
            language_preference: windui::prelude::signal(crate::i18n::language_preference_index(
                settings.language,
            )),
        }
    }
}

static SETTINGS_SAVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn settings_save_lock() -> &'static Mutex<()> {
    SETTINGS_SAVE_LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn save_settings(settings: &Settings) -> bool {
    let Ok(_save_guard) = settings_save_lock().lock() else {
        return false;
    };
    settings.save().is_ok()
}

pub(crate) fn save_settings_async(settings: &Arc<RwLock<Settings>>) {
    let settings = Arc::clone(settings);
    let _ = std::thread::Builder::new()
        .name(String::from("flux-settings-save"))
        .spawn(move || {
            // Read the latest settings snapshot after waiting for any mutation.
            if let Ok(settings_guard) = settings.read() {
                let _ = save_settings(&settings_guard);
            }
        });
}

pub(crate) fn record_query_history(
    settings: &Arc<RwLock<Settings>>,
    history: &Rc<RefCell<Vec<String>>>,
    query: &str,
) {
    let Ok(mut settings_guard) = settings.write() else {
        return;
    };
    if !settings_guard.record_query(query) {
        return;
    }
    *history.borrow_mut() = settings_guard.query_history.clone();
    drop(settings_guard);
    // Keep Enter→hide free of synchronous filesystem I/O.
    save_settings_async(settings);
}

pub(crate) fn set_result_priority(
    settings: &Arc<RwLock<Settings>>,
    priorities: Signal<Vec<PriorityEntry>>,
    result: &SearchResult,
) -> bool {
    let Some(target) = result.target.as_deref() else {
        return false;
    };
    if !matches!(result.kind, ResultKind::Application) {
        return false;
    }
    let Ok(mut settings_guard) = settings.write() else {
        return false;
    };
    settings_guard.add_priority(PriorityEntry {
        id: result.id.clone(),
        title: result.title.clone(),
        target: target.to_owned(),
    });
    let entries = settings_guard.priority_entries.clone();
    let saved = save_settings(&settings_guard);
    if saved {
        priorities.set(entries);
    }
    saved
}

pub(crate) fn remove_priority_entry(
    settings: &Arc<RwLock<Settings>>,
    priorities: Signal<Vec<PriorityEntry>>,
    id: &str,
) -> bool {
    let Ok(mut settings_guard) = settings.write() else {
        return false;
    };
    if !settings_guard.remove_priority(id) {
        return false;
    }
    let entries = settings_guard.priority_entries.clone();
    let saved = save_settings(&settings_guard);
    if saved {
        priorities.set(entries);
    }
    saved
}

pub(crate) fn move_priority_entry(
    settings: &Arc<RwLock<Settings>>,
    priorities: Signal<Vec<PriorityEntry>>,
    id: &str,
    direction: i32,
) -> bool {
    let Ok(mut settings_guard) = settings.write() else {
        return false;
    };
    let Some(index) = settings_guard
        .priority_entries
        .iter()
        .position(|entry| entry.id == id)
    else {
        return false;
    };
    if !settings_guard.move_priority(index, direction) {
        return false;
    }
    let entries = settings_guard.priority_entries.clone();
    let saved = save_settings(&settings_guard);
    if saved {
        priorities.set(entries);
    }
    saved
}

pub(crate) fn set_game_mode(
    settings: &Arc<RwLock<Settings>>,
    game_mode: Signal<bool>,
    status: Signal<String>,
    enabled: bool,
) {
    if let Ok(mut settings) = settings.write() {
        settings.game_mode = enabled;
        game_mode.set(enabled);
        status.set(crate::ui_helpers::game_mode_label(enabled));
        let _ = save_settings(&settings);
    }
}
