//! Determinate progress bar with an in-bar percent label.
//!
//! The bar is a rounded pill: a dim track plus an accent-coloured fill
//! that grows from the left. Text inside the bar renders in two colours
//! — bright over the filled region, muted over the empty region — so the
//! label stays legible regardless of the fill level.

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, Widget,
    WidgetInfo,
};

use crate::theme::{Accent, Theme};

/// A horizontal determinate progress bar.
///
/// ```no_run
/// # use elegance::{Accent, ProgressBar};
/// # egui::__run_test_ui(|ui| {
/// ui.add(ProgressBar::new(0.42));
/// ui.add(ProgressBar::new(0.8).accent(Accent::Green).text("Uploading…"));
/// # });
/// ```
#[derive(Debug, Clone)]
#[must_use = "Add with `ui.add(...)`."]
pub struct ProgressBar {
    fraction: f32,
    height: f32,
    desired_width: Option<f32>,
    color: Option<Color32>,
    accent: Option<Accent>,
    text: Option<String>,
}

impl ProgressBar {
    /// Create a progress bar at `fraction` (0..=1). NaN and out-of-range
    /// values are clamped.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: if fraction.is_nan() {
                0.0
            } else {
                fraction.clamp(0.0, 1.0)
            },
            height: 22.0,
            desired_width: None,
            color: None,
            accent: None,
            text: None,
        }
    }

    /// Bar height in points. Default: 22.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Override the bar width. Defaults to `ui.available_width()`.
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Paint the fill with an explicit colour. Clears any previously set accent.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self.accent = None;
        self
    }

    /// Pick the fill colour from one of the theme's accents. Clears any
    /// previously set explicit colour.
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = Some(accent);
        self.color = None;
        self
    }

    /// Override the in-bar text. By default the bar shows the rounded
    /// percent (e.g. "42%"); passing `""` hides the text entirely.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

impl Widget for ProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let fill_color = match (self.color, self.accent) {
            (Some(c), _) => c,
            (_, Some(a)) => p.accent_fill(a),
            _ => p.focus,
        };

        let width = self
            .desired_width
            .unwrap_or_else(|| ui.available_width())
            .max(self.height * 2.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let radius = CornerRadius::same((self.height * 0.5).round() as u8);

            // Track: input-bg with a subtle border.
            painter.rect(
                rect,
                radius,
                p.input_bg,
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );

            // Fill clipped from the left.
            let fill_w = rect.width() * self.fraction;
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
            if fill_w > 0.5 {
                painter
                    .with_clip_rect(fill_rect)
                    .rect_filled(rect, radius, fill_color);
            }

            // Label.
            let label = match self.text.as_deref() {
                Some(s) => s.to_owned(),
                None => format!("{}%", (self.fraction * 100.0).round() as u32),
            };
            if !label.is_empty() {
                let font_size = (self.height * 0.55).max(11.0);
                let galley =
                    crate::theme::placeholder_galley(ui, &label, font_size, true, f32::INFINITY);
                let text_pos = Pos2::new(
                    rect.center().x - galley.size().x * 0.5,
                    rect.center().y - galley.size().y * 0.5,
                );

                // Muted pass over the empty region.
                let empty_rect =
                    Rect::from_min_max(Pos2::new(rect.min.x + fill_w, rect.min.y), rect.max);
                painter
                    .with_clip_rect(empty_rect)
                    .galley(text_pos, galley.clone(), p.text_muted);

                // Bright pass over the filled region.
                painter
                    .with_clip_rect(fill_rect)
                    .galley(text_pos, galley, Color32::WHITE);
            }
        }

        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "progress"));
        response
    }
}
