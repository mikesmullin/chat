use rustex1_kit::h::{lit, m, Node};

use crate::components::validation_info;
use crate::css;

pub const CSS: &str = r#"
.profile-demo { display: flex; flex-direction: column; gap: 12px; }
"#;

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
            validation_info::live("'error'", "$store.ident.error"),
            m("div").class("ae-field").kids(vec![
                m("label").class("ae-label").kids(vec![
                    m("span").child(lit("Location")),
                    m("span").class("hint").child(lit("where you are · shared with agents")),
                ]),
                m("div").class("ae-input-icon").kids(vec![
                    Node::icon("globe").class("ph ph-bold ph-globe ae-input-glyph"),
                    m("input")
                        .attr("type", "text")
                        .attr("placeholder", "City, State")
                        .model("$store.ident.location")
                        .on("input", "$store.ident.queueSave($event)"),
                ]),
            ]),
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
                            m("i").bind(
                                "class",
                                "((d.userAgent || '').includes('Mobile') ? 'ph ph-bold ph-device-mobile ae-device-icon' : 'ph ph-bold ph-laptop ae-device-icon')",
                            ),
                            m("span").class("ae-device-fp").text("d.fingerprint"),
                            m("span").class("ae-device-ip").text("d.ip"),
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

pub fn demo() -> Node {
    m("div")
        .class("profile-demo")
        .init("$store.ident.panelOpen = true")
        .child(panel())
}

pub fn register() {
    css::inject("profile-panel", CSS);
}
