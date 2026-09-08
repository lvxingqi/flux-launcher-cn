use crate::i18n::I18nHub;
use windui::prelude::*;

fn action_hint(key: &'static str, label: Signal<String>) -> Element {
    Element::row()
        .height(22)
        .cross(Align::Center)
        .spacing(4)
        .child(
            Element::label(key)
                .font_size(9.0)
                .fg(Color::rgba(235, 243, 255, 235))
                .bg(Color::rgba(255, 255, 255, 24))
                .corner(5.0)
                .padding_xy(4, 2),
        )
        .child(
            Element::label(label)
                .font_size(10.0)
                .fg(Color::rgba(222, 233, 248, 220)),
        )
}

pub(crate) fn action_bar_content(i18n_hub: I18nHub) -> Element {
    // Keep the three hints in a bounded row so they remain centered between
    // the launcher content insets while the window width changes.
    Element::row()
        .height(22)
        .spacing(8)
        .child(action_hint(
            "↵",
            i18n_hub.tr(|| t!("action_bar.open").into_owned()),
        ))
        .child(action_hint(
            "Ctrl + R",
            i18n_hub.tr(|| t!("action_bar.run_as_admin").into_owned()),
        ))
        .child(action_hint(
            "Alt + Enter",
            i18n_hub.tr(|| t!("action_bar.open_file_location").into_owned()),
        ))
}
