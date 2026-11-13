use crate::css;

pub mod agents_menu;
pub mod counter;
pub mod item_group;
pub mod nav_rail;
pub mod pie_ring_graph;
pub mod reltime;
pub mod relative_timestamp;
pub mod sandboxes_menu;
pub mod settings_menu_item;
pub mod session_state;
pub mod tab;

/// register the storybook's leaf widgets.
///
/// Widget CSS is injected inside each widget's `register()`. Route-tree CSS
/// (no widget registration) is injected here, from each tree's own `CSS` const.
pub fn register() {
    counter::register();
    pie_ring_graph::register();
    settings_menu_item::register();
    reltime::register();
    session_state::register();
    tab::register();

    css::inject("item-group", item_group::CSS);
    css::inject("agents-menu", agents_menu::CSS);
    css::inject("sandboxes-menu", sandboxes_menu::CSS);
    css::inject("nav-rail", nav_rail::CSS);
}
