//! Custom content in a `MenuBar`: inline widgets through `MenuBarUi::ui`,
//! right-pinned widgets through `MenuBarUi::trailing`, and window moves
//! through `MenuBar::title_bar`.

use eframe::egui;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, ViewportCommand, ViewportId, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use elegance::{Button, ButtonSize, MenuBar, MenuItem, Theme, glyphs};

/// Horizontal padding the strip applies inside its frame.
const STRIP_PAD_X: f32 = 6.0;

#[derive(Default)]
struct Probe {
    /// The width available to the bar, i.e. the strip's outer rect.
    bar_rect: Option<Rect>,
    inline: Option<Rect>,
    close: Option<Rect>,
    close_clicks: usize,
}

#[derive(Clone, Copy, Default)]
struct Options {
    status: bool,
    title_bar: bool,
}

fn harness(options: Options) -> Harness<'static, Probe> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(640.0, 200.0))
        // Short steps, so the two clicks of a double-click, one frame per
        // event, land inside egui's double-click window.
        .with_step_dt(0.01)
        .build_ui_state(
            move |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                probe.bar_rect = Some(ui.max_rect());
                let mut bar = MenuBar::new("custom_content")
                    .brand("App")
                    .title_bar(options.title_bar);
                if options.status {
                    bar = bar.status("main \u{00b7} up to date");
                }
                bar.show(ui, |bar| {
                    bar.menu("File", |ui| {
                        ui.add(MenuItem::new("Quit"));
                    });
                    bar.ui().add_space(8.0);
                    probe.inline = Some(bar.ui().label("Inline").rect);
                    bar.menu("Help", |_| {});
                    bar.trailing(|ui| {
                        let close =
                            ui.add(Button::icon(glyphs::X, "Close window").size(ButtonSize::Small));
                        probe.close = Some(close.rect);
                        if close.clicked() {
                            probe.close_clicks += 1;
                        }
                    });
                });
            },
            Probe::default(),
        );
    h.run();
    h
}

fn assert_pinned_right(probe: &Probe) {
    let bar = probe.bar_rect.expect("bar rendered");
    let inline = probe.inline.expect("inline widget rendered");
    let close = probe.close.expect("trailing widget rendered");
    assert!(
        (close.right() - (bar.right() - STRIP_PAD_X)).abs() < 1.0,
        "trailing content should sit at the strip's right edge: close={close:?}, bar={bar:?}"
    );
    assert!(
        inline.right() < close.left(),
        "inline content should precede the trailing slot: inline={inline:?}, close={close:?}"
    );
}

#[test]
fn trailing_content_is_pinned_right() {
    // The status has no accessibility node to locate, so with a status
    // this checks only that the trailing content still holds the edge.
    for status in [false, true] {
        let h = harness(Options {
            status,
            ..Options::default()
        });
        assert_pinned_right(h.state());
    }
}

#[test]
fn custom_content_is_reachable_and_interactive() {
    let mut h = harness(Options::default());
    let _ = h.get_by_label("Inline");
    // Triggers declared on either side of the inline widget keep working.
    let _ = h.get_by_role_and_label(egui::accesskit::Role::Button, "File");
    let _ = h.get_by_role_and_label(egui::accesskit::Role::Button, "Help");

    h.get_by_label("Close window").click();
    h.run();
    assert_eq!(h.state().close_clicks, 1);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "declare every menu first")]
fn menu_after_trailing_panics_in_debug() {
    let mut h = Harness::new_ui(|ui| {
        MenuBar::new("misordered").show(ui, |bar| {
            bar.trailing(|_| {});
            bar.menu("File", |_| {});
        });
    });
    h.run();
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "called more than once")]
fn repeated_trailing_panics_in_debug() {
    let mut h = Harness::new_ui(|ui| {
        MenuBar::new("twice").show(ui, |bar| {
            bar.trailing(|_| {});
            bar.trailing(|_| {});
        });
    });
    h.run();
}

/// Feed `events` one frame each, returning every viewport command sent.
fn gesture(h: &mut Harness<'static, Probe>, events: Vec<Event>) -> Vec<ViewportCommand> {
    let mut commands = Vec::new();
    for event in events {
        h.event(event);
        h.step();
        commands.extend(
            h.output().viewport_output[&ViewportId::ROOT]
                .commands
                .clone(),
        );
    }
    commands
}

fn primary(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn drag_from(pos: Pos2) -> Vec<Event> {
    let to = pos + vec2(60.0, 0.0);
    vec![
        Event::PointerMoved(pos),
        primary(pos, true),
        Event::PointerMoved(pos + vec2(30.0, 0.0)),
        Event::PointerMoved(to),
        primary(to, false),
    ]
}

fn double_click_at(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        primary(pos, true),
        primary(pos, false),
        primary(pos, true),
        primary(pos, false),
    ]
}

/// A point on the strip that no widget covers: between the last trigger
/// and the trailing slot.
fn empty_strip_point(h: &Harness<'static, Probe>) -> Pos2 {
    let help = h.get_by_label("Help").rect();
    let close = h.state().close.expect("trailing widget rendered");
    Pos2::new((help.right() + close.left()) * 0.5, close.center().y)
}

fn title_bar_harness() -> Harness<'static, Probe> {
    harness(Options {
        title_bar: true,
        ..Options::default()
    })
}

#[test]
fn title_bar_drag_on_empty_area_moves_window() {
    let mut h = title_bar_harness();
    let at = empty_strip_point(&h);
    let commands = gesture(&mut h, drag_from(at));
    assert_eq!(commands, [ViewportCommand::StartDrag]);
}

#[test]
fn title_bar_drag_on_brand_moves_window() {
    // The brand takes no input, so it counts as empty area.
    // It is painted without an accessibility node, so aim between the
    // strip's left edge and the first trigger.
    let mut h = title_bar_harness();
    let file = h.get_by_label("File").rect();
    let bar = h.state().bar_rect.expect("bar rendered");
    let at = Pos2::new((bar.left() + file.left()) * 0.5, file.center().y);
    let commands = gesture(&mut h, drag_from(at));
    assert_eq!(commands, [ViewportCommand::StartDrag]);
}

#[test]
fn title_bar_drag_on_widgets_leaves_window() {
    let mut h = title_bar_harness();
    let trigger = h.get_by_label("File").rect().center();
    let close = h.state().close.expect("trailing widget rendered").center();
    for at in [trigger, close] {
        let commands = gesture(&mut h, drag_from(at));
        assert!(
            commands.is_empty(),
            "a drag starting at {at:?} on a widget should not move the window: {commands:?}"
        );
    }
}

#[test]
fn title_bar_drag_begun_in_press_frame_moves_window() {
    // A fast flick can press and move within one frame.
    let mut h = title_bar_harness();
    let at = empty_strip_point(&h);
    h.input_mut().events.extend([
        Event::PointerMoved(at),
        primary(at, true),
        Event::PointerMoved(at + vec2(40.0, 0.0)),
    ]);
    let mut commands = Vec::new();
    for _ in 0..2 {
        h.step();
        commands.extend(
            h.output().viewport_output[&ViewportId::ROOT]
                .commands
                .clone(),
        );
    }
    assert_eq!(commands, [ViewportCommand::StartDrag]);
}

#[test]
fn title_bar_strip_is_not_a_focus_stop() {
    // The first Tab should land on the first menu trigger, not on the
    // strip behind it.
    let mut h = title_bar_harness();
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("File").accesskit_node().is_focused());
}

#[test]
fn title_bar_double_click_toggles_maximized() {
    let mut h = title_bar_harness();
    let at = empty_strip_point(&h);
    let commands = gesture(&mut h, double_click_at(at));
    assert_eq!(commands, [ViewportCommand::Maximized(true)]);
}

#[test]
fn title_bar_double_click_on_widget_leaves_window() {
    let mut h = title_bar_harness();
    let close = h.state().close.expect("trailing widget rendered").center();
    let commands = gesture(&mut h, double_click_at(close));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(h.state().close_clicks, 2);
}

#[test]
fn strip_ignores_window_gestures_by_default() {
    let mut h = harness(Options::default());
    let at = empty_strip_point(&h);
    let mut commands = gesture(&mut h, drag_from(at));
    commands.extend(gesture(&mut h, double_click_at(at)));
    assert!(commands.is_empty(), "{commands:?}");
}
