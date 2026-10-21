//! Non-blocking notification toasts.
//!
//! Two types cooperate:
//!
//! * [`Toast`] — a builder that **enqueues** one notification via
//!   [`Toast::show`]. Takes only `&Context`, so it can fire from any
//!   callback that has access to the egui context (button handlers, input
//!   events, async completion callbacks, …).
//! * [`Toasts`] — the renderer. Call [`Toasts::new()`]`.render(ctx)` once
//!   per frame in your top-level `update`. Without this call, enqueued
//!   toasts silently accumulate and nothing is shown.
//!
//! # Usage
//!
//! ```no_run
//! # use elegance::{BadgeTone, Toast, Toasts};
//! # let ctx = egui::Context::default();
//! // Somewhere in your update loop:
//! Toasts::new().render(&ctx);
//!
//! // From any callback with access to the context:
//! Toast::new("Deploy complete")
//!     .tone(BadgeTone::Ok)
//!     .description("Rolled out to us-east-1")
//!     .show(&ctx);
//! ```

use std::{collections::VecDeque, time::Duration};

use egui::{
    Align2, Area, Color32, Context, CornerRadius, Id, Order, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, accesskit,
};

use crate::BadgeTone;
use crate::theme::Theme;

/// How long the fade-out animation takes, in seconds. Counted against
/// a toast's total lifetime (i.e., the toast disappears at
/// `birth + duration + FADE_OUT`).
const FADE_OUT: f64 = 0.20;
/// Default auto-dismiss duration, in seconds.
const DEFAULT_DURATION: f64 = 4.0;
/// Default stack cap — older toasts are dropped when this is exceeded.
const DEFAULT_MAX_VISIBLE: usize = 5;
/// Default width of a toast card, in points.
const DEFAULT_WIDTH: f32 = 320.0;
/// Vertical gap between stacked toasts, in points.
const STACK_GAP: f32 = 8.0;
/// Height of the optional "Clear all" pill, in points.
const CLEAR_ALL_HEIGHT: f32 = 26.0;
/// Gap between the "Clear all" pill and the nearest toast, in points.
const CLEAR_ALL_GAP: f32 = 6.0;
/// Stack size at or above which the "Clear all" pill becomes worth showing.
/// At a single toast the per-toast × is enough; the bulk-dismiss
/// affordance only earns its rendering cost once two or more pile up.
const CLEAR_ALL_THRESHOLD: usize = 2;

fn storage_id() -> Id {
    Id::unique("elegance::toasts")
}

/// A single enqueued notification.
///
/// Construct with [`Toast::new`], configure via the builder methods, then
/// call [`Toast::show`] to enqueue. The toast is rendered the next time
/// [`Toasts::render`] runs.
#[derive(Debug, Clone)]
#[must_use = "Call `show(ctx)` to enqueue the toast."]
pub struct Toast {
    title: String,
    description: Option<String>,
    tone: BadgeTone,
    duration: Option<Duration>,
}

impl Toast {
    /// Create a toast with a title. Defaults: [`BadgeTone::Info`], auto-dismiss
    /// after `DEFAULT_DURATION` seconds.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            tone: BadgeTone::Info,
            duration: Some(Duration::from_secs_f64(DEFAULT_DURATION)),
        }
    }

    /// Pick the tone (drives the left accent bar colour).
    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    /// Add a secondary line below the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Override how long the toast stays visible before it starts fading out.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Disable auto-dismiss. The toast stays until the user clicks × or
    /// another toast pushes it out of the stack (see [`Toasts::max_visible`]).
    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }

    /// Enqueue the toast. It is shown on the next frame that renders
    /// [`Toasts`].
    pub fn show(self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| {
            let mut state = d.get_temp::<ToastState>(storage_id()).unwrap_or_default();
            let id = state.next_id;
            state.next_id = state.next_id.wrapping_add(1);
            state.queue.push_back(ToastEntry {
                id,
                title: self.title,
                description: self.description,
                tone: self.tone,
                duration: self.duration.map(|d| d.as_secs_f64()),
                birth: now,
                dismiss_start: None,
                hover_pause_total: 0.0,
                hover_pause_started: None,
            });
            d.insert_temp(storage_id(), state);
        });
        ctx.request_repaint();
    }
}

/// Renderer for the enqueued toast stack.
///
/// Configure placement via the builder, then call [`Toasts::render`] once
/// per frame. Multiple `Toasts::render` calls per frame are a mistake —
/// each one will paint the whole stack.
#[derive(Debug, Clone)]
#[must_use = "Call `.render(ctx)` to draw the toast stack."]
pub struct Toasts {
    anchor: Align2,
    offset: Vec2,
    max_visible: usize,
    width: f32,
    clear_all_button: bool,
}

impl Default for Toasts {
    fn default() -> Self {
        Self::new()
    }
}

impl Toasts {
    /// Start a new configuration. Defaults: anchored to the bottom-right
    /// with a 12-pt offset, up to 5 toasts visible, 320-pt wide.
    pub fn new() -> Self {
        Self {
            anchor: Align2::RIGHT_BOTTOM,
            offset: Vec2::new(12.0, 12.0),
            max_visible: DEFAULT_MAX_VISIBLE,
            width: DEFAULT_WIDTH,
            clear_all_button: false,
        }
    }

    /// Anchor corner for the stack. Default: [`Align2::RIGHT_BOTTOM`].
    pub fn anchor(mut self, anchor: Align2) -> Self {
        self.anchor = anchor;
        self
    }

    /// Offset from the anchor corner, in points. Default: `(12, 12)`.
    pub fn offset(mut self, offset: impl Into<Vec2>) -> Self {
        self.offset = offset.into();
        self
    }

    /// Maximum number of toasts rendered at once. Oldest are dropped when
    /// the cap is exceeded. Default: 5.
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }

    /// Width of each toast card in points. Default: 320.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(120.0);
        self
    }

    /// Show a "Clear all" pill at the far end of the stack (above the
    /// topmost toast for bottom-anchored stacks, below the bottommost
    /// for top-anchored). Appears only when at least two non-dismissing
    /// toasts are visible; clicking it starts the fade-out animation on
    /// every entry at once. Default: `false`.
    pub fn clear_all_button(mut self, enabled: bool) -> Self {
        self.clear_all_button = enabled;
        self
    }

    /// Render the enqueued toast stack. Call once per frame.
    pub fn render(self, ctx: &Context) {
        let theme = Theme::current(ctx);
        let now = ctx.input(|i| i.time);

        // Snapshot state under a short lock, then hand the lock back.
        let mut state = ctx
            .data_mut(|d| d.get_temp::<ToastState>(storage_id()))
            .unwrap_or_default();

        // Expire fully-faded toasts, then cap the queue to max_visible by
        // dropping oldest (front).
        state.queue.retain(|entry| !entry.is_expired(now));
        while state.queue.len() > self.max_visible {
            state.queue.pop_front();
        }

        if state.queue.is_empty() {
            ctx.data_mut(|d| d.insert_temp(storage_id(), state));
            return;
        }

        // Paint toasts. We lay out manually (not via egui's own stacking)
        // so we can track sizes on the Area that each toast lives in.
        let screen = ctx.content_rect();
        let stack_up = matches!(self.anchor.y(), egui::Align::Max);

        // Compute each toast's height so we can stack them without depending
        // on previous-frame measurements.
        let entry_heights: Vec<f32> = state
            .queue
            .iter()
            .map(|e| measure_height(ctx, &theme, e, self.width))
            .collect();

        // x position of the stack.
        let x = match self.anchor.x() {
            egui::Align::Min => screen.min.x + self.offset.x,
            egui::Align::Center => screen.center().x - self.width * 0.5,
            egui::Align::Max => screen.max.x - self.offset.x - self.width,
        };

        // Starting y and step direction.
        let (mut y, step_sign): (f32, f32) = if stack_up {
            (screen.max.y - self.offset.y, -1.0)
        } else {
            (screen.min.y + self.offset.y, 1.0)
        };

        // Newest toast sits closest to the anchor edge; iterate accordingly.
        let order_is_new_to_old = stack_up;
        let indices: Vec<usize> = if order_is_new_to_old {
            (0..state.queue.len()).rev().collect()
        } else {
            (0..state.queue.len()).collect()
        };

        let mut dismiss_ids: Vec<u64> = Vec::new();
        let mut hovered_ids: Vec<u64> = Vec::new();
        let mut earliest_next_event: Option<f64> = None;
        let mut any_animating = false;

        for i in indices {
            let entry = &state.queue[i];
            let h = entry_heights[i];

            let (top, bottom) = if step_sign < 0.0 {
                (y - h, y)
            } else {
                (y, y + h)
            };
            let rect = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + self.width, bottom));

            // Animating = currently in fade-in or fade-out.
            let (alpha, is_animating, next_event) = entry.alpha_and_schedule(now);
            any_animating |= is_animating;
            if let Some(t) = next_event {
                earliest_next_event = Some(match earliest_next_event {
                    Some(prev) => prev.min(t),
                    None => t,
                });
            }

            let area_id = Id::unique(("elegance::toast", entry.id));
            let resp = Area::new(area_id)
                .order(Order::Tooltip)
                .fixed_pos(rect.min)
                .show(ctx, |ui| paint_toast(ui, &theme, entry, rect, alpha));

            let paint = resp.inner;
            if paint.close_clicked {
                dismiss_ids.push(entry.id);
            }
            if paint.hovered {
                hovered_ids.push(entry.id);
            }

            // Advance the cursor for the next toast.
            let delta = (h + STACK_GAP) * step_sign;
            y += delta;
        }

        // Record clicks into dismiss_start so next frame's alpha math picks
        // them up. Also commit any in-flight hover pause and stop tracking
        // it: alpha is now driven by `dismiss_start`, and leaving
        // `hover_pause_started` set would advance `current_pause` against
        // an auto-dismiss deadline that no longer applies.
        if !dismiss_ids.is_empty() {
            for entry in state.queue.iter_mut() {
                if dismiss_ids.contains(&entry.id) && entry.dismiss_start.is_none() {
                    entry.dismiss_start = Some(now);
                    if let Some(t0) = entry.hover_pause_started.take() {
                        entry.hover_pause_total += (now - t0).max(0.0);
                    }
                }
            }
        }

        // Hover-pause bookkeeping. Hovering freezes the auto-dismiss
        // countdown; releasing commits the paused interval. If hover
        // starts while the toast is already mid auto-fade, "rescue" it:
        // bump `hover_pause_total` so `fade_out_start` jumps to `now`,
        // restoring alpha to 1.0 and giving the user a fresh `duration`
        // window after they release.
        //
        // Repaint scheduling: `alpha_and_schedule` returns the
        // (pause-shifted) deadline as `next_event`, so the existing
        // `request_repaint_after` call sleeps until then; on wake-up we
        // recompute against the latest pause total. Hover transitions
        // are themselves pointer events that trigger a repaint.
        for entry in state.queue.iter_mut() {
            let is_hovered = entry.dismiss_start.is_none() && hovered_ids.contains(&entry.id);
            match (is_hovered, entry.hover_pause_started) {
                (true, None) => {
                    if let Some(d) = entry.duration {
                        let fade_out_start = entry.birth + d + entry.hover_pause_total;
                        if now > fade_out_start {
                            entry.hover_pause_total = now - entry.birth;
                        }
                    }
                    entry.hover_pause_started = Some(now);
                }
                (false, Some(t0)) => {
                    entry.hover_pause_total += (now - t0).max(0.0);
                    entry.hover_pause_started = None;
                }
                (true, Some(_)) | (false, None) => {}
            }
        }

        // Optional "Clear all" pill anchored at the far end of the stack.
        // Counted on non-dismissing entries so the pill hides as soon as
        // a bulk dismiss is triggered (those toasts then fade out).
        let active_count = state
            .queue
            .iter()
            .filter(|e| e.dismiss_start.is_none())
            .count();
        if self.clear_all_button && active_count >= CLEAR_ALL_THRESHOLD {
            let total_h: f32 = entry_heights.iter().sum::<f32>()
                + STACK_GAP * entry_heights.len().saturating_sub(1) as f32;
            let pill_top = if stack_up {
                (screen.max.y - self.offset.y) - total_h - CLEAR_ALL_GAP - CLEAR_ALL_HEIGHT
            } else {
                (screen.min.y + self.offset.y) + total_h + CLEAR_ALL_GAP
            };
            let pill_rect = Rect::from_min_size(
                Pos2::new(x, pill_top),
                Vec2::new(self.width, CLEAR_ALL_HEIGHT),
            );

            let area_id = Id::unique("elegance::toast::clear_all");
            let resp = Area::new(area_id)
                .order(Order::Tooltip)
                .fixed_pos(pill_rect.min)
                .show(ctx, |ui| paint_clear_all(ui, &theme, pill_rect));

            if resp.inner {
                for entry in state.queue.iter_mut() {
                    if entry.dismiss_start.is_none() {
                        entry.dismiss_start = Some(now);
                    }
                }
                any_animating = true;
            }
        }

        ctx.data_mut(|d| d.insert_temp(storage_id(), state));

        // Keep animating smoothly; otherwise schedule the next transition.
        if any_animating {
            ctx.request_repaint();
        } else if let Some(at) = earliest_next_event {
            let remaining = (at - now).max(0.0);
            ctx.request_repaint_after(Duration::from_secs_f64(remaining));
        }
    }
}

// -- internals ---------------------------------------------------------------

#[derive(Clone, Default)]
struct ToastState {
    queue: VecDeque<ToastEntry>,
    next_id: u64,
}

#[derive(Clone)]
struct ToastEntry {
    id: u64,
    title: String,
    description: Option<String>,
    tone: BadgeTone,
    /// Auto-dismiss duration in seconds. `None` = persistent.
    duration: Option<f64>,
    /// Context time when the toast was enqueued.
    birth: f64,
    /// Context time when the user clicked ×. Triggers an immediate fade-out.
    dismiss_start: Option<f64>,
    /// Total seconds the auto-dismiss timer has been frozen by the pointer
    /// hovering the toast (so the user can read or copy the text).
    hover_pause_total: f64,
    /// Context time when the current hover-pause started, or `None` when
    /// the pointer is not over this toast.
    hover_pause_started: Option<f64>,
}

impl ToastEntry {
    /// Total seconds the auto-dismiss timer is currently frozen, including
    /// any in-progress hover that hasn't been committed to `hover_pause_total`.
    fn current_pause(&self, now: f64) -> f64 {
        self.hover_pause_total
            + self
                .hover_pause_started
                .map(|t| (now - t).max(0.0))
                .unwrap_or(0.0)
    }

    /// Has the fade-out animation completed?
    fn is_expired(&self, now: f64) -> bool {
        if let Some(ds) = self.dismiss_start {
            return now >= ds + FADE_OUT;
        }
        if let Some(d) = self.duration {
            return now >= self.birth + d + self.current_pause(now) + FADE_OUT;
        }
        false
    }

    /// Returns `(alpha, is_animating, next_transition_time)`.
    ///
    /// Toasts appear at full opacity and fade out only. `is_animating` is
    /// true while the fade-out is in progress (we repaint continuously
    /// during it). `next_transition_time` is `Some(t)` when the toast is
    /// still at full opacity and we want a single deferred repaint at `t`
    /// to start the fade-out.
    fn alpha_and_schedule(&self, now: f64) -> (f32, bool, Option<f64>) {
        // Fade-out: either explicit dismiss, or past the (paused-shifted)
        // auto-dismiss instant.
        let fade_out_start = match self.dismiss_start {
            Some(ds) => Some(ds),
            None => self
                .duration
                .map(|d| self.birth + d + self.current_pause(now)),
        };

        match fade_out_start {
            Some(t0) if now >= t0 => {
                let progress = ((now - t0) / FADE_OUT).clamp(0.0, 1.0) as f32;
                (1.0 - progress, progress < 1.0, None)
            }
            // While hovered, the next transition slips forward (or jumps
            // far forward, if hover started during a fade and rescued
            // alpha back to 1.0). An already-scheduled repaint at `t0`
            // is fine: next frame we'll recompute against the new total.
            Some(t0) => (1.0, false, Some(t0)),
            None => (1.0, false, None),
        }
    }
}

fn tone_accent(theme: &Theme, tone: BadgeTone) -> Color32 {
    let p = &theme.palette;
    match tone {
        BadgeTone::Ok => p.success,
        BadgeTone::Warning => p.warning,
        BadgeTone::Danger => p.danger,
        BadgeTone::Info => p.focus,
        BadgeTone::Neutral => p.text_muted,
    }
}

/// Layout constants shared between measurement and painting.
mod layout {
    pub const PAD_X: f32 = 14.0;
    pub const PAD_Y: f32 = 10.0;
    pub const BAR_W: f32 = 3.0;
    pub const BAR_GAP: f32 = 10.0;
    pub const TITLE_DESC_GAP: f32 = 3.0;
    pub const CLOSE_W: f32 = 18.0;
    pub const CLOSE_GAP: f32 = 8.0;
    pub const TEXT_LEFT_NUDGE: f32 = 4.0;

    /// Shared so `measure_height` and `paint_toast` wrap against the same
    /// width — otherwise the stack lays out against a height the paint
    /// path doesn't reproduce.
    pub fn text_wrap_width(card_width: f32) -> f32 {
        (card_width - PAD_X * 1.5 - BAR_W - BAR_GAP - CLOSE_W - CLOSE_GAP + TEXT_LEFT_NUDGE)
            .max(1.0)
    }
}

fn measure_height(ctx: &Context, theme: &Theme, entry: &ToastEntry, width: f32) -> f32 {
    use layout::*;
    let p = &theme.palette;
    let t = &theme.typography;

    // Use the same colors here as the Label widgets in `paint_toast`, so
    // measure + paint pre-layout + Label all hit one cached galley.
    // Fade is applied via `ui.set_opacity`, not baked into colors.
    let text_width = text_wrap_width(width);
    let title_galley = ctx.fonts_mut(|f| {
        f.layout(
            entry.title.clone(),
            egui::FontId::proportional(t.body),
            p.text,
            text_width,
        )
    });

    let mut h = PAD_Y * 2.0 + title_galley.size().y;
    if let Some(desc) = &entry.description {
        let desc_galley = ctx.fonts_mut(|f| {
            f.layout(
                desc.clone(),
                egui::FontId::proportional(t.small),
                p.text_muted,
                text_width,
            )
        });
        h += TITLE_DESC_GAP + desc_galley.size().y;
    }
    h.max(44.0)
}

/// Result of painting a single toast for one frame.
struct ToastPaint {
    /// The close button was clicked this frame.
    close_clicked: bool,
    /// The pointer is currently over this toast (used to freeze the
    /// auto-dismiss timer so the user can read or copy the text).
    hovered: bool,
}

/// Paint a single toast inside its area.
fn paint_toast(
    ui: &mut Ui,
    theme: &Theme,
    entry: &ToastEntry,
    rect: Rect,
    alpha: f32,
) -> ToastPaint {
    use layout::*;
    let p = &theme.palette;
    let t = &theme.typography;

    // Upgrade the Area's Ui role from `GenericContainer` (set by
    // `Ui::new`) to an ARIA live-region role. Danger/Warning toasts use
    // `Role::Alert` (assertive — interrupts the user); others use
    // `Role::Status` (polite — announced after current speech).
    let role = match entry.tone {
        BadgeTone::Danger | BadgeTone::Warning => accesskit::Role::Alert,
        _ => accesskit::Role::Status,
    };
    let label = entry.title.clone();
    let description = entry.description.clone();
    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(role);
        node.set_label(label);
        if let Some(d) = description {
            node.set_description(d);
        }
    });

    // Apply fade-out as a painter-level opacity multiplier instead of
    // baking alpha into every color. This keeps the Label layout-job
    // cache key stable across fade frames (one cached galley reused for
    // the whole fade, instead of a fresh layout every frame).
    ui.set_opacity(alpha);

    // Claim the full toast rect so clicks don't pass through to widgets
    // beneath. `contains_pointer` (not `hovered`) so the child Labels
    // below can't shadow it when the cursor is over the text.
    let card_resp = ui.allocate_rect(rect, Sense::hover());
    let hovered = card_resp.contains_pointer();
    let painter = ui.painter();

    // Card background.
    painter.rect(
        rect,
        CornerRadius::same(theme.card_radius as u8),
        p.depth_tint(p.card, 0.04),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    // Left accent bar.
    let bar_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + 4.0, rect.min.y + 6.0),
        Pos2::new(rect.min.x + 4.0 + BAR_W, rect.max.y - 6.0),
    );
    painter.rect_filled(
        bar_rect,
        CornerRadius::same(2),
        tone_accent(theme, entry.tone),
    );

    // Close × in the top-right.
    let close_rect = Rect::from_min_size(
        Pos2::new(rect.max.x - PAD_X * 0.5 - CLOSE_W, rect.min.y + 6.0),
        Vec2::new(CLOSE_W, CLOSE_W),
    );
    let close_resp: Response = ui.allocate_rect(close_rect, Sense::click());
    let close_color = if close_resp.hovered() {
        p.text
    } else {
        p.text_muted
    };
    let close_galley = crate::theme::placeholder_galley(ui, "×", t.body + 2.0, true, f32::INFINITY);
    let close_text_pos = Pos2::new(
        close_rect.center().x - close_galley.size().x * 0.5,
        close_rect.center().y - close_galley.size().y * 0.5,
    );
    ui.painter()
        .galley(close_text_pos, close_galley, close_color);

    // Text block: title + optional description, to the right of the bar.
    // Rendered as `Label::selectable(true)` so the user can drag-select
    // and copy the contents (the dismiss timer pauses while hovered).
    let text_left = rect.min.x + PAD_X + BAR_W + BAR_GAP - TEXT_LEFT_NUDGE;
    let text_width = text_wrap_width(rect.width());

    // Pre-layout to learn each line's height. Same FontId/color/wrap as
    // `measure_height` and the Label below, so all three share one
    // cached galley per frame.
    let title_galley = ui.ctx().fonts_mut(|f| {
        f.layout(
            entry.title.clone(),
            egui::FontId::proportional(t.body),
            p.text,
            text_width,
        )
    });
    let title_size_y = title_galley.size().y;
    let title_rect = Rect::from_min_size(
        Pos2::new(text_left, rect.min.y + PAD_Y),
        Vec2::new(text_width, title_size_y),
    );
    let title_text = egui::RichText::new(&entry.title).color(p.text).size(t.body);
    put_wrapped_label(ui, title_rect, title_text);

    if let Some(desc) = &entry.description {
        let desc_galley = ui.ctx().fonts_mut(|f| {
            f.layout(
                desc.clone(),
                egui::FontId::proportional(t.small),
                p.text_muted,
                text_width,
            )
        });
        let desc_rect = Rect::from_min_size(
            Pos2::new(
                text_left,
                rect.min.y + PAD_Y + title_size_y + TITLE_DESC_GAP,
            ),
            Vec2::new(text_width, desc_galley.size().y),
        );
        let desc_text = egui::RichText::new(desc).color(p.text_muted).size(t.small);
        put_wrapped_label(ui, desc_rect, desc_text);
    }

    ToastPaint {
        close_clicked: close_resp.clicked(),
        hovered,
    }
}

/// Place a wrapping, selectable label in `rect`.
///
/// Not `Ui::put`: that imposes `Layout::centered_and_justified`, which `Label`
/// copies into `LayoutJob::justify`. Justification stretches every row but the
/// last out to the full wrap width, so a row holding one long word (a path, a
/// URL) comes back letter-spaced.
fn put_wrapped_label(ui: &mut Ui, rect: Rect, text: egui::RichText) {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
        |ui| ui.add(egui::Label::new(text).selectable(true).wrap()),
    );
}

/// Paint the "Clear all" pill that sits above (or below) the toast
/// stack when [`Toasts::clear_all_button`] is enabled and the stack has
/// at least [`CLEAR_ALL_THRESHOLD`] non-dismissing entries. Returns
/// `true` if it was clicked this frame.
fn paint_clear_all(ui: &mut Ui, theme: &Theme, rect: Rect) -> bool {
    let p = &theme.palette;
    let t = &theme.typography;

    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(accesskit::Role::Button);
        node.set_label("Clear all notifications");
    });

    let resp = ui.allocate_rect(rect, Sense::click());
    let painter = ui.painter();

    let bg = if resp.hovered() {
        p.depth_tint(p.card, 0.10)
    } else {
        p.depth_tint(p.card, 0.04)
    };
    let radius = CornerRadius::same((rect.height() * 0.5).round() as u8);
    painter.rect(
        rect,
        radius,
        bg,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    let text_color = if resp.hovered() { p.text } else { p.text_muted };
    let galley = crate::theme::placeholder_galley(ui, "Clear all", t.small, false, f32::INFINITY);
    let text_pos = Pos2::new(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    painter.galley(text_pos, galley, text_color);

    resp.clicked()
}
