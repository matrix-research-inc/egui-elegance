//! Drawer — a side-anchored slide-in overlay panel.
//!
//! A [`Drawer`] is a full-height panel that slides in from the left or right
//! edge of the viewport over a dimmed backdrop. The panel is dismissed by
//! pressing `Esc`, clicking the backdrop, or clicking the built-in close
//! button. Use one for record inspectors, edit forms, filter sidebars, and
//! anything else that's too tall for a [`Modal`](crate::Modal) but doesn't
//! deserve a route of its own.
//!
//! ```no_run
//! # use elegance::{Drawer, DrawerSide};
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! Drawer::new("inspector", &mut open)
//!     .side(DrawerSide::Right)
//!     .width(420.0)
//!     .title("INC-2187")
//!     .subtitle("api-west-02 latency spike")
//!     .show(&ctx, |ui| {
//!         ui.label("…drawer body, scroll if needed…");
//!     });
//! ```
//!
//! # Layout inside the body closure
//!
//! The panel is full-height. For the common header/scrollable-body/pinned-
//! footer layout, slice the body's vertical space yourself:
//!
//! ```no_run
//! # use elegance::{Accent, Button, Drawer};
//! # let ctx = egui::Context::default();
//! # let mut open = true;
//! Drawer::new("edit", &mut open).title("Edit member").show(&ctx, |ui| {
//!     let footer_h = 56.0;
//!     let body_h = (ui.available_height() - footer_h).max(0.0);
//!     ui.allocate_ui_with_layout(
//!         egui::vec2(ui.available_width(), body_h),
//!         egui::Layout::top_down(egui::Align::Min),
//!         |ui| {
//!             egui::ScrollArea::vertical().show(ui, |ui| {
//!                 ui.label("…form fields…");
//!             });
//!         },
//!     );
//!     ui.horizontal(|ui| {
//!         let _ = ui.add(Button::new("Save").accent(Accent::Blue));
//!         let _ = ui.add(Button::new("Cancel").outline());
//!     });
//! });
//! ```
//!
//! For *persistent* (non-overlay) side panels, reach for [`egui::SidePanel`]
//! directly: it integrates with the surrounding layout so the main content
//! resizes around it. `Drawer` is for the modal slide-in case.

use egui::{
    Align, Area, Color32, Context, CornerRadius, Frame, Id, Layout, Margin, Pos2, Rect, Sense,
    Stroke, Ui, UiBuilder, WidgetText, accesskit, emath, epaint::Shadow,
};

use crate::{overlay, theme::Theme};

/// Which edge of the viewport the drawer slides in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawerSide {
    /// Slide in from the left edge.
    Left,
    /// Slide in from the right edge. The default.
    Right,
}

/// A side-anchored slide-in overlay panel.
///
/// The `open` flag drives visibility. While it transitions from `false` to
/// `true` the panel slides in from its anchored edge; the reverse plays in
/// reverse. Pressing `Esc`, clicking the dimmed backdrop, or clicking the
/// built-in close "×" button flips it back to `false`.
///
/// A drawer is modal for as long as it is on screen: the backdrop swallows
/// clicks meant for anything beneath it (an [`egui::Window`] included), and
/// keeps doing so while the panel slides shut. A [`Modal`](crate::Modal)
/// opened over a drawer takes precedence for `Esc`, and dismissing it hands
/// the key back to the drawer; the press is consumed either way, unless the
/// drawer has opted out via [`Drawer::close_on_escape`].
/// [`Toasts`](crate::Toasts) stay above and remain clickable throughout.
///
/// Keyboard focus moves into the panel on open and returns to the previously
/// focused widget on close, but `Tab` is not fenced in: it can still reach
/// widgets behind the drawer.
#[must_use = "Call `.show(ctx, |ui| { ... })` to render the drawer."]
pub struct Drawer<'a> {
    id_salt: Id,
    open: &'a mut bool,
    side: DrawerSide,
    width: f32,
    title: Option<WidgetText>,
    subtitle: Option<WidgetText>,
    close_on_backdrop: bool,
    close_on_escape: bool,
}

impl<'a> std::fmt::Debug for Drawer<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawer")
            .field("id_salt", &self.id_salt)
            .field("open", &*self.open)
            .field("side", &self.side)
            .field("width", &self.width)
            .field("title", &self.title.as_ref().map(|t| t.text()))
            .field("subtitle", &self.subtitle.as_ref().map(|t| t.text()))
            .field("close_on_backdrop", &self.close_on_backdrop)
            .field("close_on_escape", &self.close_on_escape)
            .finish()
    }
}

impl<'a> Drawer<'a> {
    /// Create a drawer keyed by `id_salt` whose visibility is bound to `open`.
    /// Defaults: anchored to the right, 420 pt wide, no title, dismisses on
    /// `Esc` and backdrop click.
    pub fn new(id_salt: impl crate::IdSalt, open: &'a mut bool) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            open,
            side: DrawerSide::Right,
            width: 420.0,
            title: None,
            subtitle: None,
            close_on_backdrop: true,
            close_on_escape: true,
        }
    }

    /// Anchor the drawer to the left or right edge. Default: [`DrawerSide::Right`].
    #[inline]
    pub fn side(mut self, side: DrawerSide) -> Self {
        self.side = side;
        self
    }

    /// Set the panel width in points. Default: 420. Clamped to at least 120.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(120.0);
        self
    }

    /// Show a strong title at the top of the drawer, alongside the close "×"
    /// button. When unset, no automatic chrome is rendered and the body
    /// closure receives the full panel area.
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show a muted subtitle line below the title. Has no effect when
    /// [`Drawer::title`] is unset.
    pub fn subtitle(mut self, subtitle: impl Into<WidgetText>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Whether clicking the dimmed backdrop dismisses the drawer. Default: `true`.
    #[inline]
    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    /// Whether pressing `Esc` dismisses the drawer. Default: `true`.
    #[inline]
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Render the drawer. Returns `None` while the drawer is fully closed
    /// (off-screen and not animating); otherwise `Some(R)` with the body
    /// closure's return value.
    ///
    /// The closure is invoked every frame the panel is on-screen, including
    /// while it slides in or out. Treat the body as ordinary layout — the
    /// slide is applied by moving the panel's rect within a viewport-sized
    /// surface, and content that runs past the edge is clipped.
    pub fn show<R>(self, ctx: &Context, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        // --- Lifecycle: track open/closed transitions for focus restoration. ---
        let focus_storage = Id::unique(("elegance_drawer_focus", self.id_salt));
        let mut focus_state = overlay::FocusState::load(ctx, focus_storage);
        let is_open = *self.open;
        let was_open = focus_state.was_open;
        let just_opened = is_open && !was_open;
        let just_closed = !is_open && was_open;

        // --- Slide animation. 0.0 = fully off-screen, 1.0 = fully on. ---
        let progress = ctx.animate_bool_with_time_and_easing(
            Id::unique(("elegance_drawer_progress", self.id_salt)),
            is_open,
            ANIMATION_DURATION,
            emath::easing::cubic_in_out,
        );

        if just_opened {
            focus_state.prev_focus = ctx.memory(|m| m.focused());
        }
        if just_closed && let Some(prev) = focus_state.prev_focus.take() {
            ctx.memory_mut(|m| m.request_focus(prev));
            // Stage one of two: last frame's modal-layer claim can undo this
            // request, so re-assert next frame. See [`crate::overlay`].
            focus_state.pending_restore = Some(prev);
        } else if let Some(prev) = focus_state.pending_restore.take() {
            ctx.memory_mut(|m| m.request_focus(prev));
        }
        focus_state.was_open = is_open;
        focus_state.store(ctx, focus_storage);

        // Skip painting entirely when fully closed and not animating.
        if !is_open && progress < 0.001 {
            return None;
        }

        let theme = Theme::current(ctx);
        let p = &theme.palette;
        let mut should_close = false;
        let mut close_btn_id: Option<Id> = None;

        // --- Geometry ----------------------------------------------------
        let screen = ctx.content_rect();
        let panel_w = self.width;
        let slide = (1.0 - progress) * panel_w;
        let panel_rect = match self.side {
            DrawerSide::Right => Rect::from_min_max(
                Pos2::new(screen.max.x - panel_w + slide, screen.min.y),
                Pos2::new(screen.max.x + slide, screen.max.y),
            ),
            DrawerSide::Left => Rect::from_min_max(
                Pos2::new(screen.min.x - slide, screen.min.y),
                Pos2::new(screen.min.x + panel_w - slide, screen.max.y),
            ),
        };

        // Popups close themselves on `Esc`; remember whether one was open
        // before the body runs so a single press doesn't also take the drawer.
        let popup_was_open = egui::Popup::is_any_open(ctx);

        // --- Surface ------------------------------------------------------
        // Backdrop and panel share one area. See `crate::overlay` for why the
        // backdrop doesn't get a lower-order area of its own.
        let area_id = Id::unique("elegance_drawer").with(self.id_salt);
        let backdrop_alpha = (progress * 150.0).round() as u8;
        let title_text = self.title.as_ref().map(|t| t.text().to_string());
        let title = self.title;
        let subtitle = self.subtitle;
        let side = self.side;

        let outer = Area::new(area_id)
            .order(overlay::ORDER)
            .fixed_pos(screen.min)
            // This surface is positioned entirely by us: it is pinned to the
            // viewport origin and the panel inside it is laid out at absolute
            // coordinates that deliberately run off-screen mid-slide. Leaving
            // egui's constraining on would invite it to nudge the area back
            // when the panel's rect pokes past the edge.
            .constrain(false)
            .show(ctx, |ui| {
                // --- Backdrop ---
                ui.painter().rect_filled(
                    screen,
                    CornerRadius::ZERO,
                    Color32::from_rgba_premultiplied(0, 0, 0, backdrop_alpha),
                );
                let backdrop = ui.interact(screen, ui.scope_id().with("backdrop"), Sense::click());

                // --- Panel ---
                // Sensed as well, so clicks landing on the panel are consumed
                // here rather than falling through to the backdrop behind it —
                // widgets registered later win hit-testing within a layer.
                let panel = ui.scope_builder(
                    UiBuilder::new().sense(Sense::click()).max_rect(panel_rect),
                    |ui| {
                        ui.set_min_size(panel_rect.size());

                        // Clip body content to the panel rect — the area
                        // defaults to clipping at the screen edge, but we want
                        // content that would overflow the panel to be clipped
                        // to the panel itself, and content that is partly
                        // off-screen during the slide to skip tessellation
                        // outside the panel's bounds.
                        ui.set_clip_rect(panel_rect);

                        // Promote the Ui to a dialog node so screen readers
                        // announce it as a window-like surface and Tab
                        // navigates within it.
                        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                            node.set_role(accesskit::Role::Dialog);
                            if let Some(label) = title_text {
                                node.set_label(label);
                            }
                        });

                        // Paint shadow + background fill at the full panel
                        // rect. Frame::fill would paint only as tall as its
                        // content, which leaves an unfilled gap at the bottom
                        // whenever the body closure is shorter than the
                        // viewport — drawers are full-height, so we want the
                        // fill regardless of content height.
                        let shadow = Shadow {
                            offset: match side {
                                DrawerSide::Right => [-12, 0],
                                DrawerSide::Left => [12, 0],
                            },
                            blur: 28,
                            spread: 0,
                            color: Color32::from_black_alpha(110),
                        };
                        ui.painter()
                            .add(shadow.as_shape(panel_rect, CornerRadius::ZERO));
                        ui.painter()
                            .rect_filled(panel_rect, CornerRadius::ZERO, p.card);

                        let pad = theme.card_padding as i8;
                        let inner = Frame::new()
                            .inner_margin(Margin::same(pad))
                            .show(ui, |ui| {
                                if title.is_some() {
                                    paint_header(
                                        ui,
                                        &theme,
                                        title.as_ref(),
                                        subtitle.as_ref(),
                                        &mut should_close,
                                        &mut close_btn_id,
                                    );
                                    ui.separator();
                                    ui.add_space(8.0);
                                }
                                add_contents(ui)
                            })
                            .inner;

                        // Inner-edge divider — paint last so it sits on top of
                        // the Frame fill. The other three sides of the panel
                        // touch the viewport edges and don't need a border.
                        let inner_x = match side {
                            DrawerSide::Right => panel_rect.left(),
                            DrawerSide::Left => panel_rect.right(),
                        };
                        ui.painter().line_segment(
                            [
                                Pos2::new(inner_x, panel_rect.top()),
                                Pos2::new(inner_x, panel_rect.bottom()),
                            ],
                            Stroke::new(1.0, p.border),
                        );

                        inner
                    },
                );

                (panel.inner, backdrop)
            });

        let (result, backdrop) = outer.inner;
        if self.close_on_backdrop && backdrop.clicked() {
            should_close = true;
        }

        // Announce this drawer as open and find out whether it's the overlay on
        // top, which decides who gets `Esc` when overlays are stacked. Only
        // while genuinely open: a drawer mid slide-out is still painted, but it
        // has already handed control back and must not claim the key.
        let layer = overlay::layer_id(area_id);
        let is_topmost = is_open && overlay::register_open(ctx, layer);
        if self.close_on_escape && overlay::escape_dismisses(ctx, is_topmost, popup_was_open) {
            should_close = true;
        }

        // Hold the modal layer for the next frame, but only while genuinely
        // open: the claim is what keeps windows from raising themselves over
        // the drawer, and one that outlives it costs the background its focus.
        if is_open && !should_close {
            overlay::claim_modal_layer(ctx, layer);
        }

        // On the first frame the drawer opens, move keyboard focus into it
        // so Tab navigates within the dialog. Targets the close button (it's
        // always interactive when chrome is rendered); without a title there
        // is no intrinsic focus target and focus is left to the caller.
        if just_opened && let Some(id) = close_btn_id {
            ctx.memory_mut(|m| m.request_focus(id));
        }

        if should_close {
            *self.open = false;
        }

        Some(result)
    }
}

/// Animation time for the slide transition, in seconds. Chosen to match
/// the mockup's 260 ms cubic-bezier feel (eased via [`emath::easing::cubic_in_out`]).
const ANIMATION_DURATION: f32 = 0.26;

/// Paint the header row: title (strong) + optional muted subtitle on the
/// left, close "×" button on the right.
fn paint_header(
    ui: &mut Ui,
    theme: &Theme,
    title: Option<&WidgetText>,
    subtitle: Option<&WidgetText>,
    should_close: &mut bool,
    close_btn_id: &mut Option<Id>,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            if let Some(t) = title {
                ui.add(egui::Label::new(theme.heading_text(t.text())));
            }
            if let Some(s) = subtitle {
                ui.add(egui::Label::new(theme.muted_text(s.text())));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let resp = overlay::close_button(ui, "elegance_drawer_close");
            if resp.clicked() {
                *should_close = true;
            }
            *close_btn_id = Some(resp.id);
        });
    });
}
