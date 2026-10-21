//! Horizontal numeric slider with a pill track, accent-coloured fill, and
//! an optional right-aligned value display.
//!
//! The widget is generic over [`egui::emath::Numeric`], so any built-in
//! numeric type (`f32`, `f64`, integer types) can be used directly.

use std::ops::RangeInclusive;

use egui::{
    Color32, CornerRadius, CursorIcon, Event, EventFilter, Key, Painter, Pos2, Rect, Response,
    Role, Sense, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetInfo, WidgetText, emath::Numeric,
};

use crate::theme::{Accent, Theme, with_alpha};

/// Visual style of the slider thumb.
///
/// Affects [`Slider`], [`RangeSlider`](crate::RangeSlider),
/// [`MetricSlider`](crate::MetricSlider), and
/// [`PercentSlider`](crate::PercentSlider). All four default to
/// [`SliderHandle::Circle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderHandle {
    /// A circular thumb with a pale interior and an accent-coloured ring.
    /// This is the default and matches the look of egui's built-in slider.
    #[default]
    Circle,
    /// A thin, vertical accent-coloured bar. Useful for scrubber-style
    /// sliders where the pointer position should sit on top of the rail
    /// without a wide thumb obscuring nearby ticks or fill.
    Line,
}

/// Width of the line-handle bar, in points.
pub(crate) const LINE_HANDLE_W: f32 = 3.0;
/// Total height of the line-handle bar, in points. Sized so the bar pokes a
/// few points above and below the 6 pt track to read as a positive marker
/// rather than a hairline.
pub(crate) const LINE_HANDLE_H: f32 = 18.0;

/// Paint a slider thumb at `center`.
///
/// `body` is the primary fill: for [`SliderHandle::Circle`] this is the pale
/// interior; for [`SliderHandle::Line`] this is the bar fill itself
/// (typically the accent colour or a desaturated variant when disabled).
/// `accent_stroke` is the circle's outer ring and is ignored for `Line`.
/// `halo` paints a soft, larger backdrop in the same shape when `Some`.
pub(crate) fn paint_handle(
    painter: &Painter,
    handle: SliderHandle,
    center: Pos2,
    thumb_d: f32,
    body: Color32,
    accent_stroke: Stroke,
    halo: Option<Color32>,
) {
    match handle {
        SliderHandle::Circle => {
            if let Some(c) = halo {
                painter.circle_filled(center, thumb_d * 0.5 + 4.0, c);
            }
            painter.circle(center, thumb_d * 0.5, body, accent_stroke);
        }
        SliderHandle::Line => {
            if let Some(c) = halo {
                let halo_rect = Rect::from_center_size(
                    center,
                    Vec2::new(LINE_HANDLE_W + 10.0, LINE_HANDLE_H + 8.0),
                );
                painter.rect_filled(halo_rect, CornerRadius::same(6), c);
            }
            let bar = Rect::from_center_size(center, Vec2::new(LINE_HANDLE_W, LINE_HANDLE_H));
            painter.rect_filled(bar, CornerRadius::same(1), body);
        }
    }
}

/// A horizontal numeric slider.
///
/// ```no_run
/// # use elegance::Slider;
/// # egui::__run_test_ui(|ui| {
/// let mut cpu = 42.0_f32;
/// ui.add(Slider::new(&mut cpu, 0.0..=100.0).label("CPU limit").suffix("%"));
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct Slider<'a, T: Numeric> {
    value: &'a mut T,
    range: RangeInclusive<T>,
    label: Option<WidgetText>,
    suffix: String,
    decimals: Option<usize>,
    value_fmt: Option<Box<dyn Fn(f64) -> String + 'a>>,
    show_value: bool,
    step: Option<f64>,
    accent: Accent,
    desired_width: Option<f32>,
    handle: SliderHandle,
}

impl<'a, T: Numeric> std::fmt::Debug for Slider<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("range_lo", &self.range.start().to_f64())
            .field("range_hi", &self.range.end().to_f64())
            .field("suffix", &self.suffix)
            .field("decimals", &self.decimals)
            .field("show_value", &self.show_value)
            .field("step", &self.step)
            .field("accent", &self.accent)
            .field("desired_width", &self.desired_width)
            .field("handle", &self.handle)
            .finish()
    }
}

impl<'a, T: Numeric> Slider<'a, T> {
    /// Create a slider bound to `value`, constrained to `range`.
    pub fn new(value: &'a mut T, range: RangeInclusive<T>) -> Self {
        Self {
            value,
            range,
            label: None,
            suffix: String::new(),
            decimals: None,
            value_fmt: None,
            show_value: true,
            step: None,
            accent: Accent::Sky,
            desired_width: None,
            handle: SliderHandle::Circle,
        }
    }

    /// Show a label above the slider.
    ///
    /// The label also names the slider for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Suffix appended to the formatted value (e.g. `"%"`, `" dB"`).
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Number of decimal places in the value display. Defaults to `0` for
    /// integer-typed sliders and `2` for float-typed.
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = Some(n);
        self
    }

    /// Supply a custom formatter for the value display. Overrides
    /// [`suffix`](Self::suffix) and [`decimals`](Self::decimals).
    pub fn value_fmt(mut self, fmt: impl Fn(f64) -> String + 'a) -> Self {
        self.value_fmt = Some(Box::new(fmt));
        self
    }

    /// Hide the right-aligned value display.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Snap the value to multiples of `step` (in the slider's value units).
    /// Integer-typed sliders snap to `1.0` automatically unless overridden.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Pick the fill colour from one of the theme accents. Default: [`Accent::Sky`].
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Override the slider width. Defaults to `ui.available_width()`.
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Pick the thumb shape. Default: [`SliderHandle::Circle`]. Switch to
    /// [`SliderHandle::Line`] for a thin vertical bar instead of the standard
    /// circular knob.
    pub fn handle(mut self, handle: SliderHandle) -> Self {
        self.handle = handle;
        self
    }

    fn format_value(&self, v: f64) -> String {
        if let Some(fmt) = &self.value_fmt {
            return fmt(v);
        }
        let n = self.decimals.unwrap_or(if T::INTEGRAL { 0 } else { 2 });
        if self.suffix.is_empty() {
            format!("{v:.n$}")
        } else {
            format!("{v:.n$}{}", self.suffix)
        }
    }
}

impl<'a, T: Numeric> Widget for Slider<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;
        let accent_fill = p.accent_fill(self.accent);

        let lo_raw = self.range.start().to_f64();
        let hi_raw = self.range.end().to_f64();
        let (lo, hi) = if lo_raw <= hi_raw {
            (lo_raw, hi_raw)
        } else {
            (hi_raw, lo_raw)
        };

        let mut current = self.value.to_f64();
        if current.is_nan() {
            current = lo;
        }
        current = current.clamp(lo, hi);

        let step = self.step.or(if T::INTEGRAL { Some(1.0) } else { None });

        let track_h: f32 = 6.0;
        let thumb_d: f32 = 14.0;
        let row_h = thumb_d.max(t.label + 2.0);
        let value_gap: f32 = 10.0;

        let value_reserve = if self.show_value {
            let lo_text = self.format_value(lo);
            let hi_text = self.format_value(hi);
            let w_lo =
                crate::theme::placeholder_galley(ui, &lo_text, t.label, false, f32::INFINITY)
                    .size()
                    .x;
            let w_hi =
                crate::theme::placeholder_galley(ui, &hi_text, t.label, false, f32::INFINITY)
                    .size()
                    .x;
            w_lo.max(w_hi).ceil() + value_gap
        } else {
            0.0
        };

        let label_text = self
            .label
            .as_ref()
            .map(|l| l.text().to_string())
            .unwrap_or_default();

        ui.vertical(|ui| {
            if !label_text.is_empty() {
                ui.add_space(2.0);
                let rich = egui::RichText::new(&label_text)
                    .color(p.text_muted)
                    .size(t.label);
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                ui.add_space(2.0);
            }

            let total_w = self
                .desired_width
                .unwrap_or_else(|| ui.available_width())
                .max(value_reserve + thumb_d * 2.0);
            let (rect, mut response) =
                ui.allocate_exact_size(Vec2::new(total_w, row_h), Sense::click_and_drag());

            let track_w = (total_w - value_reserve).max(thumb_d);
            let thumb_pad = thumb_d * 0.5;
            let track_left = rect.min.x + thumb_pad;
            let track_right = rect.min.x + track_w - thumb_pad;
            let track_span = (track_right - track_left).max(1.0);
            let track_y = rect.center().y;
            let track_rect = Rect::from_min_max(
                Pos2::new(rect.min.x, track_y - track_h * 0.5),
                Pos2::new(rect.min.x + track_w, track_y + track_h * 0.5),
            );

            crate::focus::focus_on_press(&response);

            // Update value from pointer while the button is held on the widget.
            if response.is_pointer_button_down_on()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let clamped_x = pos.x.clamp(track_left, track_right);
                let frac = ((clamped_x - track_left) / track_span).clamp(0.0, 1.0) as f64;
                let mut new_value = lo + frac * (hi - lo);
                if let Some(step) = step
                    && step > 0.0
                {
                    new_value = lo + ((new_value - lo) / step).round() * step;
                }
                new_value = new_value.clamp(lo, hi);
                if (new_value - current).abs() > f64::EPSILON {
                    current = new_value;
                    *self.value = T::from_f64(current);
                    response.mark_changed();
                }
            }

            // Keyboard nudges when the slider has focus. Left / Right step by
            // `step` (or 1% of the range if continuous); Shift bumps to 10x;
            // Home / End jump to the endpoints. Up / Down are intentionally
            // left to egui's focus navigation so the user can move between
            // stacked controls vertically.
            //
            // `set_focus_lock_filter` claims horizontal arrows for this
            // widget while it's focused; without it, egui's focus system
            // consumes Left / Right for spatial navigation before the value
            // handler below ever sees them.
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

                let span = hi - lo;
                let small_step = step.unwrap_or(span * 0.01);
                let big_step = step.map(|s| s * 10.0).unwrap_or(span * 0.1);
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
                        let next = match key {
                            Key::ArrowLeft => Some(current - nudge),
                            Key::ArrowRight => Some(current + nudge),
                            Key::Home => Some(lo),
                            Key::End => Some(hi),
                            _ => None,
                        };
                        if let Some(mut new_value) = next {
                            if let Some(s) = step
                                && s > 0.0
                            {
                                new_value = lo + ((new_value - lo) / s).round() * s;
                            }
                            new_value = new_value.clamp(lo, hi);
                            if (new_value - current).abs() > f64::EPSILON {
                                current = new_value;
                                *self.value = T::from_f64(current);
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
                let frac = if hi > lo {
                    ((current - lo) / (hi - lo)).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                let thumb_x = track_left + track_span * frac;
                let thumb_center = Pos2::new(thumb_x, track_y);

                let painter = ui.painter();
                let track_radius = CornerRadius::same((track_h * 0.5).round() as u8);

                // Unfilled track.
                painter.rect(
                    track_rect,
                    track_radius,
                    p.input_bg,
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );

                // Filled portion up to the thumb.
                if thumb_x > track_rect.min.x + 0.5 {
                    let fill_rect = Rect::from_min_max(
                        Pos2::new(track_rect.min.x, track_rect.min.y),
                        Pos2::new(thumb_x, track_rect.max.y),
                    );
                    painter.rect_filled(fill_rect, track_radius, accent_fill);
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
                    painter,
                    self.handle,
                    thumb_center,
                    thumb_d,
                    body,
                    ring,
                    halo,
                );

                if self.show_value {
                    let text = self.format_value(current);
                    let galley =
                        crate::theme::placeholder_galley(ui, &text, t.label, false, f32::INFINITY);
                    let text_pos = Pos2::new(
                        rect.max.x - galley.size().x,
                        rect.center().y - galley.size().y * 0.5,
                    );
                    painter.galley(text_pos, galley, p.text);
                }
            }

            response.widget_info(|| WidgetInfo::labeled(Role::Slider, true, &label_text));
            response
        })
        .inner
    }
}
