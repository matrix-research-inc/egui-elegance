//! Behavioural tests for `Dock`: tab activation, closing, drag-and-drop and
//! splitter resizing, driven through real pointer events.

use eframe::egui::{self, Key, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use elegance::{Dock, DockEvent, DockLayout, DockNode, DockTab, DockViewer, Theme};

struct App {
    layout: DockLayout,
    events: Vec<DockEvent>,
    min_pane: f32,
    /// Skip showing the dock, as an app does while a tab page is hidden.
    hidden: bool,
}

struct Viewer;

impl DockViewer for Viewer {
    fn tab(&mut self, id: &str) -> DockTab {
        DockTab::new(id)
    }

    fn ui(&mut self, ui: &mut egui::Ui, id: &str) {
        ui.label(format!("body {id}"));
    }
}

fn harness(layout: DockLayout) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 500.0))
        .build_ui_state(
            |ui, app: &mut App| {
                Theme::slate().install(ui.ctx());
                if app.hidden {
                    return;
                }
                let out = Dock::new("dock").min_pane_size(app.min_pane).show(
                    ui,
                    &mut app.layout,
                    &mut Viewer,
                );
                app.events.extend(out.events);
            },
            App {
                layout,
                events: Vec::new(),
                min_pane: 96.0,
                hidden: false,
            },
        );
    harness.run();
    harness
}

/// `files | (editor, preview) over console`.
fn three_pane() -> DockLayout {
    DockLayout::from_node(DockNode::row(
        0.25,
        DockNode::tabs(["files"]),
        DockNode::column(
            0.6,
            DockNode::tabs(["editor", "preview"]),
            DockNode::tabs(["console"]),
        ),
    ))
}

fn centre(harness: &Harness<'_, App>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

// A drag is driven in phases so tests can act in the middle of it. Each phase
// runs a few frames so egui sees the pointer travel and the drag threshold is
// crossed. `run_steps` is used because a drag requests repaints forever.

fn press(harness: &mut Harness<'_, App>, at: Pos2) {
    harness.hover_at(at);
    harness.run_steps(2);
    harness.drag_at(at);
    harness.run_steps(2);
}

fn move_to(harness: &mut Harness<'_, App>, to: Pos2) {
    harness.hover_at(to);
    harness.run_steps(2);
}

fn release(harness: &mut Harness<'_, App>, at: Pos2) {
    harness.drop_at(at);
    harness.run_steps(3);
}

fn drag(harness: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    press(harness, from);
    move_to(harness, to);
    release(harness, to);
}

/// Width of the first pane of the root split: the gutter's left edge relative
/// to the dock's.
fn first_pane_width(harness: &Harness<'_, App>) -> f32 {
    let dock = harness.get_by_label("dock").rect();
    let splitter = harness
        .get_all_by_label("splitter")
        .next()
        .expect("splitter")
        .rect();
    splitter.left() - dock.left()
}

fn tab_rect(harness: &Harness<'_, App>, label: &str) -> egui::Rect {
    harness.get_by_label(label).rect()
}

/// Nothing moved: same tab order and group count, and no `TabMoved` event.
/// (Pressing a tab to start a drag still activates it, as in a browser.)
fn assert_nothing_moved(harness: &Harness<'_, App>, tabs: &[String], groups: usize) {
    assert_eq!(state(harness), (tabs.to_vec(), groups));
    assert!(
        !harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, DockEvent::TabMoved(_))),
        "a cancelled or void drop must not report a move"
    );
}

fn state(harness: &Harness<'_, App>) -> (Vec<String>, usize) {
    let layout = &harness.state().layout;
    (
        layout.tab_ids().into_iter().map(str::to_owned).collect(),
        layout.group_count(),
    )
}

#[test]
fn clicking_an_inactive_tab_activates_it() {
    let mut harness = harness(three_pane());
    assert!(harness.query_by_label("body editor").is_some());
    assert!(harness.query_by_label("body preview").is_none());

    harness.get_by_label("preview").click();
    harness.run();

    assert!(harness.query_by_label("body preview").is_some());
    assert!(harness.query_by_label("body editor").is_none());
    assert!(
        harness
            .state()
            .events
            .contains(&DockEvent::TabActivated("preview".into()))
    );
    assert!(harness.state().events.contains(&DockEvent::LayoutChanged));
}

#[test]
fn closing_a_tab_removes_it_and_collapses_an_empty_group() {
    let mut harness = harness(three_pane());
    assert_eq!(state(&harness).1, 3);

    // `console` is alone in its group; activate its close button.
    harness.get_by_label("Close console").click();
    harness.run();

    let (tabs, groups) = state(&harness);
    assert_eq!(tabs, ["files", "editor", "preview"]);
    assert_eq!(groups, 2);
    assert!(harness.query_by_label("body console").is_none());
    assert!(
        harness
            .state()
            .events
            .contains(&DockEvent::TabClosed("console".into()))
    );
}

#[test]
fn dragging_a_tab_into_another_strip_moves_it() {
    let mut harness = harness(three_pane());
    let from = centre(&harness, "preview");
    // The console strip, right of its tab.
    let console = tab_rect(&harness, "console");
    let to = Pos2::new(console.right() + 150.0, console.center().y);

    drag(&mut harness, from, to);

    let layout = &harness.state().layout;
    assert!(layout.contains("preview"));
    assert_eq!(layout.group_count(), 3);
    assert_eq!(
        harness.state().events.last(),
        Some(&DockEvent::LayoutChanged)
    );
    assert!(
        harness
            .state()
            .events
            .contains(&DockEvent::TabMoved("preview".into()))
    );
    // `preview` is now in the console group, after `console`, and active.
    let tabs = layout.tab_ids();
    let pos = |t| tabs.iter().position(|x| *x == t).unwrap();
    assert!(pos("console") < pos("preview"));
    assert!(layout.active_tab_ids().contains(&"preview"));
    assert!(harness.query_by_label("body preview").is_some());
}

#[test]
fn dragging_to_a_pane_edge_splits_it() {
    let mut harness = harness(three_pane());
    let from = centre(&harness, "preview");
    // Left edge of the files body.
    let body = harness.get_by_label("body files").rect();
    let to = Pos2::new(body.left() + 20.0, body.center().y + 100.0);

    drag(&mut harness, from, to);

    let (tabs, groups) = state(&harness);
    assert_eq!(groups, 4, "dropping on an edge adds a group");
    // The new pane sits left of `files`.
    assert_eq!(tabs, ["preview", "files", "editor", "console"]);
}

#[test]
fn dropping_outside_the_dock_leaves_the_layout_alone() {
    let mut harness = harness(three_pane());
    let (tabs, groups) = state(&harness);
    let from = centre(&harness, "preview");

    drag(&mut harness, from, Pos2::new(2000.0, 2000.0));

    assert_nothing_moved(&harness, &tabs, groups);
}

#[test]
fn escape_cancels_a_drag() {
    let mut harness = harness(three_pane());
    let (tabs, groups) = state(&harness);
    let from = centre(&harness, "preview");
    let body = harness.get_by_label("body files").rect();
    let to = Pos2::new(body.left() + 20.0, body.center().y + 100.0);

    press(&mut harness, from);
    move_to(&mut harness, to);
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    release(&mut harness, to);

    assert_nothing_moved(&harness, &tabs, groups);
}

#[test]
fn dragging_a_splitter_resizes_the_panes() {
    let mut harness = harness(three_pane());
    let before = harness.get_by_label("body editor").rect().left();
    let splitter = harness
        .get_all_by_label("splitter")
        .next()
        .expect("splitter");
    let from = splitter.rect().center();

    drag(&mut harness, from, from + egui::vec2(120.0, 0.0));

    let after = harness.get_by_label("body editor").rect().left();
    assert!(
        after > before + 80.0,
        "editor pane should move right: {before} -> {after}"
    );
    assert!(harness.state().events.contains(&DockEvent::LayoutChanged));
}

#[test]
fn splitter_stops_at_the_minimum_pane_size() {
    let mut harness = harness(three_pane());
    let from = harness
        .get_all_by_label("splitter")
        .next()
        .expect("splitter")
        .rect()
        .center();

    drag(&mut harness, from, from + egui::vec2(-2000.0, 0.0));

    let width = first_pane_width(&harness);
    assert!(
        (90.0..110.0).contains(&width),
        "default minimum is 96pt, got {width}"
    );
}

#[test]
fn min_pane_size_below_ten_percent_of_the_split_is_honoured() {
    let mut harness = harness(three_pane());
    harness.state_mut().min_pane = 20.0;
    harness.run();
    let from = harness
        .get_all_by_label("splitter")
        .next()
        .expect("splitter")
        .rect()
        .center();

    drag(&mut harness, from, from + egui::vec2(-2000.0, 0.0));

    let width = first_pane_width(&harness);
    assert!(
        (15.0..40.0).contains(&width),
        "a 20pt minimum should allow a narrow pane, got {width}"
    );
}

#[test]
fn splitter_follows_the_pointer_after_hitting_a_limit() {
    let mut harness = harness(three_pane());
    let before = first_pane_width(&harness);
    let from = harness
        .get_all_by_label("splitter")
        .next()
        .expect("splitter")
        .rect()
        .center();

    press(&mut harness, from);
    // Overshoot the minimum, then come back to just right of the start.
    move_to(&mut harness, from + egui::vec2(-1000.0, 0.0));
    move_to(&mut harness, from + egui::vec2(60.0, 0.0));
    release(&mut harness, from + egui::vec2(60.0, 0.0));

    let after = first_pane_width(&harness);
    assert!(
        (after - (before + 60.0)).abs() < 10.0,
        "handle should sit under the pointer: {before} -> {after}"
    );
}

#[test]
fn dropping_in_the_middle_of_its_own_pane_does_nothing() {
    let mut harness = harness(DockLayout::single(["a", "b", "c"]));
    let (tabs, groups) = state(&harness);
    let from = centre(&harness, "a");

    drag(&mut harness, from, Pos2::new(400.0, 300.0));

    assert_nothing_moved(&harness, &tabs, groups);
}

#[test]
fn a_drag_that_outlives_the_dock_is_dropped() {
    let mut harness = harness(three_pane());
    let (tabs, groups) = state(&harness);
    let from = centre(&harness, "preview");
    let body = harness.get_by_label("body files").rect();
    let to = Pos2::new(body.left() + 20.0, body.center().y + 100.0);

    press(&mut harness, from);
    move_to(&mut harness, to);
    // The app stops showing the dock, and the button is released meanwhile.
    harness.state_mut().hidden = true;
    harness.run_steps(3);
    release(&mut harness, to);
    // Showing it again over a valid target must not resurrect the drag.
    harness.state_mut().hidden = false;
    harness.hover_at(to);
    harness.run_steps(3);

    assert_nothing_moved(&harness, &tabs, groups);
}

#[test]
fn an_empty_layout_shows_the_empty_state() {
    let harness = harness(DockLayout::new());
    assert!(harness.query_by_label("No open tabs").is_some());
}
