//! Metric slider — a single-value slider whose central UI element is the
//! value itself, rendered large in the header.
//!
//! [`MetricSlider`] generalises the design of
//! [`PercentSlider`](crate::PercentSlider) to an arbitrary numeric range. The
//! visual identity is the same: a prominent headline value above a rail, with
//! optional ticks and a drag callout. [`PercentSlider`](crate::PercentSlider)
//! is the `0.0..=100.0` + "%" preset over this widget.
//!
//! Use `MetricSlider` when the value carries its own absolute meaning (a
//! buffer size in GiB, a latency budget in milliseconds, a refresh-rate
//! target). Use [`PercentSlider`](crate::PercentSlider) when the value is a
//! fraction of a total and the percentage is the user-facing unit.
//!
//! Three snap modes are available, in order of specificity:
//! [`step`](MetricSlider::step) snaps to multiples of a fixed size
//! (relative to the start of the range);
//! [`steps`](MetricSlider::steps) snaps to `n` evenly-spaced positions
//! including both endpoints; [`stops`](MetricSlider::stops) snaps to an
//! explicit, possibly non-uniform list of positions. When `steps` or `stops`
//! is set, the tick row renders at exactly those positions and the arrow
//! keys jump between them.
//!
//! The returned [`Response`] reports `changed()` on every intermediate value a
//! drag passes through, which is what a live preview wants. In `stops` mode
//! that means a full-rail drag fires once per stop crossed. When the reaction
//! to a change is expensive or outward-facing, gate it on
//! [`ResponseCommitExt::committed`](crate::ResponseCommitExt::committed)
//! instead, which fires once per settled adjustment.

use std::ops::RangeInclusive;

use egui::{
    Color32, CornerRadius, CursorIcon, Event, EventFilter, Key, Pos2, Rect, Response, Role, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetInfo, WidgetText,
};

use crate::slider::{SliderHandle, paint_handle};
use crate::theme::{Accent, BASELINE_FRAC, Theme, placeholder_galley, with_alpha};

/// A single-value slider whose central UI element is the value itself.
///
/// ```no_run
/// # use elegance::{Accent, MetricSlider};
/// # egui::__run_test_ui(|ui| {
/// let mut buffer = 16.0_f32;
/// ui.add(
///     MetricSlider::new(&mut buffer, 0.0..=32.0)
///         .label("Buffer size")
///         .suffix("GiB")
///         .stops([4.0, 8.0, 16.0, 32.0])
///         .accent(Accent::Amber),
/// );
/// # });
/// ```
#[must_use = "Call `ui.add(...)` to render the widget."]
pub struct MetricSlider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    label: Option<WidgetText>,
    accent: Accent,
    show_ticks: bool,
    step: Option<f32>,
    stops: Option<Vec<f32>>,
    decimals: usize,
    suffix: Option<String>,
    headline_fmt: Option<Box<dyn Fn(f32) -> String + 'a>>,
    tick_fmt: Option<Box<dyn Fn(f32) -> String + 'a>>,
    callout_fmt: Option<Box<dyn Fn(f32) -> String + 'a>>,
    desired_width: Option<f32>,
    handle: SliderHandle,
}

impl<'a> std::fmt::Debug for MetricSlider<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricSlider")
            .field("range", &self.range)
            .field("accent", &self.accent)
            .field("show_ticks", &self.show_ticks)
            .field("step", &self.step)
            .field("stops", &self.stops)
            .field("decimals", &self.decimals)
            .field("suffix", &self.suffix)
            .field("desired_width", &self.desired_width)
            .field("handle", &self.handle)
            .finish()
    }
}

impl<'a> MetricSlider<'a> {
    /// Create a slider bound to `value`, clamped to `range`.
    ///
    /// Non-finite, swapped, or zero-width ranges are silently sanitised to a
    /// safe default; callers should pass a well-formed range.
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range: sanitize_range(range),
            label: None,
            accent: Accent::Sky,
            show_ticks: true,
            step: None,
            stops: None,
            decimals: 0,
            suffix: None,
            headline_fmt: None,
            tick_fmt: None,
            callout_fmt: None,
            desired_width: None,
            handle: SliderHandle::Circle,
        }
    }

    /// Add a small label in the top-left of the header row.
    ///
    /// The label also names the slider for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Pick the fill colour from one of the theme accents. Default: [`Accent::Sky`].
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Show or hide the tick row beneath the track. Default: `true`.
    #[inline]
    pub fn show_ticks(mut self, show: bool) -> Self {
        self.show_ticks = show;
        self
    }

    /// Snap the value to multiples of `step` (relative to the start of the
    /// range). For a `0.0..=32.0` range with `step(4.0)` the snap positions
    /// are `0, 4, 8, …, 32`.
    ///
    /// Mutually exclusive with [`steps`](Self::steps) and
    /// [`stops`](Self::stops); calling this clears either of them.
    #[inline]
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self.stops = None;
        self
    }

    /// Snap to `n` evenly-spaced positions across the range, including both
    /// endpoints. `steps(2)` snaps to `{start, end}`, `steps(5)` to five
    /// equally-spaced positions, and so on. Values below `2` are promoted to
    /// `2`.
    ///
    /// When set, the tick row renders at exactly those positions and the
    /// arrow keys jump between adjacent stops. Mutually exclusive with
    /// [`step`](Self::step) and [`stops`](Self::stops); calling this clears
    /// either of them.
    #[inline]
    pub fn steps(mut self, n: usize) -> Self {
        let n = n.max(2);
        let (s, e) = (*self.range.start(), *self.range.end());
        let span = e - s;
        let last = (n - 1) as f32;
        self.stops = Some((0..n).map(|i| s + (i as f32 / last) * span).collect());
        self.step = None;
        self
    }

    /// Snap to an explicit, possibly non-uniform list of positions inside the
    /// range. Out-of-range, `NaN`, and duplicate values are filtered out; the
    /// result is sorted ascending. If fewer than two valid positions remain,
    /// falls back to `[range.start(), range.end()]`.
    ///
    /// When set, the tick row renders at exactly these positions and the
    /// arrow keys jump between adjacent stops. Mutually exclusive with
    /// [`step`](Self::step) and [`steps`](Self::steps); calling this clears
    /// either of them.
    pub fn stops(mut self, positions: impl IntoIterator<Item = f32>) -> Self {
        let (s, e) = (*self.range.start(), *self.range.end());
        let mut v: Vec<f32> = positions
            .into_iter()
            .filter(|p| p.is_finite() && (s..=e).contains(p))
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v.dedup_by(|a, b| (*a - *b).abs() < f32::EPSILON);
        if v.len() < 2 {
            v = vec![s, e];
        }
        self.stops = Some(v);
        self.step = None;
        self
    }

    /// Number of decimal places in the default headline and tick formatters.
    /// Default: `0`. Ignored when an explicit
    /// [`headline_fmt`](Self::headline_fmt) or [`tick_fmt`](Self::tick_fmt)
    /// is set.
    #[inline]
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = n;
        self
    }

    /// Render a small, muted unit string baseline-aligned with the headline
    /// value. Use this for short units like `"%"`, `"GiB"`, `"ms"`. Default:
    /// no suffix.
    #[inline]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Supply a callback to format the entire headline value text. The
    /// callback receives the current value (in the slider's range) and
    /// returns the string rendered large in the top-right of the widget.
    /// When unset, the headline renders `format!("{value:.decimals$}")`.
    ///
    /// The [`suffix`](Self::suffix) (if set) is still drawn after the
    /// formatted text; pass `None` for the suffix when the formatter already
    /// includes the unit.
    #[inline]
    pub fn headline_fmt(mut self, fmt: impl Fn(f32) -> String + 'a) -> Self {
        self.headline_fmt = Some(Box::new(fmt));
        self
    }

    /// Supply a callback to format tick-row labels. The callback receives
    /// each tick's value (in the slider's range) and returns the label to
    /// render beneath the track. When unset, ticks render
    /// `format!("{value:.decimals$}")`.
    #[inline]
    pub fn tick_fmt(mut self, fmt: impl Fn(f32) -> String + 'a) -> Self {
        self.tick_fmt = Some(Box::new(fmt));
        self
    }

    /// Supply a callback to format the drag-callout text. The callback
    /// receives the current value and returns the string to render in the
    /// callout above the thumb while the user drags. When unset, no callout
    /// is shown.
    #[inline]
    pub fn callout_fmt(mut self, fmt: impl Fn(f32) -> String + 'a) -> Self {
        self.callout_fmt = Some(Box::new(fmt));
        self
    }

    /// Override the slider width. Defaults to `ui.available_width()`.
    #[inline]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Pick the thumb shape. Default: [`SliderHandle::Circle`]. Switch to
    /// [`SliderHandle::Line`] for a thin vertical bar instead of the standard
    /// circular knob.
    #[inline]
    pub fn handle(mut self, handle: SliderHandle) -> Self {
        self.handle = handle;
        self
    }
}

impl<'a> Widget for MetricSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;
        let accent_fill = p.accent_fill(self.accent);

        let (range_start, range_end) = (*self.range.start(), *self.range.end());
        let range_span = (range_end - range_start).max(f32::EPSILON);

        let value_size = (t.heading * 1.5).round().max(20.0);
        let sign_size = t.label;
        let label_size = t.label;
        let tick_size = t.small;

        let probe = placeholder_galley(ui, "0", value_size, true, f32::INFINITY);
        let header_h = probe.size().y;

        let track_h: f32 = 6.0;
        let thumb_d: f32 = 14.0;
        let row_track_h = thumb_d.max(22.0);
        let header_gap: f32 = 8.0;
        let tick_gap: f32 = if self.show_ticks { 4.0 } else { 0.0 };
        let tick_row_h: f32 = if self.show_ticks {
            tick_size + 8.0
        } else {
            0.0
        };

        let total_h = header_h + header_gap + row_track_h + tick_gap + tick_row_h;
        let total_w = self
            .desired_width
            .unwrap_or_else(|| ui.available_width())
            .max(180.0);

        let mut current = if self.value.is_nan() {
            range_start
        } else {
            *self.value
        };
        current = current.clamp(range_start, range_end);

        let step = self.step.filter(|s| s.is_finite() && *s > 0.0);
        let stops = self.stops.as_deref();

        // Snap a raw value (in the slider's range) to whichever snap mode
        // (if any) is configured: explicit stops > step > continuous.
        // Step rounds relative to `range_start` so the endpoints of the
        // range are always reachable.
        let snap = |raw: f32| -> f32 {
            let v = if let Some(stops) = stops {
                nearest_stop(stops, raw)
            } else if let Some(s) = step {
                range_start + ((raw - range_start) / s).round() * s
            } else {
                raw
            };
            v.clamp(range_start, range_end)
        };

        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::click_and_drag());

        let thumb_pad = thumb_d * 0.5;
        let track_y = rect.min.y + header_h + header_gap + row_track_h * 0.5;
        let track_left = rect.min.x + thumb_pad;
        let track_right = rect.max.x - thumb_pad;
        let track_span = (track_right - track_left).max(1.0);
        let track_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, track_y - track_h * 0.5),
            Pos2::new(rect.max.x, track_y + track_h * 0.5),
        );

        crate::focus::focus_on_press(&response);

        if response.is_pointer_button_down_on()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let clamped_x = pos.x.clamp(track_left, track_right);
            let frac = ((clamped_x - track_left) / track_span).clamp(0.0, 1.0);
            let next = snap(range_start + frac * range_span);
            if (next - current).abs() > f32::EPSILON {
                current = next;
                *self.value = current;
                response.mark_changed();
            }
        }

        // Keyboard nudges when the slider has focus. Left / Right step by
        // `step` (or 1% of the range span); Shift bumps to 10x; Home / End
        // jump to the endpoints. Up / Down are intentionally left to egui's
        // focus navigation so the user can move between stacked controls
        // vertically.
        //
        // `set_focus_lock_filter` claims horizontal arrows for this widget
        // while it's focused; without it, egui's focus system consumes
        // Left / Right for spatial navigation before the value handler below
        // ever sees them.
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });

            let small_step = step.unwrap_or(range_span / 100.0);
            let big_step = small_step * 10.0;
            let events = ui.input(|input| input.events.clone());
            for ev in &events {
                if let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = ev
                {
                    let nudge = if modifiers.shift {
                        big_step
                    } else {
                        small_step
                    };
                    let raw = match (stops, key) {
                        (Some(stops), Key::ArrowLeft) => adjacent_stop(stops, current, -1),
                        (Some(stops), Key::ArrowRight) => adjacent_stop(stops, current, 1),
                        (Some(stops), Key::Home) => stops.first().copied(),
                        (Some(stops), Key::End) => stops.last().copied(),
                        (None, Key::ArrowLeft) => Some(current - nudge),
                        (None, Key::ArrowRight) => Some(current + nudge),
                        (None, Key::Home) => Some(range_start),
                        (None, Key::End) => Some(range_end),
                        _ => None,
                    };
                    if let Some(next) = raw {
                        let next = snap(next);
                        if (next - current).abs() > f32::EPSILON {
                            current = next;
                            *self.value = current;
                            response.mark_changed();
                            // A key press is a discrete adjustment: already
                            // settled the moment it lands.
                            crate::commit::mark_commit(ui.ctx(), response.id);
                        }
                    }
                }
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        if response.is_pointer_button_down_on() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter().clone();

            // ---- Header row ----------------------------------------------
            let decimals = self.decimals;
            let num_text = match &self.headline_fmt {
                Some(f) => f(current),
                None => format!("{current:.decimals$}"),
            };
            let num_galley = placeholder_galley(ui, &num_text, value_size, true, f32::INFINITY);
            let baseline_y = rect.min.y + num_galley.size().y * BASELINE_FRAC;

            let (num_x, label_right_edge) = if let Some(suffix) = &self.suffix {
                let sign_galley = placeholder_galley(ui, suffix, sign_size, false, f32::INFINITY);
                let sign_x = rect.max.x - sign_galley.size().x;
                let num_x = sign_x - 2.0 - num_galley.size().x;
                painter.galley(
                    Pos2::new(sign_x, baseline_y - sign_galley.size().y * BASELINE_FRAC),
                    sign_galley,
                    p.text_muted,
                );
                (num_x, num_x)
            } else {
                let num_x = rect.max.x - num_galley.size().x;
                (num_x, num_x)
            };
            painter.galley(Pos2::new(num_x, rect.min.y), num_galley, p.text);

            if let Some(label) = &self.label {
                let label_wrap = (label_right_edge - rect.min.x - 8.0).max(0.0);
                let label_galley =
                    placeholder_galley(ui, label.text(), label_size, false, label_wrap);
                let label_top = baseline_y - label_galley.size().y * BASELINE_FRAC;
                painter.galley(Pos2::new(rect.min.x, label_top), label_galley, p.text);
            }

            // ---- Track ---------------------------------------------------
            let frac = ((current - range_start) / range_span).clamp(0.0, 1.0);
            let thumb_x = track_left + track_span * frac;
            let thumb_center = Pos2::new(thumb_x, track_y);

            let track_radius = CornerRadius::same((track_h * 0.5).round() as u8);

            painter.rect(
                track_rect,
                track_radius,
                p.input_bg,
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );
            if thumb_x > track_rect.min.x + 0.5 {
                let fill_rect = Rect::from_min_max(
                    Pos2::new(track_rect.min.x, track_rect.min.y),
                    Pos2::new(thumb_x, track_rect.max.y),
                );
                painter.rect_filled(fill_rect, track_radius, accent_fill);
            }

            // Faint 10% interior divisions, slightly stronger at the midpoint.
            // Painted on top of both the filled and unfilled portions so the
            // proportion reads at a glance like a battery indicator without
            // breaking the smooth-fill feel. These are rail-tenths, not
            // value-tenths; they are decorative and independent of range.
            let div_base = if p.is_dark {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            for i in 1..10 {
                let f = i as f32 / 10.0;
                let x = track_rect.min.x + track_rect.width() * f;
                let alpha = if i == 5 { 48 } else { 24 };
                painter.line_segment(
                    [
                        Pos2::new(x, track_rect.min.y + 1.5),
                        Pos2::new(x, track_rect.max.y - 1.5),
                    ],
                    Stroke::new(1.0, with_alpha(div_base, alpha)),
                );
            }

            let active = response.has_focus() || response.is_pointer_button_down_on();
            let halo = active.then(|| with_alpha(accent_fill, 55));
            let line_body = if p.is_dark {
                p.text
            } else {
                p.accent_hover(self.accent)
            };
            let (body, ring) = match self.handle {
                SliderHandle::Circle => (p.text, Stroke::new(2.0, accent_fill)),
                SliderHandle::Line => (line_body, Stroke::NONE),
            };
            paint_handle(
                &painter,
                self.handle,
                thumb_center,
                thumb_d,
                body,
                ring,
                halo,
            );

            // ---- Ticks ---------------------------------------------------
            // With explicit stops: render a tick at each, all medium weight
            // (no major/minor distinction). Without stops: the default
            // quartile row, with start / midpoint / end drawn slightly
            // heavier than the quarter marks.
            if self.show_ticks {
                let tick_top_y = rect.min.y + header_h + header_gap + row_track_h + tick_gap;
                let tick_fmt = |v: f32| -> String {
                    match &self.tick_fmt {
                        Some(f) => f(v),
                        None => format!("{v:.decimals$}"),
                    }
                };
                let ticks: Vec<(f32, String, bool)> = match stops {
                    Some(stops) => stops
                        .iter()
                        .map(|v| {
                            let frac = ((*v - range_start) / range_span).clamp(0.0, 1.0);
                            (frac, tick_fmt(*v), false)
                        })
                        .collect(),
                    None => [0.00_f32, 0.25, 0.50, 0.75, 1.00]
                        .into_iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let v = range_start + f * range_span;
                            let major = i % 2 == 0;
                            (f, tick_fmt(v), major)
                        })
                        .collect(),
                };
                for (f, label, major) in &ticks {
                    let x = track_left + track_span * f;
                    let tick_h = if *major { 5.0 } else { 3.5 };
                    let tick_color = if *major { p.text_muted } else { p.border };
                    painter.line_segment(
                        [Pos2::new(x, tick_top_y), Pos2::new(x, tick_top_y + tick_h)],
                        Stroke::new(1.0, tick_color),
                    );
                    let g = placeholder_galley(ui, label, tick_size, false, f32::INFINITY);
                    let raw_x = x - g.size().x * 0.5;
                    let tx = raw_x.max(rect.min.x).min(rect.max.x - g.size().x);
                    let ty = tick_top_y + tick_h + 1.0;
                    painter.galley(Pos2::new(tx, ty), g, p.text_faint);
                }
            }

            // ---- Drag callout --------------------------------------------
            if response.is_pointer_button_down_on()
                && let Some(fmt) = &self.callout_fmt
            {
                paint_callout(
                    ui,
                    &theme,
                    &painter,
                    thumb_center,
                    rect,
                    thumb_d,
                    fmt(current),
                );
            }
        }

        let label_str = self
            .label
            .as_ref()
            .map(|l| l.text().to_string())
            .unwrap_or_default();
        response.widget_info(|| WidgetInfo::labeled(Role::Slider, true, &label_str));
        response
    }
}

fn sanitize_range(range: RangeInclusive<f32>) -> RangeInclusive<f32> {
    let (mut s, mut e) = range.into_inner();
    if !s.is_finite() || !e.is_finite() {
        s = 0.0;
        e = 100.0;
    } else if s > e {
        std::mem::swap(&mut s, &mut e);
    }
    if (e - s).abs() < f32::EPSILON {
        s = 0.0;
        e = 100.0;
    }
    s..=e
}

fn paint_callout(
    ui: &egui::Ui,
    theme: &Theme,
    painter: &egui::Painter,
    thumb_center: Pos2,
    widget_rect: Rect,
    thumb_d: f32,
    text: String,
) {
    let p = &theme.palette;
    let t = &theme.typography;
    let g = placeholder_galley(ui, &text, t.label, false, f32::INFINITY);

    let pad_x: f32 = 9.0;
    let pad_y: f32 = 5.0;
    let tail_h: f32 = 5.0;
    let cw = g.size().x + 2.0 * pad_x;
    let ch = g.size().y + 2.0 * pad_y;

    let half_w = cw * 0.5;
    let edge_pad = 4.0;
    let cx = thumb_center.x.clamp(
        widget_rect.min.x + half_w + edge_pad,
        widget_rect.max.x - half_w - edge_pad,
    );
    let cy_bottom = thumb_center.y - thumb_d * 0.5 - tail_h - 2.0;
    let crect = Rect::from_min_max(
        Pos2::new(cx - half_w, cy_bottom - ch),
        Pos2::new(cx + half_w, cy_bottom),
    );

    let fill = p.depth_tint(p.input_bg, 0.04);
    let border = p.border;

    painter.rect(
        crect,
        CornerRadius::same(5),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    let tail_x = thumb_center
        .x
        .clamp(crect.min.x + tail_h + 2.0, crect.max.x - tail_h - 2.0);
    let tail_top_y = crect.max.y;
    let tip = Pos2::new(tail_x, tail_top_y + tail_h);
    let left = Pos2::new(tail_x - tail_h, tail_top_y);
    let right = Pos2::new(tail_x + tail_h, tail_top_y);
    painter.add(Shape::convex_polygon(
        vec![left, tip, right],
        fill,
        Stroke::new(1.0, border),
    ));
    painter.line_segment(
        [
            Pos2::new(tail_x - tail_h + 1.0, tail_top_y),
            Pos2::new(tail_x + tail_h - 1.0, tail_top_y),
        ],
        Stroke::new(1.5, fill),
    );

    painter.galley(
        Pos2::new(crect.min.x + pad_x, crect.min.y + pad_y),
        g,
        p.text,
    );
}

/// Snap `raw` to the nearest entry in `stops`. `stops` is assumed sorted and
/// non-empty; the builder maintains both invariants.
fn nearest_stop(stops: &[f32], raw: f32) -> f32 {
    let mut best = stops[0];
    let mut best_d = (raw - best).abs();
    for &s in &stops[1..] {
        let d = (raw - s).abs();
        if d < best_d {
            best_d = d;
            best = s;
        }
    }
    best
}

/// Return the stop immediately above (`dir > 0`) or below (`dir < 0`) `current`,
/// or `None` if `current` is already at the corresponding extreme.
fn adjacent_stop(stops: &[f32], current: f32, dir: i32) -> Option<f32> {
    let eps = 0.001;
    if dir > 0 {
        stops.iter().copied().find(|&s| s > current + eps)
    } else {
        stops.iter().rev().copied().find(|&s| s < current - eps)
    }
}
