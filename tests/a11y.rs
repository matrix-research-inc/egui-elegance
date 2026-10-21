//! Accessibility tests — verify that widgets expose the right accesskit
//! roles so screen readers and automation tools can find and classify them.
//!
//! Unlike `visual.rs`, these don't compare pixels; they run a harness long
//! enough to populate the accesskit tree and then assert on it.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use elegance::{
    BadgeTone, Drawer, DrawerSide, MenuBar, MenuItem, Modal, Select, TextInput, Theme, Toast,
    Toasts,
};

fn new_harness<'a>(app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui(app);
    // Two runs to let any deferred layout settle (Modal's Area caches size
    // on the first frame).
    harness.run();
    harness.run();
    harness
}

#[test]
fn modal_exposes_dialog_role() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        let mut open = true;
        Modal::new("a11y_dialog", &mut open)
            .heading("Confirm")
            .show(ui.ctx(), |ui| {
                ui.label("Are you sure?");
            });
    });

    // The modal's root Ui should carry Role::Dialog with the heading as label.
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Dialog, "Confirm");
}

#[test]
fn modal_alert_exposes_alert_dialog_role() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        let mut open = true;
        Modal::new("a11y_alert", &mut open)
            .heading("Delete project?")
            .alert(true)
            .show(ui.ctx(), |ui| {
                ui.label("This cannot be undone.");
            });
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::AlertDialog, "Delete project?");
}

#[test]
fn toast_info_exposes_status_role() {
    let mut enqueued = false;
    let harness = new_harness(move |ui| {
        Theme::slate().install(ui.ctx());
        if !enqueued {
            Toast::new("Saved")
                .tone(BadgeTone::Ok)
                .description("Changes written to disk")
                .show(ui.ctx());
            enqueued = true;
        }
        Toasts::new().render(ui.ctx());
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Status, "Saved");
}

#[test]
fn toast_danger_exposes_alert_role() {
    let mut enqueued = false;
    let harness = new_harness(move |ui| {
        Theme::slate().install(ui.ctx());
        if !enqueued {
            Toast::new("Upload failed")
                .tone(BadgeTone::Danger)
                .show(ui.ctx());
            enqueued = true;
        }
        Toasts::new().render(ui.ctx());
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Alert, "Upload failed");
}

#[test]
fn modal_close_button_has_close_label() {
    // Without an explicit override the button's label would be "×",
    // which screen readers announce as "multiplication sign."
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        let mut open = true;
        Modal::new("close_label", &mut open)
            .heading("Dialog")
            .show(ui.ctx(), |ui| {
                ui.label("Body");
            });
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Close");
}

#[test]
fn revealable_input_exposes_operable_toggle() {
    // The reveal toggle must be a real, queryable control: a screen-reader
    // user (or automation) has to be able to find it and operate it. It also
    // announces its current state, so the label flip is proof the masking
    // toggled.
    let mut pw = String::from("hunter2");
    let mut harness = new_harness(move |ui| {
        Theme::slate().install(ui.ctx());
        ui.add(
            TextInput::new(&mut pw)
                .label("Passphrase")
                .revealable(true)
                .id_salt("a11y_reveal"),
        );
    });

    // The field itself is still exposed by its label — the reveal layout
    // must not swallow the input's own accessibility.
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::TextInput, "Passphrase");

    // Masked by default: the toggle offers to reveal.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Reveal password")
        .click();
    harness.run();
    harness.run();

    // After activating it, the toggle now offers to hide — the state flipped.
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Hide password");
}

#[derive(Clone)]
struct FocusTestState {
    open: bool,
    email: String,
}

fn focus_harness<'a>() -> Harness<'a, FocusTestState> {
    Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui_state(
            |ui, state: &mut FocusTestState| {
                Theme::slate().install(ui.ctx());
                ui.add(
                    TextInput::new(&mut state.email)
                        .label("Email")
                        .id_salt("focus_email"),
                );
                Modal::new("focus_modal", &mut state.open)
                    .heading("Dialog")
                    .show(ui.ctx(), |ui| {
                        ui.label("Body");
                    });
            },
            FocusTestState {
                open: false,
                email: "hi".into(),
            },
        )
}

#[test]
fn modal_focuses_close_button_on_open() {
    let mut harness = focus_harness();

    // Populate the a11y tree, then focus the Email input.
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
    harness.run();

    // Flip the modal open. Modal::show's just-opened branch fires on the
    // first frame is_open is true; accesskit publishes focus on the
    // following frame.
    harness.state_mut().open = true;
    harness.run();
    harness.run();

    let close = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Close");
    assert!(
        close.accesskit_node().is_focused(),
        "close button should be focused on modal open"
    );
}

#[test]
fn modal_restores_focus_on_close() {
    let mut harness = focus_harness();

    // Focus Email, then open the modal.
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
    harness.run();
    harness.state_mut().open = true;
    harness.run();
    harness.run();

    // Close the modal — focus should return to Email.
    harness.state_mut().open = false;
    harness.run();
    harness.run();

    let email = harness.get_by_role_and_label(egui::accesskit::Role::TextInput, "Email");
    assert!(
        email.accesskit_node().is_focused(),
        "focus should be restored to the widget that had it before the modal opened"
    );
}

/// A harness that only *constructs* the modal while it's open — the common
/// "drop on close" pattern (`if let Some(m) = &self.modal { … }`). Once the
/// modal closes itself, `show()` is never called again, so any focus cleanup
/// deferred to a later `show()` would never run.
fn drop_on_close_harness<'a>() -> Harness<'a, FocusTestState> {
    Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui_state(
            |ui, state: &mut FocusTestState| {
                Theme::slate().install(ui.ctx());
                ui.add(
                    TextInput::new(&mut state.email)
                        .label("Email")
                        .id_salt("focus_email"),
                );
                if state.open {
                    Modal::new("drop_modal", &mut state.open)
                        .heading("Dialog")
                        .show(ui.ctx(), |ui| {
                            ui.label("Body");
                        });
                }
            },
            FocusTestState {
                open: false,
                email: "hi".into(),
            },
        )
}

#[test]
fn modal_restores_focus_when_dropped_on_internal_close() {
    let mut harness = drop_on_close_harness();

    // Focus Email, then open the modal.
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
    harness.run();
    harness.state_mut().open = true;
    harness.run();
    harness.run();

    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Close")
            .accesskit_node()
            .is_focused(),
        "close button should be focused on first open"
    );

    // Dismiss via Esc. The modal flips `open` false and the caller stops
    // rendering it — so cleanup must happen in the same frame the close fires,
    // not on a (never-occurring) subsequent `show()`.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
            .accesskit_node()
            .is_focused(),
        "focus should return to Email after an internal close that drops the modal"
    );

    // Reopening must still grab focus — proving the lifecycle state was reset,
    // not left stuck at `was_open = true`.
    harness.state_mut().open = true;
    harness.run();
    harness.run();

    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Close")
            .accesskit_node()
            .is_focused(),
        "close button should be focused again on the second open"
    );
}

#[test]
fn non_closable_modal_hides_close_button_and_ignores_escape() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui_state(
            |ui, open: &mut bool| {
                Theme::slate().install(ui.ctx());
                Modal::new("locked_modal", open)
                    .heading("Working…")
                    .closable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label("Please wait");
                    });
            },
            true,
        );
    harness.run();
    harness.run();

    // No "×" affordance is rendered when the modal can't be dismissed.
    assert!(
        harness
            .query_by_role_and_label(egui::accesskit::Role::Button, "Close")
            .is_none(),
        "a non-closable modal should not render a close button"
    );

    // Esc must not close it.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    assert!(
        *harness.state(),
        "Esc should not dismiss a non-closable modal"
    );
}

#[test]
fn select_keeps_its_label_node_when_staged() {
    // A staged Select paints its label by hand to fit the unsaved dot on the
    // row, which bypasses the accesskit node `egui::Label` would have emitted.
    // The tree must not change shape just because the selection diverged from
    // the committed value, so assert the Label node in both states.
    for (selected, committed) in [("ms", "ms"), ("us", "ms")] {
        let mut value = String::from(selected);
        let owned_committed = String::from(committed);
        let harness = new_harness(move |ui| {
            Theme::slate().install(ui.ctx());
            ui.add(
                Select::strings("unit", &mut value, ["us", "ms", "s"])
                    .label("Unit")
                    .saved(&owned_committed),
            );
        });

        let roles: Vec<_> = harness
            .query_all_by_label("Unit")
            .map(|node| node.accesskit_node().role())
            .collect();
        assert!(
            roles.contains(&egui::accesskit::Role::Label),
            "the field label should stay in the tree when selected={selected:?} \
             and committed={committed:?}; found roles {roles:?}"
        );
    }
}

#[test]
fn disabled_select_is_exposed_as_disabled() {
    let mut value = String::from("ms");
    let harness = new_harness(move |ui| {
        Theme::slate().install(ui.ctx());
        ui.add(
            Select::strings("unit", &mut value, ["us", "ms", "s"])
                .label("Unit")
                .enabled(false),
        );
    });

    let node = harness.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Unit");
    assert!(
        node.accesskit_node().is_disabled(),
        "a disabled Select should expose accesskit Disabled so assistive tech skips it"
    );
}

#[test]
fn drawer_exposes_dialog_role() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        let mut open = true;
        Drawer::new("a11y_drawer", &mut open)
            .side(DrawerSide::Right)
            .title("Inspector")
            .subtitle("api-west-02 · INC-2187")
            .show(ui.ctx(), |ui| {
                ui.label("Body");
            });
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Dialog, "Inspector");
}

#[test]
fn drawer_close_button_has_close_label() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        let mut open = true;
        Drawer::new("a11y_drawer_close", &mut open)
            .title("Inspector")
            .show(ui.ctx(), |ui| {
                ui.label("Body");
            });
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Close");
}

fn drawer_focus_harness<'a>() -> Harness<'a, FocusTestState> {
    Harness::builder()
        .with_size(egui::Vec2::new(600.0, 400.0))
        .build_ui_state(
            |ui, state: &mut FocusTestState| {
                Theme::slate().install(ui.ctx());
                ui.add(
                    TextInput::new(&mut state.email)
                        .label("Email")
                        .id_salt("focus_email"),
                );
                Drawer::new("focus_drawer", &mut state.open)
                    .title("Inspector")
                    .show(ui.ctx(), |ui| {
                        ui.label("Body");
                    });
            },
            FocusTestState {
                open: false,
                email: "hi".into(),
            },
        )
}

#[test]
fn drawer_focuses_close_button_on_open() {
    let mut harness = drawer_focus_harness();

    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
    harness.run();

    harness.state_mut().open = true;
    // Two extra frames: one for the open transition + accesskit publish, one
    // more so the focus request settles into the new node.
    harness.run();
    harness.run();
    harness.run();

    let close = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Close");
    assert!(
        close.accesskit_node().is_focused(),
        "close button should be focused on drawer open"
    );
}

#[test]
fn drawer_restores_focus_on_close() {
    let mut harness = drawer_focus_harness();

    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
    harness.run();
    harness.state_mut().open = true;
    harness.run();
    harness.run();

    harness.state_mut().open = false;
    harness.run();
    harness.run();

    let email = harness.get_by_role_and_label(egui::accesskit::Role::TextInput, "Email");
    assert!(
        email.accesskit_node().is_focused(),
        "focus should be restored to the widget that had it before the drawer opened"
    );
}

#[test]
fn menu_bar_triggers_expose_button_role() {
    // Each MenuBar trigger should be reachable by its label so screen
    // readers can announce them and automation tools can drive the bar.
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        MenuBar::new("a11y_menu_bar").brand("App").show(ui, |bar| {
            bar.menu("File", |_| {});
            bar.menu("Edit", |_| {});
            bar.menu("View", |_| {});
        });
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "File");
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Edit");
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "View");
}

#[test]
fn menu_item_checked_reports_selected_state() {
    // Toggle items should announce as checkboxes (with selected=true/false)
    // rather than as plain buttons, so screen readers say "checked" / "not
    // checked" instead of just "button".
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        ui.add(MenuItem::new("Show sidebar").checked(true));
        ui.add(MenuItem::new("Show minimap").checked(false));
    });

    let on = harness.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Show sidebar");
    let off = harness.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Show minimap");
    assert!(
        on.accesskit_node()
            .toggled()
            .map(|t| t == egui::accesskit::Toggled::True)
            .unwrap_or(false),
        "checked menu item should report toggled=True"
    );
    assert!(
        off.accesskit_node()
            .toggled()
            .map(|t| t == egui::accesskit::Toggled::False)
            .unwrap_or(false),
        "unchecked menu item should report toggled=False"
    );
}

#[test]
fn menu_item_radio_reports_selected_state() {
    // Radio items should announce as radio buttons. accesskit reports the
    // selected state via `Toggled::True`/`False`.
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        ui.add(MenuItem::new("Compact").radio(false));
        ui.add(MenuItem::new("Comfortable").radio(true));
        ui.add(MenuItem::new("Spacious").radio(false));
    });

    let chosen = harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, "Comfortable");
    assert!(
        chosen
            .accesskit_node()
            .toggled()
            .map(|t| t == egui::accesskit::Toggled::True)
            .unwrap_or(false),
        "selected radio menu item should report toggled=True"
    );
}

#[test]
fn color_picker_continuous_surfaces_expose_slider_roles() {
    // The SV plane, hue strip, and alpha strip are keyboard-operable, so each
    // has to announce itself and what it adjusts. Before they carried widget
    // info a screen reader saw three unlabelled targets that Tab landed on and
    // could say nothing about.
    let id = "a11y_color_picker";
    let harness = new_harness(move |ui| {
        Theme::slate().install(ui.ctx());
        let mut color = egui::Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 0x80);
        ui.set_min_size(egui::vec2(360.0, 400.0));
        let trigger = ui.add(elegance::ColorPicker::new(id, &mut color));
        egui::Popup::open_id(
            ui.ctx(),
            elegance::Popover::popup_id(trigger.id, "elegance::color_picker"),
        );
    });

    for label in ["Saturation and value", "Hue", "Alpha"] {
        let _ = harness.get_by_role_and_label(egui::accesskit::Role::Slider, label);
    }
}

#[test]
fn icon_button_announces_its_label_not_its_glyph() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        ui.add(elegance::Button::icon(
            elegance::glyphs::DOWNLOAD,
            "Download capture",
        ));
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Download capture");
    assert!(
        harness
            .query_by_label(&elegance::glyphs::DOWNLOAD.to_string())
            .is_none(),
        "the glyph itself must not reach the accessibility tree"
    );
}

#[test]
fn icon_text_button_announces_its_label_not_its_glyph() {
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        ui.add(elegance::Button::icon_text(
            elegance::glyphs::DOWNLOAD,
            "Download",
        ));
    });

    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Download");
    assert!(
        harness
            .query_by_label_contains(&elegance::glyphs::DOWNLOAD.to_string())
            .is_none(),
        "the glyph itself must not reach the accessibility tree"
    );
}

#[test]
fn unlabelled_widgets_take_an_accessible_name_from_their_response() {
    // Every widget with an optional `.label` must return the response of its
    // accessible node, so an app that draws no label can still name it with
    // egui's `Response::accessible_name`. The harness's own check also fails
    // if any of these is left unnamed.
    let harness = new_harness(|ui| {
        Theme::slate().install(ui.ctx());
        ui.set_min_size(egui::vec2(560.0, 900.0));
        let mut text = String::new();
        let mut notes = String::new();
        let mut value = 0.5_f32;
        let (mut low, mut high) = (1.0_f32, 2.0_f32);
        let mut color = egui::Color32::RED;
        let mut choice = 0_usize;
        let mut tags: Vec<String> = Vec::new();
        ui.add(TextInput::new(&mut text)).accessible_name("Input");
        ui.add(elegance::TextArea::new(&mut notes))
            .accessible_name("Notes");
        ui.add(elegance::Knob::new(&mut value, 0.0..=1.0))
            .accessible_name("Knob");
        ui.add(elegance::MetricSlider::new(&mut value, 0.0..=1.0))
            .accessible_name("Metric");
        ui.add(elegance::PercentSlider::new(&mut value))
            .accessible_name("Percent");
        ui.add(elegance::Slider::new(&mut value, 0.0..=1.0))
            .accessible_name("Slider");
        ui.add(elegance::RangeSlider::new(&mut low, &mut high, 0.0..=3.0))
            .accessible_name("Range");
        ui.add(elegance::ColorPicker::new(
            "a11y_unlabelled_color",
            &mut color,
        ))
        .accessible_name("Color");
        ui.add(
            Select::new("a11y_unlabelled_select", &mut choice).options([(0_usize, "A"), (1, "B")]),
        )
        .accessible_name("Choice");
        elegance::TagInput::new("a11y_unlabelled_tags", &mut tags)
            .show(ui)
            .response
            .accessible_name("Tags");
    });

    use egui::accesskit::Role;
    for (role, name) in [
        (Role::TextInput, "Input"),
        (Role::MultilineTextInput, "Notes"),
        (Role::Slider, "Knob"),
        (Role::Slider, "Metric"),
        (Role::Slider, "Percent"),
        (Role::Slider, "Slider"),
        (Role::ColorWell, "Color"),
        (Role::ComboBox, "Choice"),
        (Role::TextInput, "Tags"),
    ] {
        let _ = harness.get_by_role_and_label(role, name);
    }
    let _ = harness.get_by_label("Range");
}
