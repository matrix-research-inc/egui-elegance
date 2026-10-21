//! Animated loading spinner — a sweeping arc inside a faint track ring.
//!
//! The arc rotates steadily while its sweep length "breathes" between a
//! short tick and a three-quarter circle, giving a Material Design-like
//! loading animation. The spinner requests continuous repaints while
//! visible so the animation keeps running.

use std::f32::consts::TAU;

use egui::{
    Color32, Response, Role, Sense, Ui, Vec2, Widget, WidgetInfo,
    epaint::{PathShape, PathStroke},
    pos2,
};

use crate::theme::{Accent, Theme, with_alpha};

/// A themed loading spinner.
///
/// ```no_run
/// # use elegance::{Accent, Spinner};
/// # egui::__run_test_ui(|ui| {
/// ui.add(Spinner::new());
/// ui.add(Spinner::new().size(28.0).accent(Accent::Green));
/// # });
/// ```
#[derive(Debug, Clone, Copy)]
#[must_use = "Add with `ui.add(...)`."]
pub struct Spinner {
    size: f32,
    thickness: Option<f32>,
    color: Option<Color32>,
    accent: Option<Accent>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// Create a spinner with the default size (18 pt) and the theme's focus accent.
    pub fn new() -> Self {
        Self {
            size: 18.0,
            thickness: None,
            color: None,
            accent: None,
        }
    }

    /// Diameter of the spinner in points. Default: 18.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Stroke thickness of the arc. Defaults to ~12 % of `size` (min 1.5 pt).
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Paint the arc with an explicit colour. Clears any previously set accent.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self.accent = None;
        self
    }

    /// Pick the arc colour from one of the theme's accents. Clears any
    /// previously set explicit colour.
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = Some(accent);
        self.color = None;
        self
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let color = match (self.color, self.accent) {
            (Some(c), _) => c,
            (_, Some(a)) => theme.palette.accent_fill(a),
            _ => theme.palette.focus,
        };
        let thickness = self.thickness.unwrap_or((self.size * 0.12).max(1.5));

        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());

        if ui.is_rect_visible(rect) {
            crate::request_repaint_at_rate(ui.ctx(), 30.0);
            let painter = ui.painter();
            let center = rect.center();
            let radius = (self.size * 0.5) - thickness * 0.5 - 1.0;
            let point_at = |a: f32| {
                let (sin, cos) = a.sin_cos();
                pos2(center.x + radius * cos, center.y + radius * sin)
            };

            // Dim track ring. Built as a polyline (not `circle_stroke`)
            // so it lives on the same primitive as the arc below and
            // lands on identical pixels.
            let n_full: usize = 96;
            let track_points: Vec<_> = (0..n_full)
                .map(|i| point_at((i as f32 / n_full as f32) * TAU))
                .collect();
            painter.add(PathShape::closed_line(
                track_points,
                PathStroke::new(thickness, with_alpha(color, 40)),
            ));

            // Sweeping arc: rotates steadily while its length breathes
            // between ~29° and ~270°. The base rotation is fast enough
            // that even while the arc is contracting, its trailing end
            // still advances (never visually spins backward).
            let t = ui.input(|i| i.time) as f32;
            let phase = 0.5 - 0.5 * (t * 1.3).cos();
            let sweep_min = TAU * 0.08;
            let sweep_max = TAU * 0.75;
            let sweep = sweep_min + (sweep_max - sweep_min) * phase;
            let rotation = t * TAU * 0.85;

            let n_points = 48;
            let points: Vec<_> = (0..=n_points)
                .map(|i| point_at(rotation + (i as f32 / n_points as f32) * sweep))
                .collect();

            // Rounded caps, since PathShape strokes are butt-ended.
            painter.circle_filled(points[0], thickness * 0.5, color);
            painter.circle_filled(points[n_points], thickness * 0.5, color);
            painter.add(PathShape::line(points, PathStroke::new(thickness, color)));
        }

        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "loading"));
        response
    }
}
