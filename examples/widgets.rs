//! Widget reference — every elegance widget laid out in labeled tiles,
//! intended for screenshotting for the README and crates.io listing.
//!
//! Run with `cargo widgets`.

#![allow(clippy::collapsible_if)]

use eframe::egui;
use elegance::{
    Accent, Accordion, Avatar, AvatarGroup, AvatarPresence, AvatarSize, AvatarTone, Badge,
    BadgeTone, BrowserTab, BrowserTabs, BrowserTabsEvent, BuiltInTheme, Button, ButtonSize,
    Callout, CalloutTone, Card, Checkbox, CollapsingSection, ColorPicker, ContextMenu, Dock,
    DockEvent, DockLayout, DockNode, DockTab, DockViewer, Drawer, DrawerSide, FileDropZone,
    GaugeZones, Indicator, IndicatorState, Knob, KnobSize, LinearGauge, LogBar, Menu, MenuBar,
    MenuItem, MenuSection, MetricSlider, Modal, PairItem, Pairing, PercentSlider, Popover,
    PopoverSide, ProgressBar, ProgressRing, RadialGauge, RangeSlider, RemovableChip, Segment,
    SegmentDot, SegmentedButton, SegmentedControl, SegmentedSize, Select, Slider, SliderHandle,
    SortableItem, SortableList, Spinner, StatCard, StatusPill, Steps, StepsStyle, SubMenuItem,
    Switch, TabBar, TagInput, TextArea, TextInput, Theme, ThemeSwitcher, Toast, Toasts, Tooltip,
    TooltipSide, glyphs,
};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Elegance — widget reference")
            .with_inner_size([980.0, 1400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Elegance widgets",
        options,
        Box::new(|_cc| Ok(Box::<App>::default())),
    )
}

#[derive(Debug)]
struct App {
    theme: BuiltInTheme,

    text_normal: String,
    text_hint: String,
    text_dirty: String,
    text_pw: String,
    text_reveal: String,
    area_body: String,
    area_mono: String,
    tag_recipients: Vec<String>,
    tag_skills: Vec<String>,
    chip_suffix: Option<String>,
    chip_filter: Option<String>,
    select_unit: String,
    select_env: String,
    select_region: String,
    select_region_saved: String,

    check_on: bool,
    check_off: bool,
    switch_on: bool,
    switch_off: bool,
    switch_green: bool,
    seg_on: bool,
    seg_off: bool,
    seg_join_a: bool,
    seg_join_b: bool,
    seg_join_c: bool,
    seg_glow: bool,
    seg_dim: bool,

    category: usize,
    tab_idx: usize,
    collapsing_open: bool,

    slider_int: u32,
    slider_float: f32,
    percent_cache: f32,
    percent_retention: f32,
    percent_compact: f32,
    percent_tier: f32,
    metric_buffer: f32,
    metric_latency: f32,
    metric_refresh: f32,
    metric_plan: f32,
    color_brand: egui::Color32,
    color_overlay: egui::Color32,
    color_status: egui::Color32,
    range_price_lo: u32,
    range_price_hi: u32,
    range_latency_lo: u32,
    range_latency_hi: u32,
    range_volume_lo: u32,
    range_volume_hi: u32,

    knob_gain: f32,
    knob_cutoff: f32,
    knob_q: f32,
    knob_mix: u32,
    knob_timebase: u32,
    knob_dc_offset: f32,

    show_modal: bool,
    show_modal_verify: bool,
    modal_verify_text: String,
    modal_verify_export: bool,
    show_modal_form: bool,
    modal_form_name: String,
    modal_form_desc: String,
    modal_form_project: String,
    modal_form_open_after: bool,
    show_modal_info: bool,
    show_modal_plain: bool,
    show_drawer_detail: bool,
    show_drawer_form: bool,
    drawer_form_name: String,
    drawer_form_email: String,
    drawer_form_role: String,
    drawer_form_notes: String,

    pop_filter_open: bool,
    pop_filter_in_review: bool,
    pop_filter_merged: bool,
    pop_filter_closed: bool,
    pop_filter_needs_review: bool,
    pop_filter_ready: bool,
    pop_filter_blocked: bool,

    callout_danger_open: bool,

    mb_show_sidebar: bool,
    mb_show_minimap: bool,
    mb_show_status: bool,
    mb_density: usize, // 0 = compact, 1 = comfortable, 2 = spacious
    mb_theme: usize,   // 0 = light, 1 = dark, 2 = system
    mb_schedule_on: bool,
    mb_last_action: String,

    pairing_clients: Vec<PairItem>,
    pairing_servers: Vec<PairItem>,
    pairing_pairs: Vec<(String, String)>,
    pairing_align: bool,

    sortable_targets: Vec<SortableItem>,

    browser_tabs: BrowserTabs,
    browser_tabs_untitled: u32,

    dock: DockLayout<DemoTab>,
    dock_untitled: u32,
    dock_log: Vec<String>,

    seg_ctrl_size: usize,
    seg_ctrl_density: usize,
    seg_ctrl_lang: usize,
    seg_ctrl_filter: usize,
    seg_ctrl_channels: [bool; 8],

    stat_deploys: StatTick,
    stat_error: StatTick,
    stat_p95: StatTick,
    stat_revenue: StatTick,
    stat_last_tick: std::time::Instant,
    stat_rng: u64,

    log: LogBar,
}

impl Default for App {
    fn default() -> Self {
        let mut log = LogBar::new();
        log.sys("Ready");
        log.out("probe_status");
        log.recv("{\"temp\":42.1,\"ok\":true}");
        log.err("retry budget exceeded");
        // Long entries to stress-test wrapping / horizontal overflow.
        log.sys(
            "Reconnecting to broker mqtts://gateway.iot.example.com:8883 \
             over TLS 1.3 — credentials accepted, resubscribing to 14 topics, \
             replaying 3 retained messages from the offline queue.",
        );
        log.out(
            "POST https://api.example.com/v2/devices/sensor-7f3a2c1e/telemetry \
             ?since=2026-04-23T18%3A04%3A12Z&fields=pressure,humidity,temp_c,vbat",
        );
        log.recv(
            "{\"id\":\"sensor-7f3a2c1e\",\"ts\":1745434752,\"readings\":{\"pressure\":1013.6,\
             \"humidity\":42.7,\"temp_c\":21.3,\"vbat\":3.842},\"firmware\":\"v2.18.4-rc3\",\
             \"site\":\"warehouse-east-3\",\"flags\":[\"calibrated\",\"online\"]}",
        );
        log.err(
            "panicked at src/pipeline/aggregator.rs:142:21: \
             assertion `left == right` failed (left: 8192, right: 4096) \
             while flushing buffered samples; backtrace truncated, see core.42137 for details",
        );
        log.sys(
            "/Users/thomas/Library/Application Support/elegance-demo/cache/snapshots/\
             2026-04-23T18-04-12Z-warehouse-east-3-sensor-7f3a2c1e.snapshot.json",
        );
        Self {
            theme: BuiltInTheme::default(),
            text_normal: "steve@example.com".into(),
            text_hint: String::new(),
            text_dirty: "3000.0".into(),
            text_pw: "hunter2".into(),
            text_reveal: "correct-horse-battery".into(),
            area_body: "Short note.\nA second line.".into(),
            area_mono: "{\n  \"id\": 42,\n  \"ok\": true\n}".into(),
            tag_recipients: vec!["thomas@example.com".into(), "team@orbit.dev".into()],
            tag_skills: vec!["rust".into(), "egui".into(), "wgpu".into()],
            chip_suffix: Some("run-1".into()),
            chip_filter: None,
            select_unit: "ms".into(),
            select_env: "Production".into(),
            select_region: "eu-west-1".into(),
            select_region_saved: "eu-west-1".into(),
            check_on: true,
            check_off: false,
            switch_on: true,
            switch_off: false,
            switch_green: true,
            seg_on: true,
            seg_off: false,
            seg_join_a: true,
            seg_join_b: true,
            seg_join_c: false,
            seg_glow: true,
            seg_dim: true,
            category: 0,
            tab_idx: 1,
            collapsing_open: true,
            slider_int: 48,
            slider_float: 0.62,
            percent_cache: 45.0,
            percent_retention: 30.0,
            percent_compact: 70.0,
            percent_tier: 25.0,
            metric_buffer: 16.0,
            metric_latency: 175.0,
            metric_refresh: 120.0,
            metric_plan: 2.0,
            color_brand: egui::Color32::from_rgb(0x38, 0xbd, 0xf8),
            color_overlay: egui::Color32::from_rgba_unmultiplied(0xc0, 0x84, 0xfc, 0xa6),
            color_status: egui::Color32::from_rgb(0xf8, 0x71, 0x71),
            range_price_lo: 24,
            range_price_hi: 118,
            range_latency_lo: 120,
            range_latency_hi: 340,
            range_volume_lo: 18,
            range_volume_hi: 62,
            knob_gain: -12.0,
            knob_cutoff: 1000.0,
            knob_q: 2.4,
            knob_mix: 35,
            knob_timebase: 3,
            knob_dc_offset: -1.4,
            show_modal: false,
            show_modal_verify: false,
            modal_verify_text: "elegance-".into(),
            modal_verify_export: false,
            show_modal_form: false,
            modal_form_name: "Request volume by region".into(),
            modal_form_desc: String::new(),
            modal_form_project: "elegance-charts".into(),
            modal_form_open_after: true,
            show_modal_info: false,
            show_modal_plain: false,
            show_drawer_detail: false,
            show_drawer_form: false,
            drawer_form_name: "Avery Lin".into(),
            drawer_form_email: "avery.lin@elegance.dev".into(),
            drawer_form_role: "Admin".into(),
            drawer_form_notes: "On-call rotation lead, Q2. Prefers Slack over email for paging."
                .into(),
            pop_filter_open: true,
            pop_filter_in_review: true,
            pop_filter_merged: false,
            pop_filter_closed: false,
            pop_filter_needs_review: true,
            pop_filter_ready: false,
            pop_filter_blocked: false,
            callout_danger_open: true,
            mb_show_sidebar: true,
            mb_show_minimap: false,
            mb_show_status: true,
            mb_density: 1,
            mb_theme: 1,
            mb_schedule_on: true,
            mb_last_action: String::new(),
            pairing_clients: vec![
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
            ],
            pairing_servers: vec![
                PairItem::new("s1", "api-east-01")
                    .detail("10.0.1.5 · us-east")
                    .icon("◂"),
                PairItem::new("s2", "api-east-02")
                    .detail("10.0.1.6 · us-east")
                    .icon("◂"),
                PairItem::new("s3", "api-west-01")
                    .detail("10.0.2.4 · us-west")
                    .icon("◂"),
                PairItem::new("s4", "api-eu-01")
                    .detail("10.0.3.2 · eu-west")
                    .icon("◂"),
            ],
            pairing_pairs: vec![
                ("c1".into(), "s3".into()),
                ("c2".into(), "s1".into()),
                ("c4".into(), "s2".into()),
            ],
            pairing_align: false,
            sortable_targets: vec![
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
            ],
            browser_tabs: BrowserTabs::new("ref_browser_tabs")
                .with_tab(BrowserTab::new("readme", "README.md"))
                .with_tab(BrowserTab::new("theme", "theme.rs").dirty(true))
                .with_tab(BrowserTab::new("button", "widgets/button.rs"))
                .with_tab(BrowserTab::new(
                    "cargo",
                    "cargo output \u{2014} a longer title",
                )),
            browser_tabs_untitled: 0,
            dock: demo_dock_layout(),
            dock_untitled: 0,
            dock_log: Vec::new(),
            seg_ctrl_size: 1,
            seg_ctrl_density: 1,
            seg_ctrl_lang: 0,
            seg_ctrl_filter: 0,
            seg_ctrl_channels: [true, false, true, true, false, false, true, false],
            stat_deploys: StatTick::new(
                &[
                    12.0, 14.0, 13.0, 15.0, 17.0, 16.0, 18.0, 20.0, 19.0, 22.0, 21.0, 22.0, 22.0,
                    24.0, 26.0, 24.0, 27.0, 28.0, 30.0, 28.0, 26.0, 24.0,
                ],
                0.12,
            ),
            stat_error: StatTick::new(
                &[
                    0.8, 0.9, 0.82, 0.75, 0.7, 0.62, 0.6, 0.58, 0.55, 0.5, 0.48, 0.46, 0.44, 0.45,
                    0.4, 0.42, 0.4, 0.38, 0.42, 0.4, 0.41, 0.42,
                ],
                -0.08,
            ),
            stat_p95: StatTick::new(
                &[
                    120.0, 118.0, 122.0, 125.0, 128.0, 130.0, 135.0, 140.0, 142.0, 148.0, 150.0,
                    155.0, 160.0, 162.0, 168.0, 170.0, 175.0, 178.0, 182.0, 180.0, 184.0, 186.0,
                ],
                0.24,
            ),
            stat_revenue: StatTick::new(
                &[
                    8.2, 8.5, 8.8, 9.2, 9.5, 9.8, 10.1, 10.4, 10.7, 11.0, 11.2, 11.4, 11.6, 11.8,
                    12.0, 12.1, 12.2, 12.3, 12.35, 12.38, 12.4, 12.4,
                ],
                0.034,
            ),
            stat_last_tick: std::time::Instant::now(),
            stat_rng: 0x9E37_79B9_7F4A_7C15,
            log,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The `ThemeSwitcher` in the header installs the selected theme
        // each frame, so no explicit `Theme::install` is needed here.
        let bg = Theme::current(ui.ctx()).palette.bg;

        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                let theme = Theme::current(ui.ctx());
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(
                        egui::RichText::new("Elegance — Widget reference")
                            .color(theme.palette.text)
                            .size(18.0)
                            .strong(),
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(ThemeSwitcher::new(&mut self.theme));
                    });
                });
            });

        self.log.show(ui);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            ui.add(TabBar::new(
                &mut self.category,
                [
                    "Buttons", "Inputs", "Numeric", "Display", "Status", "Feedback", "Layout",
                    "Overlays", "Tools",
                ],
            ));
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 10.0);
                    match self.category {
                        0 => {
                            self.section_buttons(ui);
                            self.section_toggles(ui);
                        }
                        1 => {
                            self.section_text(ui);
                            self.section_tag_input(ui);
                            self.section_removable_chip(ui);
                            self.section_selects(ui);
                            self.section_color_picker(ui);
                            self.section_file_drop_zone(ui);
                        }
                        2 => {
                            self.section_sliders(ui);
                            self.section_percent_sliders(ui);
                            self.section_metric_sliders(ui);
                            self.section_knobs(ui);
                        }
                        3 => {
                            self.section_tabs(ui);
                            self.section_segmented_control(ui);
                            self.section_browser_tabs(ui);
                            self.section_stat_cards(ui);
                        }
                        4 => {
                            self.section_status(ui);
                            self.section_avatar(ui);
                        }
                        5 => {
                            self.section_gauge(ui);
                            self.section_callouts(ui);
                            self.section_feedback(ui);
                        }
                        6 => {
                            self.section_containers(ui);
                            self.section_accordion(ui);
                            self.section_menu_bar(ui);
                            self.section_dock(ui);
                        }
                        7 => {
                            self.section_modal(ui);
                            self.section_drawer(ui);
                            self.section_menu(ui);
                            self.section_context_menu(ui);
                            self.section_toast(ui);
                            self.section_popover(ui);
                            self.section_tooltip(ui);
                        }
                        _ => {
                            self.section_pairing(ui);
                            self.section_sortable_list(ui);
                        }
                    }
                    ui.add_space(12.0);
                });
        });

        self.modal_demo(ui.ctx());
        self.modal_demo_verify(ui.ctx());
        self.modal_demo_form(ui.ctx());
        self.modal_demo_info(ui.ctx());
        self.modal_demo_plain(ui.ctx());
        self.drawer_demos(ui.ctx());
        Toasts::new().render(ui.ctx());
    }
}

impl App {
    fn section_buttons(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Buttons").show(ui, |ui| {
            labeled(ui, "Accents", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (lbl, a) in [
                        ("Blue", Accent::Blue),
                        ("Green", Accent::Green),
                        ("Red", Accent::Red),
                        ("Purple", Accent::Purple),
                        ("Amber", Accent::Amber),
                        ("Sky", Accent::Sky),
                    ] {
                        let _ = ui.add(Button::new(lbl).accent(a));
                    }
                    let _ = ui.add(Button::new("Outline").outline());
                });
            });

            labeled(ui, "Sizes", |ui| {
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
                });
            });

            labeled(ui, "States", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(Button::new("Default").accent(Accent::Blue));
                    ui.add(Button::new("Disabled").accent(Accent::Blue).enabled(false));
                    ui.add(Button::new("Loading").accent(Accent::Blue).loading(true));
                    ui.add(
                        Button::new("Loading + disabled")
                            .accent(Accent::Blue)
                            .loading(true)
                            .enabled(false),
                    );
                    ui.add(Button::new("Loading outline").outline().loading(true));
                });
            });

            labeled(ui, "Icon only", |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Beside a text button, to show the two share a height.
                    ui.add(
                        Button::icon(glyphs::DOWNLOAD, "Download")
                            .size(ButtonSize::Large)
                            .accent(Accent::Blue),
                    );
                    ui.add(Button::new("Large").size(ButtonSize::Large));
                    ui.add(Button::icon(glyphs::UPLOAD, "Upload").accent(Accent::Green));
                    ui.add(Button::icon(glyphs::TRASH, "Delete").accent(Accent::Red));
                    ui.add(
                        Button::icon(glyphs::PENCIL, "Edit")
                            .size(ButtonSize::Small)
                            .accent(Accent::Purple),
                    );
                    ui.add(Button::icon(glyphs::COPY, "Copy").outline());
                    ui.add(Button::icon(glyphs::POWER, "Power").enabled(false));
                    ui.add(Button::icon(glyphs::REFRESH, "Refresh").loading(true));
                });
            });

            labeled(ui, "Icon + text", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        Button::icon_text(glyphs::DOWNLOAD, "Download")
                            .size(ButtonSize::Large)
                            .accent(Accent::Blue),
                    );
                    ui.add(Button::icon_text(glyphs::UPLOAD, "Upload").accent(Accent::Green));
                    ui.add(Button::icon_text(glyphs::TRASH, "Delete").accent(Accent::Red));
                    ui.add(
                        Button::icon_text(glyphs::PENCIL, "Edit")
                            .size(ButtonSize::Small)
                            .accent(Accent::Purple),
                    );
                    ui.add(Button::icon_text(glyphs::COPY, "Copy").outline());
                    ui.add(Button::icon_text(glyphs::POWER, "Power").enabled(false));
                    ui.add(Button::icon_text(glyphs::REFRESH, "Refresh").loading(true));
                });
            });
        });
    }

    fn section_text(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Text inputs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextInput::new(&mut self.text_normal)
                        .label("Email")
                        .desired_width(220.0)
                        .id_salt("ref_t_normal"),
                );
                ui.add_space(12.0);
                ui.add(
                    TextInput::new(&mut self.text_hint)
                        .label("With hint")
                        .hint("e.g. sensor-01")
                        .desired_width(220.0)
                        .id_salt("ref_t_hint"),
                );
            });
            ui.horizontal(|ui| {
                ui.add(
                    TextInput::new(&mut self.text_dirty)
                        .label("Dirty (unsaved)")
                        .dirty(true)
                        .desired_width(220.0)
                        .id_salt("ref_t_dirty"),
                );
                ui.add_space(12.0);
                ui.add(
                    TextInput::new(&mut self.text_pw)
                        .label("Password")
                        .password(true)
                        .desired_width(220.0)
                        .id_salt("ref_t_pw"),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    TextInput::new(&mut self.text_reveal)
                        .label("Passphrase (revealable)")
                        .revealable(true)
                        .desired_width(220.0)
                        .id_salt("ref_t_reveal"),
                );
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.add(
                    TextArea::new(&mut self.area_body)
                        .label("Text area")
                        .rows(3)
                        .desired_width(220.0)
                        .id_salt("ref_a_body"),
                );
                ui.add_space(12.0);
                ui.add(
                    TextArea::new(&mut self.area_mono)
                        .label("Monospace")
                        .monospace(true)
                        .rows(3)
                        .desired_width(220.0)
                        .id_salt("ref_a_mono"),
                );
            });
        });
    }

    fn section_tag_input(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Tag input").show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.allocate_ui(egui::vec2(380.0, 0.0), |ui| {
                    TagInput::new("ref_ti_recipients", &mut self.tag_recipients)
                        .label("Recipients")
                        .placeholder("Add an email…")
                        .commit_on_space(true)
                        .validator(|v| {
                            if v.contains('@') && v.contains('.') {
                                Ok(())
                            } else {
                                Err(format!("\"{v}\" isn't a valid email."))
                            }
                        })
                        .show(ui);
                });
                ui.add_space(16.0);
                ui.allocate_ui(egui::vec2(380.0, 0.0), |ui| {
                    TagInput::new("ref_ti_skills", &mut self.tag_skills)
                        .label("Skills")
                        .placeholder("Add a skill…")
                        .accent(Accent::Purple)
                        .show(ui);
                });
            });
        });
    }

    fn section_removable_chip(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Removable chip").show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "An inline editable chip with an × close button. Click + to add; \
                     × or Escape on an empty editor signals removal.",
                )
                .color(Theme::current(ui.ctx()).palette.text_muted),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Path suffix:");
                ui.add_space(4.0);
                if let Some(value) = self.chip_suffix.as_mut() {
                    let resp = RemovableChip::new(value)
                        .prefix("_")
                        .placeholder("run-1")
                        .accent(Accent::Green)
                        .id_salt("ref_chip_suffix")
                        .show(ui);
                    if resp.removed {
                        self.chip_suffix = None;
                    }
                } else if ui
                    .add(Button::new("+ suffix").size(ButtonSize::Small).outline())
                    .clicked()
                {
                    self.chip_suffix = Some(String::new());
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.add_space(4.0);
                if let Some(value) = self.chip_filter.as_mut() {
                    let resp = RemovableChip::new(value)
                        .placeholder("contains…")
                        .accent(Accent::Sky)
                        .id_salt("ref_chip_filter")
                        .show(ui);
                    if resp.removed {
                        self.chip_filter = None;
                    }
                } else if ui
                    .add(Button::new("+ filter").size(ButtonSize::Small).outline())
                    .clicked()
                {
                    self.chip_filter = Some(String::new());
                }
            });
        });
    }

    fn section_selects(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Select").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    Select::strings("ref_sel_unit", &mut self.select_unit, ["us", "ms", "s"])
                        .label("Unit")
                        .width(120.0),
                );
                ui.add_space(16.0);
                ui.add(
                    Select::strings(
                        "ref_sel_env",
                        &mut self.select_env,
                        ["Production", "Staging", "Development"],
                    )
                    .label("Environment")
                    .width(180.0),
                );
            });

            // A staged picker: the dot appears the moment the selection
            // diverges from what was last saved, and clears on commit.
            ui.add_space(12.0);
            let staged = self.select_region != self.select_region_saved;
            ui.add(
                Select::strings(
                    "ref_sel_region",
                    &mut self.select_region,
                    ["eu-west-1", "us-east-1", "ap-south-1"],
                )
                .label("Region")
                .saved(&self.select_region_saved)
                .width(140.0),
            );
            ui.add_space(8.0);
            if ui
                .add(Button::new("Save").size(ButtonSize::Small).enabled(staged))
                .clicked()
            {
                self.select_region_saved = self.select_region.clone();
            }
        });
    }

    fn section_toggles(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Toggles").show(ui, |ui| {
            labeled(ui, "Checkbox", |ui| {
                ui.horizontal(|ui| {
                    ui.add(Checkbox::new(&mut self.check_on, "Enabled"));
                    ui.add_space(16.0);
                    ui.add(Checkbox::new(&mut self.check_off, "Off"));
                });
            });

            labeled(ui, "Switch", |ui| {
                ui.horizontal(|ui| {
                    ui.add(Switch::new(&mut self.switch_on, "On"));
                    ui.add_space(16.0);
                    ui.add(Switch::new(&mut self.switch_off, "Off"));
                    ui.add_space(16.0);
                    ui.add(
                        Switch::new(&mut self.switch_green, "Green accent").accent(Accent::Green),
                    );
                });
            });

            labeled(ui, "Segmented", |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        SegmentedButton::new(&mut self.seg_on, "Record")
                            .accent(Accent::Green)
                            .min_width(130.0),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        SegmentedButton::new(&mut self.seg_off, "Preview")
                            .accent(Accent::Blue)
                            .min_width(130.0),
                    );
                });
            });

            labeled(
                ui,
                "Mixed action row — Button and SegmentedButton share control height at any size",
                |ui| {
                    ui.horizontal(|ui| {
                        let _ =
                            ui.add(Button::new("Collect").accent(Accent::Green).min_width(96.0));
                        ui.add_space(8.0);
                        ui.add(
                            SegmentedButton::new(&mut self.seg_on, "Streaming")
                                .accent(Accent::Green)
                                .min_width(140.0),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let _ = ui.add(
                            Button::new("Collect")
                                .accent(Accent::Green)
                                .size(ButtonSize::Large)
                                .min_width(120.0),
                        );
                        ui.add_space(8.0);
                        ui.add(
                            SegmentedButton::new(&mut self.seg_on, "Streaming")
                                .accent(Accent::Green)
                                .size(ButtonSize::Large)
                                .min_width(180.0),
                        );
                    });
                },
            );

            labeled(ui, "Joined (shared rounding)", |ui| {
                let radius = 8;
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.horizontal(|ui| {
                        ui.add(
                            SegmentedButton::new(&mut self.seg_join_a, "Lint")
                                .accent(Accent::Green)
                                .corner_radius(egui::CornerRadius {
                                    nw: radius,
                                    sw: radius,
                                    ne: 0,
                                    se: 0,
                                })
                                .min_width(110.0),
                        );
                        ui.add(
                            SegmentedButton::new(&mut self.seg_join_b, "Test")
                                .accent(Accent::Blue)
                                .corner_radius(egui::CornerRadius::ZERO)
                                .min_width(110.0),
                        );
                        ui.add(
                            SegmentedButton::new(&mut self.seg_join_c, "Deploy")
                                .accent(Accent::Purple)
                                .corner_radius(egui::CornerRadius {
                                    nw: 0,
                                    sw: 0,
                                    ne: radius,
                                    se: radius,
                                })
                                .min_width(110.0),
                        );
                    });
                });
            });

            labeled(ui, "Dim when on", |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        SegmentedButton::new(&mut self.seg_glow, "Glow (default)")
                            .accent(Accent::Green)
                            .min_width(160.0),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        SegmentedButton::new(&mut self.seg_dim, "Subdued")
                            .accent(Accent::Green)
                            .dim_when_on(true)
                            .min_width(160.0),
                    );
                });
            });
        });
    }

    fn section_tabs(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Tabs").show(ui, |ui| {
            ui.add(TabBar::new(
                &mut self.tab_idx,
                ["Overview", "Settings", "Activity", "Logs"],
            ));
        });
    }

    fn section_segmented_control(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Segmented control").show(ui, |ui| {
            labeled(ui, "Sizes", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        SegmentedControl::new(&mut self.seg_ctrl_size, ["Day", "Week", "Month"])
                            .size(SegmentedSize::Small),
                    );
                    ui.add_space(12.0);
                    ui.add(SegmentedControl::new(
                        &mut self.seg_ctrl_density,
                        ["Compact", "Comfortable", "Spacious"],
                    ));
                    ui.add_space(12.0);
                    ui.add(
                        SegmentedControl::new(
                            &mut self.seg_ctrl_size,
                            ["Private", "Internal", "Public"],
                        )
                        .size(SegmentedSize::Large),
                    );
                });
            });

            labeled(ui, "Disabled segment", |ui| {
                ui.add(SegmentedControl::from_segments(
                    &mut self.seg_ctrl_lang,
                    [
                        Segment::text("EN"),
                        Segment::text("DE"),
                        Segment::text("JA"),
                        Segment::text("FR").enabled(false),
                    ],
                ));
            });

            labeled(ui, "Per-segment tooltips (hover each)", |ui| {
                ui.add(SegmentedControl::from_segments(
                    &mut self.seg_ctrl_density,
                    [
                        Segment::text("DEV").hover_text("Development: ephemeral, safe to break."),
                        Segment::text("STG").hover_text("Staging: mirrors production data shape."),
                        Segment::text("PROD")
                            .hover_text("Production: real users; deploy with care."),
                    ],
                ));
            });

            labeled(ui, "Filter row with status dots and counts (fill)", |ui| {
                ui.add(
                    SegmentedControl::from_segments(
                        &mut self.seg_ctrl_filter,
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
            });

            labeled(
                ui,
                "Multi-toggle with accent (click any segment, drag across to paint)",
                |ui| {
                    ui.add(
                        SegmentedControl::toggles(
                            &mut self.seg_ctrl_channels,
                            ["1", "2", "3", "4", "5", "6", "7", "8"],
                        )
                        .accent(Accent::Green),
                    );
                },
            );
        });
    }

    fn section_browser_tabs(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Browser tabs").show(ui, |ui| {
            self.browser_tabs.show(ui);
            for ev in self.browser_tabs.take_events() {
                if let BrowserTabsEvent::NewRequested = ev {
                    self.browser_tabs_untitled += 1;
                    let n = self.browser_tabs_untitled;
                    let id = format!("untitled-{n}");
                    let label = format!("Untitled-{n}");
                    self.browser_tabs.add_tab(BrowserTab::new(id, label));
                }
            }
        });
    }

    fn section_dock(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Dock").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(Button::new("New tab").size(ButtonSize::Small))
                    .clicked()
                {
                    self.dock_untitled += 1;
                    self.dock.add_tab(DemoTab::Untitled(self.dock_untitled));
                }
                if ui
                    .add(
                        Button::new("Reset layout")
                            .accent(Accent::Sky)
                            .size(ButtonSize::Small),
                    )
                    .clicked()
                {
                    self.dock = demo_dock_layout();
                }
            });
            ui.add_space(6.0);
            let size = egui::vec2(ui.available_width(), 360.0);
            ui.allocate_ui(size, |ui| {
                let out = Dock::new("ref_dock").show(ui, &mut self.dock, &mut DemoDockViewer);
                for ev in out.events {
                    match ev {
                        DockEvent::TabClosed(tab) => {
                            self.dock_log.push(format!("closed {}", tab.label()));
                        }
                        DockEvent::TabMoved(tab) => {
                            self.dock_log.push(format!("moved {}", tab.label()));
                        }
                        DockEvent::TabActivated(_) | DockEvent::LayoutChanged => {}
                    }
                }
            });
            let theme = Theme::current(ui.ctx());
            ui.add_space(6.0);
            let last = self.dock_log.last().map_or("no events yet", String::as_str);
            ui.label(theme.faint_text(format!("Last event: {last}")));
        });
    }

    fn section_status(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Status").show(ui, |ui| {
            labeled(ui, "StatusPill", |ui| {
                ui.add(
                    StatusPill::new()
                        .item("UI", IndicatorState::On)
                        .item("API", IndicatorState::Connecting)
                        .item("DB", IndicatorState::Off),
                );
            });

            labeled(ui, "Indicators", |ui| {
                ui.horizontal(|ui| {
                    for (s, caption) in [
                        (IndicatorState::On, "On"),
                        (IndicatorState::Connecting, "Connecting"),
                        (IndicatorState::Off, "Off"),
                    ] {
                        ui.add(Indicator::new(s));
                        ui.add(egui::Label::new(
                            Theme::current(ui.ctx()).faint_text(caption),
                        ));
                        ui.add_space(12.0);
                    }
                });
            });

            labeled(ui, "Badges", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(Badge::new("OK", BadgeTone::Ok));
                    ui.add(Badge::new("Warning", BadgeTone::Warning));
                    ui.add(Badge::new("Error", BadgeTone::Danger));
                    ui.add(Badge::new("Info", BadgeTone::Info));
                    ui.add(Badge::new("Neutral", BadgeTone::Neutral));
                });
            });
        });
    }

    fn section_avatar(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Avatar").show(ui, |ui| {
            labeled(ui, "Sizes", |ui| {
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
            });

            labeled(ui, "Auto tone from initials", |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    for initials in ["AL", "MR", "JK", "DP", "NV", "??"] {
                        ui.add(Avatar::new(initials));
                    }
                });
            });

            labeled(ui, "Presence", |ui| {
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
            });

            labeled(ui, "Stacked groups", |ui| {
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
            });
        });
    }

    fn section_stat_cards(&mut self, ui: &mut egui::Ui) {
        if self.stat_last_tick.elapsed().as_secs_f32() >= 2.0 {
            self.stat_deploys.advance(&mut self.stat_rng);
            self.stat_error.advance(&mut self.stat_rng);
            self.stat_p95.advance(&mut self.stat_rng);
            self.stat_revenue.advance(&mut self.stat_rng);
            self.stat_last_tick = std::time::Instant::now();
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));

        Card::new().heading("Stat cards").show(ui, |ui| {
            let cell_w = 230.0_f32;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                ui.add(
                    StatCard::new("Active deploys")
                        .accent(Accent::Blue)
                        .value(format!("{:.0}", self.stat_deploys.value))
                        .delta(self.stat_deploys.delta())
                        .trend("vs last 7 days")
                        .sparkline(&self.stat_deploys.series)
                        .width(cell_w),
                );
                ui.add(
                    StatCard::new("Error rate")
                        .accent(Accent::Purple)
                        .value(format!("{:.2}", self.stat_error.value))
                        .unit("%")
                        .delta(self.stat_error.delta())
                        .invert_delta(true)
                        .trend("vs last 24h")
                        .sparkline(&self.stat_error.series)
                        .info_tooltip("Sampled every 5 minutes.")
                        .width(cell_w),
                );
                ui.add(
                    StatCard::new("P95 latency")
                        .accent(Accent::Amber)
                        .value(format!("{:.0}", self.stat_p95.value))
                        .unit("ms")
                        .delta(self.stat_p95.delta())
                        .invert_delta(true)
                        .trend("regressed vs last hour")
                        .sparkline(&self.stat_p95.series)
                        .width(cell_w),
                );
                ui.add(
                    StatCard::new("Revenue today")
                        .accent(Accent::Green)
                        .value(format!("{:.1}", self.stat_revenue.value))
                        .unit("k")
                        .delta(self.stat_revenue.delta())
                        .trend("vs yesterday")
                        .sparkline(&self.stat_revenue.series)
                        .width(cell_w),
                );
            });
        });
    }

    fn section_callouts(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Callouts").show(ui, |ui| {
            labeled(ui, "Info", |ui| {
                Callout::new(CalloutTone::Info)
                    .title("Node editing is in preview.")
                    .body("The wire format may change before 1.0.")
                    .show(ui, |_| {});
            });

            labeled(ui, "Warning with actions", |ui| {
                Callout::new(CalloutTone::Warning)
                    .title("Unsaved changes.")
                    .body("You have 3 edits that haven't been written to disk.")
                    .show(ui, |ui| {
                        let _ = ui.add(Button::new("Save now").accent(Accent::Amber));
                        let _ = ui.add(Button::new("Discard").outline());
                    });
            });

            labeled(ui, "Danger, dismissable", |ui| {
                if self.callout_danger_open {
                    Callout::new(CalloutTone::Danger)
                        .title("Build failed.")
                        .body("cargo returned 2 errors in src/node_editor.rs.")
                        .dismissable(&mut self.callout_danger_open)
                        .show(ui, |_| {});
                } else if ui.add(Button::new("Restore banner").outline()).clicked() {
                    self.callout_danger_open = true;
                }
            });

            labeled(ui, "Success", |ui| {
                Callout::new(CalloutTone::Success)
                    .title("Deploy complete.")
                    .body("Rolled out to us-east-1.")
                    .show(ui, |_| {});
            });

            labeled(ui, "Neutral", |ui| {
                Callout::new(CalloutTone::Neutral)
                    .title("Read-only mode.")
                    .body("Database upgrade in progress.")
                    .show(ui, |_| {});
            });

            ui.add_space(6.0);
            ui.add(egui::Label::new(Theme::current(ui.ctx()).muted_text(
                "Tinted — severity-tinted background and border, no leading stripe.",
            )));
            ui.add_space(6.0);

            labeled(ui, "Tinted info", |ui| {
                Callout::new(CalloutTone::Info)
                    .tinted()
                    .title("Node editing is in preview.")
                    .body("The wire format may change before 1.0.")
                    .show(ui, |_| {});
            });

            labeled(ui, "Tinted warning with actions", |ui| {
                Callout::new(CalloutTone::Warning)
                    .tinted()
                    .title("Unsaved changes.")
                    .body("You have 3 edits that haven't been written to disk.")
                    .show(ui, |ui| {
                        let _ = ui.add(Button::new("Save now").accent(Accent::Amber));
                        let _ = ui.add(Button::new("Discard").outline());
                    });
            });

            labeled(ui, "Tinted danger", |ui| {
                Callout::new(CalloutTone::Danger)
                    .tinted()
                    .title("Build failed.")
                    .body("cargo returned 2 errors in src/node_editor.rs.")
                    .show(ui, |_| {});
            });

            labeled(ui, "Tinted success", |ui| {
                Callout::new(CalloutTone::Success)
                    .tinted()
                    .title("Deploy complete.")
                    .body("Rolled out to us-east-1.")
                    .show(ui, |_| {});
            });
        });
    }

    fn section_feedback(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Spinners & progress").show(ui, |ui| {
            labeled(ui, "Spinner sizes", |ui| {
                ui.horizontal(|ui| {
                    ui.add(Spinner::new().size(14.0));
                    ui.add_space(10.0);
                    ui.add(Spinner::new());
                    ui.add_space(10.0);
                    ui.add(Spinner::new().size(28.0));
                });
            });
            labeled(ui, "Spinner accents", |ui| {
                ui.horizontal(|ui| {
                    ui.add(Spinner::new().accent(Accent::Blue));
                    ui.add(Spinner::new().accent(Accent::Green));
                    ui.add(Spinner::new().accent(Accent::Amber));
                    ui.add(Spinner::new().accent(Accent::Red));
                    ui.add(Spinner::new().accent(Accent::Purple));
                });
            });
            labeled(ui, "Progress", |ui| {
                ui.add(ProgressBar::new(0.25));
                ui.add_space(4.0);
                ui.add(ProgressBar::new(0.6).accent(Accent::Green));
                ui.add_space(4.0);
                ui.add(ProgressBar::new(1.0).accent(Accent::Amber).text("Complete"));
            });
            // Drive every ProgressRing tile from a shared clock: 5 s of
            // linear fill, 1.5 s hold at 100%, then snap back to 0.
            let time = ui.ctx().input(|i| i.time) as f32;
            let cycle_len = 6.5_f32;
            let rise_len = 5.0_f32;
            let progress_at = |offset: f32| -> f32 {
                let t = (time + offset).rem_euclid(cycle_len);
                (t / rise_len).min(1.0)
            };
            let ring_fraction = progress_at(0.0);
            elegance::request_repaint_at_rate(ui.ctx(), 30.0);

            labeled(ui, "ProgressRing — progression", |ui| {
                // Five rings phase-offset through the cycle, so at any
                // instant the row reads as a left-to-right "progression".
                ui.horizontal(|ui| {
                    for i in 0..5 {
                        let offset = (i as f32 / 5.0) * cycle_len;
                        ui.add(ProgressRing::new(progress_at(offset)));
                        ui.add_space(8.0);
                    }
                });
            });
            labeled(ui, "ProgressRing — sizes & centre text", |ui| {
                ui.horizontal(|ui| {
                    ui.add(ProgressRing::new(ring_fraction).size(36.0));
                    ui.add_space(12.0);
                    ui.add(ProgressRing::new(ring_fraction).size(56.0));
                    ui.add_space(12.0);
                    let done = (ring_fraction * 20.0).round() as u32;
                    ui.add(
                        ProgressRing::new(ring_fraction)
                            .size(88.0)
                            .text(format!("{} / 20", done))
                            .caption("files"),
                    );
                    ui.add_space(12.0);
                    let remaining = (1.0 - ring_fraction) * 4.0;
                    ui.add(
                        ProgressRing::new(ring_fraction)
                            .size(72.0)
                            .accent(Accent::Amber)
                            .text(format!("{:.1}s", remaining))
                            .caption("remaining"),
                    );
                });
            });
            labeled(ui, "ProgressRing — accents", |ui| {
                ui.horizontal(|ui| {
                    for a in [
                        Accent::Sky,
                        Accent::Green,
                        Accent::Amber,
                        Accent::Red,
                        Accent::Purple,
                    ] {
                        ui.add(ProgressRing::new(ring_fraction).accent(a));
                        ui.add_space(8.0);
                    }
                });
            });
            labeled(ui, "Stepped — cells", |ui| {
                ui.add(Steps::new(6).current(4));
                ui.add_space(6.0);
                ui.add(Steps::new(5).current(2).errored(true));
            });
            labeled(ui, "Stepped — numbered", |ui| {
                ui.add(Steps::new(5).current(2).style(StepsStyle::Numbered));
            });
            labeled(ui, "Stepped — numbered with labels (setup wizard)", |ui| {
                ui.add(
                    Steps::labeled(["Account", "Workspace", "Billing", "Integrations", "Review"])
                        .style(StepsStyle::Numbered)
                        .current(2)
                        .active_sublabel("In progress"),
                );
                ui.add_space(18.0);
                ui.add(
                    Steps::labeled(["Details", "Payment", "Confirm"])
                        .style(StepsStyle::Numbered)
                        .current(0),
                );
            });
            labeled(ui, "Stepped — labeled (horizontal)", |ui| {
                ui.add(Steps::labeled(["Plan", "Build", "Test", "Deploy"]).current(2));
                ui.add_space(6.0);
                ui.add(
                    Steps::labeled(["Schema", "Backfill", "Reindex", "Finalize"])
                        .current(2)
                        .errored(true),
                );
            });
            labeled(ui, "Stepped — labeled (vertical)", |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width(200.0);
                        ui.add(
                            Steps::labeled(["Plan", "Design", "Build", "Test", "Deploy"])
                                .current(2)
                                .vertical(),
                        );
                    });
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(220.0);
                        ui.add(
                            Steps::labeled([
                                "Schema validated",
                                "Backfill complete",
                                "Index rebuild",
                                "Finalize",
                            ])
                            .current(2)
                            .errored(true)
                            .vertical(),
                        );
                    });
                });
            });
        });
    }

    fn section_gauge(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Gauges").show(ui, |ui| {
            let zones = GaugeZones::new(0.6, 0.85);
            labeled(ui, "Radial — half-circle with threshold zones", |ui| {
                ui.horizontal(|ui| {
                    ui.add(RadialGauge::new(0.42).zones(zones).size(180.0));
                    ui.add_space(20.0);
                    ui.add(RadialGauge::new(0.72).zones(zones).size(180.0));
                    ui.add_space(20.0);
                    ui.add(RadialGauge::new(0.94).zones(zones).size(180.0));
                });
            });
            labeled(ui, "Donut — ProgressRing in gauge mode", |ui| {
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
            });
            labeled(ui, "Linear — meter with threshold zones", |ui| {
                let theme = Theme::current(ui.ctx());
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
        });
    }

    fn section_file_drop_zone(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("FileDropZone").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Click-and-drop target. Reports dropped files via FileDropResponse.dropped_files.",
            )));
            ui.add_space(6.0);
            let drop = FileDropZone::new()
                .hint("up to 10 MB \u{00b7} PNG, JPG, CSV, PDF")
                .show(ui);
            if drop.response.clicked() {
                self.log.sys("FileDropZone clicked (open file picker here)");
            }
            for f in &drop.dropped_files {
                let path = f.path();
                let label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| path.display().to_string());
                self.log.recv(format!("dropped: {label}"));
            }
        });
    }

    fn section_sliders(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Slider").show(ui, |ui| {
            ui.add(
                Slider::new(&mut self.slider_int, 0u32..=100u32)
                    .label("Threshold")
                    .suffix("%"),
            );
            ui.add_space(6.0);
            ui.add(
                Slider::new(&mut self.slider_float, 0.0..=1.0)
                    .label("Gain")
                    .accent(Accent::Green),
            );
            ui.add_space(6.0);
            ui.add(
                Slider::new(&mut self.slider_float, 0.0..=1.0)
                    .label("Gain (line handle)")
                    .accent(Accent::Green)
                    .handle(SliderHandle::Line),
            );
        });

        Card::new().heading("RangeSlider").show(ui, |ui| {
            ui.add(
                RangeSlider::new(
                    &mut self.range_price_lo,
                    &mut self.range_price_hi,
                    0u32..=200u32,
                )
                .label("Price")
                .value_fmt(|v| format!("${v:.0}"))
                .id_salt("ex_range_price"),
            );
            ui.add_space(8.0);
            ui.add(
                RangeSlider::new(
                    &mut self.range_latency_lo,
                    &mut self.range_latency_hi,
                    0u32..=500u32,
                )
                .label("Latency target")
                .suffix(" ms")
                .step(10.0)
                .ticks(6)
                .show_tick_labels(true)
                .id_salt("ex_range_latency"),
            );
            ui.add_space(8.0);
            ui.add(
                RangeSlider::new(
                    &mut self.range_volume_lo,
                    &mut self.range_volume_hi,
                    0u32..=100u32,
                )
                .label("Volume")
                .suffix(" dB")
                .accent(Accent::Green)
                .id_salt("ex_range_volume"),
            );
            ui.add_space(8.0);
            ui.add(
                RangeSlider::new(
                    &mut self.range_volume_lo,
                    &mut self.range_volume_hi,
                    0u32..=100u32,
                )
                .label("Volume (line handles)")
                .suffix(" dB")
                .accent(Accent::Green)
                .handle(SliderHandle::Line)
                .id_salt("ex_range_volume_line"),
            );
        });
    }

    fn section_percent_sliders(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("PercentSlider").show(ui, |ui| {
            labeled(ui, "Default — duration callout while dragging", |ui| {
                ui.add(
                    PercentSlider::new(&mut self.percent_cache)
                        .label("Cache window")
                        .callout_fmt(|p| {
                            let mins = (p * 60.0 / 100.0).round() as i32;
                            format!("{mins} min")
                        })
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Snap to 5% steps, no callout", |ui| {
                ui.add(
                    PercentSlider::new(&mut self.percent_retention)
                        .label("Snapshot retention")
                        .step(5.0)
                        .accent(Accent::Green)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Compact — ticks hidden, GB total", |ui| {
                ui.add(
                    PercentSlider::new(&mut self.percent_compact)
                        .label("Disk share")
                        .show_ticks(false)
                        .accent(Accent::Purple)
                        // Callout opts in to a percent prefix; default behaviour
                        // is just whatever the closure returns.
                        .callout_fmt(|p| {
                            format!("{}% \u{00B7} {:.1} GB", p.round() as i32, p * 4.0 / 100.0)
                        })
                        .desired_width(260.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Discrete stops — non-uniform positions", |ui| {
                ui.add(
                    PercentSlider::new(&mut self.percent_tier)
                        .label("Bandwidth tier")
                        .stops([0.0, 10.0, 25.0, 75.0, 100.0])
                        .accent(Accent::Amber)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Line handle — thin vertical bar", |ui| {
                ui.add(
                    PercentSlider::new(&mut self.percent_cache)
                        .label("Cache window")
                        .handle(SliderHandle::Line)
                        .callout_fmt(|p| {
                            let mins = (p * 60.0 / 100.0).round() as i32;
                            format!("{mins} min")
                        })
                        .desired_width(380.0),
                );
            });
        });
    }

    fn section_metric_sliders(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("MetricSlider").show(ui, |ui| {
            labeled(ui, "Buffer size — discrete stops with suffix", |ui| {
                ui.add(
                    MetricSlider::new(&mut self.metric_buffer, 0.0..=32.0)
                        .label("Buffer size")
                        .suffix("GiB")
                        .stops([4.0, 8.0, 16.0, 32.0])
                        .accent(Accent::Amber)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Latency budget — step in user units", |ui| {
                ui.add(
                    MetricSlider::new(&mut self.metric_latency, 0.0..=500.0)
                        .label("Latency budget")
                        .suffix("ms")
                        .step(25.0)
                        .accent(Accent::Red)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Refresh rate — evenly-spaced steps", |ui| {
                ui.add(
                    MetricSlider::new(&mut self.metric_refresh, 30.0..=240.0)
                        .label("Refresh rate")
                        .suffix("Hz")
                        .steps(8)
                        .accent(Accent::Green)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Plan picker — headline_fmt replaces the value", |ui| {
                ui.add(
                    MetricSlider::new(&mut self.metric_plan, 0.0..=3.0)
                        .label("Plan")
                        .stops([0.0, 1.0, 2.0, 3.0])
                        .headline_fmt(|v| {
                            ["Free", "Lite", "Pro", "Studio"][v.round().clamp(0.0, 3.0) as usize]
                                .to_string()
                        })
                        .tick_fmt(|v| {
                            ["F", "L", "P", "S"][v.round().clamp(0.0, 3.0) as usize].to_string()
                        })
                        .accent(Accent::Purple)
                        .desired_width(380.0),
                );
            });
            ui.add_space(8.0);
            labeled(ui, "Line handle — thin vertical bar", |ui| {
                ui.add(
                    MetricSlider::new(&mut self.metric_refresh, 30.0..=240.0)
                        .label("Refresh rate")
                        .suffix("Hz")
                        .steps(8)
                        .accent(Accent::Green)
                        .handle(SliderHandle::Line)
                        .desired_width(380.0),
                );
            });
        });
    }

    fn section_knobs(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Knob").show(ui, |ui| {
            labeled(ui, "Instrument panel", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    ui.add(
                        Knob::new(&mut self.knob_gain, -60.0..=12.0)
                            .label("Gain")
                            .size(KnobSize::Small)
                            .default(0.0_f32)
                            .show_value(true)
                            .value_fmt(|v| format!("{v:.0} dB")),
                    );
                    ui.add(
                        Knob::new(&mut self.knob_cutoff, 20.0..=20000.0)
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
                        Knob::new(&mut self.knob_q, 0.1..=10.0)
                            .label("Q")
                            .size(KnobSize::Small)
                            .log_scale()
                            .default(0.707_f32)
                            .show_value(true),
                    );
                    ui.add(
                        Knob::new(&mut self.knob_mix, 0u32..=100u32)
                            .label("Mix")
                            .size(KnobSize::Small)
                            .default(50_u32)
                            .show_value(true)
                            .value_fmt(|v| format!("{v:.0}%"))
                            .accent(Accent::Green),
                    );
                });
            });

            labeled(ui, "Stepped detents", |ui| {
                ui.add(
                    Knob::new(&mut self.knob_timebase, 0u32..=8u32)
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
                .accessible_name("Timebase");
            });

            labeled(ui, "Bipolar (fill from zero)", |ui| {
                ui.add(
                    Knob::new(&mut self.knob_dc_offset, -5.0..=5.0)
                        .label("DC offset")
                        .bipolar()
                        .accent(Accent::Purple)
                        .default(0.0_f32)
                        .show_value(true)
                        .value_fmt(|v| format!("{v:+.2} V")),
                );
            });
        });
    }

    fn section_color_picker(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("ColorPicker").show(ui, |ui| {
            labeled(ui, "Spectrum (default) — HSV plane, alpha, hex", |ui| {
                ui.add(ColorPicker::new("ex_cp_brand", &mut self.color_brand).label("Brand"));
            });

            labeled(ui, "Curated palette + alpha + hex", |ui| {
                ui.add(
                    ColorPicker::new("ex_cp_overlay", &mut self.color_overlay)
                        .label("Overlay")
                        .palette(ColorPicker::default_palette())
                        .continuous(false),
                );
            });

            labeled(ui, "Compact, palette-only, no alpha", |ui| {
                ui.add(
                    ColorPicker::new("ex_cp_status", &mut self.color_status)
                        .label("Status")
                        .palette(ColorPicker::default_palette())
                        .continuous(false)
                        .alpha(false)
                        .hex_input(false)
                        .recents(false),
                );
            });
        });
    }

    fn section_containers(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Containers").show(ui, |ui| {
            labeled(ui, "Card (inner)", |ui| {
                let theme = Theme::current(ui.ctx());
                Card::new()
                    .padding(12.0)
                    .fill(theme.palette.input_bg)
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(theme.muted_text("An inner card.")));
                    });
            });

            labeled(ui, "CollapsingSection", |ui| {
                CollapsingSection::new("ref_collapsing", "Advanced options")
                    .open(&mut self.collapsing_open)
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(
                            Theme::current(ui.ctx())
                                .faint_text("…hidden until the header is clicked."),
                        ));
                    });
            });
        });
    }

    fn section_accordion(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Accordion").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Click a header (or focus + Space/Enter) to toggle. Use \u{2191}/\u{2193} to move between rows.",
            )));
            ui.add_space(8.0);

            labeled(ui, "FAQ — bordered", |ui| {
                Accordion::new("ref_acc_faq").show(ui, |acc| {
                    acc.item("How do I invite teammates to my workspace?")
                        .default_open(true)
                        .show(|ui| {
                            ui.add(egui::Label::new(theme.muted_text(
                                "Open Settings \u{25B8} Members and click Invite. You can paste a list of emails or share a role-scoped signup link. Invitations expire after 7 days.",
                            )));
                        });
                    acc.item("What happens when I archive a project?")
                        .show(|ui| {
                            ui.add(egui::Label::new(theme.muted_text(
                                "Archived projects are hidden from the sidebar and read-only. You can restore them within 90 days.",
                            )));
                        });
                    acc.item("Is there an API for bulk imports?").show(|ui| {
                        ui.add(egui::Label::new(theme.muted_text(
                            "Yes \u{2014} see the Imports API reference. Rate limits apply per workspace.",
                        )));
                    });
                });
            });

            labeled(ui, "Settings — exclusive, with icon halos", |ui| {
                Accordion::new("ref_acc_settings")
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
                                ui.add(egui::Label::new(theme.muted_text(
                                    "Three channels enabled. Tap Manage channels to edit them.",
                                )));
                            });
                        acc.item("Security")
                            .icon("\u{1F512}")
                            .accent(Accent::Green)
                            .subtitle("2FA, sessions, and trusted devices")
                            .meta(|ui| {
                                ui.add(Badge::new("Strong", BadgeTone::Ok));
                            })
                            .show(|ui| {
                                ui.add(egui::Label::new(theme.muted_text(
                                    "Two-factor authentication is enabled for all admins.",
                                )));
                            });
                        acc.item("Integrations")
                            .icon("\u{2731}")
                            .accent(Accent::Amber)
                            .subtitle("GitHub, Linear, PagerDuty, and 2 more")
                            .meta(|ui| {
                                ui.add(Badge::new("1 needs auth", BadgeTone::Warning));
                            })
                            .show(|ui| {
                                ui.add(egui::Label::new(theme.muted_text(
                                    "PagerDuty requires re-authorization. Other integrations are healthy.",
                                )));
                            });
                        acc.item("Billing (owner-only)")
                            .icon("\u{1F3E0}")
                            .subtitle("Invoices, plan, and seats")
                            .meta(|ui| {
                                ui.add(egui::Label::new(theme.faint_text("Admin required")));
                            })
                            .disabled(true)
                            .show(|_| {});
                    });
            });

            labeled(ui, "Flush — inline, no outer card", |ui| {
                Accordion::new("ref_acc_flush").flush(true).show(ui, |acc| {
                    acc.item("Advanced options")
                        .subtitle("(rarely needed)")
                        .default_open(true)
                        .show(|ui| {
                            ui.add(egui::Label::new(theme.muted_text(
                                "Override the default request timeout and retry behaviour.",
                            )));
                        });
                    acc.item("Experimental features").show(|ui| {
                        ui.add(egui::Label::new(theme.muted_text(
                            "Toggle features still in development.",
                        )));
                    });
                    acc.item("Danger zone").show(|ui| {
                        ui.add(egui::Label::new(theme.muted_text(
                            "Destructive actions. Cannot be undone.",
                        )));
                    });
                });
            });
        });
    }

    fn section_menu_bar(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Menu bar").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Click a trigger to open. Once any menu is open, hovering a sibling switches to it.",
            )));
            ui.add_space(8.0);

            // Simulate a small app shell so the menu bar reads as window
            // chrome rather than a free-floating control.
            Card::new()
                .padding(0.0)
                .fill(theme.palette.bg)
                .show(ui, |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        MenuBar::new("ref_menu_bar")
                            .brand("Elegance")
                            .status_with_dot("main \u{00b7} up to date", theme.palette.green)
                            .show(ui, |bar| {
                                self.menu_bar_file(bar);
                                self.menu_bar_view(bar);
                                self.menu_bar_actions(bar);
                                bar.menu("Help", |ui| {
                                    ui.add(MenuItem::new("Documentation").icon("\u{2139}"));
                                    ui.add(MenuItem::new("Keyboard shortcuts"));
                                    ui.separator();
                                    ui.add(MenuItem::new("About Elegance"));
                                });
                            });

                        // Body placeholder so the strip reads as chrome.
                        ui.allocate_space(egui::vec2(ui.available_width(), 12.0));
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.add(egui::Label::new(theme.muted_text(
                                if self.mb_last_action.is_empty() {
                                    "(menu actions will be reported here)".to_string()
                                } else {
                                    format!("Last menu action: {}", self.mb_last_action)
                                },
                            )));
                        });
                        ui.allocate_space(egui::vec2(ui.available_width(), 12.0));
                    });
                });
        });
    }

    fn menu_bar_file(&mut self, bar: &mut elegance::MenuBarUi<'_>) {
        bar.menu("File", |ui| {
            if ui
                .add(
                    MenuItem::new("New file")
                        .icon("\u{2795}")
                        .shortcut("\u{2318}N"),
                )
                .clicked()
            {
                self.mb_last_action = "New file".into();
            }
            if ui
                .add(
                    MenuItem::new("Open\u{2026}")
                        .icon("\u{1F4C2}")
                        .shortcut("\u{2318}O"),
                )
                .clicked()
            {
                self.mb_last_action = "Open\u{2026}".into();
            }

            // Submenu — flyout opens to the right when the row is hovered.
            let last_action_slot = &mut self.mb_last_action;
            SubMenuItem::new("Open Recent")
                .icon("\u{1F552}")
                .show(ui, |ui| {
                    for (label, when) in [
                        ("theme.rs", "5m ago"),
                        ("widgets/button.rs", "1h ago"),
                        ("README.md", "2d ago"),
                        ("tokens.json", "Apr 18"),
                    ] {
                        if ui.add(MenuItem::new(label).shortcut(when)).clicked() {
                            *last_action_slot = format!("Open recent \u{2192} {label}");
                        }
                    }
                    ui.separator();
                    if ui.add(MenuItem::new("Clear list")).clicked() {
                        *last_action_slot = "Clear recent files".into();
                    }
                });

            ui.separator();
            if ui
                .add(
                    MenuItem::new("Save")
                        .icon("\u{1F4BE}")
                        .shortcut("\u{2318}S"),
                )
                .clicked()
            {
                self.mb_last_action = "Save".into();
            }
            ui.add(
                MenuItem::new("Save as\u{2026}")
                    .icon("\u{1F4BE}")
                    .shortcut("\u{2318}\u{21E7}S"),
            );
            ui.add(
                MenuItem::new("Revert (no changes)")
                    .icon("\u{21BA}")
                    .enabled(false),
            );
            ui.separator();
            if ui
                .add(
                    MenuItem::new("Close window")
                        .icon("\u{2715}")
                        .shortcut("\u{2318}W"),
                )
                .clicked()
            {
                self.mb_last_action = "Close window".into();
            }
        });
    }

    fn menu_bar_view(&mut self, bar: &mut elegance::MenuBarUi<'_>) {
        bar.menu_keep_open("View", |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text("LAYOUT")));
            if ui
                .add(
                    MenuItem::new("Show sidebar")
                        .checked(self.mb_show_sidebar)
                        .shortcut("\u{2318}\\"),
                )
                .clicked()
            {
                self.mb_show_sidebar = !self.mb_show_sidebar;
                self.mb_last_action = format!(
                    "Show sidebar \u{2192} {}",
                    if self.mb_show_sidebar { "on" } else { "off" }
                );
            }
            if ui
                .add(
                    MenuItem::new("Show minimap")
                        .checked(self.mb_show_minimap)
                        .shortcut("\u{2318}\u{21E7}M"),
                )
                .clicked()
            {
                self.mb_show_minimap = !self.mb_show_minimap;
                self.mb_last_action = format!(
                    "Show minimap \u{2192} {}",
                    if self.mb_show_minimap { "on" } else { "off" }
                );
            }
            if ui
                .add(MenuItem::new("Show status bar").checked(self.mb_show_status))
                .clicked()
            {
                self.mb_show_status = !self.mb_show_status;
                self.mb_last_action = format!(
                    "Show status bar \u{2192} {}",
                    if self.mb_show_status { "on" } else { "off" }
                );
            }

            ui.separator();
            ui.add(egui::Label::new(theme.faint_text("DENSITY")));
            for (idx, label) in ["Compact", "Comfortable", "Spacious"].iter().enumerate() {
                if ui
                    .add(MenuItem::new(*label).radio(self.mb_density == idx))
                    .clicked()
                {
                    self.mb_density = idx;
                    self.mb_last_action = format!("Density \u{2192} {label}");
                }
            }

            ui.separator();
            ui.add(egui::Label::new(theme.faint_text("THEME")));
            for (idx, label) in ["Light", "Dark \u{00b7} Slate", "System"]
                .iter()
                .enumerate()
            {
                if ui
                    .add(MenuItem::new(*label).radio(self.mb_theme == idx))
                    .clicked()
                {
                    self.mb_theme = idx;
                    self.mb_last_action = format!("Theme \u{2192} {label}");
                }
            }

            // Zoom flyout. The View menu is sticky (`menu_keep_open`) so
            // child submenus inherit that close behaviour — handy here
            // since the user can fire several zoom actions in a row.
            ui.separator();
            let last_action_slot = &mut self.mb_last_action;
            SubMenuItem::new("Zoom").icon("\u{1F50D}").show(ui, |ui| {
                if ui
                    .add(MenuItem::new("Zoom in").shortcut("\u{2318}+"))
                    .clicked()
                {
                    *last_action_slot = "Zoom in".into();
                }
                if ui
                    .add(MenuItem::new("Zoom out").shortcut("\u{2318}\u{2212}"))
                    .clicked()
                {
                    *last_action_slot = "Zoom out".into();
                }
                if ui
                    .add(MenuItem::new("Reset zoom").shortcut("\u{2318}0"))
                    .clicked()
                {
                    *last_action_slot = "Reset zoom".into();
                }
                ui.separator();
                if ui.add(MenuItem::new("Fit to window")).clicked() {
                    *last_action_slot = "Fit to window".into();
                }
            });
        });
    }

    fn menu_bar_actions(&mut self, bar: &mut elegance::MenuBarUi<'_>) {
        bar.menu("Actions", |ui| {
            let theme = Theme::current(ui.ctx());
            if ui
                .add(MenuItem::new("Export as CSV").icon("\u{2B07}"))
                .clicked()
            {
                self.mb_last_action = "Export as CSV".into();
            }
            ui.add(MenuItem::new("Export as PDF").icon("\u{2B07}"));
            ui.add(
                MenuItem::new("Share link\u{2026}")
                    .icon("\u{1F517}")
                    .shortcut("\u{2318}\u{21E7}L"),
            );

            ui.separator();
            ui.add(egui::Label::new(theme.faint_text("SCHEDULE")));
            if ui
                .add(
                    MenuItem::new("Every Monday 09:00")
                        .icon("\u{1F4C5}")
                        .checked(self.mb_schedule_on),
                )
                .clicked()
            {
                self.mb_schedule_on = !self.mb_schedule_on;
                self.mb_last_action = format!(
                    "Weekly schedule \u{2192} {}",
                    if self.mb_schedule_on { "on" } else { "off" }
                );
            }
            ui.add(MenuItem::new("Change schedule\u{2026}").icon("\u{23F1}"));

            ui.separator();
            if ui
                .add(
                    MenuItem::new("Delete report\u{2026}")
                        .icon("\u{1F5D1}")
                        .danger()
                        .shortcut("\u{232B}"),
                )
                .clicked()
            {
                self.mb_last_action = "Delete report \u{2026}".into();
            }
        });
    }

    fn section_pairing(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Pairing").show(ui, |ui| {
            ui.add(Switch::new(
                &mut self.pairing_align,
                "Align servers to clients (straight lines)",
            ));
            ui.add_space(8.0);
            let mut pairing = Pairing::new(
                "ref_pairing",
                &self.pairing_clients,
                &self.pairing_servers,
                &mut self.pairing_pairs,
            )
            .left_label("Clients")
            .right_label("Servers");
            if self.pairing_align {
                pairing = pairing.align_right();
            }
            pairing.show(ui);
        });
    }

    fn section_sortable_list(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Sortable list").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(
                theme.faint_text("Drag a row by its grip to reorder. Esc cancels."),
            ));
            ui.add_space(8.0);
            ui.set_max_width(540.0);
            SortableList::new("ref_sortable_list", &mut self.sortable_targets).show(ui);
        });
    }

    fn section_modal(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Modal").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Centered dialog over a dimmed backdrop. Esc or × to dismiss.",
            )));
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(Button::new("Confirm destructive").accent(Accent::Red))
                    .clicked()
                {
                    self.show_modal = true;
                }
                if ui.add(Button::new("Verify to delete").outline()).clicked() {
                    self.show_modal_verify = true;
                }
                if ui
                    .add(Button::new("Form dialog").accent(Accent::Blue))
                    .clicked()
                {
                    self.show_modal_form = true;
                }
                if ui
                    .add(Button::new("Informational").accent(Accent::Green))
                    .clicked()
                {
                    self.show_modal_info = true;
                }
                if ui.add(Button::new("Plain").outline()).clicked() {
                    self.show_modal_plain = true;
                }
            });
        });
    }

    fn section_drawer(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Drawer").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Side-anchored slide-in panel. Built-in focus capture, Esc-to-close, and × button.",
            )));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add(Button::new("Open detail drawer").accent(Accent::Blue))
                    .clicked()
                {
                    self.show_drawer_detail = true;
                }
                if ui
                    .add(Button::new("Open form drawer (left)").outline())
                    .clicked()
                {
                    self.show_drawer_form = true;
                }
            });
        });
    }

    fn section_menu(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Menu").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Click-anchored list of actions with optional shortcuts and a destructive item.",
            )));
            ui.add_space(6.0);
            let menu_trigger = ui.add(Button::new("Open menu").outline().size(ButtonSize::Medium));
            Menu::new("ref_menu").show_below(&menu_trigger, |ui| {
                let _ = ui.add(MenuItem::new("Edit").shortcut("⌘ E"));
                let _ = ui.add(MenuItem::new("Duplicate").shortcut("⌘ D"));
                ui.separator();
                let _ = ui.add(MenuItem::new("Delete").danger());
            });
        });
    }

    fn section_context_menu(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("ContextMenu").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Right-click a target to open a popup menu at the pointer. Works on any \
                 Response with a click sense — text labels, buttons, custom regions.",
            )));
            ui.add_space(10.0);

            // 1. Text label — file row with per-file actions.
            ui.add(egui::Label::new(
                theme.faint_text("Right-click the file name:"),
            ));
            let row = ui.add(
                egui::Label::new(
                    egui::RichText::new("theme.rs   11.4 KB \u{00b7} 34 min ago")
                        .color(theme.palette.text),
                )
                .sense(egui::Sense::click()),
            );
            ContextMenu::new("ref_ctx_file_row").show(&row, |ui| {
                let _ = ui.add(MenuItem::new("Open").shortcut("\u{21B5}"));
                let _ =
                    ui.add(MenuItem::new("Open in new split").shortcut("\u{2318}\u{21E7}\u{21B5}"));
                SubMenuItem::new("Open with").show(ui, |ui| {
                    let _ = ui.add(MenuItem::new("Source editor").shortcut("default"));
                    let _ = ui.add(MenuItem::new("Preview"));
                    let _ = ui.add(MenuItem::new("Hex viewer"));
                    ui.separator();
                    let _ = ui.add(MenuItem::new("Configure defaults\u{2026}"));
                });
                ui.separator();
                ui.add(MenuSection::new("Edit"));
                let _ = ui.add(MenuItem::new("Copy").shortcut("\u{2318}C"));
                let _ = ui.add(MenuItem::new("Duplicate").shortcut("\u{2318}D"));
                let _ = ui.add(MenuItem::new("Rename\u{2026}").shortcut("F2"));
                let _ = ui.add(
                    MenuItem::new("Move to workspace\u{2026}")
                        .shortcut("read-only")
                        .enabled(false),
                );
                ui.separator();
                let _ = ui.add(MenuItem::new("Delete").danger().shortcut("\u{232B}"));
            });

            ui.add_space(14.0);

            // 2. Button — same pattern, attached to a clickable widget.
            // Buttons already have Sense::click(), so no extra wiring is needed.
            ui.add(egui::Label::new(theme.faint_text(
                "Right-click the button (left-click still triggers it normally):",
            )));
            let deploy = ui.add(Button::new("Deploy").accent(Accent::Sky));
            if deploy.clicked() {
                Toast::new("Deploy started")
                    .description("orbit:v2.14.3 \u{2192} staging")
                    .tone(BadgeTone::Info)
                    .show(ui.ctx());
            }
            ContextMenu::new("ref_ctx_deploy_btn")
                .min_width(220.0)
                .show(&deploy, |ui| {
                    ui.add(MenuSection::new("Deploy"));
                    let _ = ui.add(MenuItem::new("Deploy now").shortcut("\u{2318}\u{21B5}"));
                    let _ =
                        ui.add(MenuItem::new("Deploy with options\u{2026}").shortcut("\u{2318}."));
                    let _ = ui.add(MenuItem::new("Dry run").shortcut("\u{2318}\u{21E7}D"));
                    ui.separator();
                    ui.add(MenuSection::new("History"));
                    let _ = ui.add(MenuItem::new("Re-deploy last release"));
                    let _ = ui.add(
                        MenuItem::new("Roll back\u{2026}")
                            .danger()
                            .shortcut("\u{2318}\u{232B}"),
                    );
                });

            ui.add_space(14.0);

            // 3. Custom region — a card-like swatch sensed for clicks.
            ui.add(egui::Label::new(
                theme.faint_text("Right-click anywhere on the card:"),
            ));
            let p = &theme.palette;
            let (rect, card_resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width().min(280.0), 56.0),
                egui::Sense::click(),
            );
            ui.painter().rect(
                rect,
                egui::CornerRadius::same(theme.card_radius as u8),
                p.card,
                egui::Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.left_center() + egui::vec2(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                "ELG-218 \u{00b7} Ship dialog widget",
                egui::FontId::proportional(theme.typography.body),
                p.text,
            );
            ContextMenu::new("ref_ctx_card").show(&card_resp, |ui| {
                let _ = ui.add(MenuItem::new("Rename\u{2026}"));
                SubMenuItem::new("Label color").show(ui, |ui| {
                    let _ = ui.add(MenuItem::new("Sky").radio(false));
                    let _ = ui.add(MenuItem::new("Amber").radio(true));
                    let _ = ui.add(MenuItem::new("Green").radio(false));
                    let _ = ui.add(MenuItem::new("Red").radio(false));
                    ui.separator();
                    let _ = ui.add(MenuItem::new("Clear label"));
                });
                let _ = ui.add(MenuItem::new("Add to sprint\u{2026}"));
                ui.separator();
                let _ = ui.add(MenuItem::new("Archive card").danger());
            });
        });
    }

    fn section_toast(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Toast").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Non-blocking notification. Auto-dismisses after a few seconds.",
            )));
            ui.add_space(6.0);
            if ui.add(Button::new("Toast").accent(Accent::Green)).clicked() {
                Toast::new("Saved")
                    .description("All changes have been persisted.")
                    .tone(BadgeTone::Ok)
                    .show(ui.ctx());
            }
        });
    }

    fn section_popover(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Popover").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Click-anchored floating panel. Lighter than Modal: no backdrop, no focus trap.",
            )));
            ui.add_space(6.0);
            self.popover_examples(ui);
        });
    }

    fn section_tooltip(&mut self, ui: &mut egui::Ui) {
        Card::new().heading("Tooltip").show(ui, |ui| {
            let theme = Theme::current(ui.ctx());
            ui.add(egui::Label::new(theme.faint_text(
                "Hover- or focus-triggered hint. Inherits egui's delay and grace-window chaining.",
            )));
            ui.add_space(6.0);

            labeled(ui, "Label only", |ui| {
                let trigger = ui.add(Button::new("Share").outline().size(ButtonSize::Small));
                Tooltip::new("Copy share link").show(&trigger);
            });

            labeled(ui, "Heading + body + shortcut", |ui| {
                let trigger = ui.add(Button::new("Save").outline());
                Tooltip::new("Write the working tree to disk. Remote sync runs in the background.")
                    .heading("Save changes")
                    .shortcut("\u{2318} S")
                    .show(&trigger);
            });

            labeled(ui, "Explaining a status pill (below)", |ui| {
                let trigger = ui.add(
                    Button::new("degraded")
                        .accent(Accent::Amber)
                        .size(ButtonSize::Small),
                );
                Tooltip::new(
                    "api-west-01 is returning elevated 5xx. Other regions healthy. \
                     Updated 38s ago.",
                )
                .heading("Partial outage")
                .side(TooltipSide::Bottom)
                .show(&trigger);
            });

            labeled(ui, "Field help", |ui| {
                ui.horizontal(|ui| {
                    let theme = Theme::current(ui.ctx());
                    ui.add(egui::Label::new(theme.muted_text("Retry budget")));
                    let trigger = ui.add(Button::new("?").outline().size(ButtonSize::Small));
                    Tooltip::new(
                        "Maximum retries per minute before the circuit breaker opens \
                         and new requests fail fast.",
                    )
                    .heading("Retry budget")
                    .side(TooltipSide::Bottom)
                    .show(&trigger);
                });
            });
        });
    }

    fn popover_examples(&mut self, ui: &mut egui::Ui) {
        labeled(ui, "Placements", |ui| {
            ui.horizontal(|ui| {
                for (label, side) in [
                    ("Top", PopoverSide::Top),
                    ("Bottom", PopoverSide::Bottom),
                    ("Left", PopoverSide::Left),
                    ("Right", PopoverSide::Right),
                ] {
                    let trigger = ui.add(Button::new(label).outline().size(ButtonSize::Small));
                    Popover::new(("placement", label))
                        .side(side)
                        .show(&trigger, |ui| {
                            let theme = Theme::current(ui.ctx());
                            ui.add(egui::Label::new(theme.muted_text(format!(
                                "Opens on the {} side.",
                                label.to_lowercase()
                            ))));
                        });
                }
            });
        });

        labeled(ui, "Title + body + destructive footer", |ui| {
            let trigger = ui.add(Button::new("Delete branch").outline());
            Popover::new("pop_confirm")
                .side(PopoverSide::Bottom)
                .title("Delete feature/snap-baseline?")
                .show(&trigger, |ui| {
                    let theme = Theme::current(ui.ctx());
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

        labeled(ui, "Info card, no footer, fixed width", |ui| {
            let trigger = ui.add(Button::new("What's a baseline?").outline());
            Popover::new("pop_info")
                .side(PopoverSide::Top)
                .title("Baselines")
                .width(300.0)
                .show(&trigger, |ui| {
                    let theme = Theme::current(ui.ctx());
                    ui.add(egui::Label::new(theme.muted_text(
                        "A baseline is the accepted reference image for a widget. \
                         Tests compare each render against it pixel by pixel.",
                    )));
                });
        });

        labeled(ui, "Multi-select filter", |ui| {
            ui.horizontal(|ui| {
                let trigger = ui.add(Button::new("Filter ▾").outline());
                Popover::new("pop_filter")
                    .side(PopoverSide::Bottom)
                    .width(220.0)
                    .show(&trigger, |ui| {
                        let theme = Theme::current(ui.ctx());
                        ui.add(egui::Label::new(theme.faint_text("STATUS")));
                        ui.add(Checkbox::new(&mut self.pop_filter_open, "Open"));
                        ui.add(Checkbox::new(&mut self.pop_filter_in_review, "In review"));
                        ui.add(Checkbox::new(&mut self.pop_filter_merged, "Merged"));
                        ui.add(Checkbox::new(&mut self.pop_filter_closed, "Closed"));
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(theme.faint_text("LABELS")));
                        ui.add(Checkbox::new(
                            &mut self.pop_filter_needs_review,
                            "Needs review",
                        ));
                        ui.add(Checkbox::new(&mut self.pop_filter_ready, "Ready to merge"));
                        ui.add(Checkbox::new(&mut self.pop_filter_blocked, "Blocked"));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let _ = ui.add(Button::new("Clear").outline().size(ButtonSize::Small));
                            let _ = ui.add(
                                Button::new("Apply")
                                    .accent(Accent::Blue)
                                    .size(ButtonSize::Small),
                            );
                        });
                    });
            });
        });

        labeled(ui, "No arrow, custom gap (dropdown surface)", |ui| {
            let trigger = ui.add(Button::new("Quick actions ▾").outline());
            Popover::new("pop_plain")
                .side(PopoverSide::Bottom)
                .arrow(false)
                .gap(4.0)
                .show(&trigger, |ui| {
                    ui.add(MenuItem::new("Rename"));
                    ui.add(MenuItem::new("Duplicate"));
                    ui.add(MenuItem::new("Archive"));
                    ui.separator();
                    ui.add(MenuItem::new("Delete").danger());
                });
        });
    }

    fn modal_demo(&mut self, ctx: &egui::Context) {
        if !self.show_modal {
            return;
        }
        let mut confirm = false;
        let mut cancel = false;
        Modal::new("ref_modal", &mut self.show_modal)
            .heading("Delete project?")
            .subtitle("This action cannot be undone.")
            .header_icon("!")
            .header_accent(Accent::Red)
            .max_width(420.0)
            .alert(true)
            .footer(|ui| {
                if ui
                    .add(Button::new("Delete project").accent(Accent::Red))
                    .clicked()
                {
                    confirm = true;
                }
                if ui.add(Button::new("Cancel").outline()).clicked() {
                    cancel = true;
                }
            })
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                ui.add(egui::Label::new(theme.muted_text(
                    "All dashboards, alerts, and 3 active deployments will be permanently \
                     removed. Members will lose access immediately.",
                )));
            });
        if confirm || cancel {
            self.show_modal = false;
        }
    }

    /// Verify-to-delete: typed phrase gates the destructive action; left
    /// footer slot carries an "Export before deletion" checkbox.
    fn modal_demo_verify(&mut self, ctx: &egui::Context) {
        if !self.show_modal_verify {
            return;
        }
        const PHRASE: &str = "elegance-labs";
        let armed = self.modal_verify_text == PHRASE;
        let mut confirm = false;
        let mut cancel = false;
        let verify_export = &mut self.modal_verify_export;
        let verify_text = &mut self.modal_verify_text;
        Modal::new("ref_modal_verify", &mut self.show_modal_verify)
            .heading("Delete workspace")
            .subtitle("This will remove all 38 projects, 214 dashboards, and 7 team members.")
            .header_icon("!")
            .header_accent(Accent::Red)
            .max_width(480.0)
            .alert(true)
            .footer_left(|ui| {
                ui.add(Checkbox::new(verify_export, "Export data before deletion"));
            })
            .footer(|ui| {
                if ui
                    .add(
                        Button::new("Delete workspace")
                            .accent(Accent::Red)
                            .enabled(armed),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui.add(Button::new("Cancel").outline()).clicked() {
                    cancel = true;
                }
            })
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Label::new(theme.muted_text("Type")));
                    ui.add(egui::Label::new(
                        egui::RichText::new(PHRASE)
                            .monospace()
                            .color(theme.palette.red),
                    ));
                    ui.add(egui::Label::new(theme.muted_text("to confirm.")));
                });
                ui.add_space(6.0);
                ui.add(
                    TextInput::new(verify_text)
                        .desired_width(f32::INFINITY)
                        .id_salt("ref_modal_verify_phrase"),
                );
            });
        if confirm || cancel {
            self.show_modal_verify = false;
        }
    }

    /// Form dialog: multiple fields, kbd-hint in the left footer slot,
    /// Cancel + Create on the right.
    fn modal_demo_form(&mut self, ctx: &egui::Context) {
        if !self.show_modal_form {
            return;
        }
        let mut create = false;
        let mut cancel = false;
        let name = &mut self.modal_form_name;
        let desc = &mut self.modal_form_desc;
        let project = &mut self.modal_form_project;
        let open_after = &mut self.modal_form_open_after;
        Modal::new("ref_modal_form", &mut self.show_modal_form)
            .heading("New dashboard")
            .subtitle("Dashboards can be shared across projects.")
            .header_icon("+")
            .header_accent(Accent::Sky)
            .max_width(540.0)
            .footer_left(|ui| {
                let theme = Theme::current(ui.ctx());
                ui.add(egui::Label::new(
                    theme.faint_text("Esc to cancel · ⌘ ↵ to create"),
                ));
            })
            .footer(|ui| {
                if ui
                    .add(Button::new("Create dashboard").accent(Accent::Blue))
                    .clicked()
                {
                    create = true;
                }
                if ui.add(Button::new("Cancel").outline()).clicked() {
                    cancel = true;
                }
            })
            .show(ctx, |ui| {
                ui.add(
                    TextInput::new(name)
                        .label("Name")
                        .desired_width(f32::INFINITY)
                        .id_salt("ref_modal_form_name"),
                );
                ui.add_space(8.0);
                ui.add(
                    TextInput::new(desc)
                        .label("Description")
                        .hint("Shown in the listing and search results")
                        .desired_width(f32::INFINITY)
                        .id_salt("ref_modal_form_desc"),
                );
                ui.add_space(8.0);
                ui.add(
                    Select::strings(
                        "ref_modal_form_project",
                        project,
                        ["Elegance Core", "elegance-charts", "Ingestion"],
                    )
                    .label("Project"),
                );
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.add(Checkbox::new(open_after, "Open dashboard after creation"));
            });
        if create || cancel {
            self.show_modal_form = false;
        }
    }

    /// Plain modal: heading + body + single Close action, no icon halo.
    fn modal_demo_plain(&mut self, ctx: &egui::Context) {
        if !self.show_modal_plain {
            return;
        }
        let mut close = false;
        Modal::new("ref_modal_plain", &mut self.show_modal_plain)
            .heading("Run summary")
            .max_width(420.0)
            .footer(|ui| {
                if ui.add(Button::new("Close").outline()).clicked() {
                    close = true;
                }
            })
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                ui.add(egui::Label::new(
                    theme.muted_text("Build #4128 · main · 2m 04s · 312 tests passed"),
                ));
                ui.add_space(8.0);
                ui.add(egui::Label::new(theme.faint_text(
                    "No icon, no subtitle. The minimal modal — heading row, body, optional \
                     footer.",
                )));
            });
        if close {
            self.show_modal_plain = false;
        }
    }

    /// Informational modal: success halo, single primary action.
    fn modal_demo_info(&mut self, ctx: &egui::Context) {
        if !self.show_modal_info {
            return;
        }
        let mut done = false;
        Modal::new("ref_modal_info", &mut self.show_modal_info)
            .heading("Payment received")
            .subtitle("Invoice INV-4208 · $842.00")
            .header_icon("✓")
            .header_accent(Accent::Green)
            .max_width(380.0)
            .footer(|ui| {
                if ui.add(Button::new("Done").accent(Accent::Blue)).clicked() {
                    done = true;
                }
            })
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                ui.add(egui::Label::new(theme.muted_text(
                    "Your billing period has been extended to May 31, 2026. A receipt has been \
                     emailed to avery@example.com.",
                )));
            });
        if done {
            self.show_modal_info = false;
        }
    }

    fn drawer_demos(&mut self, ctx: &egui::Context) {
        // Right-side detail drawer with status pill, KV details, and a
        // pinned footer with two action buttons.
        Drawer::new("ref_drawer_detail", &mut self.show_drawer_detail)
            .side(DrawerSide::Right)
            .width(420.0)
            .title("INC-2187 — api-west-02")
            .subtitle("Latency spike · 18 minutes ago")
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                let footer_h = 56.0;
                let body_h = (ui.available_height() - footer_h).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(egui::Label::new(theme.faint_text("STATUS")));
                            ui.add_space(4.0);
                            ui.add(StatusPill::new().item("api-web", IndicatorState::Connecting));
                            ui.add_space(14.0);

                            ui.add(egui::Label::new(theme.faint_text("DETAILS")));
                            ui.add_space(4.0);
                            for (k, v) in [
                                ("Service", "api-web"),
                                ("Region", "us-west-2"),
                                ("Signal", "p95_latency_ms > 300 for 3m"),
                                ("Owner", "Platform Edge"),
                                ("Assignee", "Avery Lin"),
                            ] {
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [110.0, 18.0],
                                        egui::Label::new(theme.muted_text(k)),
                                    );
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(v)
                                            .color(theme.palette.text)
                                            .monospace()
                                            .size(theme.typography.label),
                                    ));
                                });
                                ui.add_space(2.0);
                            }
                            ui.add_space(12.0);
                            ui.add(egui::Label::new(theme.faint_text("SUMMARY")));
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(theme.muted_text(
                                "Traffic to /v1/query is 3.4× baseline since 14:12 UTC. \
                                 p95 latency has crossed 300 ms for the last 18 minutes. \
                                 Upstream search is healthy; reranker queue depth is rising.",
                            )));
                        });
                    },
                );
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.add(
                            Button::new("Acknowledge")
                                .accent(Accent::Blue)
                                .size(ButtonSize::Small),
                        );
                        let _ = ui.add(Button::new("Snooze").outline().size(ButtonSize::Small));
                    });
                });
            });

        // Left-side form drawer — exercises the Left anchor + form inputs.
        let mut form_cancel = false;
        Drawer::new("ref_drawer_form", &mut self.show_drawer_form)
            .side(DrawerSide::Left)
            .width(440.0)
            .title("Edit member · Avery Lin")
            .subtitle("Changes apply after save.")
            .show(ctx, |ui| {
                let theme = Theme::current(ui.ctx());
                let footer_h = 56.0;
                let body_h = (ui.available_height() - footer_h).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(egui::Label::new(theme.faint_text("PROFILE")));
                            ui.add_space(4.0);
                            ui.add(
                                TextInput::new(&mut self.drawer_form_name)
                                    .label("Display name")
                                    .id_salt("drawer_form_name"),
                            );
                            ui.add(
                                TextInput::new(&mut self.drawer_form_email)
                                    .label("Email")
                                    .id_salt("drawer_form_email"),
                            );
                            ui.add(
                                TextInput::new(&mut self.drawer_form_role)
                                    .label("Role")
                                    .id_salt("drawer_form_role"),
                            );
                            ui.add_space(10.0);
                            ui.add(egui::Label::new(theme.faint_text("NOTES")));
                            ui.add_space(4.0);
                            ui.add(
                                TextArea::new(&mut self.drawer_form_notes)
                                    .label("Notes (internal)")
                                    .rows(3)
                                    .id_salt("drawer_form_notes"),
                            );
                        });
                    },
                );
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.add(
                            Button::new("Save changes")
                                .accent(Accent::Blue)
                                .size(ButtonSize::Small),
                        );
                        if ui
                            .add(Button::new("Cancel").outline().size(ButtonSize::Small))
                            .clicked()
                        {
                            form_cancel = true;
                        }
                    });
                });
            });
        if form_cancel {
            self.show_drawer_form = false;
        }
    }
}

/// The demo dock's tabs. Variants can carry data, like an untitled tab's
/// number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DemoTab {
    Files,
    Editor,
    Preview,
    Console,
    Untitled(u32),
}

impl DemoTab {
    fn label(self) -> String {
        match self {
            Self::Files => "Files".into(),
            Self::Editor => "theme.rs".into(),
            Self::Preview => "Preview".into(),
            Self::Console => "Console".into(),
            Self::Untitled(n) => format!("untitled-{n}"),
        }
    }
}

struct DemoDockViewer;

impl DockViewer for DemoDockViewer {
    type Tab = DemoTab;

    fn tab(&mut self, tab: &DemoTab) -> DockTab {
        let base = DockTab::new(tab.label());
        match tab {
            DemoTab::Files => base.icon(glyphs::FOLDER),
            DemoTab::Editor => base.dirty(true),
            DemoTab::Console => base.icon(glyphs::TERMINAL).scroll([false, true]),
            DemoTab::Preview | DemoTab::Untitled(_) => base,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &DemoTab) {
        let theme = Theme::current(ui.ctx());
        if *tab == DemoTab::Console {
            for i in 1..=40 {
                ui.label(theme.faint_text(format!("[{i:02}] build step {i} finished")));
            }
            return;
        }
        ui.label(theme.muted_text(format!("Contents of {}", tab.label())));
        ui.add_space(6.0);
        ui.label(theme.faint_text("Drag a tab to reorder it, move it to another pane,"));
        ui.label(
            theme.faint_text("or drop it on a pane edge to split. Drag the gutters to resize."),
        );
    }
}

fn demo_dock_layout() -> DockLayout<DemoTab> {
    let mut layout = DockLayout::from_node(DockNode::row(
        0.28,
        DockNode::tabs([DemoTab::Files]),
        DockNode::column(
            0.62,
            DockNode::tabs([DemoTab::Editor, DemoTab::Preview]),
            DockNode::tabs([DemoTab::Console]),
        ),
    ));
    layout.activate(&DemoTab::Editor);
    layout
}

fn labeled(ui: &mut egui::Ui, label: &str, body: impl FnOnce(&mut egui::Ui)) {
    let theme = Theme::current(ui.ctx());
    ui.add_space(4.0);
    ui.add(egui::Label::new(theme.muted_text(label)));
    ui.add_space(4.0);
    body(ui);
    ui.add_space(8.0);
}

/// A rolling window of metric samples backing a stat card. Each tick a new
/// sample is generated from a simple random walk (the same scheme the HTML
/// mockup uses) and appended to the series; the oldest sample falls off so
/// the window stays at 24 points.
#[derive(Debug)]
struct StatTick {
    series: Vec<f32>,
    value: f32,
    prev_value: f32,
}

impl StatTick {
    /// `series` seeds the rolling window; the displayed value is taken
    /// from its last sample so the headline number always matches the
    /// sparkline endpoint. `initial_delta` back-derives `prev_value` so
    /// the first render shows that delta on the chip; subsequent ticks
    /// recompute it from the previous-vs-current ratio.
    fn new(series: &[f32], initial_delta: f32) -> Self {
        let value = *series.last().expect("StatTick series must be non-empty");
        let prev_value = value / (1.0 + initial_delta);
        Self {
            series: series.to_vec(),
            value,
            prev_value,
        }
    }

    fn delta(&self) -> f32 {
        if self.prev_value.abs() < f32::EPSILON {
            0.0
        } else {
            (self.value - self.prev_value) / self.prev_value
        }
    }

    fn advance(&mut self, rng: &mut u64) {
        let last = *self.series.last().unwrap_or(&self.value);
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &v in &self.series {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
        let spread = ((max - min) * 0.15).max(1.0);
        let next = (last + (next_unit(rng) - 0.5) * spread).max(0.0);
        self.series.push(next);
        if self.series.len() > 24 {
            self.series.remove(0);
        }
        self.prev_value = self.value;
        self.value = next;
    }
}

/// xorshift64 step returning a value in `[0.0, 1.0)`. Cheap, deterministic
/// per seed, and doesn't pull in a `rand` dependency just for a demo.
fn next_unit(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    (x >> 11) as f32 / (1u64 << 53) as f32
}
