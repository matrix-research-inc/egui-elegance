//! Elegance — opinionated, beautiful widgets for egui.
//!
//! Elegance is a small companion crate to [`egui`] that provides a cohesive
//! design system inspired by modern web UIs: chunky rounded buttons in a
//! handful of accent colors, crisp inputs with a focus ring, pill-shaped
//! status indicators, cards, tabs, segmented buttons, and a matching colour
//! palette. Four palettes ship built-in — two dark
//! ([`Theme::slate`] and [`Theme::charcoal`]) and two light
//! ([`Theme::frost`] and [`Theme::paper`]) — paired so you can toggle without
//! a layout shift.
//!
//! # Getting started
//!
//! ```no_run
//! use eframe::egui;
//! use elegance::{Theme, Button, Card, Accent};
//!
//! fn main() -> eframe::Result<()> {
//!     eframe::run_ui_native(
//!         "elegance demo",
//!         eframe::NativeOptions::default(),
//!         |ui, _| {
//!             Theme::slate().install(ui.ctx());
//!             egui::CentralPanel::default().show(ui, |ui| {
//!                 Card::new().heading("Hello").show(ui, |ui| {
//!                     if ui.add(Button::new("Click me").accent(Accent::Blue))
//!                         .clicked()
//!                     {
//!                         println!("clicked!");
//!                     }
//!                 });
//!             });
//!         },
//!     )
//! }
//! ```
//!
//! # Design
//!
//! All visuals are driven by a [`Theme`] value. Calling [`Theme::install`]
//! once at startup configures [`egui::Style`] so that built-in widgets
//! (labels, sliders, etc.) inherit the elegance look, and it stores the
//! theme in `ctx` memory so elegance widgets can pick it up automatically.

#![warn(missing_debug_implementations)]
#![deny(missing_docs)]

mod accordion;
mod avatar;
mod badge;
mod browser_tabs;
mod button;
mod callout;
mod card;
mod checkbox;
mod collapsing;
mod color_picker;
mod commit;
mod context_menu;
mod drawer;
mod file_drop_zone;
mod flash;
mod focus;
mod gauge;
mod indicator;
mod input;
mod knob;
mod log_bar;
mod menu;
mod menu_bar;
mod metric_slider;
mod modal;
mod overlay;
mod pairing;
mod percent_slider;
mod pill;
mod popover;
mod progress_bar;
mod progress_ring;
mod range_slider;
mod removable_chip;
mod segmented;
mod segmented_control;
mod select;
mod slider;
mod sortable_list;
mod spinner;
mod stat_card;
mod steps;
mod switch;
mod tabs;
mod tag_input;
mod text_area;
mod theme;
mod theme_switcher;
mod toast;
mod tooltip;

pub use accordion::{Accordion, AccordionItem, AccordionUi};
pub use avatar::{Avatar, AvatarGroup, AvatarPresence, AvatarSize, AvatarTone};
pub use badge::{Badge, BadgeTone};
pub use browser_tabs::{BrowserTab, BrowserTabs, BrowserTabsEvent};
pub use button::{Button, ButtonSize};
pub use callout::{Callout, CalloutTone};
pub use card::Card;
pub use checkbox::Checkbox;
pub use collapsing::CollapsingSection;
pub use color_picker::ColorPicker;
pub use commit::ResponseCommitExt;
pub use context_menu::ContextMenu;
pub use drawer::{Drawer, DrawerSide};
/// The bound on a caller-supplied widget id salt: any `Hash + Debug` value,
/// such as a string, integer, or tuple.
///
/// An alias of [`egui::AsIdSalt`] under elegance's original name. Not to be
/// confused with [`egui::IdSalt`], the hashed salt value itself.
pub use egui::AsIdSalt as IdSalt;
pub use file_drop_zone::{FileDropResponse, FileDropZone};
pub use flash::{FLASH_DURATION, FlashKind, ResponseFlashExt, flash_error, flash_success};
pub use gauge::{GaugeZones, LinearGauge, RadialGauge};
pub use indicator::{Indicator, IndicatorState};
pub use input::TextInput;
pub use knob::{Knob, KnobScale, KnobSize};
pub use log_bar::{LogBar, LogEntry, LogKind};
pub use menu::{Menu, MenuItem, MenuSection, SubMenuItem};
pub use menu_bar::{BrandLogo, MenuBar, MenuBarUi};
pub use metric_slider::MetricSlider;
pub use modal::Modal;
pub use pairing::{IconTint, PairItem, Pairing};
pub use percent_slider::PercentSlider;
pub use pill::StatusPill;
pub use popover::{Popover, PopoverSide};
pub use progress_bar::ProgressBar;
pub use progress_ring::ProgressRing;
pub use range_slider::RangeSlider;
pub use removable_chip::{RemovableChip, RemovableChipResponse};
pub use segmented::SegmentedButton;
pub use segmented_control::{Segment, SegmentDot, SegmentedControl, SegmentedSize};
pub use select::Select;
pub use slider::{Slider, SliderHandle};
pub use sortable_list::{SortableItem, SortableList, SortableStatus};
pub use spinner::Spinner;
pub use stat_card::StatCard;
pub use steps::{Steps, StepsStyle};
pub use switch::Switch;
pub use tabs::TabBar;
pub use tag_input::{TagInput, TagInputResponse};
pub use text_area::TextArea;
pub use theme::{Accent, BuiltInTheme, Palette, Theme, Typography};
pub use theme_switcher::ThemeSwitcher;
pub use toast::{Toast, Toasts};
pub use tooltip::{Tooltip, TooltipSide};

/// Re-export of [`egui`] for convenience.
pub use egui;

/// Re-export of [`egui::Margin`] so callers can build per-side padding
/// values (e.g. for [`Card::padding`]) without reaching into `egui`.
pub use egui::Margin;

/// Stable codepoints for the icon glyphs bundled in the Elegance Symbols
/// font. All icons are sourced from [Lucide](https://lucide.dev) and are
/// kept in sync via `scripts/update_lucide_glyphs.py`. Use these in
/// [`egui::RichText`] when you want one of elegance's icons in your own
/// UI.
///
/// ```no_run
/// # use elegance::glyphs;
/// # egui::__run_test_ui(|ui| {
/// ui.label(egui::RichText::new(glyphs::UPLOAD).size(24.0));
/// # });
/// ```
pub mod glyphs {
    /// Upload-tray icon. Source: [Lucide `upload`](https://lucide.dev/icons/upload).
    pub const UPLOAD: char = '\u{E000}';
    /// Download-tray icon. Source: [Lucide `download`](https://lucide.dev/icons/download).
    pub const DOWNLOAD: char = '\u{E001}';
    /// Search / magnifier icon. Source: [Lucide `search`](https://lucide.dev/icons/search).
    pub const SEARCH: char = '\u{E002}';
    /// Pin icon. Source: [Lucide `pin`](https://lucide.dev/icons/pin).
    pub const PIN: char = '\u{E003}';
    /// Copy / duplicate icon. Source: [Lucide `copy`](https://lucide.dev/icons/copy).
    pub const COPY: char = '\u{E004}';
    /// Circular alert icon. Source: [Lucide `circle-alert`](https://lucide.dev/icons/circle-alert).
    pub const CIRCLE_ALERT: char = '\u{E005}';
    /// Network / hub icon. Source: [Lucide `network`](https://lucide.dev/icons/network).
    pub const NETWORK: char = '\u{E006}';
    /// Zoom-in (magnifier with `+`) icon. Source: [Lucide `zoom-in`](https://lucide.dev/icons/zoom-in).
    pub const ZOOM_IN: char = '\u{E007}';
    /// Zoom-out (magnifier with `-`) icon. Source: [Lucide `zoom-out`](https://lucide.dev/icons/zoom-out).
    pub const ZOOM_OUT: char = '\u{E008}';
    /// Power icon. Source: [Lucide `power`](https://lucide.dev/icons/power).
    pub const POWER: char = '\u{E009}';
    /// Trash / delete icon, for destructive actions. Source: [Lucide `trash-2`](https://lucide.dev/icons/trash-2).
    pub const TRASH: char = '\u{E00A}';
    /// Pencil / edit icon. Source: [Lucide `pencil`](https://lucide.dev/icons/pencil).
    pub const PENCIL: char = '\u{E00B}';
    /// Plus / add icon, for "new" / "create" actions. Source: [Lucide `plus`](https://lucide.dev/icons/plus).
    pub const PLUS: char = '\u{E00C}';
    /// Terminal / console icon, for opening a shell session. Source: [Lucide `terminal`](https://lucide.dev/icons/terminal).
    pub const TERMINAL: char = '\u{E00D}';
    /// Key icon, for keys / credentials / secrets. Source: [Lucide `key-round`](https://lucide.dev/icons/key-round).
    pub const KEY: char = '\u{E00E}';
    /// Refresh / reload icon, for re-fetch or re-scan. Source: [Lucide `refresh-cw`](https://lucide.dev/icons/refresh-cw).
    pub const REFRESH: char = '\u{E00F}';
    /// Up-arrow icon, for ascending sort, "move up", or navigation. Source: [Lucide `arrow-up`](https://lucide.dev/icons/arrow-up).
    pub const ARROW_UP: char = '\u{E010}';
    /// Down-arrow icon, for descending sort, "move down", or navigation. Source: [Lucide `arrow-down`](https://lucide.dev/icons/arrow-down).
    pub const ARROW_DOWN: char = '\u{E011}';
    /// Info icon, for the informational callout tone. Source: [Lucide `info`](https://lucide.dev/icons/info).
    pub const INFO: char = '\u{E012}';
    /// Warning triangle, for the caution callout tone. Source: [Lucide `triangle-alert`](https://lucide.dev/icons/triangle-alert).
    pub const TRIANGLE_ALERT: char = '\u{E013}';
    /// Circled cross, for the error / danger callout tone. Source: [Lucide `circle-x`](https://lucide.dev/icons/circle-x).
    pub const CIRCLE_X: char = '\u{E014}';
    /// Circled check, for the success callout tone. Source: [Lucide `circle-check`](https://lucide.dev/icons/circle-check).
    pub const CIRCLE_CHECK: char = '\u{E015}';
    /// Settings / preferences gear. Source: [Lucide `settings`](https://lucide.dev/icons/settings).
    pub const SETTINGS: char = '\u{E016}';
    /// Hamburger menu icon, for nav drawers. Source: [Lucide `menu`](https://lucide.dev/icons/menu).
    pub const MENU: char = '\u{E017}';
    /// Left-arrow icon, for back / previous / navigation. Source: [Lucide `arrow-left`](https://lucide.dev/icons/arrow-left).
    pub const ARROW_LEFT: char = '\u{E018}';
    /// Right-arrow icon, for forward / next / navigation. Source: [Lucide `arrow-right`](https://lucide.dev/icons/arrow-right).
    pub const ARROW_RIGHT: char = '\u{E019}';
    /// External-link icon, for links that open outside the app. Source: [Lucide `external-link`](https://lucide.dev/icons/external-link).
    pub const EXTERNAL_LINK: char = '\u{E01A}';
    /// Right chevron, for nav / expand affordances. Source: [Lucide `chevron-right`](https://lucide.dev/icons/chevron-right).
    pub const CHEVRON_RIGHT: char = '\u{E01B}';
    /// Down chevron, for nav / expand affordances. Source: [Lucide `chevron-down`](https://lucide.dev/icons/chevron-down).
    pub const CHEVRON_DOWN: char = '\u{E01C}';
    /// Filter / funnel icon, for refining lists and tables. Source: [Lucide `funnel`](https://lucide.dev/icons/funnel).
    pub const FILTER: char = '\u{E01D}';
    /// Eye icon, for "visible" / reveal toggles. Source: [Lucide `eye`](https://lucide.dev/icons/eye).
    pub const EYE: char = '\u{E01E}';
    /// Eye-off icon, for "hidden" / mask toggles. Source: [Lucide `eye-off`](https://lucide.dev/icons/eye-off).
    pub const EYE_OFF: char = '\u{E01F}';
    /// Home icon, for the navigation root. Source: [Lucide `house`](https://lucide.dev/icons/house).
    pub const HOME: char = '\u{E020}';
    /// Lock icon, for a secured / locked state (complements [`KEY`]). Source: [Lucide `lock`](https://lucide.dev/icons/lock).
    pub const LOCK: char = '\u{E021}';
    /// Save icon, for persisting changes. Source: [Lucide `save`](https://lucide.dev/icons/save).
    pub const SAVE: char = '\u{E022}';
    /// Folder icon, for directories, collections, and breadcrumb roots
    /// (pairs with [`HOME`]). Source: [Lucide `folder`](https://lucide.dev/icons/folder).
    pub const FOLDER: char = '\u{E023}';
    /// Open-folder icon, for the expanded or current directory in a tree or
    /// breadcrumb (pairs with [`FOLDER`]).
    /// Source: [Lucide `folder-open`](https://lucide.dev/icons/folder-open).
    pub const FOLDER_OPEN: char = '\u{E024}';
    /// Check / done mark, mapped at standard U+2713 so plain `'✓'` literals
    /// also pick up the elegance treatment.
    /// Source: [Lucide `check`](https://lucide.dev/icons/check).
    pub const CHECK: char = '\u{2713}';
    /// Cross / dismiss mark, mapped at standard U+2717 so plain `'✗'` literals
    /// also pick up the elegance treatment.
    /// Source: [Lucide `x`](https://lucide.dev/icons/x).
    pub const X: char = '\u{2717}';
}

/// Request a repaint such that the next paint comes ~`1/hz` seconds from now,
/// independent of display refresh rate.
///
/// [`egui::Context::request_repaint_after`] internally subtracts `predicted_dt`
/// from the requested delay to budget for the paint taking time. On a 60 Hz
/// integration (egui's default) that subtraction is ~16.7 ms, so a naive
/// `request_repaint_after(1/30 s)` lands on the very next vsync and produces
/// ~60 Hz — double the rate you asked for. This helper adds `predicted_dt`
/// back in so the effective cadence lands near `1/hz` on any refresh rate.
///
/// Typical use: throttle continuously-animating widgets (spinners, progress
/// fills) to 20–30 Hz so they don't burn a full vsync budget on motion the
/// eye can't resolve.
#[track_caller]
pub fn request_repaint_at_rate(ctx: &egui::Context, hz: f32) {
    let pd = ctx.input(|i| i.predicted_dt);
    if let Ok(d) = std::time::Duration::try_from_secs_f32(1.0 / hz + pd) {
        ctx.request_repaint_after(d);
    }
}
