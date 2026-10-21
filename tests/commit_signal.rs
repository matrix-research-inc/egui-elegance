//! Interaction tests for [`elegance::ResponseCommitExt::committed`].
//!
//! Unlike the pixel tests in `visual.rs`, these assert *when* a signal fires,
//! so they drive the harness one frame per input event and count the frames on
//! which `changed()` and `committed()` were true.
//!
//! The invariant under test: `changed()` fires on every intermediate value,
//! `committed()` fires exactly once per settled adjustment.

use eframe::egui;
use egui::{Key, Modifiers, PointerButton, Pos2, Rect, TouchDeviceId, TouchId, TouchPhase};
use egui_kittest::Harness;
use elegance::{Knob, MetricSlider, RangeSlider, ResponseCommitExt, Slider, Theme};

/// Counts the frames each signal fired on, so a test can assert on timing
/// rather than just on the final value.
struct Probe {
    value: f32,
    changed: usize,
    committed: usize,
    /// The widget rect, captured each frame so tests can aim at the rail.
    rect: Rect,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            value: 0.0,
            changed: 0,
            committed: 0,
            rect: Rect::ZERO,
        }
    }
}

impl Probe {
    fn reset_counts(&mut self) {
        self.changed = 0;
        self.committed = 0;
    }
}

const STOPS: [f32; 5] = [0.0, 8.0, 16.0, 24.0, 32.0];

/// A harness holding a single `stops` slider — the configuration where the
/// intermediate-value burst is most pronounced.
fn slider_harness() -> Harness<'static, Probe> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(
                        MetricSlider::new(&mut probe.value, 0.0..=32.0)
                            .stops(STOPS)
                            .desired_width(400.0),
                    )
                    .accessible_name("Value");
                probe.rect = resp.rect;
                if resp.changed() {
                    probe.changed += 1;
                }
                if resp.committed() {
                    probe.committed += 1;
                }
            },
            Probe::default(),
        );
    // Settle layout so `rect` is populated, then discard the setup frames.
    harness.run();
    harness.state_mut().reset_counts();
    harness
}

/// The x coordinate a fraction of the way along the *track*.
///
/// The track is inset from the widget rect by half a thumb on each side. That
/// inset is private to `MetricSlider`, so rather than hardcode it these tests
/// aim at the interior and assert on the resulting value: `track_x(rect, 0.5)`
/// is the midpoint under any inset, and the endpoints clamp. A future change to
/// the thumb diameter therefore cannot silently make these tests aim at the
/// wrong stop while still passing.
fn track_x(rect: Rect, frac: f32) -> f32 {
    rect.min.x + rect.width() * frac
}

fn release_at(harness: &Harness<'_, Probe>, pos: Pos2) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
}

#[test]
fn drag_across_stops_commits_once_on_release() {
    let mut harness = slider_harness();
    let rect = harness.state().rect;
    let y = rect.center().y;
    let start = Pos2::new(track_x(rect, 0.0), y);
    let end = Pos2::new(track_x(rect, 1.0), y);

    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();

    // Sweep the full rail in quarters, crossing every stop. Each move is well
    // past `max_click_dist`, so egui decides this is a drag and not a click.
    for i in 1..=4 {
        harness.hover_at(Pos2::new(track_x(rect, i as f32 / 4.0), y));
        harness.step();
    }

    assert!(
        harness.state().changed >= 4,
        "expected `changed` on each crossed stop, got {}",
        harness.state().changed
    );
    assert_eq!(
        harness.state().committed,
        0,
        "must not commit mid-drag — that is the whole point"
    );

    release_at(&harness, end);
    harness.step();

    assert_eq!(
        harness.state().committed,
        1,
        "release after a drag must commit exactly once"
    );
    assert_eq!(
        harness.state().value,
        32.0,
        "the committed value must be the settled one"
    );
}

/// The regression this predicate exists for. A widget sensing both click and
/// drag is not `dragged()` yet on the press frame, so a `changed() &&
/// !dragged()` predicate would commit on press *and* again via `clicked()` on
/// release.
#[test]
fn click_to_set_commits_once_on_release_not_on_press() {
    let mut harness = slider_harness();
    let rect = harness.state().rect;
    let y = rect.center().y;
    let target = Pos2::new(track_x(rect, 0.5), y);

    harness.hover_at(target);
    harness.step();

    harness.drag_at(target);
    harness.step();

    assert_eq!(harness.state().changed, 1, "the press frame sets the value");
    assert_eq!(
        harness.state().committed,
        0,
        "the press frame must not commit — the interaction has not settled"
    );

    release_at(&harness, target);
    harness.step();

    assert_eq!(
        harness.state().committed,
        1,
        "a click must commit exactly once, on release"
    );
    assert_eq!(harness.state().value, 16.0);
}

#[test]
fn keyboard_nudge_commits_immediately() {
    let mut harness = slider_harness();
    // Tab to the slider — the path a keyboard user takes. `key_press` queues a
    // down and an up, so this advances two frames, which also covers egui's
    // one-frame delay before a newly requested focus takes effect.
    harness.key_press(Key::Tab);
    harness.step();
    harness.state_mut().reset_counts();
    assert_eq!(harness.state().value, 0.0);

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert_eq!(
        harness.state().value,
        8.0,
        "ArrowRight should advance one stop"
    );
    assert_eq!(
        harness.state().changed,
        1,
        "a keyboard nudge changes the value once"
    );
    assert_eq!(
        harness.state().committed,
        1,
        "a keyboard nudge is already atomic, so it commits on the same frame"
    );
}

#[test]
fn idle_frames_signal_nothing() {
    let mut harness = slider_harness();

    for _ in 0..3 {
        harness.step();
    }

    assert_eq!(harness.state().changed, 0);
    assert_eq!(harness.state().committed, 0);
}

/// A harness holding a single `Knob`, reusing the same probe.
fn knob_harness() -> Harness<'static, Probe> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 300.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Knob::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
                if resp.changed() {
                    probe.changed += 1;
                }
                if resp.committed() {
                    probe.committed += 1;
                }
            },
            Probe::default(),
        );
    harness.run();
    harness.state_mut().reset_counts();
    harness
}

/// The trait is not `MetricSlider`-specific. `Knob` has the same
/// live-`changed()`-during-drag pattern and picks the signal up for free.
#[test]
fn knob_drag_commits_once_on_release() {
    let mut harness = knob_harness();
    let center = harness.state().rect.center();
    harness.hover_at(center);
    harness.step();
    harness.drag_at(center);
    harness.step();

    // A knob tracks vertical drag distance.
    for i in 1..=4 {
        harness.hover_at(Pos2::new(center.x, center.y - (i as f32) * 12.0));
        harness.step();
    }

    assert!(
        harness.state().changed >= 2,
        "expected several intermediate changes, got {}",
        harness.state().changed
    );
    assert_eq!(harness.state().committed, 0, "must not commit mid-drag");

    harness.event(egui::Event::PointerButton {
        pos: Pos2::new(center.x, center.y - 48.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();

    assert_eq!(harness.state().committed, 1);
    assert!(
        harness.state().value > 0.0,
        "the drag should have raised it"
    );
}

/// A wheel notch does not arrive as one event — egui smooths the delta across
/// many frames — so without the widget-side settle a single notch would commit
/// once per frame it lasted, which is the burst this signal exists to prevent.
#[test]
fn knob_scroll_notch_commits_once_when_it_stops() {
    let mut harness = knob_harness();
    let center = harness.state().rect.center();

    harness.hover_at(center);
    harness.step();
    harness.state_mut().reset_counts();

    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });
    for _ in 0..30 {
        harness.step();
    }

    assert!(
        harness.state().changed >= 1,
        "the notch should move the value"
    );
    assert_eq!(
        harness.state().committed,
        1,
        "one notch must commit exactly once, on the frame the scroll settles"
    );
    assert!(harness.state().value > 0.0);
}

/// The sliders gate their write on `is_pointer_button_down_on`, which is
/// button-agnostic, so a click with *any* button moves the value. `clicked()`
/// is primary-only, so every other button needs covering explicitly or the
/// adjustment is written and never committed. `Extra1`/`Extra2` are the side
/// buttons; they were the gap an earlier version of this list left open.
#[test]
fn every_pointer_button_commits() {
    for button in [
        PointerButton::Primary,
        PointerButton::Secondary,
        PointerButton::Middle,
        PointerButton::Extra1,
        PointerButton::Extra2,
    ] {
        let mut harness = slider_harness();
        let rect = harness.state().rect;
        let target = Pos2::new(track_x(rect, 0.5), rect.center().y);

        harness.hover_at(target);
        harness.step();
        harness.event(egui::Event::PointerButton {
            pos: target,
            button,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        harness.event(egui::Event::PointerButton {
            pos: target,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();

        assert_eq!(
            harness.state().value,
            16.0,
            "{button:?} moves the value (the write gate is button-agnostic)"
        );
        assert_eq!(
            harness.state().committed,
            1,
            "{button:?} must commit exactly once"
        );
    }
}

/// A touch held past `max_click_duration` is the one gesture that ends without
/// a click or a drag: egui clears both the potential-click and potential-drag
/// ids at that moment, so the eventual lift reports neither. Without the
/// `long_touched` term a touch user who pauses to aim would move the value and
/// never commit.
///
/// The hold is simulated, not slept: kittest advances `input.time` by `step_dt`
/// each frame, and `is_long_press` is a pure time comparison against
/// `press_start_time`.
#[test]
fn touch_long_press_commits_once_across_the_whole_gesture() {
    let mut harness = slider_harness();
    let rect = harness.state().rect;
    let target = Pos2::new(track_x(rect, 0.5), rect.center().y);

    // A real touch produces both events; egui needs the `Touch` for
    // `any_touches()` and the `PointerButton` for the press itself.
    let touch = |phase| egui::Event::Touch {
        device_id: TouchDeviceId(0),
        id: TouchId(0),
        phase,
        pos: target,
        force: None,
    };
    harness.event(touch(TouchPhase::Start));
    harness.event(egui::Event::PointerButton {
        pos: target,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();

    assert_eq!(harness.state().value, 16.0, "the press sets the value");
    assert_eq!(
        harness.state().committed,
        0,
        "the press frame is not yet a long touch"
    );

    // Hold still, well past `max_click_duration`.
    for _ in 0..6 {
        harness.step();
    }
    assert_eq!(
        harness.state().committed,
        1,
        "holding past the click duration must commit once"
    );

    harness.event(touch(TouchPhase::End));
    release_at(&harness, target);
    harness.step();
    harness.step();

    assert_eq!(
        harness.state().committed,
        1,
        "the lift must not commit a second time for the same gesture"
    );
}

/// A scroll settle is reported by `Knob` itself rather than inferred from the
/// response, so unrelated input on the same frame cannot be mistaken for it.
/// While the predicate still read `changed()`, an unrelated key press during
/// the smoothed scroll added a second commit for one notch.
#[test]
fn unrelated_key_during_scroll_commits_once() {
    let mut harness = knob_harness();
    let center = harness.state().rect.center();
    harness.hover_at(center);
    harness.step();
    harness.state_mut().reset_counts();

    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    });
    harness.step();
    // A key the knob does not own, pressed while the smoothed delta is still
    // moving the value. Nothing has focus.
    harness.event(egui::Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    for _ in 0..30 {
        harness.step();
    }

    assert_eq!(
        harness.state().committed,
        1,
        "one notch must commit once regardless of unrelated input"
    );
}

/// `Slider` picks the signal up from the same widget-side report.
#[test]
fn slider_keyboard_commits_once() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Slider::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
                if resp.changed() {
                    probe.changed += 1;
                }
                if resp.committed() {
                    probe.committed += 1;
                }
            },
            Probe::default(),
        );
    harness.run();
    harness.state_mut().reset_counts();

    harness.key_press(Key::Tab);
    harness.step();
    harness.state_mut().reset_counts();

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert!(harness.state().value > 0.0, "ArrowRight should raise it");
    assert_eq!(harness.state().committed, 1);
}

/// `RangeSlider` returns `bg | thumb[0] | thumb[1]`, and `Response::union` keeps
/// only the left id, so focus lives on a thumb id the combined response does not
/// carry. Any id-based focus check in the predicate would silently stop
/// committing here, which is exactly why this test exists.
#[test]
fn range_slider_keyboard_commits_once() {
    struct RangeProbe {
        low: f32,
        high: f32,
        committed: usize,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut RangeProbe| {
                Theme::slate().install(ui.ctx());
                let resp = ui.add(RangeSlider::new(
                    &mut probe.low,
                    &mut probe.high,
                    0.0..=100.0,
                ));
                if resp.committed() {
                    probe.committed += 1;
                }
            },
            RangeProbe {
                low: 20.0,
                high: 80.0,
                committed: 0,
            },
        );
    harness.run();
    harness.state_mut().committed = 0;

    // Two tabs: the widget, then its first thumb.
    harness.key_press(Key::Tab);
    harness.step();
    harness.key_press(Key::Tab);
    harness.step();
    harness.state_mut().committed = 0;

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert!(
        harness.state().low > 20.0,
        "ArrowRight should raise the low endpoint, got {}",
        harness.state().low
    );
    assert_eq!(
        harness.state().committed,
        1,
        "a keyboard nudge on a thumb must commit exactly once"
    );
}
