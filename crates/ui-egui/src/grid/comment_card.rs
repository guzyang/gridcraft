//! The cell's comment card: the thread beside the cell, as Excel's modern comment box.
//!
//! A cell with a threaded comment carries a small triangle in its top-right corner, as Excel's
//! comment marker; pressing it opens the card, which lists what the thread says — every entry
//! with the name and the date it was written — and takes the next entry, so a comment is a
//! conversation kept in the cell instead of a side pane the user has to find.

use egui::{Align2, Color32, Painter, Pos2, Rect, Stroke, pos2};
use gridcraft_engine::core::CellRef;
use gridcraft_engine::model::Sheet;

use super::geo::Geo;
use crate::{SheetApp, json};

/// The size of a cell's comment marker, in points.
const MARKER: f32 = 7.0;

/// The marker in a cell's top-right corner, as Excel draws its comment triangle.
pub(super) fn marker_rect(sh: &Sheet, geo: &Geo, c: CellRef) -> Option<Rect> {
    sh.comments.get(&c).filter(|cm| cm.threaded)?;
    let r = geo.cell_rect(sh, c);
    if geo.hidden_under_frozen(sh, c) {
        return None;
    }
    Some(Rect::from_min_max(pos2(r.right() - MARKER - 1.0, r.top() + 1.0), pos2(r.right() - 1.0, r.top() + MARKER + 1.0)))
}

/// Draws the marker of every cell with a threaded comment.
pub(super) fn paint_markers(p: &Painter, sh: &Sheet, geo: &Geo) {
    for c in sh.comments.keys() {
        let Some(r) = marker_rect(sh, geo, *c) else { continue };
        if !p.clip_rect().intersects(r) {
            continue;
        }
        // A right triangle in the corner, the way Excel marks a commented cell.
        let points = [pos2(r.right(), r.top()), pos2(r.right(), r.bottom()), pos2(r.left(), r.bottom())];
        p.add(egui::Shape::convex_polygon(
            points.to_vec(),
            Color32::from_rgb(0xC0, 0x39, 0x5C),
            Stroke::new(0.5, Color32::from_rgb(0xC0, 0x39, 0x5C)),
        ));
    }
}

/// Draws the comment card of the cell it is open for: the thread, and the box the next entry
/// goes into. Closed by Esc or a press outside it.
pub(super) fn comment_card(app: &mut SheetApp, ui: &mut egui::Ui, geo: &Geo, was_open: bool) {
    let Some(c) = app.grid.comment_card else { return };
    let Some(d) = app.session.active() else { return };
    let wb = d.wb.clone();
    let si = wb.active_sheet();
    let Some(sh) = wb.sheet(si) else { return };
    let comment = sh.comments.get(&c).cloned();
    let cell_rect = geo.cell_rect(sh, c);
    // Beside the cell, as Excel's comment box sits; to its left when there is no room on the right.
    let screen = ui.ctx().content_rect();
    let width = 320.0;
    let on_right = cell_rect.right() + width + 12.0 <= screen.right();
    let (at, pivot) = if on_right {
        (pos2(cell_rect.right() + 10.0, cell_rect.top()), Align2::LEFT_TOP)
    } else {
        (pos2(cell_rect.left() - 10.0, cell_rect.top()), Align2::RIGHT_TOP)
    };
    let mut close = false;
    let response = egui::Area::new(egui::Id::unique(("comment_card", c.row, c.col))).fixed_pos(at).pivot(pivot).order(egui::Order::Foreground).show(
        ui.ctx(),
        |ui| {
            egui::Frame::popup(ui.style()).fill(Color32::WHITE).inner_margin(8.0).show(ui, |ui| {
                ui.set_max_width(width);
                match comment {
                    Some(cm) if cm.threaded => {
                        thread_entries(ui, &cm);
                        ui.separator();
                        reply_box(app, ui, c);
                    }
                    _ => {
                        ui.label(egui::RichText::new("Start a conversation").strong().color(Color32::BLACK));
                        ui.add_space(4.0);
                        new_comment_box(app, ui, c);
                    }
                }
            });
        },
    );
    // A press outside the card closes it, as does Esc; a press inside it does not.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || (was_open && response.response.clicked_elsewhere()) {
        close = true;
    }
    if close {
        app.grid.comment_card = None;
    }
}

/// What the thread says: the first entry and every reply, each with the name and the date it
/// was written, as Excel's comment box shows them.
fn thread_entries(ui: &mut egui::Ui, cm: &gridcraft_engine::model::Comment) {
    let entries = std::iter::once((cm.author.as_str(), cm.text.as_str())).chain(cm.replies.iter().map(|(a, t)| (a.as_str(), t.as_str())));
    for (i, (author, text)) in entries.enumerate() {
        if i > 0 {
            ui.separator();
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(author).strong().color(Color32::BLACK));
            // The thread's date first, then one per reply, in the order they were written.
            if let Some(date) = cm.dates.get(i).and_then(|d| gridcraft_engine::core::date::iso8601_local(d)) {
                ui.label(egui::RichText::new(date).small().color(Color32::from_gray(110)));
            }
        });
        ui.label(egui::RichText::new(text).color(Color32::BLACK));
    }
}

/// The box the next entry goes into. Enter starts a line, as in Excel's reply box; Cmd/Ctrl+Enter
/// (or Send) posts it.
fn reply_box(app: &mut SheetApp, ui: &mut egui::Ui, c: CellRef) {
    let key = egui::Id::unique(("comment_reply", c.row, c.col));
    let mut draft: String = ui.ctx().data_mut(|m| m.get_temp(key)).unwrap_or_default();
    let send = ui.small_button("Send").clicked();
    let r = ui.add(egui::TextEdit::multiline(&mut draft).hint_text("Reply…").desired_width(300.0).desired_rows(2));
    let cmd_enter = r.has_focus() && ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Enter));
    if (send || cmd_enter) && !draft.trim().is_empty() {
        app.run_or_alert("review.replyComment", json!({"cell": c.a1(), "text": draft.trim().to_string()}));
        draft.clear();
    }
    ui.ctx().data_mut(|m| m.insert_temp(key, draft));
}

/// The box the first entry goes into, for a cell that has no comment yet.
fn new_comment_box(app: &mut SheetApp, ui: &mut egui::Ui, c: CellRef) {
    let key = egui::Id::unique(("comment_new", c.row, c.col));
    let mut draft: String = ui.ctx().data_mut(|m| m.get_temp(key)).unwrap_or_default();
    let r = ui.add(egui::TextEdit::multiline(&mut draft).hint_text("Type a comment…").desired_width(300.0).desired_rows(3));
    let post = ui.small_button("Post").clicked();
    let cmd_enter = r.has_focus() && ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Enter));
    if (post || cmd_enter) && !draft.trim().is_empty() {
        app.run_or_alert("review.newComment", json!({"cell": c.a1(), "text": draft.trim().to_string()}));
        draft.clear();
    }
    ui.ctx().data_mut(|m| m.insert_temp(key, draft));
}

/// The corner of a cell the marker sits in, for the press that opens the card.
pub(super) fn marker_hit(sh: &Sheet, geo: &Geo, c: CellRef, p: Pos2) -> bool {
    marker_rect(sh, geo, c).is_some_and(|r| r.contains(p))
}
