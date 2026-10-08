//! The bottom bar: view modes, sort, rating/flags pill, copy/paste settings, zoom and view toggles.

use egui::{Align2, Rect, Sense, pos2, vec2};
use lightcraft_catalog::Flag;
use serde_json::json;

use crate::LightcraftApp;
use crate::icons::{Icon, paint};
use crate::state::{BeforeAfter, ViewMode, Zoom};
use crate::theme::Tokens;
use crate::widgets::{icon_button, register, stars};

pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("bottom_bar")
        .exact_size(t.bottom_bar_h)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            let modes = [
                ("photoGrid", Icon::GridPhoto, ViewMode::PhotoGrid, "Photo Grid (G)"),
                ("squareGrid", Icon::GridSquare, ViewMode::SquareGrid, "Square Grid (G toggles)"),
                ("detail", Icon::Single, ViewMode::Detail, "Detail (D)"),
                ("compare", Icon::Compare, ViewMode::Compare, "Compare (Shift+C)"),
                ("survey", Icon::Survey, ViewMode::Survey, "Survey (N)"),
                ("people", Icon::Subject, ViewMode::People, "People"),
            ];
            let left_end = ui
                .horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if full.width() < NARROW_W {
                        // no room for six buttons next to the zoom group (an iPad in portrait with
                        // My Photos open): one button with the current mode opens the list
                        let (_, icon, _, _) = modes.iter().find(|m| m.2 == app.ui.view).copied().unwrap_or(modes[0]);
                        let b = icon_button(ui, "viewMode", icon, vec2(32.0, 32.0), true, true, "View mode");
                        egui::Popup::menu(&b).show(|ui| {
                            for (id, _, mode, tip) in modes {
                                if ui.selectable_label(app.ui.view == mode, crate::i18n::tr(tip)).clicked() {
                                    let _ = app.run(&format!("view.{id}"), json!({}));
                                }
                            }
                        });
                    } else {
                        for (id, icon, mode, tip) in modes {
                            if icon_button(ui, id, icon, vec2(32.0, 32.0), app.ui.view == mode, true, tip).clicked() {
                                let _ = app.run(&format!("view.{id}"), json!({}));
                            }
                        }
                    }
                    ui.add_space(10.0);
                    let (sep, _) = ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, t.button_border);
                    ui.add_space(10.0);
                    let sort = icon_button(ui, "sort", Icon::Sort, vec2(30.0, 30.0), false, true, "Sort");
                    egui::Popup::menu(&sort).show(|ui| sort_menu(app, ui));
                })
                .response
                .rect
                .right();
            let right_start = right_side(app, ui, full, left_end + 12.0);
            centre(app, ui, full, left_end + 12.0, right_start - 12.0);
        });
}

/// Below this width the view-mode buttons collapse into one (they and the zoom group need ~420).
const NARROW_W: f32 = 440.0;

/// The rating/flag pill's parts: five stars, a gap, three flag buttons, `PILL_GAP` apart.
const STAR: f32 = 19.0;
const FLAG: f32 = 22.0;
const PILL_GAP: f32 = 3.0;
const GROUP_GAP: f32 = 6.0;
/// Space between the pill's edge and its first / last button. The right side is 3 pt less: the
/// flag glyphs sit 3 pt inside their buttons and the stars hardly any, so the visible gaps match.
const PILL_PAD: f32 = 12.0;
const PILL_PAD_R: f32 = PILL_PAD - 3.0;
/// Width of the pill: its contents plus the padding (it was a fixed 196 pt, narrower than its
/// contents, so the last flag touched the edge).
const PILL_W: f32 = PILL_PAD + 5.0 * STAR + 5.0 * PILL_GAP + GROUP_GAP + 3.0 * FLAG + 2.0 * PILL_GAP + PILL_PAD_R;

/// The rating/flag pill and copy/paste settings, between `from` and `to` (the side groups): the
/// copy buttons go first when there is no room (they are in the Edit menu too), then the pill.
fn centre(app: &mut LightcraftApp, ui: &mut egui::Ui, full: Rect, from: f32, to: f32) {
    let t = Tokens::get(ui.ctx());
    let has_clip = app.session.clipboard.is_some();
    let label = crate::i18n::tr(if has_clip { "Paste Edit Settings" } else { "Copy Edit Settings" });
    let copy_width = (ui.painter().layout_no_wrap(label.to_string(), t.font(13.0), t.text_label).size().x + 24.0).max(136.0);
    let centre_width = PILL_W + 10.0 + copy_width + 4.0 + 30.0;
    let room = to - from;
    if room < PILL_W {
        return;
    }
    let with_copy = room >= centre_width;
    let w = if with_copy { centre_width } else { PILL_W };
    // where it sits with room to spare (slightly left of centre), kept between the side groups
    let left = (full.center().x - 60.0 - PILL_W / 2.0).clamp(from, to - w);
    let active = app.session.active().and_then(|id| app.session.catalog.photo(id).cloned());
    let pill = Rect::from_min_size(pos2(left, full.center().y - 15.0), vec2(PILL_W, 30.0));
    ui.painter().rect_filled(pill, 15.0, t.canvas);
    register(ui.ctx(), "pill:rating", pill);
    let inner = Rect::from_min_max(pos2(pill.left() + PILL_PAD, pill.top() + 4.0), pos2(pill.right() - PILL_PAD_R, pill.bottom() - 4.0));
    let mut pill_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    pill_ui.spacing_mut().item_spacing.x = PILL_GAP;
    let rating = active.as_ref().map(|p| p.rating).unwrap_or(0);
    if let Some(r) = stars(&mut pill_ui, "bottom", rating, STAR) {
        let mut params = json!({"rating": r});
        crate::panels::compare::target_active(app, &mut params);
        let _ = app.run("photo.rate", params);
    }
    pill_ui.add_space(GROUP_GAP);
    let flag = active.as_ref().map(|p| p.flag).unwrap_or_default();
    for (id, icon, f) in [("unflag", Icon::Circle, Flag::None), ("pick", Icon::FlagPick, Flag::Pick), ("reject", Icon::FlagReject, Flag::Reject)] {
        let (r, resp) = pill_ui.allocate_exact_size(vec2(FLAG, FLAG), Sense::click());
        register(pill_ui.ctx(), format!("flag:{id}"), r);
        let on = flag == f && f != Flag::None;
        let c = if on {
            if f == Flag::Reject { t.reject } else { t.pick }
        } else if resp.hovered() {
            t.text
        } else {
            t.icon
        };
        paint(pill_ui.painter(), r.shrink(3.0), icon, c);
        if resp.clicked() {
            let mut params = json!({"flag": match f { Flag::Pick => "pick", Flag::Reject => "reject", Flag::None => "none" }});
            crate::panels::compare::target_active(app, &mut params);
            let _ = app.run("photo.flag", params);
        }
    }
    // copy / paste settings
    if !with_copy {
        return;
    }
    let copy_r = Rect::from_min_size(pos2(pill.right() + 10.0, pill.top()), vec2(copy_width, 30.0));
    let cresp = ui.interact(copy_r, egui::Id::new("copy-settings"), Sense::click());
    register(ui.ctx(), "button:copySettings", copy_r);
    ui.painter().rect_filled(copy_r, 15.0, if cresp.hovered() { t.hover } else { t.canvas });
    ui.painter().text(copy_r.center(), Align2::CENTER_CENTER, label, t.font(13.0), if active.is_some() { t.text_label } else { t.text_disabled });
    if cresp.clicked() && active.is_some() {
        let _ = if has_clip { app.run("develop.paste", json!({})) } else { app.run("develop.copy", json!({})) };
        let msg = if has_clip { "Settings pasted" } else { "Edit settings copied" };
        app.toast(ui.ctx(), crate::i18n::tr(msg));
    }
    let gear_r = Rect::from_min_size(pos2(copy_r.right() + 4.0, pill.top()), vec2(30.0, 30.0));
    let gresp = ui.interact(gear_r, egui::Id::new("copy-gear"), Sense::click());
    register(ui.ctx(), "icon:copyGear", gear_r);
    ui.painter().rect_filled(gear_r, 15.0, if gresp.hovered() { t.hover } else { t.canvas });
    paint(ui.painter(), gear_r.shrink(7.0), Icon::Gear, t.icon);
    if gresp.clicked() {
        let _ = app.run("dialog.copySettings", json!({}));
    }
}

/// The right-hand group (zoom, view toggles or thumbnail size), right of `from` (the left group's
/// end); returns its left edge. Where there is no room (an iPad in portrait with My Photos open)
/// Face boxes and Click 1:1 are left out first: both are in the View menu too.
fn right_side(app: &mut LightcraftApp, ui: &mut egui::Ui, full: Rect, from: f32) -> f32 {
    let t = Tokens::get(ui.ctx());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(pos2(from.max(full.right() - 360.0).min(full.right()), full.top()), full.right_bottom()))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 6.0;
    if matches!(app.ui.view, ViewMode::Compare | ViewMode::Survey)
        && icon_button(
            &mut child,
            "autoAdvance",
            Icon::ChevronRight,
            vec2(30.0, 30.0),
            app.ui.auto_advance,
            true,
            "Auto Advance after rating or flagging",
        )
        .clicked()
    {
        let _ = app.run("view.autoAdvance", json!({}));
    }
    if app.ui.view == ViewMode::Survey {
        // no zoom or thumbnail size in Survey
    } else if matches!(app.ui.view, ViewMode::Detail | ViewMode::Compare) {
        // Before/After only with room for it, Filmstrip and the zoom menu
        if child.available_width() >= 36.0 + 36.0 + 10.0 + 64.0
            && icon_button(
                &mut child,
                "beforeAfter",
                Icon::BeforeAfter,
                vec2(30.0, 30.0),
                app.ui.before_after != BeforeAfter::Off,
                true,
                "Before/After (Y)",
            )
            .clicked()
        {
            let _ = app.run("view.beforeAfter", json!({}));
        }
        // room left for Face boxes and what has to follow it (Filmstrip, the zoom menu)
        if app.ui.view == ViewMode::Detail
            && child.available_width() >= 36.0 + 36.0 + 10.0 + 64.0
            && icon_button(&mut child, "faceBoxes", Icon::FaceBox, vec2(30.0, 30.0), app.ui.face_boxes, true, "Face boxes").clicked()
        {
            let _ = app.run("view.faceBoxes", json!({}));
        }
        if icon_button(&mut child, "filmstrip", Icon::Filmstrip, vec2(30.0, 30.0), app.ui.filmstrip, true, "Filmstrip (/)").clicked() {
            let _ = app.run("view.filmstrip", json!({}));
        }
        child.add_space(10.0);
        let zoom_label = match app.ui.zoom {
            Zoom::Fit => "Fit".to_string(),
            Zoom::Fill => "Fill".to_string(),
            Zoom::Percent(p) => format!("{p:.0}%"),
        };
        let zr = crate::widgets::dropdown(&mut child, "zoom", crate::i18n::tr(&zoom_label), t.font(13.0), t.text_label);
        egui::Popup::menu(&zr).show(|ui| {
            for (label, z) in [
                ("Fit", Zoom::Fit),
                ("Fill", Zoom::Fill),
                ("50%", Zoom::Percent(50.0)),
                ("100%", Zoom::Percent(100.0)),
                ("200%", Zoom::Percent(200.0)),
                ("400%", Zoom::Percent(400.0)),
            ] {
                if ui.selectable_label(app.ui.zoom == z, label).clicked() {
                    app.ui.zoom = z;
                    app.ui.pan = (0.5, 0.5);
                }
            }
        });
        if child.available_width() < 6.0 + 90.0 {
            return child.min_rect().left();
        }
        child.add_space(6.0);
        let cz = crate::widgets::dropdown(
            &mut child,
            "clickZoom",
            &crate::i18n::tr_format!("Click {}:1", app.ui.click_zoom / 100),
            t.font(13.0),
            t.text_label,
        );
        egui::Popup::menu(&cz).show(|ui| {
            for pct in crate::state::CLICK_ZOOMS {
                if ui.selectable_label(app.ui.click_zoom == pct, format!("{}:1", pct / 100)).clicked() {
                    let _ = app.run("view.clickZoom", json!({"ratio": pct / 100}));
                }
            }
        });
    } else {
        // thumbnail size slider
        let (r, resp) = child.allocate_exact_size(vec2(110.0, 20.0), Sense::click_and_drag());
        register(child.ctx(), "slider:thumbSize", r);
        let p = child.painter();
        let y = r.center().y;
        p.line_segment([pos2(r.left(), y), pos2(r.right(), y)], egui::Stroke::new(2.0, t.track));
        let f = (app.ui.thumb_size - 90.0) / (480.0 - 90.0);
        let x = r.left() + f * r.width();
        p.rect_filled(Rect::from_center_size(pos2(x, y), vec2(3.0, 14.0)), 1.0, t.thumb_hover);
        if let Some(pp) = resp.interact_pointer_pos()
            && (resp.dragged() || resp.clicked())
        {
            app.ui.thumb_size = 90.0 + ((pp.x - r.left()) / r.width()).clamp(0.0, 1.0) * (480.0 - 90.0);
        }
    }
    if child.min_rect().width() > 0.0 { child.min_rect().left() } else { full.right() }
}

pub(crate) fn sort_menu(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    use lightcraft_catalog::GroupBy;
    use lightcraft_catalog::SortKey::*;
    let cur = app.session.sort;
    ui.label(egui::RichText::new(crate::i18n::tr("Sort by")).weak());
    for (label, key, k) in [
        ("Capture Date", CaptureDate, "captureDate"),
        ("Import Date", ImportDate, "importDate"),
        ("Modified Date", EditDate, "editDate"),
        ("File Name", FileName, "fileName"),
        ("Rating", Rating, "rating"),
        ("File Size", FileSize, "fileSize"),
        ("Random", Random, "random"),
    ] {
        if ui.selectable_label(cur.key == key, crate::i18n::tr(label)).clicked() {
            let _ = app.run("library.sort", json!({"key": k}));
        }
    }
    if cur.key == Random && ui.button(crate::i18n::tr("Reshuffle")).clicked() {
        let _ = app.run("library.shuffle", json!({}));
    }
    ui.separator();
    // a shuffle has no direction worth choosing
    ui.add_enabled_ui(cur.key != Random, |ui| {
        if ui.selectable_label(cur.key != Random && cur.ascending, crate::i18n::tr("Ascending")).clicked() {
            let _ = app.run("library.sort", json!({"ascending": true}));
        }
        if ui.selectable_label(cur.key != Random && !cur.ascending, crate::i18n::tr("Descending")).clicked() {
            let _ = app.run("library.sort", json!({"ascending": false}));
        }
    });
    ui.separator();
    ui.label(egui::RichText::new(crate::i18n::tr("Group by date")).weak());
    for (label, g, k) in [
        ("Automatic", GroupBy::Auto, "auto"),
        ("Day", GroupBy::Day, "day"),
        ("Month", GroupBy::Month, "month"),
        ("Year", GroupBy::Year, "year"),
        ("None", GroupBy::None, "none"),
    ] {
        if ui.selectable_label(cur.group == g, crate::i18n::tr(label)).clicked() {
            let _ = app.run("library.sort", json!({"group": k}));
        }
    }
}
