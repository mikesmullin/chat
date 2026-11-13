use crate::css;

pub mod agents_menu;
pub mod approval_card;
pub mod assistant_card;
pub mod automation_form;
pub mod composer;
pub mod counter;
pub mod demo;
pub mod dropdown;
pub mod error_card;
pub mod field;
pub mod hint_text;
pub mod item_group;
pub mod live;
pub mod label;
pub mod nav_rail;
pub mod pie_ring_graph;
pub mod radio_input;
pub mod range_input;
pub mod reasoning_card;
pub mod reltime;
pub mod relative_timestamp;
pub mod sandboxes_menu;
pub mod session;
pub mod session_history;
pub mod session_panel;
pub mod session_state;
pub mod settings_menu_item;
pub mod system_card;
pub mod tab;
pub mod tag_input;
pub mod task_list;
pub mod text_area;
pub mod text_input;
pub mod thinking;
pub mod toggle_button;
pub mod tool_call_card;
pub mod tool_response_card;
pub mod user_card;
pub mod validation_info;

/// register the storybook's leaf widgets.
///
/// Widget CSS is injected inside each widget's `register()`. Route-tree CSS
/// (no widget registration) is injected here, from each tree's own `CSS` const.
pub fn register() {
    approval_card::register();
    assistant_card::register();
    automation_form::register();
    composer::register();
    counter::register();
    demo::register();
    dropdown::register();
    error_card::register();
    field::register();
    hint_text::register();
    live::register();
    label::register();
    pie_ring_graph::register();
    radio_input::register();
    range_input::register();
    reasoning_card::register();
    settings_menu_item::register();
    reltime::register();
    session::register();
    session_history::register();
    session_panel::register();
    session_state::register();
    system_card::register();
    tab::register();
    tag_input::register();
    task_list::register();
    text_area::register();
    text_input::register();
    thinking::register();
    toggle_button::register();
    tool_call_card::register();
    tool_response_card::register();
    user_card::register();
    validation_info::register();

    css::inject("item-group", item_group::CSS);
    css::inject("agents-menu", agents_menu::CSS);
    css::inject("sandboxes-menu", sandboxes_menu::CSS);
    css::inject("nav-rail", nav_rail::CSS);
}
