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

pub(crate) fn selection_color_hex(value: u32) -> String {
    format!("#{value:06X}")
}

pub(crate) fn selection_palette(custom_selection_color: Signal<String>) -> Element {
    const COLORS: &[u32] = &[
        0x4c8bf4, 0x0078d4, 0x00a4ef, 0x107c10, 0x498205, 0xffb900, 0xd83b01, 0xe74856, 0x8764b8,
        0x744da9, 0x038387, 0x605e5c,
    ];
    let mut row = Element::row().spacing(6).width_match();
    for &value in COLORS {
        let label = selection_color_hex(value);
        row = row.child(
            Element::col()
                .width(24)
                .height(24)
                .bg(Color::rgb(
                    ((value >> 16) & 0xff) as u8,
                    ((value >> 8) & 0xff) as u8,
                    (value & 0xff) as u8,
                ))
                .corner(6.0)
                .clickable()
                .tooltip(label)
                .on_click(move |_| custom_selection_color.set(selection_color_hex(value))),
        );
    }
    row
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
