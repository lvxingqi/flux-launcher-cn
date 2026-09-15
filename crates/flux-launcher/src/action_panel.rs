use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use crate::actions::ActionItem;
use crate::query::ProviderResults;
use crate::ui_helpers::action_row;
use flux_core::{PriorityEntry, SearchResult, Settings};
use windui::app::WindowSizeHandle;
use windui::prelude::{Element, Signal};

pub(crate) struct ActionListContext {
    pub(crate) action_items: Signal<Vec<ActionItem>>,
    pub(crate) action_index: Signal<usize>,
    pub(crate) action_scroll_pending: Signal<bool>,
    pub(crate) result_source: Signal<Vec<SearchResult>>,
    pub(crate) selected_id: Signal<String>,
    pub(crate) selected_index: Signal<usize>,
    pub(crate) action_mode: Signal<bool>,
    pub(crate) launcher_width: Signal<u16>,
    pub(crate) launcher_height: Signal<u16>,
    pub(crate) action_window_slot: Rc<RefCell<Option<WindowSizeHandle>>>,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) priorities: Signal<Vec<PriorityEntry>>,
    pub(crate) provider_results: Rc<RefCell<ProviderResults>>,
    pub(crate) query: Signal<String>,
}

pub(crate) fn action_list(context: ActionListContext) -> Element {
    let ActionListContext {
        action_items,
        action_index,
        action_scroll_pending,
        result_source,
        selected_id,
        selected_index,
        action_mode,
        launcher_width,
        launcher_height,
        action_window_slot,
        settings,
        priorities,
        provider_results,
        query,
    } = context;

    Element::list_signal(
        action_items,
        |item| item.id.clone(),
        move |item| {
            let item_index = action_items
                .get()
                .iter()
                .position(|candidate| candidate.id == item.id)
                .unwrap_or_default();
            action_row(
                &item,
                item_index,
                action_index,
                action_scroll_pending,
                result_source,
                selected_id,
                selected_index,
                action_mode,
                launcher_width,
                launcher_height,
                action_window_slot.clone(),
                Arc::clone(&settings),
                priorities,
                Rc::clone(&provider_results),
                query,
            )
        },
    )
    .height(174)
    .corner(12.0)
    .visible_signal(action_mode)
}
