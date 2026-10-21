//! Percentage slider — a 0–100% control whose central UI element is the
//! percentage value itself.
//!
//! [`PercentSlider`] is the percent-flavoured preset over
//! [`MetricSlider`](crate::MetricSlider). It locks the range to
//! `0.0..=100.0`, prints a small muted `%` suffix beside the headline, and
//! defaults the tick row to the quartile labels `0`, `25%`, `50%`, `75%`,
//! `100%`. Reach for [`MetricSlider`](crate::MetricSlider) when the value
//! carries its own absolute meaning instead.
//!
//! [`PercentSlider`] differs from [`Slider`](crate::Slider) in three ways:
//!
//! 1. The value is always a 0–100 percentage (`f32`). Pair it with
//!    [`PercentSlider::callout_fmt`] when the percentage maps to a meaningful
//!    absolute quantity (a duration, a file size, a budget share) and the
//!    absolute value will surface in a callout while the user drags.
//! 2. The visual hierarchy puts the percentage value front and centre,
//!    rendered large in the top-right of the widget. An optional small
//!    label sits in the top-left.
//! 3. Quartile ticks (`0`, `25%`, `50%`, `75%`, `100%`) sit beneath the
//!    track so "what fraction of the total am I setting?" reads at a
//!    glance. Hide them with [`PercentSlider::show_ticks`] for compact
//!    layouts.
//!
//! Three snap modes are available, in order of specificity:
//! [`step`](PercentSlider::step) snaps to multiples of a fixed size
//! (`5.0` → 0, 5, 10, …); [`steps`](PercentSlider::steps) snaps to `n`
//! evenly-spaced positions including both endpoints (`steps(5)` → 0, 25,
//! 50, 75, 100); [`stops`](PercentSlider::stops) snaps to an explicit,
//! possibly non-uniform list of positions. When `steps` or `stops` is
//! set, the tick row renders at exactly those positions and the arrow
//! keys jump between them.

use egui::{Response, Ui, Widget, WidgetText};

use crate::metric_slider::MetricSlider;
use crate::slider::SliderHandle;
use crate::theme::Accent;

/// A 0–100% slider whose central UI element is the percentage value itself.
///
/// ```no_run
/// # use elegance::PercentSlider;
/// # egui::__run_test_ui(|ui| {
/// let mut share = 45.0_f32;
/// ui.add(
///     PercentSlider::new(&mut share)
///         .label("Cache window")
///         .callout_fmt(|p| {
///             let mins = (p * 60.0 / 100.0).round() as i32;
///             format!("{mins} min")
///         }),
/// );
/// # });
/// ```
#[must_use = "Call `ui.add(...)` to render the widget."]
pub struct PercentSlider<'a> {
    inner: MetricSlider<'a>,
}

impl<'a> std::fmt::Debug for PercentSlider<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PercentSlider")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<'a> PercentSlider<'a> {
    /// Create a slider bound to `value`, clamped to `0.0..=100.0`.
    pub fn new(value: &'a mut f32) -> Self {
        Self {
            inner: MetricSlider::new(value, 0.0..=100.0)
                .suffix("%")
                .tick_fmt(|v| {
                    if v == 0.0 {
                        "0".to_string()
                    } else {
                        format!("{}%", v.round() as i32)
                    }
                }),
        }
    }

    /// Add a small label in the top-left of the header row.
    ///
    /// The label also names the slider for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.inner = self.inner.label(label);
        self
    }

    /// Pick the fill colour from one of the theme accents. Default: [`Accent::Sky`].
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.inner = self.inner.accent(accent);
        self
    }

    /// Show or hide the quartile tick row beneath the track. Default: `true`.
    #[inline]
    pub fn show_ticks(mut self, show: bool) -> Self {
        self.inner = self.inner.show_ticks(show);
        self
    }

    /// Snap the value to multiples of `step` percentage points. Default:
    /// continuous. Common values: `5.0` for "round to 5%", `25.0` for
    /// quartile snap.
    ///
    /// Mutually exclusive with [`steps`](Self::steps) and
    /// [`stops`](Self::stops); calling this clears either of them.
    #[inline]
    pub fn step(mut self, step: f32) -> Self {
        self.inner = self.inner.step(step);
        self
    }

    /// Snap to `n` evenly-spaced positions, including both endpoints.
    /// `steps(2)` snaps to `{0, 100}`, `steps(5)` to `{0, 25, 50, 75, 100}`,
    /// `steps(7)` to seven positions whose spacing the widget computes for
    /// you (so the awkward `100 / 6` math stays out of caller code).
    ///
    /// When set, the tick row renders at exactly those positions and the
    /// arrow keys jump between adjacent stops. Values below `2` are
    /// promoted to `2`. Mutually exclusive with [`step`](Self::step) and
    /// [`stops`](Self::stops); calling this clears either of them.
    #[inline]
    pub fn steps(mut self, n: usize) -> Self {
        self.inner = self.inner.steps(n);
        self
    }

    /// Snap to an explicit, possibly non-uniform list of positions in
    /// `0.0..=100.0`. Out-of-range, `NaN`, and duplicate values are filtered
    /// out; the result is sorted ascending. If fewer than two valid
    /// positions remain, falls back to `[0.0, 100.0]`.
    ///
    /// When set, the tick row renders at exactly these positions and the
    /// arrow keys jump between adjacent stops. Mutually exclusive with
    /// [`step`](Self::step) and [`steps`](Self::steps); calling this clears
    /// either of them.
    #[inline]
    pub fn stops(mut self, positions: impl IntoIterator<Item = f32>) -> Self {
        self.inner = self.inner.stops(positions);
        self
    }

    /// Number of decimal places in the headline value. Default: `0`.
    #[inline]
    pub fn decimals(mut self, n: usize) -> Self {
        self.inner = self.inner.decimals(n);
        self
    }

    /// Supply a callback to format the *entire* drag-callout text. The
    /// callback receives the current percentage in `0.0..=100.0` and returns
    /// the string to render in the callout above the thumb while the user
    /// drags. When unset, no callout is shown.
    ///
    /// The callback has full control over the text. The widget does not
    /// prepend the percentage. Common patterns:
    ///
    /// ```ignore
    /// // Just the absolute quantity (the headline already shows the percent):
    /// .callout_fmt(|p| format!("{} min", (p * 60.0 / 100.0).round() as i32))
    ///
    /// // Percent and absolute together:
    /// .callout_fmt(|p| {
    ///     let mins = (p * 60.0 / 100.0).round() as i32;
    ///     format!("{}% \u{00B7} {} min", p.round() as i32, mins)
    /// })
    /// ```
    #[inline]
    pub fn callout_fmt(mut self, fmt: impl Fn(f32) -> String + 'a) -> Self {
        self.inner = self.inner.callout_fmt(fmt);
        self
    }

    /// Override the slider width. Defaults to `ui.available_width()`.
    #[inline]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.inner = self.inner.desired_width(width);
        self
    }

    /// Pick the thumb shape. Default: [`SliderHandle::Circle`]. Switch to
    /// [`SliderHandle::Line`] for a thin vertical bar instead of the standard
    /// circular knob.
    #[inline]
    pub fn handle(mut self, handle: SliderHandle) -> Self {
        self.inner = self.inner.handle(handle);
        self
    }
}

impl<'a> Widget for PercentSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.inner.ui(ui)
    }
}
