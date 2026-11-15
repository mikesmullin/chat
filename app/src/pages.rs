//! The app's routes.
//!
//! `/` is the app shell (`home`); `/storybook` and its pages are the design
//! guide (`storybook`). There is nothing else: the routes are thin, because
//! every page is composed from `components/*`, and a page that drew its own
//! markup would be a second definition of whatever it drew.
//!
//! There used to be a third route here, `/chat-demo` — the original
//! `m-js-cmp-chat` demo grid, ported markup-for-markup. It was the last place
//! that drew a transcript, a composer and a history rail by hand, and it is
//! gone: those are components now, and the design guide shows them against
//! sample data without a second copy of the markup.

use rustex1_kit::{mjs, Runtime};
use rustex1_kit::mjs::Def;

pub mod home;
pub mod storybook;

pub fn register(rt: &mut Runtime) {
    mjs::router_set_mode("path");
    // the home shell owns a CSS block and the `home` store (aside collapse)
    home::register();
    // `/` is the three-column app shell, composed from the storybook
    // components.
    rt.route("/", "Chat", |_params| {
        let mut d = Def::new().named("route /");
        d.tree(home::shell());
        d.finish()
    });

    storybook::register(rt);
}
