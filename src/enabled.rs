//! Disabled widgets whose responses egui also sees as disabled.
//!
//! An elegance widget with `.enabled(false)` paints its own disabled palette
//! and stops sensing clicks. That alone leaves egui's view of it enabled:
//! [`Response::enabled`] reports `true`, [`Response::on_hover_text`] still
//! opens, and [`Response::on_disabled_hover_text`] never does. egui marks a
//! response disabled only when it is created in a disabled [`Ui`], so these
//! helpers create the widget's response through [`Ui::add_enabled`].
//!
//! `add_enabled` disables the caller's own `Ui` for one call and then restores
//! it, so the allocation lands, and takes its id, exactly where it would have
//! anyway. Only the allocation goes through it: the widget paints afterwards,
//! in the restored `Ui`, so its own disabled palette is not also faded by
//! egui's disabled opacity. [`Ui::add_enabled_ui`] would not do: it builds a
//! child scope, which cannot wrap in a wrapping layout and moves the ids.

use egui::{Id, Rect, Response, Sense, Ui, Vec2};

/// [`Ui::allocate_exact_size`], with a response that is disabled unless
/// `enabled`.
pub(crate) fn allocate_exact_size(
    ui: &mut Ui,
    desired_size: Vec2,
    sense: Sense,
    enabled: bool,
) -> (Rect, Response) {
    let mut rect = Rect::NOTHING;
    let response = ui.add_enabled(enabled, |ui: &mut Ui| {
        let (allocated, response) = ui.allocate_exact_size(desired_size, sense);
        rect = allocated;
        response
    });
    (rect, response)
}

/// [`Ui::allocate_at_least`], with a response that is disabled unless
/// `enabled`.
pub(crate) fn allocate_at_least(
    ui: &mut Ui,
    desired_size: Vec2,
    sense: Sense,
    enabled: bool,
) -> (Rect, Response) {
    let response = ui.add_enabled(enabled, |ui: &mut Ui| {
        ui.allocate_response(desired_size, sense)
    });
    (response.rect, response)
}

/// [`Ui::interact`], with a response that is disabled unless `enabled`.
pub(crate) fn interact(ui: &mut Ui, rect: Rect, id: Id, sense: Sense, enabled: bool) -> Response {
    ui.add_enabled(enabled, |ui: &mut Ui| ui.interact(rect, id, sense))
}
