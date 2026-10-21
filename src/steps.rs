//! Stepped progress — a sequence of discrete steps with per-step state.
//!
//! Three visual styles share the same state model:
//! - [`StepsStyle::Cells`] renders a segmented bar of uniform rounded cells
//!   with small gaps between them. Compact "N of M" progress.
//! - [`StepsStyle::Numbered`] renders numbered circles connected by thin
//!   lines. Done steps show a checkmark; the active step glows. Suits
//!   user-facing wizard and onboarding flows. Pair with [`Steps::labeled`]
//!   to render captions under each circle, and
//!   [`Steps::active_sublabel`] for an "in progress" hint under the
//!   active step.
//! - [`StepsStyle::Labeled`] renders a sequence of labeled pills — taller
//!   cells containing a text label. Horizontal by default (a progress bar
//!   with readable stage names); flip to vertical with
//!   [`Steps::vertical`] for wizard sidebars and checklist flows. Pair
//!   with [`Steps::labeled`] to supply the labels.
//!
//! All styles read the same three fields: `total` steps, `current` = how
//! many are complete (0..=total), and `errored` = whether the current
//! step failed.

use egui::{
    Color32, CornerRadius, Painter, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    Vec2, Widget, WidgetInfo,
};

use crate::theme::{Theme, with_alpha};

/// Visual style for a [`Steps`] widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepsStyle {
    /// Segmented bar of uniform rounded cells with small gaps.
    Cells,
    /// Numbered circles connected by thin lines. Done dots show a check;
    /// the active dot glows.
    Numbered,
    /// Labeled pills — taller cells containing a text label. Horizontal by
    /// default; flip with [`Steps::vertical`] for a wizard sidebar.
    Labeled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepState {
    Done,
    Active,
    Error,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Orient {
    Horizontal,
    Vertical,
}

/// A stepped progress indicator.
///
/// ```no_run
/// # use elegance::{Steps, StepsStyle};
/// # egui::__run_test_ui(|ui| {
/// // Release pipeline: 4 of 6 steps complete, step 5 running.
/// ui.add(Steps::new(6).current(4));
///
/// // Migration failed on step 3.
/// ui.add(Steps::new(5).current(2).errored(true));
///
/// // Onboarding — numbered circles.
/// ui.add(Steps::new(5).current(2).style(StepsStyle::Numbered));
///
/// // Setup wizard — numbered circles with labels and an "in progress"
/// // hint under the active step.
/// ui.add(
///     Steps::labeled(["Account", "Workspace", "Billing", "Integrations", "Review"])
///         .style(StepsStyle::Numbered)
///         .current(2)
///         .active_sublabel("In progress"),
/// );
///
/// // Horizontal labeled strip — a progress bar with stage names.
/// ui.add(
///     Steps::labeled(["Plan", "Build", "Test", "Deploy"])
///         .current(2),
/// );
///
/// // Vertical wizard sidebar.
/// ui.add(
///     Steps::labeled(["Plan", "Design", "Build", "Test", "Deploy"])
///         .current(2)
///         .vertical(),
/// );
/// # });
/// ```
#[derive(Debug, Clone)]
#[must_use = "Add with `ui.add(...)`."]
pub struct Steps {
    total: usize,
    current: usize,
    errored: bool,
    style: StepsStyle,
    orientation: Orient,
    labels: Vec<String>,
    active_sublabel: Option<String>,
    height: Option<f32>,
    desired_width: Option<f32>,
}

impl Steps {
    /// Create a cells-style stepped bar with `total` steps (clamped to at
    /// least 1), all pending.
    pub fn new(total: usize) -> Self {
        Self {
            total: total.max(1),
            current: 0,
            errored: false,
            style: StepsStyle::Cells,
            orientation: Orient::Horizontal,
            labels: Vec::new(),
            active_sublabel: None,
            height: None,
            desired_width: None,
        }
    }

    /// Create a [`StepsStyle::Labeled`] widget whose step count and labels
    /// come from `labels`. Horizontal by default; call [`Self::vertical`]
    /// for a wizard-sidebar layout. All steps start pending; add
    /// `.current(n)` to mark the first `n` as done.
    ///
    /// Pair with `.style(StepsStyle::Numbered)` to render the same labels
    /// as captions under numbered circles, an onboarding stepper layout.
    pub fn labeled(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        Self {
            total: labels.len().max(1),
            current: 0,
            errored: false,
            style: StepsStyle::Labeled,
            orientation: Orient::Horizontal,
            labels,
            active_sublabel: None,
            height: None,
            desired_width: None,
        }
    }

    /// Render labeled cells stacked vertically. Only affects
    /// [`StepsStyle::Labeled`].
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.orientation = Orient::Vertical;
        self
    }

    /// Render labeled cells arranged horizontally (the default for
    /// [`Steps::labeled`]). Provided as the explicit counterpart to
    /// [`Self::vertical`].
    #[inline]
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orient::Horizontal;
        self
    }

    /// Set how many steps are complete. Clamped to `0..=total`.
    #[inline]
    pub fn current(mut self, current: usize) -> Self {
        self.current = current.min(self.total);
        self
    }

    /// When `true`, paint the step at `current` as errored instead of
    /// active. No effect when `current == total` (nothing to error on).
    #[inline]
    pub fn errored(mut self, errored: bool) -> Self {
        self.errored = errored;
        self
    }

    /// Pick the visual style. Default: [`StepsStyle::Cells`].
    #[inline]
    pub fn style(mut self, style: StepsStyle) -> Self {
        self.style = style;
        self
    }

    /// Caption shown directly under the active step's label (numbered
    /// style with labels only). Useful for stepper-style "In progress" or
    /// "Action required" hints. No effect on other styles or when the
    /// pipeline has no active step.
    #[inline]
    pub fn active_sublabel(mut self, text: impl Into<String>) -> Self {
        self.active_sublabel = Some(text.into());
        self
    }

    /// Override the cell height (cells style) or dot diameter (numbered
    /// style). Defaults: 6 for cells, 22 for numbered.
    #[inline]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Override the total width. Defaults to `ui.available_width()`.
    #[inline]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    fn step_state(&self, i: usize) -> StepState {
        if i < self.current {
            StepState::Done
        } else if i == self.current && self.current < self.total {
            if self.errored {
                StepState::Error
            } else {
                StepState::Active
            }
        } else {
            StepState::Pending
        }
    }
}

impl Widget for Steps {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.style {
            StepsStyle::Cells => paint_cells(ui, &self),
            StepsStyle::Numbered => paint_numbered(ui, &self),
            StepsStyle::Labeled => paint_labeled(ui, &self),
        }
    }
}

fn paint_cells(ui: &mut Ui, s: &Steps) -> Response {
    let theme = Theme::current(ui.ctx());
    let p = &theme.palette;

    let height = s.height.unwrap_or(6.0);
    let gap = 4.0;
    let width = s
        .desired_width
        .unwrap_or_else(|| ui.available_width())
        .max(height * 2.0);

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let n = s.total as f32;
        let total_gap = gap * (n - 1.0).max(0.0);
        let cell_w = ((width - total_gap) / n).max(1.0);
        let radius = CornerRadius::same((height * 0.65).round().clamp(2.0, 8.0) as u8);
        let pending_fill = p.depth_tint(p.card, 0.08);

        for i in 0..s.total {
            let x = rect.min.x + (cell_w + gap) * i as f32;
            let cell_rect =
                Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(cell_w, height));
            let fill = match s.step_state(i) {
                StepState::Done => p.success,
                StepState::Active => p.focus,
                StepState::Error => p.danger,
                StepState::Pending => pending_fill,
            };
            painter.rect(cell_rect, radius, fill, Stroke::NONE, StrokeKind::Inside);
        }
    }

    response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "progress"));
    response
}

fn paint_numbered(ui: &mut Ui, s: &Steps) -> Response {
    let theme = Theme::current(ui.ctx());
    let p = &theme.palette;
    let t = &theme.typography;

    let dot_d = s.height.unwrap_or(22.0);
    let dot_r = dot_d * 0.5;
    let conn_h = (dot_d * 0.09).max(1.5);
    let conn_inset = 4.0;
    let width = s
        .desired_width
        .unwrap_or_else(|| ui.available_width())
        .max(dot_d * s.total as f32);

    let has_labels = !s.labels.is_empty();
    let has_sublabel = has_labels && s.active_sublabel.is_some() && s.current < s.total;
    let label_gap = 8.0;
    let sublabel_gap = 2.0;
    let label_block = if has_labels { t.body + label_gap } else { 0.0 };
    let sublabel_block = if has_sublabel {
        t.small + sublabel_gap
    } else {
        0.0
    };
    let total_h = dot_d + label_block + sublabel_block;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, total_h), Sense::hover());

    if !ui.is_rect_visible(rect) {
        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "progress"));
        return response;
    }

    let painter = ui.painter();
    let center_y = rect.min.y + dot_r;

    let dot_center_x = |i: usize| -> f32 {
        if s.total == 1 {
            rect.center().x
        } else {
            let f = i as f32 / (s.total - 1) as f32;
            rect.min.x + dot_r + f * (width - dot_d)
        }
    };

    let pending_fill = p.depth_tint(p.card, 0.08);

    for i in 0..s.total.saturating_sub(1) {
        let x_start = dot_center_x(i) + dot_r + conn_inset;
        let x_end = dot_center_x(i + 1) - dot_r - conn_inset;
        if x_end <= x_start {
            continue;
        }
        let conn_rect = Rect::from_min_max(
            Pos2::new(x_start, center_y - conn_h * 0.5),
            Pos2::new(x_end, center_y + conn_h * 0.5),
        );
        let color = match s.step_state(i) {
            StepState::Done => p.success,
            _ => pending_fill,
        };
        painter.rect_filled(conn_rect, CornerRadius::ZERO, color);
    }

    for i in 0..s.total {
        let center = Pos2::new(dot_center_x(i), center_y);
        let state = s.step_state(i);
        let (fill, text_color) = match state {
            StepState::Done => (p.success, Color32::WHITE),
            StepState::Active => (p.focus, Color32::WHITE),
            StepState::Error => (p.danger, Color32::WHITE),
            StepState::Pending => (pending_fill, p.text_muted),
        };

        if matches!(state, StepState::Active) {
            painter.circle_filled(center, dot_r + 3.0, with_alpha(p.focus, 64));
        }

        painter.circle_filled(center, dot_r, fill);

        if matches!(state, StepState::Done) {
            paint_check(painter, center, dot_r * 0.45, text_color);
        } else {
            let label = (i + 1).to_string();
            let font_size = (dot_d * 0.55).max(10.0).min(t.body);
            let galley =
                crate::theme::placeholder_galley(ui, &label, font_size, true, f32::INFINITY);
            let pos = Pos2::new(
                center.x - galley.size().x * 0.5,
                center.y - galley.size().y * 0.5,
            );
            painter.galley(pos, galley, text_color);
        }

        if has_labels && let Some(label_text) = s.labels.get(i) {
            let is_active = matches!(state, StepState::Active | StepState::Error);
            let label_color = match state {
                StepState::Done => p.text_muted,
                StepState::Active => p.text,
                StepState::Error => p.danger,
                StepState::Pending => p.text_muted,
            };
            let label_galley =
                crate::theme::placeholder_galley(ui, label_text, t.body, is_active, f32::INFINITY);
            let label_y = rect.min.y + dot_d + label_gap;
            let pos = Pos2::new(center.x - label_galley.size().x * 0.5, label_y);
            painter.galley(pos, label_galley, label_color);

            if has_sublabel
                && i == s.current
                && let Some(sub) = s.active_sublabel.as_deref()
            {
                let sub_galley =
                    crate::theme::placeholder_galley(ui, sub, t.small, false, f32::INFINITY);
                let sub_y = label_y + t.body + sublabel_gap;
                let sub_pos = Pos2::new(center.x - sub_galley.size().x * 0.5, sub_y);
                painter.galley(sub_pos, sub_galley, p.text_faint);
            }
        }
    }

    response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "progress"));
    response
}

fn paint_labeled(ui: &mut Ui, s: &Steps) -> Response {
    let theme = Theme::current(ui.ctx());
    let p = &theme.palette;
    let t = &theme.typography;

    let pill_h = s.height.unwrap_or(32.0);
    let horizontal = matches!(s.orientation, Orient::Horizontal);
    // Wider gaps on Labeled so a chevron connector fits cleanly between cells.
    let gap = if horizontal { 22.0 } else { 20.0 };
    let icon_gap = 6.0;
    let pending_fill = p.depth_tint(p.card, 0.08);
    let n = s.total;
    let width = s.desired_width.unwrap_or_else(|| ui.available_width());

    let (alloc_size, cell_w) = if horizontal {
        let total_gap = gap * n.saturating_sub(1) as f32;
        let cell_w = ((width - total_gap) / n as f32).max(1.0);
        (Vec2::new(width, pill_h), cell_w)
    } else {
        let total_h = pill_h * n as f32 + gap * n.saturating_sub(1) as f32;
        (Vec2::new(width, total_h), width)
    };

    let (rect, response) = ui.allocate_exact_size(alloc_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same((pill_h * 0.22).round().clamp(4.0, 12.0) as u8);
        let chevron_color = p.text_faint;

        for i in 0..n {
            let cell_rect = if horizontal {
                let x = rect.min.x + (cell_w + gap) * i as f32;
                Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(cell_w, pill_h))
            } else {
                let y = rect.min.y + (pill_h + gap) * i as f32;
                Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(cell_w, pill_h))
            };

            if i + 1 < n {
                let (chev_center, direction) = if horizontal {
                    (
                        Pos2::new(cell_rect.max.x + gap * 0.5, cell_rect.center().y),
                        ChevronDir::Right,
                    )
                } else {
                    (
                        Pos2::new(cell_rect.center().x, cell_rect.max.y + gap * 0.5),
                        ChevronDir::Down,
                    )
                };
                paint_chevron(painter, chev_center, direction, chevron_color);
            }

            let state = s.step_state(i);
            let (fill, text_color) = match state {
                StepState::Done => (p.success, Color32::WHITE),
                StepState::Active => (p.focus, Color32::WHITE),
                StepState::Error => (p.danger, Color32::WHITE),
                StepState::Pending => (pending_fill, p.text_muted),
            };
            painter.rect(cell_rect, radius, fill, Stroke::NONE, StrokeKind::Inside);

            let label = s.labels.get(i).map(String::as_str).unwrap_or("");
            let galley = if label.is_empty() {
                None
            } else {
                Some(crate::theme::placeholder_galley(
                    ui,
                    label,
                    t.body,
                    true,
                    f32::INFINITY,
                ))
            };
            let galley_w = galley.as_ref().map_or(0.0, |g| g.size().x);
            let galley_h = galley.as_ref().map_or(0.0, |g| g.size().y);
            let check_scale = pill_h * 0.2;

            if horizontal {
                // Centered group: optional check + label, clipped to the cell.
                let has_check = matches!(state, StepState::Done);
                let check_block = if has_check {
                    check_scale * 2.0 + icon_gap
                } else {
                    0.0
                };
                let group_w = check_block + galley_w;
                let start_x = cell_rect.center().x - group_w * 0.5;
                let clip = painter.clip_rect().intersect(cell_rect.shrink(2.0));
                let clipped = painter.with_clip_rect(clip);

                let mut cursor_x = start_x;
                if has_check {
                    let check_center = Pos2::new(start_x + check_scale, cell_rect.center().y);
                    paint_check(&clipped, check_center, check_scale, text_color);
                    cursor_x = check_center.x + check_scale + icon_gap;
                }
                if let Some(g) = galley {
                    let pos = Pos2::new(cursor_x, cell_rect.center().y - galley_h * 0.5);
                    clipped.galley(pos, g, text_color);
                }
            } else {
                // Left-aligned: optional check, then label.
                let pad_x = 12.0;
                let mut text_x = cell_rect.min.x + pad_x;
                if matches!(state, StepState::Done) {
                    let check_center = Pos2::new(text_x + check_scale, cell_rect.center().y);
                    paint_check(painter, check_center, check_scale, text_color);
                    text_x = check_center.x + check_scale + icon_gap;
                }
                if let Some(g) = galley {
                    let pos = Pos2::new(text_x, cell_rect.center().y - galley_h * 0.5);
                    painter.galley(pos, g, text_color);
                }
            }
        }
    }

    response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, true, "progress"));
    response
}

fn paint_check(painter: &Painter, center: Pos2, scale: f32, color: Color32) {
    let stroke = Stroke::new((scale * 0.45).max(1.5), color);
    let a = Pos2::new(center.x - scale, center.y);
    let b = Pos2::new(center.x - scale * 0.375, center.y + scale * 0.625);
    let c = Pos2::new(center.x + scale, center.y - scale * 0.75);
    painter.line_segment([a, b], stroke);
    painter.line_segment([b, c], stroke);
}

#[derive(Clone, Copy)]
enum ChevronDir {
    Right,
    Down,
}

fn paint_chevron(painter: &Painter, center: Pos2, dir: ChevronDir, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    // Apex angle = 2·atan(w / 2d). Pick d=3, w=2d·tan(60°) ≈ 10.4 for 120°.
    let d = 3.0;
    let w = 10.4;
    let (a, apex, b) = match dir {
        ChevronDir::Right => (
            Pos2::new(center.x - d, center.y - w),
            Pos2::new(center.x + d, center.y),
            Pos2::new(center.x - d, center.y + w),
        ),
        ChevronDir::Down => (
            Pos2::new(center.x - w, center.y - d),
            Pos2::new(center.x, center.y + d),
            Pos2::new(center.x + w, center.y - d),
        ),
    };
    painter.line_segment([a, apex], stroke);
    painter.line_segment([apex, b], stroke);
}
