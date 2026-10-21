//! Gauges: radial half-circle and linear meter.
//!
//! Two widgets for displaying a current value (as a 0..1 fraction)
//! against optional threshold zones:
//!
//! - [`RadialGauge`] — half-circle speedometer with an optional needle and
//!   value readout in the bowl. The classic dashboard gauge.
//! - [`LinearGauge`] — horizontal bar with optional faded threshold bands
//!   behind the fill, optional ticks and labels above.
//!
//! For the donut form (a circular gauge with no needle), use
//! [`ProgressRing`](crate::ProgressRing) with [`ProgressRing::zones`]:
//! it shares the same shape as a determinate progress indicator, plus
//! [`ProgressRing::unit`] for a baseline-aligned suffix and
//! [`ProgressRing::caption_below`] to anchor a caption outside the ring.
//!
//! Both gauges derive their fill colour from [`GaugeZones`] when supplied
//! (`success` / `warning` / `danger` based on which band the value falls
//! in). Without zones they fall back to the theme's focus accent. Override
//! either with the per-widget `color` builder.

use std::f32::consts::PI;

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, Widget,
    WidgetInfo,
    epaint::{PathShape, PathStroke},
    pos2,
};

use crate::theme::{BASELINE_FRAC, Palette, Theme, placeholder_galley, with_alpha};

/// Threshold breakpoints driving automatic colouring on a gauge.
///
/// Values up to `warn` paint in the theme's `success` colour, values
/// from `warn..crit` paint in `warning`, and values `>= crit` paint in
/// `danger`. Both fields are clamped to `0..=1` and `crit` is forced to
/// be at least `warn`.
///
/// ```
/// # use elegance::GaugeZones;
/// let z = GaugeZones::new(0.6, 0.85);
/// assert_eq!(z.warn(), 0.6);
/// assert_eq!(z.crit(), 0.85);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaugeZones {
    warn: f32,
    crit: f32,
}

impl GaugeZones {
    /// Create a new zone breakdown. `warn` is the green→amber boundary,
    /// `crit` the amber→red boundary. Both are clamped to `0..=1`; `crit`
    /// is raised to `warn` if it would otherwise be smaller.
    pub fn new(warn: f32, crit: f32) -> Self {
        let warn = warn.clamp(0.0, 1.0);
        let crit = crit.clamp(0.0, 1.0).max(warn);
        Self { warn, crit }
    }

    /// The green→amber boundary as a fraction in `0..=1`.
    pub const fn warn(self) -> f32 {
        self.warn
    }

    /// The amber→red boundary as a fraction in `0..=1`.
    pub const fn crit(self) -> f32 {
        self.crit
    }

    pub(crate) fn color(&self, fraction: f32, palette: &Palette) -> Color32 {
        if fraction >= self.crit {
            palette.danger
        } else if fraction >= self.warn {
            palette.warning
        } else {
            palette.success
        }
    }
}

fn clamp_fraction(f: f32) -> f32 {
    if f.is_nan() { 0.0 } else { f.clamp(0.0, 1.0) }
}

fn track_color(palette: &Palette) -> Color32 {
    if palette.is_dark {
        palette.bg
    } else {
        palette.depth_tint(palette.input_bg, 0.04)
    }
}

// --- Radial -----------------------------------------------------------------

/// Half-circle dashboard gauge with an optional needle and value readout.
///
/// ```no_run
/// # use elegance::{RadialGauge, GaugeZones};
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     RadialGauge::new(0.42)
///         .zones(GaugeZones::new(0.6, 0.85)),
/// );
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "Add with `ui.add(...)`."]
pub struct RadialGauge {
    fraction: f32,
    size: f32,
    color: Option<Color32>,
    zones: Option<GaugeZones>,
    needle: bool,
    text: Option<String>,
    unit: Option<String>,
    show_scale: bool,
}

impl RadialGauge {
    /// Create a gauge displaying `fraction` (0..=1). NaN and out-of-range
    /// values are clamped.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: clamp_fraction(fraction),
            size: 200.0,
            color: None,
            zones: None,
            needle: true,
            text: None,
            unit: None,
            show_scale: true,
        }
    }

    /// Outer width in points. The gauge's height is roughly `0.74 * size`
    /// for the arc plus a small reserve for the scale labels. Default: 200.
    /// Clamped to at least 80.
    #[inline]
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(80.0);
        self
    }

    /// Override the fill colour. Clears any previously set zones-based
    /// colouring (zones still drive the threshold bands if configured).
    #[inline]
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Configure threshold zones. The fill colour is auto-derived from the
    /// zone the current fraction falls in (success/warning/danger), and
    /// faint bands are painted behind the active fill at the boundaries.
    #[inline]
    pub fn zones(mut self, zones: GaugeZones) -> Self {
        self.zones = Some(zones);
        self
    }

    /// Whether to draw the needle. Default: on.
    #[inline]
    pub fn needle(mut self, on: bool) -> Self {
        self.needle = on;
        self
    }

    /// Override the value readout. Default: rounded percent (e.g. "42").
    /// Pass `""` to hide the readout entirely.
    #[inline]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Override the unit suffix shown after the value. Default: `"%"` when
    /// the readout uses the auto percent; no unit otherwise. Pass `""` to
    /// hide the unit.
    #[inline]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Whether to draw the `0`/`100` scale labels under the arc. Default: on.
    #[inline]
    pub fn show_scale(mut self, on: bool) -> Self {
        self.show_scale = on;
        self
    }
}

impl Widget for RadialGauge {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        let scale_size = (self.size * 0.052).clamp(9.0, 12.0);
        let scale_h = if self.show_scale {
            scale_size + 4.0
        } else {
            0.0
        };
        let arc_h = self.size * 0.74;
        let total_h = arc_h + scale_h;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.size, total_h), Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let arc_rect = Rect::from_min_size(rect.min, Vec2::new(self.size, arc_h));
            let cx = arc_rect.center().x;
            let cy = arc_rect.top() + self.size * 0.5;
            let r = self.size * 0.4;
            let stroke_w = self.size * 0.07;

            // Sample the half-arc; arc_point(t) maps t in 0..1 from the
            // left endpoint (fraction 0) to the right endpoint (fraction 1)
            // sweeping over the top.
            let n_segments: usize = 96;
            let arc_point = |t: f32| -> Pos2 {
                let a = PI - PI * t;
                pos2(cx + r * a.cos(), cy - r * a.sin())
            };
            let arc_points = |start: f32, end: f32| -> Vec<Pos2> {
                let span = (end - start).max(0.0);
                let n = ((n_segments as f32 * span).ceil() as usize).max(2);
                (0..=n)
                    .map(|i| arc_point(start + span * (i as f32 / n as f32)))
                    .collect()
            };

            // Track.
            painter.add(PathShape::line(
                arc_points(0.0, 1.0),
                PathStroke::new(stroke_w, track_color(p)),
            ));

            // Threshold bands.
            if let Some(z) = &self.zones {
                painter.add(PathShape::line(
                    arc_points(0.0, z.warn),
                    PathStroke::new(stroke_w, with_alpha(p.success, 56)),
                ));
                painter.add(PathShape::line(
                    arc_points(z.warn, z.crit),
                    PathStroke::new(stroke_w, with_alpha(p.warning, 60)),
                ));
                painter.add(PathShape::line(
                    arc_points(z.crit, 1.0),
                    PathStroke::new(stroke_w, with_alpha(p.danger, 66)),
                ));
            }

            // Active fill.
            let fill_color = self.color.unwrap_or_else(|| {
                self.zones
                    .as_ref()
                    .map(|z| z.color(self.fraction, p))
                    .unwrap_or(p.focus)
            });
            if self.fraction > 0.0 {
                painter.add(PathShape::line(
                    arc_points(0.0, self.fraction),
                    PathStroke::new(stroke_w, fill_color),
                ));
            }

            // Tick marks at zone boundaries.
            if let Some(z) = &self.zones {
                for &boundary in &[z.warn, z.crit] {
                    let a = PI - PI * boundary;
                    let inner_r = r + stroke_w * 0.5 + 1.0;
                    let outer_r = inner_r + stroke_w * 0.55;
                    let inner = pos2(cx + inner_r * a.cos(), cy - inner_r * a.sin());
                    let outer = pos2(cx + outer_r * a.cos(), cy - outer_r * a.sin());
                    painter.line_segment([inner, outer], Stroke::new(1.0, p.text_muted));
                }
            }

            // Needle.
            if self.needle {
                let a = PI - PI * self.fraction;
                let needle_len = r * 0.9;
                let half_w = (self.size * 0.013).max(1.5);
                let perp = a + PI * 0.5;
                let tip = pos2(cx + needle_len * a.cos(), cy - needle_len * a.sin());
                let base_l = pos2(cx + half_w * perp.cos(), cy - half_w * perp.sin());
                let base_r = pos2(cx - half_w * perp.cos(), cy + half_w * perp.sin());
                painter.add(PathShape::convex_polygon(
                    vec![tip, base_l, base_r],
                    p.text,
                    Stroke::NONE,
                ));

                let pivot_r = (self.size * 0.03).max(4.0);
                painter.circle_filled(pos2(cx, cy), pivot_r, p.card);
                painter.circle_stroke(pos2(cx, cy), pivot_r, Stroke::new(1.5, p.text));
                painter.circle_filled(pos2(cx, cy), pivot_r * 0.28, p.bg);
            }

            // Value readout (in the bowl, below the pivot).
            let primary_size = (self.size * 0.15).clamp(14.0, 36.0);
            let unit_size = (self.size * 0.085).clamp(12.0, 22.0);
            let primary = self
                .text
                .clone()
                .unwrap_or_else(|| format!("{}", (self.fraction * 100.0).round() as u32));
            let unit = self.unit.clone().unwrap_or_else(|| {
                if self.text.is_none() {
                    "%".into()
                } else {
                    String::new()
                }
            });

            if !primary.is_empty() {
                let g_num = placeholder_galley(ui, &primary, primary_size, true, f32::INFINITY);
                let g_unit = (!unit.is_empty())
                    .then(|| placeholder_galley(ui, &unit, unit_size, false, f32::INFINITY));
                let num_w = g_num.size().x;
                let num_h = g_num.size().y;
                let unit_w = g_unit.as_ref().map_or(0.0, |g| g.size().x);
                let gap = if g_unit.is_some() { 3.0 } else { 0.0 };
                let total_w = num_w + gap + unit_w;

                let bottom_y = arc_rect.bottom() - 6.0;
                let num_top = bottom_y - num_h;
                let start_x = cx - total_w * 0.5;
                painter.galley(pos2(start_x, num_top), g_num, p.text);
                if let Some(g) = g_unit {
                    let baseline = num_top + num_h * BASELINE_FRAC;
                    let unit_y = baseline - g.size().y * BASELINE_FRAC;
                    painter.galley(pos2(start_x + num_w + gap, unit_y), g, p.text_muted);
                }
            }

            // Scale labels.
            if self.show_scale {
                let label_y = arc_rect.bottom() + 2.0;
                let g_left = placeholder_galley(ui, "0", scale_size, false, f32::INFINITY);
                let g_right = placeholder_galley(ui, "100", scale_size, false, f32::INFINITY);
                painter.galley(
                    pos2(cx - r - g_left.size().x * 0.5, label_y),
                    g_left,
                    p.text_faint,
                );
                painter.galley(
                    pos2(cx + r - g_right.size().x * 0.5, label_y),
                    g_right,
                    p.text_faint,
                );
            }
        }

        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "gauge"));
        response
    }
}

// --- Linear -----------------------------------------------------------------

/// Horizontal meter with optional threshold bands and labels.
///
/// Differs from [`ProgressBar`](crate::ProgressBar) in three ways:
/// threshold zones drive the fill colour (success / warning / danger),
/// faint bands paint behind the fill at zone boundaries, and a thumb
/// marker pins the current position over the bar. Threshold ticks and
/// labels above the bar are opt-in via [`LinearGauge::threshold_label`]
/// or [`LinearGauge::show_zone_labels`].
///
/// ```no_run
/// # use elegance::{LinearGauge, GaugeZones};
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     LinearGauge::new(0.42)
///         .zones(GaugeZones::new(0.6, 0.85))
///         .show_zone_labels(),
/// );
/// # });
/// ```
#[derive(Clone, Debug)]
#[must_use = "Add with `ui.add(...)`."]
pub struct LinearGauge {
    fraction: f32,
    height: f32,
    desired_width: Option<f32>,
    color: Option<Color32>,
    zones: Option<GaugeZones>,
    threshold_labels: Vec<(f32, String)>,
    thumb: bool,
}

impl LinearGauge {
    /// Create a meter at `fraction` (0..=1). NaN and out-of-range values
    /// are clamped.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: clamp_fraction(fraction),
            height: 14.0,
            desired_width: None,
            color: None,
            zones: None,
            threshold_labels: Vec::new(),
            thumb: true,
        }
    }

    /// Bar height in points. Default: 14.
    #[inline]
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(6.0);
        self
    }

    /// Override the bar width. Defaults to `ui.available_width()`.
    #[inline]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Override the fill colour.
    #[inline]
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Configure threshold zones. Faint bands paint behind the fill at
    /// the boundaries and the fill colour auto-derives from the active
    /// zone (success / warning / danger).
    #[inline]
    pub fn zones(mut self, zones: GaugeZones) -> Self {
        self.zones = Some(zones);
        self
    }

    /// Add a tick + label above the bar at `position` (0..=1). Stack
    /// multiple calls to register more.
    pub fn threshold_label(mut self, position: f32, label: impl Into<String>) -> Self {
        self.threshold_labels
            .push((position.clamp(0.0, 1.0), label.into()));
        self
    }

    /// Convenience: add ticks + labels at the configured zone boundaries,
    /// formatting each as a percent. Has no effect unless [`zones`] is
    /// also set.
    ///
    /// [`zones`]: LinearGauge::zones
    pub fn show_zone_labels(mut self) -> Self {
        if let Some(z) = self.zones {
            self.threshold_labels
                .push((z.warn, format!("{}", (z.warn * 100.0).round() as u32)));
            self.threshold_labels
                .push((z.crit, format!("{}", (z.crit * 100.0).round() as u32)));
        }
        self
    }

    /// Whether to draw the thumb marker at the current position. Default: on.
    #[inline]
    pub fn thumb(mut self, on: bool) -> Self {
        self.thumb = on;
        self
    }
}

impl Widget for LinearGauge {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        let label_size = 10.0;
        let label_pad = 6.0;
        let label_h = if self.threshold_labels.is_empty() {
            0.0
        } else {
            label_size + label_pad
        };
        let width = self
            .desired_width
            .unwrap_or_else(|| ui.available_width())
            .max(self.height * 4.0);
        let total_h = self.height + label_h;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, total_h), Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let bar_rect = Rect::from_min_size(
                pos2(rect.left(), rect.top() + label_h),
                Vec2::new(width, self.height),
            );
            let radius = CornerRadius::same((self.height * 0.5).round() as u8);

            // Track.
            painter.rect(
                bar_rect,
                radius,
                p.input_bg,
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );

            // Zone bands behind the fill, with rounded outer corners only
            // so they meet the bar's pill shape on either end.
            if let Some(z) = &self.zones {
                let r = (self.height * 0.5).round() as u8;
                let band =
                    |start: f32, end: f32, color: Color32, left_round: bool, right_round: bool| {
                        if end <= start {
                            return;
                        }
                        let x0 = bar_rect.left() + bar_rect.width() * start;
                        let x1 = bar_rect.left() + bar_rect.width() * end;
                        let cr = CornerRadius {
                            nw: if left_round { r } else { 0 },
                            sw: if left_round { r } else { 0 },
                            ne: if right_round { r } else { 0 },
                            se: if right_round { r } else { 0 },
                        };
                        let rect = Rect::from_min_max(
                            pos2(x0, bar_rect.top()),
                            pos2(x1, bar_rect.bottom()),
                        );
                        painter.rect_filled(rect.shrink(0.5), cr, color);
                    };
                band(0.0, z.warn, with_alpha(p.success, 50), true, false);
                band(z.warn, z.crit, with_alpha(p.warning, 56), false, false);
                band(z.crit, 1.0, with_alpha(p.danger, 60), false, true);
            }

            // Active fill.
            let fill_color = self.color.unwrap_or_else(|| {
                self.zones
                    .as_ref()
                    .map(|z| z.color(self.fraction, p))
                    .unwrap_or(p.focus)
            });
            let fill_w = bar_rect.width() * self.fraction;
            if fill_w > 0.5 {
                let fill_rect =
                    Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
                painter
                    .with_clip_rect(fill_rect)
                    .rect_filled(bar_rect, radius, fill_color);
            }

            // Thumb.
            if self.thumb && self.fraction > 0.0 {
                let x = bar_rect.left() + fill_w;
                painter.line_segment(
                    [
                        pos2(x, bar_rect.top() + 1.0),
                        pos2(x, bar_rect.bottom() - 1.0),
                    ],
                    Stroke::new(2.0, p.text),
                );
            }

            // Threshold ticks + labels.
            for (pos, label) in &self.threshold_labels {
                let x = bar_rect.left() + bar_rect.width() * pos.clamp(0.0, 1.0);
                let g = placeholder_galley(ui, label, label_size, false, f32::INFINITY);
                let label_y = rect.top();
                painter.galley(pos2(x - g.size().x * 0.5, label_y), g, p.text_faint);
                let tick_top = label_y + label_size + 1.0;
                let tick_bot = bar_rect.top() - 1.0;
                if tick_bot > tick_top {
                    painter.line_segment(
                        [pos2(x, tick_top), pos2(x, tick_bot)],
                        Stroke::new(1.0, p.text_faint),
                    );
                }
            }
        }

        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "meter"));
        response
    }
}
