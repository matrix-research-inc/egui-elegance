//! Action menus — a popup list of items attached to a trigger [`Response`].
//!
//! [`Menu`] opens a themed popup below a trigger widget when the trigger is
//! clicked. [`MenuItem`] is the styled leaf inside the popup: label on the
//! left, optional keyboard-shortcut hint on the right, optional `danger`
//! tint for destructive actions. Separators between groups use the stock
//! `ui.separator()`.
//!
//! ```no_run
//! # use elegance::{Button, ButtonSize, Menu, MenuItem};
//! # egui::__run_test_ui(|ui| {
//! let trigger = ui.add(Button::new("⋯").outline().size(ButtonSize::Small));
//! Menu::new("row_actions").show_below(&trigger, |ui| {
//!     if ui.add(MenuItem::new("Edit").shortcut("⌘ E")).clicked() {
//!         // …
//!     }
//!     if ui.add(MenuItem::new("Duplicate")).clicked() { /* … */ }
//!     ui.separator();
//!     if ui.add(MenuItem::new("Delete").danger()).clicked() { /* … */ }
//! });
//! # });
//! ```
//!
//! The popup is dismissed by clicking any item, clicking outside, or
//! pressing `Esc`. Keyboard navigation (arrows + Enter) is not implemented
//! in this version.

use egui::{
    CornerRadius, Popup, PopupCloseBehavior, Pos2, Response, Role, Sense, Ui, Vec2, Widget,
    WidgetInfo, WidgetText,
};

use crate::theme::{Theme, with_alpha};

/// A click-to-open popup menu anchored below a trigger [`Response`].
///
/// Call [`Menu::show_below`] after painting the trigger; it opens on
/// trigger clicks and closes on item click, outside-click, or `Esc`.
#[derive(Debug, Clone)]
#[must_use = "Call `.show_below(&trigger, |ui| ...)` to render the menu."]
pub struct Menu {
    id_salt: egui::IdSalt,
    min_width: f32,
}

impl Menu {
    /// Create a menu keyed by `id_salt`. The salt is combined with the
    /// trigger's id to persist the open/closed state across frames, so it
    /// only needs to be unique among the menus on one trigger.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance::menu", id_salt)),
            min_width: 180.0,
        }
    }

    /// Minimum width of the popup in points. Default: 180.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Render the menu below `trigger`. Returns `Some(R)` with the body
    /// closure's return value while the menu is open, `None` while closed.
    pub fn show_below<R>(
        self,
        trigger: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let popup_id = trigger.id.with_salt(self.id_salt);
        Popup::menu(trigger)
            .id(popup_id)
            .close_behavior(PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.set_min_width(self.min_width);
                // Tight stacking — MenuItem has its own interior padding.
                ui.spacing_mut().item_spacing.y = 2.0;
                add_contents(ui)
            })
            .map(|r| r.inner)
    }
}

/// A single selectable row inside a [`Menu`].
///
/// Add with `ui.add(MenuItem::new("…"))` inside a menu body. The returned
/// [`Response`]'s `.clicked()` fires on activation.
///
/// The optional [`icon`](Self::icon), [`checked`](Self::checked), and
/// [`radio`](Self::radio) builders all reserve the same leading gutter,
/// so toggle items in a menu align cleanly with action items as long as
/// every item in that menu opts into one of them.
#[must_use = "Add with `ui.add(...)`."]
pub struct MenuItem {
    label: WidgetText,
    shortcut: Option<String>,
    danger: bool,
    enabled: bool,
    leading: Option<Leading>,
    submenu_arrow: bool,
}

#[derive(Clone)]
enum Leading {
    Icon(WidgetText),
    Checked(bool),
    Radio(bool),
}

impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label.text())
            .field("shortcut", &self.shortcut)
            .field("danger", &self.danger)
            .field("enabled", &self.enabled)
            .field("leading", &self.leading.is_some())
            .finish()
    }
}

impl MenuItem {
    /// Create a menu item with the given label.
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            danger: false,
            enabled: true,
            leading: None,
            submenu_arrow: false,
        }
    }

    #[doc(hidden)]
    /// Render a body-sized right chevron in the right gutter to mark a
    /// submenu trigger. Hidden because callers should reach for
    /// [`SubMenuItem`] rather than building this on the raw `MenuItem`.
    pub fn with_submenu_arrow(mut self) -> Self {
        self.submenu_arrow = true;
        self
    }

    /// Display a keyboard-shortcut hint on the right (informational only —
    /// the actual shortcut is not bound).
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Render the item in the danger tone — red label, red hover highlight.
    /// Use for destructive actions.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// Disable the item. Disabled items do not fire `clicked()` and render
    /// with muted text.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show a leading icon (any text, typically a unicode glyph) in the
    /// gutter to the left of the label. Reserves the gutter even when the
    /// glyph is narrow, so adjacent items align.
    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.leading = Some(Leading::Icon(icon.into()));
        self
    }

    /// Render the item as a checkbox toggle: a checkmark in the leading
    /// gutter when `on`, an empty gutter when off. The item is announced
    /// via accesskit as a checkbox with the given selected state.
    pub fn checked(mut self, on: bool) -> Self {
        self.leading = Some(Leading::Checked(on));
        self
    }

    /// Render the item as a radio-button toggle: a filled dot in the
    /// leading gutter when `on`, an empty gutter when off. Use within a
    /// group of mutually-exclusive choices. Announced via accesskit as a
    /// radio button.
    pub fn radio(mut self, on: bool) -> Self {
        self.leading = Some(Leading::Radio(on));
        self
    }
}

impl Widget for MenuItem {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let pad_x = 10.0;
        let pad_y = 6.0;
        let gap_x = 16.0;
        let gutter_w = 16.0; // Reserved leading-glyph slot.
        let gutter_gap = 8.0;

        let label_color = if !self.enabled {
            p.text_faint
        } else if self.danger {
            p.danger
        } else {
            p.text
        };

        let label_galley =
            crate::theme::placeholder_galley(ui, self.label.text(), t.body, false, f32::INFINITY);

        let shortcut_galley = self
            .shortcut
            .as_deref()
            .map(|s| crate::theme::placeholder_galley(ui, s, t.small, false, f32::INFINITY));

        // Submenu arrow: a `›` glyph rendered well above body size so it
        // reads as a flyout indicator rather than a small auxiliary
        // mark.
        let submenu_arrow_galley = if self.submenu_arrow {
            Some(crate::theme::placeholder_galley(
                ui,
                "\u{203A}",
                24.0,
                false,
                f32::INFINITY,
            ))
        } else {
            None
        };

        let leading_glyph_galley = match &self.leading {
            Some(Leading::Icon(icon)) => Some(crate::theme::placeholder_galley(
                ui,
                icon.text(),
                t.body,
                false,
                f32::INFINITY,
            )),
            Some(Leading::Checked(true)) => Some(crate::theme::placeholder_galley(
                ui,
                "\u{2713}",
                t.body,
                true,
                f32::INFINITY,
            )),
            Some(Leading::Radio(true)) => Some(crate::theme::placeholder_galley(
                ui,
                "\u{2022}",
                t.body,
                true,
                f32::INFINITY,
            )),
            // Off-state toggles still reserve the gutter so siblings align.
            Some(Leading::Checked(false)) | Some(Leading::Radio(false)) => None,
            None => None,
        };

        let leading_offset = if self.leading.is_some() {
            gutter_w + gutter_gap
        } else {
            0.0
        };

        let trailing_w = shortcut_galley
            .as_ref()
            .map_or(0.0, |g| g.size().x + gap_x)
            .max(
                submenu_arrow_galley
                    .as_ref()
                    .map_or(0.0, |g| g.size().x + gap_x),
            );
        let content_w = leading_offset + label_galley.size().x + trailing_w;
        // Width is the natural content size: the parent menu's
        // `top_down_justified` layout stretches each row to match the
        // widest one, and the popup as a whole sizes to that maximum.
        // Don't `.max(available_width)` here — that would let each item
        // greedily expand to whatever space the parent offers, which
        // makes the popup balloon out to its container's width.
        let desired = Vec2::new(
            content_w + pad_x * 2.0,
            label_galley.size().y.max(t.body) + pad_y * 2.0,
        );

        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        // `allocate_at_least` returns the full layout slot rect (which,
        // under `top_down_justified`, expands to the widest sibling).
        // We need that here so the trailing shortcut right-aligns with
        // the popup's right edge — `allocate_exact_size` would clamp the
        // returned rect to `desired` and the shortcut would sit right
        // after the label.
        let (rect, response) = ui.allocate_at_least(desired, sense);

        if ui.is_rect_visible(rect) {
            let is_hovered = response.hovered() && self.enabled;
            if is_hovered {
                let bg = if self.danger {
                    with_alpha(p.red, 40)
                } else {
                    with_alpha(p.focus, 28)
                };
                let radius = CornerRadius::same((theme.control_radius as u8).saturating_sub(2));
                ui.painter().rect_filled(rect, radius, bg);
            }

            if let Some(glyph) = leading_glyph_galley {
                // Centre the glyph within the gutter slot.
                let slot_x = rect.min.x + pad_x;
                let glyph_color = match &self.leading {
                    Some(Leading::Checked(true)) | Some(Leading::Radio(true)) => p.focus,
                    _ if !self.enabled => p.text_faint,
                    _ => p.text_muted,
                };
                let pos = Pos2::new(
                    slot_x + (gutter_w - glyph.size().x) * 0.5,
                    rect.center().y - glyph.size().y * 0.5,
                );
                ui.painter().galley(pos, glyph, glyph_color);
            }

            let label_pos = Pos2::new(
                rect.min.x + pad_x + leading_offset,
                rect.center().y - label_galley.size().y * 0.5,
            );
            ui.painter().galley(label_pos, label_galley, label_color);

            if let Some(galley) = shortcut_galley {
                let pos = Pos2::new(
                    rect.max.x - pad_x - galley.size().x,
                    rect.center().y - galley.size().y * 0.5,
                );
                let color = if !self.enabled {
                    p.text_faint
                } else if self.danger {
                    with_alpha(p.danger, 200)
                } else {
                    p.text_muted
                };
                ui.painter().galley(pos, galley, color);
            }

            if let Some(galley) = submenu_arrow_galley {
                let pos = Pos2::new(
                    rect.max.x - pad_x - galley.size().x,
                    rect.center().y - galley.size().y * 0.5,
                );
                let color = if !self.enabled {
                    p.text_faint
                } else {
                    p.text_muted
                };
                ui.painter().galley(pos, galley, color);
            }
        }

        response.widget_info(|| match &self.leading {
            Some(Leading::Checked(on)) => {
                WidgetInfo::selected(Role::CheckBox, self.enabled, *on, self.label.text())
            }
            Some(Leading::Radio(on)) => {
                WidgetInfo::selected(Role::RadioButton, self.enabled, *on, self.label.text())
            }
            _ => WidgetInfo::labeled(Role::Button, self.enabled, self.label.text()),
        });
        response
    }
}

/// A menu row that opens a flyout submenu when hovered.
///
/// Visually a [`MenuItem`] with a right-pointing chevron; pair the
/// trigger with a body closure that fills the child menu. Use inside any
/// elegance menu — a [`MenuBar`](crate::MenuBar) dropdown body or a
/// [`Menu`] popup. The submenu opens to the right and stays open while
/// the pointer remains over either the trigger or the flyout panel.
///
/// ```no_run
/// # use elegance::{MenuBar, MenuItem, SubMenuItem};
/// # egui::__run_test_ui(|ui| {
/// MenuBar::new("app").show(ui, |bar| {
///     bar.menu("File", |ui| {
///         ui.add(MenuItem::new("New"));
///         SubMenuItem::new("Open Recent").show(ui, |ui| {
///             ui.add(MenuItem::new("theme.rs"));
///             ui.add(MenuItem::new("README.md"));
///         });
///         ui.add(MenuItem::new("Save"));
///     });
/// });
/// # });
/// ```
///
/// Hover-to-flyout, click-to-pin, and proper "stay open while child is
/// open" behavior come from `egui`'s built-in submenu machinery; this
/// type just wires our [`MenuItem`] visual into that pipeline.
#[must_use = "Call `.show(ui, |ui| ...)` to render the submenu trigger and flyout."]
pub struct SubMenuItem {
    label: WidgetText,
    icon: Option<WidgetText>,
    enabled: bool,
}

impl std::fmt::Debug for SubMenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubMenuItem")
            .field("label", &self.label.text())
            .field("icon", &self.icon.is_some())
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl SubMenuItem {
    /// Create a new submenu trigger with the given label.
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            enabled: true,
        }
    }

    /// Show a leading icon (any text, typically a unicode glyph) in the
    /// gutter to the left of the label.
    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Disable the submenu trigger. Disabled triggers do not open the
    /// flyout and render with muted text.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Render the trigger row and attach the flyout submenu. The body
    /// closure populates the child menu and is invoked while the flyout
    /// is open. Returns `Some(R)` with the body's return value while the
    /// submenu is open, `None` while closed.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let mut item = MenuItem::new(self.label)
            .enabled(self.enabled)
            .with_submenu_arrow();
        if let Some(icon) = self.icon {
            item = item.icon(icon);
        }
        let response = ui.add(item);
        sub_menu_show(ui, &response, body)
    }
}

/// Open a flyout submenu next to `button_response`, with a wider gap than
/// `egui::containers::menu::SubMenu` produces by default.
///
/// This vendors egui 0.34's `SubMenu::show` logic verbatim so we keep its
/// hover/click/close-behavior semantics intact, but bumps the popup's
/// `gap` so the flyout reads as a visually distinct panel rather than a
/// continuation of the parent menu. egui hard-codes the gap as
/// `frame.margin / 2 + 2` (~8 pt with our menu_margin), which is enough
/// for the popup to clear the parent's border but still close enough
/// that, with two same-coloured `Frame::menu` panels side by side, they
/// read as one wider menu.
fn sub_menu_show<R>(
    ui: &Ui,
    button_response: &Response,
    content: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    use egui::containers::menu::{MenuConfig, MenuState};
    use egui::{
        Frame, Layout, Margin, PointerButton, Popup, PopupCloseBehavior, Rect, UiKind, UiStackInfo,
        emath::{Align, RectAlign},
        pos2,
    };

    // Horizontal gap between the trigger row's right edge and the
    // submenu's left edge. Since the submenu also drops downward off
    // the trigger row's bottom (see the anchor manipulation below),
    // there's already a clear vertical break between the two panels;
    // we don't need extra horizontal offset to read them as distinct.
    const GAP: f32 = 0.0;

    // Tighten the submenu's vertical padding to match our top-level
    // dropdown's `inner_margin: 4` so the first submenu item sits close
    // to the panel's top edge instead of dropping ~6 pt below it.
    let frame = Frame::menu(ui.style()).inner_margin(Margin::same(4));
    let id = button_response.id.with("submenu");

    let (open_item, menu_id) =
        MenuState::from_ui(ui, |state, stack| (state.open_item, stack.unique_id));
    // `MenuConfig::find` walks the stack from the parent menu (we haven't
    // shown the submenu yet, so the current ui's stack tail is the
    // parent). The submenu inherits the parent's close behavior and
    // style.
    let menu_config = MenuConfig::find(ui);

    let menu_root_response = ui
        .ctx()
        .read_response(menu_id)
        .expect("submenu must be inside a menu");
    let hover_pos = ui.ctx().pointer_hover_pos();
    let menu_rect = menu_root_response.rect - frame.total_margin();
    let is_hovering_menu = hover_pos.is_some_and(|pos| {
        ui.ctx().layer_id_at(pos) == Some(menu_root_response.layer_id) && menu_rect.contains(pos)
    });

    let is_any_open = open_item.is_some();
    let mut is_open = open_item == Some(id);
    let was_open = is_open;
    let mut set_open: Option<bool> = None;

    let button_rect = button_response
        .rect
        .expand2(ui.style().spacing.item_spacing / 2.0);
    let is_hovered = hover_pos.is_some_and(|pos| button_rect.contains(pos));

    let clicked = button_response.clicked();
    let clicked_by_pointer = button_response.clicked_by(PointerButton::Primary);
    let clicked_by_keyboard_or_access = clicked && !clicked_by_pointer;

    if ui.is_enabled() && is_open && clicked_by_keyboard_or_access {
        set_open = Some(false);
        is_open = false;
    }

    let should_open = ui.is_enabled() && ((!was_open && clicked) || (is_hovered && !is_any_open));
    if should_open {
        set_open = Some(true);
        is_open = true;
        MenuState::from_id(ui.ctx(), menu_id, |state| {
            state.open_item = None;
        });
    }

    // Anchor the popup to a zero-height strip along the trigger row's
    // bottom, indented from the row's left edge by `LEFT_INSET` so a
    // `BOTTOM_START` align drops the submenu down with its left edge
    // *inside* the trigger row (rather than column-flush with the
    // parent menu). 24pt aligns roughly with where the trigger's label
    // text starts past the leading icon gutter, so the submenu reads
    // as continuing from the trigger's content rather than from the
    // panel's edge. We only touch `interact_rect` (what
    // `Popup::from_response` uses for anchoring); hover/click
    // detection further down still consults the original `rect`.
    const LEFT_INSET: f32 = 24.0;
    let mut response = button_response.clone();
    let bottom = button_response.rect.bottom();
    let left = (button_response.rect.left() + LEFT_INSET).min(button_response.rect.right());
    response.interact_rect = Rect::from_min_max(
        pos2(left, bottom),
        pos2(button_response.rect.right(), bottom),
    );

    let popup_response = Popup::from_response(&response)
        .id(id)
        .open(is_open)
        // Drop straight down from the trigger row: popup top-left aligns
        // with the anchor strip's left-bottom, so the submenu shares a
        // column with the trigger and grows to the right only as wide as
        // its contents need.
        .align(RectAlign::BOTTOM_START)
        // Pin the alignment — without this, egui's `find_best_align`
        // tries `RectAlign::MENU_ALIGNS` as fallbacks if it thinks our
        // requested side doesn't fit, and silently picks a different
        // alignment, which is what makes our `gap` look like it has no
        // effect when the parent menu sits on a side of the viewport.
        .align_alternatives(&[])
        .layout(Layout::top_down_justified(Align::Min))
        .gap(GAP)
        .style(menu_config.style.clone())
        .frame(frame)
        .close_behavior(PopupCloseBehavior::IgnoreClicks)
        .info(
            UiStackInfo::new(UiKind::Menu)
                .with_tag_value(MenuConfig::MENU_CONFIG_TAG, menu_config.clone()),
        )
        .show(|ui| {
            if button_response.clicked() || button_response.is_pointer_button_down_on() {
                ui.ctx().move_to_top(ui.layer_id());
            }
            content(ui)
        });

    if let Some(popup_response) = &popup_response {
        let is_deepest_submenu = MenuState::is_deepest_open_sub_menu(ui.ctx(), id);
        let clicked_outside = is_deepest_submenu
            && popup_response.response.clicked_elsewhere()
            && menu_root_response.clicked_elsewhere();
        let submenu_button_clicked = button_response.clicked();
        let clicked_inside = is_deepest_submenu
            && !submenu_button_clicked
            && response.ctx.input(|i| i.pointer.any_click())
            && hover_pos.is_some_and(|pos| popup_response.response.interact_rect.contains(pos));

        let click_close = match menu_config.close_behavior {
            PopupCloseBehavior::CloseOnClick => clicked_outside || clicked_inside,
            PopupCloseBehavior::CloseOnClickOutside => clicked_outside,
            PopupCloseBehavior::IgnoreClicks => false,
        };

        if click_close {
            set_open = Some(false);
        }

        let is_moving_towards_rect = ui.input(|i| {
            i.pointer
                .is_moving_towards_rect(&popup_response.response.rect)
        });
        if is_moving_towards_rect {
            ui.ctx().request_repaint();
        }
        let hovering_other_menu_entry = is_open
            && !is_hovered
            && !popup_response.response.contains_pointer()
            && !is_moving_towards_rect
            && is_hovering_menu;

        if hovering_other_menu_entry {
            set_open = Some(false);
        }
    }

    if let Some(open) = set_open {
        MenuState::from_id(ui.ctx(), menu_id, |state| {
            state.open_item = open.then_some(id);
        });
    }

    if is_open {
        MenuState::mark_shown(ui.ctx(), id);
    }

    popup_response.map(|r| r.inner)
}

/// A small uppercase header used to label a group of items inside a
/// [`Menu`] popup, [`ContextMenu`](crate::ContextMenu), or
/// [`MenuBar`](crate::MenuBar) dropdown.
///
/// Add with `ui.add(MenuSection::new("Edit"))` between groups of related
/// items. The header is non-interactive and renders in the muted text
/// tone with extra top padding so it visually separates from the item
/// above without needing a separator.
///
/// ```no_run
/// # use elegance::{Menu, MenuItem, MenuSection};
/// # egui::__run_test_ui(|ui| {
/// # let trigger = ui.button("\u{22EF}");
/// Menu::new("row_actions").show_below(&trigger, |ui| {
///     ui.add(MenuItem::new("Copy").shortcut("\u{2318}C"));
///     ui.separator();
///     ui.add(MenuSection::new("Selection"));
///     ui.add(MenuItem::new("Highlight matches").checked(true));
///     ui.add(MenuItem::new("Show whitespace").checked(false));
/// });
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct MenuSection {
    label: WidgetText,
}

impl std::fmt::Debug for MenuSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuSection")
            .field("label", &self.label.text())
            .finish()
    }
}

impl MenuSection {
    /// Create a section header with the given label. The text is
    /// uppercased at render time.
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl Widget for MenuSection {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let pad_x = 10.0;
        let pad_top = 6.0;
        let pad_bottom = 2.0;

        let text = self.label.text().to_uppercase();
        let galley = crate::theme::placeholder_galley(ui, &text, t.small, false, f32::INFINITY);

        let desired = Vec2::new(
            galley.size().x + pad_x * 2.0,
            galley.size().y + pad_top + pad_bottom,
        );
        // Use `allocate_at_least` to inherit the parent menu's
        // top-down-justified width so the row spans the popup's interior.
        let (rect, response) = ui.allocate_at_least(desired, Sense::hover());

        if ui.is_rect_visible(rect) {
            let pos = Pos2::new(rect.min.x + pad_x, rect.min.y + pad_top);
            ui.painter().galley(pos, galley, p.text_faint);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
        response
    }
}
