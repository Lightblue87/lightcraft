//! Headless tests of the compact (phone-width) layout: the photo keeps its room, the right panel
//! and Presets open in a full-width bottom sheet under it, My Photos is a full-width drawer, and a
//! desktop-sized window is unchanged.

use std::time::Duration;

use serde_json::json;

use crate::headless::Headless;
use crate::state::RightPanel;
use crate::{LightcraftApp, Services};

const T: Duration = Duration::from_secs(20);
const SETTLE: Duration = Duration::from_secs(120);
const PHONE: [f32; 2] = [402.0, 874.0];

fn demo(size: [f32; 2], ui: serde_json::Value) -> Headless {
    let services = Services { png: None, ..Default::default() };
    let app = LightcraftApp::new(lightcraft_engine::Session::with_demo(), services);
    let mut h = Headless::new(app, size, 1.0);
    let r = h.request("ui.set", ui, T);
    assert_eq!(r["ok"], true, "{r}");
    h.settle(SETTLE);
    h
}

fn widget(h: &Headless, id: &str) -> Option<egui::Rect> {
    h.app.widgets.iter().find(|(w, _)| w == id).map(|(_, r)| *r)
}

fn click(h: &mut Headless, id: &str) {
    let r = h.request("ui.clickWidget", json!({"id": id}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
    h.step();
}

#[test]
fn phone_width_puts_the_editor_in_a_sheet_under_the_photo() {
    let mut h = demo(PHONE, json!({"view": "detail", "right": "edit", "leftPanel": true}));
    assert!(crate::panels::compact::is_compact(&h.view.ctx));
    // entering the compact layout closed My Photos, which would cover the photo
    assert!(!h.app.ui.left_panel);
    let sheet = widget(&h, "panel:sheet").expect("the edit sheet is up");
    let right = widget(&h, "panel:right_panel").expect("the Edit panel is in the sheet");
    assert!(right.width() >= PHONE[0] - 2.0, "the Edit panel spans the width: {right:?}");
    assert!(sheet.top() > 300.0 && sheet.bottom() <= PHONE[1], "{sheet:?}");
    let canvas = h.app.canvas_rect.expect("the loupe is drawn");
    assert!(canvas.width() >= PHONE[0] - 2.0 && canvas.height() > 250.0, "the photo keeps its room: {canvas:?}");
    assert!(canvas.bottom() <= sheet.top() + 1.0, "the photo sits above the sheet");
    assert!(widget(&h, "slider:light.exposure").is_some(), "the Light sliders are in the sheet");
    // a tool in the sheet's row switches the panel; Presets replaces it at full width
    click(&mut h, "icon:crop");
    assert_eq!(h.app.ui.right, RightPanel::Crop);
    click(&mut h, "icon:presets");
    assert!(h.app.ui.presets);
    let presets = widget(&h, "panel:sheet").expect("still in the sheet");
    assert!(presets.width() >= PHONE[0] - 2.0);
    // Hide puts the sheet away and gives the photo the height back
    click(&mut h, "icon:sheetClose");
    assert!(!h.app.ui.presets && h.app.ui.right == RightPanel::None);
    h.step();
    assert!(widget(&h, "panel:sheet").is_none());
    assert!(h.app.canvas_rect.is_some_and(|c| c.height() > canvas.height() + 150.0));
    // the bottom bar's Edit button brings it back on the Edit panel
    click(&mut h, "icon:edit");
    assert_eq!(h.app.ui.right, RightPanel::Edit);
    assert!(widget(&h, "panel:sheet").is_some());
}

#[test]
fn phone_width_opens_my_photos_as_a_drawer() {
    let mut h = demo(PHONE, json!({"view": "photoGrid"}));
    click(&mut h, "icon:sidebar");
    assert!(h.app.ui.left_panel);
    let left = widget(&h, "panel:left_panel").expect("My Photos is open");
    assert!(left.width() >= PHONE[0] - 2.0, "full width: {left:?}");
    // picking a source closes it onto that source's photos
    click(&mut h, "source:picks");
    assert_eq!(h.app.session.source, lightcraft_engine::LibrarySource::Picks);
    assert!(!h.app.ui.left_panel);
}

#[test]
fn desktop_width_keeps_the_side_panels() {
    let h = demo([1400.0, 900.0], json!({"view": "detail", "right": "edit"}));
    assert!(!crate::panels::compact::is_compact(&h.view.ctx));
    assert!(widget(&h, "panel:sheet").is_none());
    let right = widget(&h, "panel:right_panel").expect("the right panel");
    assert!(right.width() < 600.0 && right.right() > 1300.0, "a column at the right: {right:?}");
    assert!(widget(&h, "icon:discord").is_some(), "the desktop top bar");
}

/// The dialog window's rectangle (`panels::dialogs` gives every dialog this id).
fn dialog_rect(h: &Headless) -> Option<egui::Rect> {
    h.view.ctx.memory(|m| m.area_rect(egui::Id::new("lightcraft-dialog")))
}

fn open(h: &mut Headless, command: &str) {
    let r = h.request("engine.execute", json!({"command": command, "params": {}}), T);
    assert_eq!(r["ok"], true, "{command}: {r}");
    for _ in 0..4 {
        h.step();
    }
}

#[test]
fn phone_width_dialogs_fit_the_screen() {
    let mut h = demo(PHONE, json!({"view": "detail"}));
    let r = h.request("engine.execute", json!({"command": "library.selectAll", "params": {}}), T);
    assert_eq!(r["ok"], true, "{r}");
    // the wide ones on the desktop: Settings (560 pt), Export (label column + button rows),
    // Shortcuts (a three-column table), Merge (a 520 pt preview beside its options), and two
    // ordinary ones
    for command in ["app.settings", "dialog.export", "app.shortcuts", "dialog.mergeHdr", "dialog.labelNames", "dialog.copySettings"] {
        open(&mut h, command);
        assert!(h.app.ui.dialog.is_some(), "{command} opened a dialog");
        let r = dialog_rect(&h).unwrap_or_else(|| panic!("{command}: no dialog window"));
        assert!(r.left() >= 0.0 && r.right() <= PHONE[0], "{command} fits across: {r:?}");
        assert!(r.top() >= 0.0 && r.bottom() <= PHONE[1], "{command} fits down: {r:?}");
        assert!(r.width() > PHONE[0] - 40.0, "{command} uses the width: {r:?}");
        h.request("ui.dialog.cancel", json!({}), T);
        h.step();
    }
    h.app.ui.dialog = Some(crate::state::Dialog::ConfirmDelete { count: 1 });
    for _ in 0..4 {
        h.step();
    }
    let r = dialog_rect(&h).expect("the delete confirmation");
    assert!(r.left() >= 0.0 && r.right() <= PHONE[0], "{r:?}");
}

#[test]
fn desktop_width_dialogs_keep_their_size() {
    let mut h = demo([1400.0, 900.0], json!({"view": "detail"}));
    open(&mut h, "app.settings");
    let r = dialog_rect(&h).expect("Settings");
    assert!(r.width() >= 560.0 && r.width() < 900.0, "Settings at its desktop width: {r:?}");
    h.request("ui.dialog.cancel", json!({}), T);
    h.step();
    open(&mut h, "dialog.labelNames");
    let r = dialog_rect(&h).expect("label names");
    assert!(r.width() < 600.0, "an ordinary dialog stays compact on the desktop: {r:?}");
}
