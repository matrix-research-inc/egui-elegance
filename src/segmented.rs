//! A "segmented" toggle button with an LED dot.

use egui::{
    Color32, CornerRadius, Response, Role, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo, WidgetText,
};

use crate::button::ButtonSize;
use crate::theme::{Accent, Theme, with_alpha};

/// A toggle button with a built-in LED dot.
///
/// `SegmentedButton` shares [`ButtonSize`] with [`Button`](crate::Button), so a
/// mixed action row stays aligned when both widgets pass matching
/// `.size(...)` values.
///
/// ```no_run
/// # use elegance::{Accent, Button, ButtonSize, SegmentedButton};
/// # egui::__run_test_ui(|ui| {
/// let mut on = false;
/// // Default: ButtonSize::Medium — aligns with Medium Button.
/// ui.add(SegmentedButton::new(&mut on, "Continuous").accent(Accent::Green));
///
/// // Chunkier row — Button + SegmentedButton, same height.
/// ui.horizontal(|ui| {
///     ui.add(Button::new("Collect").size(ButtonSize::Large));
///     ui.add(
///         SegmentedButton::new(&mut on, "Continuous")
///             .accent(Accent::Green)
///             .size(ButtonSize::Large),
///     );
/// });
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct SegmentedButton<'a> {
    on: &'a mut bool,
    label: WidgetText,
    accent: Accent,
    size: ButtonSize,
    /// When `true`, the `on` state is dimmed — useful for showing that a
    /// linked toggle or prerequisite isn't active.
    dim_when_on: bool,
    rounded: bool,
    corner_radius: Option<CornerRadius>,
    min_width: Option<f32>,
}

impl<'a> std::fmt::Debug for SegmentedButton<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentedButton")
            .field("on", &*self.on)
            .field("label", &self.label.text())
            .field("accent", &self.accent)
            .field("size", &self.size)
            .field("dim_when_on", &self.dim_when_on)
            .field("rounded", &self.rounded)
            .field("corner_radius", &self.corner_radius)
            .field("min_width", &self.min_width)
            .finish()
    }
}

impl<'a> SegmentedButton<'a> {
    /// Create a segmented button bound to `on` with the given label.
    pub fn new(on: &'a mut bool, label: impl Into<WidgetText>) -> Self {
        Self {
            on,
            label: label.into(),
            accent: Accent::Green,
            size: ButtonSize::Medium,
            dim_when_on: false,
            rounded: true,
            corner_radius: None,
            min_width: None,
        }
    }

    /// Pick the `on`-state colour from one of the theme's accents. Default: [`Accent::Green`].
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Pick a size preset matching [`Button`](crate::Button)'s sizes so a
    /// mixed `Button` + `SegmentedButton` row stays aligned at any size.
    /// Default: [`ButtonSize::Medium`].
    #[inline]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// When the button is on, render its fill dimmed and the label muted.
    /// Used to indicate "enabled but not currently applicable".
    pub fn dim_when_on(mut self, dim: bool) -> Self {
        self.dim_when_on = dim;
        self
    }

    /// Set whether the button has rounded corners. Disable for segmented
    /// groups where neighbours share edges.
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Explicitly set the corner radius (per-corner). Overrides [`Self::rounded`].
    /// Useful for segmented strips where only the end cells should be rounded.
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = Some(radius.into());
        self
    }

    /// Force the button to occupy at least this width. When wider than
    /// the LED + text, the content is centered horizontally.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    fn on_fill(&self, theme: &Theme) -> Color32 {
        theme.palette.accent_fill(self.accent)
    }

    fn on_fill_hover(&self, theme: &Theme) -> Color32 {
        theme.palette.accent_hover(self.accent)
    }
}

impl<'a> Widget for SegmentedButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        let padding = self.size.padding(&theme);
        let font_size = self.size.font_size(&theme);
        let led_size = 8.0;
        let led_gap = 7.0;

        let galley =
            crate::theme::placeholder_galley(ui, self.label.text(), font_size, true, f32::INFINITY);

        let content_w = led_size + led_gap + galley.size().x;
        let mut desired = Vec2::new(
            padding.x * 2.0 + content_w,
            (padding.y * 2.0 + galley.size().y).max(font_size + 2.0 * padding.y),
        );
        if let Some(min_w) = self.min_width {
            desired.x = desired.x.max(min_w);
        }
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let on = *self.on;
            let hovered = response.hovered();
            let is_down = response.is_pointer_button_down_on();

            let (fill, text_color, led_color, led_glow) = if on {
                let mut fill = if is_down {
                    crate::theme::mix(self.on_fill_hover(&theme), Color32::BLACK, 0.1)
                } else if hovered {
                    self.on_fill_hover(&theme)
                } else {
                    self.on_fill(&theme)
                };
                let mut text = Color32::WHITE;
                if self.dim_when_on {
                    fill = crate::theme::mix(fill, p.card, 0.55);
                    text = p.text_muted;
                }
                let led = Color32::WHITE;
                let glow = !self.dim_when_on;
                (fill, text, led, glow)
            } else {
                let fill = if hovered {
                    p.depth_tint(p.input_bg, 0.05)
                } else {
                    p.input_bg
                };
                let text = if hovered { p.text_muted } else { p.text_faint };
                let led = p.text_faint;
                (fill, text, led, false)
            };

            let radius = self.corner_radius.unwrap_or_else(|| {
                if self.rounded {
                    CornerRadius::same(theme.control_radius as u8 + 2)
                } else {
                    CornerRadius::ZERO
                }
            });
            ui.painter()
                .rect(rect, radius, fill, Stroke::NONE, egui::StrokeKind::Inside);

            // Center the LED + text combo within the allocated rect.
            let content_start = rect.center().x - content_w * 0.5;
            let led_center = egui::pos2(content_start + led_size * 0.5, rect.center().y);
            if led_glow {
                ui.painter().circle_filled(
                    led_center,
                    led_size * 0.5 + 2.0,
                    with_alpha(Color32::WHITE, 70),
                );
            }
            ui.painter()
                .circle_filled(led_center, led_size * 0.5, led_color);

            let text_pos = egui::pos2(
                led_center.x + led_size * 0.5 + led_gap,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, text_color);
        }

        response.widget_info(|| {
            WidgetInfo::selected(Role::CheckBox, true, *self.on, self.label.text())
        });
        response
    }
}
