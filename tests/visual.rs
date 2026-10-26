//! Pixel snapshot tests for every widget × every built-in theme.
//!
//! Baselines live in `tests/snapshots/{name}_{theme}.png`. On a mismatch the
//! harness writes `{name}_{theme}.new.png` and `{name}_{theme}.diff.png`
//! beside the baseline; inspect those to see what changed.
//!
//! Regenerate after an intentional visual change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=true cargo test --test visual
//! ```

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use elegance::{
    Accent, Accordion, Avatar, AvatarGroup, AvatarPresence, AvatarSize, AvatarTone, Badge,
    BadgeTone, BrowserTab, BrowserTabs, Button, ButtonSize, Callout, CalloutTone, Card, Checkbox,
    CollapsingSection, ColorPicker, Dock, DockLayout, DockNode, DockTab, DockViewer, Drawer,
    DrawerSide, FileDropZone, GaugeZones, Indicator, IndicatorState, Knob, KnobSize, LinearGauge,
    LogBar, MenuBar, MenuItem, MenuSection, MetricSlider, Modal, PairItem, Pairing, PercentSlider,
    Popover, PopoverSide, ProgressBar, ProgressRing, RadialGauge, RangeSlider, RemovableChip,
    Segment, SegmentDot, SegmentedButton, SegmentedControl, SegmentedSize, Select, Slider,
    SliderHandle, SortableItem, SortableList, Spinner, StatCard, StatusPill, Steps, StepsStyle,
    Switch, TabBar, TagInput, TextArea, TextInput, Theme, Tooltip, TooltipSide, glyphs,
};

fn snap(name: &str, theme: Theme, ui_fn: fn(&mut egui::Ui)) {
    snap_with_setup(name, theme, ui_fn, |_| {});
}

fn snap_with_setup(name: &str, theme: Theme, ui_fn: fn(&mut egui::Ui), setup: fn(&Harness<'_>)) {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(900.0, 600.0))
        .with_pixels_per_point(2.0)
        .wgpu()
        .build_ui(move |ui| {
            theme.clone().install(ui.ctx());
            let t = Theme::current(ui.ctx());
            egui::Frame::new()
                .fill(t.palette.bg)
                .inner_margin(egui::Margin::same(16))
                .show(ui, ui_fn);
        });
    harness.run();
    harness.fit_contents();
    harness.run();
    setup(&harness);
    harness.run();
    harness.snapshot(name);
}

/// Like [`snap`] but keeps a fixed canvas instead of shrinking to fit the
/// content. Needed for overlay widgets (e.g. [`Modal`]) that paint into a
/// screen-sized `Area` rather than into the central `Ui`, so `fit_contents`
/// would collapse the canvas out from under them. Runs a couple of extra
/// frames so any deferred `Area` sizing and cross-frame bookkeeping settles.
fn snap_fixed(name: &str, theme: Theme, size: egui::Vec2, ui_fn: fn(&mut egui::Ui)) {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(2.0)
        .wgpu()
        .build_ui(move |ui| {
            theme.clone().install(ui.ctx());
            let t = Theme::current(ui.ctx());
            // Fill the whole canvas so an overlay's dimmed backdrop sits over a
            // uniform themed background rather than stray central-panel content.
            let screen = ui.ctx().content_rect();
            ui.painter().rect_filled(screen, 0.0, t.palette.bg);
            ui_fn(ui);
        });
    harness.run();
    harness.run();
    harness.snapshot(name);
}

macro_rules! theme_tests {
    ($name:ident, $ui_fn:expr) => {
        mod $name {
            use super::*;
            #[test]
            fn slate() {
                snap(
                    &format!("{}_slate", stringify!($name)),
                    Theme::slate(),
                    $ui_fn,
                );
            }
            #[test]
            fn charcoal() {
                snap(
                    &format!("{}_charcoal", stringify!($name)),
                    Theme::charcoal(),
                    $ui_fn,
                );
            }
            #[test]
            fn frost() {
                snap(
                    &format!("{}_frost", stringify!($name)),
                    Theme::frost(),
                    $ui_fn,
                );
            }
            #[test]
            fn paper() {
                snap(
                    &format!("{}_paper", stringify!($name)),
                    Theme::paper(),
                    $ui_fn,
                );
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Widget renderers. Each fn is stateless — any mutable state needed by a
// widget is declared locally so the render is deterministic across frames.
// ---------------------------------------------------------------------------

fn buttons_ui(ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (lbl, a) in [
            ("Blue", Accent::Blue),
            ("Green", Accent::Green),
            ("Red", Accent::Red),
            ("Purple", Accent::Purple),
            ("Amber", Accent::Amber),
            ("Sky", Accent::Sky),
        ] {
            ui.add(Button::new(lbl).accent(a));
        }
        ui.add(Button::new("Outline").outline());
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            Button::new("Small")
                .size(ButtonSize::Small)
                .accent(Accent::Blue),
        );
        ui.add(
            Button::new("Medium")
                .size(ButtonSize::Medium)
                .accent(Accent::Blue),
        );
        ui.add(
            Button::new("Large")
                .size(ButtonSize::Large)
                .accent(Accent::Blue),
        );
        ui.add(Button::new("Disabled").accent(Accent::Blue).enabled(false));
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            Button::new("Small")
                .size(ButtonSize::Small)
                .accent(Accent::Blue)
                .loading(true),
        );
        ui.add(Button::new("Medium").accent(Accent::Blue).loading(true));
        ui.add(
            Button::new("Large")
                .size(ButtonSize::Large)
                .accent(Accent::Blue)
                .loading(true),
        );
        ui.add(
            Button::new("+ disabled")
                .accent(Accent::Blue)
                .loading(true)
                .enabled(false),
        );
        ui.add(Button::new("Outline").outline().loading(true));
    });
}

fn icon_buttons_ui(ui: &mut egui::Ui) {
    // Each size beside a text button of the same size: the point of the square
    // geometry is that the two line up, so a regression shows up here.
    for (size, lbl) in [
        (ButtonSize::Small, "Small"),
        (ButtonSize::Medium, "Medium"),
        (ButtonSize::Large, "Large"),
    ] {
        ui.horizontal(|ui| {
            ui.add(Button::icon(glyphs::DOWNLOAD, format!("Download {lbl}")).size(size));
            ui.add(Button::new(lbl).size(size));
            ui.add(Button::icon(glyphs::TRASH, format!("Delete {lbl}")).size(size));
        });
        ui.add_space(8.0);
    }
    ui.horizontal_wrapped(|ui| {
        for (glyph, name, accent) in [
            (glyphs::UPLOAD, "Upload", Accent::Green),
            (glyphs::TRASH, "Delete", Accent::Red),
            (glyphs::PENCIL, "Edit", Accent::Purple),
            (glyphs::REFRESH, "Refresh", Accent::Amber),
            (glyphs::SEARCH, "Search", Accent::Sky),
        ] {
            ui.add(Button::icon(glyph, name).accent(accent));
        }
        ui.add(Button::icon(glyphs::COPY, "Copy").outline());
        ui.add(Button::icon(glyphs::POWER, "Power").enabled(false));
        ui.add(Button::icon(glyphs::NETWORK, "Connect").loading(true));
    });
}

fn icon_text_buttons_ui(ui: &mut egui::Ui) {
    // Each size between a text button and an icon button of the same size:
    // all three must share a height.
    for (size, lbl) in [
        (ButtonSize::Small, "Small"),
        (ButtonSize::Medium, "Medium"),
        (ButtonSize::Large, "Large"),
    ] {
        ui.horizontal(|ui| {
            ui.add(Button::new(lbl).size(size));
            ui.add(Button::icon_text(glyphs::DOWNLOAD, "Download").size(size));
            ui.add(Button::icon(glyphs::TRASH, format!("Delete {lbl}")).size(size));
        });
        ui.add_space(8.0);
    }
    ui.horizontal_wrapped(|ui| {
        for (glyph, name, accent) in [
            (glyphs::UPLOAD, "Upload", Accent::Green),
            (glyphs::TRASH, "Delete", Accent::Red),
            (glyphs::PENCIL, "Edit", Accent::Purple),
            (glyphs::REFRESH, "Refresh", Accent::Amber),
            (glyphs::SEARCH, "Search", Accent::Sky),
        ] {
            ui.add(Button::icon_text(glyph, name).accent(accent));
        }
        ui.add(Button::icon_text(glyphs::COPY, "Copy").outline());
        ui.add(Button::icon_text(glyphs::POWER, "Power").enabled(false));
        ui.add(Button::icon_text(glyphs::NETWORK, "Connect").loading(true));
        ui.add(Button::icon_text(glyphs::SEARCH, "Wide").min_width(140.0));
    });
}

fn text_inputs_ui(ui: &mut egui::Ui) {
    let mut normal = "steve@example.com".to_string();
    let mut hint = String::new();
    let mut dirty = "3000.0".to_string();
    let mut pw = "hunter2".to_string();
    let mut reveal = "correct-horse".to_string();
    ui.horizontal(|ui| {
        ui.add(
            TextInput::new(&mut normal)
                .label("Email")
                .desired_width(240.0)
                .id_salt("t_normal"),
        );
        ui.add_space(12.0);
        ui.add(
            TextInput::new(&mut hint)
                .label("With hint")
                .hint("e.g. sensor-01")
                .desired_width(240.0)
                .id_salt("t_hint"),
        );
    });
    ui.horizontal(|ui| {
        ui.add(
            TextInput::new(&mut dirty)
                .label("Dirty (unsaved)")
                .dirty(true)
                .desired_width(240.0)
                .id_salt("t_dirty"),
        );
        ui.add_space(12.0);
        ui.add(
            TextInput::new(&mut pw)
                .label("Password")
                .password(true)
                .desired_width(240.0)
                .id_salt("t_pw"),
        );
    });
    ui.horizontal(|ui| {
        ui.add(
            TextInput::new(&mut reveal)
                .label("Passphrase (revealable)")
                .revealable(true)
                .desired_width(240.0)
                .id_salt("t_reveal"),
        );
    });
}

fn removable_chips_ui(ui: &mut egui::Ui) {
    let mut filled = String::from("run-1");
    let mut empty = String::new();
    let mut disabled = String::from("locked");

    ui.horizontal(|ui| {
        RemovableChip::new(&mut filled)
            .prefix("_")
            .placeholder("run-1")
            .accent(Accent::Green)
            .id_salt("rc_filled")
            .show(ui);
        ui.add_space(8.0);
        RemovableChip::new(&mut empty)
            .placeholder("filter…")
            .accent(Accent::Sky)
            .id_salt("rc_empty")
            .show(ui);
        ui.add_space(8.0);
        RemovableChip::new(&mut disabled)
            .prefix("#")
            .placeholder("tag")
            .enabled(false)
            .id_salt("rc_disabled")
            .show(ui);
    });
}

fn tag_inputs_ui(ui: &mut egui::Ui) {
    let mut recipients: Vec<String> = vec![
        "thomas@example.com".into(),
        "team@orbit.dev".into(),
        "oncall@orbit.dev".into(),
    ];
    let mut tags: Vec<String> = vec!["rust".into(), "egui".into()];
    let mut empty: Vec<String> = Vec::new();

    ui.allocate_ui(egui::vec2(420.0, 0.0), |ui| {
        TagInput::new("ti_recipients", &mut recipients)
            .label("Recipients")
            .placeholder("Add an email…")
            .commit_on_space(true)
            .show(ui);
        ui.add_space(10.0);
        TagInput::new("ti_tags", &mut tags)
            .label("Tags")
            .placeholder("Add tags…")
            .accent(Accent::Purple)
            .show(ui);
        ui.add_space(10.0);
        TagInput::new("ti_empty", &mut empty)
            .label("Skills")
            .placeholder("e.g. Rust, egui, gpu…")
            .show(ui);
    });
}

fn text_areas_ui(ui: &mut egui::Ui) {
    let mut body = "A multi-line text area.\nDrop notes, logs, or JSON here.".to_string();
    let mut mono = "{\n  \"id\": 42,\n  \"ok\": true\n}".to_string();
    ui.horizontal(|ui| {
        ui.add(
            TextArea::new(&mut body)
                .label("Text area")
                .rows(3)
                .desired_width(240.0)
                .id_salt("a_body"),
        );
        ui.add_space(12.0);
        ui.add(
            TextArea::new(&mut mono)
                .label("Monospace")
                .monospace(true)
                .rows(3)
                .desired_width(240.0)
                .id_salt("a_mono"),
        );
    });
}

fn selects_ui(ui: &mut egui::Ui) {
    let mut unit = "ms".to_string();
    let mut env = "Production".to_string();
    let mut region = "eu-west-1".to_string();
    let saved_region = "us-east-1".to_string();
    ui.horizontal(|ui| {
        ui.add(
            Select::strings("s_unit", &mut unit, ["us", "ms", "s"])
                .label("Unit")
                .width(120.0),
        );
        ui.add_space(16.0);
        ui.add(
            Select::strings("s_env", &mut env, ["Production", "Staging", "Development"])
                .label("Environment")
                .width(180.0),
        );
        ui.add_space(16.0);
        ui.add(
            Select::strings("s_region", &mut region, ["eu-west-1", "us-east-1"])
                .label("Region")
                .saved(&saved_region)
                .width(140.0),
        );
    });
}

fn toggles_ui(ui: &mut egui::Ui) {
    let mut check_on = true;
    let mut check_off = false;
    let mut switch_on = true;
    let mut switch_off = false;
    let mut switch_green = true;
    let mut seg_on = true;
    let mut seg_off = false;

    let theme = Theme::current(ui.ctx());
    ui.label(theme.muted_text("Checkbox"));
    ui.horizontal(|ui| {
        ui.add(Checkbox::new(&mut check_on, "Enabled"));
        ui.add_space(16.0);
        ui.add(Checkbox::new(&mut check_off, "Off"));
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("Switch"));
    ui.horizontal(|ui| {
        ui.add(Switch::new(&mut switch_on, "On"));
        ui.add_space(16.0);
        ui.add(Switch::new(&mut switch_off, "Off"));
        ui.add_space(16.0);
        ui.add(Switch::new(&mut switch_green, "Green accent").accent(Accent::Green));
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("Segmented"));
    ui.horizontal(|ui| {
        ui.add(
            SegmentedButton::new(&mut seg_on, "Record")
                .accent(Accent::Green)
                .min_width(140.0),
        );
        ui.add_space(8.0);
        ui.add(
            SegmentedButton::new(&mut seg_off, "Preview")
                .accent(Accent::Blue)
                .min_width(140.0),
        );
    });
    ui.add_space(10.0);

    // Mixed-control row: Button + SegmentedButton share theme.control_padding_y
    // so they align cleanly at any shared ButtonSize.
    ui.label(theme.muted_text("Mixed action row (Medium, Large)"));
    ui.horizontal(|ui| {
        ui.add(Button::new("Collect").accent(Accent::Green).min_width(96.0));
        ui.add_space(8.0);
        ui.add(
            SegmentedButton::new(&mut seg_on, "Continuous")
                .accent(Accent::Green)
                .min_width(140.0),
        );
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add(
            Button::new("Collect")
                .accent(Accent::Green)
                .size(ButtonSize::Large)
                .min_width(120.0),
        );
        ui.add_space(8.0);
        ui.add(
            SegmentedButton::new(&mut seg_on, "Continuous")
                .accent(Accent::Green)
                .size(ButtonSize::Large)
                .min_width(180.0),
        );
    });
}

fn tabs_ui(ui: &mut egui::Ui) {
    let mut tab = 1usize;
    ui.set_min_width(520.0);
    ui.add(TabBar::new(
        &mut tab,
        ["Overview", "Settings", "Activity", "Logs"],
    ));
}

fn segmented_control_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());

    ui.set_min_width(640.0);

    ui.label(theme.muted_text("Sizes"));
    ui.horizontal(|ui| {
        let mut sm = 1usize;
        ui.add(SegmentedControl::new(&mut sm, ["Day", "Week", "Month"]).size(SegmentedSize::Small));
        ui.add_space(12.0);
        let mut md = 1usize;
        ui.add(SegmentedControl::new(
            &mut md,
            ["Day", "Week", "Month", "Year"],
        ));
        ui.add_space(12.0);
        let mut lg = 1usize;
        ui.add(
            SegmentedControl::new(&mut lg, ["Private", "Internal", "Public"])
                .size(SegmentedSize::Large),
        );
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("Disabled segment"));
    let mut lang = 0usize;
    ui.add(SegmentedControl::from_segments(
        &mut lang,
        [
            Segment::text("EN"),
            Segment::text("DE"),
            Segment::text("JA"),
            Segment::text("FR").enabled(false),
        ],
    ));
    ui.add_space(10.0);

    ui.label(theme.muted_text("Filter row with status dots and counts"));
    let mut bucket = 0usize;
    ui.add(
        SegmentedControl::from_segments(
            &mut bucket,
            [
                Segment::text("Open").dot(SegmentDot::Amber).count("12"),
                Segment::text("Triaged")
                    .dot(SegmentDot::Neutral)
                    .count("84"),
                Segment::text("Resolved")
                    .dot(SegmentDot::Green)
                    .count("1,204"),
                Segment::text("Rejected").dot(SegmentDot::Red).count("31"),
            ],
        )
        .fill(),
    );
    ui.add_space(10.0);

    ui.label(theme.muted_text("Multi-toggle (independent on/off per segment)"));
    ui.horizontal(|ui| {
        let mut both_on = [true, true];
        ui.add(SegmentedControl::toggles(
            &mut both_on,
            ["Server", "Client"],
        ));
        ui.add_space(12.0);
        let mut one_on = [true, false];
        ui.add(SegmentedControl::toggles(&mut one_on, ["Server", "Client"]));
        ui.add_space(12.0);
        let mut none_on = [false, false];
        ui.add(SegmentedControl::toggles(
            &mut none_on,
            ["Server", "Client"],
        ));
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("Filled accent style (multi-toggle, drag to paint)"));
    let mut channels = [true, false, true, true, false, false, true, false];
    ui.add(
        SegmentedControl::toggles(&mut channels, ["1", "2", "3", "4", "5", "6", "7", "8"])
            .accent(Accent::Green),
    );
    ui.add_space(6.0);
    let mut single_idx = 1usize;
    ui.add(
        SegmentedControl::new(&mut single_idx, ["Day", "Week", "Month", "Year"])
            .accent(Accent::Blue),
    );
}

fn browser_tabs_ui(ui: &mut egui::Ui) {
    let mut tabs = BrowserTabs::new("vis_browser_tabs")
        .with_tab(BrowserTab::new("readme", "README.md"))
        .with_tab(BrowserTab::new("theme", "theme.rs").dirty(true))
        .with_tab(BrowserTab::new("button", "widgets/button.rs"))
        .with_tab(BrowserTab::new(
            "cargo",
            "cargo output \u{2014} a longer title that truncates",
        ));
    tabs.set_selected("theme");
    ui.set_min_width(720.0);
    tabs.show(ui);
}

struct DockDemoViewer;

impl DockViewer for DockDemoViewer {
    type Tab = String;

    fn tab(&mut self, id: &String) -> DockTab {
        match id.as_str() {
            "files" => DockTab::new("Files").icon(glyphs::FOLDER),
            "editor" => DockTab::new("theme.rs").dirty(true),
            "preview" => DockTab::new("Preview"),
            "console" => DockTab::new("Console")
                .icon(glyphs::TERMINAL)
                .dirty(true)
                .closable(false),
            "long" => DockTab::new("a_really_long_file_name.rs").dirty(true),
            "search" => DockTab::new("Search").icon(glyphs::SEARCH).dirty(true),
            other => DockTab::new(other),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, id: &String) {
        let theme = Theme::current(ui.ctx());
        ui.label(theme.muted_text(format!("Contents of {id}")));
        ui.add_space(6.0);
        ui.add(Button::new("Action"));
    }
}

fn dock_layout() -> DockLayout<String> {
    let mut layout = DockLayout::from_node(DockNode::row(
        0.28,
        DockNode::tabs(["files"]),
        DockNode::column(
            0.62,
            DockNode::tabs(["editor", "preview"]),
            DockNode::tabs(["console"]),
        ),
    ));
    layout.activate("editor");
    layout
}

fn dock_ui(ui: &mut egui::Ui) {
    let mut layout = dock_layout();
    ui.allocate_ui(egui::vec2(720.0, 360.0), |ui| {
        Dock::new("vis_dock").show(ui, &mut layout, &mut DockDemoViewer);
    });
}

/// A narrow dock whose strip overflows: tabs shrink to their minimum width,
/// and the strip scrolls behind an overflow button.
fn dock_overflow_ui(ui: &mut egui::Ui) {
    let mut layout: DockLayout<String> =
        DockLayout::single(["files", "editor", "preview", "long", "search"]);
    // A middle tab, so tabs are hidden past both ends of the strip.
    layout.activate("preview");
    ui.allocate_ui(egui::vec2(300.0, 140.0), |ui| {
        Dock::new("vis_dock_overflow").show(ui, &mut layout, &mut DockDemoViewer);
    });
}

/// Snapshot a dock mid-drag: `from` and `to` are tab / pane labels resolved
/// to their on-screen centres once the first frame has been laid out.
fn snap_dock_drag(name: &str, theme: Theme, grab: &str, drop_at: fn(egui::Rect) -> egui::Pos2) {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(760.0, 400.0))
        .with_pixels_per_point(2.0)
        .wgpu()
        .build_ui(move |ui| {
            theme.clone().install(ui.ctx());
            let t = Theme::current(ui.ctx());
            let screen = ui.ctx().content_rect();
            ui.painter().rect_filled(screen, 0.0, t.palette.bg);
            egui::Frame::new()
                .inner_margin(egui::Margin::same(16))
                .show(ui, dock_ui);
        });
    harness.run();
    let from = harness.get_by_label(grab).rect().center();
    let target = harness.get_by_label("Contents of editor").rect();
    harness.hover_at(from);
    harness.run_steps(2);
    harness.drag_at(from);
    harness.run_steps(2);
    let to = drop_at(target);
    harness.hover_at(from.lerp(to, 0.5));
    harness.run_steps(2);
    harness.hover_at(to);
    harness.run_steps(3);
    harness.snapshot(name);
}

fn status_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());

    ui.label(theme.muted_text("StatusPill"));
    ui.add(
        StatusPill::new()
            .item("UI", IndicatorState::On)
            .item("API", IndicatorState::Connecting)
            .item("DB", IndicatorState::Off),
    );
    ui.add_space(10.0);

    ui.label(theme.muted_text("Indicators"));
    ui.horizontal(|ui| {
        for (s, text) in [
            (IndicatorState::On, "On"),
            (IndicatorState::Connecting, "Connecting"),
            (IndicatorState::Off, "Off"),
        ] {
            ui.add(Indicator::new(s));
            ui.label(theme.faint_text(text));
            ui.add_space(12.0);
        }
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("Badges"));
    ui.horizontal_wrapped(|ui| {
        ui.add(Badge::new("OK", BadgeTone::Ok));
        ui.add(Badge::new("Warning", BadgeTone::Warning));
        ui.add(Badge::new("Error", BadgeTone::Danger));
        ui.add(Badge::new("Info", BadgeTone::Info));
        ui.add(Badge::new("Neutral", BadgeTone::Neutral));
    });
    ui.add_space(6.0);
    ui.label(theme.muted_text("Case-preserving (identifiers)"));
    ui.horizontal_wrapped(|ui| {
        ui.add(Badge::new("feature/login", BadgeTone::Ok).preserve_case());
        ui.add(Badge::new("v2.14.3", BadgeTone::Neutral).preserve_case());
        ui.add(Badge::new("@carmela", BadgeTone::Info).preserve_case());
    });
}

fn avatars_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_width(620.0);

    ui.label(theme.muted_text("Sizes"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for size in [
            AvatarSize::XSmall,
            AvatarSize::Small,
            AvatarSize::Medium,
            AvatarSize::Large,
            AvatarSize::XLarge,
        ] {
            ui.add(Avatar::new("EL").size(size).tone(AvatarTone::Sky));
        }
    });
    ui.add_space(12.0);

    ui.label(theme.muted_text("Auto-tone from name"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for initials in ["AL", "MR", "JK", "DP", "NV", "??"] {
            ui.add(Avatar::new(initials));
        }
    });
    ui.add_space(12.0);

    ui.label(theme.muted_text("Presence dots"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.add(
            Avatar::new("MR")
                .size(AvatarSize::Large)
                .tone(AvatarTone::Green)
                .presence(AvatarPresence::Online),
        );
        ui.add(
            Avatar::new("JK")
                .size(AvatarSize::Large)
                .tone(AvatarTone::Amber)
                .presence(AvatarPresence::Away),
        );
        ui.add(
            Avatar::new("DP")
                .size(AvatarSize::Large)
                .tone(AvatarTone::Red)
                .presence(AvatarPresence::Busy),
        );
        ui.add(
            Avatar::new("NV")
                .size(AvatarSize::Large)
                .tone(AvatarTone::Neutral)
                .presence(AvatarPresence::Offline),
        );
    });
    ui.add_space(12.0);

    ui.label(theme.muted_text("Stacked groups"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 28.0;
        ui.add(
            AvatarGroup::new()
                .size(AvatarSize::Medium)
                .item(Avatar::new("AL").tone(AvatarTone::Sky))
                .item(Avatar::new("MR").tone(AvatarTone::Green))
                .item(Avatar::new("JK").tone(AvatarTone::Amber))
                .item(Avatar::new("DP").tone(AvatarTone::Red))
                .overflow(7),
        );
        ui.add(
            AvatarGroup::new()
                .size(AvatarSize::Small)
                .item(Avatar::new("NV").tone(AvatarTone::Purple))
                .item(Avatar::new("AL").tone(AvatarTone::Sky))
                .item(Avatar::new("MR").tone(AvatarTone::Green)),
        );
    });
    ui.add_space(12.0);

    ui.label(theme.muted_text("On a card surface"));
    Card::new().show(ui, |ui| {
        ui.horizontal(|ui| {
            let card_bg = Theme::current(ui.ctx()).palette.card;
            ui.add(
                Avatar::new("AL")
                    .size(AvatarSize::Large)
                    .tone(AvatarTone::Sky)
                    .presence(AvatarPresence::Online)
                    .surface(card_bg),
            );
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.add(egui::Label::new(theme.body_text("Avery Lin")));
                ui.add(egui::Label::new(theme.muted_text("Engineering · online")));
            });
        });
    });
}

fn sliders_ui(ui: &mut egui::Ui) {
    let mut threshold: u32 = 48;
    let mut gain: f32 = 0.62;
    ui.set_max_width(420.0);
    ui.add(
        Slider::new(&mut threshold, 0u32..=100u32)
            .label("Threshold")
            .suffix("%"),
    );
    ui.add_space(6.0);
    ui.add(
        Slider::new(&mut gain, 0.0..=1.0)
            .label("Gain")
            .accent(Accent::Green),
    );
}

fn sliders_line_ui(ui: &mut egui::Ui) {
    let mut threshold: u32 = 48;
    let mut gain: f32 = 0.62;
    ui.set_max_width(420.0);
    ui.add(
        Slider::new(&mut threshold, 0u32..=100u32)
            .label("Threshold")
            .suffix("%")
            .handle(SliderHandle::Line),
    );
    ui.add_space(6.0);
    ui.add(
        Slider::new(&mut gain, 0.0..=1.0)
            .label("Gain")
            .accent(Accent::Green)
            .handle(SliderHandle::Line),
    );
}

fn percent_sliders_ui(ui: &mut egui::Ui) {
    let mut cache: f32 = 45.0;
    let mut retention: f32 = 30.0;
    let mut compact: f32 = 70.0;
    let mut tier: f32 = 25.0;
    ui.set_max_width(420.0);
    ui.add(
        PercentSlider::new(&mut cache)
            .label("Cache window")
            .callout_fmt(|p| {
                let mins = (p * 60.0 / 100.0).round() as i32;
                format!("{mins} min")
            }),
    );
    ui.add_space(10.0);
    ui.add(
        PercentSlider::new(&mut retention)
            .label("Snapshot retention")
            .step(5.0)
            .accent(Accent::Green),
    );
    ui.add_space(10.0);
    ui.add(
        PercentSlider::new(&mut compact)
            .label("Disk share")
            .show_ticks(false)
            .accent(Accent::Purple)
            // Callout opts back in to the percent prefix when the headline
            // alone isn't enough context (e.g. the slider is small and
            // glanceable rather than the user's focus).
            .callout_fmt(|p| format!("{}% \u{00B7} {:.1} GB", p.round() as i32, p * 4.0 / 100.0)),
    );
    ui.add_space(10.0);
    ui.add(
        PercentSlider::new(&mut tier)
            .label("Bandwidth tier")
            .stops([0.0, 10.0, 25.0, 75.0, 100.0])
            .accent(Accent::Amber),
    );
}

fn percent_sliders_line_ui(ui: &mut egui::Ui) {
    let mut cache: f32 = 45.0;
    let mut retention: f32 = 30.0;
    ui.set_max_width(420.0);
    ui.add(
        PercentSlider::new(&mut cache)
            .label("Cache window")
            .handle(SliderHandle::Line)
            .callout_fmt(|p| {
                let mins = (p * 60.0 / 100.0).round() as i32;
                format!("{mins} min")
            }),
    );
    ui.add_space(10.0);
    ui.add(
        PercentSlider::new(&mut retention)
            .label("Snapshot retention")
            .step(5.0)
            .accent(Accent::Green)
            .handle(SliderHandle::Line),
    );
}

fn metric_sliders_ui(ui: &mut egui::Ui) {
    let mut buffer: f32 = 16.0;
    let mut latency: f32 = 175.0;
    let mut refresh: f32 = 120.0;
    let mut tier: f32 = 2.0;
    ui.set_max_width(420.0);
    ui.add(
        MetricSlider::new(&mut buffer, 0.0..=32.0)
            .label("Buffer size")
            .suffix("GiB")
            .stops([4.0, 8.0, 16.0, 32.0])
            .accent(Accent::Amber),
    );
    ui.add_space(10.0);
    ui.add(
        MetricSlider::new(&mut latency, 0.0..=500.0)
            .label("Latency budget")
            .suffix("ms")
            .step(25.0)
            .accent(Accent::Red),
    );
    ui.add_space(10.0);
    ui.add(
        MetricSlider::new(&mut refresh, 30.0..=240.0)
            .label("Refresh rate")
            .suffix("Hz")
            .steps(8)
            .accent(Accent::Green),
    );
    ui.add_space(10.0);
    // Headline-fmt override: the value is a tier index but the headline
    // renders its name. No suffix; ticks show short labels.
    ui.add(
        MetricSlider::new(&mut tier, 0.0..=3.0)
            .label("Plan")
            .stops([0.0, 1.0, 2.0, 3.0])
            .headline_fmt(|v| {
                ["Free", "Lite", "Pro", "Studio"][v.round().clamp(0.0, 3.0) as usize].to_string()
            })
            .tick_fmt(|v| ["F", "L", "P", "S"][v.round().clamp(0.0, 3.0) as usize].to_string())
            .accent(Accent::Purple),
    );
}

fn metric_sliders_line_ui(ui: &mut egui::Ui) {
    let mut buffer: f32 = 16.0;
    let mut refresh: f32 = 120.0;
    ui.set_max_width(420.0);
    ui.add(
        MetricSlider::new(&mut buffer, 0.0..=32.0)
            .label("Buffer size")
            .suffix("GiB")
            .stops([4.0, 8.0, 16.0, 32.0])
            .accent(Accent::Amber)
            .handle(SliderHandle::Line),
    );
    ui.add_space(10.0);
    ui.add(
        MetricSlider::new(&mut refresh, 30.0..=240.0)
            .label("Refresh rate")
            .suffix("Hz")
            .steps(8)
            .accent(Accent::Green)
            .handle(SliderHandle::Line),
    );
}

fn knobs_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_max_width(560.0);

    // Row of small instrument-panel knobs.
    ui.label(theme.muted_text("Instrument panel"));
    ui.horizontal_wrapped(|ui| {
        let mut gain = -12.0_f32;
        let mut cutoff = 1000.0_f32;
        let mut q = 2.4_f32;
        let mut mix = 35_u32;
        ui.spacing_mut().item_spacing.x = 14.0;
        ui.add(
            Knob::new(&mut gain, -60.0..=12.0)
                .label("Gain")
                .size(KnobSize::Small)
                .default(0.0_f32)
                .show_value(true)
                .value_fmt(|v| format!("{v:.0} dB")),
        );
        ui.add(
            Knob::new(&mut cutoff, 20.0..=20000.0)
                .label("Cutoff")
                .size(KnobSize::Small)
                .log_scale()
                .default(1000.0_f32)
                .show_value(true)
                .value_fmt(|v| {
                    if v >= 1000.0 {
                        format!("{:.1} kHz", v / 1000.0)
                    } else {
                        format!("{v:.0} Hz")
                    }
                }),
        );
        ui.add(
            Knob::new(&mut q, 0.1..=10.0)
                .label("Q")
                .size(KnobSize::Small)
                .log_scale()
                .default(0.707_f32)
                .show_value(true),
        );
        ui.add(
            Knob::new(&mut mix, 0u32..=100u32)
                .label("Mix")
                .size(KnobSize::Small)
                .default(50_u32)
                .show_value(true)
                .value_fmt(|v| format!("{v:.0}%"))
                .accent(Accent::Green),
        );
    });
    ui.add_space(14.0);

    // Stepped knob with labeled detents.
    let caption = ui.label(theme.muted_text("Stepped Timebase"));
    let mut idx: u32 = 3;
    ui.add(
        Knob::new(&mut idx, 0u32..=8u32)
            .size(KnobSize::Large)
            .step(1.0)
            .detents([
                (0u32, "1µ"),
                (1u32, "2µ"),
                (2u32, "5µ"),
                (3u32, "10µ"),
                (4u32, "20µ"),
                (5u32, "50µ"),
                (6u32, "100µ"),
                (7u32, "200µ"),
                (8u32, "500µ"),
            ])
            .default(3_u32),
    )
    .labelled_by(caption.id);
    ui.add_space(14.0);

    // Bipolar knob.
    ui.label(theme.muted_text("Bipolar DC offset"));
    let mut dc = -1.4_f32;
    ui.add(
        Knob::new(&mut dc, -5.0..=5.0)
            .label("DC offset")
            .bipolar()
            .accent(Accent::Purple)
            .default(0.0_f32)
            .show_value(true)
            .value_fmt(|v| format!("{:+.2} V", v)),
    );
}

fn stat_cards_ui(ui: &mut egui::Ui) {
    const DEPLOYS: &[f32] = &[
        12.0, 14.0, 13.0, 15.0, 17.0, 16.0, 18.0, 20.0, 19.0, 22.0, 21.0, 24.0, 22.0, 24.0, 26.0,
        24.0, 27.0, 28.0, 30.0, 28.0, 26.0, 24.0,
    ];
    const ERROR: &[f32] = &[
        0.8, 0.9, 0.82, 0.75, 0.7, 0.62, 0.6, 0.58, 0.55, 0.5, 0.48, 0.46, 0.44, 0.45, 0.4, 0.42,
        0.4, 0.38, 0.42, 0.4, 0.41, 0.42,
    ];
    const P95: &[f32] = &[
        120.0, 118.0, 122.0, 125.0, 128.0, 130.0, 135.0, 140.0, 142.0, 148.0, 150.0, 155.0, 160.0,
        162.0, 168.0, 170.0, 175.0, 178.0, 182.0, 180.0, 184.0, 186.0,
    ];

    let cell_w = 230.0_f32;
    ui.set_min_width(cell_w * 4.0 + 36.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.add(
            StatCard::new("Active deploys")
                .accent(Accent::Blue)
                .value("24")
                .delta(0.12)
                .trend("vs last 7 days")
                .sparkline(DEPLOYS)
                .width(cell_w),
        );
        ui.add(
            StatCard::new("Error rate")
                .accent(Accent::Purple)
                .value("0.42")
                .unit("%")
                .delta(-0.08)
                .invert_delta(true)
                .trend("vs last 24h")
                .sparkline(ERROR)
                .width(cell_w),
        );
        ui.add(
            StatCard::new("P95 latency")
                .accent(Accent::Amber)
                .value("184")
                .unit("ms")
                .delta(0.24)
                .invert_delta(true)
                .trend("regressed vs last hour")
                .sparkline(P95)
                .width(cell_w),
        );
        ui.add(
            StatCard::new("Revenue today")
                .accent(Accent::Green)
                .loading(true)
                .width(cell_w),
        );
    });
}

fn range_sliders_ui(ui: &mut egui::Ui) {
    let (mut price_lo, mut price_hi): (u32, u32) = (24, 118);
    let (mut latency_lo, mut latency_hi): (u32, u32) = (120, 340);
    let (mut volume_lo, mut volume_hi): (u32, u32) = (18, 62);
    let (mut retention_lo, mut retention_hi): (u32, u32) = (7, 30);
    let w = 520.0_f32;
    Card::new().show(ui, |ui| {
        ui.set_max_width(w);
        ui.add(
            RangeSlider::new(&mut price_lo, &mut price_hi, 0u32..=200u32)
                .label("Price")
                .value_fmt(|v| format!("${v:.0}"))
                .desired_width(w)
                .id_salt("rs_price"),
        );
        ui.add_space(8.0);
        ui.add(
            RangeSlider::new(&mut latency_lo, &mut latency_hi, 0u32..=500u32)
                .label("Latency target")
                .suffix(" ms")
                .step(10.0)
                .ticks(6)
                .show_tick_labels(true)
                .desired_width(w)
                .id_salt("rs_latency"),
        );
        ui.add_space(8.0);
        ui.add(
            RangeSlider::new(&mut volume_lo, &mut volume_hi, 0u32..=100u32)
                .label("Volume")
                .suffix(" dB")
                .accent(Accent::Green)
                .desired_width(w)
                .id_salt("rs_volume"),
        );
        ui.add_space(8.0);
        ui.add(
            RangeSlider::new(&mut retention_lo, &mut retention_hi, 1u32..=90u32)
                .label("Retention window (locked)")
                .suffix(" days")
                .enabled(false)
                .desired_width(w)
                .id_salt("rs_retention"),
        );
    });
}

fn range_sliders_line_ui(ui: &mut egui::Ui) {
    let (mut price_lo, mut price_hi): (u32, u32) = (24, 118);
    let (mut latency_lo, mut latency_hi): (u32, u32) = (120, 340);
    let (mut retention_lo, mut retention_hi): (u32, u32) = (7, 30);
    let w = 520.0_f32;
    Card::new().show(ui, |ui| {
        ui.set_max_width(w);
        ui.add(
            RangeSlider::new(&mut price_lo, &mut price_hi, 0u32..=200u32)
                .label("Price")
                .value_fmt(|v| format!("${v:.0}"))
                .handle(SliderHandle::Line)
                .desired_width(w)
                .id_salt("rsl_price"),
        );
        ui.add_space(8.0);
        ui.add(
            RangeSlider::new(&mut latency_lo, &mut latency_hi, 0u32..=500u32)
                .label("Latency target")
                .suffix(" ms")
                .step(10.0)
                .ticks(6)
                .show_tick_labels(true)
                .handle(SliderHandle::Line)
                .desired_width(w)
                .id_salt("rsl_latency"),
        );
        ui.add_space(8.0);
        ui.add(
            RangeSlider::new(&mut retention_lo, &mut retention_hi, 1u32..=90u32)
                .label("Retention window (locked)")
                .suffix(" days")
                .enabled(false)
                .handle(SliderHandle::Line)
                .desired_width(w)
                .id_salt("rsl_retention"),
        );
    });
}

fn feedback_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_max_width(500.0);

    ui.label(theme.muted_text("Spinner"));
    ui.horizontal(|ui| {
        ui.add(Spinner::new().size(14.0));
        ui.add_space(10.0);
        ui.add(Spinner::new());
        ui.add_space(10.0);
        ui.add(Spinner::new().size(28.0));
        ui.add_space(20.0);
        ui.add(Spinner::new().accent(Accent::Green));
        ui.add(Spinner::new().accent(Accent::Amber));
        ui.add(Spinner::new().accent(Accent::Red));
        ui.add(Spinner::new().accent(Accent::Purple));
    });
    ui.add_space(10.0);

    ui.label(theme.muted_text("ProgressBar"));
    ui.add(ProgressBar::new(0.25));
    ui.add_space(4.0);
    ui.add(ProgressBar::new(0.6).accent(Accent::Green));
    ui.add_space(4.0);
    ui.add(ProgressBar::new(1.0).accent(Accent::Amber).text("Complete"));
}

fn progress_rings_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_max_width(560.0);

    ui.label(theme.muted_text("Progression"));
    ui.horizontal(|ui| {
        for f in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            ui.add(ProgressRing::new(f));
            ui.add_space(8.0);
        }
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Sizes"));
    ui.horizontal(|ui| {
        ui.add(ProgressRing::new(0.62).size(36.0));
        ui.add_space(12.0);
        ui.add(ProgressRing::new(0.62).size(56.0));
        ui.add_space(12.0);
        ui.add(ProgressRing::new(0.62).size(88.0));
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Accents"));
    ui.horizontal(|ui| {
        for a in [
            Accent::Sky,
            Accent::Green,
            Accent::Amber,
            Accent::Red,
            Accent::Purple,
        ] {
            ui.add(ProgressRing::new(0.62).accent(a));
            ui.add_space(8.0);
        }
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Custom centre"));
    ui.horizontal(|ui| {
        ui.add(
            ProgressRing::new(0.6)
                .size(88.0)
                .text("12 / 20")
                .caption("files"),
        );
        ui.add_space(16.0);
        ui.add(
            ProgressRing::new(0.83)
                .size(88.0)
                .accent(Accent::Amber)
                .text("3.4s")
                .caption("remaining"),
        );
        ui.add_space(16.0);
        ui.add(ProgressRing::new(1.0).size(72.0).accent(Accent::Green));
    });
}

fn gauge_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    let zones = GaugeZones::new(0.6, 0.85);
    ui.set_min_width(720.0);

    ui.label(theme.muted_text("Radial — half-circle with threshold zones"));
    ui.horizontal(|ui| {
        ui.add(RadialGauge::new(0.42).zones(zones).size(180.0));
        ui.add_space(20.0);
        ui.add(RadialGauge::new(0.72).zones(zones).size(180.0));
        ui.add_space(20.0);
        ui.add(RadialGauge::new(0.94).zones(zones).size(180.0));
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Donut — ProgressRing in gauge mode"));
    ui.horizontal(|ui| {
        ui.add(
            ProgressRing::new(0.68)
                .size(160.0)
                .zones(zones)
                .text("68")
                .unit("GB")
                .caption_below("of 100"),
        );
        ui.add_space(20.0);
        ui.add(
            ProgressRing::new(0.65)
                .size(160.0)
                .zones(zones)
                .text("65")
                .unit("%")
                .caption_below("of monthly budget"),
        );
        ui.add_space(20.0);
        ui.add(
            ProgressRing::new(0.8)
                .size(160.0)
                .text("32")
                .unit("/ 40")
                .caption_below("widgets complete"),
        );
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Linear — meter with threshold zones"));
    Card::new().show(ui, |ui| {
        ui.set_min_width(640.0);
        for (label, frac, value) in [
            ("CPU", 0.42_f32, "42%"),
            ("Memory", 0.72, "72%"),
            ("Disk", 0.94, "94%"),
        ] {
            ui.horizontal(|ui| {
                ui.add_sized([100.0, 14.0], egui::Label::new(theme.body_text(label)));
                ui.add(
                    LinearGauge::new(frac)
                        .zones(zones)
                        .show_zone_labels()
                        .desired_width(420.0),
                );
                ui.add_sized([60.0, 14.0], egui::Label::new(theme.muted_text(value)));
            });
            ui.add_space(4.0);
        }
        ui.horizontal(|ui| {
            ui.add_sized(
                [100.0, 14.0],
                egui::Label::new(theme.body_text("Queue depth")),
            );
            ui.add(
                LinearGauge::new(186.0 / 850.0)
                    .zones(GaugeZones::new(0.4, 0.75))
                    .threshold_label(0.4, "340")
                    .threshold_label(0.75, "638")
                    .desired_width(420.0),
            );
            ui.add_sized(
                [60.0, 14.0],
                egui::Label::new(theme.muted_text("186 / 850")),
            );
        });
    });
}

fn steps_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_max_width(520.0);

    ui.label(theme.muted_text("Cells — in progress"));
    ui.add(Steps::new(6).current(4));
    ui.add_space(10.0);

    ui.label(theme.muted_text("Cells — errored"));
    ui.add(Steps::new(5).current(2).errored(true));
    ui.add_space(10.0);

    ui.label(theme.muted_text("Cells — complete"));
    ui.add(Steps::new(4).current(4));
    ui.add_space(14.0);

    ui.label(theme.muted_text("Numbered — in progress"));
    ui.add(Steps::new(5).current(2).style(StepsStyle::Numbered));
    ui.add_space(10.0);

    ui.label(theme.muted_text("Numbered — complete"));
    ui.add(Steps::new(4).current(4).style(StepsStyle::Numbered));
    ui.add_space(10.0);

    ui.label(theme.muted_text("Numbered — errored"));
    ui.add(
        Steps::new(5)
            .current(2)
            .errored(true)
            .style(StepsStyle::Numbered),
    );
    ui.add_space(14.0);

    ui.label(theme.muted_text("Numbered — labeled with active sublabel"));
    ui.add(
        Steps::labeled(["Account", "Workspace", "Billing", "Integrations", "Review"])
            .style(StepsStyle::Numbered)
            .current(2)
            .active_sublabel("In progress"),
    );
    ui.add_space(10.0);

    ui.label(theme.muted_text("Numbered — labeled, no sublabel"));
    ui.add(
        Steps::labeled(["Details", "Payment", "Confirm"])
            .style(StepsStyle::Numbered)
            .current(0),
    );
    ui.add_space(14.0);

    ui.label(theme.muted_text("Labeled — horizontal"));
    ui.add(Steps::labeled(["Plan", "Build", "Test", "Deploy"]).current(2));
    ui.add_space(6.0);
    ui.add(
        Steps::labeled(["Schema", "Backfill", "Reindex", "Finalize"])
            .current(2)
            .errored(true),
    );
    ui.add_space(14.0);

    ui.label(theme.muted_text("Labeled — vertical"));
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width(200.0);
            ui.add(
                Steps::labeled(["Plan", "Design", "Build", "Test", "Deploy"])
                    .current(2)
                    .vertical(),
            );
        });
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.set_max_width(220.0);
            ui.add(
                Steps::labeled(["Schema validated", "Backfill", "Index rebuild", "Finalize"])
                    .current(2)
                    .errored(true)
                    .vertical(),
            );
        });
    });
}

fn containers_ui(ui: &mut egui::Ui) {
    let mut open = true;
    let theme = Theme::current(ui.ctx());
    ui.set_min_width(440.0);

    ui.label(theme.muted_text("Card"));
    Card::new().heading("Account").show(ui, |ui| {
        ui.label(theme.body_text("Primary email: steve@example.com"));
        ui.label(theme.muted_text("Two-factor authentication enabled."));
    });
    ui.add_space(8.0);

    ui.label(theme.muted_text("CollapsingSection"));
    CollapsingSection::new("advanced", "Advanced")
        .open(&mut open)
        .show(ui, |ui| {
            ui.label(theme.body_text("Hidden until expanded. Nest anything here."));
        });
}

fn accordion_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_width(620.0);

    ui.label(theme.muted_text("FAQ — bordered, default"));
    Accordion::new("vis_acc_faq").show(ui, |acc| {
        acc.item("How do I invite teammates to my workspace?")
            .default_open(true)
            .show(|ui| {
                ui.label(theme.muted_text(
                    "Open Settings ▸ Members and click Invite. Paste a list of emails or share a role-scoped signup link.",
                ));
            });
        acc.item("What happens when I archive a project?")
            .show(|_| {});
        acc.item("Is there an API for bulk imports?").show(|_| {});
    });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Settings — exclusive, with icon and meta"));
    Accordion::new("vis_acc_settings")
        .exclusive(true)
        .show(ui, |acc| {
            acc.item("Notifications")
                .icon("\u{1F514}")
                .accent(Accent::Sky)
                .subtitle("Email, Slack, and in-app alerts")
                .meta(|ui| {
                    ui.add(Badge::new("3 channels", BadgeTone::Ok));
                })
                .default_open(true)
                .show(|ui| {
                    ui.label(
                        theme
                            .muted_text("Three channels enabled. Tap Manage channels for details."),
                    );
                });
            acc.item("Security")
                .icon("\u{1F512}")
                .accent(Accent::Green)
                .subtitle("2FA, sessions, and trusted devices")
                .meta(|ui| {
                    ui.add(Badge::new("Strong", BadgeTone::Ok));
                })
                .show(|_| {});
            acc.item("Integrations")
                .icon("\u{2731}")
                .accent(Accent::Amber)
                .subtitle("GitHub, Linear, PagerDuty, and 2 more")
                .meta(|ui| {
                    ui.add(Badge::new("1 needs auth", BadgeTone::Warning));
                })
                .show(|_| {});
            acc.item("Billing (owner-only)")
                .icon("\u{1F3E0}")
                .subtitle("Invoices, plan, and seats")
                .meta(|ui| {
                    ui.label(theme.faint_text("Admin required"));
                })
                .disabled(true)
                .show(|_| {});
        });
    ui.add_space(14.0);

    ui.label(theme.muted_text("Flush — inline, no outer card"));
    Accordion::new("vis_acc_flush").flush(true).show(ui, |acc| {
        acc.item("Advanced options")
            .subtitle("(rarely needed)")
            .default_open(true)
            .show(|ui| {
                ui.label(
                    theme.muted_text("Override the default request timeout and retry behavior."),
                );
            });
        acc.item("Experimental features").show(|_| {});
        acc.item("Danger zone").show(|_| {});
    });
}

fn sortable_list_ui(ui: &mut egui::Ui) {
    let mut items = vec![
        SortableItem::new("api", "api-east-01")
            .subtitle("10.0.1.5 · us-east-1")
            .icon("◔")
            .status("live", BadgeTone::Ok),
        SortableItem::new("worker", "worker-pool-a")
            .subtitle("24 instances · autoscale")
            .icon("◑")
            .status("idle", BadgeTone::Neutral),
        SortableItem::new("edge", "edge-proxy-01")
            .subtitle("8 instances · global")
            .icon("◎")
            .status("degraded", BadgeTone::Warning),
        SortableItem::new("etl", "warehouse-etl")
            .subtitle("nightly batch · 02:00 UTC")
            .icon("○")
            .status("offline", BadgeTone::Danger),
        SortableItem::new("logs", "log-ingestor")
            .subtitle("12 instances · kafka")
            .icon("◕")
            .status("live", BadgeTone::Ok),
    ];
    Card::new().heading("Deployment targets").show(ui, |ui| {
        ui.set_min_width(480.0);
        SortableList::new("snap_sortable_list", &mut items).show(ui);
    });
}

fn log_bar_ui(ui: &mut egui::Ui) {
    let mut log = LogBar::new();
    log.sys("Ready");
    log.out("probe_status");
    log.recv("{\"temp\":42.1,\"ok\":true}");
    log.err("retry budget exceeded");
    ui.set_min_width(700.0);
    log.show(ui);
}

fn callouts_ui(ui: &mut egui::Ui) {
    let mut dismiss_open = true;
    ui.set_min_width(640.0);

    Callout::new(CalloutTone::Info)
        .title("Node editing is in preview.")
        .body("The wire format may change before 1.0.")
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Warning)
        .title("Unsaved changes.")
        .body("You have 3 edits that haven't been written to disk.")
        .show(ui, |ui| {
            ui.add(Button::new("Save now").accent(Accent::Amber));
            ui.add(Button::new("Discard").outline());
        });
    ui.add_space(6.0);

    Callout::new(CalloutTone::Danger)
        .title("Build failed.")
        .body("cargo returned 2 errors in src/node_editor.rs.")
        .dismissable(&mut dismiss_open)
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Success)
        .title("Deploy complete.")
        .body("Rolled out to us-east-1.")
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Neutral)
        .title("Read-only mode.")
        .body("Database upgrade in progress.")
        .show(ui, |_| {});
}

fn callouts_tinted_ui(ui: &mut egui::Ui) {
    ui.set_min_width(640.0);

    Callout::new(CalloutTone::Info)
        .tinted()
        .title("Node editing is in preview.")
        .body("The wire format may change before 1.0.")
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Warning)
        .tinted()
        .title("Unsaved changes.")
        .body("You have 3 edits that haven't been written to disk.")
        .show(ui, |ui| {
            ui.add(Button::new("Save now").accent(Accent::Amber));
            ui.add(Button::new("Discard").outline());
        });
    ui.add_space(6.0);

    Callout::new(CalloutTone::Danger)
        .tinted()
        .title("Build failed.")
        .body("cargo returned 2 errors in src/node_editor.rs.")
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Success)
        .tinted()
        .title("Deploy complete.")
        .body("Rolled out to us-east-1.")
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Neutral)
        .tinted()
        .title("Read-only mode.")
        .body("Database upgrade in progress.")
        .show(ui, |_| {});
}

fn callouts_multiline_ui(ui: &mut egui::Ui) {
    let mut dismiss_open = true;
    ui.set_min_width(560.0);

    Callout::new(CalloutTone::Info)
        .multiline()
        .title("Node editing is in preview.")
        .body(
            "The wire format may change before 1.0. Saved graphs will keep \
             loading, but rendered ports and inline expressions may shift \
             positions when the layout engine is updated.",
        )
        .show(ui, |_| {});
    ui.add_space(6.0);

    Callout::new(CalloutTone::Warning)
        .multiline()
        .title("Unsaved changes.")
        .body(
            "You have 3 edits that haven't been written to disk. Closing \
             the window or opening a different project will discard them.",
        )
        .show(ui, |ui| {
            ui.add(Button::new("Save now").accent(Accent::Amber));
            ui.add(Button::new("Discard").outline());
        });
    ui.add_space(6.0);

    Callout::new(CalloutTone::Danger)
        .tinted()
        .multiline()
        .title("Build failed.")
        .body(
            "cargo returned 2 errors in src/node_editor.rs. The first \
             unresolved import is on line 42; the rest cascade from there.",
        )
        .dismissable(&mut dismiss_open)
        .show(ui, |_| {});
}

fn file_drop_zone_ui(ui: &mut egui::Ui) {
    ui.set_max_width(560.0);
    let _ = FileDropZone::new()
        .hint("up to 10 MB \u{00b7} PNG, JPG, CSV, PDF")
        .show(ui);
    ui.add_space(10.0);
    let _ = FileDropZone::new()
        .prompt("Drop a CSV to import")
        .min_height(96.0)
        .enabled(false)
        .show(ui);
}

fn pairing_ui(ui: &mut egui::Ui) {
    let clients = vec![
        // c1 / c3 exercise the tinted-icon path (tone-resolved), c2 / c4 the
        // untinted default.
        PairItem::new("c1", "worker-pool-a")
            .detail("24 instances")
            .icon(glyphs::CIRCLE_CHECK)
            .icon_tone(BadgeTone::Ok),
        PairItem::new("c2", "edge-proxy-01")
            .detail("8 instances")
            .icon("▸"),
        PairItem::new("c3", "cache-layer")
            .detail("unreachable")
            .icon(glyphs::CIRCLE_X)
            .icon_tone(BadgeTone::Danger),
        PairItem::new("c4", "batch-workers")
            .detail("12 instances")
            .icon("▸"),
    ];
    let servers = vec![
        PairItem::new("s1", "api-east-01")
            .detail("10.0.1.5 · us-east")
            .icon("◂"),
        PairItem::new("s2", "api-east-02")
            .detail("10.0.1.6 · us-east")
            .icon("◂"),
        PairItem::new("s3", "api-west-01")
            .detail("10.0.2.4 · us-west")
            .icon("◂"),
        // Same glyph as its neighbours, tinted with an explicit colour: the
        // `Solid` path bypasses the palette, so this stays the same violet in
        // all four themes while the tone-resolved icons above shift.
        PairItem::new("s4", "api-eu-01")
            .detail("10.0.3.2 · eu-west")
            .icon("◂")
            .icon_color(egui::Color32::from_rgb(0xc0, 0x84, 0xfc)),
    ];
    let mut pairs = vec![
        ("c1".to_string(), "s3".to_string()),
        ("c2".to_string(), "s1".to_string()),
        ("c4".to_string(), "s2".to_string()),
    ];
    ui.set_max_width(680.0);
    Pairing::new("vis_pairing", &clients, &servers, &mut pairs)
        .left_label("Clients")
        .right_label("Servers")
        .show(ui);
}

fn color_picker_triggers_ui(ui: &mut egui::Ui) {
    let mut a = egui::Color32::from_rgb(0x38, 0xbd, 0xf8);
    let mut b = egui::Color32::from_rgb(0x4a, 0xde, 0x80);
    let mut c = egui::Color32::from_rgba_unmultiplied(0xc0, 0x84, 0xfc, 0xa6);
    ui.set_min_width(360.0);
    ui.horizontal(|ui| {
        ui.add(ColorPicker::new("cp_a", &mut a).label("Brand"));
        ui.add_space(12.0);
        ui.add(ColorPicker::new("cp_b", &mut b).label("Success"));
        ui.add_space(12.0);
        ui.add(
            ColorPicker::new("cp_c", &mut c)
                .label("Overlay")
                .alpha(true),
        );
    });
}

fn color_picker_palette_open_ui(ui: &mut egui::Ui) {
    let id = "cp_open_palette";
    let mut color = egui::Color32::from_rgb(0x38, 0xbd, 0xf8);
    ui.set_min_size(egui::vec2(360.0, 540.0));
    ui.add_space(8.0);
    let trigger = ui.add(
        ColorPicker::new(id, &mut color)
            .palette(ColorPicker::default_palette())
            .continuous(false)
            .recents(false),
    );
    open_color_picker(ui, &trigger);
}

fn color_picker_continuous_open_ui(ui: &mut egui::Ui) {
    let id = "cp_open_continuous";
    let mut color = egui::Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 0xd9);
    ui.set_min_size(egui::vec2(360.0, 380.0));
    ui.add_space(8.0);
    let trigger = ui.add(ColorPicker::new(id, &mut color).recents(false));
    open_color_picker(ui, &trigger);
}

/// Hold a color picker's popover open; it shows from the next frame.
fn open_color_picker(ui: &egui::Ui, trigger: &egui::Response) {
    egui::Popup::open_id(
        ui.ctx(),
        elegance::Popover::popup_id(trigger.id, "elegance::color_picker"),
    );
}

// egui only allows one popup open at a time per viewport, so each side
// gets its own snapshot test.

fn menu_bar_ui(ui: &mut egui::Ui) {
    // Closed-bar snapshot — the strip itself is what we care about pinning.
    // Live dropdowns paint into a top-level Area, which the kittest harness
    // doesn't compose into the same surface, so we don't open one here.
    let theme = Theme::current(ui.ctx());
    MenuBar::new("snap_menu_bar")
        .brand("Elegance")
        .status_with_dot("main \u{00b7} up to date", theme.palette.green)
        .show(ui, |bar| {
            bar.menu("File", |ui| {
                ui.add(MenuItem::new("New").shortcut("\u{2318}N"));
            });
            bar.menu("Edit", |ui| {
                ui.add(MenuItem::new("Undo").shortcut("\u{2318}Z"));
            });
            bar.menu("View", |_| {});
            bar.menu("Window", |_| {});
            bar.menu("Help", |_| {});
        });
}

fn context_menu_panel_ui(ui: &mut egui::Ui) {
    // The live ContextMenu popup paints into a top-level Area (anchored to
    // the right-click pointer position), which the kittest harness can't
    // place deterministically. Render a panel inline using the same Frame
    // styling so the visual treatment of MenuItem/MenuSection variants —
    // sections, separators, shortcuts, danger, disabled, checked, radio,
    // and the submenu arrow — is pinned across themes.
    let theme = Theme::current(ui.ctx());
    let p = &theme.palette;
    let r = theme.card_radius as u8;
    let frame = egui::Frame::new()
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(r))
        .inner_margin(egui::Margin::same(4));
    ui.set_min_width(260.0);
    frame.show(ui, |ui| {
        ui.set_min_width(232.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add(MenuItem::new("Open").shortcut("\u{21B5}"));
        ui.add(MenuItem::new("Open in new split").shortcut("\u{2318}\u{21E7}\u{21B5}"));
        ui.separator();
        ui.add(MenuSection::new("Edit"));
        ui.add(MenuItem::new("Copy").shortcut("\u{2318}C"));
        ui.add(MenuItem::new("Duplicate").shortcut("\u{2318}D"));
        ui.add(MenuItem::new("Rename\u{2026}").shortcut("F2"));
        ui.add(
            MenuItem::new("Move to workspace\u{2026}")
                .shortcut("read-only")
                .enabled(false),
        );
        ui.separator();
        ui.add(MenuSection::new("Selection"));
        ui.add(
            MenuItem::new("Highlight matches")
                .checked(true)
                .shortcut("\u{2318}\u{21E7}L"),
        );
        ui.add(MenuItem::new("Show whitespace").checked(false));
        ui.separator();
        ui.add(MenuSection::new("Font size"));
        ui.add(MenuItem::new("Small").radio(false));
        ui.add(MenuItem::new("Medium").radio(true));
        ui.add(MenuItem::new("Large").radio(false));
        ui.separator();
        ui.add(MenuItem::new("Delete").shortcut("\u{232B}").danger());
    });
}

fn popover_bottom_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_size(egui::vec2(320.0, 260.0));
    ui.add_space(40.0);
    ui.horizontal(|ui| {
        ui.add_space(60.0);
        let bottom = ui.add(Button::new("Delete branch").outline());
        egui::Popup::open_id(ui.ctx(), Popover::popup_id(bottom.id, "pop_bottom"));
        Popover::new("pop_bottom")
            .side(PopoverSide::Bottom)
            .title("Delete feature/snap-baseline?")
            .show(&bottom, |ui| {
                ui.add(egui::Label::new(
                    theme.muted_text("This removes the branch from origin too."),
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let _ = ui.add(Button::new("Cancel").outline().size(ButtonSize::Small));
                    let _ = ui.add(
                        Button::new("Delete")
                            .accent(Accent::Red)
                            .size(ButtonSize::Small),
                    );
                });
            });
    });
}

fn popover_top_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_size(egui::vec2(260.0, 180.0));
    ui.add_space(100.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        let top = ui.add(Button::new("What's a baseline?").outline());
        egui::Popup::open_id(ui.ctx(), Popover::popup_id(top.id, "pop_top"));
        Popover::new("pop_top")
            .side(PopoverSide::Top)
            .title("Baselines")
            .width(260.0)
            .show(&top, |ui| {
                ui.add(egui::Label::new(theme.muted_text(
                    "Accepted reference image a widget is compared against.",
                )));
            });
    });
}

fn popover_left_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_size(egui::vec2(420.0, 120.0));
    ui.add_space(40.0);
    ui.horizontal(|ui| {
        ui.add_space(260.0);
        let left = ui.add(Button::new("Details").outline());
        egui::Popup::open_id(ui.ctx(), Popover::popup_id(left.id, "pop_left"));
        Popover::new("pop_left")
            .side(PopoverSide::Left)
            .show(&left, |ui| {
                ui.add(egui::Label::new(theme.muted_text("Opens to the left.")));
            });
    });
}

fn popover_right_ui(ui: &mut egui::Ui) {
    let theme = Theme::current(ui.ctx());
    ui.set_min_size(egui::vec2(420.0, 120.0));
    ui.add_space(40.0);
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        let right = ui.add(Button::new("Details").outline());
        egui::Popup::open_id(ui.ctx(), Popover::popup_id(right.id, "pop_right"));
        Popover::new("pop_right")
            .side(PopoverSide::Right)
            .show(&right, |ui| {
                ui.add(egui::Label::new(theme.muted_text("Opens to the right.")));
            });
    });
}

// Tooltips piggy-back on egui's hover-driven tooltip system; force them
// open via memory().set_everything_is_visible(true) so the snapshot
// captures the rendered card without needing a pointer-hover fixture.
fn tooltip_label_ui(ui: &mut egui::Ui) {
    ui.ctx().memory_mut(|m| m.set_everything_is_visible(true));
    ui.set_min_size(egui::vec2(280.0, 120.0));
    ui.add_space(80.0);
    ui.horizontal(|ui| {
        ui.add_space(80.0);
        let trigger = ui.add(Button::new("Share").outline().size(ButtonSize::Small));
        Tooltip::new("Copy share link").show(&trigger);
    });
}

fn tooltip_rich_ui(ui: &mut egui::Ui) {
    ui.ctx().memory_mut(|m| m.set_everything_is_visible(true));
    ui.set_min_size(egui::vec2(360.0, 180.0));
    ui.add_space(120.0);
    ui.horizontal(|ui| {
        ui.add_space(80.0);
        let trigger = ui.add(Button::new("Save").outline());
        Tooltip::new("Write the working tree to disk. Remote sync runs in the background.")
            .heading("Save changes")
            .shortcut("\u{2318} S")
            .show(&trigger);
    });
}

fn tooltip_below_ui(ui: &mut egui::Ui) {
    ui.ctx().memory_mut(|m| m.set_everything_is_visible(true));
    ui.set_min_size(egui::vec2(360.0, 180.0));
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(80.0);
        let trigger = ui.add(
            Button::new("degraded")
                .accent(Accent::Amber)
                .size(ButtonSize::Small),
        );
        Tooltip::new("api-west-01 is returning elevated 5xx. Other regions healthy.")
            .heading("Partial outage")
            .side(TooltipSide::Bottom)
            .show(&trigger);
    });
}

theme_tests!(buttons, buttons_ui);
theme_tests!(icon_buttons, icon_buttons_ui);
theme_tests!(icon_text_buttons, icon_text_buttons_ui);
theme_tests!(text_inputs, text_inputs_ui);
theme_tests!(tag_inputs, tag_inputs_ui);
theme_tests!(removable_chips, removable_chips_ui);
theme_tests!(text_areas, text_areas_ui);
theme_tests!(selects, selects_ui);
theme_tests!(toggles, toggles_ui);
theme_tests!(tabs, tabs_ui);
theme_tests!(segmented_control, segmented_control_ui);
theme_tests!(browser_tabs, browser_tabs_ui);
theme_tests!(dock, dock_ui);
theme_tests!(dock_overflow, dock_overflow_ui);
theme_tests!(status, status_ui);
theme_tests!(stat_cards, stat_cards_ui);
theme_tests!(avatars, avatars_ui);
theme_tests!(sliders, sliders_ui);
theme_tests!(sliders_line, sliders_line_ui);
theme_tests!(percent_sliders, percent_sliders_ui);
theme_tests!(percent_sliders_line, percent_sliders_line_ui);
theme_tests!(metric_sliders, metric_sliders_ui);
theme_tests!(metric_sliders_line, metric_sliders_line_ui);
theme_tests!(range_sliders, range_sliders_ui);
theme_tests!(range_sliders_line, range_sliders_line_ui);
theme_tests!(knobs, knobs_ui);
theme_tests!(feedback, feedback_ui);
theme_tests!(progress_rings, progress_rings_ui);
theme_tests!(gauge, gauge_ui);
theme_tests!(steps, steps_ui);
theme_tests!(containers, containers_ui);
theme_tests!(accordion, accordion_ui);
theme_tests!(callouts, callouts_ui);
theme_tests!(callouts_tinted, callouts_tinted_ui);
theme_tests!(callouts_multiline, callouts_multiline_ui);
theme_tests!(file_drop_zone, file_drop_zone_ui);
theme_tests!(log_bar, log_bar_ui);
theme_tests!(sortable_list, sortable_list_ui);
theme_tests!(pairing, pairing_ui);
theme_tests!(popover_bottom, popover_bottom_ui);
theme_tests!(popover_top, popover_top_ui);
theme_tests!(popover_left, popover_left_ui);
theme_tests!(popover_right, popover_right_ui);
theme_tests!(menu_bar, menu_bar_ui);
theme_tests!(context_menu_panel, context_menu_panel_ui);
theme_tests!(tooltip_label, tooltip_label_ui);
theme_tests!(tooltip_rich, tooltip_rich_ui);
theme_tests!(tooltip_below, tooltip_below_ui);
theme_tests!(color_picker_triggers, color_picker_triggers_ui);
theme_tests!(color_picker_palette_open, color_picker_palette_open_ui);
theme_tests!(
    color_picker_continuous_open,
    color_picker_continuous_open_ui
);

// ---------------------------------------------------------------------------
// Interaction-state tests. Each renders a single widget, injects a mouse /
// keyboard event after the initial layout, and snapshots the resulting
// hover / focus / pressed visual.
// ---------------------------------------------------------------------------

fn single_button_ui(ui: &mut egui::Ui) {
    ui.add(Button::new("Deploy").accent(Accent::Green));
}

fn single_switch_off_ui(ui: &mut egui::Ui) {
    let mut off = false;
    ui.add(Switch::new(&mut off, "Notify"));
}

fn single_text_input_ui(ui: &mut egui::Ui) {
    let mut email = "steve@example.com".to_string();
    ui.add(
        TextInput::new(&mut email)
            .label("Email")
            .desired_width(240.0)
            .id_salt("focus_email"),
    );
}

fn dirty_text_input_ui(ui: &mut egui::Ui) {
    let mut val = "3000.0".to_string();
    ui.add(
        TextInput::new(&mut val)
            .label("Dirty")
            .dirty(true)
            .desired_width(240.0)
            .id_salt("dirty_focus"),
    );
}

fn revealable_text_input_ui(ui: &mut egui::Ui) {
    let mut secret = "correct-horse".to_string();
    ui.add(
        TextInput::new(&mut secret)
            .label("Passphrase")
            .revealable(true)
            .desired_width(240.0)
            .id_salt("reveal_focus"),
    );
}

fn hover_deploy(h: &Harness) {
    h.get_by_label("Deploy").hover();
}

fn focus_sv_plane(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::Slider, "Saturation and value")
        .focus();
}

fn focus_hue_strip(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::Slider, "Hue")
        .focus();
}

fn focus_alpha_strip(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::Slider, "Alpha")
        .focus();
}

fn focus_deploy(h: &Harness) {
    h.get_by_label("Deploy").focus();
}

fn hover_notify(h: &Harness) {
    h.get_by_label("Notify").hover();
}

fn focus_email(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Email")
        .focus();
}

fn focus_dirty(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Dirty")
        .focus();
}

fn focus_reveal(h: &Harness) {
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Passphrase")
        .focus();
}

macro_rules! interact_tests {
    ($name:ident, $ui_fn:expr, $setup:expr) => {
        mod $name {
            use super::*;
            #[test]
            fn slate() {
                snap_with_setup(
                    &format!("{}_slate", stringify!($name)),
                    Theme::slate(),
                    $ui_fn,
                    $setup,
                );
            }
            #[test]
            fn charcoal() {
                snap_with_setup(
                    &format!("{}_charcoal", stringify!($name)),
                    Theme::charcoal(),
                    $ui_fn,
                    $setup,
                );
            }
            #[test]
            fn frost() {
                snap_with_setup(
                    &format!("{}_frost", stringify!($name)),
                    Theme::frost(),
                    $ui_fn,
                    $setup,
                );
            }
            #[test]
            fn paper() {
                snap_with_setup(
                    &format!("{}_paper", stringify!($name)),
                    Theme::paper(),
                    $ui_fn,
                    $setup,
                );
            }
        }
    };
}

interact_tests!(button_hovered, single_button_ui, hover_deploy);
interact_tests!(button_focused, single_button_ui, focus_deploy);
interact_tests!(switch_hovered_off, single_switch_off_ui, hover_notify);
interact_tests!(text_input_focused, single_text_input_ui, focus_email);
interact_tests!(text_input_dirty_focused, dirty_text_input_ui, focus_dirty);
interact_tests!(
    color_picker_sv_plane_focused,
    color_picker_continuous_open_ui,
    focus_sv_plane
);
interact_tests!(
    color_picker_hue_strip_focused,
    color_picker_continuous_open_ui,
    focus_hue_strip
);
// The alpha strip's ring is the one drawn over a checkerboard rather than a
// flat fill, so it is the one most worth a baseline.
interact_tests!(
    color_picker_alpha_strip_focused,
    color_picker_continuous_open_ui,
    focus_alpha_strip
);
interact_tests!(
    revealable_input_focused,
    revealable_text_input_ui,
    focus_reveal
);

// ---------------------------------------------------------------------------
// Rounded-corner regression tests (issue #7). A footer / row fill must follow
// the surface's rounded corners rather than square them off. The modal is an
// overlay (fixed canvas); the accordion focus ring is exercised on the last,
// closed row, whose bottom edge abuts the card's rounded bottom corners.
// ---------------------------------------------------------------------------

fn modal_with_footer_ui(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let theme = Theme::current(&ctx);
    // The modal is never dismissed in the test, so fresh locals each frame are
    // fine — `open` stays true and the field text/checkbox stay constant.
    let mut open = true;
    let mut text = String::from("elegance-");
    let mut export = false;
    Modal::new("vis_modal", &mut open)
        .heading("Delete workspace")
        .subtitle("This will remove all 38 projects, 214 dashboards, and 7 team members.")
        .header_icon("!")
        .header_accent(Accent::Red)
        .max_width(480.0)
        .alert(true)
        .footer_left(|ui| {
            ui.add(Checkbox::new(&mut export, "Export data before deletion"));
        })
        .footer(|ui| {
            let _ = ui.add(
                Button::new("Delete workspace")
                    .accent(Accent::Red)
                    .enabled(false),
            );
            let _ = ui.add(Button::new("Cancel").outline());
        })
        .show(&ctx, |ui| {
            let caption = ui
                .horizontal_wrapped(|ui| {
                    [
                        ui.add(egui::Label::new(theme.muted_text("Type"))).id,
                        ui.add(egui::Label::new(
                            egui::RichText::new("elegance-labs")
                                .monospace()
                                .color(theme.palette.red),
                        ))
                        .id,
                        ui.add(egui::Label::new(theme.muted_text("to confirm."))).id,
                    ]
                })
                .inner;
            ui.add_space(6.0);
            caption.into_iter().fold(
                ui.add(
                    TextInput::new(&mut text)
                        .desired_width(f32::INFINITY)
                        .id_salt("vis_modal_phrase"),
                ),
                |field, label| field.labelled_by(label),
            );
        });
}

fn drawer_ui(ui: &mut egui::Ui) {
    // Something behind the panel, so the backdrop's dimming and the panel's
    // inner-edge divider both have content to read against.
    ui.heading("Incidents");
    ui.add(egui::Label::new(
        Theme::current(ui.ctx()).muted_text("3 open, 1 acknowledged"),
    ));

    let ctx = ui.ctx().clone();
    // Never dismissed in the test, so fresh locals each frame are fine.
    let mut open = true;
    let mut owner = String::from("api-west-02");
    Drawer::new("vis_drawer", &mut open)
        .side(DrawerSide::Right)
        .width(360.0)
        .title("INC-2187")
        .subtitle("api-west-02 latency spike")
        .show(&ctx, |ui| {
            let caption = ui.add(egui::Label::new(
                Theme::current(ui.ctx()).muted_text("Owner"),
            ));
            ui.add_space(4.0);
            ui.add(
                TextInput::new(&mut owner)
                    .desired_width(f32::INFINITY)
                    .id_salt("vis_drawer_owner"),
            )
            .labelled_by(caption.id);
            ui.add_space(12.0);
            let _ = ui.add(Button::new("Acknowledge").accent(Accent::Blue));
        });
}

fn single_accordion_ui(ui: &mut egui::Ui) {
    ui.set_min_width(420.0);
    Accordion::new("vis_acc_focus").show(ui, |acc| {
        acc.item("Invite teammates")
            .subtitle("Members and roles")
            .show(|_| {});
        acc.item("Archive a project").show(|_| {});
        acc.item("Delete account").show(|_| {});
    });
}

fn focus_last_accordion_row(h: &Harness) {
    h.get_by_label("Delete account").focus();
}

macro_rules! fixed_theme_tests {
    ($name:ident, $size:expr, $ui_fn:expr) => {
        mod $name {
            use super::*;
            #[test]
            fn slate() {
                snap_fixed(
                    &format!("{}_slate", stringify!($name)),
                    Theme::slate(),
                    $size,
                    $ui_fn,
                );
            }
            #[test]
            fn charcoal() {
                snap_fixed(
                    &format!("{}_charcoal", stringify!($name)),
                    Theme::charcoal(),
                    $size,
                    $ui_fn,
                );
            }
            #[test]
            fn frost() {
                snap_fixed(
                    &format!("{}_frost", stringify!($name)),
                    Theme::frost(),
                    $size,
                    $ui_fn,
                );
            }
            #[test]
            fn paper() {
                snap_fixed(
                    &format!("{}_paper", stringify!($name)),
                    Theme::paper(),
                    $size,
                    $ui_fn,
                );
            }
        }
    };
}

fixed_theme_tests!(
    modal_footer,
    egui::Vec2::new(640.0, 440.0),
    modal_with_footer_ui
);
fixed_theme_tests!(drawer, egui::Vec2::new(760.0, 440.0), drawer_ui);
interact_tests!(
    accordion_row_focused,
    single_accordion_ui,
    focus_last_accordion_row
);

/// A snapshot freezes whatever it is handed, so it pins this only as long as
/// nobody regenerates it without looking. Assert the property directly: the
/// glyph's ink must sit in the middle of the button.
///
/// Box-centring a lone glyph misses by ~2.75 pt, since the row box carries the
/// font's whole ascent and descent and an icon's advance width carries uneven
/// side bearings, which reads as an icon shoved up and to the left.
#[test]
fn icon_button_centres_its_glyph() {
    const PPP: f32 = 2.0;
    let theme = Theme::slate();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(200.0, 120.0))
        .with_pixels_per_point(PPP)
        .wgpu()
        .build_ui(move |ui| {
            theme.clone().install(ui.ctx());
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, theme.palette.bg);
            ui.add(
                Button::icon(glyphs::DOWNLOAD, "Download")
                    .accent(Accent::Blue)
                    .size(ButtonSize::Large),
            );
        });
    harness.run();
    harness.run();
    let button = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Download")
        .rect();
    let image = harness.render().expect("render");

    // The white glyph is the only near-white ink: slate's bg (0f,17,2a) and its
    // blue fill (25,63,eb) both fail a >200-on-every-channel test, as does any
    // antialiased blend of the two.
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for (x, y, px) in image.enumerate_pixels() {
        if px[0] > 200 && px[1] > 200 && px[2] > 200 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    assert!(x1 > x0 && y1 > y0, "found no glyph ink in the button");

    // An inclusive pixel span x0..=x1 covers points [x0/ppp, (x1+1)/ppp).
    let ink = egui::pos2(
        (x0 + x1 + 1) as f32 / (2.0 * PPP),
        (y0 + y1 + 1) as f32 / (2.0 * PPP),
    );
    let off = ink - button.center();
    // Half a point of slack for rounding; box-centring was off by 2.75.
    assert!(
        off.length() <= 0.5,
        "glyph ink off centre by {off:?} pt — button {button:?}, ink centre {ink:?}"
    );
}

mod dock_drag {
    use super::*;

    fn left_of(r: egui::Rect) -> egui::Pos2 {
        egui::pos2(r.left() + 24.0, r.center().y + 60.0)
    }

    fn centre_of(r: egui::Rect) -> egui::Pos2 {
        egui::pos2(r.center().x + 120.0, r.center().y + 60.0)
    }

    #[test]
    fn edge_slate() {
        snap_dock_drag("dock_drag_edge_slate", Theme::slate(), "Preview", left_of);
    }

    #[test]
    fn group_slate() {
        snap_dock_drag("dock_drag_group_slate", Theme::slate(), "Files", centre_of);
    }
}
