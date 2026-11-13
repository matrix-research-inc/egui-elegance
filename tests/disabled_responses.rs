//! A widget's own `enabled(false)` disables its response the way egui's
//! `Ui::add_enabled` would: `Response::enabled` is false, so egui's disabled
//! tooltip opens and its ordinary one does not. Disabling must not move the
//! widget, change its id, or shift the ids of the widgets after it.

use egui::accesskit::Role;
use egui::{Id, Rect, Response, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use elegance::{
    Accordion, Button, FileDropZone, Knob, MenuBar, MenuItem, RangeSlider, RemovableChip, Select,
    SubMenuItem, Switch, TagInput, Theme,
};

/// Adds one widget, enabled or not, and returns the response it hands back.
type AddWidget = fn(&mut Ui, bool) -> Response;

const WIDGETS: &[(&str, AddWidget)] = &[
    ("Button", |ui, on| ui.add(Button::new("Deploy").enabled(on))),
    ("Switch", |ui, on| {
        let mut state = false;
        ui.add(Switch::new(&mut state, "Wi-Fi").enabled(on))
    }),
    ("FileDropZone", |ui, on| {
        FileDropZone::new().enabled(on).show(ui).response
    }),
    ("Knob", |ui, on| {
        let mut gain = 0.5_f32;
        ui.add(Knob::new(&mut gain, 0.0..=1.0).label("Gain").enabled(on))
    }),
    ("RangeSlider", |ui, on| {
        let (mut low, mut high) = (1.0_f32, 2.0_f32);
        ui.add(
            RangeSlider::new(&mut low, &mut high, 0.0..=3.0)
                .label("Band")
                .enabled(on),
        )
    }),
    ("RemovableChip", |ui, on| {
        let mut text = String::from("alpha");
        RemovableChip::new(&mut text).enabled(on).show(ui).response
    }),
    ("TagInput", |ui, on| {
        let mut tags = vec![String::from("alpha")];
        TagInput::new("tags", &mut tags)
            .label("Tags")
            .enabled(on)
            .show(ui)
            .response
    }),
    ("MenuItem", |ui, on| {
        ui.add(MenuItem::new("Save").enabled(on))
    }),
    ("AccordionItem", |ui, on| {
        Accordion::new("sections").show(ui, |acc| {
            acc.item("Section")
                .disabled(!on)
                .show(|ui| ui.label("Body"))
                .response
        })
    }),
    ("Select", |ui, on| {
        let mut unit = String::from("ms");
        ui.add(
            Select::strings("unit", &mut unit, ["us", "ms", "s"])
                .label("Unit")
                .enabled(on),
        )
    }),
];

/// The layouts a widget must sit in identically whether enabled or not.
#[derive(Clone, Copy, Debug)]
enum Layout {
    TopDown,
    Centered,
    /// A wrapping row with a filler that leaves too little room, so the
    /// widget wraps onto the second row.
    Wrapped,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Probe {
    enabled: bool,
    id: Option<Id>,
    rect: Option<Rect>,
    /// The auto id the next widget after this one would get.
    next_id: Option<Id>,
}

fn probe(add: AddWidget, on: bool, layout: Layout) -> Probe {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 800.0))
        .build_ui_state(
            move |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let mut record = |ui: &mut Ui| {
                    let response = add(ui, on);
                    *probe = Probe {
                        enabled: response.enabled(),
                        id: Some(response.id),
                        rect: Some(response.rect),
                        next_id: Some(ui.next_auto_id()),
                    };
                };
                match layout {
                    Layout::TopDown => record(ui),
                    Layout::Centered => {
                        ui.vertical_centered(|ui| record(ui));
                    }
                    Layout::Wrapped => {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(ui.available_width() - 4.0);
                            record(ui);
                        });
                    }
                }
            },
            Probe::default(),
        );
    harness.run();
    *harness.state()
}

#[test]
fn disabled_widgets_return_disabled_responses() {
    for (name, add) in WIDGETS {
        assert!(probe(*add, true, Layout::TopDown).enabled, "{name} enabled");
        assert!(
            !probe(*add, false, Layout::TopDown).enabled,
            "{name} disabled"
        );
    }
}

#[test]
fn disabling_keeps_layout_and_ids() {
    for (name, add) in WIDGETS {
        for layout in [Layout::TopDown, Layout::Centered, Layout::Wrapped] {
            let enabled = probe(*add, true, layout);
            let disabled = probe(*add, false, layout);
            assert_eq!(enabled.rect, disabled.rect, "{name} rect in {layout:?}");
            assert_eq!(enabled.id, disabled.id, "{name} id in {layout:?}");
            assert_eq!(
                enabled.next_id, disabled.next_id,
                "{name} shifted the next widget's id in {layout:?}"
            );
        }
    }
}

#[test]
fn disabled_widget_shows_its_disabled_tooltip_only() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .build_ui(|ui| {
            Theme::slate().install(ui.ctx());
            let mut state = false;
            ui.add(Switch::new(&mut state, "Wi-Fi").enabled(false))
                .on_hover_text("Toggle Wi-Fi")
                .on_disabled_hover_text("Airplane mode is on");
        });
    harness.run();
    harness.get_by_role_and_label(Role::Switch, "Wi-Fi").hover();
    harness.run_steps(8);
    assert!(harness.query_by_label("Airplane mode is on").is_some());
    assert!(harness.query_by_label("Toggle Wi-Fi").is_none());
}

#[test]
fn disabled_knob_reports_disabled_to_assistive_tech() {
    let harness = Harness::builder()
        .with_size(egui::vec2(400.0, 300.0))
        .build_ui(|ui| {
            Theme::slate().install(ui.ctx());
            let mut gain = 0.5_f32;
            ui.add(Knob::new(&mut gain, 0.0..=1.0).label("Gain").enabled(false));
        });
    let node = harness.get_by_role_and_label(Role::Slider, "Gain");
    assert!(node.accesskit_node().is_disabled());
}

#[test]
fn disabled_submenu_does_not_open_on_hover() {
    for enabled in [true, false] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 400.0))
            .build_ui(move |ui| {
                Theme::slate().install(ui.ctx());
                MenuBar::new("app").show(ui, |bar| {
                    bar.menu("File", |ui| {
                        SubMenuItem::new("Open Recent")
                            .enabled(enabled)
                            .show(ui, |ui| ui.add(MenuItem::new("notes.txt")));
                    });
                });
            });
        harness.run();
        harness.get_by_label("File").click();
        harness.run();
        harness.get_by_label("Open Recent").hover();
        harness.run_steps(8);
        assert_eq!(
            harness.query_by_label("notes.txt").is_some(),
            enabled,
            "flyout open state for enabled={enabled}"
        );
    }
}
