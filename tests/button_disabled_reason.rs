//! `Button::disabled_reason`: one value both disables the button and explains why.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use elegance::{Button, Theme};

const REASON: &str = "Pick a target first";

/// A Deploy button disabled by `reason`, counting its clicks into the state.
fn harness<'a>(reason: Option<&'static str>, wrap_disabled: bool) -> Harness<'a, u32> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 200.0))
        .build_ui_state(
            move |ui, clicks: &mut u32| {
                Theme::slate().install(ui.ctx());
                ui.add_enabled_ui(!wrap_disabled, |ui| {
                    if ui
                        .add(Button::new("Deploy").disabled_reason(reason))
                        .clicked()
                    {
                        *clicks += 1;
                    }
                });
            },
            0,
        );
    harness.run();
    harness
}

#[test]
fn reason_disables_and_describes_the_button() {
    let harness = harness(Some(REASON), false);
    let node = harness.get_by_role_and_label(Role::Button, "Deploy");
    assert!(node.accesskit_node().is_disabled());
    assert_eq!(node.accesskit_node().description().as_deref(), Some(REASON));
}

#[test]
fn reason_shows_as_tooltip_on_hover() {
    // The reason must show whether the button disables itself or sits in a
    // caller's disabled `Ui`, which egui disables by a different path.
    for wrap_disabled in [false, true] {
        let mut harness = harness(Some(REASON), wrap_disabled);
        assert!(harness.query_by_label(REASON).is_none());
        harness
            .get_by_role_and_label(Role::Button, "Deploy")
            .hover();
        harness.run_steps(8);
        assert!(
            harness.query_by_label(REASON).is_some(),
            "tooltip should show the reason (wrap_disabled={wrap_disabled})"
        );
    }
}

#[test]
fn reason_blocks_clicks() {
    let mut harness = harness(Some(REASON), false);
    harness
        .get_by_role_and_label(Role::Button, "Deploy")
        .click();
    harness.run();
    assert_eq!(*harness.state(), 0);
}

#[test]
fn no_reason_leaves_the_button_enabled() {
    let mut harness = harness(None, false);
    let node = harness.get_by_role_and_label(Role::Button, "Deploy");
    assert!(!node.accesskit_node().is_disabled());
    assert_eq!(node.accesskit_node().description(), None);
    node.click();
    harness.run();
    assert_eq!(*harness.state(), 1);
}
