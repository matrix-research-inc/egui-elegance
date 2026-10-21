//! Right-click popup menu — elegance-styled context menu.
//!
//! [`ContextMenu`] opens a themed popup at the cursor position when its
//! anchor [`Response`] is secondary-clicked. The popup hosts the same
//! [`MenuItem`](crate::MenuItem), [`MenuSection`](crate::MenuSection),
//! and [`SubMenuItem`](crate::SubMenuItem) widgets as the rest of the
//! menu family, so the visual treatment matches both [`Menu`](crate::Menu)
//! popups and [`MenuBar`](crate::MenuBar) dropdowns.
//!
//! The target [`Response`] must have a click sense for egui to register
//! the secondary click — most interactive widgets (buttons, list rows
//! sensed via `Sense::click()`) already do; for plain labels or custom
//! regions, allocate the rect with `Sense::click()` first.
//!
//! ```no_run
//! # use elegance::{ContextMenu, MenuItem, MenuSection, SubMenuItem};
//! # egui::__run_test_ui(|ui| {
//! let row = ui.add(egui::Label::new("theme.rs").sense(egui::Sense::click()));
//! ContextMenu::new("file_row").show(&row, |ui| {
//!     ui.add(MenuItem::new("Open").shortcut("\u{21B5}"));
//!     ui.add(MenuItem::new("Open in new split").shortcut("\u{2318}\u{21E7}\u{21B5}"));
//!     SubMenuItem::new("Open with").show(ui, |ui| {
//!         ui.add(MenuItem::new("Source editor"));
//!         ui.add(MenuItem::new("Preview"));
//!     });
//!     ui.separator();
//!     ui.add(MenuSection::new("Edit"));
//!     ui.add(MenuItem::new("Copy").shortcut("\u{2318}C"));
//!     ui.add(MenuItem::new("Rename\u{2026}").shortcut("F2"));
//!     ui.separator();
//!     ui.add(MenuItem::new("Delete").danger().shortcut("\u{232B}"));
//! });
//! # });
//! ```
//!
//! The popup is dismissed by clicking any item, clicking outside, or
//! pressing `Esc`.

use egui::{CornerRadius, Frame, Margin, Popup, PopupCloseBehavior, Response, Stroke, Ui};

use crate::theme::Theme;

/// A right-click-anchored popup menu attached to a [`Response`].
///
/// See the module-level docs for usage. The menu opens at the cursor
/// position when `target` is secondary-clicked.
#[derive(Debug, Clone)]
#[must_use = "Call `.show(&target, |ui| ...)` to render the context menu."]
pub struct ContextMenu {
    id_salt: egui::IdSalt,
    min_width: f32,
}

impl ContextMenu {
    /// Create a context menu keyed by `id_salt`. The salt is combined with
    /// the target's id to key the popup's open/closed state in egui memory,
    /// so it only needs to be unique among the context menus on one target.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance::context_menu", id_salt)),
            min_width: 200.0,
        }
    }

    /// Minimum width of the popup in points. Default: 200.
    #[inline]
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Render the context menu attached to `target`. The popup opens on
    /// secondary-click on `target` at the pointer position; clicking
    /// inside an item, clicking outside, or pressing `Esc` closes it.
    ///
    /// Returns `Some(R)` with the body closure's return value while the
    /// menu is open, `None` while closed.
    pub fn show<R>(self, target: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let theme = Theme::current(&target.ctx);
        let p = &theme.palette;
        let r = theme.card_radius as u8;
        let frame = Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(r))
            .inner_margin(Margin::same(4));

        Popup::context_menu(target)
            .id(target.id.with_salt(self.id_salt))
            .frame(frame)
            .close_behavior(PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.set_min_width(self.min_width);
                ui.spacing_mut().item_spacing.y = 2.0;
                add_contents(ui)
            })
            .map(|r| r.inner)
    }
}
