//! Buttons in the elegance style.
//!
//! A [`Button`] is a chunky, rounded rectangle with a coloured fill, bold
//! text, and smooth hover/press transitions. Six accent colours are
//! available: Blue, Green, Red, Purple, Amber, and Sky. For secondary
//! actions, [`Button::outline`] gives a transparent, bordered treatment.
//! [`Button::icon`] renders one of the [`glyphs`](crate::glyphs) as a square
//! icon-only button.

use egui::{
    Color32, CornerRadius, Response, Role, Sense, Shape, Stroke, Ui, Vec2, Widget, WidgetInfo,
    WidgetText, pos2, vec2,
};

use crate::theme::{Accent, Theme, mix};

/// Size presets for buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    /// Compact — tight padding, the small typography size.
    Small,
    /// The default button size.
    Medium,
    /// Chunky — extra padding and a slightly larger font.
    Large,
}

impl ButtonSize {
    /// Resolve the `(pad_x, pad_y)` padding for a given size against the
    /// active theme. Used by [`Button`] and [`SegmentedButton`](crate::SegmentedButton)
    /// so both widgets produce identical control heights at a given size.
    pub fn padding(self, theme: &Theme) -> Vec2 {
        match self {
            ButtonSize::Small => vec2(theme.control_padding_x * 0.6, theme.control_padding_y * 0.6),
            ButtonSize::Medium => vec2(theme.control_padding_x, theme.control_padding_y),
            ButtonSize::Large => vec2(
                theme.control_padding_x * 1.25,
                theme.control_padding_y * 1.2,
            ),
        }
    }

    /// Resolve the label font size for a given size against the active theme.
    pub fn font_size(self, theme: &Theme) -> f32 {
        match self {
            ButtonSize::Small => theme.typography.small,
            ButtonSize::Medium => theme.typography.button,
            ButtonSize::Large => theme.typography.body + 1.0,
        }
    }
}

/// Glyph size for an icon-only button, as a fraction of the button's side.
/// Lucide glyphs ink about three quarters of their em box, so this draws the
/// icon at roughly half the button's width, the usual icon-button proportion.
const ICON_GLYPH_FRACTION: f32 = 0.68;

/// A coloured, rounded button.
///
/// ```no_run
/// # use elegance::{Button, Accent};
/// # egui::__run_test_ui(|ui| {
/// if ui.add(Button::new("Save").accent(Accent::Green)).clicked() {
///     // ...
/// }
/// # });
/// ```
#[must_use = "Call `ui.add(...)` to render the button."]
pub struct Button {
    /// Painted for a text button; for an icon button this is the accessible
    /// name and `icon` is what gets painted.
    text: WidgetText,
    icon: Option<char>,
    accent: Accent,
    size: ButtonSize,
    outline: bool,
    min_width: Option<f32>,
    full_width: bool,
    enabled: bool,
    loading: bool,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("icon", &self.icon)
            .field("accent", &self.accent)
            .field("size", &self.size)
            .field("outline", &self.outline)
            .field("min_width", &self.min_width)
            .field("full_width", &self.full_width)
            .field("enabled", &self.enabled)
            .field("loading", &self.loading)
            .finish()
    }
}

impl Button {
    /// Create a new button. Defaults to the Blue accent and medium size.
    ///
    /// Only the string content of `text` is used: the label is laid out at the
    /// size preset's font size in the button's own state colour, so a
    /// [`RichText`](egui::RichText)'s size, colour, and style are ignored. To
    /// enlarge a glyph, use [`Button::icon`] rather than sizing the text.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            accent: Accent::Blue,
            size: ButtonSize::Medium,
            outline: false,
            min_width: None,
            full_width: false,
            enabled: true,
            loading: false,
        }
    }

    /// Create a square icon-only button showing one [`glyph`](crate::glyphs).
    ///
    /// The button is as tall as a text button of the same [`ButtonSize`], so
    /// the two line up in a row, and the glyph is sized off that box rather
    /// than off the label preset, which inks under a third of the button.
    ///
    /// `label` is not painted. It is the accessible name, so pass what the
    /// button *does* ("Download", "Delete run"): an icon alone announces
    /// nothing, and every other builder method still applies.
    ///
    /// ```no_run
    /// # use elegance::{Button, glyphs};
    /// # egui::__run_test_ui(|ui| {
    /// ui.add(Button::icon(glyphs::DOWNLOAD, "Download capture"));
    /// # });
    /// ```
    pub fn icon(glyph: char, label: impl Into<WidgetText>) -> Self {
        Self {
            icon: Some(glyph),
            ..Self::new(label)
        }
    }

    /// Pick the button accent colour. Ignored when the button is set to
    /// [`Button::outline`], which has no fill colour of its own.
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Render the button as a transparent, bordered "ghost" treatment for
    /// secondary actions.
    #[inline]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Pick a size preset.
    #[inline]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set a minimum width (in points). Useful to line up button groups.
    #[inline]
    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = Some(w);
        self
    }

    /// Stretch to fill the available horizontal space.
    #[inline]
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Disable the button.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show a barber-pole busy animation and suppress clicks while it runs.
    /// Independent of [`Button::enabled`]: combine the two to get the
    /// dimmed disabled fill *and* the moving stripes at the same time.
    #[inline]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    fn padding(&self, theme: &Theme) -> Vec2 {
        self.size.padding(theme)
    }

    fn font_size(&self, theme: &Theme) -> f32 {
        self.size.font_size(theme)
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let padding = self.padding(&theme);
        let font_size = self.font_size(&theme);

        let (galley, mut desired) = match self.icon {
            // Square, and exactly the height the text arm produces at this
            // size preset, so a row can mix the two without stepping.
            Some(glyph) => {
                let row = ui
                    .ctx()
                    .fonts_mut(|f| f.row_height(&egui::FontId::proportional(font_size)));
                let side = row.max(font_size) + 2.0 * padding.y;
                let galley = crate::theme::placeholder_galley(
                    ui,
                    &glyph.to_string(),
                    side * ICON_GLYPH_FRACTION,
                    false,
                    f32::INFINITY,
                );
                (galley, Vec2::splat(side))
            }
            None => {
                let wrap_width = (ui.available_width() - 2.0 * padding.x).max(0.0);
                let galley = crate::theme::placeholder_galley(
                    ui,
                    self.text.text(),
                    font_size,
                    false,
                    wrap_width,
                );
                let mut desired = galley.size() + 2.0 * padding;
                desired.y = desired.y.max(font_size + 2.0 * padding.y);
                (galley, desired)
            }
        };
        if let Some(min_w) = self.min_width {
            desired.x = desired.x.max(min_w);
        }
        if self.full_width {
            desired.x = ui.available_width().max(desired.x);
        }

        let interactive = self.enabled && !self.loading;
        let sense = if interactive {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(desired, sense);

        let visible = ui.is_rect_visible(rect);
        if visible {
            // Work out fill and text colour for the current state.
            let (fill, stroke, text_color) = resolve_colors(
                &theme,
                self.accent,
                self.outline,
                self.enabled,
                self.loading,
                &response,
            );

            let radius = CornerRadius::same(theme.control_radius as u8);
            ui.painter()
                .rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);

            if self.loading {
                paint_barber_pole(ui, rect, &theme, self.outline);
            }

            // A label centres on its layout box, which lines its baseline up
            // with neighbouring text. A lone glyph centres on its ink, which
            // omits the font's unused ascent/descent and uneven side bearings.
            let ink = galley.mesh_bounds;
            let offset = if self.icon.is_some() && ink.is_positive() {
                ink.center().to_vec2()
            } else {
                galley.size() * 0.5
            };
            ui.painter()
                .galley(rect.center() - offset, galley, text_color);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Button, interactive, self.text.text()));
        response
    }
}

/// Overlay diagonal "barber pole" stripes drifting across the button, between
/// the base fill and the text. Inset by a fraction of the corner radius so
/// the stripes never bleed into the rounded corner cutouts.
fn paint_barber_pole(ui: &Ui, rect: egui::Rect, theme: &Theme, outline: bool) {
    crate::request_repaint_at_rate(ui.ctx(), 30.0);

    // Inset enough that a square-clip never overlaps the rounded corner area.
    // For a corner radius `r`, the corner triangle outside the arc has a
    // diagonal depth of `r * (1 - 1/sqrt(2)) ≈ 0.293r`; round up.
    let inset = (theme.control_radius * 0.32).max(1.0);
    let stripe_rect = rect.shrink(inset);
    if stripe_rect.width() <= 0.0 || stripe_rect.height() <= 0.0 {
        return;
    }

    // White stripes read on saturated fills; for outline buttons the fill is
    // transparent, so use the theme's text colour against the page background.
    let stripe_color = if outline {
        crate::theme::with_alpha(theme.palette.text, 32)
    } else {
        crate::theme::with_alpha(Color32::WHITE, 32)
    };

    // Geometry scales with the visible height so Small / Medium / Large all
    // read with the same striped-vs-fill ratio. The 0.85 factor lands a Medium
    // button (~26 pt inset height) at ~22 pt period, matching the original
    // pixel mockup. Speed is one full period per ~700 ms regardless of size.
    let stripe_period = (stripe_rect.height() * 0.85).max(8.0);
    let stripe_width = stripe_period * 0.5;
    let speed = stripe_period / 0.7;

    let time = ui.input(|i| i.time) as f32;
    let offset = (time * speed).rem_euclid(stripe_period);

    let h = stripe_rect.height();
    let num_stripes = ((stripe_rect.width() + h) / stripe_period).ceil() as i32 + 2;
    let start_x = stripe_rect.left() - h + offset;

    let painter = ui.painter().with_clip_rect(stripe_rect);
    for i in 0..num_stripes {
        let x = start_x + (i as f32) * stripe_period;
        let pts = vec![
            pos2(x, stripe_rect.top()),
            pos2(x + stripe_width, stripe_rect.top()),
            pos2(x + stripe_width - h, stripe_rect.bottom()),
            pos2(x - h, stripe_rect.bottom()),
        ];
        painter.add(Shape::convex_polygon(pts, stripe_color, Stroke::NONE));
    }
}

fn resolve_colors(
    theme: &Theme,
    accent: Accent,
    outline: bool,
    enabled: bool,
    loading: bool,
    response: &Response,
) -> (Color32, Stroke, Color32) {
    let p = &theme.palette;
    if !enabled {
        if outline {
            return (
                Color32::TRANSPARENT,
                Stroke::new(1.0, p.border),
                mix(p.text_muted, p.card, 0.4),
            );
        }
        return (
            mix(p.accent_fill(accent), p.card, 0.55),
            Stroke::NONE,
            mix(p.text, p.card, 0.4),
        );
    }
    let is_down = !loading && response.is_pointer_button_down_on();
    let is_hovered = !loading && response.hovered();

    if outline {
        let text = if is_hovered { p.text } else { p.text_muted };
        let stroke_color = if is_hovered { p.text_muted } else { p.border };
        let fill = if is_down {
            with_alpha(p.text_muted, 30)
        } else if is_hovered {
            with_alpha(p.text_muted, 20)
        } else {
            Color32::TRANSPARENT
        };
        return (fill, Stroke::new(1.0, stroke_color), text);
    }

    let resting = p.accent_fill(accent);
    let hover = p.accent_hover(accent);
    let fill = if is_down {
        // Slightly darker than the resting hover colour for a satisfying click.
        mix(hover, Color32::BLACK, 0.08)
    } else if is_hovered {
        hover
    } else {
        resting
    };
    let stroke = if response.has_focus() {
        Stroke::new(2.0, with_alpha(p.focus, 180))
    } else {
        Stroke::NONE
    };
    (fill, stroke, Color32::WHITE)
}

fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    crate::theme::with_alpha(c, alpha)
}
