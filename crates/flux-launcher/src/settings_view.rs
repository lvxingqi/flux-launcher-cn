use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::everything::EverythingStartup;
use crate::i18n::I18nHub;
use crate::plugins::native_plugin_install_path;
use crate::query::ProviderResults;
use crate::settings_state::save_settings;
use crate::ui_helpers::priority_row;
use flux_core::{PriorityEntry, SearchResult, Settings};
use windui::prelude::*;

/// Shared reactive state for the Settings surface.
#[derive(Clone, Copy)]
pub(crate) struct SettingsUiState {
    pub(crate) visible: Signal<bool>,
    pub(crate) tab: Signal<usize>,
}

pub(crate) fn priority_list(
    priorities: Signal<Vec<PriorityEntry>>,
    results: Signal<Vec<SearchResult>>,
    providers: Rc<RefCell<ProviderResults>>,
    query: Signal<String>,
    settings: Arc<RwLock<Settings>>,
) -> Element {
    Element::list_signal(
        priorities,
        |entry| entry.id.clone(),
        move |entry| {
            let rank = priorities
                .get()
                .iter()
                .position(|candidate| candidate.id == entry.id)
                .map(|index| index + 1)
                .unwrap_or_default();
            priority_row(
                &entry,
                rank,
                priorities,
                results,
                Rc::clone(&providers),
                query,
                Arc::clone(&settings),
            )
        },
    )
}

pub(crate) fn launcher_width_reset_button(
    i18n_hub: I18nHub,
    launcher_width: Signal<u16>,
    launcher_height: Signal<u16>,
    launcher_width_input: Signal<String>,
    launcher_width_slider: Signal<f32>,
    launcher_preview_text: Signal<String>,
    visual_preview_generation: Signal<u64>,
) -> Element {
    Element::button(i18n_hub.tr(|| t!("settings.visual.reset").into_owned()))
        .neutral()
        .on_click(move |_| {
            let width = flux_core::DEFAULT_LAUNCHER_WIDTH;
            let height = launcher_height.get();
            eprintln!("Visual width reset clicked: {}x{}", width, height);
            launcher_width.set(width);
            launcher_width_input.set(width.to_string());
            launcher_width_slider.set(crate::dimension_slider_fraction(
                width,
                flux_core::MIN_LAUNCHER_WIDTH,
                flux_core::MAX_LAUNCHER_WIDTH,
            ));
            launcher_preview_text.set(
                t!(
                    "settings.visual.client_area",
                    width = width,
                    height = height
                )
                .into_owned(),
            );
            visual_preview_generation.set(visual_preview_generation.get().saturating_add(1));
        })
}

pub(crate) fn launcher_height_reset_button(
    i18n_hub: I18nHub,
    launcher_width: Signal<u16>,
    launcher_height: Signal<u16>,
    launcher_height_input: Signal<String>,
    launcher_height_slider: Signal<f32>,
    launcher_preview_text: Signal<String>,
    visual_preview_generation: Signal<u64>,
) -> Element {
    Element::button(i18n_hub.tr(|| t!("settings.visual.reset").into_owned()))
        .neutral()
        .on_click(move |_| {
            let width = launcher_width.get();
            let height = flux_core::DEFAULT_LAUNCHER_HEIGHT;
            eprintln!("Visual height reset clicked: {}x{}", width, height);
            launcher_height.set(height);
            launcher_height_input.set(height.to_string());
            launcher_height_slider.set(crate::dimension_slider_fraction(
                height,
                flux_core::MIN_LAUNCHER_HEIGHT,
                flux_core::MAX_LAUNCHER_HEIGHT,
            ));
            launcher_preview_text.set(
                t!(
                    "settings.visual.client_area",
                    width = width,
                    height = height
                )
                .into_owned(),
            );
            visual_preview_generation.set(visual_preview_generation.get().saturating_add(1));
        })
}

pub(crate) struct VisualApplyContext {
    pub(crate) i18n_hub: I18nHub,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) launcher_width: Signal<u16>,
    pub(crate) launcher_height: Signal<u16>,
    pub(crate) launcher_width_input: Signal<String>,
    pub(crate) launcher_height_input: Signal<String>,
    pub(crate) launcher_width_slider: Signal<f32>,
    pub(crate) launcher_height_slider: Signal<f32>,
    pub(crate) launcher_preview_text: Signal<String>,
    pub(crate) smooth_caret: Signal<bool>,
    pub(crate) caret_duration: Signal<String>,
    pub(crate) settings_visible: Signal<bool>,
    pub(crate) show_results: Signal<bool>,
    pub(crate) window_size: windui::app::WindowSizeHandle,
    pub(crate) window_position: windui::app::WindowPositionHandle,
}

pub(crate) fn visual_apply_button(context: VisualApplyContext) -> Element {
    let VisualApplyContext {
        i18n_hub,
        settings,
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
        window_size,
        window_position,
    } = context;

    Element::button(i18n_hub.tr(|| t!("settings.visual.apply").into_owned())).on_click(move |ctx| {
        let mut width = crate::parse_dimension_input(
            &launcher_width_input.get(),
            flux_core::MIN_LAUNCHER_WIDTH,
            flux_core::MAX_LAUNCHER_WIDTH,
        )
        .unwrap_or(flux_core::DEFAULT_LAUNCHER_WIDTH);
        let mut height = crate::parse_dimension_input(
            &launcher_height_input.get(),
            flux_core::MIN_LAUNCHER_HEIGHT,
            flux_core::MAX_LAUNCHER_HEIGHT,
        )
        .unwrap_or(flux_core::DEFAULT_LAUNCHER_HEIGHT);
        let duration = caret_duration
            .get()
            .trim()
            .parse::<u16>()
            .unwrap_or(95)
            .clamp(60, 160);
        let Ok(mut settings) = settings.write() else {
            ctx.toast_ok(t!("settings.lock_failed"));
            return;
        };
        settings.launcher_width = width;
        settings.launcher_height = height;
        settings.smooth_caret = smooth_caret.get();
        settings.smooth_caret_duration_ms = duration;
        settings.normalize();
        width = settings.launcher_width;
        height = settings.launcher_height;
        let preference = settings.monitor_preference;
        if !crate::settings_state::save_settings(&settings) {
            ctx.toast_ok(t!("settings.visual.save_failed"));
            return;
        }
        launcher_width.set(width);
        launcher_height.set(height);
        launcher_width_input.set(width.to_string());
        launcher_height_input.set(height.to_string());
        launcher_width_slider.set(crate::dimension_slider_fraction(
            width,
            flux_core::MIN_LAUNCHER_WIDTH,
            flux_core::MAX_LAUNCHER_WIDTH,
        ));
        launcher_height_slider.set(crate::dimension_slider_fraction(
            height,
            flux_core::MIN_LAUNCHER_HEIGHT,
            flux_core::MAX_LAUNCHER_HEIGHT,
        ));
        launcher_preview_text.set(
            t!(
                "settings.visual.client_area",
                width = width,
                height = height
            )
            .into_owned(),
        );
        eprintln!("Visual Apply dimensions clicked: {}x{}", width, height);
        settings_visible.set(false);
        let target_height = if show_results.get() {
            i32::from(height)
        } else {
            crate::COMPACT_WINDOW_HEIGHT
        };
        crate::window_state::request_monitor_position(
            &window_position,
            preference,
            i32::from(width),
            target_height,
        );
        window_size.set(i32::from(width), target_height);
        ctx.show_window();
        ctx.toast_ok(t!("settings.visual.applied"));
    })
}

/// Everything 自动启用检测的后台结果槽。
///
/// windui 的 `Signal` 只能在 UI 线程读写，而 Everything 检测（注册表扫描、
/// `tasklist` 进程枚举、IPC 连接）可能阻塞数百毫秒以上，不能在复选框回调里
/// 同步执行。后台线程把结果写入本槽，UI 线程在既有 `on_interval` 轮询中取走
/// 并更新信号，保证复选框点击立即有视觉反馈。
#[derive(Default)]
pub(crate) struct EverythingDetectionSlot {
    running: AtomicBool,
    result: Mutex<Option<Result<EverythingStartup, String>>>,
}

impl EverythingDetectionSlot {
    /// 在后台线程启动一次检测；已有检测在跑时忽略重复请求。
    pub(crate) fn spawn_detection(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let slot = Arc::clone(self);
        thread::spawn(move || {
            let result = crate::everything::start_background_if_installed();
            if let Ok(mut pending) = slot.result.lock() {
                *pending = Some(result);
            }
            slot.running.store(false, Ordering::SeqCst);
        });
    }

    /// UI 线程取走已完成的检测结果（若有）；重复取返回 `None`。
    pub(crate) fn take_result(&self) -> Option<Result<EverythingStartup, String>> {
        self.result.lock().ok()?.take()
    }
}

pub(crate) struct EverythingSettingsContext {
    pub(crate) i18n_hub: I18nHub,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) auto_enable: Signal<bool>,
    pub(crate) installed: Signal<bool>,
    pub(crate) status: Signal<String>,
    pub(crate) detection: Arc<EverythingDetectionSlot>,
    pub(crate) obsidian_enabled: Signal<bool>,
    pub(crate) obsidian_alias: Signal<String>,
    pub(crate) google_enabled: Signal<bool>,
    pub(crate) google_alias: Signal<String>,
    pub(crate) system_commands_enabled: Signal<bool>,
}

pub(crate) fn everything_settings(context: EverythingSettingsContext) -> Element {
    let EverythingSettingsContext {
        i18n_hub,
        settings,
        auto_enable,
        installed,
        status,
        detection,
        obsidian_enabled,
        obsidian_alias,
        google_enabled,
        google_alias,
        system_commands_enabled,
    } = context;
    let settings_for_toggle = Arc::clone(&settings);
    let auto_enable_for_toggle = auto_enable;
    let status_for_toggle = status;
    let installed_for_ui = installed;
    let status_for_install = status;
    let detection_for_toggle = Arc::clone(&detection);

    Element::col()
        .width_match()
        .spacing(12)
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.everything").into_owned()))
                .font_size(17.0)
                .fg(Color::WHITE),
        )
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.everything_tab_desc").into_owned()))
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 180))
                .max_lines(3)
                .truncate(Truncate::End),
        )
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.everything").into_owned()),
            Element::checkbox(
                i18n_hub.tr(|| t!("settings.everything_desc").into_owned()),
                auto_enable,
            )
            .on_toggle(move |_| {
                let enabled = auto_enable_for_toggle.get();
                if let Ok(mut settings) = settings_for_toggle.write() {
                    settings.auto_enable_everything = enabled;
                    settings.normalize();
                    let _ = save_settings(&settings);
                }
                if !enabled {
                    status_for_toggle.set(t!("everything.auto_enable_disabled").into_owned());
                    return;
                }
                // 立即给出反馈，检测放到后台线程；结果由 on_interval 轮询回填，
                // 避免在 UI 线程上同步等待注册表扫描 / tasklist / IPC 连接。
                status_for_toggle.set(t!("everything.detecting").into_owned());
                detection_for_toggle.spawn_detection();
            }),
        ))
        .child(
            Element::label(i18n_hub.tr(|| t!("everything.is_installed").into_owned()))
                .font_size(12.0)
                .fg(Color::rgba(180, 255, 205, 235))
                .visible_when(move || installed_for_ui.get()),
        )
        .child(
            Element::label(i18n_hub.tr(|| t!("everything.is_not_installed").into_owned()))
                .font_size(12.0)
                .fg(Color::rgba(255, 225, 175, 235))
                .visible_when(move || !installed.get()),
        )
        .child(
            Element::label_signal(status)
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 190))
                .max_lines(2)
                .truncate(Truncate::End)
                .width_match(),
        )
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.everything_command").into_owned()))
                .font_size(10.0)
                .fg(Color::rgba(235, 241, 255, 155))
                .visible_when(move || !installed_for_ui.get())
                .width_match(),
        )
        .child(
            Element::button(i18n_hub.tr(|| t!("everything.install").into_owned()))
                .visible_when(move || !installed_for_ui.get())
                .on_click(
                    move |ctx| match crate::everything::launch_winget_install() {
                        Ok(()) => {
                            status_for_install
                                .set(t!("everything.winget_started_restart").into_owned());
                            ctx.toast_ok(t!("everything.winget_started_toast"));
                        }
                        Err(error) => {
                            status_for_install.set(error.clone());
                            ctx.toast_ok(error);
                        }
                    },
                ),
        )
        .child(plugin_settings(
            i18n_hub,
            obsidian_enabled,
            obsidian_alias,
            google_enabled,
            google_alias,
            system_commands_enabled,
        ))
}

impl SettingsUiState {
    pub(crate) fn new(visible: Signal<bool>, tab: Signal<usize>) -> Self {
        Self { visible, tab }
    }
}

/// Wrap the Settings panel without changing its transparent Acrylic surface or
/// its visibility binding. Page contents remain assembled by the launcher while
/// their lifecycle callbacks are migrated incrementally.
pub(crate) fn settings_page(panel: Element, state: SettingsUiState) -> Element {
    Element::col()
        .fill()
        .padding(18)
        .child(panel)
        .visible_signal(state.visible)
}

/// Build the Settings header while keeping tab state and the Back callback explicit.
pub(crate) fn settings_header(
    settings_tab: Signal<usize>,
    i18n_hub: I18nHub,
    cancel_settings: Rc<dyn Fn()>,
) -> Element {
    Element::row()
        .width_match()
        .child(
            Element::col()
                .weight(1.0)
                .spacing(3)
                .child(
                    Element::label(i18n_hub.tr(|| t!("settings.title").into_owned()))
                        .font_size(25.0)
                        .fg(Color::WHITE),
                )
                .child(
                    Element::label(i18n_hub.tr(|| t!("settings.apply_immediate").into_owned()))
                        .font_size(12.0)
                        .fg(Color::rgba(235, 241, 255, 180)),
                ),
        )
        .child(Element::segmented_signal(
            i18n_hub.tr_vec(|| {
                vec![
                    t!("settings.tab.general").to_string(),
                    t!("settings.tab.visual").to_string(),
                    t!("settings.tab.priorities").to_string(),
                    t!("settings.tab.plugins").to_string(),
                ]
            }),
            settings_tab,
        ))
        .child(
            Element::button(i18n_hub.tr(|| t!("settings.back").into_owned()))
                .neutral()
                .on_click({
                    let cancel_settings = Rc::clone(&cancel_settings);
                    move |ctx| {
                        cancel_settings();
                        ctx.show_window();
                    }
                }),
        )
}

/// Build the native plugin configuration section.
pub(crate) fn plugin_settings(
    i18n_hub: I18nHub,
    obsidian_enabled: Signal<bool>,
    obsidian_alias: Signal<String>,
    google_enabled: Signal<bool>,
    google_alias: Signal<String>,
    system_commands_enabled: Signal<bool>,
) -> Element {
    Element::col()
        .width_match()
        .spacing(12)
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.plugins.title").into_owned()))
                .font_size(17.0)
                .fg(Color::WHITE),
        )
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.plugins.description").into_owned()))
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 180))
                .max_lines(3)
                .truncate(Truncate::End),
        )
        .child(
            Element::label(t!(
                "settings.plugins.folder",
                path = native_plugin_install_path()
            ))
            .font_size(10.0)
            .fg(Color::rgba(235, 241, 255, 150))
            .max_lines(2)
            .truncate(Truncate::End),
        )
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.plugins.config_hint").into_owned()))
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 180))
                .max_lines(2)
                .truncate(Truncate::End),
        )
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.plugins.obsidian").into_owned()),
            Element::checkbox(
                i18n_hub.tr(|| t!("settings.plugins.obsidian_desc").into_owned()),
                obsidian_enabled,
            ),
        ))
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.plugins.action_keyword").into_owned()),
            Element::text_input(obsidian_alias, "ob").width_match(),
        ))
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.plugins.obsidian_hint").into_owned()))
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 175))
                .max_lines(3)
                .truncate(Truncate::End),
        )
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.plugins.google").into_owned()),
            Element::checkbox(
                i18n_hub.tr(|| t!("settings.plugins.google_desc").into_owned()),
                google_enabled,
            ),
        ))
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.plugins.action_keyword").into_owned()),
            Element::text_input(google_alias, "g").width_match(),
        ))
        .child(
            Element::label(i18n_hub.tr(|| t!("settings.plugins.google_hint").into_owned()))
                .font_size(11.0)
                .fg(Color::rgba(235, 241, 255, 175))
                .max_lines(3)
                .truncate(Truncate::End),
        )
        .child(Element::field_signal(
            i18n_hub.tr(|| t!("settings.plugins.system_commands").into_owned()),
            Element::checkbox(
                i18n_hub.tr(|| t!("settings.plugins.system_commands_desc").into_owned()),
                system_commands_enabled,
            ),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_slot_delivers_result_exactly_once() {
        let slot = Arc::new(EverythingDetectionSlot::default());
        assert!(slot.take_result().is_none());

        // 冒烟环境变量让检测在无 Everything 的机器上确定性地返回 Installed。
        std::env::set_var("FLUX_SMOKE_EVERYTHING_INSTALLED", "1");
        slot.spawn_detection();
        let mut delivered = None;
        for _ in 0..300 {
            if let Some(result) = slot.take_result() {
                delivered = Some(result);
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        std::env::remove_var("FLUX_SMOKE_EVERYTHING_INSTALLED");

        // 槽的不变量是「结果恰好投递一次」。Ok/Err 取决于当前机器能否真的拉起
        // Everything：冒烟钩子只保证注册表状态为「已安装」，在没有任何
        // Everything.exe 的机器（如 CI runner）上后台拉起会失败并返回 Err，
        // 因此这里不断言 Ok/Err，只在 Ok 时校验安装状态。
        let delivered = delivered.expect("detection result was not delivered within 3s");
        if let Ok(outcome) = &delivered {
            assert!(outcome.is_installed());
        }
        assert!(slot.take_result().is_none());
    }

    #[test]
    fn duplicate_spawn_requests_are_ignored_while_running() {
        let slot = Arc::new(EverythingDetectionSlot::default());
        // 直接置位运行标记，验证重复 spawn 不产生第二个结果。
        slot.running.store(true, Ordering::SeqCst);
        slot.spawn_detection();
        assert!(slot.take_result().is_none());
        slot.running.store(false, Ordering::SeqCst);
    }
}
