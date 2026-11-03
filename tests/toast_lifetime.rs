//! A toast's auto-dismiss countdown starts on the first frame that renders it.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use elegance::{Theme, Toast, Toasts};

/// Whether the toast is on screen, by the live-region node it announces with.
fn showing(harness: &Harness<'_>) -> bool {
    harness
        .query_by_role_and_label(Role::Status, "Export finished")
        .is_some()
}

/// Run one frame at `time` seconds, as a backend would after any gap.
fn frame_at(harness: &mut Harness<'_>, time: f64) {
    harness.input_mut().time = Some(time);
    harness.step();
}

#[test]
fn toast_raised_while_no_frames_run_gets_its_full_duration() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui(|ui| {
            Theme::slate().install(ui.ctx());
            Toasts::new().render(ui.ctx());
        });
    frame_at(&mut harness, 1.0);

    // Raised from a callback while the window is hidden: no frame runs, so
    // egui's input time is still the last frame's.
    Toast::new("Export finished").show(&harness.ctx);

    // The window reappears a minute later. A new `Area` spends its first
    // frame measuring itself invisibly, so give it a second to show.
    frame_at(&mut harness, 61.0);
    frame_at(&mut harness, 61.05);
    assert!(
        showing(&harness),
        "the toast must not expire against time no frame rendered"
    );

    // It still dismisses on schedule, counted from that frame: 4 s default
    // duration plus the 0.2 s fade.
    frame_at(&mut harness, 64.9);
    assert!(showing(&harness));
    frame_at(&mut harness, 65.3);
    assert!(!showing(&harness));
}
