//! Interaction tests for the shared "grab gives focus" policy.
//!
//! egui does not focus a widget on press, so before this policy existed you
//! could click a slider's rail and then press an arrow key and nothing would
//! happen: the key went to focus navigation instead of the widget you were
//! plainly interacting with. Each test here performs a pointer press, releases,
//! and then presses an arrow key, asserting the value moves.
//!
//! Every test therefore fails if its widget stops taking focus on press, which
//! is the drift these guard against — `RangeSlider` had the policy and the other
//! three did not.

use eframe::egui;
use egui::{Id, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use elegance::{Knob, MetricSlider, Modal, RangeSlider, Slider, Theme};

/// The bound value plus the widget rect, captured each frame so tests can aim.
struct Probe {
    value: f32,
    rect: Rect,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            value: 0.0,
            rect: Rect::ZERO,
        }
    }
}

fn release_at(harness: &Harness<'_, Probe>, pos: Pos2) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
}

/// Press and release the primary button at `pos`, one frame per event.
fn click_at(harness: &mut Harness<'_, Probe>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    release_at(harness, pos);
    harness.step();
}

#[test]
fn metric_slider_click_then_arrow_key_nudges() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(MetricSlider::new(&mut probe.value, 0.0..=100.0).desired_width(400.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
            },
            Probe::default(),
        );
    harness.run();

    let rect = harness.state().rect;
    click_at(&mut harness, rect.center());
    let after_click = harness.state().value;
    assert!(after_click > 0.0, "the click should have set a value");

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert!(
        harness.state().value > after_click,
        "ArrowRight after a click should nudge, got {} then {}",
        after_click,
        harness.state().value
    );
}

#[test]
fn slider_click_then_arrow_key_nudges() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Slider::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
            },
            Probe::default(),
        );
    harness.run();

    let rect = harness.state().rect;
    click_at(&mut harness, rect.center());
    let after_click = harness.state().value;
    assert!(after_click > 0.0, "the click should have set a value");

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert!(
        harness.state().value > after_click,
        "ArrowRight after a click should nudge, got {} then {}",
        after_click,
        harness.state().value
    );
}

/// A `Knob` reads pointer *motion*, so a press that does not move leaves the
/// value alone. That makes the assertion sharper than for a slider: the value
/// can only have moved because the arrow key reached the widget.
#[test]
fn knob_click_then_arrow_key_nudges() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 300.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Knob::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
            },
            Probe {
                value: 50.0,
                rect: Rect::ZERO,
            },
        );
    harness.run();

    let rect = harness.state().rect;
    click_at(&mut harness, rect.center());
    assert_eq!(
        harness.state().value,
        50.0,
        "a motionless press must not move a knob"
    );

    harness.key_press(Key::ArrowUp);
    harness.step();

    assert!(
        harness.state().value > 50.0,
        "ArrowUp after a click should nudge, got {}",
        harness.state().value
    );
}

/// `RangeSlider` focused the thumb it *picked* for a background press, but a
/// press landing directly on a thumb never reached that branch — the thumb sits
/// above the background and takes the pointer — so the thumb you grabbed most
/// deliberately was the one that did not get focus.
#[test]
fn range_slider_thumb_press_then_arrow_key_nudges() {
    struct RangeProbe {
        low: f32,
        high: f32,
        rect: Rect,
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
                probe.rect = resp.rect;
            },
            // The returned response is `bg | thumb[0] | thumb[1]`, and `union`
            // unions the rects, so the centre of that rect is only the centre of
            // the low thumb while both thumbs sit inside the track. Parking the
            // high thumb at 100% would push its hit rect past the track's end
            // and drag the union centre off the low thumb; 75% keeps it clear of
            // both the edge and the press.
            RangeProbe {
                low: 50.0,
                high: 75.0,
                rect: Rect::ZERO,
            },
        );
    harness.run();

    let target = harness.state().rect.center();
    harness.hover_at(target);
    harness.step();
    harness.drag_at(target);
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos: target,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();

    // Landing on the thumb writes back the value under the pointer, which is
    // the thumb's own position. A press that missed it would jump the value.
    let after_press = harness.state().low;
    assert!(
        (after_press - 50.0).abs() < 0.05,
        "expected the press to land on the low thumb, got {after_press}"
    );

    harness.key_press(Key::ArrowRight);
    harness.step();

    assert!(
        harness.state().low > after_press,
        "ArrowRight after grabbing the low thumb should nudge it, got {} then {}",
        after_press,
        harness.state().low
    );
    assert_eq!(
        harness.state().high,
        75.0,
        "the unfocused thumb must not move"
    );
}

/// A `Knob` binds all four arrows, so it must claim both axes with a focus-lock
/// filter or egui routes the key to spatial focus navigation *as well*: the knob
/// nudges once and then hands focus to whatever sits next to it, and every
/// further press drives the neighbour. A single-widget harness cannot see that,
/// hence the pair.
#[test]
fn knob_arrow_keys_do_not_leak_focus_to_a_neighbour() {
    struct PairProbe {
        left: f32,
        right: f32,
        left_rect: Rect,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut PairProbe| {
                Theme::slate().install(ui.ctx());
                ui.horizontal(|ui| {
                    let left = ui
                        .add(Knob::new(&mut probe.left, 0.0..=100.0))
                        .accessible_name("Left");
                    probe.left_rect = left.rect;
                    ui.add(Knob::new(&mut probe.right, 0.0..=100.0))
                        .accessible_name("Right");
                });
            },
            PairProbe {
                left: 50.0,
                right: 50.0,
                left_rect: Rect::ZERO,
            },
        );
    harness.run();

    let target = harness.state().left_rect.center();
    harness.hover_at(target);
    harness.step();
    harness.drag_at(target);
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos: target,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();

    for _ in 0..3 {
        harness.key_press(Key::ArrowRight);
        harness.step();
    }

    assert!(
        harness.state().left > 50.0,
        "the grabbed knob should have taken every nudge"
    );
    assert_eq!(
        harness.state().right,
        50.0,
        "focus must not leak to the neighbouring knob, which took {} of the nudges",
        harness.state().right
    );
}

/// Focusing on press puts every value widget within reach of the click egui
/// synthesises from Space on a focused widget. A `Knob` binds a reset-to-default
/// gesture, so if that gesture answered to Space, pressing it after setting a
/// knob by hand would wipe the value just set. It answers to `0` instead.
#[test]
fn space_after_a_grab_does_not_reset_a_knob() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 300.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Knob::new(&mut probe.value, 0.0..=100.0).default(0.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
            },
            Probe {
                value: 50.0,
                rect: Rect::ZERO,
            },
        );
    harness.run();

    let rect = harness.state().rect;
    click_at(&mut harness, rect.center());

    harness.key_press(Key::Space);
    harness.step();
    assert_eq!(
        harness.state().value,
        50.0,
        "Space must not reset a knob the pointer just focused"
    );

    harness.key_press(Key::Num0);
    harness.step();
    assert_eq!(
        harness.state().value,
        0.0,
        "`0` is the keyboard reset and must still work"
    );
}

/// `is_pointer_button_down_on` stays true until release, so a widget grabbed
/// while it was interactive keeps reporting the grab after a `Modal` opens over
/// it. egui surrenders its focus every pass; without a matching guard the widget
/// would take focus straight back and hold it against the modal's focus trap.
#[test]
fn a_grab_behind_a_modal_does_not_hold_focus() {
    struct ModalProbe {
        value: f32,
        rect: Rect,
        id: Option<Id>,
        open: bool,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui_state(
            |ui, probe: &mut ModalProbe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add(Slider::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
                probe.id = Some(resp.id);
                if probe.open {
                    Modal::new("m", &mut probe.open)
                        .heading("Confirm")
                        .show(ui.ctx(), |ui| {
                            ui.label("Are you sure?");
                        });
                }
            },
            ModalProbe {
                value: 0.0,
                rect: Rect::ZERO,
                id: None,
                open: false,
            },
        );
    harness.run();

    // Grab the slider and keep holding it.
    let target = harness.state().rect.center();
    harness.hover_at(target);
    harness.step();
    harness.drag_at(target);
    harness.step();

    let slider_id = harness.state().id.expect("slider rendered");
    assert_eq!(
        harness.ctx.memory(|m| m.focused()),
        Some(slider_id),
        "the grab should have focused the slider"
    );

    // Something opens a modal while the button is still down.
    harness.state_mut().open = true;
    for _ in 0..3 {
        harness.step();
    }

    assert_ne!(
        harness.ctx.memory(|m| m.focused()),
        Some(slider_id),
        "a slider behind a modal must not hold keyboard focus"
    );
}

/// Same shape as the modal case: a widget disabled mid-drag still reports the
/// grab, and must not claw its focus back from egui.
#[test]
fn a_widget_disabled_mid_drag_does_not_hold_focus() {
    struct EnabledProbe {
        value: f32,
        rect: Rect,
        id: Option<Id>,
        enabled: bool,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 200.0))
        .build_ui_state(
            |ui, probe: &mut EnabledProbe| {
                Theme::slate().install(ui.ctx());
                let resp = ui
                    .add_enabled(probe.enabled, Slider::new(&mut probe.value, 0.0..=100.0))
                    .accessible_name("Value");
                probe.rect = resp.rect;
                probe.id = Some(resp.id);
            },
            EnabledProbe {
                value: 0.0,
                rect: Rect::ZERO,
                id: None,
                enabled: true,
            },
        );
    harness.run();

    let target = harness.state().rect.center();
    harness.hover_at(target);
    harness.step();
    harness.drag_at(target);
    harness.step();

    let slider_id = harness.state().id.expect("slider rendered");
    assert_eq!(
        harness.ctx.memory(|m| m.focused()),
        Some(slider_id),
        "the grab should have focused the slider"
    );

    harness.state_mut().enabled = false;
    for _ in 0..3 {
        harness.step();
    }

    assert_ne!(
        harness.ctx.memory(|m| m.focused()),
        Some(slider_id),
        "a disabled slider must not hold keyboard focus"
    );
}
