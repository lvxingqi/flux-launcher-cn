use std::sync::{Arc, RwLock};

use flux_core::Settings;
use windui::prelude::{Color, Element, Signal, Truncate};

use crate::everything;
use crate::i18n::I18nHub;
use crate::launch;
use crate::settings_state::save_settings;

pub(crate) struct EverythingInstallPromptContext {
    pub(crate) visible: Signal<bool>,
    pub(crate) status: Signal<String>,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) i18n_hub: I18nHub,
}

pub(crate) fn everything_install_prompt(context: EverythingInstallPromptContext) -> Element {
    let EverythingInstallPromptContext {
        visible,
        status,
        settings,
        i18n_hub,
    } = context;
    let close_visible = visible;
    let decline_visible = visible;
    let install_visible = visible;
    let close_settings = Arc::clone(&settings);
    let decline_settings = Arc::clone(&settings);
    let install_settings = Arc::clone(&settings);
    let install_status = status;

    Element::dialog_glass_panel(
        visible,
        t!("everything.install").into_owned(),
        400,
        move |_| {
            close_visible.set(false);
            mark_everything_prompt_seen(&close_settings);
        },
        Element::col()
            .spacing(10)
            .child(
                Element::label(
                    i18n_hub.tr(|| t!("everything.prompt_install_question").into_owned()),
                )
                .font_size(13.0)
                .fg(Color::rgba(245, 248, 255, 245)),
            )
            .child(
                Element::label(i18n_hub.tr(|| t!("everything.prompt_winget_command").into_owned()))
                    .font_size(11.0)
                    .fg(Color::rgba(235, 241, 255, 180))
                    .max_lines(2)
                    .truncate(Truncate::End),
            ),
        Element::row()
            .width_match()
            .spacing(8)
            .child(Element::flex_spacer())
            .child(
                Element::button(i18n_hub.tr(|| t!("everything.not_now").into_owned()))
                    .neutral()
                    .outline_soft()
                    .on_click(move |_| {
                        decline_visible.set(false);
                        mark_everything_prompt_seen(&decline_settings);
                    }),
            )
            .child(
                Element::button(i18n_hub.tr(|| t!("everything.install").into_owned())).on_click(
                    move |ctx| {
                        install_visible.set(false);
                        mark_everything_prompt_seen(&install_settings);
                        match everything::launch_winget_install() {
                            Ok(()) => {
                                install_status.set(t!("everything.install_started").into_owned());
                                ctx.toast_ok(t!("everything.install_started_toast"));
                            }
                            Err(error) => {
                                install_status.set(error.clone());
                                ctx.toast_ok(error);
                            }
                        }
                    },
                ),
            )
            .padding_edges(0, 0, 0, 12),
    )
}

fn mark_everything_prompt_seen(settings: &Arc<RwLock<Settings>>) {
    if let Ok(mut settings) = settings.write() {
        settings.everything_install_prompt_seen = true;
        let _ = save_settings(&settings);
    }
}

pub(crate) struct RecycleBinDialogContext {
    pub(crate) visible: Signal<bool>,
    pub(crate) status: Signal<String>,
    pub(crate) i18n_hub: I18nHub,
}

pub(crate) fn recycle_bin_dialog(context: RecycleBinDialogContext) -> Element {
    let RecycleBinDialogContext {
        visible,
        status,
        i18n_hub,
    } = context;
    let close_visible = visible;
    let cancel_visible = visible;
    let empty_visible = visible;
    let empty_status = status;

    Element::dialog_panel(
        visible,
        t!("recycle_bin.title").into_owned(),
        360,
        move |_| close_visible.set(false),
        Element::col()
            .spacing(8)
            .child(
                Element::label(i18n_hub.tr(|| t!("recycle_bin.warning").into_owned()))
                    .font_size(13.0)
                    .fg(Color::rgba(245, 248, 255, 245)),
            )
            .child(
                Element::label(t!("recycle_bin.irreversible"))
                    .font_size(12.0)
                    .fg(Color::rgba(255, 190, 190, 235)),
            ),
        Element::row()
            .width_match()
            .spacing(8)
            .child(Element::flex_spacer())
            .child(
                Element::button(t!("common.cancel"))
                    .neutral()
                    .outline_soft()
                    .on_click(move |_| cancel_visible.set(false)),
            )
            .child(
                Element::button(t!("recycle_bin.title"))
                    .danger()
                    .on_click(move |_| {
                        empty_visible.set(false);
                        if launch::empty_recycle_bin() {
                            empty_status.set(t!("recycle_bin.emptied").into_owned());
                        } else {
                            empty_status.set(t!("recycle_bin.empty_failed").into_owned());
                        }
                    }),
            ),
    )
}
