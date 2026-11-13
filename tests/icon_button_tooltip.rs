//! Icon-only buttons show their name as a tooltip: nothing else on screen
//! says what the icon does.

use egui::accesskit::Role;
use egui::{Response, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use elegance::{Button, DockStripButton, Theme, glyphs};

const NAME: &str = "Download capture";

/// `add` inside a `Ui` that is enabled or not.
fn harness<'a>(enabled: bool, add: impl Fn(&mut Ui) -> Response + 'a) -> Harness<'a> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            Theme::slate().install(ui.ctx());
            ui.add_enabled_ui(enabled, |ui| add(ui));
        });
    harness.run();
    harness
}

/// Hover the button named `NAME` and count the tooltip labels reading `text`.
fn tooltips_after_hover(harness: &mut Harness<'_>, text: &str) -> usize {
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Label, text)
            .count(),
        0
    );
    harness.get_by_role_and_label(Role::Button, NAME).hover();
    harness.run_steps(8);
    harness
        .query_all_by_role_and_label(Role::Label, text)
        .count()
}

#[test]
fn icon_button_shows_its_name_once() {
    for loading in [false, true] {
        let mut harness = harness(true, |ui| {
            ui.add(Button::icon(glyphs::DOWNLOAD, NAME).loading(loading))
        });
        assert_eq!(
            tooltips_after_hover(&mut harness, NAME),
            1,
            "loading={loading}"
        );
    }
}

#[test]
fn disabled_icon_button_still_shows_its_name() {
    // The button can be disabled itself or sit in a disabled `Ui`, which
    // egui disables by a different path.
    for parent_enabled in [true, false] {
        let mut harness = harness(parent_enabled, |ui| {
            ui.add(Button::icon(glyphs::DOWNLOAD, NAME).enabled(!parent_enabled))
        });
        let shown = tooltips_after_hover(&mut harness, NAME);
        assert_eq!(shown, 1, "parent_enabled={parent_enabled}");
    }
}

#[test]
fn disabled_reason_replaces_the_name_tooltip() {
    const REASON: &str = "Nothing captured yet";
    let mut harness = harness(true, |ui| {
        ui.add(Button::icon(glyphs::DOWNLOAD, NAME).disabled_reason(Some(REASON)))
    });
    assert_eq!(tooltips_after_hover(&mut harness, REASON), 1);
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Label, NAME)
            .count(),
        0
    );
}

#[test]
fn name_is_not_repeated_as_the_description() {
    // Screen readers read the name and then the description; the same text
    // in both would be announced twice.
    let mut harness = harness(true, |ui| ui.add(Button::icon(glyphs::DOWNLOAD, NAME)));
    assert_eq!(tooltips_after_hover(&mut harness, NAME), 1);
    let node = harness.get_by_role_and_label(Role::Button, NAME);
    assert_eq!(node.accesskit_node().description(), None);
}

#[test]
fn buttons_with_visible_labels_show_no_tooltip() {
    for icon_text in [false, true] {
        let mut harness = harness(true, |ui| {
            ui.add(if icon_text {
                Button::icon_text(glyphs::DOWNLOAD, NAME)
            } else {
                Button::new(NAME)
            })
        });
        assert_eq!(
            tooltips_after_hover(&mut harness, NAME),
            0,
            "icon_text={icon_text}"
        );
    }
}

#[test]
fn dock_strip_button_shows_its_name() {
    let mut harness = harness(true, |ui| ui.add(DockStripButton::new(glyphs::PLUS, NAME)));
    assert_eq!(tooltips_after_hover(&mut harness, NAME), 1);
    let node = harness.get_by_role_and_label(Role::Button, NAME);
    assert_eq!(node.accesskit_node().description(), None);
}
