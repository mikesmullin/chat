//! `profile panel` — who *you* are (PLAN44): the per-user half of the old
//! combined Settings form — **Location** and **Devices** — opened from the
//! account chip's *Profile…* item. The global half (the perimeter's
//! **Shields**) stays in Settings, and the Share panel is per-resource and
//! belongs to neither.
//!
//! Reuses the editor's field styles (`.ae-*`) like every inspector form —
//! an inspector panel is an inspector panel.

use rustex1_kit::h::{lit, m, Node};

use crate::components::validation_info;
use crate::css;

pub const CSS: &str = r#"
.profile-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

/// The per-user form: verdict banner, then Location and Devices.
pub fn panel() -> Node {
    m("div")
        .class("agent-editor profile-panel")
        .show("$store.ident.panelOpen")
        .init("$store.ident.loadDevices()")
        .kids(vec![
            m("div").class("agent-editor-head").kids(vec![
                Node::icon("user").class("ph ph-user"),
                m("span").class("agent-editor-title").child(lit("Profile")),
            ]),
            // The verdict banner: always at the top of the form, just above
            // the first field — one brick, every aside form. This form
            // auto-saves, so the banner is only ever the bad news.
            validation_info::live("'error'", "$store.ident.error"),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Location")),
                    m("span").class("hint").child(lit("where you are · shared with agents")),
                ]),
                // The globe lives inside the field — inset at the left,
                // the input's text padded past it. Absolute, so the field
                // keeps its height.
                m("div").class("ae-input-icon").kids(vec![
                    Node::icon("globe").class("ph ph-bold ph-globe ae-input-glyph"),
                    m("input")
                        .attr("type", "text")
                        .attr("placeholder", "City, State")
                        .model("$store.ident.location")
                        // Auto-save on edit, debounced — this form has no
                        // submit button (the fields save themselves).
                        .on("input", "$store.ident.queueSave($event)"),
                ]),
            ]),
            // ---- identity: recognized devices (PLAN44 A2) --------------
            // Every login this account owns — live ones only, most recent
            // first. The trash force-expires them.
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Devices")),
                    m("span").class("hint").child(lit("most recent activity")),
                ]),
                m("div")
                    .class("ae-devices")
                    .init("$store.ident.loadDevices()")
                    .kids(vec![m("div")
                        .class("ae-device")
                        .for_each("d in $store.ident.devices")
                        .kids(vec![
                            // The row's bullet: a phone for phones, a
                            // laptop for the rest (one class binding —
                            // stacked binds clobber each other).
                            m("i").bind(
                                "class",
                                "((d.userAgent || '').includes('Mobile') ? 'ph ph-bold ph-device-mobile ae-device-icon' : 'ph ph-bold ph-laptop ae-device-icon')",
                            ),
                            m("span").class("ae-device-fp").text("d.fingerprint"),
                            m("span").class("ae-device-ip").text("d.ip"),
                            // Last seen, ticking (PLAN44 A2): the kit's
                            // relative clock, same one the bylines use.
                            Node::host("uiRelTime", &[])
                                .class("ae-device-when")
                                .bind("at", "new Date(d.lastSeenAt).toISOString()"),
                            m("button")
                                .class("ae-device-del")
                                .attr("type", "button")
                                .attr("title", "Force-expire this device")
                                .on("click", "$store.ident.revoke(d.id)")
                                .child(Node::icon("trash").class("ph ph-trash")),
                        ])]),
            ]),
        ])
}

/// standalone demo — the panel as the chip's Profile… opens it.
pub fn demo() -> Node {
    m("div")
        .class("profile-demo")
        .init("$store.ident.panelOpen = true")
        .child(panel())
}

pub fn register() {
    css::inject("profile-panel", CSS);
}
