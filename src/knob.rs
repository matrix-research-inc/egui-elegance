//! Rotary knob: 270-degree arc, accent-coloured fill, optional labeled
//! detents.
//!
//! A leaf widget bound to caller state, generic over [`egui::emath::Numeric`].
//! The visual treatment matches an instrument-panel knob: a thin arc track
//! with an active fill that grows clockwise from the lower-left, a body with
//! a subtle depth-tinted rim, and a tick indicator pointing at the current
//! value. Three preset sizes cover compact instrument rows, default forms,
//! and prominent stepped controls with labeled positions.
//!
//! Interaction: drag combines horizontal and vertical motion (right and
//! up both increase, left and down both decrease, so a diagonal flick reads
//! as a single gesture); `Shift` slows the drag for fine control. Scroll
//! wheel, arrow keys / Page Up / Page Down / Home / End all nudge. Grabbing
//! the knob gives it keyboard focus, so a drag can be refined with the arrow
//! keys without tabbing to it first.
//! Alt+click, double-click, or `0` resets to a configured default. Bipolar knobs
//! fill from the centre of the range outward toward the current value,
//! suited to signed offsets (DC bias, pan, balance).

use std::f32::consts::PI;
use std::ops::RangeInclusive;

use egui::{
    Align2, Color32, FontId, Pos2, Response, Role, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo,
    WidgetText,
    emath::Numeric,
    epaint::{PathShape, PathStroke},
    pos2, vec2,
};

use crate::theme::{Accent, Theme, with_alpha};

/// Visual size preset for [`Knob`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KnobSize {
    /// Compact knob suited to instrument-panel rows of four-plus controls.
    Small,
    /// Default knob, readable as a primary control inside a form.
    #[default]
    Medium,
    /// Prominent knob, sized to host labeled detents around its rim.
    Large,
}

/// Value mapping along the arc.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KnobScale {
    /// Linear position-to-value mapping (default).
    #[default]
    Linear,
    /// Logarithmic mapping. Requires the range minimum to be positive;
    /// non-positive minima fall back to linear.
    Log,
}

#[derive(Clone, Copy)]
struct Geom {
    arc_r: f32,
    arc_stroke: f32,
    rim_r: f32,
    face_r: f32,
    inner_r: f32,
    indicator_inner: f32,
    indicator_outer: f32,
    indicator_w: f32,
    label_size: f32,
}

impl Geom {
    fn for_size(size: KnobSize) -> Self {
        match size {
            KnobSize::Small => Self {
                arc_r: 22.0,
                arc_stroke: 3.5,
                rim_r: 18.0,
                face_r: 14.0,
                inner_r: 0.0,
                indicator_inner: 8.0,
                indicator_outer: 16.0,
                indicator_w: 1.8,
                label_size: 9.5,
            },
            KnobSize::Medium => Self {
                arc_r: 34.0,
                arc_stroke: 5.0,
                rim_r: 27.0,
                face_r: 22.0,
                inner_r: 18.0,
                indicator_inner: 12.0,
                indicator_outer: 24.0,
                indicator_w: 2.4,
                label_size: 10.5,
            },
            KnobSize::Large => Self {
                arc_r: 52.0,
                arc_stroke: 6.0,
                rim_r: 42.0,
                face_r: 35.0,
                inner_r: 28.0,
                indicator_inner: 18.0,
                indicator_outer: 38.0,
                indicator_w: 3.0,
                label_size: 11.0,
            },
        }
    }
}

/// Rotary knob bound to a numeric value.
///
/// ```no_run
/// # use elegance::{Accent, Knob, KnobSize};
/// # egui::__run_test_ui(|ui| {
/// let mut gain = -12.0_f32;
/// ui.add(
///     Knob::new(&mut gain, -60.0..=12.0)
///         .label("Gain")
///         .size(KnobSize::Small)
///         .default(0.0_f32)
///         .value_fmt(|v| format!("{v:.0} dB")),
/// );
///
/// let mut dc = -1.4_f32;
/// ui.add(
///     Knob::new(&mut dc, -5.0..=5.0)
///         .label("DC offset")
///         .bipolar()
///         .accent(Accent::Purple)
///         .show_value(true),
/// );
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct Knob<'a, T: Numeric> {
    value: &'a mut T,
    range: RangeInclusive<T>,
    label: Option<WidgetText>,
    size: KnobSize,
    accent: Accent,
    bipolar: bool,
    detents: Option<Vec<(f64, String)>>,
    step: Option<f64>,
    scale: KnobScale,
    value_fmt: Option<Box<dyn Fn(f64) -> String + 'a>>,
    show_value: bool,
    default_value: Option<f64>,
    enabled: bool,
}

impl<'a, T: Numeric> std::fmt::Debug for Knob<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Knob")
            .field("range_lo", &self.range.start().to_f64())
            .field("range_hi", &self.range.end().to_f64())
            .field("size", &self.size)
            .field("accent", &self.accent)
            .field("bipolar", &self.bipolar)
            .field("detent_count", &self.detents.as_ref().map(Vec::len))
            .field("step", &self.step)
            .field("scale", &self.scale)
            .field("show_value", &self.show_value)
            .field("default", &self.default_value)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl<'a, T: Numeric> Knob<'a, T> {
    /// Create a knob bound to `value`, constrained to `range`.
    pub fn new(value: &'a mut T, range: RangeInclusive<T>) -> Self {
        Self {
            value,
            range,
            label: None,
            size: KnobSize::Medium,
            accent: Accent::Sky,
            bipolar: false,
            detents: None,
            step: None,
            scale: KnobScale::Linear,
            value_fmt: None,
            show_value: false,
            default_value: None,
            enabled: true,
        }
    }

    /// Show a label above the knob.
    ///
    /// The label also names the knob for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Pick a visual size. Default: [`KnobSize::Medium`].
    #[inline]
    pub fn size(mut self, size: KnobSize) -> Self {
        self.size = size;
        self
    }

    /// Pick the fill colour from one of the theme accents. Default: [`Accent::Sky`].
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Render as a bipolar knob: the active arc fills from the centre of the
    /// range toward the current value, suited to signed values (DC offset,
    /// pan, balance).
    #[inline]
    pub fn bipolar(mut self) -> Self {
        self.bipolar = true;
        self
    }

    /// Snap to a fixed list of `(value, label)` detents and render a labeled
    /// tick at each. Drag, scroll, and arrow keys step between detents.
    /// Existing [`step`](Self::step) is overridden.
    pub fn detents<I, S>(mut self, detents: I) -> Self
    where
        I: IntoIterator<Item = (T, S)>,
        S: Into<String>,
    {
        self.detents = Some(
            detents
                .into_iter()
                .map(|(v, lbl)| (v.to_f64(), lbl.into()))
                .collect(),
        );
        self
    }

    /// Snap continuous values to multiples of `step` (in the knob's value
    /// units). Integer-typed knobs snap to `1.0` automatically.
    /// Ignored if [`detents`](Self::detents) is set.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Use a logarithmic value mapping. Falls back to linear if the range
    /// minimum is non-positive.
    #[inline]
    pub fn log_scale(mut self) -> Self {
        self.scale = KnobScale::Log;
        self
    }

    /// Provide a value formatter for the optional inline display.
    pub fn value_fmt(mut self, fmt: impl Fn(f64) -> String + 'a) -> Self {
        self.value_fmt = Some(Box::new(fmt));
        self
    }

    /// Render the formatted value below the knob. Default: hidden.
    #[inline]
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Value to reset to on Alt+click, double-click, or `0` while focused. If
    /// unset, the reset gesture is a no-op.
    pub fn default(mut self, default: T) -> Self {
        self.default_value = Some(default.to_f64());
        self
    }

    /// Disable the knob — no pointer, scroll, or keyboard input. Default: enabled.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Position [0,1] along the 270-degree sweep, where 0 is the lower-left
/// (-135 degrees from north, going clockwise to +135 degrees).
fn pos_to_angle(pos: f32) -> f32 {
    // Egui's angle convention: 0 = east, +y is down. Convert from the
    // "0 at north, clockwise" convention used by the JS mockup.
    let north = -PI * 0.5;
    let from_north = (pos.clamp(0.0, 1.0) * 270.0 - 135.0).to_radians();
    north + from_north
}

fn radial_point(center: Pos2, r: f32, pos: f32) -> Pos2 {
    let a = pos_to_angle(pos);
    let (s, c) = a.sin_cos();
    pos2(center.x + r * c, center.y + r * s)
}

fn linear_value_to_pos(v: f64, lo: f64, hi: f64) -> f64 {
    if hi > lo {
        ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn log_value_to_pos(v: f64, lo: f64, hi: f64) -> f64 {
    let lmin = lo.ln();
    let lmax = hi.ln();
    ((v.max(lo).ln() - lmin) / (lmax - lmin)).clamp(0.0, 1.0)
}

fn pos_to_linear_value(p: f64, lo: f64, hi: f64) -> f64 {
    lo + p.clamp(0.0, 1.0) * (hi - lo)
}

fn pos_to_log_value(p: f64, lo: f64, hi: f64) -> f64 {
    let lmin = lo.ln();
    let lmax = hi.ln();
    (lmin + p.clamp(0.0, 1.0) * (lmax - lmin)).exp()
}

impl<'a, T: Numeric> Widget for Knob<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Destructure up front so the borrow checker sees independent
        // borrows for `value` (mutable) vs. config fields (shared) inside
        // the layout closure.
        let Knob {
            value,
            range,
            label,
            size,
            accent,
            bipolar,
            detents,
            step,
            scale,
            value_fmt,
            show_value,
            default_value,
            enabled,
        } = self;

        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;
        let accent_fill = p.accent_fill(accent);

        // ---- value range / current ----
        let lo_raw = range.start().to_f64();
        let hi_raw = range.end().to_f64();
        let (lo, hi) = if lo_raw <= hi_raw {
            (lo_raw, hi_raw)
        } else {
            (hi_raw, lo_raw)
        };
        let log_ok = matches!(scale, KnobScale::Log) && lo > 0.0;

        let value_to_pos = |v: f64| -> f64 {
            if log_ok {
                log_value_to_pos(v, lo, hi)
            } else {
                linear_value_to_pos(v, lo, hi)
            }
        };
        let pos_to_value = |p: f64| -> f64 {
            if log_ok {
                pos_to_log_value(p, lo, hi)
            } else {
                pos_to_linear_value(p, lo, hi)
            }
        };

        let snap = |v: f64| -> f64 {
            if let Some(d) = detents.as_ref() {
                if d.is_empty() {
                    return v.clamp(lo, hi);
                }
                let target_pos = value_to_pos(v);
                let mut best = d[0].0;
                let mut best_d = (value_to_pos(best) - target_pos).abs();
                for (dv, _) in d.iter().skip(1) {
                    let dd = (value_to_pos(*dv) - target_pos).abs();
                    if dd < best_d {
                        best_d = dd;
                        best = *dv;
                    }
                }
                return best;
            }
            let eff_step = step.or(if T::INTEGRAL { Some(1.0) } else { None });
            let mut snapped = v;
            if let Some(s) = eff_step
                && s > 0.0
            {
                snapped = lo + ((v - lo) / s).round() * s;
            }
            snapped.clamp(lo, hi)
        };

        let mut current = value.to_f64();
        if current.is_nan() {
            current = lo;
        }
        current = snap(current);
        // Write back any clamping/snapping so the bound state is consistent.
        if (current - value.to_f64()).abs() > f64::EPSILON {
            *value = T::from_f64(current);
        }

        // ---- geometry ----
        let g = Geom::for_size(size);
        let label_text = label
            .as_ref()
            .map(|l| l.text().to_string())
            .unwrap_or_default();

        // Tick label sizing for the labeled-detent variant.
        let detent_labels: Vec<(f32, String)> = detents
            .as_ref()
            .map(|d| {
                d.iter()
                    .filter(|(_, lbl)| !lbl.is_empty())
                    .map(|(v, lbl)| (value_to_pos(*v) as f32, lbl.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let tick_inner_r = g.arc_r + g.arc_stroke * 0.5 + 2.0;
        let tick_outer_r = tick_inner_r + 5.0;
        let label_gap = 6.0;
        let mut max_label_w: f32 = 0.0;
        let mut max_label_h: f32 = 0.0;
        if !detent_labels.is_empty() {
            for (_, txt) in &detent_labels {
                let galley =
                    crate::theme::placeholder_galley(ui, txt, g.label_size, false, f32::INFINITY);
                max_label_w = max_label_w.max(galley.size().x);
                max_label_h = max_label_h.max(galley.size().y);
            }
        }

        // Outer extent of the dial (centre to furthest visible pixel).
        let outer_r = if detent_labels.is_empty() {
            g.arc_r + g.arc_stroke * 0.5 + 4.0
        } else {
            tick_outer_r + label_gap + max_label_w.max(max_label_h)
        };
        // Add a touch of padding to keep AA edges clean.
        let dial_diameter = (outer_r * 2.0).ceil() + 2.0;

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;

            if !label_text.is_empty() {
                let rich = egui::RichText::new(&label_text)
                    .color(p.text_muted)
                    .size(t.label);
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
            }

            // Allocate the dial.
            let sense = if enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(dial_diameter), sense);
            let center = rect.center();

            // ---- interaction ----
            if enabled {
                crate::focus::focus_on_press(&response);

                if response.double_clicked()
                    && let Some(d) = default_value
                {
                    let snapped = snap(d.clamp(lo, hi));
                    if (snapped - current).abs() > f64::EPSILON {
                        current = snapped;
                        *value = T::from_f64(current);
                        response.mark_changed();
                    }
                }

                if response.drag_started() {
                    let alt = ui.input(|i| i.modifiers.alt);
                    if alt && let Some(d) = default_value {
                        let snapped = snap(d.clamp(lo, hi));
                        if (snapped - current).abs() > f64::EPSILON {
                            current = snapped;
                            *value = T::from_f64(current);
                            response.mark_changed();
                        }
                    }
                }

                if response.dragged() {
                    let alt = ui.input(|i| i.modifiers.alt);
                    if !alt {
                        // Combine horizontal and vertical motion: right = up = +.
                        // Diagonal drags add naturally so a single up-right or
                        // down-left flick reads as one gesture rather than two.
                        let delta = response.drag_delta();
                        let combined = (delta.x - delta.y) as f64;
                        let fine = ui.input(|i| i.modifiers.shift);
                        let sensitivity = if fine { 1.0 / 600.0 } else { 1.0 / 180.0 };
                        let cur_pos = value_to_pos(current);
                        let new_pos = (cur_pos + combined * sensitivity).clamp(0.0, 1.0);
                        let mut new_v = pos_to_value(new_pos);
                        new_v = snap(new_v);
                        if (new_v - current).abs() > f64::EPSILON {
                            current = new_v;
                            *value = T::from_f64(current);
                            response.mark_changed();
                        }
                    }
                }

                // A wheel notch does not arrive as one event: egui smooths the
                // delta across many frames, so the value moves on each of them.
                // Track that a scroll is in flight and report a single commit on
                // the frame it stops, rather than one per frame it lasted.
                let scroll_pending_id = response.id.with("elegance::knob_scroll_pending");
                let scrolling = ui.input(|i| i.smooth_scroll_delta.y.abs() > 0.5);
                let mut scroll_moved_value = false;

                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);

                    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll.abs() > 0.5 {
                        let dir = if scroll > 0.0 { 1.0 } else { -1.0 };
                        let fine = ui.input(|i| i.modifiers.shift);
                        let new_v = nudge_value(
                            current,
                            dir,
                            fine,
                            lo,
                            hi,
                            step,
                            T::INTEGRAL,
                            detents.as_deref(),
                            &value_to_pos,
                            &pos_to_value,
                        );
                        let new_v = snap(new_v);
                        if (new_v - current).abs() > f64::EPSILON {
                            current = new_v;
                            *value = T::from_f64(current);
                            response.mark_changed();
                            scroll_moved_value = true;
                        }
                    }
                }

                // Resolved outside the `hovered` block so a scroll that ends
                // after the pointer has moved away still commits.
                let scroll_was_pending: bool = ui
                    .ctx()
                    .data(|d| d.get_temp(scroll_pending_id).unwrap_or(false));
                if scroll_moved_value {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(scroll_pending_id, true));
                    // Guarantee a later frame in which to notice it stopped.
                    ui.ctx().request_repaint();
                } else if scroll_was_pending {
                    if scrolling {
                        ui.ctx().request_repaint();
                    } else {
                        ui.ctx().data_mut(|d| d.remove::<bool>(scroll_pending_id));
                        crate::commit::mark_commit(ui.ctx(), response.id);
                    }
                }

                if response.has_focus() {
                    // Claim every arrow for the value. Unlike the sliders, which
                    // bind only the horizontal pair and leave the vertical one
                    // to egui's spatial focus navigation, a knob reads all four
                    // (up and right increase, down and left decrease), so
                    // without this an arrow would nudge once and then hand focus
                    // to a neighbour. `Tab` and `Esc` still move focus away.
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            response.id,
                            egui::EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        );
                    });

                    let (up, down, page_up, page_down, home, end_, reset) = ui.input(|i| {
                        (
                            i.key_pressed(egui::Key::ArrowUp)
                                || i.key_pressed(egui::Key::ArrowRight),
                            i.key_pressed(egui::Key::ArrowDown)
                                || i.key_pressed(egui::Key::ArrowLeft),
                            i.key_pressed(egui::Key::PageUp),
                            i.key_pressed(egui::Key::PageDown),
                            i.key_pressed(egui::Key::Home),
                            i.key_pressed(egui::Key::End),
                            // Deliberately not Space: egui synthesises a click
                            // from it on any focused widget, and a knob is now
                            // focused by the pointer, so Space would wipe the
                            // value of the knob you had just set by hand.
                            i.key_pressed(egui::Key::Num0),
                        )
                    });
                    let fine = ui.input(|i| i.modifiers.shift);
                    let mut next = current;
                    if up {
                        next = nudge_value(
                            next,
                            1.0,
                            fine,
                            lo,
                            hi,
                            step,
                            T::INTEGRAL,
                            detents.as_deref(),
                            &value_to_pos,
                            &pos_to_value,
                        );
                    }
                    if down {
                        next = nudge_value(
                            next,
                            -1.0,
                            fine,
                            lo,
                            hi,
                            step,
                            T::INTEGRAL,
                            detents.as_deref(),
                            &value_to_pos,
                            &pos_to_value,
                        );
                    }
                    if page_up {
                        for _ in 0..4 {
                            next = nudge_value(
                                next,
                                1.0,
                                fine,
                                lo,
                                hi,
                                step,
                                T::INTEGRAL,
                                detents.as_deref(),
                                &value_to_pos,
                                &pos_to_value,
                            );
                        }
                    }
                    if page_down {
                        for _ in 0..4 {
                            next = nudge_value(
                                next,
                                -1.0,
                                fine,
                                lo,
                                hi,
                                step,
                                T::INTEGRAL,
                                detents.as_deref(),
                                &value_to_pos,
                                &pos_to_value,
                            );
                        }
                    }
                    if home {
                        next = lo;
                    }
                    if end_ {
                        next = hi;
                    }
                    if reset && let Some(d) = default_value {
                        next = d.clamp(lo, hi);
                    }
                    next = snap(next);
                    if (next - current).abs() > f64::EPSILON {
                        current = next;
                        *value = T::from_f64(current);
                        response.mark_changed();
                        // A key press is a discrete adjustment: already settled
                        // the moment it lands.
                        crate::commit::mark_commit(ui.ctx(), response.id);
                    }
                }
            }

            // ---- paint ----
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let track_color = p.depth_tint(p.card, 0.18);
                let pos = value_to_pos(current) as f32;

                // Track (full 270deg sweep).
                paint_arc(
                    painter,
                    center,
                    g.arc_r,
                    g.arc_stroke,
                    track_color,
                    0.0,
                    1.0,
                );

                // Active fill.
                if bipolar {
                    let lo_p = pos.min(0.5);
                    let hi_p = pos.max(0.5);
                    if (hi_p - lo_p).abs() > 1e-4 {
                        paint_arc(
                            painter,
                            center,
                            g.arc_r,
                            g.arc_stroke,
                            accent_fill,
                            lo_p,
                            hi_p,
                        );
                    }
                } else if pos > 1e-4 {
                    paint_arc(
                        painter,
                        center,
                        g.arc_r,
                        g.arc_stroke,
                        accent_fill,
                        0.0,
                        pos,
                    );
                }

                // Detent ticks + labels.
                if !detent_labels.is_empty() {
                    let active_pos = pos;
                    for (dpos, txt) in &detent_labels {
                        let hot = (dpos - active_pos).abs() < 1e-3;
                        let tick_color = if hot { p.text } else { p.border };
                        let a = radial_point(center, tick_inner_r, *dpos);
                        let b = radial_point(center, tick_outer_r, *dpos);
                        painter.line_segment([a, b], Stroke::new(1.0, tick_color));

                        let lp = radial_point(center, tick_outer_r + label_gap, *dpos);
                        // Anchor each label on the side of its rect closest to the
                        // knob centre so the text always extends *outward*. Without
                        // this, labels near 12 o'clock sit BELOW their anchor and
                        // crowd into the tick.
                        let dx = lp.x - center.x;
                        let dy = lp.y - center.y;
                        let h = if dx.abs() < 4.0 {
                            egui::Align::Center
                        } else if dx < 0.0 {
                            egui::Align::Max
                        } else {
                            egui::Align::Min
                        };
                        let v = if dy.abs() < 4.0 {
                            egui::Align::Center
                        } else if dy < 0.0 {
                            egui::Align::Max
                        } else {
                            egui::Align::Min
                        };
                        let anchor = Align2([h, v]);
                        let label_color = if hot { accent_fill } else { p.text_muted };
                        painter.text(
                            lp,
                            anchor,
                            txt,
                            FontId::proportional(g.label_size),
                            label_color,
                        );
                    }
                }

                // Body: rim + face + optional inner ring.
                let rim_fill = p.depth_tint(p.card, 0.12);
                let face_fill = p.card;
                painter.circle(center, g.rim_r, rim_fill, Stroke::new(1.0, p.border));
                painter.circle_filled(center, g.face_r, face_fill);
                if g.inner_r > 0.0 {
                    painter.circle_stroke(center, g.inner_r, Stroke::new(1.0, p.border));
                }

                // Indicator: line pointing along current angle, drawn from
                // a near-centre radius outward to near the rim. Coloured
                // accent so it reads against the card-tone face.
                let angle = pos_to_angle(pos);
                let (s, c) = angle.sin_cos();
                let dir = vec2(c, s);
                let a = center + dir * g.indicator_inner;
                let b = center + dir * g.indicator_outer;
                let ind_color = if enabled { accent_fill } else { p.text_faint };
                painter.line_segment([a, b], Stroke::new(g.indicator_w, ind_color));

                // Focus ring.
                if response.has_focus() {
                    painter.circle_stroke(
                        center,
                        g.rim_r + 4.0,
                        Stroke::new(1.5, with_alpha(p.focus, 180)),
                    );
                }
            }

            if show_value {
                let text = if let Some(f) = &value_fmt {
                    f(current)
                } else if T::INTEGRAL {
                    format!("{current:.0}")
                } else {
                    format!("{current:.2}")
                };
                let rich = egui::RichText::new(text)
                    .color(p.text)
                    .size(t.small)
                    .strong();
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
            }

            response.widget_info(|| WidgetInfo::labeled(Role::Slider, true, &label_text));
            response
        })
        .inner
    }
}

/// Compute one nudge step in value space. Direction is +/-1.
#[allow(clippy::too_many_arguments)]
fn nudge_value(
    current: f64,
    dir: f64,
    fine: bool,
    lo: f64,
    hi: f64,
    step: Option<f64>,
    integral: bool,
    detents: Option<&[(f64, String)]>,
    value_to_pos: &dyn Fn(f64) -> f64,
    pos_to_value: &dyn Fn(f64) -> f64,
) -> f64 {
    if let Some(detents) = detents
        && !detents.is_empty()
    {
        let mut sorted: Vec<f64> = detents.iter().map(|(v, _)| *v).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut idx = 0usize;
        let mut best = (sorted[0] - current).abs();
        for (i, v) in sorted.iter().enumerate().skip(1) {
            let dv = (v - current).abs();
            if dv < best {
                best = dv;
                idx = i;
            }
        }
        let next_idx = if dir > 0.0 {
            (idx + 1).min(sorted.len() - 1)
        } else {
            idx.saturating_sub(1)
        };
        return sorted[next_idx];
    }
    let effective_step = step.or(if integral { Some(1.0) } else { None });
    if let Some(s) = effective_step {
        let mult = if fine { 0.25 } else { 1.0 };
        return (current + dir * s * mult).clamp(lo, hi);
    }
    let frac = if fine { 1.0 / 200.0 } else { 1.0 / 40.0 };
    let cur_pos = value_to_pos(current);
    let new_pos = (cur_pos + dir * frac).clamp(0.0, 1.0);
    pos_to_value(new_pos)
}

fn paint_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    stroke: f32,
    color: Color32,
    p0: f32,
    p1: f32,
) {
    if (p1 - p0).abs() < 1e-4 {
        return;
    }
    let a0 = pos_to_angle(p0);
    let a1 = pos_to_angle(p1);
    // ~64 segments across the full 270deg sweep, scaled by the visible fraction.
    let n = ((p1 - p0).abs() * 96.0).ceil() as usize + 2;
    let points: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let a = a0 + (a1 - a0) * t;
            let (s, c) = a.sin_cos();
            pos2(center.x + radius * c, center.y + radius * s)
        })
        .collect();
    // Rounded endpoint caps so the arc doesn't read as a clipped rectangle.
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        painter.circle_filled(*first, stroke * 0.5, color);
        painter.circle_filled(*last, stroke * 0.5, color);
    }
    painter.add(PathShape::line(points, PathStroke::new(stroke, color)));
}
