//! Modal dialog — a centered themed card over a dimmed backdrop.
//!
//! A full-viewport dimmed backdrop swallows clicks (and closes the modal when
//! clicked) beneath a centered [`Card`]-like window with an optional heading
//! row and a close "×" button. Press `Esc` to dismiss.
//!
//! Both are painted in a single [`Area`], and the modal blocks interaction
//! with everything below it for as long as it is open — see
//! [`crate::overlay`] for the layering rules that make that hold even against
//! an [`egui::Window`] of the same order.

use egui::{
    Align, Align2, Area, Color32, Context, CornerRadius, FontId, Frame, Id, Layout, Margin, Pos2,
    Rect, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, WidgetText, accesskit,
};

use crate::{Accent, overlay, theme::Theme};

/// Boxed `FnOnce(&mut Ui)` callback used by the footer slots.
type UiFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A centered modal dialog.
///
/// The `open` flag drives visibility: when it's `false` on entry to
/// [`Modal::show`], nothing is rendered; when the user clicks the backdrop,
/// presses `Esc`, or clicks the "×" button, it's flipped to `false`.
///
/// While open, the backdrop swallows clicks meant for anything beneath it —
/// including an [`egui::Window`], a [`Drawer`] and the panels behind them.
/// Stack several overlays and each `Esc` dismisses only the topmost, consuming
/// the key so application-level `Esc` handlers don't also fire. (A modal that
/// has opted out of `Esc` via [`Modal::close_on_escape`] or
/// [`Modal::closable`] leaves the key alone for the app to handle.)
/// [`Toasts`](crate::Toasts) deliberately stay above and remain clickable, so
/// a notification can still surface while a dialog is up.
///
/// Keyboard focus is moved into the dialog on open and returned to the
/// previously focused widget on close, but `Tab` is not fenced in: it can still
/// reach widgets behind the modal.
///
/// [`Drawer`]: crate::Drawer
///
/// ```no_run
/// # use elegance::Modal;
/// # let ctx = egui::Context::default();
/// # let mut open = true;
/// Modal::new("stats", &mut open)
///     .heading("Run Summary")
///     .show(&ctx, |ui| {
///         ui.label("…");
///     });
/// ```
#[must_use = "Call `.show(ctx, |ui| { ... })` to render the modal."]
pub struct Modal<'a> {
    id_salt: Id,
    heading: Option<WidgetText>,
    subtitle: Option<WidgetText>,
    header_icon: Option<WidgetText>,
    header_accent: Option<Accent>,
    open: &'a mut bool,
    max_width: f32,
    closable: bool,
    close_on_backdrop: bool,
    close_on_escape: bool,
    alert: bool,
    footer: Option<UiFn<'a>>,
    footer_left: Option<UiFn<'a>>,
}

impl<'a> std::fmt::Debug for Modal<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Modal")
            .field("id_salt", &self.id_salt)
            .field("heading", &self.heading.as_ref().map(|h| h.text()))
            .field("subtitle", &self.subtitle.as_ref().map(|h| h.text()))
            .field("header_icon", &self.header_icon.as_ref().map(|h| h.text()))
            .field("header_accent", &self.header_accent)
            .field("open", &*self.open)
            .field("max_width", &self.max_width)
            .field("closable", &self.closable)
            .field("close_on_backdrop", &self.close_on_backdrop)
            .field("close_on_escape", &self.close_on_escape)
            .field("alert", &self.alert)
            .field("footer", &self.footer.as_ref().map(|_| "<closure>"))
            .field(
                "footer_left",
                &self.footer_left.as_ref().map(|_| "<closure>"),
            )
            .finish()
    }
}

impl<'a> Modal<'a> {
    /// Create a modal keyed by `id_salt` whose visibility is bound to `open`.
    /// A modal is shown from a [`egui::Context`], so `id_salt` must be
    /// unique across the app.
    pub fn new(id_salt: impl crate::IdSalt, open: &'a mut bool) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            heading: None,
            subtitle: None,
            header_icon: None,
            header_accent: None,
            open,
            max_width: 440.0,
            closable: true,
            close_on_backdrop: true,
            close_on_escape: true,
            alert: false,
            footer: None,
            footer_left: None,
        }
    }

    /// Show a strong heading at the top of the modal, alongside the close button.
    pub fn heading(mut self, heading: impl Into<WidgetText>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    /// Show a muted subtitle line under the heading.
    pub fn subtitle(mut self, subtitle: impl Into<WidgetText>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Paint a glyph in a tinted circular halo to the left of the heading.
    /// Use any short text — `"⚠"`, `"✓"`, `"!"`, an emoji, or a symbol from
    /// the bundled `Elegance Symbols` font. The halo's tint comes from
    /// [`Modal::header_accent`] and defaults to [`Accent::Sky`].
    pub fn header_icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.header_icon = Some(icon.into());
        self
    }

    /// Override the accent used for the header icon halo. No-op without
    /// [`Modal::header_icon`].
    pub fn header_accent(mut self, accent: Accent) -> Self {
        self.header_accent = Some(accent);
        self
    }

    /// Override the maximum width of the modal card in points. Default: 440.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width;
        self
    }

    /// Whether the user may dismiss the modal at all. When `false`, the
    /// close "×" button is hidden and `Esc` / backdrop clicks are ignored —
    /// regardless of [`Modal::close_on_backdrop`] / [`Modal::close_on_escape`].
    ///
    /// Use this to force the user to see an in-progress action through (or
    /// cancel it via an explicit footer button) — for example, blocking
    /// dismissal while a long-running task runs, instead of juggling the
    /// `open` flag with an external "is it running?" guard.
    ///
    /// This only removes the *user-driven* dismissal affordances; the caller
    /// is still free to set the bound `open` flag to `false` programmatically
    /// to close the modal from code. Default: `true`.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Whether clicking the dimmed backdrop dismisses the modal. Default: `true`.
    pub fn close_on_backdrop(mut self, close: bool) -> Self {
        self.close_on_backdrop = close;
        self
    }

    /// Whether pressing `Esc` dismisses the modal. Default: `true`.
    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    /// Mark this modal as an *alert dialog* — a dialog that demands the
    /// user's attention to proceed, such as a destructive confirmation or
    /// an unsaved-changes prompt. Screen readers announce alert dialogs
    /// more assertively than ordinary dialogs. Default: `false`.
    ///
    /// Under the hood this exposes `accesskit::Role::AlertDialog` on the
    /// modal's root node instead of the default `Role::Dialog`.
    pub fn alert(mut self, alert: bool) -> Self {
        self.alert = alert;
        self
    }

    /// Add a footer row at the bottom of the modal. The closure runs in a
    /// right-to-left layout, so widgets added in source order land
    /// rightmost-first — matching the typical "Cancel | Confirm" reading.
    /// The footer renders below a horizontal divider and over a slightly
    /// recessed fill, separating it visually from the body.
    pub fn footer<F: FnOnce(&mut Ui) + 'a>(mut self, add_footer: F) -> Self {
        self.footer = Some(Box::new(add_footer));
        self
    }

    /// Add a left-aligned slot to the footer (only rendered when
    /// [`Modal::footer`] is also set). Useful for an "export before delete"
    /// checkbox or a keyboard-shortcut hint that should sit opposite the
    /// action buttons.
    pub fn footer_left<F: FnOnce(&mut Ui) + 'a>(mut self, add_left: F) -> Self {
        self.footer_left = Some(Box::new(add_left));
        self
    }

    /// Render the modal. Returns `None` if the modal was suppressed because
    /// the bound `open` flag was `false`; otherwise returns `Some(R)` with
    /// the content closure's return value.
    pub fn show<R>(self, ctx: &Context, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        // --- Focus lifecycle ------------------------------------------------
        // Track the open/closed transition so we can (a) record which widget
        // had keyboard focus before the modal opened and (b) restore that
        // focus when the modal closes. Without this the user's focus is
        // visually eclipsed by the modal but structurally remains behind it —
        // Tab would navigate widgets on the underlying page.
        let focus_storage = Id::unique(("elegance_modal_focus", self.id_salt));
        let mut focus_state = overlay::FocusState::load(ctx, focus_storage);
        let is_open = *self.open;

        // Second half of the two-stage restore described in `crate::overlay`.
        // This is the first frame with no modal-layer claim in force, so this
        // is the attempt that sticks.
        if let Some(prev) = focus_state.pending_restore.take() {
            ctx.memory_mut(|m| m.request_focus(prev));
            focus_state.store(ctx, focus_storage);
        }

        if focus_state.was_open && !is_open {
            // Just closed this frame — return focus to whatever had it before.
            if let Some(prev) = focus_state.prev_focus {
                ctx.memory_mut(|m| m.request_focus(prev));
            }
            overlay::FocusState::default().store(ctx, focus_storage);
            return None;
        }

        if !is_open {
            return None;
        }

        let just_opened = !focus_state.was_open;
        if just_opened {
            focus_state.prev_focus = ctx.memory(|m| m.focused());
            focus_state.was_open = true;
            focus_state.store(ctx, focus_storage);
        }

        let theme = Theme::current(ctx);
        let p = &theme.palette;
        let mut should_close = false;
        let mut close_btn_id: Option<Id> = None;
        let closable = self.closable;

        // Popups close themselves on `Esc`; remember whether one was open
        // before the body runs so a single press doesn't also take the modal.
        let popup_was_open = egui::Popup::is_any_open(ctx);

        // --- Surface ------------------------------------------------------
        // Backdrop and card share one area. See `crate::overlay` for why the
        // backdrop doesn't get a lower-order area of its own.
        let area_id = Id::unique("elegance_modal").with(self.id_salt);
        let alert = self.alert;
        let heading_text: Option<String> = self.heading.as_ref().map(|h| h.text().to_string());
        let max_width = self.max_width;
        let outer = Area::new(area_id)
            .order(overlay::ORDER)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                // --- Backdrop ---
                // Painted and sensed at absolute viewport coordinates. It is
                // sensed via `interact` rather than allocated: the enclosing
                // area is sized by the card so it stays centred, and a
                // full-viewport allocation here would stretch it.
                let screen = ui.ctx().content_rect();
                ui.painter().rect_filled(
                    screen,
                    CornerRadius::ZERO,
                    Color32::from_rgba_premultiplied(0, 0, 0, 150),
                );
                let backdrop = ui.interact(screen, ui.scope_id().with("backdrop"), Sense::click());

                // --- Card ---
                // Sensed as well, so clicks that land on the card are consumed
                // here instead of falling through to the backdrop behind it —
                // widgets registered later win hit-testing within a layer.
                let card = ui.scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                    // Upgrade this Ui's accesskit role from `GenericContainer`
                    // (set automatically by `Ui::new`) to a dialog role, so
                    // screen readers announce the modal correctly and
                    // platforms that support dialog focus tracking (AT-SPI)
                    // treat it as a window-like surface.
                    let role = if alert {
                        accesskit::Role::AlertDialog
                    } else {
                        accesskit::Role::Dialog
                    };
                    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                        node.set_role(role);
                        if let Some(label) = heading_text {
                            node.set_label(label);
                        }
                    });

                    ui.set_max_width(max_width);
                    Frame::new()
                        .fill(p.card)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(CornerRadius::same(theme.card_radius as u8))
                        .show(ui, |ui| {
                            let pad = theme.card_padding;
                            let has_heading = self.heading.is_some();
                            let has_icon = self.header_icon.is_some();
                            if has_heading || has_icon {
                                // Header band — same horizontal padding as body,
                                // tighter bottom so the optional separator + body
                                // continue to read as one block.
                                Frame::new()
                                    .inner_margin(Margin {
                                        left: pad as i8,
                                        right: pad as i8,
                                        top: pad as i8,
                                        bottom: 0,
                                    })
                                    .show(ui, |ui| {
                                        ui.horizontal_top(|ui| {
                                            if let Some(icon) = &self.header_icon {
                                                paint_icon_halo(
                                                    ui,
                                                    icon.text(),
                                                    self.header_accent.unwrap_or(Accent::Sky),
                                                    &theme,
                                                );
                                                ui.add_space(10.0);
                                            }
                                            ui.vertical(|ui| {
                                                if let Some(h) = &self.heading {
                                                    ui.add(egui::Label::new(
                                                        theme.heading_text(h.text()),
                                                    ));
                                                }
                                                if let Some(sub) = &self.subtitle {
                                                    ui.add(egui::Label::new(
                                                        theme.muted_text(sub.text()),
                                                    ));
                                                }
                                            });
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Min),
                                                |ui| {
                                                    // A non-closable modal shows no "×":
                                                    // there's no user-driven way out, so
                                                    // an affordance would only mislead.
                                                    if closable {
                                                        let resp = overlay::close_button(
                                                            ui,
                                                            "elegance_modal_close",
                                                        );
                                                        if resp.clicked() {
                                                            should_close = true;
                                                        }
                                                        close_btn_id = Some(resp.id);
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(10.0);
                            }
                            // --- Body ---
                            let body_result = Frame::new()
                                .inner_margin(Margin {
                                    left: pad as i8,
                                    right: pad as i8,
                                    top: if has_heading || has_icon {
                                        0
                                    } else {
                                        pad as i8
                                    },
                                    bottom: if self.footer.is_some() {
                                        pad as i8 / 2
                                    } else {
                                        pad as i8
                                    },
                                })
                                .show(ui, |ui| add_contents(ui))
                                .inner;

                            // --- Footer ---
                            if let Some(footer) = self.footer {
                                ui.separator();
                                // The recessed footer fill is painted by hand rather
                                // than via the frame's own `.fill`. A plain frame
                                // fill is a square-cornered rectangle flush with the
                                // card edges, so it paints over the card's rounded
                                // bottom corners and bottom border — the non-round
                                // corners reported in issue #7. Instead we lay the
                                // footer out with no fill, then drop a rounded fill
                                // into a slot reserved *behind* the content, tucked
                                // one pixel inside the 1px border so the border (and
                                // its rounded corners) stays unbroken all the way
                                // around.
                                let footer_fill = theme.palette.depth_tint(p.card, 0.04);
                                let fill_idx = ui.painter().add(Shape::Noop);
                                let footer_rect = Frame::new()
                                    .inner_margin(Margin::symmetric(pad as i8, pad as i8 * 3 / 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if let Some(left) = self.footer_left {
                                                left(ui);
                                            }
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    footer(ui);
                                                },
                                            );
                                        });
                                    })
                                    .response
                                    .rect;
                                // Round the bottom corners one pixel tighter than the
                                // card so the fill follows the inside of the border's
                                // curve; leave the top flush with the divider above.
                                let r = (theme.card_radius - 1.0).max(0.0) as u8;
                                let fill_rect = Rect::from_min_max(
                                    Pos2::new(footer_rect.left() + 1.0, footer_rect.top()),
                                    Pos2::new(
                                        footer_rect.right() - 1.0,
                                        footer_rect.bottom() - 1.0,
                                    ),
                                );
                                ui.painter().set(
                                    fill_idx,
                                    Shape::rect_filled(
                                        fill_rect,
                                        CornerRadius {
                                            nw: 0,
                                            ne: 0,
                                            sw: r,
                                            se: r,
                                        },
                                        footer_fill,
                                    ),
                                );
                            }
                            body_result
                        })
                        .inner
                });

                (card.inner, backdrop)
            });

        let (result, backdrop) = outer.inner;
        if closable && self.close_on_backdrop && backdrop.clicked() {
            should_close = true;
        }

        // Announce this modal as open and find out whether it's the one on
        // top, which decides who gets `Esc` when overlays are stacked.
        let layer = overlay::layer_id(area_id);
        let is_topmost = overlay::register_open(ctx, layer);
        if closable
            && self.close_on_escape
            && overlay::escape_dismisses(ctx, is_topmost, popup_was_open)
        {
            should_close = true;
        }

        // Hold the modal layer for the next frame, but not on the frame we
        // close: the claim is what keeps windows from raising themselves over
        // the modal, and letting it outlive the modal would cost the
        // background its focus.
        if !should_close {
            overlay::claim_modal_layer(ctx, layer);
        }

        // On the first frame a modal is open, move keyboard focus into it so
        // Tab navigates within the dialog rather than the background. We
        // target the close button when a heading is present (it has a
        // stable id and is always interactive); without a heading there's
        // no intrinsic focus target, so focus is left to the caller.
        if just_opened && let Some(id) = close_btn_id {
            ctx.memory_mut(|m| m.request_focus(id));
        }

        if should_close {
            *self.open = false;
            // Restore focus and clear the lifecycle state right now — in the
            // same frame the close is triggered — rather than deferring to the
            // `was_open && !is_open` branch on a subsequent `show()`. Callers
            // routinely drop the modal the instant it closes (the natural
            // `if let Some(m) = &self.modal { … }` + `self.modal = None`
            // pattern), so `show()` is never called again. A deferred cleanup
            // would then silently leak: focus is never returned to the
            // pre-modal widget, and the stale `was_open = true` defeats the
            // just-opened focus grab the next time a modal with this salt opens.
            if let Some(prev) = focus_state.prev_focus {
                ctx.memory_mut(|m| m.request_focus(prev));
            }
            // Stage one of two: last frame's modal-layer claim can undo this
            // request, so leave a note for the next `show()` to re-assert it.
            // See [`crate::overlay`].
            overlay::FocusState {
                pending_restore: focus_state.prev_focus,
                ..Default::default()
            }
            .store(ctx, focus_storage);
        }

        Some(result)
    }
}

/// Paint a circular tinted halo with a centered glyph. The fg uses the full
/// accent colour; the bg is the same colour at low alpha so the halo reads
/// as a coloured "wash" against the card surface.
fn paint_icon_halo(ui: &mut Ui, glyph: &str, accent: Accent, theme: &Theme) {
    let size = 32.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let fg = theme.palette.accent_fill(accent);
    let bg = Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), 36);
    let painter = ui.painter();
    painter.circle_filled(rect.center(), size * 0.5, bg);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(theme.typography.heading + 2.0),
        fg,
    );
}
