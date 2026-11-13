//! `Button::icon` shows its label as a tooltip: nothing else on screen says
//! what the icon does.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use elegance::{Button, Theme, glyphs};

const NAME: &str = "Download capture";

fn harness<'a>(button: impl Fn() -> Button + 'a) -> Harness<'a> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            Theme::slate().install(ui.ctx());
            ui.add(button());
        });
    harness.run();
    harness
}

/// Hover the button named `NAME` and report whether a tooltip showed `text`.
fn tooltip_after_hover(harness: &mut Harness<'_>, text: &str) -> bool {
    assert!(harness.query_by_role_and_label(Role::Label, text).is_none());
    harness.get_by_role_and_label(Role::Button, NAME).hover();
    harness.run_steps(8);
    harness.query_by_role_and_label(Role::Label, text).is_some()
}

#[test]
fn icon_button_shows_its_name_on_hover() {
    let mut harness = harness(|| Button::icon(glyphs::DOWNLOAD, NAME));
    assert!(tooltip_after_hover(&mut harness, NAME));
}

#[test]
fn disabled_icon_button_still_shows_its_name() {
    let mut harness = harness(|| Button::icon(glyphs::DOWNLOAD, NAME).enabled(false));
    assert!(tooltip_after_hover(&mut harness, NAME));
}

#[test]
fn disabled_reason_replaces_the_name_tooltip() {
    const REASON: &str = "Nothing captured yet";
    let mut harness =
        harness(|| Button::icon(glyphs::DOWNLOAD, NAME).disabled_reason(Some(REASON)));
    assert!(tooltip_after_hover(&mut harness, REASON));
    assert!(harness.query_by_role_and_label(Role::Label, NAME).is_none());
}

#[test]
fn name_is_not_repeated_as_the_description() {
    // Screen readers read the name and then the description; the same text
    // in both would be announced twice.
    let mut harness = harness(|| Button::icon(glyphs::DOWNLOAD, NAME));
    tooltip_after_hover(&mut harness, NAME);
    let node = harness.get_by_role_and_label(Role::Button, NAME);
    assert_eq!(node.accesskit_node().description(), None);
}

#[test]
fn text_button_shows_no_tooltip() {
    // Its label is already on screen.
    let mut harness = harness(|| Button::new(NAME));
    assert!(!tooltip_after_hover(&mut harness, NAME));
}
