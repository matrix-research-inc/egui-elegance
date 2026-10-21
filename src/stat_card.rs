//! Compact stat tile for dashboards: label, value, delta chip, and inline sparkline.
//!
//! Use [`StatCard`] for at-a-glance numeric KPIs in observability or admin
//! UIs. A small upper-cased label sits above the headline value, paired
//! with an optional unit suffix and a coloured delta chip indicating
//! direction of change. A subtitle line carries comparison context (e.g.
//! `"vs last 7 days"`), and an optional 44 pt filled-area sparkline tinted
//! by the card's accent shows the recent trend at a glance.

use egui::{
    Align, Color32, CornerRadius, FontId, FontSelection, Frame, Layout, Margin, Pos2, Rect,
    Response, Role, Sense, Shape, Stroke, StrokeKind, TextWrapMode, Ui, Vec2, Widget, WidgetInfo,
    WidgetText,
    epaint::{Mesh, PathShape, PathStroke},
    pos2,
};

use crate::theme::{Accent, Palette, Theme, with_alpha};

#[derive(Copy, Clone, PartialEq, Eq)]
enum DeltaDir {
    Up,
    Down,
    Flat,
}

impl DeltaDir {
    fn from_value(delta: f32) -> Self {
        if delta > 0.005 {
            Self::Up
        } else if delta < -0.005 {
            Self::Down
        } else {
            Self::Flat
        }
    }

    fn arrow(self) -> char {
        match self {
            Self::Up => '\u{2191}',
            Self::Down => '\u{2193}',
            Self::Flat => '\u{2192}',
        }
    }
}

/// A dashboard stat tile.
///
/// Renders an upper-cased label, a headline value (with optional unit
/// suffix), a coloured delta chip, a short comparison subtitle, and an
/// optional filled-area sparkline of recent values, all inside a rounded
/// card surface. The accent colour tints the sparkline. While the
/// underlying metric is still loading, call [`StatCard::loading`] to
/// render a shimmer placeholder in place of the value and sparkline.
///
/// ```no_run
/// # use elegance::{StatCard, Accent};
/// # egui::__run_test_ui(|ui| {
/// let series = [12.0, 14.0, 13.0, 15.0, 17.0, 16.0, 18.0, 22.0_f32];
/// ui.add(
///     StatCard::new("Active deploys")
///         .accent(Accent::Blue)
///         .value("24")
///         .delta(0.12)
///         .trend("vs last 7 days")
///         .sparkline(&series),
/// );
/// # });
/// ```
#[must_use = "Add the stat card with `ui.add(...)`."]
pub struct StatCard<'a> {
    label: WidgetText,
    accent: Accent,
    value: Option<WidgetText>,
    unit: Option<WidgetText>,
    delta: Option<f32>,
    invert_delta: bool,
    trend: Option<WidgetText>,
    sparkline: Option<&'a [f32]>,
    sparkline_color: Option<Color32>,
    width: Option<f32>,
    loading: bool,
    info_tooltip: Option<WidgetText>,
}

impl std::fmt::Debug for StatCard<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatCard")
            .field("label", &self.label.text())
            .field("accent", &self.accent)
            .field("value", &self.value.as_ref().map(|v| v.text()))
            .field("unit", &self.unit.as_ref().map(|v| v.text()))
            .field("delta", &self.delta)
            .field("invert_delta", &self.invert_delta)
            .field("trend", &self.trend.as_ref().map(|v| v.text()))
            .field("sparkline_len", &self.sparkline.map(|s| s.len()))
            .field("width", &self.width)
            .field("loading", &self.loading)
            .finish()
    }
}

impl<'a> StatCard<'a> {
    /// Create a new stat card with the given label.
    ///
    /// The label is rendered in upper-case (matching the mockup's
    /// `"ACTIVE DEPLOYS"` treatment) regardless of the case the caller
    /// passes in. Defaults: blue accent, no value, no delta, no trend, no
    /// sparkline.
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            accent: Accent::Blue,
            value: None,
            unit: None,
            delta: None,
            invert_delta: false,
            trend: None,
            sparkline: None,
            sparkline_color: None,
            width: None,
            loading: false,
            info_tooltip: None,
        }
    }

    /// Set the accent colour driving the sparkline tint. Default:
    /// [`Accent::Blue`].
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Set the headline value text. The caller is responsible for any
    /// numeric formatting (rounding, locale separators, etc.).
    #[inline]
    pub fn value(mut self, value: impl Into<WidgetText>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set a small unit suffix rendered next to the value (`%`, `ms`,
    /// `req/s`, ...). Default: none.
    #[inline]
    pub fn unit(mut self, unit: impl Into<WidgetText>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set the fractional change to display as a coloured delta chip
    /// (e.g. `0.12` for a `+12.0%` rise). Sign drives the arrow direction
    /// (up / down / flat); magnitude is rendered as a percentage with one
    /// decimal. Default: no chip.
    #[inline]
    pub fn delta(mut self, delta: f32) -> Self {
        self.delta = Some(delta);
        self
    }

    /// When set, a *negative* delta is treated as the good direction (chip
    /// renders green) and a positive delta as bad (chip renders red).
    /// Useful for metrics where down is good, like latency or error rate.
    /// Default: false.
    #[inline]
    pub fn invert_delta(mut self, invert: bool) -> Self {
        self.invert_delta = invert;
        self
    }

    /// Set the small subtitle below the value (e.g. `"vs last 7 days"`).
    #[inline]
    pub fn trend(mut self, trend: impl Into<WidgetText>) -> Self {
        self.trend = Some(trend.into());
        self
    }

    /// Render an inline filled-area sparkline of recent values beneath
    /// the trend line. The series is read at the point of `ui.add(...)`
    /// and not retained. At least two points are required.
    #[inline]
    pub fn sparkline(mut self, series: &'a [f32]) -> Self {
        self.sparkline = Some(series);
        self
    }

    /// Override the sparkline's tint. Defaults to the accent colour.
    #[inline]
    pub fn sparkline_color(mut self, color: Color32) -> Self {
        self.sparkline_color = Some(color);
        self
    }

    /// Pin the card width. Defaults to the parent's available width,
    /// which lets the card flow inside grid cells or `horizontal_wrapped`
    /// rows.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Render a shimmer skeleton in place of the value, trend, and
    /// sparkline. Use while the underlying metric is loading.
    #[inline]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Attach a tooltip shown on card hover, plus a small painted info
    /// indicator next to the label.
    #[inline]
    pub fn info_tooltip(mut self, tooltip: impl Into<WidgetText>) -> Self {
        self.info_tooltip = Some(tooltip.into());
        self
    }
}

impl Widget for StatCard<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let StatCard {
            label,
            accent,
            value,
            unit,
            delta,
            invert_delta,
            trend,
            sparkline,
            sparkline_color,
            width,
            loading,
            info_tooltip,
        } = self;

        let card_radius = CornerRadius::same(theme.card_radius as u8);
        let inner_margin = Margin {
            left: 18,
            right: 18,
            top: 16,
            bottom: 14,
        };
        let card_width = width.unwrap_or_else(|| ui.available_width()).max(180.0);

        let value_size = (t.heading * 1.75).max(t.body * 1.6);
        let unit_size = t.body;
        let small_size = t.small;
        let label_size = t.small;

        let line_color = sparkline_color.unwrap_or_else(|| p.accent_fill(accent));
        let label_text = label.text().to_uppercase();
        let a11y_label = label_text.clone();
        let has_info = info_tooltip.is_some();

        let frame_response = ui
            .scope(|ui| {
                ui.set_min_width(card_width);
                ui.set_max_width(card_width);

                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    Frame::new()
                        .fill(p.card)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(card_radius)
                        .inner_margin(inner_margin)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                            ui.horizontal(|ui| {
                                let g = WidgetText::from(
                                    egui::RichText::new(&label_text)
                                        .color(p.text_muted)
                                        .size(label_size),
                                )
                                .into_galley(
                                    ui,
                                    Some(TextWrapMode::Truncate),
                                    ui.available_width(),
                                    FontSelection::FontId(FontId::proportional(label_size)),
                                );
                                let size = g.size();
                                let (lrect, _) = ui.allocate_exact_size(size, Sense::hover());
                                if ui.is_rect_visible(lrect) {
                                    ui.painter().galley(lrect.min, g, p.text_muted);
                                }
                                if has_info {
                                    ui.add_space(4.0);
                                    paint_info_glyph(ui, p.text_faint);
                                }
                            });

                            ui.add_space(8.0);

                            if loading {
                                skeleton_bar(ui, ui.available_width() * 0.4, value_size * 0.95, p);
                            } else if let Some(v) = value {
                                paint_value_row(
                                    ui,
                                    p,
                                    v,
                                    unit,
                                    delta,
                                    invert_delta,
                                    value_size,
                                    unit_size,
                                    small_size,
                                );
                            }

                            ui.add_space(2.0);

                            if !loading && let Some(trend) = trend {
                                let g = WidgetText::from(
                                    egui::RichText::new(trend.text())
                                        .color(p.text_faint)
                                        .size(small_size),
                                )
                                .into_galley(
                                    ui,
                                    Some(TextWrapMode::Truncate),
                                    ui.available_width(),
                                    FontSelection::FontId(FontId::proportional(small_size)),
                                );
                                let size = g.size();
                                let (tr, _) = ui.allocate_exact_size(size, Sense::hover());
                                if ui.is_rect_visible(tr) {
                                    ui.painter().galley(tr.min, g, p.text_faint);
                                }
                            }

                            if loading {
                                ui.add_space(14.0);
                                skeleton_bar(ui, ui.available_width(), 44.0, p);
                            } else if let Some(series) = sparkline {
                                ui.add_space(14.0);
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 44.0),
                                    Sense::hover(),
                                );
                                if ui.is_rect_visible(rect) {
                                    paint_sparkline(ui, rect, series, line_color);
                                }
                            }
                        })
                        .response
                })
                .inner
            })
            .inner;

        if loading {
            crate::request_repaint_at_rate(ui.ctx(), 30.0);
        }

        let mut response = frame_response;
        if let Some(tooltip) = info_tooltip {
            let text = tooltip.text().to_string();
            response = response.on_hover_text(text);
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &a11y_label));
        response
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_value_row(
    ui: &mut Ui,
    palette: &Palette,
    value: WidgetText,
    unit: Option<WidgetText>,
    delta: Option<f32>,
    invert_delta: bool,
    value_size: f32,
    unit_size: f32,
    small_size: f32,
) {
    let value_galley = WidgetText::from(
        egui::RichText::new(value.text())
            .color(palette.text)
            .size(value_size)
            .strong(),
    )
    .into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::FontId(FontId::proportional(value_size)),
    );
    let value_v = value_galley.size();

    let unit_galley = unit.map(|u| {
        WidgetText::from(
            egui::RichText::new(u.text())
                .color(palette.text_muted)
                .size(unit_size),
        )
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            FontSelection::FontId(FontId::proportional(unit_size)),
        )
    });
    let unit_v = unit_galley.as_ref().map(|g| g.size()).unwrap_or(Vec2::ZERO);

    let row_height = value_v.y.max(unit_v.y);
    let avail_w = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, row_height), Sense::hover());
    if !ui.is_rect_visible(row_rect) {
        return;
    }

    let mut x = row_rect.left();
    let value_pos = pos2(x, row_rect.bottom() - value_v.y);
    ui.painter().galley(value_pos, value_galley, palette.text);
    x += value_v.x + 2.0;

    if let Some(g) = unit_galley {
        // Pull the unit slightly below the value's baseline so descender
        // metrics line up visually instead of mathematically.
        let pos = pos2(x, row_rect.bottom() - unit_v.y - 2.0);
        ui.painter().galley(pos, g, palette.text_muted);
        x += unit_v.x;
    }

    if let Some(d) = delta {
        x += 10.0;
        paint_delta_chip(
            ui,
            pos2(x, row_rect.center().y),
            palette,
            DeltaDir::from_value(d),
            d.abs(),
            invert_delta,
            small_size,
        );
    }
}

fn paint_delta_chip(
    ui: &mut Ui,
    anchor_left_center: Pos2,
    palette: &Palette,
    dir: DeltaDir,
    magnitude: f32,
    invert: bool,
    small_size: f32,
) {
    let good = if invert { DeltaDir::Down } else { DeltaDir::Up };
    let (fg, bg, border) = match dir {
        DeltaDir::Flat => (
            palette.text_muted,
            with_alpha(palette.text_muted, 22),
            with_alpha(palette.text_muted, 51),
        ),
        d if d == good => (
            palette.success,
            with_alpha(palette.success, 26),
            with_alpha(palette.success, 64),
        ),
        _ => (
            palette.danger,
            with_alpha(palette.danger, 26),
            with_alpha(palette.danger, 64),
        ),
    };

    let label = format!("{} {:.1}%", dir.arrow(), magnitude * 100.0);
    let galley = WidgetText::from(
        egui::RichText::new(label)
            .color(fg)
            .size(small_size)
            .strong(),
    )
    .into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::FontId(FontId::proportional(small_size)),
    );

    let pad = Vec2::new(8.0, 3.0);
    let chip_size = galley.size() + pad * 2.0;
    let chip_min = pos2(
        anchor_left_center.x,
        anchor_left_center.y - chip_size.y * 0.5,
    );
    let chip_rect = Rect::from_min_size(chip_min, chip_size);
    let radius = CornerRadius::same((chip_size.y * 0.5).round() as u8);

    let painter = ui.painter();
    painter.rect_filled(chip_rect, radius, bg);
    painter.rect_stroke(
        chip_rect,
        radius,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let text_pos = pos2(
        chip_rect.min.x + pad.x,
        chip_rect.center().y - galley.size().y * 0.5,
    );
    painter.galley(text_pos, galley, fg);
}

fn paint_info_glyph(ui: &mut Ui, color: Color32) {
    let radius = 5.5;
    let size = Vec2::splat(radius * 2.0 + 1.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(center, radius, Stroke::new(1.0, color));
    painter.circle_filled(center + Vec2::new(0.0, -2.5), 0.85, color);
    painter.line_segment(
        [center + Vec2::new(0.0, -0.5), center + Vec2::new(0.0, 2.4)],
        Stroke::new(1.0, color),
    );
}

fn skeleton_bar(ui: &mut Ui, width: f32, height: f32, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let phase = (ui.input(|i| i.time) % 1.4) as f32 / 1.4;
    let pulse = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
    let alpha = (20.0 + 21.0 * pulse).round() as u8;
    let fill = with_alpha(palette.text_muted, alpha);
    let r = (height.min(8.0) * 0.5).round() as u8;
    ui.painter().rect_filled(rect, CornerRadius::same(r), fill);
}

fn paint_sparkline(ui: &mut Ui, rect: Rect, series: &[f32], color: Color32) {
    if series.len() < 2 {
        return;
    }
    let plot = rect.shrink2(Vec2::splat(2.0));

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in series {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    let span = max - min;
    let pts: Vec<Pos2> = series
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let t = i as f32 / (series.len() - 1) as f32;
            let x = plot.left() + t * plot.width();
            let y = if span < 1e-6 {
                plot.center().y
            } else {
                plot.top() + (1.0 - (v - min) / span) * plot.height()
            };
            pos2(x, y)
        })
        .collect();

    // Vertex-coloured strip from the curve down to the bar's bottom edge,
    // alpha fading from 35 % at the SVG top to 0 % at the bottom. This
    // mirrors the linear gradient in the HTML mockup without needing a
    // shader.
    let mut mesh = Mesh::default();
    let top_y = rect.top();
    let bottom_y = rect.bottom();
    let h = (bottom_y - top_y).max(1.0);
    let (cr, cg, cb) = (color.r(), color.g(), color.b());
    for p in &pts {
        let y_ratio = ((p.y - top_y) / h).clamp(0.0, 1.0);
        let alpha = ((1.0 - y_ratio) * 0.35 * 255.0).round().clamp(0.0, 255.0) as u8;
        let top_color = Color32::from_rgba_unmultiplied(cr, cg, cb, alpha);
        let bottom_color = Color32::from_rgba_unmultiplied(cr, cg, cb, 0);
        mesh.colored_vertex(*p, top_color);
        mesh.colored_vertex(pos2(p.x, bottom_y), bottom_color);
    }
    for i in 0..pts.len() - 1 {
        let a = (i * 2) as u32;
        let b = (i * 2 + 1) as u32;
        let c = ((i + 1) * 2) as u32;
        let d = ((i + 1) * 2 + 1) as u32;
        mesh.add_triangle(a, b, c);
        mesh.add_triangle(b, d, c);
    }
    ui.painter().add(Shape::mesh(mesh));

    let line = PathShape::line(pts.clone(), PathStroke::new(1.75, color));
    ui.painter().add(line);

    if let Some(last) = pts.last() {
        ui.painter().circle_filled(*last, 2.5, color);
    }
}
