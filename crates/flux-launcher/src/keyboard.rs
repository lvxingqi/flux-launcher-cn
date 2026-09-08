use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::actions::{
    actions_for_result, copy_result_file, copy_result_path, execute_result_action, selected_result,
    ActionItem, ActionKind,
};
use crate::launch;
use crate::plugins;
use crate::plugins::PluginAction;
use crate::query::{refresh_merged_results, ProviderResults};
use crate::settings_state::{record_query_history, set_result_priority};
use flux_core::{history_results, PriorityEntry, SearchResult, Settings};
use windui::app::{WindowOpHandle, WindowSizeHandle};
use windui::event::Key;
use windui::prelude::Signal;

pub(crate) struct ActionKeyContext {
    pub(crate) action_mode: Signal<bool>,
    pub(crate) action_index: Signal<usize>,
    pub(crate) action_items: Signal<Vec<ActionItem>>,
    pub(crate) action_scroll_pending: Signal<bool>,
    pub(crate) history_mode: Signal<bool>,
    pub(crate) query: Signal<String>,
    pub(crate) query_history: Rc<RefCell<Vec<String>>>,
    pub(crate) current_results: Vec<SearchResult>,
    pub(crate) selected_id: Signal<String>,
    pub(crate) selected_index: Signal<usize>,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) priorities: Signal<Vec<PriorityEntry>>,
    pub(crate) providers: Rc<RefCell<ProviderResults>>,
    pub(crate) priority_query: Signal<String>,
    pub(crate) results: Signal<Vec<SearchResult>>,
    pub(crate) window_op: WindowOpHandle,
    pub(crate) window_size: WindowSizeHandle,
    pub(crate) launcher_width: Signal<u16>,
    pub(crate) launcher_height: Signal<u16>,
}

pub(crate) fn handle_action_mode(key: Key, context: &ActionKeyContext) -> bool {
    let count = context.action_items.get().len();
    if count == 0 {
        context.action_mode.set(false);
        return true;
    }
    match key {
        Key::Up => {
            context.action_index.set(
                context
                    .action_index
                    .get()
                    .checked_sub(1)
                    .unwrap_or(count - 1),
            );
            context.action_scroll_pending.set(true);
        }
        Key::Down => {
            context
                .action_index
                .set((context.action_index.get() + 1) % count);
            context.action_scroll_pending.set(true);
        }
        Key::Left | Key::Escape => {
            context.action_mode.set(false);
            context.action_index.set(0);
            context.window_size.set(
                i32::from(context.launcher_width.get()),
                i32::from(context.launcher_height.get()),
            );
        }
        Key::Enter | Key::Space => {
            if context.history_mode.get() {
                if let Some(result) = selected_result(
                    &context.current_results,
                    &context.selected_id.get(),
                    context.selected_index.get(),
                ) {
                    context.query.set(result.title.clone());
                    context.history_mode.set(false);
                }
            } else {
                record_query_history(
                    &context.settings,
                    &context.query_history,
                    &context.query.get(),
                );
                if let Some(result) = selected_result(
                    &context.current_results,
                    &context.selected_id.get(),
                    context.selected_index.get(),
                ) {
                    if let Some(action) = context
                        .action_items
                        .get()
                        .get(context.action_index.get())
                        .cloned()
                    {
                        let executed = if matches!(action.kind, ActionKind::SetPriority) {
                            let saved =
                                set_result_priority(&context.settings, context.priorities, &result);
                            if saved {
                                refresh_merged_results(
                                    &context.providers,
                                    context.priority_query,
                                    context.priorities,
                                    context.results,
                                );
                            }
                            saved
                        } else {
                            execute_result_action(&result, &action.kind)
                        };
                        if executed {
                            context.window_op.hide_window();
                        }
                    }
                }
                context.action_mode.set(false);
                context.window_size.set(
                    i32::from(context.launcher_width.get()),
                    i32::from(context.launcher_height.get()),
                );
            }
        }
        _ => {}
    }
    true
}

pub(crate) fn handle_result_navigation(
    key: Key,
    current_results: &[SearchResult],
    selected_id: Signal<String>,
    selected_index: Signal<usize>,
    selection_touched: Signal<bool>,
    scroll_pending: Signal<bool>,
) -> bool {
    let count = current_results.len();
    if count == 0 || !matches!(key, Key::Up | Key::Down) {
        return false;
    }
    let next = match key {
        Key::Up => selected_index.get().checked_sub(1).unwrap_or(count - 1),
        Key::Down => (selected_index.get() + 1) % count,
        _ => return false,
    };
    selection_touched.set(true);
    selected_index.set(next);
    if let Some(result) = current_results.get(next) {
        selected_id.set(result.id.clone());
    }
    scroll_pending.set(true);
    true
}

pub(crate) fn handle_copy_shortcut(
    copy_file: bool,
    event_shift: bool,
    physical_shift: bool,
    current_results: &[SearchResult],
    selected_id: Signal<String>,
    selected_index: Signal<usize>,
) -> bool {
    if copy_file {
        eprintln!(
            "Ctrl+Shift+C dispatch: event_shift={event_shift} physical_shift={physical_shift}"
        );
        if let Some(result) =
            selected_result(current_results, &selected_id.get(), selected_index.get())
        {
            eprintln!("Ctrl+Shift+C target={:?}", result.target);
            if copy_result_file(&result) {
                return true;
            }
        }
        return false;
    }

    if let Some(result) = selected_result(current_results, &selected_id.get(), selected_index.get())
    {
        if copy_result_path(&result) {
            return true;
        }
    }
    false
}

pub(crate) fn handle_open_location_shortcut(
    history_mode: Signal<bool>,
    query: Signal<String>,
    query_history: &Rc<RefCell<Vec<String>>>,
    settings: &Arc<RwLock<Settings>>,
    current_results: &[SearchResult],
    selected_id: Signal<String>,
    selected_index: Signal<usize>,
) -> bool {
    if history_mode.get() {
        if let Some(result) =
            selected_result(current_results, &selected_id.get(), selected_index.get())
        {
            query.set(result.title.clone());
            history_mode.set(false);
        }
        return true;
    }

    record_query_history(settings, query_history, &query.get());
    if let Some(result) = selected_result(current_results, &selected_id.get(), selected_index.get())
    {
        if let Some(target) = result.target.as_deref() {
            let _ = launch::open_file_location(target);
        }
    }
    true
}

pub(crate) fn handle_run_as_admin_shortcut(
    query: Signal<String>,
    query_history: &Rc<RefCell<Vec<String>>>,
    settings: &Arc<RwLock<Settings>>,
    current_results: &[SearchResult],
    selected_id: Signal<String>,
    selected_index: Signal<usize>,
    window_op: &WindowOpHandle,
) -> bool {
    record_query_history(settings, query_history, &query.get());
    if let Some(result) = selected_result(current_results, &selected_id.get(), selected_index.get())
    {
        if let Some(target) = result.target.as_deref() {
            if launch::run_as_admin(target) {
                window_op.hide_window();
            }
        }
    }
    true
}

pub(crate) fn handle_action_entry(
    query: Signal<String>,
    query_caret_position: Signal<usize>,
    current_results: &[SearchResult],
    selected_id: Signal<String>,
    selected_index: Signal<usize>,
    plugin_actions: &Rc<RefCell<HashMap<String, PluginAction>>>,
    action_items: Signal<Vec<ActionItem>>,
    action_index: Signal<usize>,
    action_scroll_pending: Signal<bool>,
    action_mode: Signal<bool>,
    show_results: Signal<bool>,
    window_size: &WindowSizeHandle,
    launcher_width: Signal<u16>,
) -> bool {
    if query_caret_position.get() != query.get().chars().count() {
        return false;
    }
    if let Some(result) = selected_result(current_results, &selected_id.get(), selected_index.get())
    {
        let actions = actions_for_result(&result, &plugin_actions.borrow());
        if !actions.is_empty() {
            action_items.set(actions);
            action_index.set(0);
            action_scroll_pending.set(true);
            action_mode.set(true);
            show_results.set(true);
            window_size.set(i32::from(launcher_width.get()), crate::ACTION_WINDOW_HEIGHT);
        }
    }
    true
}

pub(crate) struct EnterKeyContext {
    pub(crate) history_mode: Signal<bool>,
    pub(crate) query: Signal<String>,
    pub(crate) query_history: Rc<RefCell<Vec<String>>>,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) current_results: Vec<SearchResult>,
    pub(crate) selected_id: Signal<String>,
    pub(crate) selected_index: Signal<usize>,
    pub(crate) recycle_bin_confirmation: Signal<bool>,
    pub(crate) settings_visible: Signal<bool>,
    pub(crate) window_size: WindowSizeHandle,
    pub(crate) window_op: WindowOpHandle,
    pub(crate) plugin_actions: Rc<RefCell<HashMap<String, PluginAction>>>,
}

pub(crate) fn handle_enter_key(context: &EnterKeyContext) -> bool {
    if context.history_mode.get() {
        if let Some(result) = selected_result(
            &context.current_results,
            &context.selected_id.get(),
            context.selected_index.get(),
        ) {
            context.query.set(result.title.clone());
            context.history_mode.set(false);
        }
        return true;
    }

    record_query_history(
        &context.settings,
        &context.query_history,
        &context.query.get(),
    );
    if let Some(result) = selected_result(
        &context.current_results,
        &context.selected_id.get(),
        context.selected_index.get(),
    ) {
        if result.id == "empty-recycle-bin" {
            context.recycle_bin_confirmation.set(true);
        } else if result.id == "flux-settings" {
            context.settings_visible.set(true);
            context
                .window_size
                .set(crate::SETTINGS_WINDOW_WIDTH, crate::SETTINGS_WINDOW_HEIGHT);
        } else if result.id == "open-recycle-bin" {
            launch::open_recycle_bin_async();
            context.window_op.hide_window();
        } else if let Some(target) = result.target.as_deref() {
            launch::open_path_async(target);
            context.window_op.hide_window();
        } else if let Some(action) = context.plugin_actions.borrow().get(&result.id).cloned() {
            plugins::execute_async(action);
            context.window_op.hide_window();
        }
    }
    true
}

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
