use crate::css;

pub mod agents_menu;
pub mod automation_form;
pub mod counter;
pub mod demo;
pub mod field;
pub mod hint_text;
pub mod item_group;
pub mod label;
pub mod nav_rail;
pub mod pie_ring_graph;
pub mod radio_input;
pub mod range_input;
pub mod reltime;
pub mod relative_timestamp;
pub mod sandboxes_menu;
pub mod settings_menu_item;
pub mod session_state;
pub mod tab;
pub mod tag_input;
pub mod text_area;
pub mod text_input;
pub mod toggle_button;
pub mod validation_info;

/// register the storybook's leaf widgets.
///
/// Widget CSS is injected inside each widget's `register()`. Route-tree CSS
/// (no widget registration) is injected here, from each tree's own `CSS` const.
pub fn register() {
    automation_form::register();
    counter::register();
    demo::register();
    field::register();
    hint_text::register();
    label::register();
    pie_ring_graph::register();
    radio_input::register();
    range_input::register();
    settings_menu_item::register();
    reltime::register();
    session_state::register();
    tab::register();
    tag_input::register();
    text_area::register();
    text_input::register();
    toggle_button::register();
    validation_info::register();

    css::inject("item-group", item_group::CSS);
    css::inject("agents-menu", agents_menu::CSS);
    css::inject("sandboxes-menu", sandboxes_menu::CSS);
    css::inject("nav-rail", nav_rail::CSS);
}
