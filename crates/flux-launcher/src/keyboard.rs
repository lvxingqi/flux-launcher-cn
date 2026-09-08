use crate::actions::ActionItem;
use flux_core::{history_results, SearchResult};
use windui::app::WindowSizeHandle;
use windui::event::Key;
use windui::prelude::Signal;

pub(crate) fn history_cursor_step(
    history_len: usize,
    cursor: Option<usize>,
    key: Key,
) -> Option<usize> {
    if history_len == 0 {
        return None;
    }
    Some(match (key, cursor) {
        (Key::Up, Some(index)) => index.saturating_sub(1),
        (Key::Down, Some(index)) => (index + 1).min(history_len - 1),
        (_, _) => history_len - 1,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn open_history_mode(
    history: &[String],
    query: Signal<String>,
    history_mode: Signal<bool>,
    history_cursor: Signal<Option<usize>>,
    action_mode: Signal<bool>,
    action_items: Signal<Vec<ActionItem>>,
    inline_completion: Signal<String>,
    selected_index: Signal<usize>,
    selected_id: Signal<String>,
    results: Signal<Vec<SearchResult>>,
    show_results: Signal<bool>,
    window_size: &WindowSizeHandle,
    launcher_width: Signal<u16>,
    launcher_height: Signal<u16>,
) -> bool {
    if history.is_empty() {
        return false;
    }
    let filtered = history_results(history, &query.get());
    history_mode.set(true);
    history_cursor.set(None);
    action_mode.set(false);
    action_items.set(Vec::new());
    inline_completion.set(String::new());
    selected_index.set(0);
    selected_id.set(
        filtered
            .first()
            .map(|result| result.id.clone())
            .unwrap_or_default(),
    );
    results.set(filtered);
    show_results.set(true);
    window_size.set(launcher_width.get() as i32, launcher_height.get() as i32);
    true
}

pub(crate) fn cycle_query_history(
    history: &[String],
    key: Key,
    history_cursor: Signal<Option<usize>>,
    history_mode: Signal<bool>,
    query: Signal<String>,
) -> bool {
    if history.is_empty() {
        return false;
    }
    let Some(next) = history_cursor_step(history.len(), history_cursor.get(), key) else {
        return false;
    };
    history_cursor.set(Some(next));
    history_mode.set(false);
    query.set(history[next].clone());
    true
}
