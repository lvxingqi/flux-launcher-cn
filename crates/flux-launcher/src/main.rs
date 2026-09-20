#![cfg_attr(windows, windows_subsystem = "windows")]

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

mod accent;
mod action_panel;
mod actions;
mod applications;
mod builtin;
mod entry;
mod everything;
mod fullscreen;
mod hotkeys;
mod i18n;
mod icons;
mod interval_state;
mod keyboard;
mod keyboard_layout;
mod launch;
mod launcher_dialogs;
mod monitor;
mod native_host;
mod plugin_limits;
mod plugin_transport;
mod plugins;
mod provider_state;
mod query;
mod result_row;
mod settings_state;
mod settings_view;
mod startup;
mod ui_helpers;
mod update_state;
mod updater;
mod visual_preview;
mod window_lifecycle;
mod window_state;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;

use crate::icons::{
    icon_completion_generation_changed, prewarm_icon_targets, request_shell_icon, tray_icon,
    SHELL_ICON_COMPLETION_GENERATION,
};
use crate::launch::trace_launch_event;
use accent::{parse_selection_color, selection_color_for_settings};
use actions::ActionItem;
#[cfg(test)]
pub(crate) use actions::ActionKind;
use everything::{refresh_everything_status_cheap, EverythingRuntimeState};
use flux_core::{
    should_suppress_activation, HotkeyConfig, MonitorPreference, SearchModel,
    DEFAULT_LAUNCHER_HEIGHT, DEFAULT_LAUNCHER_WIDTH, MAX_LAUNCHER_HEIGHT, MAX_LAUNCHER_WIDTH,
    MIN_LAUNCHER_HEIGHT, MIN_LAUNCHER_WIDTH,
};
use i18n::{
    apply_configured_locale, apply_system_locale, configured_locale,
    language_preference_from_index, language_preference_index,
};
use interval_state::dispatch_query;
#[cfg(test)]
pub(crate) use keyboard::history_cursor_step;
use keyboard::{
    alt_key_is_down, cycle_query_history, handle_action_entry, handle_action_mode,
    handle_copy_shortcut, handle_enter_key, handle_open_location_shortcut,
    handle_result_navigation, handle_run_as_admin_shortcut, is_run_as_admin_key, open_history_mode,
    shift_key_is_down, ActionKeyContext, EnterKeyContext,
};
use provider_state::{register_provider_channels, ProviderChannelContext, ProviderWorkers};
#[cfg(test)]
pub(crate) use query::should_publish_initial_query_results;
use query::SearchRuntimeState;
use result_row::result_row;
use settings_state::{save_settings, set_game_mode, LauncherSettingsState};
use ui_helpers::{
    action_bar_content, game_mode_label, launcher_theme, priorities_empty, selection_color_hex,
    selection_palette,
};
use update_state::{
    maybe_request_update_check, register_update_channels, request_update_check,
    request_update_install, update_check_due,
};
use window_state::{
    apply_launcher_size, dimension_from_slider, dimension_slider_fraction,
    launcher_window_geometry_with_prompt, launcher_window_geometry_with_sizes,
    monitor_preference_from_index, monitor_preference_index, parse_dimension_input,
    request_monitor_position, request_scroll, should_show_launcher, visual_preview_position,
    WindowBootstrap,
};
use windui::app::WindowSizeHandle;
use windui::core::Widget;
use windui::event::{Key, KeyEvent};
use windui::prelude::*;
use windui::render::Canvas;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SINGLE_INSTANCE_ID: &str = "lvxingqi.flux-launcher-cn";
const SETTINGS_WINDOW_WIDTH: i32 = 720;
const EVERYTHING_PROMPT_WINDOW_WIDTH: i32 = 440;
const EVERYTHING_PROMPT_WINDOW_HEIGHT: i32 = 242;
// The empty launcher is a compact search strip; the results state keeps the user-configured height.
const COMPACT_WINDOW_HEIGHT: i32 = 56;
const VISUAL_SLIDER_WIDTH: i32 = 200;
// The action group stays narrower than the minimum launcher content width so it
// can be centered between the same left/right content insets at every size.
const ACTION_BAR_WIDTH: i32 = 340;
const ACTION_BAR_HEIGHT: i32 = 22;
// Keep the result palette compact like the reference while exposing a six-row
// viewport; additional results remain available through the native wheel scroll.
const ACTION_WINDOW_HEIGHT: i32 = 250;
// Six 46-DIP result rows plus local scroll padding keep the footer close to the results.
const RESULT_VIEWPORT_HEIGHT: i32 = 288;
// 结果行行高（DIP），与 result_row 中的行高保持一致，用于推导图标请求窗口。
pub(crate) const RESULT_ROW_HEIGHT: i32 = 46;
/// 图标请求「立即提取」的行数上限：可见行（RESULT_VIEWPORT_HEIGHT / RESULT_ROW_HEIGHT）
/// 加两行余量。其余行在滚到选区附近时由响应式回调补发请求，避免一次结果集变化
/// 把几十个 Everything 文件全部塞进唯一的图标提取线程。
pub(crate) const EAGER_ICON_ROW_COUNT: usize =
    (RESULT_VIEWPORT_HEIGHT / RESULT_ROW_HEIGHT + 2) as usize;
const SETTINGS_WINDOW_HEIGHT: i32 = 520;
const LAUNCHER_FONT_FAMILY: &str = "Segoe UI Variable";
const SEARCH_INTERVAL: Duration = Duration::from_millis(40);
const EVERYTHING_MIN_QUERY_LEN: usize = 1;
const PLUGIN_MIN_QUERY_LEN: usize = 2;

#[derive(Default)]
struct ActionBarGeometryProbe {
    last: Cell<Option<(i32, i32, i32, i32)>>,
}

impl Widget for ActionBarGeometryProbe {
    fn paint(
        &self,
        bounds: Rect,
        _content: Rect,
        _focused: bool,
        _enabled: bool,
        _canvas: &mut dyn Canvas,
        _style: &Style,
    ) {
        if std::env::var_os("FLUX_SMOKE_ACTION_BAR").is_none() {
            return;
        }
        let geometry = (bounds.x, bounds.y, bounds.w, bounds.h);
        if self.last.get() != Some(geometry) {
            eprintln!(
                "ActionBarGeometry: x={} y={} width={} height={}",
                geometry.0, geometry.1, geometry.2, geometry.3
            );
            self.last.set(Some(geometry));
        }
    }
}

fn main() {
    #[cfg(windows)]
    {
        // Monitor coordinates are queried before windui creates the HWND. Set
        // per-monitor awareness first so Windows does not virtualize the
        // 4K/mixed-DPI work area used for the initial center position.
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }
    apply_system_locale();
    let mut args = std::env::args_os();
    let _executable = args.next();
    let mode = args.next();
    if entry::run_special_mode(mode.as_deref(), &mut args) {
        return;
    }
    let single_instance_disabled = std::env::var_os("FLUX_DISABLE_SINGLE_INSTANCE").is_some();
    if !single_instance_disabled
        && entry::should_claim_single_instance(mode.as_deref())
        && matches!(
            windui::claim_instance(SINGLE_INSTANCE_ID),
            windui::InstanceRole::Handoff
        )
    {
        return;
    }
    // The uninstaller uses this one-shot mode only to reach the already-running
    // instance through the single-instance listener. Never create a new UI if
    // there is no instance left to shut down.
    if entry::is_shutdown_mode(mode.as_deref()) {
        return;
    }
    let (settings, startup_launch) = entry::load_settings(mode.as_deref());
    let activation_hotkey = hotkeys::activation_hotkey(&settings.activation_hotkey);
    let settings_state = LauncherSettingsState::from_settings(&settings);
    let shared_settings = settings_state.shared_settings;
    let query_history = settings_state.query_history;
    let priorities = settings_state.priorities;
    let history_cursor = signal(None::<usize>);
    let history_mode = signal(false);

    let query = signal(String::new());
    let selected_id = signal(String::new());
    let selected_index = signal(0_usize);
    let selection_touched = signal(false);
    let action_mode = signal(false);
    let action_index = signal(0_usize);
    let action_scroll_pending = signal(false);
    let recycle_bin_confirmation = signal(false);
    let action_items = signal(Vec::<ActionItem>::new());
    let action_window_slot = Rc::new(RefCell::new(None::<WindowSizeHandle>));
    let i18n_hub = settings_state.i18n_hub;
    let status = i18n_hub.tr(|| t!("status.ready").into_owned());
    let update_status = i18n_hub.tr(|| t!("updater.checked_automatically").into_owned());
    let update_available = signal(None::<updater::StableUpdate>);
    let update_install_progress = signal(None::<(String, updater::DownloadProgress)>);
    let update_installing = signal(false);
    let current_sequence = signal(0_u64);
    let game_mode = settings_state.game_mode;
    let game_mode_status = settings_state.game_mode_status;
    let settings_ui = settings_state.settings_ui;
    let settings_visible = settings_ui.visible;
    let settings_tab = settings_ui.tab;
    let language_preference = settings_state.language_preference;
    let show_results = signal(false);
    let activation_key = signal(settings.activation_hotkey.key.clone());
    let activation_display = signal(hotkeys::display_config(&settings.activation_hotkey));
    let activation_recording = signal(false);
    let activation_ctrl = signal(settings.activation_hotkey.ctrl);
    let activation_alt = signal(settings.activation_hotkey.alt);
    let activation_shift = signal(settings.activation_hotkey.shift);
    let activation_meta = signal(settings.activation_hotkey.meta);
    let ignore_fullscreen = signal(settings.ignore_hotkeys_in_fullscreen);
    let smooth_caret = signal(settings.smooth_caret);
    let switch_to_english_layout = signal(settings.switch_to_english_layout);
    let use_system_accent = signal(settings.use_system_accent);
    let custom_selection_color = signal(selection_color_hex(settings.custom_selection_color));
    let launcher_width = signal(settings.launcher_width);
    let launcher_height = signal(settings.launcher_height);
    let launcher_width_input = signal(settings.launcher_width.to_string());
    let launcher_height_input = signal(settings.launcher_height.to_string());
    let launcher_width_slider = signal(dimension_slider_fraction(
        settings.launcher_width,
        MIN_LAUNCHER_WIDTH,
        MAX_LAUNCHER_WIDTH,
    ));
    let launcher_height_slider = signal(dimension_slider_fraction(
        settings.launcher_height,
        MIN_LAUNCHER_HEIGHT,
        MAX_LAUNCHER_HEIGHT,
    ));
    let launcher_preview_text = signal(
        t!(
            "settings.visual.client_area",
            width = settings.launcher_width,
            height = settings.launcher_height
        )
        .into_owned(),
    );
    let visual_preview_generation = signal(0_u64);
    let clear_query_on_activation = signal(settings.clear_query_on_activation);
    let start_with_windows = signal(settings.start_with_windows);
    let auto_enable_everything = signal(settings.auto_enable_everything);
    let everything_detection = Arc::new(crate::settings_view::EverythingDetectionSlot::default());
    let update_checks_enabled = signal(settings.update_checks_enabled);
    let update_interval_hours = signal(settings.update_interval_hours.to_string());
    let auto_install_updates = signal(settings.auto_install_updates);
    let obsidian_enabled = signal(settings.obsidian_enabled);
    let obsidian_alias = signal(settings.obsidian_alias.clone());
    let google_enabled = signal(settings.google_enabled);
    let google_alias = signal(settings.google_alias.clone());
    let system_commands_enabled = signal(settings.system_commands_enabled);
    let initial_monitor_preference = std::env::var("FLUX_SMOKE_MONITOR_PREFERENCE")
        .ok()
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "primary" => Some(MonitorPreference::Primary),
            "cursor" => Some(MonitorPreference::Cursor),
            "foreground" => Some(MonitorPreference::Foreground),
            _ => None,
        })
        .unwrap_or(settings.monitor_preference);
    let monitor_preference = signal(monitor_preference_index(initial_monitor_preference));
    let everything_state = EverythingRuntimeState::new(&settings);
    let everything_installed = everything_state.installed;
    let everything_prompt_visible_at_start = everything_state.prompt_visible_at_start;
    let everything_prompt_visible = everything_state.prompt_visible;
    let everything_status = everything_state.status;
    let selection_color = signal(selection_color_for_settings(&settings));
    let caret_duration = signal(settings.smooth_caret_duration_ms.to_string());

    let mut model = SearchModel::new();
    let search_state = SearchRuntimeState::new(model.results().to_vec());
    let results = search_state.results;
    let provider_results = search_state.provider_results;
    let plugin_actions = search_state.plugin_actions;
    let icon_refresh_generation = search_state.icon_refresh_generation;
    let inline_completion = search_state.inline_completion;
    let result_source = results;
    let selected_for_rows = selected_id;
    let selected_index_for_rows = selected_index;
    let selection_touched_for_rows = selection_touched;
    let actions_for_rows = Rc::clone(&plugin_actions);
    let settings_for_rows = Arc::clone(&shared_settings);
    let history_for_rows = Rc::clone(&query_history);
    let history_mode_for_rows = history_mode;
    let action_items_for_rows = action_items;
    let action_index_for_rows = action_index;
    let action_mode_for_rows = action_mode;
    let launcher_width_for_rows = launcher_width;
    let query_for_rows = query;
    let scroll_request_for_rows = signal(false);
    let settings_visible_for_rows = settings_visible;
    let window_size_slot_for_rows = Rc::clone(&action_window_slot);
    let query_caret_position = signal(query.with(|text| text.chars().count()));

    let search_placeholder = i18n_hub.tr(|| t!("search.placeholder").into_owned());
    let search_box = Element::text_input(query, search_placeholder)
        .cursor_position(query_caret_position)
        .leading_icon('⌕')
        .transparent_surface()
        .smooth_caret(settings.smooth_caret, settings.smooth_caret_duration_ms)
        .inline_completion(inline_completion)
        .show_focus_ring(false)
        .width_match()
        .font_family(LAUNCHER_FONT_FAMILY)
        .font_size(15.0)
        .font_weight(500)
        .corner(10.0)
        // The entire Search control stays transparent so the Windows Acrylic
        // material remains visible through the input, caret, and leading icon.
        .border(Color::rgba(0, 0, 0, 0), 0)
        .padding_xy(13, 0);

    let action_bar_content = action_bar_content(i18n_hub.clone());
    let action_bar = Element::stack()
        .width(ACTION_BAR_WIDTH)
        .height(ACTION_BAR_HEIGHT)
        .child(action_bar_content.align(Align::Center))
        // Keep the probe inside the same real frame so its telemetry describes
        // the exact slot that is centered between the launcher insets.
        .child(
            Element::leaf()
                .widget(ActionBarGeometryProbe::default())
                .fill(),
        )
        .align(Align::Center)
        .visible_when(move || show_results.get() && !action_mode.get());

    let result_list_body = Element::host_signal(result_source, move |result| {
        result_row(
            result,
            selected_for_rows,
            selected_index_for_rows,
            selection_touched_for_rows,
            result_source,
            icon_refresh_generation,
            Rc::clone(&actions_for_rows),
            action_items_for_rows,
            action_index_for_rows,
            action_scroll_pending,
            action_mode_for_rows,
            launcher_width_for_rows,
            query_for_rows,
            scroll_request_for_rows,
            selection_color,
            Arc::clone(&settings_for_rows),
            Rc::clone(&history_for_rows),
            history_mode_for_rows,
            recycle_bin_confirmation,
            settings_visible_for_rows,
            Rc::clone(&window_size_slot_for_rows),
        )
    })
    .width_match()
    // Keep the result body transparent so the window remains one continuous
    // Acrylic surface. Only individual result rows draw controls. The extra
    // right inset is local to the scroll content: it keeps the thumb clear of
    // row cards without changing the launcher window width.
    .padding_edges(6, 6, 18, 6);
    let result_list = Element::scroll()
        .width_match()
        .height(RESULT_VIEWPORT_HEIGHT)
        .child(result_list_body)
        .visible_when(move || show_results.get() && !action_mode.get());

    let everything_install_prompt = launcher_dialogs::everything_install_prompt(
        launcher_dialogs::EverythingInstallPromptContext {
            visible: everything_prompt_visible,
            status: everything_status,
            settings: Arc::clone(&shared_settings),
            i18n_hub: i18n_hub.clone(),
        },
    );
    let recycle_bin_dialog =
        launcher_dialogs::recycle_bin_dialog(launcher_dialogs::RecycleBinDialogContext {
            visible: recycle_bin_confirmation,
            status,
            i18n_hub: i18n_hub.clone(),
        });

    let action_list = action_panel::action_list(action_panel::ActionListContext {
        action_items: action_items_for_rows,
        action_index: action_index_for_rows,
        action_scroll_pending,
        result_source,
        selected_id: selected_for_rows,
        selected_index: selected_index_for_rows,
        action_mode: action_mode_for_rows,
        launcher_width: launcher_width_for_rows,
        launcher_height,
        action_window_slot: Rc::clone(&action_window_slot),
        settings: Arc::clone(&shared_settings),
        priorities,
        provider_results: Rc::clone(&provider_results),
        query,
    });

    // The HWND itself owns the system Acrylic surface. Keep this root transparent so
    // the blur fills the complete client area instead of becoming an inset card. The
    // content must match the live window width so result rows expand with resizing.
    // Keep the empty search strip and the results palette intrinsically sized. A
    // full-height column plus a weighted spacer made the compact state look too
    // tall and left an oversized gap between the last result and the footer.
    // Keep the compact Search baseline genuinely centered: equal vertical
    // insets avoid moving the empty-state control toward either edge.
    let launcher_content = Element::col()
        .width_match()
        .padding_edges(10, 7, 10, 7)
        .spacing(4)
        .child(search_box)
        .child(result_list)
        // The result viewport now ends immediately before the fixed footer. Do not
        // add a weighted spacer: it creates a visible blank band for short queries.
        .child(action_bar)
        .child(action_list)
        .child(recycle_bin_dialog)
        .child(everything_install_prompt);
    let launcher_surface = Element::stack()
        .fill()
        .bg(Color::rgba(0, 0, 0, 0))
        .child(launcher_content.align(Align::Center));

    let query_for_interval = query;
    let results_for_interval = results;
    let width_for_interval = launcher_width;
    let height_for_interval = launcher_height;
    let width_input_for_interval = launcher_width_input;
    let height_input_for_interval = launcher_height_input;
    let width_slider_for_interval = launcher_width_slider;
    let height_slider_for_interval = launcher_height_slider;
    let preview_text_for_interval = launcher_preview_text;
    let icon_refresh_generation_for_interval = icon_refresh_generation;
    let status_for_interval = status;
    let show_results_for_interval = show_results;
    let inline_completion_for_interval = inline_completion;
    let selection_touched_for_interval = selection_touched;
    let sequence_for_interval = current_sequence;
    let providers_for_interval = Rc::clone(&provider_results);
    let scroll_request_for_interval = scroll_request_for_rows;
    let actions_for_interval = Rc::clone(&plugin_actions);
    let auto_enable_everything_for_interval = auto_enable_everything;
    let obsidian_enabled_for_interval = obsidian_enabled;
    let obsidian_alias_for_interval = obsidian_alias;
    let google_enabled_for_interval = google_enabled;
    let google_alias_for_interval = google_alias;
    let system_commands_enabled_for_interval = system_commands_enabled;
    let history_mode_for_interval = history_mode;
    let language_preference_for_interval = language_preference;
    let settings_visible_for_interval = settings_visible;
    let settings_tab_for_interval = settings_tab;
    let everything_prompt_visible_for_interval = everything_prompt_visible;
    let everything_installed_for_interval = everything_installed;
    let everything_status_for_interval = everything_status;
    let everything_detection_for_interval = Arc::clone(&everything_detection);
    let visual_preview_generation_for_interval = visual_preview_generation;
    let visual_preview_smoke_for_interval =
        std::env::var_os("FLUX_SMOKE_VISUAL_SETTINGS").is_some();
    let everything_plugins_smoke_for_interval =
        std::env::var_os("FLUX_SMOKE_EVERYTHING_PLUGINS").is_some();
    let mut last_icon_generation = icon_refresh_generation.get();
    let mut last_launcher_width = launcher_width.get();
    let mut last_launcher_height = launcher_height.get();
    let mut last_settings_visible = settings_visible.get();
    let mut last_everything_prompt_visible = everything_prompt_visible.get();
    let mut last_query = String::new();
    // 空查询默认结果集只预热一次：启动后首个空闲 tick 完成，之后全部命中缓存。
    let mut default_icon_prewarm_done = false;
    let mut visual_preview_process: Option<visual_preview::PreviewProcess> = None;
    let mut last_visual_preview_request: Option<(u16, u16)> = None;
    let mut last_visual_preview_locale = String::new();
    let mut last_visual_preview_generation = visual_preview_generation.get();
    let mut last_visual_control_state: Option<(u16, u16, u32, u32)> = None;
    let mut everything_plugins_smoke_reported = false;
    let mut sequence = 0_u64;

    let settings_at_start = settings_visible.get();
    let window_icon = tray_icon();
    let window_bootstrap = WindowBootstrap::new(
        settings_at_start,
        everything_prompt_visible_at_start,
        launcher_width.get() as i32,
        initial_monitor_preference,
        &window_icon,
    );
    let mut app = window_bootstrap.app;
    if everything_prompt_visible_at_start
        && std::env::var_os("FLUX_SMOKE_EVERYTHING_PROMPT").is_some()
    {
        eprintln!("Everything install prompt: visible at startup");
        eprintln!(
            "Everything install prompt style: glass-transparent panel_fill=none modal_scrim=none window_background=transparent"
        );
    }
    let window_size = window_bootstrap.window_size;
    let window_position = window_bootstrap.window_position;
    let position_for_interval = window_position.clone();
    let settings_for_interval_geometry = Arc::clone(&shared_settings);
    let window_op = window_bootstrap.window_op;
    let cursor_visibility = window_bootstrap.cursor_visibility;
    let update_channels = register_update_channels(
        &mut app,
        Arc::clone(&shared_settings),
        update_status,
        update_available,
        update_install_progress,
        update_installing,
    );
    let update_install_sender = update_channels.install_sender;
    let update_sender = update_channels.update_sender;
    let update_install_in_flight = update_channels.install_in_flight;
    let update_check_in_flight = update_channels.check_in_flight;
    let update_checks_allowed = std::env::var("FLUX_DISABLE_UPDATE_CHECKS")
        .map(|value| value != "1")
        .unwrap_or(true);
    if update_checks_allowed && settings.update_checks_enabled && update_check_due(&settings) {
        request_update_check(update_sender.clone(), &update_check_in_flight);
    }
    let settings_for_update_interval = Arc::clone(&shared_settings);
    let update_sender_for_interval = update_sender.clone();
    let update_check_in_flight_for_interval = Rc::clone(&update_check_in_flight);
    *action_window_slot.borrow_mut() = Some(window_size.clone());
    let size_for_interval = window_size.clone();
    let size_for_visibility = window_size.clone();
    let provider_channels = register_provider_channels(
        &mut app,
        ProviderChannelContext {
            query,
            results,
            inline_completion,
            status,
            selected_id,
            selected_index,
            selection_touched,
            current_sequence,
            provider_results: Rc::clone(&provider_results),
            priorities,
            plugin_actions: Rc::clone(&plugin_actions),
            auto_enable_everything,
            everything_installed,
            everything_status,
        },
    );
    let application_sender = provider_channels.application_sender;
    let everything_sender = provider_channels.everything_sender;
    let plugin_sender = provider_channels.plugin_sender;
    let native_sender = provider_channels.native_sender;
    if settings.auto_enable_everything {
        match everything::start_background_if_installed() {
            Ok(outcome) => {
                everything_installed.set(outcome.is_installed());
                everything_status.set(outcome.status_message());
            }
            Err(error) => {
                everything_status.set(error);
            }
        }
    } else {
        everything_status.set(t!("everything.auto_enable_disabled").into_owned());
    }
    let provider_workers = ProviderWorkers::new(
        application_sender,
        everything_sender,
        plugin_sender,
        native_sender,
    );
    let application_worker = provider_workers.applications;
    let everything_worker = provider_workers.everything;
    let plugin_worker = provider_workers.plugins;
    let native_plugin_worker = provider_workers.native_plugins;

    let settings_for_activation = Arc::clone(&shared_settings);
    let position_for_activation = window_position.clone();
    let cursor_visibility_for_activation = cursor_visibility.clone();
    let size_for_activation = window_size.clone();
    let query_for_activation = query;
    let results_for_activation = results;
    let selected_id_for_activation = selected_id;
    let selected_index_for_activation = selected_index;
    let selection_touched_for_activation = selection_touched;
    let show_results_for_activation = show_results;
    let history_mode_for_activation = history_mode;
    let history_cursor_for_activation = history_cursor;
    let action_mode_for_activation = action_mode;
    let action_index_for_activation = action_index;
    let action_items_for_activation = action_items;
    let inline_completion_for_activation = inline_completion;
    let scroll_request_for_activation = scroll_request_for_rows;
    let settings_visible_for_activation = settings_visible;
    let activation_handle = app.hotkey_handle(activation_hotkey, move |ctx| {
        let settings = settings_for_activation
            .read()
            .map(|settings| settings.clone())
            .unwrap_or_default();
        if !should_suppress_activation(&settings, fullscreen::foreground_is_fullscreen()) {
            // Clear before toggling visibility. The previous implementation did
            // this only from on_window_hide, which allowed the old query frame to
            // survive in the compositor until the next repaint after re-show.
            if settings.clear_query_on_activation {
                query_for_activation.set(String::new());
                results_for_activation.set(Vec::new());
                selected_id_for_activation.set(String::new());
                selected_index_for_activation.set(0);
                selection_touched_for_activation.set(false);
                show_results_for_activation.set(false);
                history_mode_for_activation.set(false);
                history_cursor_for_activation.set(None);
                action_mode_for_activation.set(false);
                action_index_for_activation.set(0);
                action_items_for_activation.set(Vec::new());
                inline_completion_for_activation.set(String::new());
                scroll_request_for_activation.set(false);
                let (compact_width, compact_height) = launcher_window_geometry_with_sizes(
                    settings_visible_for_activation.get(),
                    false,
                    i32::from(launcher_width.get()),
                    i32::from(launcher_height.get()),
                );
                size_for_activation.set(compact_width, compact_height);
            }
            let (width, height) = launcher_window_geometry_with_sizes(
                settings_visible_for_activation.get(),
                show_results_for_activation.get(),
                i32::from(launcher_width.get()),
                i32::from(launcher_height.get()),
            );
            request_monitor_position(
                &position_for_activation,
                settings.monitor_preference,
                width,
                height,
            );
            cursor_visibility_for_activation.show();
            // 切换方向按窗口「实际可见性」判定，而不是按前台归属：launcher 自隐藏后
            // （例如刚启动一个不前置窗口或瞬间退出的程序）系统可能仍把它视为前台
            // 窗口，此时按前台判断会误判为「已显示」而只发 hide，用户再按 Alt+Space
            // 就唤不起窗口。可见性快照由 windui 平台层在派发 WM_HOTKEY 时提供
            // （`HotkeyCtx` 刻意不持有 hwnd，见 vendor/windui 的借用纪律）。
            if should_show_launcher(ctx.window_visible()) {
                ctx.show_window();
            } else {
                ctx.hide_window();
            }
        }
    });

    let activation_handle_for_recorder = activation_handle.clone();
    let activation_recording_for_keys = activation_recording;
    let activation_display_for_keys = activation_display;
    let activation_key_for_keys = activation_key;
    let activation_ctrl_for_keys = activation_ctrl;
    let activation_alt_for_keys = activation_alt;
    let activation_shift_for_keys = activation_shift;
    let activation_meta_for_keys = activation_meta;
    let query_for_keys = query;
    let query_caret_position_for_keys = query_caret_position;
    let results_for_keys = results;
    let selected_id_for_keys = selected_id;
    let selected_index_for_keys = selected_index;
    let scroll_request_for_keys = scroll_request_for_rows;
    let selection_touched_for_keys = selection_touched;
    let action_mode_for_keys = action_mode;
    let action_index_for_keys = action_index;
    let action_items_for_keys = action_items;
    let action_scroll_pending_for_keys = action_scroll_pending;
    let recycle_bin_confirmation_for_keys = recycle_bin_confirmation;
    let plugin_actions_for_keys = Rc::clone(&plugin_actions);
    let inline_completion_for_keys = inline_completion;
    let settings_visible_for_keys = settings_visible;
    let query_history_for_keys = Rc::clone(&query_history);
    let history_mode_for_keys = history_mode;
    let history_cursor_for_keys = history_cursor;
    let settings_for_history_for_keys = Arc::clone(&shared_settings);
    let settings_for_priority_for_keys = Arc::clone(&shared_settings);
    let priorities_for_keys = priorities;
    let providers_for_keys = Rc::clone(&provider_results);
    let query_for_priority_keys = query;
    let window_op_for_keys = window_op.clone();
    let cursor_visibility_for_keys = cursor_visibility.clone();
    let size_for_keys = window_size.clone();
    let show_results_for_keys = show_results;
    let settings_for_game_hotkey = Arc::clone(&shared_settings);
    let game_mode_for_hotkey = game_mode;
    let game_mode_status_for_hotkey = game_mode_status;
    app = app.hotkey(hotkeys::game_mode_toggle_hotkey(), move |_| {
        let enabled = !game_mode_for_hotkey.get();
        set_game_mode(
            &settings_for_game_hotkey,
            game_mode_for_hotkey,
            game_mode_status_for_hotkey,
            enabled,
        );
    });

    app = app.on_key(move |event: KeyEvent| {
        if activation_recording_for_keys.get() {
            if event.pressed {
                if let Some(configuration) =
                    hotkeys::capture_config(&event, alt_key_is_down(), hotkeys::meta_key_is_down())
                {
                    activation_key_for_keys.set(configuration.key.clone());
                    activation_ctrl_for_keys.set(configuration.ctrl);
                    activation_alt_for_keys.set(configuration.alt);
                    activation_shift_for_keys.set(configuration.shift);
                    activation_meta_for_keys.set(configuration.meta);
                    activation_display_for_keys.set(hotkeys::display_config(&configuration));
                    activation_recording_for_keys.set(false);
                    activation_handle_for_recorder.set_enabled(true);
                }
            }
            return true;
        }
        if !event.pressed || settings_visible_for_keys.get() {
            return false;
        }
        let alt_down = alt_key_is_down();
        if !event.ctrl
            && !alt_down
            && matches!(event.key, Key::Char(_) | Key::Backspace | Key::Delete)
        {
            history_cursor_for_keys.set(None);
            cursor_visibility_for_keys.hide();
        }
        if event.ctrl
            && (event.shift || shift_key_is_down())
            && matches!(
                event.key,
                Key::Other(0x43) | Key::Char('c') | Key::Char('C')
            )
        {
            return handle_copy_shortcut(
                true,
                event.shift,
                shift_key_is_down(),
                &results_for_keys.get(),
                selected_id_for_keys,
                selected_index_for_keys,
            );
        }
        if event.ctrl
            && !event.shift
            && !shift_key_is_down()
            && matches!(
                event.key,
                Key::Other(0x43) | Key::Char('c') | Key::Char('C')
            )
        {
            return handle_copy_shortcut(
                false,
                event.shift,
                shift_key_is_down(),
                &results_for_keys.get(),
                selected_id_for_keys,
                selected_index_for_keys,
            );
        }
        if event.ctrl && matches!(event.key, Key::Char('h') | Key::Char('H')) {
            let history = query_history_for_keys.borrow();
            return open_history_mode(
                &history,
                query_for_keys,
                history_mode_for_keys,
                history_cursor_for_keys,
                action_mode_for_keys,
                action_items_for_keys,
                inline_completion_for_keys,
                selected_index_for_keys,
                selected_id_for_keys,
                results_for_keys,
                show_results_for_keys,
                &size_for_keys,
                launcher_width,
                launcher_height,
            );
        }
        let query = query_for_keys.get();
        let history = query_history_for_keys.borrow();
        if alt_down && !event.ctrl && !event.shift && matches!(event.key, Key::Up | Key::Down) {
            return cycle_query_history(
                &history,
                event.key,
                history_cursor_for_keys,
                history_mode_for_keys,
                query_for_keys,
            );
        }
        if !history_mode_for_keys.get()
            && event.key == Key::Up
            && !alt_down
            && !event.ctrl
            && !event.shift
            && query.trim().is_empty()
        {
            if let Some(latest) = history.last() {
                history_cursor_for_keys.set(Some(history.len() - 1));
                query_for_keys.set(latest.clone());
                return true;
            }
        }
        drop(history);
        if query.trim().is_empty() {
            return false;
        }
        let current_results = results_for_keys.get();
        if current_results.is_empty() {
            return false;
        }

        if event.ctrl && event.key == Key::Tab {
            let suffix = inline_completion_for_keys.get();
            if !suffix.is_empty() {
                query_for_keys.set(format!("{query}{suffix}"));
                return true;
            }
        }

        // Match Flow Launcher: plain Tab selects the next result, while
        // Shift+Tab selects the previous result. Ctrl+Tab remains reserved
        // for inline completion above.
        if !event.ctrl && !alt_down && event.key == Key::Tab {
            let count = current_results.len();
            let next = if event.shift {
                selected_index_for_keys
                    .get()
                    .checked_sub(1)
                    .unwrap_or(count - 1)
            } else {
                (selected_index_for_keys.get() + 1) % count
            };
            selection_touched_for_keys.set(true);
            selected_index_for_keys.set(next);
            if let Some(result) = current_results.get(next) {
                selected_id_for_keys.set(result.id.clone());
            }
            // Keep the existing row tree intact while changing only selection.
            // Rebuilding the DynList here resets row geometry and prevents the
            // pending scroll request from bringing the next result into view.
            request_scroll(scroll_request_for_keys);
            return true;
        }

        if event.key == Key::Enter && alt_key_is_down() {
            return handle_open_location_shortcut(
                history_mode_for_keys,
                query_for_keys,
                &query_history_for_keys,
                &settings_for_history_for_keys,
                &current_results,
                selected_id_for_keys,
                selected_index_for_keys,
            );
        }

        if action_mode_for_keys.get() {
            return handle_action_mode(
                event.key,
                &ActionKeyContext {
                    action_mode: action_mode_for_keys,
                    action_index: action_index_for_keys,
                    action_items: action_items_for_keys,
                    action_scroll_pending: action_scroll_pending_for_keys,
                    history_mode: history_mode_for_keys,
                    query: query_for_keys,
                    query_history: Rc::clone(&query_history_for_keys),
                    current_results: current_results.clone(),
                    selected_id: selected_id_for_keys,
                    selected_index: selected_index_for_keys,
                    settings: Arc::clone(&settings_for_priority_for_keys),
                    priorities: priorities_for_keys,
                    providers: Rc::clone(&providers_for_keys),
                    priority_query: query_for_priority_keys,
                    results: results_for_keys,
                    window_op: window_op_for_keys.clone(),
                    window_size: size_for_keys.clone(),
                    launcher_width,
                    launcher_height,
                },
            );
        }

        if is_run_as_admin_key(&event) {
            return handle_run_as_admin_shortcut(
                query_for_keys,
                &query_history_for_keys,
                &settings_for_history_for_keys,
                &current_results,
                selected_id_for_keys,
                selected_index_for_keys,
                &window_op_for_keys,
            );
        }
        match event.key {
            Key::Up | Key::Down => handle_result_navigation(
                event.key,
                &current_results,
                selected_id_for_keys,
                selected_index_for_keys,
                selection_touched_for_keys,
                scroll_request_for_keys,
            ),
            Key::Right => handle_action_entry(
                query_for_keys,
                query_caret_position_for_keys,
                &current_results,
                selected_id_for_keys,
                selected_index_for_keys,
                &plugin_actions_for_keys,
                action_items_for_keys,
                action_index_for_keys,
                action_scroll_pending_for_keys,
                action_mode_for_keys,
                show_results_for_keys,
                &size_for_keys,
                launcher_width,
            ),
            Key::Enter => handle_enter_key(&EnterKeyContext {
                history_mode: history_mode_for_keys,
                query: query_for_keys,
                query_history: Rc::clone(&query_history_for_keys),
                settings: Arc::clone(&settings_for_history_for_keys),
                current_results: current_results.clone(),
                selected_id: selected_id_for_keys,
                selected_index: selected_index_for_keys,
                recycle_bin_confirmation: recycle_bin_confirmation_for_keys,
                settings_visible: settings_visible_for_keys,
                window_size: size_for_keys.clone(),
                window_op: window_op_for_keys.clone(),
                plugin_actions: Rc::clone(&plugin_actions_for_keys),
            }),
            _ => false,
        }
    });

    let settings_for_tray_toggle = Arc::clone(&shared_settings);
    let game_mode_for_tray = game_mode;
    let game_status_for_tray = game_mode_status;
    let settings_visible_for_tray = settings_visible;
    let settings_visible_for_left_click = settings_visible;
    let show_results_for_left_click = show_results;
    let size_for_left_click = window_size.clone();
    let position_for_left_click = window_position.clone();
    let settings_for_left_click = Arc::clone(&shared_settings);
    let show_results_for_tray = show_results;
    let size_for_tray = window_size.clone();
    let position_for_tray = window_position.clone();
    let settings_for_tray_position = Arc::clone(&shared_settings);
    let size_for_settings = window_size.clone();
    let position_for_settings = window_position.clone();
    let settings_for_settings_position = Arc::clone(&shared_settings);
    let tray = Tray::new()
        .tooltip("Flux Launcher CN")
        .icon_rgba(16, 16, &tray_icon())
        .on_left_click(move |ctx| {
            settings_visible_for_left_click.set(false);
            let height = if show_results_for_left_click.get() {
                launcher_height.get() as i32
            } else {
                COMPACT_WINDOW_HEIGHT
            };
            if let Ok(settings) = settings_for_left_click.read() {
                request_monitor_position(
                    &position_for_left_click,
                    settings.monitor_preference,
                    launcher_width.get() as i32,
                    height,
                );
            }
            size_for_left_click.set(launcher_width.get() as i32, height);
            ctx.show_window();
        })
        .menu(vec![
            TrayMenuItem::item(i18n_hub.tr(|| t!("tray.show").into_owned()), move |ctx| {
                settings_visible_for_tray.set(false);
                let height = if show_results_for_tray.get() {
                    launcher_height.get() as i32
                } else {
                    COMPACT_WINDOW_HEIGHT
                };
                if let Ok(settings) = settings_for_tray_position.read() {
                    request_monitor_position(
                        &position_for_tray,
                        settings.monitor_preference,
                        launcher_width.get() as i32,
                        height,
                    );
                }
                size_for_tray.set(launcher_width.get() as i32, height);
                ctx.show_window();
            }),
            TrayMenuItem::item(
                i18n_hub.tr(|| t!("tray.settings").into_owned()),
                move |ctx| {
                    settings_visible.set(true);
                    if let Ok(settings) = settings_for_settings_position.read() {
                        request_monitor_position(
                            &position_for_settings,
                            settings.monitor_preference,
                            SETTINGS_WINDOW_WIDTH,
                            SETTINGS_WINDOW_HEIGHT,
                        );
                    }
                    // Queue the Settings size before showing the hidden tray window. The
                    // first frame must not use the compact 72-DIP launcher height.
                    size_for_settings.set(SETTINGS_WINDOW_WIDTH, SETTINGS_WINDOW_HEIGHT);
                    ctx.show_window();
                    // Keep the request after show as well because the native show lifecycle
                    // may consume a stale compact-size request from the previous hide.
                    size_for_settings.set(SETTINGS_WINDOW_WIDTH, SETTINGS_WINDOW_HEIGHT);
                },
            ),
            TrayMenuItem::separator(),
            TrayMenuItem::check(
                i18n_hub.tr(|| t!("tray.game_mode").into_owned()),
                game_mode,
                move |_| {
                    let enabled = !game_mode_for_tray.get();
                    set_game_mode(
                        &settings_for_tray_toggle,
                        game_mode_for_tray,
                        game_status_for_tray,
                        enabled,
                    );
                },
            ),
            TrayMenuItem::separator(),
            TrayMenuItem::item(i18n_hub.tr(|| t!("tray.exit").into_owned()), |ctx| {
                ctx.quit()
            }),
        ]);

    let i18n_hub_for_apply = i18n_hub.clone();
    let settings_for_apply = Arc::clone(&shared_settings);
    let language_preference_for_apply = language_preference;
    let position_for_apply = window_position.clone();
    let activation_handle_for_apply = activation_handle.clone();
    let activation_handle_for_record_button = activation_handle.clone();
    let activation_recording_for_record_button = activation_recording;
    let activation_recording_for_apply = activation_recording;
    let activation_display_for_ui = activation_display;
    let activation_display_for_apply = activation_display;
    let game_mode_status_for_apply = game_mode_status;
    let settings_visible_for_apply = settings_visible;
    let size_for_apply = window_size.clone();
    let settings_for_clear_history = Arc::clone(&shared_settings);
    let history_for_clear = Rc::clone(&query_history);
    let history_cursor_for_clear = history_cursor;
    let start_with_windows_for_apply = start_with_windows;
    let update_checks_enabled_for_apply = update_checks_enabled;
    let update_interval_hours_for_apply = update_interval_hours;
    let auto_install_updates_for_apply = auto_install_updates;
    let update_status_for_apply = update_status;
    let update_available_for_install = update_available;
    let update_status_for_install = update_status;
    let update_installing_for_ui = update_installing;
    let update_install_progress_for_ui = update_install_progress;
    let update_install_sender_for_ui = update_install_sender.clone();
    let update_install_in_flight_for_ui = Rc::clone(&update_install_in_flight);
    let update_sender_for_apply = update_sender.clone();
    let update_sender_for_check_now = update_sender_for_apply.clone();
    let update_check_in_flight_for_apply = Rc::clone(&update_check_in_flight);
    let update_check_in_flight_for_check_now = Rc::clone(&update_check_in_flight);
    let auto_enable_everything_for_apply = auto_enable_everything;
    let obsidian_enabled_for_apply = obsidian_enabled;
    let obsidian_alias_for_apply = obsidian_alias;
    let google_enabled_for_apply = google_enabled;
    let google_alias_for_apply = google_alias;
    let system_commands_enabled_for_apply = system_commands_enabled;
    let everything_status_for_apply = everything_status;
    let cancel_settings = {
        let settings = Arc::clone(&shared_settings);
        let activation_handle = activation_handle.clone();
        let i18n_hub = i18n_hub.clone();
        let window_size = window_size.clone();
        let window_position = window_position.clone();

        Rc::new(move || {
            let Ok(saved) = settings.read() else {
                return;
            };
            let saved = saved.clone();

            settings_visible.set(false);
            language_preference.set(language_preference_index(saved.language));
            apply_configured_locale(saved.language);
            i18n_hub.refresh();

            activation_key.set(saved.activation_hotkey.key.clone());
            activation_ctrl.set(saved.activation_hotkey.ctrl);
            activation_alt.set(saved.activation_hotkey.alt);
            activation_shift.set(saved.activation_hotkey.shift);
            activation_meta.set(saved.activation_hotkey.meta);
            activation_display.set(hotkeys::display_config(&saved.activation_hotkey));
            activation_recording.set(false);
            activation_handle.set(hotkeys::activation_hotkey(&saved.activation_hotkey));
            activation_handle.set_enabled(true);

            ignore_fullscreen.set(saved.ignore_hotkeys_in_fullscreen);
            game_mode.set(saved.game_mode);
            game_mode_status.set(game_mode_label(saved.game_mode));
            smooth_caret.set(saved.smooth_caret);
            switch_to_english_layout.set(saved.switch_to_english_layout);
            use_system_accent.set(saved.use_system_accent);
            custom_selection_color.set(selection_color_hex(saved.custom_selection_color));
            selection_color.set(selection_color_for_settings(&saved));

            launcher_width.set(saved.launcher_width);
            launcher_height.set(saved.launcher_height);
            launcher_width_input.set(saved.launcher_width.to_string());
            launcher_height_input.set(saved.launcher_height.to_string());
            launcher_width_slider.set(dimension_slider_fraction(
                saved.launcher_width,
                MIN_LAUNCHER_WIDTH,
                MAX_LAUNCHER_WIDTH,
            ));
            launcher_height_slider.set(dimension_slider_fraction(
                saved.launcher_height,
                MIN_LAUNCHER_HEIGHT,
                MAX_LAUNCHER_HEIGHT,
            ));
            launcher_preview_text.set(
                t!(
                    "settings.visual.client_area",
                    width = saved.launcher_width,
                    height = saved.launcher_height
                )
                .into_owned(),
            );

            clear_query_on_activation.set(saved.clear_query_on_activation);
            start_with_windows.set(saved.start_with_windows);
            auto_enable_everything.set(saved.auto_enable_everything);
            update_checks_enabled.set(saved.update_checks_enabled);
            update_interval_hours.set(saved.update_interval_hours.to_string());
            auto_install_updates.set(saved.auto_install_updates);
            obsidian_enabled.set(saved.obsidian_enabled);
            obsidian_alias.set(saved.obsidian_alias.clone());
            google_enabled.set(saved.google_enabled);
            google_alias.set(saved.google_alias.clone());
            system_commands_enabled.set(saved.system_commands_enabled);
            monitor_preference.set(monitor_preference_index(saved.monitor_preference));
            // 设置变更（例如切换语言）后刷新状态文案；这里同样使用低成本刷新，
            // 如实反映 Everything 当前的安装与 IPC 状态。
            refresh_everything_status_cheap(
                auto_enable_everything,
                everything_installed,
                everything_status,
            );

            let target_height = if show_results.get() {
                i32::from(saved.launcher_height)
            } else {
                COMPACT_WINDOW_HEIGHT
            };
            request_monitor_position(
                &window_position,
                saved.monitor_preference,
                i32::from(saved.launcher_width),
                target_height,
            );
            window_size.set(i32::from(saved.launcher_width), target_height);
        }) as Rc<dyn Fn()>
    };
    let priority_list = settings_view::priority_list(
        priorities,
        results,
        Rc::clone(&provider_results),
        query,
        Arc::clone(&shared_settings),
    );
    let priorities_empty = priorities_empty(priorities);

    let visual_preview_generation_for_width_reset = visual_preview_generation;
    let visual_preview_generation_for_height_reset = visual_preview_generation;

    // Settings shares the same continuous Acrylic surface as the launcher.
    // Do not add a dark card here: it hides the blur and creates the old opaque
    // search-style slab inside the transparent window.
    let settings_panel = Element::col()
        .fill()
        .padding(24)
        .spacing(14)
        .corner(20.0)
        .bg(Color::rgba(0, 0, 0, 0))
        .border(Color::rgba(0, 0, 0, 0), 0)
        .child(settings_view::settings_header(
            settings_tab,
            i18n_hub.clone(),
            Rc::clone(&cancel_settings),
        ))
        .child(
            Element::scroll()
                .weight(1.0)
                .visible_when(move || settings_tab.get() == 0)
                .child(
                    Element::col()
                        .width_match()
                        .spacing(12)
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.activation_key").into_owned()),
                            Element::col()
                                .width_match()
                                .spacing(6)
                                .child(
                                    Element::row()
                                        .width_match()
                                        .spacing(8)
                                        .child(
                                            Element::label_signal(activation_display_for_ui)
                                                .width_match()
                                                .padding_xy(10, 8)
                                                .bg(Color::rgba(255, 255, 255, 24))
                                                .corner(8.0),
                                        )
                                        .child(
                                            Element::button(
                                                i18n_hub
                                                    .tr(|| t!("settings.record_key").into_owned()),
                                            )
                                            .neutral()
                                            .on_click(
                                                move |ctx| {
                                                    activation_recording_for_record_button
                                                        .set(true);
                                                    activation_handle_for_record_button
                                                        .set_enabled(false);
                                                    ctx.toast_ok(t!("settings.press_desired_key"));
                                                },
                                            ),
                                        ),
                                )
                                .child(
                                    Element::label(
                                        i18n_hub.tr(|| t!("settings.record_hint").into_owned()),
                                    )
                                    .font_size(11.0)
                                    .fg(Color::rgba(235, 241, 255, 170))
                                    .visible_when(move || {
                                        activation_recording_for_record_button.get()
                                    }),
                                ),
                        ))
                        .child(
                            Element::row()
                                .width_match()
                                .spacing(10)
                                .child(Element::checkbox(
                                    i18n_hub.tr(|| t!("settings.modifier.ctrl").into_owned()),
                                    activation_ctrl,
                                ))
                                .child(Element::checkbox(
                                    i18n_hub.tr(|| t!("settings.modifier.alt").into_owned()),
                                    activation_alt,
                                ))
                                .child(Element::checkbox(
                                    i18n_hub.tr(|| t!("settings.modifier.shift").into_owned()),
                                    activation_shift,
                                ))
                                .child(Element::checkbox(
                                    i18n_hub.tr(|| t!("settings.modifier.windows").into_owned()),
                                    activation_meta,
                                )),
                        )
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.fullscreen_protection").into_owned()),
                            Element::checkbox(
                                i18n_hub
                                    .tr(|| t!("settings.fullscreen_protection_desc").into_owned()),
                                ignore_fullscreen,
                            ),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.game_mode").into_owned()),
                            Element::checkbox(
                                i18n_hub.tr(|| t!("settings.game_mode_desc").into_owned()),
                                game_mode,
                            ),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.keyboard_layout").into_owned()),
                            Element::checkbox(
                                i18n_hub.tr(|| t!("settings.keyboard_layout_desc").into_owned()),
                                switch_to_english_layout,
                            ),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.query_on_activation").into_owned()),
                            Element::checkbox(
                                i18n_hub
                                    .tr(|| t!("settings.query_on_activation_desc").into_owned()),
                                clear_query_on_activation,
                            ),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.windows_startup").into_owned()),
                            Element::checkbox(
                                i18n_hub.tr(|| t!("settings.windows_startup_desc").into_owned()),
                                start_with_windows,
                            ),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.language").into_owned()),
                            Element::dropdown_signal(
                                i18n_hub.tr_vec(|| {
                                    vec![
                                        t!("settings.language_options.follow_system").into_owned(),
                                        t!("settings.language_options.english").into_owned(),
                                        t!("settings.language_options.chinese").into_owned(),
                                    ]
                                }),
                                language_preference,
                            )
                            .on_dropdown_change({
                                let i18n_hub = i18n_hub.clone();
                                move |ctx, index| {
                                    let target_lang = language_preference_from_index(index);
                                    apply_configured_locale(target_lang);
                                    i18n_hub.refresh();
                                    ctx.mark_dirty_all();
                                }
                            }),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.open_launcher_on").into_owned()),
                            Element::col()
                                .spacing(6)
                                .child(Element::radio(
                                    i18n_hub.tr(|| t!("settings.monitor.primary").into_owned()),
                                    monitor_preference,
                                    0,
                                ))
                                .child(Element::radio(
                                    i18n_hub.tr(|| t!("settings.monitor.cursor").into_owned()),
                                    monitor_preference,
                                    1,
                                ))
                                .child(Element::radio(
                                    i18n_hub.tr(|| t!("settings.monitor.foreground").into_owned()),
                                    monitor_preference,
                                    2,
                                )),
                        ))
                        .child(
                            Element::col()
                                .width_match()
                                .spacing(8)
                                .child(Element::field_signal(
                                    i18n_hub.tr(|| t!("settings.updates").into_owned()),
                                    Element::checkbox(
                                        i18n_hub.tr(|| t!("settings.updates_desc").into_owned()),
                                        update_checks_enabled,
                                    ),
                                ))
                                .child(
                                    Element::row()
                                        .width_match()
                                        .spacing(8)
                                        .child(
                                            Element::text_input(update_interval_hours, "24")
                                                .width_match(),
                                        )
                                        .child(
                                            Element::label(i18n_hub.tr(|| {
                                                t!("settings.hours_between_checks").into_owned()
                                            }))
                                            .font_size(11.0),
                                        ),
                                )
                                .child(
                                    Element::row()
                                        .width_match()
                                        .spacing(8)
                                        .child(
                                            Element::label(
                                                i18n_hub.tr(|| {
                                                    t!("settings.update_action").into_owned()
                                                }),
                                            )
                                            .width_match(),
                                        )
                                        .child(
                                            Element::label(t!(
                                                "settings.current_version",
                                                version = CURRENT_VERSION
                                            ))
                                            .font_size(11.0)
                                            .fg(Color::rgba(235, 241, 255, 190)),
                                        ),
                                )
                                .child(Element::checkbox(
                                    i18n_hub
                                        .tr(|| t!("settings.auto_install_updates").into_owned()),
                                    auto_install_updates,
                                ))
                                .child(
                                    Element::row()
                                        .width_match()
                                        .spacing(8)
                                        .child(
                                            Element::label_signal(update_status)
                                                .font_size(11.0)
                                                .fg(Color::rgba(235, 241, 255, 190))
                                                .max_lines(2)
                                                .truncate(Truncate::End)
                                                .width_match(),
                                        )
                                        .child(
                                            Element::button(
                                                i18n_hub.tr(|| {
                                                    t!("settings.check_updates").into_owned()
                                                }),
                                            )
                                            .on_click(
                                                move |ctx| {
                                                    update_status_for_apply.set(
                                                        t!("updater.checking_github").into_owned(),
                                                    );
                                                    request_update_check(
                                                        update_sender_for_check_now.clone(),
                                                        &update_check_in_flight_for_check_now,
                                                    );
                                                    ctx.toast_ok(t!("updater.checking_toast"));
                                                },
                                            ),
                                        )
                                        .child(
                                            Element::button(
                                                i18n_hub
                                                    .tr(|| t!("settings.install_now").into_owned()),
                                            )
                                            .visible_when(move || {
                                                update_available_for_install.get().is_some()
                                                    && !update_installing_for_ui.get()
                                            })
                                            .on_click(
                                                move |ctx| {
                                                    if update_installing_for_ui.get() {
                                                        return;
                                                    }
                                                    if let Some(update) =
                                                        update_available_for_install.get()
                                                    {
                                                        update_installing_for_ui.set(true);
                                                        update_install_progress_for_ui.set(None);
                                                        update_status_for_install.set(
                                                            t!(
                                                                "updater.preparing_download",
                                                                version = update.version
                                                            )
                                                            .into_owned(),
                                                        );
                                                        if !request_update_install(
                                                            update,
                                                            update_install_sender_for_ui.clone(),
                                                            &update_install_in_flight_for_ui,
                                                            updater::RelaunchMode::Visible,
                                                        ) {
                                                            update_installing_for_ui.set(false);
                                                            update_status_for_install.set(
                                                                t!("updater.already_installing")
                                                                    .into_owned(),
                                                            );
                                                            ctx.toast_ok(t!(
                                                                "updater.already_installing"
                                                            ));
                                                        }
                                                    }
                                                },
                                            ),
                                        ),
                                ),
                        )
                        .child(
                            Element::row()
                                .width_match()
                                .spacing(10)
                                .child(
                                    Element::label(
                                        i18n_hub
                                            .tr(|| t!("settings.query_history_hint").into_owned()),
                                    )
                                    .font_size(11.0)
                                    .fg(Color::rgba(235, 241, 255, 175))
                                    .width_match(),
                                )
                                .child(
                                    Element::button(
                                        i18n_hub.tr(|| t!("settings.clear_history").into_owned()),
                                    )
                                    .on_click(move |ctx| {
                                        if let Ok(mut settings) = settings_for_clear_history.write()
                                        {
                                            settings.clear_query_history();
                                            let _ = save_settings(&settings);
                                        }
                                        history_for_clear.borrow_mut().clear();
                                        history_cursor_for_clear.set(None);
                                        ctx.toast_ok(t!("settings.history_cleared"));
                                    }),
                                ),
                        )
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.native_plugins_hint").into_owned()),
                            )
                            .font_size(12.0)
                            .fg(Color::rgba(235, 241, 255, 160)),
                        )
                        .child(
                            Element::button(i18n_hub.tr(|| t!("settings.apply").into_owned()))
                                .on_click(move |ctx| {
                                    let duration = caret_duration
                                        .get()
                                        .trim()
                                        .parse::<u16>()
                                        .unwrap_or(95)
                                        .clamp(60, 160);
                                    let configuration = HotkeyConfig {
                                        ctrl: activation_ctrl.get(),
                                        alt: activation_alt.get(),
                                        shift: activation_shift.get(),
                                        meta: activation_meta.get(),
                                        key: activation_key.get(),
                                    };
                                    let custom_color =
                                        parse_selection_color(&custom_selection_color.get())
                                            .unwrap_or(0x4c8bf4);
                                    let configured_width = parse_dimension_input(
                                        &launcher_width_input.get(),
                                        MIN_LAUNCHER_WIDTH,
                                        MAX_LAUNCHER_WIDTH,
                                    )
                                    .unwrap_or(DEFAULT_LAUNCHER_WIDTH);
                                    let configured_height = parse_dimension_input(
                                        &launcher_height_input.get(),
                                        MIN_LAUNCHER_HEIGHT,
                                        MAX_LAUNCHER_HEIGHT,
                                    )
                                    .unwrap_or(DEFAULT_LAUNCHER_HEIGHT);
                                    if let Ok(mut settings) = settings_for_apply.write() {
                                        let previous_language = settings.language;
                                        settings.activation_hotkey = configuration;
                                        settings.ignore_hotkeys_in_fullscreen =
                                            ignore_fullscreen.get();
                                        settings.game_mode = game_mode.get();
                                        settings.smooth_caret = smooth_caret.get();
                                        settings.switch_to_english_layout =
                                            switch_to_english_layout.get();
                                        settings.use_system_accent = use_system_accent.get();
                                        settings.custom_selection_color = custom_color;
                                        settings.launcher_width = configured_width;
                                        settings.launcher_height = configured_height;
                                        settings.clear_query_on_activation =
                                            clear_query_on_activation.get();
                                        settings.start_with_windows =
                                            start_with_windows_for_apply.get();
                                        settings.update_checks_enabled =
                                            update_checks_enabled_for_apply.get();
                                        settings.update_interval_hours =
                                            update_interval_hours_for_apply
                                                .get()
                                                .trim()
                                                .parse::<u32>()
                                                .unwrap_or(24)
                                                .clamp(1, 168);
                                        settings.auto_install_updates =
                                            auto_install_updates_for_apply.get();
                                        update_interval_hours_for_apply
                                            .set(settings.update_interval_hours.to_string());
                                        settings.auto_enable_everything =
                                            auto_enable_everything_for_apply.get();
                                        settings.obsidian_enabled =
                                            obsidian_enabled_for_apply.get();
                                        settings.obsidian_alias = obsidian_alias_for_apply.get();
                                        settings.google_enabled = google_enabled_for_apply.get();
                                        settings.google_alias = google_alias_for_apply.get();
                                        settings.system_commands_enabled =
                                            system_commands_enabled_for_apply.get();
                                        settings.monitor_preference =
                                            monitor_preference_from_index(monitor_preference.get());
                                        settings.language = language_preference_from_index(
                                            language_preference_for_apply.get(),
                                        );
                                        settings.smooth_caret_duration_ms = duration;
                                        settings.normalize();
                                        activation_recording_for_apply.set(false);
                                        activation_display_for_apply.set(hotkeys::display_config(
                                            &settings.activation_hotkey,
                                        ));
                                        selection_color
                                            .set(selection_color_for_settings(&settings));
                                        custom_selection_color.set(selection_color_hex(
                                            settings.custom_selection_color,
                                        ));
                                        launcher_width.set(settings.launcher_width);
                                        launcher_height.set(settings.launcher_height);
                                        launcher_width_input
                                            .set(settings.launcher_width.to_string());
                                        launcher_height_input
                                            .set(settings.launcher_height.to_string());
                                        launcher_width_slider.set(dimension_slider_fraction(
                                            settings.launcher_width,
                                            MIN_LAUNCHER_WIDTH,
                                            MAX_LAUNCHER_WIDTH,
                                        ));
                                        launcher_height_slider.set(dimension_slider_fraction(
                                            settings.launcher_height,
                                            MIN_LAUNCHER_HEIGHT,
                                            MAX_LAUNCHER_HEIGHT,
                                        ));
                                        launcher_preview_text.set(
                                            t!(
                                                "settings.visual.client_area",
                                                width = settings.launcher_width,
                                                height = settings.launcher_height
                                            )
                                            .into_owned(),
                                        );
                                        activation_handle_for_apply.set(
                                            hotkeys::activation_hotkey(&settings.activation_hotkey),
                                        );
                                        activation_handle_for_apply.set_enabled(true);
                                        game_mode_status_for_apply
                                            .set(game_mode_label(settings.game_mode));
                                        if settings.auto_enable_everything {
                                            match everything::start_background_if_installed() {
                                                Ok(outcome) => {
                                                    everything_installed
                                                        .set(outcome.is_installed());
                                                    everything_status_for_apply
                                                        .set(outcome.status_message());
                                                }
                                                Err(error) => {
                                                    everything_status_for_apply.set(error)
                                                }
                                            }
                                        } else {
                                            everything_status_for_apply.set(
                                                t!("everything.auto_enable_disabled").into_owned(),
                                            );
                                        }
                                        let _ = save_settings(&settings);
                                        apply_configured_locale(settings.language);
                                        if previous_language != settings.language {
                                            launcher_preview_text.set(
                                                t!(
                                                    "settings.visual.client_area",
                                                    width = launcher_width.get(),
                                                    height = launcher_height.get()
                                                )
                                                .into_owned(),
                                            );
                                            i18n_hub_for_apply.refresh();
                                        }
                                        if let Err(error) =
                                            startup::set_enabled(settings.start_with_windows)
                                        {
                                            ctx.toast_ok(t!(
                                                "settings.startup_failed",
                                                error = error
                                            ));
                                        }
                                        if settings.update_checks_enabled
                                            && update_check_due(&settings)
                                        {
                                            update_status_for_apply
                                                .set(t!("updater.checking_github").into_owned());
                                            request_update_check(
                                                update_sender_for_apply.clone(),
                                                &update_check_in_flight_for_apply,
                                            );
                                        }
                                    }
                                    settings_visible_for_apply.set(false);
                                    let selected_preference =
                                        monitor_preference_from_index(monitor_preference.get());
                                    let applied_width = launcher_width.get() as i32;
                                    let applied_height = launcher_height.get() as i32;
                                    let target_height = if show_results.get() {
                                        applied_height
                                    } else {
                                        COMPACT_WINDOW_HEIGHT
                                    };
                                    request_monitor_position(
                                        &position_for_apply,
                                        selected_preference,
                                        applied_width,
                                        target_height,
                                    );
                                    size_for_apply.set(applied_width, target_height);
                                    ctx.show_window();
                                    ctx.toast_ok(t!("settings.applied"));
                                }),
                        ),
                ),
        )
        .child(
            Element::scroll()
                .weight(1.0)
                .visible_when(move || settings_tab.get() == 3)
                .child(settings_view::everything_settings(
                    settings_view::EverythingSettingsContext {
                        i18n_hub: i18n_hub.clone(),
                        settings: Arc::clone(&shared_settings),
                        auto_enable: auto_enable_everything,
                        installed: everything_installed,
                        status: everything_status,
                        detection: Arc::clone(&everything_detection),
                        obsidian_enabled,
                        obsidian_alias,
                        google_enabled,
                        google_alias,
                        system_commands_enabled,
                    },
                )),
        )
        .child(
            Element::scroll()
                .weight(1.0)
                .visible_when(move || settings_tab.get() == 1)
                .child(
                    Element::col()
                        .width_match()
                        .spacing(12)
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.visual.title").into_owned()),
                            )
                            .font_size(17.0)
                            .fg(Color::WHITE),
                        )
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.visual.preview_desc").into_owned()),
                            )
                            .font_size(11.0)
                            .fg(Color::rgba(235, 241, 255, 180))
                            .max_lines(3)
                            .truncate(Truncate::End),
                        )
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.smooth_caret").into_owned()),
                            Element::row()
                                .width_match()
                                .spacing(8)
                                .child(
                                    Element::checkbox(
                                        i18n_hub
                                            .tr(|| t!("settings.smooth_caret_desc").into_owned()),
                                        smooth_caret,
                                    )
                                    .width_match(),
                                )
                                .child(Element::text_input(caret_duration, "95").width(76))
                                .child(Element::label("ms").font_size(11.0)),
                        ))
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.visual.selection_color").into_owned()),
                            Element::checkbox(
                                i18n_hub
                                    .tr(|| t!("settings.visual.use_system_accent").into_owned()),
                                use_system_accent,
                            ),
                        ))
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.visual.accent_hint").into_owned()),
                            )
                            .font_size(10.0)
                            .fg(Color::rgba(235, 241, 255, 150))
                            .max_lines(2)
                            .truncate(Truncate::End),
                        )
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.visual.preview_hint").into_owned()),
                            )
                            .font_size(10.0)
                            .fg(Color::rgba(235, 241, 255, 170))
                            .max_lines(2)
                            .truncate(Truncate::End),
                        )
                        .child(
                            Element::col()
                                .spacing(8)
                                .visible_when(move || !use_system_accent.get())
                                .child(
                                    Element::text_input(custom_selection_color, "#4C8BF4")
                                        .width_match(),
                                )
                                .child(selection_palette(custom_selection_color)),
                        )
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.visual.launcher_width").into_owned()),
                            Element::row()
                                .width_match()
                                .spacing(8)
                                .child(
                                    Element::slider(launcher_width_slider)
                                        .width(VISUAL_SLIDER_WIDTH),
                                )
                                .child(Element::text_input(launcher_width_input, "420").width(76))
                                .child(settings_view::launcher_width_reset_button(
                                    i18n_hub.clone(),
                                    launcher_width,
                                    launcher_height,
                                    launcher_width_input,
                                    launcher_width_slider,
                                    launcher_preview_text,
                                    visual_preview_generation_for_width_reset,
                                ))
                                .child(
                                    Element::label(
                                        i18n_hub.tr(|| t!("settings.visual.dip").into_owned()),
                                    )
                                    .font_size(11.0),
                                ),
                        ))
                        .child(
                            Element::label(i18n_hub.tr(|| {
                                t!(
                                    "settings.visual.safe_range",
                                    min = MIN_LAUNCHER_WIDTH,
                                    max = MAX_LAUNCHER_WIDTH
                                )
                                .into_owned()
                            }))
                            .font_size(10.0)
                            .fg(Color::rgba(235, 241, 255, 150)),
                        )
                        .child(Element::field_signal(
                            i18n_hub.tr(|| t!("settings.visual.results_height").into_owned()),
                            Element::row()
                                .width_match()
                                .spacing(8)
                                .child(
                                    Element::slider(launcher_height_slider)
                                        .width(VISUAL_SLIDER_WIDTH),
                                )
                                .child(Element::text_input(launcher_height_input, "382").width(76))
                                .child(settings_view::launcher_height_reset_button(
                                    i18n_hub.clone(),
                                    launcher_width,
                                    launcher_height,
                                    launcher_height_input,
                                    launcher_height_slider,
                                    launcher_preview_text,
                                    visual_preview_generation_for_height_reset,
                                ))
                                .child(
                                    Element::label(
                                        i18n_hub.tr(|| t!("settings.visual.dip").into_owned()),
                                    )
                                    .font_size(11.0),
                                ),
                        ))
                        .child(
                            Element::label(i18n_hub.tr(|| {
                                t!(
                                    "settings.visual.safe_range",
                                    min = MIN_LAUNCHER_HEIGHT,
                                    max = MAX_LAUNCHER_HEIGHT
                                )
                                .into_owned()
                            }))
                            .font_size(10.0)
                            .font_size(10.0)
                            .fg(Color::rgba(235, 241, 255, 150)),
                        )
                        .child(
                            Element::label_signal(launcher_preview_text)
                                .font_size(12.0)
                                .fg(Color::WHITE),
                        )
                        .child(
                            Element::label(
                                i18n_hub
                                    .tr(|| t!("settings.visual.native_preview_hint").into_owned()),
                            )
                            .font_size(11.0)
                            .fg(Color::rgba(235, 241, 255, 175))
                            .max_lines(2)
                            .truncate(Truncate::End),
                        )
                        .child(settings_view::visual_apply_button(
                            settings_view::VisualApplyContext {
                                i18n_hub: i18n_hub.clone(),
                                settings: Arc::clone(&shared_settings),
                                launcher_width,
                                launcher_height,
                                launcher_width_input,
                                launcher_height_input,
                                launcher_width_slider,
                                launcher_height_slider,
                                launcher_preview_text,
                                smooth_caret,
                                caret_duration,
                                settings_visible,
                                show_results,
                                window_size: window_size.clone(),
                                window_position: window_position.clone(),
                            },
                        )),
                ),
        )
        .child(
            Element::scroll()
                .weight(1.0)
                .visible_when(move || settings_tab.get() == 2)
                .child(
                    Element::col()
                        .width_match()
                        .spacing(10)
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.priorities.title").into_owned()),
                            )
                            .font_size(17.0)
                            .fg(Color::WHITE),
                        )
                        .child(
                            Element::label(
                                i18n_hub.tr(|| t!("settings.priorities.description").into_owned()),
                            )
                            .font_size(11.0)
                            .fg(Color::rgba(235, 241, 255, 180))
                            .max_lines(2)
                            .truncate(Truncate::End),
                        )
                        .child(priorities_empty)
                        .child(priority_list),
                ),
        );

    let launcher_page = Element::stack()
        .fill()
        .child(launcher_surface)
        .visible_when(move || !settings_visible.get());
    let settings_page = settings_view::settings_page(settings_panel, settings_ui);
    if std::env::var_os("FLUX_SMOKE_SETTINGS_UI").is_some() {
        eprintln!(
            "Settings UI contract: UpdateActionVersionLabel=Current version: {CURRENT_VERSION}; SmoothCaretTab=Visual; SmoothCaretGeneral=false"
        );
    }

    let content = Element::stack()
        .fill()
        .font_family(LAUNCHER_FONT_FAMILY)
        .child(launcher_page)
        .child(settings_page);

    let mut app = if startup_launch {
        app.start_hidden()
    } else {
        app
    };
    let second_instance_sender = app.channel::<()>(|ctx, ()| {
        ctx.show_window();
    });
    let second_instance_sender_for_callback = second_instance_sender.clone();
    let second_instance_window_op = window_op.clone();
    let shutdown_window_op = window_op.clone();
    if !single_instance_disabled {
        app = app.single_instance(SINGLE_INSTANCE_ID, move |argv| {
            if argv.iter().any(|arg| arg == "--shutdown") {
                // Uninstall is an application-controlled handoff: destroy the native
                // window and exit the event loop instead of applying hide_on_close.
                shutdown_window_op.quit();
                return;
            }
            // The native windui listener activates the window; queue Show as well so a
            // tray-hidden startup is made visible before the channel callback is drained.
            second_instance_window_op.show_window();
            let _ = second_instance_sender_for_callback.send(());
        });
    }

    app.tray(tray)
        .hide_on_close()
        .hide_on_deactivate()
        .on_window_deactivated(|| {
            trace_launch_event("window-deactivated");
        })
        .focus_first_control_on_show()
        // Keep the HWND background transparent so Acrylic/DWM remains visible
        // through the launcher and its install prompt instead of adding a solid slab.
        .bg(Color::TRANSPARENT)
        .centered()
        .frameless()
        .resizable(false)
        .min_size(MIN_LAUNCHER_WIDTH as i32, COMPACT_WINDOW_HEIGHT)
        .renderer(Renderer::Auto)
        .backdrop(Backdrop::Acrylic)
        .theme(launcher_theme())
        .content(content)
        .on_interval(SEARCH_INTERVAL, move |_ctx| {
            let current_width = width_for_interval.get();
            let current_height = height_for_interval.get();
            let settings_is_visible = settings_visible_for_interval.get();
            let prompt_is_visible = everything_prompt_visible_for_interval.get();
            let visual_tab_is_visible = settings_tab_for_interval.get() == 1;
            let plugins_tab_is_visible = settings_tab_for_interval.get() == 3;
            let visual_preview_is_visible = settings_is_visible && visual_tab_is_visible;
            if everything_plugins_smoke_for_interval
                && settings_is_visible
                && plugins_tab_is_visible
                && !everything_plugins_smoke_reported
            {
                let installed = everything_installed_for_interval.get();
                let auto_enable = auto_enable_everything_for_interval.get();
                let status = everything_status_for_interval.get();
                eprintln!(
                    "Everything Plugins UI: tab_visible=true everything_section=true auto_enable_checkbox=true status_label=true install_button_label=Install_Everything already_installed_label=Everything_is_already_installed auto_enable={} installed={} install_button_visible={} already_installed_visible={} status={}",
                    auto_enable,
                    installed,
                    !installed,
                    installed,
                    status.replace(' ', "_")
                );
                everything_plugins_smoke_reported = true;
            }
            if let Some(result) = everything_detection_for_interval.take_result() {
                // Everything 自动启用检测在后台线程完成；此处仅在 UI 线程回填信号。
                // 若用户在此期间关闭了自动启用，则以禁用文案为准，不回填陈旧检测结果。
                if !auto_enable_everything_for_interval.get() {
                    everything_status_for_interval
                        .set(t!("everything.auto_enable_disabled").into_owned());
                } else {
                    match result {
                        Ok(outcome) => {
                            everything_installed_for_interval.set(outcome.is_installed());
                            everything_status_for_interval.set(outcome.status_message());
                        }
                        Err(error) => everything_status_for_interval.set(error),
                    }
                }
            }
            if settings_is_visible && !last_settings_visible {
                if let Ok(settings) = settings_for_interval_geometry.read() {
                    request_monitor_position(
                        &position_for_interval,
                        settings.monitor_preference,
                        SETTINGS_WINDOW_WIDTH,
                        SETTINGS_WINDOW_HEIGHT,
                    );
                }
                size_for_interval.set(SETTINGS_WINDOW_WIDTH, SETTINGS_WINDOW_HEIGHT);
            }
            if prompt_is_visible != last_everything_prompt_visible {
                last_everything_prompt_visible = prompt_is_visible;
                let (prompt_width, prompt_height) = launcher_window_geometry_with_prompt(
                    settings_is_visible,
                    prompt_is_visible,
                    show_results_for_interval.get(),
                    width_for_interval.get() as i32,
                    height_for_interval.get() as i32,
                );
                if let Ok(settings) = settings_for_interval_geometry.read() {
                    request_monitor_position(
                        &position_for_interval,
                        settings.monitor_preference,
                        prompt_width,
                        prompt_height,
                    );
                }
                size_for_interval.set(prompt_width, prompt_height);
            }
            if visual_preview_is_visible {
                let preference = settings_for_interval_geometry
                    .read()
                    .map(|settings| settings.monitor_preference)
                    .unwrap_or(MonitorPreference::Cursor);
                let child_exited = visual_preview_process
                    .as_mut()
                    .is_some_and(|preview| !preview.is_alive());
                if child_exited {
                    visual_preview_process.take();
                    last_visual_preview_request = None;
                    last_visual_preview_locale.clear();
                }
                if let Some(preview) = visual_preview_process.as_mut() {
                    match preview.poll_ready() {
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("Could not ready visual preview: {error}");
                            visual_preview_process.take();
                            last_visual_preview_request = None;
                            last_visual_preview_locale.clear();
                        }
                    }
                }
                if visual_preview_process.is_none() {
                    let preview_width = i32::from(current_width);
                    let preview_height = i32::from(current_height);
                    let (preview_x, preview_y) =
                        visual_preview_position(preference, preview_width, preview_height);
                    match visual_preview::PreviewProcess::start(
                        preview_width,
                        preview_height,
                        preview_x,
                        preview_y,
                        &configured_locale(language_preference_from_index(language_preference.get())),
                    ) {
                        Ok(preview) => {
                            visual_preview_process = Some(preview);
                            last_visual_preview_request = None;
                            last_visual_preview_locale = configured_locale(
                                language_preference_from_index(language_preference.get()),
                            );
                        }
                        Err(error) => eprintln!("Could not start visual preview: {error}"),
                    }
                }
            } else if let Some(preview) = visual_preview_process.as_mut() {
                eprintln!(
                    "Visual preview closing because Settings/Visual is hidden: pid={}",
                    preview.pid()
                );
                visual_preview_process.take();
                last_visual_preview_request = None;
                last_visual_preview_locale.clear();
            }
            last_settings_visible = settings_is_visible;
            let slider_width = dimension_from_slider(
                width_slider_for_interval.get(),
                MIN_LAUNCHER_WIDTH,
                MAX_LAUNCHER_WIDTH,
            );
            let slider_height = dimension_from_slider(
                height_slider_for_interval.get(),
                MIN_LAUNCHER_HEIGHT,
                MAX_LAUNCHER_HEIGHT,
            );
            if visual_preview_smoke_for_interval {
                let control_state = (
                    width_for_interval.get(),
                    height_for_interval.get(),
                    (width_slider_for_interval.get() * 10_000.0).round() as u32,
                    (height_slider_for_interval.get() * 10_000.0).round() as u32,
                );
                if last_visual_control_state != Some(control_state) {
                    eprintln!(
                        "Visual control state: width={} height={} width_slider={} height_slider={}",
                        control_state.0, control_state.1, control_state.2, control_state.3
                    );
                    last_visual_control_state = Some(control_state);
                }
            }
            let typed_width = parse_dimension_input(
                &width_input_for_interval.get(),
                MIN_LAUNCHER_WIDTH,
                MAX_LAUNCHER_WIDTH,
            );
            let typed_height = parse_dimension_input(
                &height_input_for_interval.get(),
                MIN_LAUNCHER_HEIGHT,
                MAX_LAUNCHER_HEIGHT,
            );
            let next_width = typed_width
                .filter(|value| *value != current_width)
                .unwrap_or(if slider_width != current_width {
                    slider_width
                } else {
                    current_width
                });
            let next_height = typed_height
                .filter(|value| *value != current_height)
                .unwrap_or(if slider_height != current_height {
                    slider_height
                } else {
                    current_height
                });
            if next_width != current_width || next_height != current_height {
                width_for_interval.set(next_width);
                height_for_interval.set(next_height);
                width_input_for_interval.set(next_width.to_string());
                height_input_for_interval.set(next_height.to_string());
                width_slider_for_interval.set(dimension_slider_fraction(
                    next_width,
                    MIN_LAUNCHER_WIDTH,
                    MAX_LAUNCHER_WIDTH,
                ));
                height_slider_for_interval.set(dimension_slider_fraction(
                    next_height,
                    MIN_LAUNCHER_HEIGHT,
                    MAX_LAUNCHER_HEIGHT,
                ));
                preview_text_for_interval.set(t!(
                    "settings.visual.client_area",
                    width=next_width, height=next_height
                ).into_owned());
                if !(settings_visible_for_interval.get() && settings_tab_for_interval.get() == 1) {
                    apply_launcher_size(
                        &size_for_interval,
                        &position_for_interval,
                        &settings_for_interval_geometry,
                        next_width,
                        next_height,
                        false,
                        show_results_for_interval.get(),
                    );
                }
                if visual_preview_smoke_for_interval {
                    eprintln!(
                        "Visual preview dimension state: {}x{} logical px",
                        next_width, next_height
                    );
                }
                last_launcher_width = next_width;
                last_launcher_height = next_height;
            } else if current_width != last_launcher_width || current_height != last_launcher_height
            {
                last_launcher_width = current_width;
                last_launcher_height = current_height;
            }

            let preview_generation = visual_preview_generation_for_interval.get();
            if visual_preview_is_visible {
                let requested = (width_for_interval.get(), height_for_interval.get());
                let preview_locale = configured_locale(language_preference_from_index(
                    language_preference_for_interval.get(),
                ));
                let must_dispatch = last_visual_preview_request != Some(requested)
                    || last_visual_preview_generation != preview_generation
                    || last_visual_preview_locale != preview_locale;
                if must_dispatch {
                    let preference = settings_for_interval_geometry
                        .read()
                        .map(|settings| settings.monitor_preference)
                        .unwrap_or(MonitorPreference::Cursor);
                    let (preview_x, preview_y) = visual_preview_position(
                        preference,
                        i32::from(requested.0),
                        i32::from(requested.1),
                    );
                    let dispatch_result = if let Some(preview) = visual_preview_process.as_mut() {
                        match preview.poll_ready() {
                            Ok(true) => Some(
                                preview
                                    .set_locale(&preview_locale)
                                    .and_then(|_| {
                                        preview.resize(
                                            i32::from(requested.0),
                                            i32::from(requested.1),
                                            preview_x,
                                            preview_y,
                                        )
                                    }),
                            ),
                            Ok(false) => None,
                            Err(error) => Some(Err(error)),
                        }
                    } else {
                        None
                    };
                    match dispatch_result {
                        Some(Ok(())) => {
                            last_visual_preview_request = Some(requested);
                            last_visual_preview_generation = preview_generation;
                            last_visual_preview_locale = preview_locale.clone();
                            eprintln!(
                                "Visual preview IPC resize dispatched: {}x{}",
                                requested.0, requested.1
                            );
                        }
                        Some(Err(error)) => {
                            eprintln!("Could not update visual preview: {error}");
                            visual_preview_process.take();
                            last_visual_preview_request = None;
                            last_visual_preview_locale.clear();
                        }
                        None => {}
                    }
                }
            } else {
                last_visual_preview_request = None;
                last_visual_preview_generation = preview_generation;
            }

            if let Ok(settings) = settings_for_update_interval.read() {
                maybe_request_update_check(
                    &settings,
                    update_sender_for_interval.clone(),
                    &update_check_in_flight_for_interval,
                );
            }
            // 空查询默认结果集预热：启动后首个空闲 tick 把首屏默认目标的 shell 图标
            // 排入后台提取，首次显示结果时即命中缓存，不再出现图标逐个弹入。
            if !default_icon_prewarm_done {
                default_icon_prewarm_done = true;
                for target in prewarm_icon_targets(model.results(), EAGER_ICON_ROW_COUNT) {
                    request_shell_icon(&target);
                }
            }
            let completed_icon_generation =
                SHELL_ICON_COMPLETION_GENERATION.load(Ordering::Acquire);
            if icon_completion_generation_changed(last_icon_generation, completed_icon_generation) {
                last_icon_generation = completed_icon_generation;
                icon_refresh_generation_for_interval.set(completed_icon_generation);
            }
            let next_query = query_for_interval.get();
            if next_query == last_query {
                return;
            }

            let has_query = !next_query.trim().is_empty();
            history_mode_for_interval.set(false);
            show_results_for_interval.set(has_query);
            // Query cleanup also happens when hide-on-deactivate hides the
            // launcher. Do not let that asynchronous query transition resize
            // an already-open Settings panel back to the compact search strip.
            let (target_width, target_height) = launcher_window_geometry_with_prompt(
                settings_visible_for_interval.get(),
                everything_prompt_visible_for_interval.get(),
                has_query,
                launcher_width.get() as i32,
                launcher_height.get() as i32,
            );
            size_for_interval.set(target_width, target_height);
            dispatch_query(
                &mut model,
                &mut sequence,
                &next_query,
                has_query,
                auto_enable_everything_for_interval,
                obsidian_enabled_for_interval,
                obsidian_alias_for_interval,
                google_enabled_for_interval,
                google_alias_for_interval,
                system_commands_enabled_for_interval,
                results_for_interval,
                Rc::clone(&providers_for_interval),
                selected_id,
                selected_index,
                selection_touched_for_interval,
                inline_completion_for_interval,
                scroll_request_for_interval,
                action_mode,
                action_index,
                action_items,
                Rc::clone(&actions_for_interval),
                status_for_interval,
                i18n_hub.tr(|| t!("status.ready").into_owned()),
                &application_worker,
                &everything_worker,
                &plugin_worker,
                &native_plugin_worker,
            );
            sequence_for_interval.set(sequence);
            last_query = next_query;
        })
        .on_window_show(window_lifecycle::on_window_show(
            Arc::clone(&shared_settings),
            cursor_visibility.clone(),
            settings_visible,
            selection_color,
            window_size.clone(),
        ))
        .on_window_activated(window_lifecycle::on_window_activated(
            Arc::clone(&shared_settings),
            move || {
                refresh_everything_status_cheap(
                    auto_enable_everything,
                    everything_installed,
                    everything_status,
                );
            },
        ))
        .on_window_hide(window_lifecycle::on_window_hide(
            Arc::clone(&shared_settings),
            Rc::clone(&cancel_settings),
            settings_visible,
            clear_query_on_activation,
            query,
            results,
            selected_id,
            selected_index,
            selection_touched,
            show_results,
            history_mode,
            history_cursor,
            action_mode,
            action_index,
            action_items,
            inline_completion,
            scroll_request_for_rows,
            launcher_width,
            launcher_height,
            size_for_visibility,
        ))
        .run();
}

#[cfg(test)]
mod tests;
