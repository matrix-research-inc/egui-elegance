//! Shared internals for the overlay widgets: layering, modality, focus
//! lifecycle, and the chrome they both render.
//!
//! [`Modal`](crate::Modal) and [`Drawer`](crate::Drawer) both stack the same
//! two things over the app: a full-viewport backdrop that must swallow every
//! interaction behind it, and a content surface that must stay above that
//! backdrop. Getting this right in egui takes more care than it looks, so the
//! rules live here rather than being duplicated (and drifting) per widget.
//!
//! # The layer contract
//!
//! **One overlay is one [`Area`](egui::Area), at [`Order::Foreground`].** The
//! backdrop is painted and sensed *inside* that area rather than getting an
//! area of its own. A split across two orders (the obvious first design —
//! backdrop at [`Order::Middle`], content at [`Order::Foreground`]) leaves a
//! gap that any ordinary [`egui::Window`] slots straight into: the window
//! renders above the backdrop, so it keeps receiving clicks the overlay was
//! supposed to block, and clicking it raises it above the overlay's content.
//! Sharing one layer also makes the backdrop-below-content relationship
//! structural instead of something to re-assert every frame.
//!
//! **Pointer blocking is the backdrop's own doing.** It senses clicks across
//! the whole viewport, and egui routes a click to the topmost layer under the
//! cursor, so everything below the overlay's layer stops being reachable.
//!
//! That ordering needs no maintenance, because a window that cannot be clicked
//! never asks to be raised. Do *not* be tempted to re-assert it with a
//! per-frame `move_to_top`: egui applies that as a *stable sort* at the end of
//! the pass, lifting the layer above every layer that did not also ask. Popups,
//! menus and combo-box dropdowns are plain [`Order::Foreground`] areas, so an
//! overlay that re-raised itself each frame would sink any popup opened from
//! inside its own content.
//!
//! **Anything above [`Order::Foreground`] stays above.** Toasts and drag
//! ghosts live at [`Order::Tooltip`] and deliberately keep floating over an
//! open overlay, and remain interactive there.
//!
//! # The modal-layer claim, and the focus trap it sets
//!
//! Blocking *widgets* is not quite enough. egui also does area-level
//! hit-testing: [`egui::Area`] raises itself whenever the pointer is pressed
//! and `Memory::layer_id_at` names it. That query walks *area* rects, and an
//! overlay's area rect is only as big as its content: the full-viewport
//! backdrop is registered with [`egui::Ui::interact`], which senses a rect
//! without claiming layout space, or the card could not stay centred (an area
//! sized to the viewport has nothing left to centre within). So without help, a
//! press outside the card resolves to the window underneath and raises it above
//! the overlay: exactly the bug this design exists to fix.
//!
//! [`egui::Memory::set_modal_layer`] is what closes that gap. Its documented
//! job is confining keyboard focus, but `Memory::layer_id_at` also clamps its
//! answer to the modal layer, so a press over anything below the overlay is
//! attributed to the overlay and nothing beneath it ever asks to be raised.
//! The claim is therefore load-bearing for pointer layering, not a bonus.
//!
//! It comes with a trap. A claim takes effect the frame *after* it is made, so
//! on the frame an overlay closes, the previous frame's claim is still in
//! force. Widgets below the claimed layer are not "interested in focus", and
//! egui responds by actively surrendering their focus (see
//! `Context::create_widget`). An overlay that asks for the pre-dialog widget to
//! be focused again has it taken straight back, whenever that widget is drawn
//! *after* the overlay in the same frame. A trailing claim cannot be retracted,
//! and egui's only end-of-pass hook is a permanent plugin callback, so there is
//! nowhere to re-assert focus once per close. Overlays therefore restore it
//! twice: immediately, and again on the following frame via a pending flag.
//! The second attempt is the one that survives.

use egui::{Context, Id, Key, LayerId, Modifiers, Order, Response, Role, Ui, WidgetInfo};

use crate::{Button, ButtonSize};

/// The order every overlay lives in. See the module docs for why backdrop and
/// content share it instead of straddling [`Order::Middle`].
pub(crate) const ORDER: Order = Order::Foreground;

/// The [`LayerId`] an overlay with this area id occupies.
pub(crate) fn layer_id(area_id: Id) -> LayerId {
    LayerId::new(ORDER, area_id)
}

/// Which overlays were open, tracked one pass behind.
///
/// An overlay can only be compared against the others once they have all had a
/// chance to render, so the completed set from the previous pass is what
/// [`register_open`] consults. `current` accumulates this pass's registrations
/// and rotates into `previous` the first time a new pass touches it.
#[derive(Clone, Default)]
struct OpenOverlays {
    /// The pass `current` is being accumulated for.
    pass: u64,
    /// Overlays that have registered so far this pass.
    current: Vec<LayerId>,
    /// The complete set from the previous pass.
    previous: Vec<LayerId>,
}

/// Record that this overlay is open, and report whether it is the one on top.
///
/// Call once per pass from an open overlay, at the point it decides whether to
/// act on `Esc`.
///
/// "On top" means no other recently-open overlay sits above this one in egui's
/// layer order. Overlays that have since disappeared are ignored, so closing
/// the upper of a stack hands the answer straight to the one beneath.
///
/// One caveat, shared with egui's own [`egui::Modal`]: the comparison set is a
/// pass old, so on the single frame an overlay opens over an existing one, the
/// older overlay still believes it is on top, and an `Esc` landing in that
/// one-frame window goes to the wrong overlay. An overlay drawn before another
/// cannot know the other is about to appear above it, so short of deferring
/// every dismissal by a frame, immediate mode leaves no way to close this.
pub(crate) fn register_open(ctx: &Context, layer: LayerId) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.memory_mut(|m| {
        let order: Vec<LayerId> = m.layer_ids().collect();
        let reg: &mut OpenOverlays = m
            .data
            .get_temp_mut_or_default(Id::unique("elegance_open_overlays"));
        if reg.pass != pass {
            reg.previous = std::mem::take(&mut reg.current);
            reg.pass = pass;
        }
        if !reg.current.contains(&layer) {
            reg.current.push(layer);
        }
        let depth = |l: &LayerId| order.iter().position(|x| x == l);
        // Distinct layers occupy distinct positions, so the only entry that
        // can tie with `mine` is this overlay's own from the previous pass.
        // Hence `<=` rather than `<` plus a filter to exclude it.
        depth(&layer).is_some_and(|mine| reg.previous.iter().filter_map(depth).all(|o| o <= mine))
    })
}

/// Claim the modal layer, so presses over anything below this overlay are
/// attributed to it and nothing beneath raises itself above it.
///
/// Call only while the overlay intends to still be open *next* frame, which is
/// when egui applies the claim — see the module docs for why a claim that
/// outlives its overlay costs the background its focus.
pub(crate) fn claim_modal_layer(ctx: &Context, layer: LayerId) {
    ctx.memory_mut(|m| m.set_modal_layer(layer));
}

/// Persistent focus-lifecycle state for a single overlay, keyed by its
/// `id_salt`. Stored via `ctx.data_mut`.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct FocusState {
    /// Whether the overlay was rendered open last frame. Used to detect
    /// open/close transitions.
    pub was_open: bool,
    /// Which widget (if any) had keyboard focus at the moment the overlay
    /// opened. Restored on close.
    pub prev_focus: Option<Id>,
    /// A focus restore that still needs re-asserting on the next `show()`,
    /// because the one issued on the closing frame was undone by the trailing
    /// modal-layer claim. See the module docs.
    pub pending_restore: Option<Id>,
}

impl FocusState {
    pub(crate) fn load(ctx: &Context, storage: Id) -> Self {
        ctx.data(|d| d.get_temp(storage).unwrap_or_default())
    }

    pub(crate) fn store(self, ctx: &Context, storage: Id) {
        ctx.data_mut(|d| d.insert_temp(storage, self));
    }
}

/// Render an overlay's close button under the stable id `id_salt`, so focus
/// requests targeting it survive layout changes. Returns its `Response` so the
/// caller can route focus to it and observe `clicked()`. The accesskit label is
/// set to `"Close"` explicitly — without it screen readers announce the "×"
/// glyph as "multiplication sign".
pub(crate) fn close_button(ui: &mut Ui, id_salt: &'static str) -> Response {
    let inner = ui
        .push_id(id_salt, |ui| {
            ui.add(Button::new("\u{d7}").outline().size(ButtonSize::Small))
        })
        .inner;
    let enabled = inner.enabled();
    inner.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, "Close"));
    inner
}

/// Whether `Esc` should dismiss this overlay on this frame, consuming the key
/// press if so.
///
/// Three things have to line up. The overlay must be the topmost one, so that
/// a modal raised over a drawer takes the key and the drawer keeps its state.
/// No popup may have been open when the overlay's content ran, since egui's
/// popups close on `Esc` themselves and a single press should not collapse
/// both the dropdown and the dialog holding it. And the press is consumed, so
/// application-level `Esc` handlers don't act on a key the dialog just used.
pub(crate) fn escape_dismisses(ctx: &Context, is_topmost: bool, popup_was_open: bool) -> bool {
    is_topmost && !popup_was_open && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
}
