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

fn exposure(h: &Headless) -> f64 {
    let id = h.app.session.active().expect("a photo in Detail");
    h.app.session.develop_of(id).unwrap_or_default().light.exposure
}

/// The edit sheet scrolled (mouse wheel) so the Light sliders are clear of the bottom bar; the
/// Exposure track's rectangle.
fn exposure_in_view(h: &mut Headless) -> egui::Rect {
    let sheet = widget(h, "panel:sheet").expect("the edit sheet");
    let r = h.request("ui.move", json!({"x": sheet.center().x, "y": sheet.center().y}), T);
    assert_eq!(r["ok"], true, "{r}");
    let r = h.request("ui.scroll", json!({"dy": -220.0}), T);
    assert_eq!(r["ok"], true, "{r}");
    for _ in 0..30 {
        h.step();
    }
    let track = widget(h, "slider:light.exposure").expect("the Exposure slider");
    assert!(track.bottom() < sheet.bottom() - 60.0, "Exposure scrolled into view: {track:?} in {sheet:?}");
    track
}

/// One finger from `from` to `to` in `steps` moves, then lifted.
fn touch_drag(h: &mut Headless, from: egui::Pos2, to: egui::Pos2, steps: usize) {
    let touch = |phase, pos| egui::Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(1), phase, pos, force: None };
    let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    h.input([touch(egui::TouchPhase::Start, from), egui::Event::PointerMoved(from), button(from, true)]);
    h.step();
    for i in 1..=steps {
        let p = from + (to - from) * (i as f32 / steps as f32);
        h.input([touch(egui::TouchPhase::Move, p), egui::Event::PointerMoved(p)]);
        h.step();
    }
    h.input([touch(egui::TouchPhase::End, to), button(to, false), egui::Event::PointerGone]);
    h.step();
    h.step();
}

#[test]
fn touch_swipe_up_on_a_slider_scrolls_the_sheet_instead() {
    let mut h = demo(PHONE, json!({"view": "detail", "right": "edit"}));
    let before = exposure(&h);
    let track = exposure_in_view(&mut h);
    // start on the track, left of the thumb, and swipe up
    let from = egui::pos2(track.left() + 20.0, track.center().y);
    touch_drag(&mut h, from, from - egui::vec2(4.0, 160.0), 16);
    assert_eq!(exposure(&h), before, "the value didn't move");
    let moved = widget(&h, "slider:light.exposure").expect("still on screen or scrolled");
    assert!(moved.top() < track.top() - 100.0, "the sheet scrolled: {track:?} → {moved:?}");
}

#[test]
fn touch_swipe_sideways_on_a_slider_adjusts_it_by_the_distance() {
    let mut h = demo(PHONE, json!({"view": "detail", "right": "edit"}));
    let before = exposure(&h);
    let track = exposure_in_view(&mut h);
    // from near the left end, 40 pt right: by touch the value moves by 40 pt's worth, rather
    // than jumping to the finger (which would set it near the minimum)
    let from = egui::pos2(track.left() + 10.0, track.center().y);
    touch_drag(&mut h, from, from + egui::vec2(40.0, 2.0), 8);
    let after = exposure(&h);
    let spec = lightcraft_develop::controls::find("light.exposure").expect("exposure spec");
    let expected = 40.0 / track.width() as f64 * (spec.max - spec.min);
    assert!(after > before, "it went up: {before} → {after}");
    assert!((after - before - expected).abs() < expected * 0.4, "by about {expected}: {before} → {after}");
    // the slider stayed put: no scrolling
    assert_eq!(widget(&h, "slider:light.exposure").map(|r| r.top()), Some(track.top()));
}

#[test]
fn touch_tap_on_a_slider_leaves_the_value() {
    let mut h = demo(PHONE, json!({"view": "detail", "right": "edit"}));
    let before = exposure(&h);
    let track = exposure_in_view(&mut h);
    let at = egui::pos2(track.left() + 10.0, track.center().y);
    touch_drag(&mut h, at, at, 0);
    assert_eq!(exposure(&h), before);
}

#[test]
fn mouse_click_on_a_slider_still_sets_the_value_there() {
    // no touch input in this session: the desktop behaviour
    let mut h = demo(PHONE, json!({"view": "detail", "right": "edit"}));
    let before = exposure(&h);
    let track = exposure_in_view(&mut h);
    let r = h.request("ui.click", json!({"x": track.left() + 2.0, "y": track.center().y}), T);
    assert_eq!(r["ok"], true, "{r}");
    for _ in 0..4 {
        h.step();
    }
    assert!(exposure(&h) < before - 1.0, "set near the minimum: {before} → {}", exposure(&h));
}

/// The soft keyboard, as a phone shell reports it: 330 pt of the screen's bottom.
const KEYBOARD: f32 = 330.0;

fn keyboard_up(h: &mut Headless) {
    h.set_safe_area(egui::SafeAreaInsets(egui::epaint::MarginF32 { bottom: KEYBOARD, ..Default::default() }));
    for _ in 0..8 {
        h.step();
    }
}

fn focus(h: &mut Headless, field: &str) {
    let r = h.request("ui.clickWidget", json!({"id": field}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
    h.step();
    assert!(h.view.ctx.memory(|m| m.focused()).is_some(), "{field} has focus");
}

#[test]
fn keyboard_lifts_the_layout_and_keeps_a_sheet_field_in_view() {
    let mut h = demo(PHONE, json!({"view": "detail", "right": "info"}));
    let sheet = widget(&h, "panel:sheet").expect("the Info sheet");
    // the lowest text field visible in the sheet, which a 330 pt keyboard would cover
    let (field, rect) = h
        .app
        .widgets
        .iter()
        .filter(|(id, r)| id.starts_with("field:") && sheet.contains_rect(*r))
        .max_by(|a, b| a.1.bottom().total_cmp(&b.1.bottom()))
        .map(|(id, r)| (id.clone(), *r))
        .expect("a text field in the Info panel");
    assert!(rect.bottom() > PHONE[1] - KEYBOARD, "{field} starts where the keyboard goes: {rect:?}");
    focus(&mut h, &field);
    keyboard_up(&mut h);
    let content = h.view.ctx.content_rect();
    assert_eq!(content.bottom(), PHONE[1] - KEYBOARD, "the content area ends at the keyboard");
    let bar = widget(&h, "icon:photoGrid").expect("the bottom bar");
    assert!(bar.bottom() <= content.bottom(), "the bottom bar sits on the keyboard: {bar:?}");
    let now = widget(&h, &field).expect("the field");
    assert!(
        now.bottom() <= content.bottom() && now.top() >= h.app.canvas_rect.map_or(0.0, |c| c.bottom()),
        "{field} in view above the keyboard: {now:?}"
    );
}

#[test]
fn keyboard_keeps_a_dialog_field_in_view() {
    let mut h = demo(PHONE, json!({"view": "detail"}));
    open(&mut h, "dialog.rename");
    focus(&mut h, "field:renameTemplate");
    keyboard_up(&mut h);
    let content = h.view.ctx.content_rect();
    let dialog = dialog_rect(&h).expect("the Rename dialog");
    assert!(dialog.bottom() <= content.bottom() && dialog.top() >= 0.0, "the dialog sits above the keyboard: {dialog:?}");
    let field = widget(&h, "field:renameTemplate").expect("the template field");
    assert!(field.bottom() <= content.bottom(), "the field is in view: {field:?}");
}
