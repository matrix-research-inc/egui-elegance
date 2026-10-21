//! Dual-handle range slider for selecting a `[low, high]` interval.
//!
//! Shares the pill-track styling with [`Slider`](crate::Slider) but renders
//! two thumbs and an accent fill that spans only the selected portion of
//! the track. The header row above the track shows the label on the left
//! and the current low / high values on the right.

use std::ops::RangeInclusive;

use egui::{
    CornerRadius, CursorIcon, Event, EventFilter, Id, Key, Pos2, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, Widget, WidgetInfo, WidgetText, emath::Numeric,
};

use crate::slider::{SliderHandle, paint_handle};
use crate::theme::{Accent, Theme, mix, with_alpha};

/// A horizontal numeric range slider with two thumbs.
///
/// Both endpoints are bound to caller-owned values; the widget keeps
/// `low <= high` automatically (a thumb dragged past its sibling clamps
/// to the sibling's value rather than swapping).
///
/// ```no_run
/// # use elegance::RangeSlider;
/// # egui::__run_test_ui(|ui| {
/// let (mut lo, mut hi): (u32, u32) = (24, 118);
/// ui.add(
///     RangeSlider::new(&mut lo, &mut hi, 0u32..=200u32)
///         .label("Price")
///         .suffix("$"),
/// );
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct RangeSlider<'a, T: Numeric> {
    low: &'a mut T,
    high: &'a mut T,
    range: RangeInclusive<T>,
    label: Option<WidgetText>,
    suffix: String,
    decimals: Option<usize>,
    value_fmt: Option<Box<dyn Fn(f64) -> String + 'a>>,
    show_value: bool,
    step: Option<f64>,
    ticks: Option<usize>,
    show_tick_labels: bool,
    accent: Accent,
    desired_width: Option<f32>,
    enabled: bool,
    id_salt: Option<Id>,
    handle: SliderHandle,
}

impl<'a, T: Numeric> std::fmt::Debug for RangeSlider<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeSlider")
            .field("range_lo", &self.range.start().to_f64())
            .field("range_hi", &self.range.end().to_f64())
            .field("suffix", &self.suffix)
            .field("decimals", &self.decimals)
            .field("show_value", &self.show_value)
            .field("step", &self.step)
            .field("ticks", &self.ticks)
            .field("show_tick_labels", &self.show_tick_labels)
            .field("accent", &self.accent)
            .field("desired_width", &self.desired_width)
            .field("enabled", &self.enabled)
            .field("handle", &self.handle)
            .finish()
    }
}

impl<'a, T: Numeric> RangeSlider<'a, T> {
    /// Create a range slider bound to `low` and `high`, constrained to `range`.
    pub fn new(low: &'a mut T, high: &'a mut T, range: RangeInclusive<T>) -> Self {
        Self {
            low,
            high,
            range,
            label: None,
            suffix: String::new(),
            decimals: None,
            value_fmt: None,
            show_value: true,
            step: None,
            ticks: None,
            show_tick_labels: false,
            accent: Accent::Sky,
            desired_width: None,
            enabled: true,
            id_salt: None,
            handle: SliderHandle::Circle,
        }
    }

    /// Show a label on the left of the header row above the track.
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Suffix appended to each formatted endpoint value (e.g. `"%"`, `" dB"`).
    #[inline]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Number of decimal places in the value display. Defaults to `0` for
    /// integer-typed sliders and `2` for float-typed.
    #[inline]
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = Some(n);
        self
    }

    /// Custom formatter applied to both endpoint values. Overrides
    /// [`suffix`](Self::suffix) and [`decimals`](Self::decimals).
    pub fn value_fmt(mut self, fmt: impl Fn(f64) -> String + 'a) -> Self {
        self.value_fmt = Some(Box::new(fmt));
        self
    }

    /// Hide the right-aligned `low – high` display in the header row. The
    /// label, if any, still renders. Default: shown.
    #[inline]
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Snap values to multiples of `step` (in the slider's value units).
    /// Integer-typed sliders snap to `1.0` automatically unless overridden.
    #[inline]
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Draw `n` evenly-spaced tick marks across the track (including both
    /// endpoints, so `.ticks(5)` draws 5 ticks at 0%, 25%, 50%, 75%, 100%).
    /// Pass `0` or `1` to suppress ticks.
    #[inline]
    pub fn ticks(mut self, n: usize) -> Self {
        self.ticks = Some(n);
        self
    }

    /// When tick marks are drawn, also render the value at each tick beneath
    /// the track. No effect without [`ticks`](Self::ticks).
    #[inline]
    pub fn show_tick_labels(mut self, show: bool) -> Self {
        self.show_tick_labels = show;
        self
    }

    /// Pick the fill colour. Default: [`Accent::Sky`].
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Override the slider width. Defaults to `ui.available_width()`.
    #[inline]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Disable the widget. A disabled slider still paints its current values
    /// but ignores pointer and keyboard input. Default: enabled.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Salt used to derive the per-thumb interaction ids. Set this when
    /// multiple range sliders share a parent ui so their thumbs don't collide
    /// on focus or drag state.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl crate::IdSalt) -> Self {
        self.id_salt = Some(Id::unique(id_salt));
        self
    }

    /// Pick the thumb shape. Both thumbs share the same style. Default:
    /// [`SliderHandle::Circle`]. Switch to [`SliderHandle::Line`] for a thin
    /// vertical bar instead of the standard circular knob.
    #[inline]
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

impl<'a, T: Numeric> Widget for RangeSlider<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;
        let accent_fill = p.accent_fill(self.accent);

        let lo_raw = self.range.start().to_f64();
        let hi_raw = self.range.end().to_f64();
        let (range_lo, range_hi) = if lo_raw <= hi_raw {
            (lo_raw, hi_raw)
        } else {
            (hi_raw, lo_raw)
        };
        let span = (range_hi - range_lo).max(f64::EPSILON);

        let mut low_v = self.low.to_f64();
        let mut high_v = self.high.to_f64();
        if low_v.is_nan() {
            low_v = range_lo;
        }
        if high_v.is_nan() {
            high_v = range_hi;
        }
        low_v = low_v.clamp(range_lo, range_hi);
        high_v = high_v.clamp(range_lo, range_hi);
        if low_v > high_v {
            std::mem::swap(&mut low_v, &mut high_v);
        }

        let step = self.step.or(if T::INTEGRAL { Some(1.0) } else { None });

        let track_h: f32 = 6.0;
        let thumb_d: f32 = 14.0;
        let halo_pad: f32 = 4.0; // accounts for the focus halo painted beyond the thumb radius
        let strip_h = thumb_d + 2.0 * halo_pad;
        let mut row_h = strip_h;
        // Reserve space below the track for tick marks (and optional labels)
        // so the next widget in the parent layout doesn't overlap them.
        if let Some(n) = self.ticks
            && n >= 2
        {
            row_h += 4.0;
            if self.show_tick_labels {
                row_h += t.small + 4.0;
            }
        }

        let id_salt = self
            .id_salt
            .unwrap_or_else(|| Id::unique(ui.next_auto_id()));
        let drag_state_id = id_salt.with("range_slider_drag_idx");
        let label_text = self
            .label
            .as_ref()
            .map(|l| l.text().to_string())
            .unwrap_or_default();

        ui.vertical(|ui| {
            // Header row: optional label on the left, optional value display on the right.
            if self.label.is_some() || self.show_value {
                ui.horizontal(|ui| {
                    if let Some(label) = &self.label {
                        let color = if self.enabled { p.text } else { p.text_faint };
                        let rich = egui::RichText::new(label.text()).color(color).size(t.label);
                        ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                    }
                    if self.show_value {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let lo_text = self.format_value(low_v);
                            let hi_text = self.format_value(high_v);
                            let value_text = format!("{lo_text} \u{2013} {hi_text}");
                            let color = if self.enabled {
                                p.text_muted
                            } else {
                                p.text_faint
                            };
                            let rich = egui::RichText::new(value_text)
                                .color(color)
                                .size(t.label)
                                .family(egui::FontFamily::Monospace);
                            ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                        });
                    }
                });
                ui.add_space(4.0);
            }

            // Allocate the slider strip.
            let total_w = self
                .desired_width
                .unwrap_or_else(|| ui.available_width())
                .max(thumb_d * 4.0);
            let bg_sense = if self.enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let (rect, bg_resp) = ui.allocate_exact_size(Vec2::new(total_w, row_h), bg_sense);

            let thumb_pad = thumb_d * 0.5;
            let track_left = rect.min.x + thumb_pad;
            let track_right = rect.max.x - thumb_pad;
            let track_span = (track_right - track_left).max(1.0);
            // Centre the track within the strip so the focus halo fits above
            // and below; tick labels (if any) sit in the extra space below.
            let track_y = rect.min.y + strip_h * 0.5;

            let to_x = |v: f64| -> f32 {
                let frac = ((v - range_lo) / span).clamp(0.0, 1.0) as f32;
                track_left + track_span * frac
            };
            let to_value = |x: f32| -> f64 {
                let frac = ((x - track_left) / track_span).clamp(0.0, 1.0) as f64;
                range_lo + frac * span
            };
            let snap = |mut v: f64| -> f64 {
                if let Some(s) = step
                    && s > 0.0
                {
                    v = range_lo + ((v - range_lo) / s).round() * s;
                }
                v.clamp(range_lo, range_hi)
            };

            let thumb_x = [to_x(low_v), to_x(high_v)];
            let thumb_centers = [
                Pos2::new(thumb_x[0], track_y),
                Pos2::new(thumb_x[1], track_y),
            ];

            // Per-thumb interaction rects with comfortable click targets.
            let thumb_hit = thumb_d.max(20.0);
            let thumb_rects = [
                Rect::from_center_size(thumb_centers[0], Vec2::splat(thumb_hit)),
                Rect::from_center_size(thumb_centers[1], Vec2::splat(thumb_hit)),
            ];
            let thumb_sense = if self.enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let thumb_resp = [
                ui.interact(thumb_rects[0], id_salt.with("low"), thumb_sense),
                ui.interact(thumb_rects[1], id_salt.with("high"), thumb_sense),
            ];

            let mut new_low = low_v;
            let mut new_high = high_v;
            let mut changed = false;
            // Tracked apart from `changed` because only the keyboard path is a
            // discrete, already-settled adjustment; the pointer paths settle on
            // release and are detected from the response.
            let mut keyboard_changed = false;

            // 1. Per-thumb pointer drag takes priority.
            let mut handled_thumb_drag = false;
            if self.enabled {
                for (i, resp) in thumb_resp.iter().enumerate() {
                    // A press that lands on a thumb focuses that thumb here.
                    // The background branch below focuses the thumb it picks,
                    // but it only ever runs for presses that missed both.
                    crate::focus::focus_on_press(resp);
                    if resp.is_pointer_button_down_on()
                        && let Some(pos) = resp.interact_pointer_pos()
                    {
                        let v = snap(to_value(pos.x));
                        apply_to_endpoint(&mut new_low, &mut new_high, i, v);
                        changed = true;
                        handled_thumb_drag = true;
                    }
                }
            }

            // 2. Background click/drag: pick the nearer thumb and update it.
            // Sticky during a drag — once we pick a thumb, keep using it until
            // the pointer is released.
            if self.enabled && bg_resp.is_pointer_button_down_on() && !handled_thumb_drag {
                if let Some(pos) = bg_resp.interact_pointer_pos() {
                    let v = to_value(pos.x);
                    let stored: Option<usize> = ui.ctx().data(|d| d.get_temp(drag_state_id));
                    let idx = match stored {
                        Some(i) => i,
                        None => {
                            let i = if (v - low_v).abs() <= (v - high_v).abs() {
                                0
                            } else {
                                1
                            };
                            ui.ctx().data_mut(|d| d.insert_temp(drag_state_id, i));
                            thumb_resp[i].request_focus();
                            i
                        }
                    };
                    let snapped = snap(v);
                    apply_to_endpoint(&mut new_low, &mut new_high, idx, snapped);
                    changed = true;
                }
            } else {
                ui.ctx().data_mut(|d| d.remove::<usize>(drag_state_id));
            }

            // 3. Keyboard nudges per focused thumb. Left / Right adjust the
            // value; Shift bumps to 10x; Home / End jump to the endpoints. Up
            // / Down are intentionally left to egui's focus navigation so the
            // user can move off the slider vertically.
            //
            // `set_focus_lock_filter` claims horizontal arrows for the
            // focused thumb; without it, egui's focus system consumes Left /
            // Right for spatial navigation before the value handler below
            // ever sees them.
            if self.enabled {
                for (i, resp) in thumb_resp.iter().enumerate() {
                    if !resp.has_focus() {
                        continue;
                    }
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            resp.id,
                            EventFilter {
                                horizontal_arrows: true,
                                ..Default::default()
                            },
                        );
                    });
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
                            let cur = if i == 0 { new_low } else { new_high };
                            let nudge = if modifiers.shift {
                                big_step
                            } else {
                                small_step
                            };
                            let next = match key {
                                Key::ArrowLeft => Some(cur - nudge),
                                Key::ArrowRight => Some(cur + nudge),
                                Key::Home => Some(range_lo),
                                Key::End => Some(range_hi),
                                _ => None,
                            };
                            if let Some(v) = next {
                                let v = snap(v);
                                apply_to_endpoint(&mut new_low, &mut new_high, i, v);
                                changed = true;
                                keyboard_changed = true;
                            }
                        }
                    }
                }
            }

            // Cursor.
            if self.enabled {
                let any_hovered =
                    bg_resp.hovered() || thumb_resp[0].hovered() || thumb_resp[1].hovered();
                let any_pressed = bg_resp.is_pointer_button_down_on()
                    || thumb_resp[0].is_pointer_button_down_on()
                    || thumb_resp[1].is_pointer_button_down_on();
                if any_pressed {
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                } else if any_hovered {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
            }

            // Commit changes back to the bound state.
            if changed {
                if (new_low - low_v).abs() > f64::EPSILON {
                    *self.low = T::from_f64(new_low);
                    low_v = new_low;
                }
                if (new_high - high_v).abs() > f64::EPSILON {
                    *self.high = T::from_f64(new_high);
                    high_v = new_high;
                }
            }

            // Render.
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let track_radius = CornerRadius::same((track_h * 0.5).round() as u8);
                let track_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x, track_y - track_h * 0.5),
                    Pos2::new(rect.max.x, track_y + track_h * 0.5),
                );

                // Unfilled track.
                painter.rect(
                    track_rect,
                    track_radius,
                    p.input_bg,
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );

                // Filled portion between low and high.
                let lo_x = to_x(low_v);
                let hi_x = to_x(high_v);
                if hi_x > lo_x + 0.5 {
                    let fill_rect = Rect::from_min_max(
                        Pos2::new(lo_x, track_rect.min.y),
                        Pos2::new(hi_x, track_rect.max.y),
                    );
                    let fill = if self.enabled {
                        accent_fill
                    } else {
                        mix(accent_fill, p.card, 0.55)
                    };
                    painter.rect_filled(fill_rect, track_radius, fill);
                }

                // Tick marks.
                if let Some(n) = self.ticks
                    && n >= 2
                {
                    for i in 0..n {
                        let frac = i as f32 / (n - 1) as f32;
                        let x = track_left + track_span * frac;
                        let v = range_lo + (frac as f64) * span;
                        let inside_fill = v >= low_v && v <= high_v;
                        let color = if inside_fill {
                            p.text_muted
                        } else {
                            p.text_faint
                        };
                        painter.line_segment(
                            [
                                Pos2::new(x, track_y - track_h * 0.5 - 3.0),
                                Pos2::new(x, track_y - track_h * 0.5 - 7.0),
                            ],
                            Stroke::new(1.0, color),
                        );

                        if self.show_tick_labels {
                            let label = self.format_value(v);
                            let galley = crate::theme::placeholder_galley(
                                ui,
                                &label,
                                t.small,
                                false,
                                f32::INFINITY,
                            );
                            let pos =
                                Pos2::new(x - galley.size().x * 0.5, track_y + track_h * 0.5 + 4.0);
                            painter.galley(pos, galley, p.text_faint);
                        }
                    }
                }

                // Thumbs.
                for i in 0..2 {
                    let center = thumb_centers[i];
                    let active = self.enabled
                        && (thumb_resp[i].has_focus() || thumb_resp[i].is_pointer_button_down_on());
                    let halo = active.then(|| with_alpha(accent_fill, 55));
                    let line_body = if p.is_dark {
                        p.text
                    } else {
                        p.accent_hover(self.accent)
                    };
                    let (body, ring) = match (self.handle, self.enabled) {
                        (SliderHandle::Circle, true) => (p.text, Stroke::new(2.0, accent_fill)),
                        (SliderHandle::Circle, false) => {
                            (mix(p.text, p.card, 0.5), Stroke::new(1.0, p.border))
                        }
                        (SliderHandle::Line, true) => (line_body, Stroke::NONE),
                        (SliderHandle::Line, false) => (mix(line_body, p.card, 0.5), Stroke::NONE),
                    };
                    paint_handle(painter, self.handle, center, thumb_d, body, ring, halo);
                }
            }

            // a11y info on each thumb response.
            for (i, side) in ["low", "high"].iter().enumerate() {
                let v = if i == 0 { low_v } else { high_v };
                let label = if label_text.is_empty() {
                    (*side).to_string()
                } else {
                    format!("{label_text} ({side})")
                };
                let resp = thumb_resp[i].clone();
                resp.widget_info(|| {
                    let mut info = WidgetInfo::labeled(Role::Slider, self.enabled, &label);
                    info.value = Some(v);
                    info
                });
            }

            let mut combined = bg_resp;
            combined |= thumb_resp[0].clone();
            combined |= thumb_resp[1].clone();
            if changed {
                combined.mark_changed();
            }
            if keyboard_changed {
                // Keyed to `combined.id`, which `union` takes from `bg_resp`:
                // that is the id the caller sees, and focus living on a thumb
                // makes any id-based check on `combined` unreliable.
                crate::commit::mark_commit(ui.ctx(), combined.id);
            }
            combined
        })
        .inner
    }
}

fn apply_to_endpoint(low: &mut f64, high: &mut f64, idx: usize, v: f64) {
    if idx == 0 {
        *low = v.min(*high);
    } else {
        *high = v.max(*low);
    }
}
