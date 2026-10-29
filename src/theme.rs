//! Theme: colours, typography, and egui `Style` integration.
//!
//! Four built-in palettes ship with the crate, paired as dark/light:
//! [`Palette::slate`] (cool dark blue — the default) and [`Palette::frost`]
//! (cool light blue-tinted) form one pair; [`Palette::charcoal`] (neutral
//! dark grey) and [`Palette::paper`] (neutral warm light) form the other.
//! Switching between members of a pair keeps layouts pixel-identical and
//! only swaps luminance.

use egui::{
    Color32, Context, CornerRadius, FontData, FontFamily, FontId, Id, Margin, Stroke, Style,
    TextStyle, Vec2, Visuals, WidgetText,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    style::{Selection, Widgets},
};

/// Bundled subset of DejaVu Sans covering the arrow / key / math-ellipsis
/// glyphs that aren't in egui's default fonts. Registered as a fallback in
/// both Proportional and Monospace families by [`Theme::install`].
const SYMBOLS_FONT_BYTES: &[u8] = include_bytes!("../assets/elegance-symbols.ttf");
const SYMBOLS_FONT_KEY: &str = "elegance-symbols";

/// The six accent colours supported by elegance.
///
/// Every accent has a resting and a pressed/hover shade. These drive
/// [`Button`](crate::Button), the segmented button's `on` state, and any
/// other accent-tinted widget. Structural treatments like the outline
/// button are widget options (e.g. [`Button::outline`](crate::Button::outline)),
/// not accents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Accent {
    /// Primary blue — the default button accent.
    Blue,
    /// Green — affirmative actions (Deploy, Save).
    Green,
    /// Red — destructive actions (Delete, Rollback).
    Red,
    /// Purple — neutral-positive actions or brand moments.
    Purple,
    /// Amber — caution-leaning actions that aren't destructive.
    Amber,
    /// Sky — shares the focus accent used by focus rings and active states.
    Sky,
}

/// All the colours used by the design system.
///
/// You can tweak individual fields before calling [`Theme::install`] if you
/// want to nudge the default slate look.
///
/// # Re-theming to a brand colour
///
/// "The accent" is two separate roles, deliberately kept apart:
///
/// - **Structural accent** — focus rings, active tabs, selection, info
///   tone, step markers, drop zones. All driven by the single [`focus`]
///   field, so one assignment recolours every active/selected state:
///   `palette.focus = my_gold;`
/// - **Semantic button palette** — the [`Accent`] enum (`Blue`, `Green`,
///   `Red`, `Purple`, `Amber`). These encode *meaning* (red = destructive,
///   green = affirmative) and are intentionally not a single colour.
///   [`Button`](crate::Button) defaults to [`Accent::Blue`], so to rebrand
///   the *primary* button across the whole app, retint that one accent:
///   `palette.blue = my_gold; palette.blue_hover = my_gold_dark;` — no
///   per-button `.accent(…)` call needed.
///
/// So a one-colour gold rebrand is two assignments with distinct intent
/// (`focus` for active states, `blue` for the primary button), not a single
/// override — collapsing them would discard the semantic button colours.
///
/// [`focus`]: Palette::focus
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// Whether this palette is a dark-mode palette.
    ///
    /// Drives [`Visuals::dark`] vs [`Visuals::light`] and flips the
    /// direction of subtle-lift mixes (see [`Palette::depth_tint`]).
    /// If you build a custom palette, set this to match the luminance of
    /// `bg` / `card`.
    pub is_dark: bool,

    /// Overall application background.
    pub bg: Color32,
    /// Card / panel surface colour.
    pub card: Color32,
    /// Input field background (typically the same as `bg`).
    pub input_bg: Color32,
    /// Border colour used for inputs, cards and separators.
    pub border: Color32,

    /// Primary text colour.
    pub text: Color32,
    /// Secondary text (labels, field captions).
    pub text_muted: Color32,
    /// Tertiary text (hints, placeholders, disabled-ish).
    pub text_faint: Color32,

    /// Blue accent, resting state — backs [`Accent::Blue`].
    pub blue: Color32,
    /// Blue accent, hover/pressed state.
    pub blue_hover: Color32,
    /// Green accent, resting state — backs [`Accent::Green`].
    pub green: Color32,
    /// Green accent, hover/pressed state.
    pub green_hover: Color32,
    /// Red accent, resting state — backs [`Accent::Red`].
    pub red: Color32,
    /// Red accent, hover/pressed state.
    pub red_hover: Color32,
    /// Purple accent, resting state — backs [`Accent::Purple`].
    pub purple: Color32,
    /// Purple accent, hover/pressed state.
    pub purple_hover: Color32,
    /// Amber accent, resting state — backs [`Accent::Amber`].
    pub amber: Color32,
    /// Amber accent, hover/pressed state.
    pub amber_hover: Color32,
    /// The structural focus accent: focus rings, active tabs, text
    /// selection, "dirty" input bars, info-toned badges/callouts, active
    /// step markers, drop-zone highlights, and more.
    ///
    /// This is the single field to retint when rebranding the "active /
    /// selected" colour of the whole UI — it is independent of the
    /// semantic button palette (`blue`/`green`/`red`/… backing the
    /// [`Accent`] enum). See the [`Palette`] docs for re-theming guidance.
    pub focus: Color32,

    /// Success accent used by the status light and flashy feedback.
    pub success: Color32,
    /// Danger accent used by the status light and flashy feedback.
    pub danger: Color32,
    /// Warning accent used by the "connecting" status light.
    pub warning: Color32,
}

impl Palette {
    /// The default "slate" palette — cool corporate dark blue with a sky-blue
    /// focus ring. Matches the reference design.
    pub fn slate() -> Self {
        Self {
            is_dark: true,
            bg: rgb(0x0f, 0x17, 0x2a),
            card: rgb(0x1e, 0x29, 0x3b),
            input_bg: rgb(0x0f, 0x17, 0x2a),
            border: rgb(0x33, 0x41, 0x55),

            text: rgb(0xe2, 0xe8, 0xf0),
            text_muted: rgb(0x94, 0xa3, 0xb8),
            text_faint: rgb(0x64, 0x74, 0x8b),

            blue: rgb(0x25, 0x63, 0xeb),
            blue_hover: rgb(0x1d, 0x4e, 0xd8),
            green: rgb(0x16, 0xa3, 0x4a),
            green_hover: rgb(0x15, 0x80, 0x3d),
            red: rgb(0xdc, 0x26, 0x26),
            red_hover: rgb(0xb9, 0x1c, 0x1c),
            purple: rgb(0x7c, 0x3a, 0xed),
            purple_hover: rgb(0x6d, 0x28, 0xd9),
            amber: rgb(0xd9, 0x77, 0x06),
            amber_hover: rgb(0xb4, 0x53, 0x09),
            focus: rgb(0x38, 0xbd, 0xf8),

            success: rgb(0x4a, 0xde, 0x80),
            danger: rgb(0xf8, 0x71, 0x71),
            warning: rgb(0xfb, 0xbf, 0x24),
        }
    }

    /// The "charcoal" palette — a neutral dark-grey surface with a
    /// cyan focus accent. Minimalist and monochrome compared to the
    /// blue-tinged [`Palette::slate`].
    pub fn charcoal() -> Self {
        Self {
            is_dark: true,
            bg: rgb(0x0f, 0x0f, 0x10),
            card: rgb(0x1c, 0x1c, 0x1e),
            input_bg: rgb(0x0f, 0x0f, 0x10),
            border: rgb(0x38, 0x38, 0x3a),

            text: rgb(0xfa, 0xfa, 0xfa),
            text_muted: rgb(0xa1, 0xa1, 0xaa),
            text_faint: rgb(0x71, 0x71, 0x7a),

            blue: rgb(0x3b, 0x82, 0xf6),
            blue_hover: rgb(0x25, 0x63, 0xeb),
            green: rgb(0x22, 0xc5, 0x5e),
            green_hover: rgb(0x16, 0xa3, 0x4a),
            red: rgb(0xef, 0x44, 0x44),
            red_hover: rgb(0xdc, 0x26, 0x26),
            purple: rgb(0x8b, 0x5c, 0xf6),
            purple_hover: rgb(0x7c, 0x3a, 0xed),
            amber: rgb(0xf5, 0x9e, 0x0b),
            amber_hover: rgb(0xd9, 0x77, 0x06),
            focus: rgb(0x22, 0xd3, 0xee),

            success: rgb(0x4a, 0xde, 0x80),
            danger: rgb(0xf8, 0x71, 0x71),
            warning: rgb(0xfb, 0xbf, 0x24),
        }
    }

    /// The "frost" palette — the light-mode counterpart to
    /// [`Palette::slate`]. Slate-tinted off-white surfaces, deep slate
    /// text, and the same cool accent family with slightly deepened
    /// shades so white-on-accent button labels remain legible.
    pub fn frost() -> Self {
        Self {
            is_dark: false,
            bg: rgb(0xe2, 0xe8, 0xf0),
            card: rgb(0xf8, 0xfa, 0xfc),
            input_bg: rgb(0xff, 0xff, 0xff),
            border: rgb(0x94, 0xa3, 0xb8),

            text: rgb(0x0f, 0x17, 0x2a),
            text_muted: rgb(0x47, 0x55, 0x69),
            text_faint: rgb(0x64, 0x74, 0x8b),

            blue: rgb(0x25, 0x63, 0xeb),
            blue_hover: rgb(0x1d, 0x4e, 0xd8),
            green: rgb(0x16, 0xa3, 0x4a),
            green_hover: rgb(0x15, 0x80, 0x3d),
            red: rgb(0xdc, 0x26, 0x26),
            red_hover: rgb(0xb9, 0x1c, 0x1c),
            purple: rgb(0x7c, 0x3a, 0xed),
            purple_hover: rgb(0x6d, 0x28, 0xd9),
            amber: rgb(0xd9, 0x77, 0x06),
            amber_hover: rgb(0xb4, 0x53, 0x09),
            focus: rgb(0x03, 0x74, 0xb0),

            success: rgb(0x16, 0xa3, 0x4a),
            danger: rgb(0xdc, 0x26, 0x26),
            warning: rgb(0xd9, 0x77, 0x06),
        }
    }

    /// The "paper" palette — the light-mode counterpart to
    /// [`Palette::charcoal`]. Warm neutral off-white surfaces with a
    /// darkened cyan focus accent to match charcoal's cool accent flavour.
    pub fn paper() -> Self {
        Self {
            is_dark: false,
            bg: rgb(0xec, 0xe9, 0xe4),
            card: rgb(0xfa, 0xf8, 0xf3),
            input_bg: rgb(0xff, 0xff, 0xff),
            border: rgb(0xbc, 0xb6, 0xa8),

            text: rgb(0x1c, 0x1a, 0x16),
            text_muted: rgb(0x57, 0x52, 0x4a),
            text_faint: rgb(0x8a, 0x83, 0x77),

            blue: rgb(0x25, 0x63, 0xeb),
            blue_hover: rgb(0x1d, 0x4e, 0xd8),
            green: rgb(0x16, 0xa3, 0x4a),
            green_hover: rgb(0x15, 0x80, 0x3d),
            red: rgb(0xdc, 0x26, 0x26),
            red_hover: rgb(0xb9, 0x1c, 0x1c),
            purple: rgb(0x7c, 0x3a, 0xed),
            purple_hover: rgb(0x6d, 0x28, 0xd9),
            amber: rgb(0xd9, 0x77, 0x06),
            amber_hover: rgb(0xb4, 0x53, 0x09),
            focus: rgb(0x0c, 0x80, 0x9e),

            success: rgb(0x16, 0xa3, 0x4a),
            danger: rgb(0xdc, 0x26, 0x26),
            warning: rgb(0xd9, 0x77, 0x06),
        }
    }

    /// Mix `base` toward a "more recessed" colour by factor `t`.
    ///
    /// In dark palettes this mixes toward white (adding luminance — a
    /// subtle *lift*); in light palettes it mixes toward black (removing
    /// luminance — a subtle *shade*). Either way the result pops slightly
    /// off the neighbouring surface. Used for hover states on otherwise
    /// plain fills, and the faint card-ish backgrounds.
    pub fn depth_tint(&self, base: Color32, t: f32) -> Color32 {
        let toward = if self.is_dark {
            Color32::WHITE
        } else {
            Color32::BLACK
        };
        mix(base, toward, t)
    }

    /// Resolve the resting fill colour for a given accent.
    pub fn accent_fill(&self, accent: Accent) -> Color32 {
        match accent {
            Accent::Blue => self.blue,
            Accent::Green => self.green,
            Accent::Red => self.red,
            Accent::Purple => self.purple,
            Accent::Amber => self.amber,
            Accent::Sky => self.focus,
        }
    }

    /// Resolve the hover / pressed fill colour for a given accent.
    pub fn accent_hover(&self, accent: Accent) -> Color32 {
        match accent {
            Accent::Blue => self.blue_hover,
            Accent::Green => self.green_hover,
            Accent::Red => self.red_hover,
            Accent::Purple => self.purple_hover,
            Accent::Amber => self.amber_hover,
            Accent::Sky => mix(self.focus, Color32::BLACK, 0.15),
        }
    }
}

/// Typography settings shared by all widgets.
///
/// Font sizes are expressed in egui points (equivalent to CSS pixels at
/// the default zoom level).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    /// Default body text size.
    pub body: f32,
    /// Button label size.
    pub button: f32,
    /// Field-label size (the text above a [`TextInput`](crate::TextInput), for example).
    pub label: f32,
    /// Secondary text size — hints, captions, badges.
    pub small: f32,
    /// Heading size used by [`Card`](crate::Card) titles.
    pub heading: f32,
    /// Monospace size used by code-style content.
    pub monospace: f32,
}

impl Typography {
    /// The default typography scale.
    pub fn elegant() -> Self {
        Self {
            body: 14.0,
            button: 13.5,
            label: 13.0,
            small: 12.0,
            heading: 16.0,
            monospace: 13.0,
        }
    }
}

/// The full elegance theme — colours + typography + a handful of shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Colour palette driving every widget.
    pub palette: Palette,
    /// Font sizes shared across widgets.
    pub typography: Typography,

    /// Corner radius used for buttons, inputs, selects and segmented buttons.
    pub control_radius: f32,
    /// Corner radius used for cards.
    pub card_radius: f32,
    /// Inner padding applied to cards.
    pub card_padding: f32,
    /// Vertical padding inside buttons and inputs.
    pub control_padding_y: f32,
    /// Horizontal padding inside buttons.
    pub control_padding_x: f32,
}

impl Theme {
    /// The default elegance theme: slate palette, elegant typography.
    pub fn slate() -> Self {
        Self {
            palette: Palette::slate(),
            typography: Typography::elegant(),
            control_radius: 6.0,
            card_radius: 10.0,
            card_padding: 18.0,
            control_padding_y: 6.5,
            control_padding_x: 14.0,
        }
    }

    /// The "charcoal" theme: neutral dark-grey palette with a cyan
    /// focus accent. Shares shape and typography with [`Theme::slate`]
    /// so layouts transfer cleanly between the two.
    pub fn charcoal() -> Self {
        Self {
            palette: Palette::charcoal(),
            ..Self::slate()
        }
    }

    /// The "frost" theme: the light-mode counterpart to
    /// [`Theme::slate`]. Shares shape and typography so you can toggle
    /// between the two without any layout shift.
    pub fn frost() -> Self {
        Self {
            palette: Palette::frost(),
            ..Self::slate()
        }
    }

    /// The "paper" theme: the light-mode counterpart to
    /// [`Theme::charcoal`]. Shares shape and typography so you can toggle
    /// between the two without any layout shift.
    pub fn paper() -> Self {
        Self {
            palette: Palette::paper(),
            ..Self::slate()
        }
    }

    /// Install the theme into an [`egui::Context`].
    ///
    /// This writes both of egui's styles (dark and light), so stock widgets
    /// (labels, sliders, scroll bars, etc.) inherit the palette whichever one
    /// egui has active, including after the OS switches dark/light mode. To
    /// follow the OS instead, install a theme picked from [`Context::theme`]
    /// every frame (e.g. [`Theme::frost`] for `Light`, [`Theme::slate`]
    /// otherwise). It also registers the bundled
    /// `Elegance Symbols` font as a lowest-priority Proportional + Monospace
    /// fallback so glyphs like `→ ⌫ ⋯` render out of the box, and stores
    /// the theme in context memory so elegance widgets can read it back.
    ///
    /// Cheap to call every frame: when the incoming theme equals the one
    /// already installed, the style and memory writes are skipped. The
    /// font install is idempotent (by font name) inside egui.
    ///
    /// The font registration uses [`Context::add_font`], which appends to
    /// the existing registry. Host fonts installed via `add_font` — at any
    /// time, before or after `Theme::install` — coexist with the symbols
    /// font. A host call to `ctx.set_fonts(...)` after `Theme::install`
    /// still clobbers the symbols font (and egui's defaults, and anything
    /// else), but that's inherent to `set_fonts` taking over the registry.
    pub fn install(self, ctx: &Context) {
        install_symbols_font(ctx);

        let unchanged = ctx.data(|d| {
            d.get_temp::<Theme>(Self::storage_id())
                .is_some_and(|t| t == self)
        });
        if unchanged {
            return;
        }
        ctx.all_styles_mut(|style| self.apply_to_style(style));
        ctx.data_mut(|d| d.insert_temp(Self::storage_id(), self));
    }

    /// Read the currently-installed theme, or return [`Theme::slate`] if
    /// none has been installed yet.
    pub fn current(ctx: &Context) -> Theme {
        ctx.data(|d| {
            d.get_temp::<Theme>(Self::storage_id())
                .unwrap_or_else(Theme::slate)
        })
    }

    fn storage_id() -> Id {
        Id::unique("elegance::theme")
    }
}

fn install_symbols_font(ctx: &Context) {
    ctx.add_font(FontInsert::new(
        SYMBOLS_FONT_KEY,
        FontData::from_static(SYMBOLS_FONT_BYTES),
        vec![
            InsertFontFamily {
                family: FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: FontFamily::Monospace,
                priority: FontPriority::Lowest,
            },
        ],
    ));
}

impl Theme {
    fn apply_to_style(&self, style: &mut Style) {
        let p = &self.palette;
        let t = &self.typography;

        // Text styles.
        use FontFamily::{Monospace, Proportional};
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::new(t.heading, Proportional));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(t.body, Proportional));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::new(t.button, Proportional));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::new(t.small, Proportional));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::new(t.monospace, Monospace));

        // Spacing.
        let sp = &mut style.spacing;
        sp.item_spacing = Vec2::new(8.0, 6.0);
        sp.button_padding = Vec2::new(self.control_padding_x, self.control_padding_y);
        sp.interact_size = Vec2::new(24.0, 24.0);
        sp.icon_width = 16.0;
        sp.icon_width_inner = 10.0;
        sp.icon_spacing = 6.0;
        sp.combo_width = 120.0;
        sp.text_edit_width = 180.0;
        sp.window_margin = Margin::same(10);
        sp.menu_margin = Margin::same(6);
        sp.indent = 16.0;

        // Interaction. Override after install via
        // `ctx.all_styles_mut(|s| s.interaction.tooltip_delay = ...)` to taste.
        style.interaction.tooltip_delay = 0.35;
        style.interaction.tooltip_grace_time = 0.2;

        // Visuals.
        let v = &mut style.visuals;
        *v = if p.is_dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        v.dark_mode = p.is_dark;
        v.override_text_color = Some(p.text);
        v.panel_fill = p.bg;
        v.window_fill = p.card;
        v.window_stroke = Stroke::new(1.0, p.border);
        v.window_corner_radius = CornerRadius::same(self.card_radius as u8);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = p.input_bg;
        v.faint_bg_color = p.depth_tint(p.card, 0.02);
        v.code_bg_color = p.input_bg;
        v.hyperlink_color = p.focus;
        v.warn_fg_color = p.warning;
        v.error_fg_color = p.danger;
        v.button_frame = true;
        v.striped = false;

        v.selection = Selection {
            bg_fill: with_alpha(p.focus, 70),
            stroke: Stroke::new(1.0, p.focus),
        };

        // Widget visuals: we use these for built-in widgets. Elegance
        // widgets mostly paint themselves, so we keep the stock styling
        // tidy rather than exact.
        let control_radius = CornerRadius::same(self.control_radius as u8);
        v.widgets = Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: p.card,
                weak_bg_fill: p.card,
                bg_stroke: Stroke::new(1.0, p.border),
                corner_radius: control_radius,
                fg_stroke: Stroke::new(1.0, p.text),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: p.input_bg,
                weak_bg_fill: p.input_bg,
                bg_stroke: Stroke::new(1.0, p.border),
                corner_radius: control_radius,
                fg_stroke: Stroke::new(1.0, p.text),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: p.depth_tint(p.input_bg, 0.04),
                weak_bg_fill: p.depth_tint(p.input_bg, 0.04),
                bg_stroke: Stroke::new(1.0, p.text_muted),
                corner_radius: control_radius,
                fg_stroke: Stroke::new(1.5, p.text),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: mix(p.input_bg, p.focus, 0.15),
                weak_bg_fill: mix(p.input_bg, p.focus, 0.15),
                bg_stroke: Stroke::new(1.0, p.focus),
                corner_radius: control_radius,
                fg_stroke: Stroke::new(1.5, p.text),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: p.input_bg,
                weak_bg_fill: p.input_bg,
                bg_stroke: Stroke::new(1.0, p.focus),
                corner_radius: control_radius,
                fg_stroke: Stroke::new(1.0, p.text),
                expansion: 0.0,
            },
        };
    }

    /// Create a [`WidgetText`] coloured with the primary text colour and
    /// sized for body copy.
    pub fn body_text(&self, text: impl Into<String>) -> WidgetText {
        egui::RichText::new(text.into())
            .color(self.palette.text)
            .size(self.typography.body)
            .into()
    }

    /// Create a strong [`WidgetText`] coloured and sized for a heading.
    pub fn heading_text(&self, text: impl Into<String>) -> WidgetText {
        egui::RichText::new(text.into())
            .color(self.palette.text)
            .size(self.typography.heading)
            .strong()
            .into()
    }

    /// Create a [`WidgetText`] coloured with the muted text colour.
    pub fn muted_text(&self, text: impl Into<String>) -> WidgetText {
        egui::RichText::new(text.into())
            .color(self.palette.text_muted)
            .size(self.typography.label)
            .into()
    }

    /// Create a [`WidgetText`] coloured with the faint (tertiary) text colour.
    pub fn faint_text(&self, text: impl Into<String>) -> WidgetText {
        egui::RichText::new(text.into())
            .color(self.palette.text_faint)
            .size(self.typography.small)
            .into()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::slate()
    }
}

/// One of the four built-in elegance themes, as a typed enum.
///
/// Useful as the bound value for [`ThemeSwitcher`](crate::ThemeSwitcher) or
/// anywhere you want to remember a theme choice without stringly-typing it.
/// Marked `#[non_exhaustive]` so future built-in additions won't break
/// exhaustive matches in downstream code.
///
/// ```
/// # use elegance::BuiltInTheme;
/// let choice = BuiltInTheme::Frost;
/// let theme = choice.theme();
/// assert_eq!(choice.label(), "Frost");
/// assert!(!theme.palette.is_dark);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BuiltInTheme {
    /// [`Theme::slate`] — cool dark blue. The default.
    #[default]
    Slate,
    /// [`Theme::charcoal`] — neutral dark grey.
    Charcoal,
    /// [`Theme::frost`] — light counterpart to slate.
    Frost,
    /// [`Theme::paper`] — light counterpart to charcoal.
    Paper,
}

impl BuiltInTheme {
    /// Display label used by [`ThemeSwitcher`](crate::ThemeSwitcher).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Slate => "Slate",
            Self::Charcoal => "Charcoal",
            Self::Frost => "Frost",
            Self::Paper => "Paper",
        }
    }

    /// Resolve to a concrete [`Theme`].
    pub fn theme(self) -> Theme {
        match self {
            Self::Slate => Theme::slate(),
            Self::Charcoal => Theme::charcoal(),
            Self::Frost => Theme::frost(),
            Self::Paper => Theme::paper(),
        }
    }

    /// All four built-in themes in their canonical display order: dark
    /// variants first (Slate, Charcoal), then light (Frost, Paper).
    pub const fn all() -> [BuiltInTheme; 4] {
        [Self::Slate, Self::Charcoal, Self::Frost, Self::Paper]
    }
}

// --- colour utilities ------------------------------------------------------

#[inline]
const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub(crate) fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Baseline position within an egui galley as a fraction of the row
/// height. egui's default proportional fonts place the baseline at
/// roughly this fraction; aligning two galleys' baselines (rather than
/// their bottoms) is what HTML's `align-items: baseline` does and reads
/// noticeably more natural when one glyph run is much smaller than the
/// other.
pub(crate) const BASELINE_FRAC: f32 = 0.78;

/// Run `f` with the `Ui`'s visuals temporarily mutable. Any changes made
/// inside the closure are reverted when it returns, so widgets can paint
/// nested egui primitives with themed visuals without leaking those
/// mutations to sibling widgets.
pub(crate) fn with_themed_visuals<R>(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let saved = ui.visuals().clone();
    let result = f(ui);
    *ui.visuals_mut() = saved;
    result
}

/// Apply the shared "input-like frame" visuals to `v`: every widget state
/// gets the same `bg_fill` / `weak_bg_fill` / `corner_radius`, and each
/// state's border stroke follows the elegance convention
/// (inactive → border, hovered → text_muted, active/open → focus).
///
/// Callers layer their variant-specific tweaks on top — text edits add
/// `extreme_bg_color` + selection colours, selects add per-state
/// `fg_stroke` + `override_text_color`.
pub(crate) fn themed_input_visuals(v: &mut Visuals, theme: &Theme, bg_fill: Color32) {
    let p = &theme.palette;
    let radius = CornerRadius::same(theme.control_radius as u8);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_fill = bg_fill;
        w.weak_bg_fill = bg_fill;
        w.corner_radius = radius;
        // egui defaults hovered/active expansion to 1.0 (widgets "pop" outward on
        // hover). That's fine for buttons but reads as jitter on text inputs —
        // the border visibly jumps on every mouse hover, and any overlaid marker
        // (e.g. the dirty bar) has to jitter with it. Keep inputs frame-stable.
        w.expansion = 0.0;
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.text_muted);
    v.widgets.active.bg_stroke = Stroke::new(1.5, p.focus);
    v.widgets.open.bg_stroke = Stroke::new(1.5, p.focus);
}

/// Lay out `text` as a proportional-font galley with `Color32::PLACEHOLDER`
/// baked in. The placeholder colour lets `painter.galley(..., fallback_color)`
/// actually control the rendered colour — otherwise `WidgetText::into_galley`
/// bakes `visuals.override_text_color` (or `strong_text_color` when `strong`
/// is set) into the galley and silently overrides the fallback.
pub(crate) fn placeholder_galley(
    ui: &egui::Ui,
    text: &str,
    font_size: f32,
    strong: bool,
    wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut rt = egui::RichText::new(text)
        .size(font_size)
        .color(Color32::PLACEHOLDER);
    if strong {
        rt = rt.strong();
    }
    egui::WidgetText::from(rt).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        wrap_width,
        egui::FontSelection::FontId(egui::FontId::proportional(font_size)),
    )
}

/// Linear mix between `a` and `b`; `t = 0.0` returns `a`, `t = 1.0` returns `b`.
pub(crate) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 {
        let xf = x as f32;
        let yf = y as f32;
        (xf + (yf - xf) * t).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a().max(1), b.a().max(1)),
    )
}
