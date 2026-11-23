//! The cell's comment card: the thread beside the cell, and the box the next entry goes into.

use egui::vec2;
use egui_kittest::kittest::Queryable;
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

const WIDTH: f32 = 1200.0;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = crate::common::builder().with_step_dt(1.0 / 60.0).with_size(vec2(WIDTH, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    h
}

fn cell(a1: &str) -> gridcraft_engine::core::CellRef {
    gridcraft_engine::core::CellRef::parse(a1).unwrap()
}

/// The card shows what the thread says: every entry with the name it was written under, and a
/// box the next entry goes into.
#[test]
fn the_card_lists_the_thread_with_its_names_and_takes_the_next_entry() {
    let mut h = harness();
    h.state_mut().run("review.newComment", json!({"cell": "B2", "text": "first entry", "author": "Ada"})).unwrap();
    h.state_mut().run("review.replyComment", json!({"cell": "B2", "text": "second entry", "author": "Bob"})).unwrap();
    h.state_mut().grid.comment_card = Some(cell("B2"));
    h.run_steps(6);
    for want in ["Ada", "Bob", "first entry", "second entry"] {
        assert!(h.query_all_by_value(want).next().is_some(), "the card does not show {want:?}");
    }
    // The button that posts the next entry.
    h.get_by_label("Send");
}

/// A cell with no comment yet gets a box for the first entry, not a reply box.
#[test]
fn a_cell_without_a_comment_gets_a_box_for_the_first_entry() {
    let mut h = harness();
    h.state_mut().grid.comment_card = Some(cell("B2"));
    h.run_steps(6);
    assert!(h.query_all_by_value("Start a conversation").next().is_some());
    h.get_by_label("Post");
}

/// The date each entry was written is shown beside the name.
#[test]
fn each_entry_shows_the_date_it_was_written() {
    let mut h = harness();
    h.state_mut().run("review.newComment", json!({"cell": "B2", "text": "first entry", "author": "Ada"})).unwrap();
    h.state_mut().grid.comment_card = Some(cell("B2"));
    h.run_steps(6);
    assert!(h.query_all_by_value("Ada").next().is_some());
    // The date is shown the way Excel shows it: 11/20/26 1:09 PM.
    let date = gridcraft_engine::core::date::iso8601_now().unwrap();
    let local = gridcraft_engine::core::date::iso8601_local(&date).unwrap();
    assert!(h.query_all_by_value(&local).next().is_some(), "the date {local:?} is not shown");
}
