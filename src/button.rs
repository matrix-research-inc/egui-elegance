//! Buttons in the elegance style.
//!
//! A [`Button`] is a chunky, rounded rectangle with a coloured fill, bold
//! text, and smooth hover/press transitions. Six accent colours are
//! available: Blue, Green, Red, Purple, Amber, and Sky. For secondary
//! actions, [`Button::outline`] gives a transparent, bordered treatment,
//! neutral unless given an accent.
//! [`Button::icon`] renders one of the [`glyphs`](crate::glyphs) as a square
//! icon-only button, and [`Button::icon_text`] puts one beside the label.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Response, Role, Sense, Shape, Stroke, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, pos2, vec2,
};

use crate::theme::{Accent, Theme, mix, with_alpha};

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

/// Glyph size beside a label, as a multiple of the label's font size. A Lucide
/// glyph inks well inside its em box, so at the label's own size it reads
/// smaller than the text; this scale inks it at about the label's cap height.
const LEADING_GLYPH_SCALE: f32 = 1.15;

/// Gap between a leading glyph's ink and the label, as a multiple of the
/// label's font size, so the spacing tracks the [`ButtonSize`] preset.
const LEADING_GLYPH_GAP: f32 = 0.45;

/// What a [`Button`] paints. Whatever is painted, the button's `text` is its
/// accessible name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Content {
    /// The label alone.
    Text,
    /// A lone glyph in a square button; the label is not painted.
    Icon(char),
    /// A glyph followed by the label.
    IconText(char),
}

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
    /// The accessible name, and the painted label unless `content` is
    /// [`Content::Icon`].
    text: WidgetText,
    content: Content,
    /// `None` is Blue when filled and neutral when outlined.
    accent: Option<Accent>,
    size: ButtonSize,
    outline: bool,
    min_width: Option<f32>,
    full_width: bool,
    enabled: bool,
    /// Set by [`Button::disabled_reason`]; `Some` disables the button.
    disabled_reason: Option<String>,
    loading: bool,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("content", &self.content)
            .field("accent", &self.accent)
            .field("size", &self.size)
            .field("outline", &self.outline)
            .field("min_width", &self.min_width)
            .field("full_width", &self.full_width)
            .field("enabled", &self.enabled)
            .field("disabled_reason", &self.disabled_reason)
            .field("loading", &self.loading)
            .finish()
    }
}

impl Button {
    /// Create a new button. Defaults to medium size and, when filled, the
    /// Blue accent.
    ///
    /// Only the string content of `text` is used: the label is laid out at the
    /// size preset's font size in the button's own state colour, so a
    /// [`RichText`](egui::RichText)'s size, colour, and style are ignored. To
    /// enlarge a glyph, use [`Button::icon`] rather than sizing the text.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            content: Content::Text,
            accent: None,
            size: ButtonSize::Medium,
            outline: false,
            min_width: None,
            full_width: false,
            enabled: true,
            disabled_reason: None,
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
            content: Content::Icon(glyph),
            ..Self::new(label)
        }
    }

    /// Create a button showing one [`glyph`](crate::glyphs) followed by a
    /// label.
    ///
    /// The button is exactly as tall as a text button of the same
    /// [`ButtonSize`], so it lines up with text and icon-only buttons in a
    /// row. The glyph is drawn slightly larger than the label so the two read
    /// at the same visual weight, and only the label reaches the
    /// accessibility tree. As with [`Button::new`], only the string content of
    /// `label` is used.
    ///
    /// ```no_run
    /// # use elegance::{Accent, Button, glyphs};
    /// # egui::__run_test_ui(|ui| {
    /// ui.add(Button::icon_text(glyphs::DOWNLOAD, "Download").accent(Accent::Blue));
    /// # });
    /// ```
    pub fn icon_text(glyph: char, label: impl Into<WidgetText>) -> Self {
        Self {
            content: Content::IconText(glyph),
            ..Self::new(label)
        }
    }

    /// Pick the button accent colour. On an [`outline`](Self::outline)
    /// button it tints the border and label, e.g. red for a risky secondary
    /// action:
    ///
    /// ```no_run
    /// # use elegance::{Accent, Button};
    /// # egui::__run_test_ui(|ui| {
    /// ui.add(Button::new("Continue anyway").outline().accent(Accent::Red));
    /// # });
    /// ```
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Render the button as a transparent, bordered "ghost" treatment for
    /// secondary actions. Neutral grey unless given an
    /// [`accent`](Self::accent).
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
    ///
    /// To also tell the user why, use [`Button::disabled_reason`].
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable the button and say why. `Some(reason)` disables it and shows
    /// `reason` as a tooltip on hover and as its accessible description.
    /// `None` leaves the button as it is.
    ///
    /// The reason doubles as the disabled state, so the two cannot drift
    /// apart the way a separate [`enabled`](Self::enabled) condition and
    /// tooltip text can:
    ///
    /// ```no_run
    /// # use elegance::Button;
    /// # egui::__run_test_ui(|ui| {
    /// # let has_target = false;
    /// let reason = (!has_target).then_some("Pick a target first");
    /// ui.add(Button::new("Deploy").disabled_reason(reason));
    /// # });
    /// ```
    ///
    /// Combines with [`enabled`](Self::enabled): the button is disabled if
    /// either disables it, and the tooltip shows only when a reason is given.
    #[inline]
    pub fn disabled_reason<S: Into<String>>(mut self, reason: Option<S>) -> Self {
        self.disabled_reason = reason.map(Into::into);
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

        // The label row's height at this size preset: every arm below is at
        // least this tall, so a row can mix them without stepping.
        let min_height = font_size + 2.0 * padding.y;
        let layout = match self.content {
            // Square, and exactly the height the text arm produces at this
            // size preset.
            Content::Icon(glyph) => {
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
                Layout {
                    glyph: Some(galley),
                    label: None,
                    gap: 0.0,
                    desired: Vec2::splat(side),
                }
            }
            Content::Text => {
                let wrap_width = (ui.available_width() - 2.0 * padding.x).max(0.0);
                let label = crate::theme::placeholder_galley(
                    ui,
                    self.text.text(),
                    font_size,
                    false,
                    wrap_width,
                );
                let mut desired = label.size() + 2.0 * padding;
                desired.y = desired.y.max(min_height);
                Layout {
                    glyph: None,
                    label: Some(label),
                    gap: 0.0,
                    desired,
                }
            }
            Content::IconText(glyph) => {
                let glyph = crate::theme::placeholder_galley(
                    ui,
                    &glyph.to_string(),
                    font_size * LEADING_GLYPH_SCALE,
                    false,
                    f32::INFINITY,
                );
                let gap = font_size * LEADING_GLYPH_GAP;
                let glyph_ink = ink_size(&glyph);
                let wrap_width =
                    (ui.available_width() - 2.0 * padding.x - glyph_ink.x - gap).max(0.0);
                let label = crate::theme::placeholder_galley(
                    ui,
                    self.text.text(),
                    font_size,
                    false,
                    wrap_width,
                );
                let desired = vec2(
                    glyph_ink.x + gap + label.size().x + 2.0 * padding.x,
                    (label.size().y.max(glyph_ink.y) + 2.0 * padding.y).max(min_height),
                );
                Layout {
                    glyph: Some(glyph),
                    label: Some(label),
                    gap,
                    desired,
                }
            }
        };
        let mut desired = layout.desired;
        if let Some(min_w) = self.min_width {
            desired.x = desired.x.max(min_w);
        }
        if self.full_width {
            desired.x = ui.available_width().max(desired.x);
        }

        let enabled = self.enabled && self.disabled_reason.is_none();
        let interactive = enabled && !self.loading;
        let sense = if interactive {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = crate::enabled::allocate_exact_size(ui, desired, sense, enabled);

        let visible = ui.is_rect_visible(rect);
        if visible {
            // Work out fill and text colour for the current state.
            let (fill, stroke, text_color) = resolve_colors(
                &theme,
                self.accent,
                self.outline,
                enabled,
                self.loading,
                &response,
            );

            let radius = CornerRadius::same(theme.control_radius as u8);
            ui.painter()
                .rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);

            if self.loading {
                paint_barber_pole(ui, rect, &theme, with_alpha(text_color, 32));
            }

            // A label centres on its layout box, which lines its baseline up
            // with neighbouring text. A glyph centres on its ink, which omits
            // the font's unused ascent/descent and uneven side bearings. The
            // glyph-and-label pair is centred as one block.
            let glyph_w = layout.glyph.as_ref().map_or(0.0, |g| ink_size(g).x);
            let label_w = layout.label.as_ref().map_or(0.0, |g| g.size().x);
            let left = rect.center().x - (glyph_w + layout.gap + label_w) * 0.5;
            if let Some(glyph) = layout.glyph {
                let ink = glyph.mesh_bounds;
                let pos = if ink.is_positive() {
                    pos2(left - ink.min.x, rect.center().y - ink.center().y)
                } else {
                    pos2(left, rect.center().y - glyph.size().y * 0.5)
                };
                ui.painter().galley(pos, glyph, text_color);
            }
            if let Some(label) = layout.label {
                let pos = pos2(
                    left + glyph_w + layout.gap,
                    rect.center().y - label.size().y * 0.5,
                );
                ui.painter().galley(pos, label, text_color);
            }
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Button, interactive, self.text.text()));
        match self.disabled_reason {
            Some(reason) => explain_disabled(response, reason),
            None => response,
        }
    }
}

/// Show why a disabled button is disabled: as its tooltip and as its
/// accessible description.
fn explain_disabled(response: Response, reason: String) -> Response {
    // `on_disabled_hover_text`, unlike `on_hover_text`, leaves the accessible
    // description unset, so set it here.
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_description(reason.clone());
    });
    response.on_disabled_hover_text(reason)
}

/// The laid-out content of a [`Button`], before it is allocated.
struct Layout {
    glyph: Option<Arc<Galley>>,
    label: Option<Arc<Galley>>,
    /// Space between the glyph's ink and the label.
    gap: f32,
    desired: Vec2,
}

/// The size of a glyph's ink, falling back to its layout box when the font
/// has no outline for it (the glyph then renders as nothing or a box).
fn ink_size(galley: &Galley) -> Vec2 {
    let ink = galley.mesh_bounds;
    if ink.is_positive() {
        ink.size()
    } else {
        galley.size()
    }
}

/// Overlay diagonal "barber pole" stripes drifting across the button, between
/// the base fill and the text. Inset by a fraction of the corner radius so
/// the stripes never bleed into the rounded corner cutouts. Pass a faint
/// label colour, which reads on filled and outline buttons alike.
fn paint_barber_pole(ui: &Ui, rect: egui::Rect, theme: &Theme, stripe_color: Color32) {
    crate::request_repaint_at_rate(ui.ctx(), 30.0);

    // Inset enough that a square-clip never overlaps the rounded corner area.
    // For a corner radius `r`, the corner triangle outside the arc has a
    // diagonal depth of `r * (1 - 1/sqrt(2)) ≈ 0.293r`; round up.
    let inset = (theme.control_radius * 0.32).max(1.0);
    let stripe_rect = rect.shrink(inset);
    if stripe_rect.width() <= 0.0 || stripe_rect.height() <= 0.0 {
        return;
    }

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
    accent: Option<Accent>,
    outline: bool,
    enabled: bool,
    loading: bool,
    response: &Response,
) -> (Color32, Stroke, Color32) {
    let p = &theme.palette;
    let is_down = !loading && response.is_pointer_button_down_on();
    let is_hovered = !loading && response.hovered();
    // A wash over the background: pressed, hovered, or none.
    let wash = |color: Color32, hover: u8, down: u8| {
        if is_down {
            with_alpha(color, down)
        } else if is_hovered {
            with_alpha(color, hover)
        } else {
            Color32::TRANSPARENT
        }
    };

    match (outline, accent) {
        (true, _) if !enabled => (
            Color32::TRANSPARENT,
            Stroke::new(1.0, p.border),
            mix(p.text_muted, p.card, 0.4),
        ),
        (true, Some(accent)) => {
            let tint = p.accent_fill(accent);
            let border = if is_hovered {
                tint
            } else {
                with_alpha(tint, 210)
            };
            // The accent eased toward the text colour, so the label reads on
            // the page in light and dark themes alike.
            (
                wash(tint, 24, 40),
                Stroke::new(1.0, border),
                mix(tint, p.text, 0.4),
            )
        }
        (true, None) => {
            let (border, text) = if is_hovered {
                (p.text_muted, p.text)
            } else {
                (p.border, p.text_muted)
            };
            (wash(p.text_muted, 20, 30), Stroke::new(1.0, border), text)
        }
        (false, accent) => {
            let accent = accent.unwrap_or(Accent::Blue);
            if !enabled {
                return (
                    mix(p.accent_fill(accent), p.card, 0.55),
                    Stroke::NONE,
                    mix(p.text, p.card, 0.4),
                );
            }

            let hover = p.accent_hover(accent);
            let fill = if is_down {
                // Slightly darker than the hover colour for a satisfying click.
                mix(hover, Color32::BLACK, 0.08)
            } else if is_hovered {
                hover
            } else {
                p.accent_fill(accent)
            };
            // Outline buttons get no ring: modals and drawers focus their
            // close button on open, and egui can't tell that from keyboard
            // focus.
            let stroke = if response.has_focus() {
                Stroke::new(2.0, with_alpha(p.focus, 180))
            } else {
                Stroke::NONE
            };
            (fill, stroke, Color32::WHITE)
        }
    }
}
