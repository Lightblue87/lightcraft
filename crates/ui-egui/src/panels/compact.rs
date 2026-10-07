//! The compact layout: phones, and any window narrower than [`COMPACT_WIDTH`] (iPad Split View, a
//! phone browser). The desktop layout's side columns leave such a window no room for the photo,
//! so here:
//!
//! - the top bar keeps the sidebar toggle, Back, the menus (one "Menu" button), a search field
//!   that takes the free width, Filter and Export;
//! - the tool strip goes; its tools sit in a row atop a **bottom sheet** under the photo, which
//!   holds the right panel (Edit, Crop, Masking …) or the Presets column at full width;
//! - My Photos (the left panel) opens as a full-width drawer, closed again by picking a source;
//! - the bottom bar keeps the view modes, Sort and an Edit toggle.
//!
//! The panels themselves are the desktop ones: [`super::resizable_side`] and the Presets column
//! fill the region they're given while [`is_compact`] holds.

use egui::{Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use crate::LightcraftApp;
use crate::icons::{Icon, paint};
use crate::state::{RightPanel, ViewMode};
use crate::theme::Tokens;
use crate::widgets::{icon_button, register};

/// Content width (points, inside the safe area) below which the compact layout is used.
pub const COMPACT_WIDTH: f32 = 600.0;

/// Touch-friendly button size in the compact bars.
const BUTTON: egui::Vec2 = egui::vec2(40.0, 40.0);

fn flag_id() -> egui::Id {
    egui::Id::new("compact-layout")
}

/// Whether this frame uses the compact layout.
pub fn is_compact(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(flag_id())).unwrap_or(false)
}

/// Decide the layout for this frame from the content width. Entering the compact layout closes
/// My Photos, which would otherwise cover the photo as a full-width drawer.
pub(crate) fn update(app: &mut LightcraftApp, ctx: &egui::Context, content_width: f32) {
    let compact = content_width < COMPACT_WIDTH;
    if compact && !is_compact(ctx) {
        app.ui.left_panel = false;
    }
    ctx.data_mut(|d| d.insert_temp(flag_id(), compact));
}

/// The bottom sheet is up: Detail view with a right panel or Presets open.
pub fn sheet_open(app: &LightcraftApp) -> bool {
    app.ui.view == ViewMode::Detail && (app.ui.right != RightPanel::None || app.ui.presets)
}

/// The window's chrome in the compact layout; the caller fills the central area as usual.
pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    topbar(app, ui);
    super::library_problem::banner(app, ui);
    bottombar(app, ui);
    if app.ui.left_panel {
        // picking a source (album, folder, date …) closes the drawer onto its photos
        let source = app.session.source;
        super::left::show(app, ui);
        if app.session.source != source {
            app.ui.left_panel = false;
        }
    } else if sheet_open(app) {
        sheet(app, ui);
    }
}

fn topbar(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("top_bar")
        .exact_size(t.top_bar_h + 4.0)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 4, right: 4, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if icon_button(ui, "sidebar", Icon::Sidebar, BUTTON, app.ui.left_panel, true, "Show/hide My Photos panel").clicked() {
                    let _ = app.run("view.leftPanel", json!({}));
                }
                if icon_button(ui, "back", Icon::Back, BUTTON, false, true, "Back").clicked() {
                    let _ = app.run("view.back", json!({}));
                }
                if !app.native_menu {
                    ui.add_space(4.0);
                    crate::menubar::show_in_window(app, ui, 0.0);
                    ui.add_space(8.0);
                }
                // right-to-left from the edge: Export, Filter, then the search field takes the rest
                let rest = ui.available_rect_before_wrap();
                let mut right = ui.new_child(egui::UiBuilder::new().max_rect(rest).layout(egui::Layout::right_to_left(egui::Align::Center)));
                right.spacing_mut().item_spacing.x = 0.0;
                if icon_button(&mut right, "share", Icon::Share, BUTTON, false, true, "Export").clicked() {
                    let _ = app.run("dialog.export", json!({}));
                }
                let filtering = app.session.filter != Default::default() || app.ui.filter_bar;
                if icon_button(&mut right, "filter", Icon::Filter, BUTTON, filtering, true, "Filter bar").clicked() {
                    let _ = app.run("view.filterBar", json!({}));
                }
                // a failing save keeps its warning visible (the desktop's cloud icon)
                if let Some((n, e)) = app.session.unsaved() {
                    let (r, resp) = right.allocate_exact_size(BUTTON, Sense::hover());
                    paint(right.painter(), r.shrink(10.0), Icon::Cloud, t.caution);
                    register(right.ctx(), "indicator:unsaved", r);
                    resp.on_hover_text(format!("{n} unsaved: {e}"));
                }
                right.add_space(4.0);
                let field = right.available_rect_before_wrap();
                if field.width() >= 80.0 {
                    search(app, &mut right, Rect::from_center_size(field.center(), vec2(field.width(), 32.0)));
                }
            });
        });
}

/// The search field in `r` (same id and command as the desktop top bar's).
fn search(app: &mut LightcraftApp, ui: &mut egui::Ui, r: Rect) {
    let t = Tokens::get(ui.ctx());
    let id = egui::Id::new("search-field");
    if std::mem::take(&mut app.ui.focus_search) {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let focused = ui.memory(|m| m.has_focus(id));
    ui.painter().rect(
        r,
        6.0,
        if focused { t.canvas } else { t.field },
        Stroke::new(1.0, if focused { t.accent } else { t.field_border }),
        StrokeKind::Inside,
    );
    register(ui.ctx(), "field:search", r);
    let inner = r.shrink2(vec2(10.0, 4.0));
    if app.ui.search.is_empty() && !focused {
        paint(ui.painter(), Rect::from_min_size(pos2(inner.left(), r.center().y - 8.0), vec2(16.0, 16.0)), Icon::Search, t.text_dim);
    }
    let text = Rect::from_min_max(pos2(inner.left() + 22.0, inner.top()), inner.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let resp = child.add(
        egui::TextEdit::singleline(&mut app.ui.search)
            .id(id)
            .frame(egui::Frame::NONE)
            .hint_text(crate::i18n::tr("Search"))
            .desired_width(text.width())
            .font(t.font(14.0))
            .text_color(t.text),
    );
    if resp.changed() {
        let q = app.ui.search.clone();
        let _ = app.run("library.filter", json!({"text": q}));
    }
}

fn bottombar(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("bottom_bar")
        .exact_size(t.bottom_bar_h + 4.0)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 6, right: 6, top: 0, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (id, icon, mode, tip) in [
                    ("photoGrid", Icon::GridPhoto, ViewMode::PhotoGrid, "Photo Grid (G)"),
                    ("squareGrid", Icon::GridSquare, ViewMode::SquareGrid, "Square Grid (G toggles)"),
                    ("detail", Icon::Single, ViewMode::Detail, "Detail (D)"),
                ] {
                    if icon_button(ui, id, icon, BUTTON, app.ui.view == mode, true, tip).clicked() {
                        let _ = app.run(&format!("view.{id}"), json!({}));
                    }
                }
                let sort = icon_button(ui, "sort", Icon::Sort, BUTTON, false, true, "Sort");
                egui::Popup::menu(&sort).show(|ui| super::bottombar::sort_menu(app, ui));
            });
            let mut right = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_max(pos2(full.center().x, full.top()), full.max))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            right.spacing_mut().item_spacing.x = 2.0;
            let has_photo = app.session.active().is_some();
            // Edit: open the sheet on the Edit panel (from the grid too), or put it away
            if icon_button(&mut right, "edit", Icon::Sliders, BUTTON, sheet_open(app), has_photo, "Edit (E)").clicked() {
                if sheet_open(app) {
                    close_sheet(app);
                } else {
                    if app.ui.view != ViewMode::Detail {
                        let _ = app.run("view.detail", json!({}));
                    }
                    if app.ui.right == RightPanel::None && !app.ui.presets {
                        let _ = app.run("panel.edit", json!({}));
                    }
                }
            }
            if app.ui.view == ViewMode::Detail
                && icon_button(&mut right, "filmstrip", Icon::Filmstrip, BUTTON, app.ui.filmstrip, true, "Filmstrip").clicked()
            {
                app.ui.filmstrip = !app.ui.filmstrip;
            }
        });
}

fn close_sheet(app: &mut LightcraftApp) {
    app.ui.right = RightPanel::None;
    app.ui.presets = false;
}

/// The bottom sheet: a row of tools, then the right panel or Presets at full width.
fn sheet(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let avail = ui.available_rect_before_wrap();
    // the photo keeps the upper part; the sheet gets the rest, within reason
    let height = (avail.height() * 0.5).clamp(220.0, (avail.height() - 160.0).max(220.0));
    egui::Panel::bottom("compact_sheet")
        .exact_size(height)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider)))
        .show(ui, |ui| {
            register(ui.ctx(), "panel:sheet", ui.max_rect());
            egui::Panel::top("compact_sheet_tools")
                .exact_size(BUTTON.y + 4.0)
                .frame(egui::Frame::NONE.inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 2 }))
                .show(ui, |ui| tools(app, ui));
            if app.ui.presets {
                super::presets::show(app, ui);
            } else {
                super::right::show(app, ui);
            }
        });
}

fn tools(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let has_photo = app.session.active().is_some();
    let full = ui.max_rect();
    egui::ScrollArea::horizontal().id_salt("compact-tools").auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "presets", Icon::Presets, BUTTON, app.ui.presets, has_photo, "Presets (Shift+P)").clicked() {
                let _ = app.run("panel.presets", json!({}));
            }
            for (id, icon, panel, tip) in [
                ("edit", Icon::Sliders, RightPanel::Edit, "Edit (E)"),
                ("crop", Icon::Crop, RightPanel::Crop, "Crop & Rotate (C)"),
                ("remove", Icon::Eraser, RightPanel::Remove, "Remove (H)"),
                ("masking", Icon::Mask, RightPanel::Masking, "Masking (M)"),
                ("redeye", Icon::Eye, RightPanel::RedEye, "Red Eye"),
                ("info", Icon::Info, RightPanel::Info, "Info (I)"),
                ("keywords", Icon::Tag, RightPanel::Keywords, "Keywords (K)"),
                ("versions", Icon::Versions, RightPanel::Versions, "Versions (Shift+V)"),
                ("activity", Icon::Activity, RightPanel::Activity, "History & Activity (Y)"),
            ] {
                let on = !app.ui.presets && (app.ui.right == panel || (panel == RightPanel::Edit && app.ui.right == RightPanel::Profiles));
                if icon_button(ui, id, icon, BUTTON, on, has_photo || panel == RightPanel::Activity, tip).clicked() {
                    // one thing at a time in the sheet: a tool replaces Presets instead of toggling off
                    app.ui.presets = false;
                    if !on {
                        let _ = app.run(&format!("panel.{id}"), json!({}));
                    }
                }
            }
            // room to scroll the last tool out from under the Hide button
            ui.add_space(BUTTON.x);
        });
    });
    // put the sheet away (pinned to the row's right end, over the scrolling tools)
    let r = Rect::from_min_size(pos2(full.right() - BUTTON.x, full.center().y - BUTTON.y / 2.0), BUTTON);
    let t = Tokens::get(ui.ctx());
    ui.painter().rect_filled(r, 0.0, t.chrome);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
    if icon_button(&mut child, "sheetClose", Icon::ChevronDown, BUTTON, false, true, "Hide").clicked() {
        close_sheet(app);
    }
}
