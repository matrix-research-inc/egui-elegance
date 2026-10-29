//! `Select::option_enabled`: disabled options stay listed but can't be picked.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use elegance::{Select, Theme};

fn harness<'a>() -> Harness<'a, u32> {
    Harness::builder()
        .with_size(egui::Vec2::new(400.0, 300.0))
        .build_ui_state(
            |ui, rate: &mut u32| {
                Theme::slate().install(ui.ctx());
                ui.add(
                    Select::new("rate", rate)
                        .label("Rate")
                        .options([
                            (44_100, "44.1 kHz"),
                            (48_000, "48 kHz"),
                            (192_000, "192 kHz"),
                        ])
                        .option_enabled(|&r| r <= 96_000),
                );
            },
            48_000,
        )
}

fn open(h: &mut Harness<'_, u32>) {
    h.get_by_role_and_label(Role::ComboBox, "Rate").click();
    h.run();
}

#[test]
fn disabled_option_is_listed_but_cannot_be_picked() {
    let mut h = harness();
    open(&mut h);

    let option = h.get_by_role_and_label(Role::Button, "192 kHz");
    assert!(
        option.accesskit_node().is_disabled(),
        "a disabled option should expose accesskit Disabled"
    );
    option.click();
    h.run();

    assert_eq!(*h.state(), 48_000);
    assert!(
        h.query_by_role_and_label(Role::Button, "192 kHz").is_some(),
        "clicking a disabled option should leave the popup open"
    );
}

#[test]
fn enabled_option_is_picked_and_closes_the_popup() {
    let mut h = harness();
    open(&mut h);

    let option = h.get_by_role_and_label(Role::Button, "44.1 kHz");
    assert!(!option.accesskit_node().is_disabled());
    option.click();
    h.run();

    assert_eq!(*h.state(), 44_100);
    assert!(
        h.query_by_role_and_label(Role::Button, "44.1 kHz")
            .is_none(),
        "picking an option should close the popup"
    );
}
