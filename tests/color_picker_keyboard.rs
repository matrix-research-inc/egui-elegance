//! Keyboard operation of `ColorPicker`'s three continuous surfaces.
//!
//! The SV plane, hue strip, and alpha strip were pointer-only: focusable, so
//! `Tab` landed on them, but bound to no keys, so the arrows navigated straight
//! back off and a keyboard user could not pick a colour at all. These tests
//! drive each surface by key and assert the bound colour moves.

use eframe::egui;
use egui::accesskit::Role;
use egui::{Color32, Key, ecolor::HsvaGamma};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use elegance::{ColorPicker, Theme};

const ID: &str = "cp_keys";

struct Probe {
    color: Color32,
}

/// A picker with its popover forced open, so the surfaces are live every frame.
fn picker_harness(initial: Color32) -> Harness<'static, Probe> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 460.0))
        .build_ui_state(
            |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                egui::Popup::open_id(
                    &ui.ctx().clone(),
                    elegance::Popover::popup_id(("elegance::color_picker", egui::Id::unique(ID))),
                );
                ui.set_min_size(egui::vec2(360.0, 400.0));
                ui.add(ColorPicker::new(ID, &mut probe.color));
            },
            Probe { color: initial },
        );
    harness.run();
    harness.run();
    harness
}

fn focus(harness: &Harness<'_, Probe>, label: &str) {
    harness.get_by_role_and_label(Role::Slider, label).focus();
}

fn hsv(probe: &Probe) -> HsvaGamma {
    HsvaGamma::from(probe.color)
}

fn recents(harness: &Harness<'_, Probe>) -> Vec<Color32> {
    harness
        .ctx
        .data(|d| d.get_temp::<Vec<Color32>>(egui::Id::unique(ID).with("color_picker::recents")))
        .unwrap_or_default()
}

fn shift_press(harness: &mut Harness<'_, Probe>, key: Key) {
    harness.event(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::SHIFT,
    });
    harness.step();
}

/// A mid-range colour: every channel has room to move in both directions.
fn mid() -> Color32 {
    Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 0x80)
}

#[test]
fn sv_plane_arrows_move_saturation_and_value() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Saturation and value");
    harness.run();

    let before = hsv(harness.state());

    harness.key_press(Key::ArrowRight);
    harness.step();
    let after_right = hsv(harness.state());
    assert!(
        after_right.s > before.s,
        "ArrowRight should raise saturation, {} -> {}",
        before.s,
        after_right.s
    );

    harness.key_press(Key::ArrowUp);
    harness.step();
    let after_up = hsv(harness.state());
    assert!(
        after_up.v > after_right.v,
        "ArrowUp should raise value, {} -> {}",
        after_right.v,
        after_up.v
    );

    harness.key_press(Key::ArrowLeft);
    harness.step();
    assert!(
        hsv(harness.state()).s < after_up.s,
        "ArrowLeft should lower saturation"
    );
}

#[test]
fn hue_strip_arrows_move_hue() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Hue");
    harness.run();

    let before = hsv(harness.state()).h;

    harness.key_press(Key::ArrowRight);
    harness.step();
    assert!(
        hsv(harness.state()).h > before,
        "ArrowRight should advance hue, {} -> {}",
        before,
        hsv(harness.state()).h
    );

    harness.key_press(Key::Home);
    harness.step();
    assert!(
        hsv(harness.state()).h < 0.001,
        "Home should jump hue to the start of the strip, got {}",
        hsv(harness.state()).h
    );
}

#[test]
fn alpha_strip_arrows_move_alpha() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Alpha");
    harness.run();

    let before = harness.state().color.a();

    harness.key_press(Key::ArrowRight);
    harness.step();
    assert!(
        harness.state().color.a() > before,
        "ArrowRight should raise alpha, {} -> {}",
        before,
        harness.state().color.a()
    );

    harness.key_press(Key::End);
    harness.step();
    assert_eq!(
        harness.state().color.a(),
        255,
        "End should take alpha to fully opaque"
    );

    harness.key_press(Key::Home);
    harness.step();
    assert_eq!(
        harness.state().color.a(),
        0,
        "Home should take alpha to fully transparent"
    );
}

/// `Shift` is a 10x nudge, matching the crate's sliders — exactly 10x, not
/// merely larger. One shifted press has to land where ten plain ones do.
#[test]
fn shift_is_a_ten_times_nudge() {
    let mut harness = picker_harness(Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 0x00));

    focus(&harness, "Alpha");
    harness.run();
    for _ in 0..10 {
        harness.key_press(Key::ArrowRight);
        harness.step();
    }
    let ten_plain = harness.state().color.a();

    // Back to zero, then a single shifted press.
    harness.key_press(Key::Home);
    harness.step();
    shift_press(&mut harness, Key::ArrowRight);
    let one_shifted = harness.state().color.a();

    assert_eq!(
        ten_plain, one_shifted,
        "Shift+ArrowRight should land exactly where ten plain presses do"
    );
}

/// The same 10x, on the two surfaces whose step is not the strips' 1%: the hue
/// strip counts in degrees, and the plane's vertical axis is its own binding.
#[test]
fn shift_is_a_ten_times_nudge_on_hue_and_the_plane() {
    let mut harness = picker_harness(mid());

    focus(&harness, "Hue");
    harness.run();
    let start = hsv(harness.state()).h;
    for _ in 0..10 {
        harness.key_press(Key::ArrowRight);
        harness.step();
    }
    let ten_plain = hsv(harness.state()).h - start;

    harness.key_press(Key::Home);
    harness.step();
    shift_press(&mut harness, Key::ArrowRight);
    let one_shifted = hsv(harness.state()).h;

    assert!(
        (ten_plain - one_shifted).abs() < 0.002,
        "Shift on hue should be ten degrees: ten plain moved {ten_plain}, one shifted moved {one_shifted}"
    );

    let mut harness = picker_harness(mid());
    focus(&harness, "Saturation and value");
    harness.run();
    let start = hsv(harness.state()).v;
    for _ in 0..10 {
        harness.key_press(Key::ArrowDown);
        harness.step();
    }
    let ten_plain = start - hsv(harness.state()).v;

    let mut harness = picker_harness(mid());
    focus(&harness, "Saturation and value");
    harness.run();
    shift_press(&mut harness, Key::ArrowDown);
    let one_shifted = start - hsv(harness.state()).v;

    assert!(
        (ten_plain - one_shifted).abs() < 0.005,
        "Shift on the plane's vertical axis should be ten steps: {ten_plain} vs {one_shifted}"
    );
}

/// The hue circle closes: 1.0 is the same red as 0.0. `End` jumping there would
/// make it a synonym for `Home` and leave `ArrowRight` with nowhere to go, so it
/// stops one step short, at the last hue distinct from the first.
#[test]
fn hue_end_is_not_a_second_home() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Hue");
    harness.run();

    harness.key_press(Key::End);
    harness.step();
    let at_end = harness.state().color;

    harness.key_press(Key::Home);
    harness.step();
    let at_home = harness.state().color;

    assert_ne!(
        at_end, at_home,
        "End and Home should not name the same colour"
    );

    // And End parks at the top of the range, not the bottom: a nudge back down
    // moves, where the same nudge at Home is already clamped.
    harness.key_press(Key::ArrowLeft);
    harness.step();
    assert_eq!(
        harness.state().color,
        at_home,
        "ArrowLeft at Home should be clamped"
    );

    harness.key_press(Key::End);
    harness.step();
    harness.key_press(Key::ArrowLeft);
    harness.step();
    assert!(
        hsv(harness.state()).h < hsv(&Probe { color: at_end }).h,
        "ArrowLeft should walk back down the strip from End"
    );
}

/// The plane is the only surface in the crate that claims both arrow axes,
/// which is the shape that traps a keyboard user if the filter ever grows a
/// `tab` or `escape`. Both must still get out of it.
#[test]
fn tab_and_escape_still_leave_the_sv_plane() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Saturation and value");
    harness.run();

    let plane = harness.ctx.memory(|m| m.focused());
    assert!(plane.is_some(), "the plane should hold focus to start with");

    harness.key_press(Key::Tab);
    harness.step();
    harness.run();
    assert_ne!(
        harness.ctx.memory(|m| m.focused()),
        plane,
        "Tab must still move focus off the plane"
    );

    focus(&harness, "Saturation and value");
    harness.run();
    assert_eq!(harness.ctx.memory(|m| m.focused()), plane);

    harness.key_press(Key::Escape);
    harness.step();
    harness.run();
    assert_ne!(
        harness.ctx.memory(|m| m.focused()),
        plane,
        "Escape must still surrender the plane's focus"
    );
}

/// The plane binds all four arrows, so it has to claim both axes. Without that,
/// the first `ArrowDown` nudges value *and* hands focus to the strip below,
/// leaving the rest of the presses to drive something else — so the tell is not
/// where focus went, it is that only one of three presses landed.
#[test]
fn plane_takes_every_vertical_nudge_instead_of_leaking_focus() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Saturation and value");
    harness.run();

    let before = hsv(harness.state()).v;

    for _ in 0..3 {
        harness.key_press(Key::ArrowDown);
        harness.step();
    }

    // Three steps of 0.01 each. One step, the leaky outcome, cannot clear 0.02
    // even with the rounding of a Color32 round-trip.
    let moved = before - hsv(harness.state()).v;
    assert!(
        moved > 0.02,
        "expected three nudges of value, got {moved} — focus leaked after the first"
    );
}

/// A keyboard adjustment is a settled one, so it reports a commit the same way
/// a pointer release does — which is what pushes the colour to the recents row.
/// Without that, a keyboard user could pick a colour and never build a history.
#[test]
fn a_keyboard_adjustment_reaches_the_recents_row() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Hue");
    harness.run();

    assert!(
        recents(&harness).is_empty(),
        "nothing should be recorded before any adjustment"
    );

    harness.key_press(Key::ArrowRight);
    harness.step();
    harness.run();

    let after = recents(&harness);
    assert_eq!(
        after.first().copied(),
        Some(harness.state().color),
        "the nudged colour should head the recents row, got {after:?}"
    );
}

/// A key press settles the value the instant it lands, so a run of them would
/// push one recents entry each: a held arrow key at auto-repeat speed fills the
/// row with shades no one can tell apart and evicts everything the user had
/// collected. A run is one gesture and leaves one entry, just as a pointer drag
/// leaves one on release.
#[test]
fn a_run_of_nudges_leaves_one_recents_entry() {
    let mut harness = picker_harness(mid());
    focus(&harness, "Hue");
    harness.run();

    for _ in 0..12 {
        harness.key_press(Key::ArrowRight);
        harness.step();
    }
    harness.run();

    assert_eq!(
        recents(&harness).as_slice(),
        &[harness.state().color],
        "twelve nudges should leave the one colour they reached"
    );
}

/// The run ends when its surface loses focus, so moving on and adjusting again
/// builds history rather than overwriting the entry already there.
#[test]
fn moving_focus_starts_a_new_recents_entry() {
    let mut harness = picker_harness(mid());

    focus(&harness, "Hue");
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.step();
    harness.run();
    let from_hue = harness.state().color;

    focus(&harness, "Alpha");
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.step();
    harness.run();
    let from_alpha = harness.state().color;

    assert_eq!(
        recents(&harness).as_slice(),
        &[from_alpha, from_hue],
        "each surface's run should hold its own entry, newest first"
    );
}

/// Coming back to a surface starts a second run, not a continuation of the
/// first: leaving it is what ends the run, so the entry already recorded stays
/// put instead of being amended by an adjustment made minutes later.
#[test]
fn returning_to_a_surface_starts_a_new_recents_entry() {
    let mut harness = picker_harness(mid());

    focus(&harness, "Hue");
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.step();
    harness.run();
    let first = harness.state().color;

    // Away and back, without adjusting anything in between.
    focus(&harness, "Alpha");
    harness.run();
    focus(&harness, "Hue");
    harness.run();

    harness.key_press(Key::ArrowRight);
    harness.step();
    harness.run();
    let second = harness.state().color;

    assert_eq!(
        recents(&harness).as_slice(),
        &[second, first],
        "the second visit should record beside the first, not over it"
    );
}
