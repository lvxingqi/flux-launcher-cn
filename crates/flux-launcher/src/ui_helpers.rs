use crate::i18n::I18nHub;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use flux_core::{PriorityEntry, SearchResult, Settings};

use crate::query::{refresh_merged_results, ProviderResults};
use crate::settings_state::{move_priority_entry, remove_priority_entry};
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

pub(crate) fn priority_row(
    entry: &PriorityEntry,
    rank: usize,
    priorities: Signal<Vec<PriorityEntry>>,
    results: Signal<Vec<SearchResult>>,
    providers: Rc<RefCell<ProviderResults>>,
    query: Signal<String>,
    settings: Arc<RwLock<Settings>>,
) -> Element {
    let entry_id = entry.id.clone();
    let title = entry.title.clone();
    let target = entry.target.clone();
    let settings_for_up = Arc::clone(&settings);
    let settings_for_down = Arc::clone(&settings);
    let settings_for_remove = Arc::clone(&settings);
    let providers_for_up = Rc::clone(&providers);
    let providers_for_down = Rc::clone(&providers);
    let providers_for_remove = Rc::clone(&providers);
    let query_for_up = query;
    let query_for_down = query;
    let query_for_remove = query;
    let id_for_up = entry_id.clone();
    let id_for_down = entry_id.clone();
    let id_for_remove = entry_id;

    Element::row()
        .width_match()
        .height(58)
        .padding_xy(10, 5)
        .spacing(8)
        .corner(9.0)
        .bg(Color::rgba(255, 255, 255, 12))
        .child(
            Element::label(format!("{rank}"))
                .font_size(16.0)
                .fg(Color::rgba(170, 204, 255, 245))
                .width(24)
                .align(Align::Center),
        )
        .child(
            Element::col()
                .weight(1.0)
                .spacing(1)
                .child(
                    Element::label(title)
                        .font_size(13.0)
                        .fg(Color::WHITE)
                        .max_lines(1)
                        .truncate(Truncate::End),
                )
                .child(
                    Element::label(target)
                        .font_size(10.0)
                        .fg(Color::rgba(235, 241, 255, 170))
                        .max_lines(1)
                        .truncate(Truncate::End),
                ),
        )
        .child(
            Element::button("↑")
                .neutral()
                .outline_soft()
                .on_click(move |ctx| {
                    if move_priority_entry(&settings_for_up, priorities, &id_for_up, -1) {
                        refresh_merged_results(
                            &providers_for_up,
                            query_for_up,
                            priorities,
                            results,
                        );
                        ctx.toast_ok(t!("priorities.moved_up"));
                    }
                }),
        )
        .child(
            Element::button("↓")
                .neutral()
                .outline_soft()
                .on_click(move |ctx| {
                    if move_priority_entry(&settings_for_down, priorities, &id_for_down, 1) {
                        refresh_merged_results(
                            &providers_for_down,
                            query_for_down,
                            priorities,
                            results,
                        );
                        ctx.toast_ok(t!("priorities.moved_down"));
                    }
                }),
        )
        .child(
            Element::button(t!("priorities.remove"))
                .neutral()
                .outline_soft()
                .on_click(move |ctx| {
                    if remove_priority_entry(&settings_for_remove, priorities, &id_for_remove) {
                        refresh_merged_results(
                            &providers_for_remove,
                            query_for_remove,
                            priorities,
                            results,
                        );
                        ctx.toast_ok(t!("priorities.removed"));
                    }
                }),
        )
}
