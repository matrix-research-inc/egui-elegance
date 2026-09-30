//! Top-of-window menu bar — a horizontal strip of click-to-open menus.
//!
//! [`MenuBar`] paints a desktop-style menu strip with an optional brand on
//! the left, a row of menu triggers (File, Edit, View, …), and an optional
//! status slot on the right. Clicking a trigger opens its dropdown; once
//! one menu is open, hovering a sibling trigger switches to it. That
//! "menu mode" hover-switching matches how native menubars feel.
//!
//! ```no_run
//! # use elegance::{MenuBar, MenuItem};
//! # egui::__run_test_ui(|ui| {
//! MenuBar::new("app_menubar")
//!     .brand("Elegance")
//!     .status("main \u{00b7} up to date")
//!     .show(ui, |bar| {
//!         bar.menu("File", |ui| {
//!             ui.add(MenuItem::new("New").shortcut("\u{2318}N"));
//!             ui.add(MenuItem::new("Open\u{2026}").shortcut("\u{2318}O"));
//!             ui.separator();
//!             ui.add(MenuItem::new("Save").shortcut("\u{2318}S"));
//!         });
//!         bar.menu("Edit", |ui| {
//!             ui.add(MenuItem::new("Undo").shortcut("\u{2318}Z"));
//!         });
//!     });
//! # });
//! ```
//!
//! By default the brand renders as a sky-blue square followed by the label.
//! Use [`MenuBar::brand_logo`] to swap the square for a different accent
//! colour, a unicode glyph, an image, or to omit the logo slot entirely:
//!
//! ```no_run
//! # use elegance::{Accent, BrandLogo, MenuBar};
//! # egui::__run_test_ui(|ui| {
//! MenuBar::new("app_menubar")
//!     .brand("Elegance")
//!     .brand_logo(BrandLogo::Glyph("\u{25C6}".into()))
//!     .show(ui, |_bar| {});
//! # });
//! ```
//!
//! Dropdowns close on outside-click, `Esc`, or clicking an item.
//!
//! Custom widgets go in two places. [`MenuBarUi::ui`] exposes the strip's
//! [`Ui`] for content inline with the triggers, and [`MenuBarUi::trailing`]
//! pins content to the right edge, outside the status slot.
//!
//! For an app that hides the native title bar, the strip can stand in for
//! it: [`MenuBar::title_bar`] lets its empty area move and maximise the
//! window, and the trailing slot holds the window controls:
//!
//! ```no_run
//! # use elegance::{Button, ButtonSize, MenuBar, MenuItem, glyphs};
//! # egui::__run_test_ui(|ui| {
//! MenuBar::new("app_menubar")
//!     .brand("Elegance")
//!     .status("main \u{00b7} up to date")
//!     .title_bar(true)
//!     .show(ui, |bar| {
//!         bar.menu("File", |ui| {
//!             ui.add(MenuItem::new("Quit"));
//!         });
//!         bar.trailing(|ui| {
//!             let close = Button::icon(glyphs::X, "Close window")
//!                 .outline()
//!                 .size(ButtonSize::Small);
//!             if ui.add(close).clicked() {
//!                 ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
//!             }
//!         });
//!     });
//! # });
//! ```
//!
//! For a single click-to-open menu attached to an arbitrary trigger button,
//! use [`Menu`](crate::Menu) directly.

use egui::{
    Align, Color32, Context, CornerRadius, Frame, Id, ImageSource, Layout, Margin, PointerButton,
    Popup, PopupCloseBehavior, Pos2, Rect, Response, Sense, SetOpenCommand, Stroke, Ui, UiBuilder,
    Vec2, ViewportCommand, WidgetInfo, WidgetText, WidgetType, emath::RectAlign,
};

use crate::theme::{Accent, Theme, mix, with_alpha};

const STRIP_PAD_Y: f32 = 4.0;
const STRIP_PAD_X: f32 = 6.0;
const TRIGGER_PAD_X: f32 = 10.0;
const TRIGGER_PAD_Y: f32 = 5.0;
const BRAND_LOGO_SIZE: f32 = 14.0;
/// Gap between the trailing slot's content and the status to its left.
const TRAILING_STATUS_GAP: f32 = 8.0;

#[derive(Debug, Clone)]
struct StatusContent {
    text: WidgetText,
    dot: Option<Color32>,
}

/// What to paint in the brand's logo slot, left of the brand label.
///
/// The default ([`BrandLogo::Square`] with [`Accent::Sky`]) is used when a
/// [`MenuBar`] sets brand text without an explicit logo. Use
/// [`MenuBar::brand_logo`] to override it.
///
/// The slot is opinionated: it is a fixed size and paints glyphs in the
/// theme's body text colour. Pick the variant that matches the kind of
/// logo you have, and use [`BrandLogo::Image`] when you need full control
/// over colour and detail.
#[derive(Debug, Clone)]
pub enum BrandLogo {
    /// Omit the logo slot entirely. Use this to suppress the default
    /// square next to a brand label, giving a text-only brand that sits
    /// flush against the strip's left padding.
    None,
    /// Filled square in the given accent colour. The library default
    /// (with [`Accent::Sky`]).
    Square(Accent),
    /// A unicode glyph, e.g. a character from an icon font like Material
    /// Icons or Phosphor. Painted in the theme's body text colour at the
    /// fixed logo size; styling beyond the glyph text itself is not
    /// honoured. For a coloured or detailed mark, use [`BrandLogo::Image`].
    Glyph(String),
    /// A bitmap or SVG image, scaled to the logo size.
    ///
    /// The host app must have registered egui image loaders (e.g. via
    /// `egui_extras::install_image_loaders`) for this to render; without
    /// loaders, egui paints its placeholder.
    ///
    /// The `'static` bound suits the two common cases: a compiled-in
    /// asset from [`egui::include_image!`], or an owned URI string via
    /// `ImageSource::Uri(Cow::Owned(...))`. Borrowed URIs need to be
    /// promoted to owned (or interned) by the caller.
    Image(ImageSource<'static>),
}

impl Default for BrandLogo {
    fn default() -> Self {
        Self::Square(Accent::Sky)
    }
}

#[derive(Debug, Clone)]
struct Brand {
    text: Option<WidgetText>,
    /// `None` means "use the default logo if `text` is set, otherwise omit".
    logo: Option<BrandLogo>,
}

/// A horizontal desktop-style menu bar with click-to-open dropdowns.
///
/// See the module-level docs for an example.
#[derive(Debug, Clone)]
#[must_use = "Call `.show(ui, |bar| ...)` to render the menu bar."]
pub struct MenuBar {
    id_salt: Id,
    brand: Option<Brand>,
    status: Option<StatusContent>,
    title_bar: bool,
}

impl MenuBar {
    /// Create a new menu bar keyed by `id_salt`. The salt scopes per-menu
    /// open state in egui memory and must be stable across frames.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: Id::new(("elegance::menu_bar", Id::new(id_salt))),
            brand: None,
            status: None,
            title_bar: false,
        }
    }

    /// Show a brand label on the left. By default it is preceded by a small
    /// sky-blue accent square; call [`MenuBar::brand_logo`] to override that
    /// visual.
    ///
    /// Use this for the application name.
    #[inline]
    pub fn brand(mut self, text: impl Into<WidgetText>) -> Self {
        let logo = self.brand.and_then(|b| b.logo);
        self.brand = Some(Brand {
            text: Some(text.into()),
            logo,
        });
        self
    }

    /// Override the brand's logo slot. Pass [`BrandLogo::Square`] with a
    /// different [`Accent`] to recolour the default square,
    /// [`BrandLogo::Glyph`] for a unicode/icon-font glyph, [`BrandLogo::Image`]
    /// for a bitmap or SVG, or [`BrandLogo::None`] for a text-only brand.
    ///
    /// Calling this without [`MenuBar::brand`] renders the logo on its own
    /// (no brand label).
    #[inline]
    pub fn brand_logo(mut self, logo: BrandLogo) -> Self {
        let text = self.brand.and_then(|b| b.text);
        self.brand = Some(Brand {
            text,
            logo: Some(logo),
        });
        self
    }

    /// Show a muted status line on the right (e.g. `"main · up to date"`).
    #[inline]
    pub fn status(mut self, text: impl Into<WidgetText>) -> Self {
        self.status = Some(StatusContent {
            text: text.into(),
            dot: None,
        });
        self
    }

    /// Show a status line preceded by a coloured dot, useful for indicating
    /// connection or run state (green for healthy, amber for running, red
    /// for failing).
    #[inline]
    pub fn status_with_dot(mut self, text: impl Into<WidgetText>, dot: Color32) -> Self {
        self.status = Some(StatusContent {
            text: text.into(),
            dot: Some(dot),
        });
        self
    }

    /// Make the strip act as the window's title bar, for apps that hide the
    /// native one (e.g. with `ViewportBuilder::with_decorations(false)`).
    /// Dragging the strip's empty area moves the window, and
    /// double-clicking it toggles maximised. Default: `false`.
    ///
    /// The brand and status take no input, so they count as empty area.
    /// Widgets that sense clicks or drags keep their input: menu triggers,
    /// trailing buttons, and any interactive widget added through
    /// [`MenuBarUi::ui`]. On desktop that includes egui's selectable
    /// labels, which select text on drag rather than moving the window;
    /// add them with `.selectable(false)` if they should act as empty area.
    #[inline]
    pub fn title_bar(mut self, title_bar: bool) -> Self {
        self.title_bar = title_bar;
        self
    }

    /// Render the menu bar. The closure receives a [`MenuBarUi`] used to
    /// declare each menu's trigger label and dropdown body.
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut MenuBarUi<'_>) -> R) -> R {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        // The strip sits between the page background and the elevated card
        // tone — close to body bg on dark themes, close to card on light
        // themes. Mixing keeps the same visual relationship across all four
        // built-in palettes.
        let menubar_fill = mix(p.bg, p.card, 0.45);

        // Read the previous frame's snapshot: which menus existed, where
        // their triggers were, and whether any was open.
        let state_id = self.id_salt.with("__state");
        let press_id = self.id_salt.with("__title_bar_press");
        let prev_state: MenuBarFrameState = ui
            .ctx()
            .data(|d| d.get_temp::<MenuBarFrameState>(state_id))
            .unwrap_or_default();

        // Hover arbitrator. If a sibling trigger is being hovered while
        // another menu of ours is currently open, close that other menu
        // *before* any popup renders this frame. The new menu's normal
        // hover-switch logic then opens itself in its own paint, and only
        // one popup ends up visible. Without this step, the previously
        // open menu would render its area for one extra frame because its
        // `Popup::show` runs (and renders) before the new menu's call to
        // `open_id` overwrites the memory slot.
        if prev_state.any_open
            && let Some(pointer) = ui.ctx().pointer_hover_pos()
        {
            let open_idx = prev_state
                .triggers
                .iter()
                .position(|(id, _)| Popup::is_id_open(ui.ctx(), *id));
            if let Some(open_idx) = open_idx {
                let on_sibling = prev_state
                    .triggers
                    .iter()
                    .enumerate()
                    .any(|(i, (_, rect))| i != open_idx && rect.contains(pointer));
                if on_sibling {
                    Popup::close_id(ui.ctx(), prev_state.triggers[open_idx].0);
                }
            }
        }

        let frame = Frame::new()
            .fill(menubar_fill)
            .inner_margin(Margin::symmetric(STRIP_PAD_X as i8, STRIP_PAD_Y as i8));

        // A `Ui`'s own sense sits below the widgets inside it, so in title
        // bar mode the strip receives only the presses its triggers and
        // custom widgets don't claim. Clicks only: the strip must not be a
        // focus stop, and sensing drags would let it take over drags that
        // start on click-only widgets (see `move_window_from_strip`).
        let strip_sense = if self.title_bar {
            Sense::CLICK
        } else {
            Sense::hover()
        };
        let strip = ui.scope_builder(UiBuilder::new().sense(strip_sense), |ui| {
            frame
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Triggers abut, carrying their own padding. The caller's
                        // spacing is kept for the trailing slot, where custom
                        // widgets sit side by side.
                        let item_spacing_x = ui.spacing().item_spacing.x;
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.set_min_height(theme.typography.body + TRIGGER_PAD_Y * 2.0);

                        if let Some(brand) = self.brand.as_ref() {
                            paint_brand(ui, &theme, brand);
                        }

                        let mut bar = MenuBarUi {
                            ui,
                            base_id: self.id_salt,
                            next_idx: 0,
                            any_open_prev: prev_state.any_open,
                            any_open_now: false,
                            triggers: Vec::with_capacity(prev_state.triggers.len()),
                            status: self.status.as_ref(),
                            item_spacing_x,
                            trailing_shown: false,
                        };
                        let r = body(&mut bar);
                        let any_open_now = bar.any_open_now;
                        let triggers = std::mem::take(&mut bar.triggers);

                        // `trailing` paints the status alongside its own content;
                        // otherwise the status still needs its slot.
                        if !bar.trailing_shown
                            && let Some(status) = bar.status
                        {
                            bar.ui
                                .with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    paint_status(ui, &theme, status);
                                });
                        }

                        bar.ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                state_id,
                                MenuBarFrameState {
                                    triggers,
                                    any_open: any_open_now,
                                },
                            )
                        });

                        r
                    })
                    .inner
                })
                .inner
        });

        if self.title_bar {
            move_window_from_strip(ui.ctx(), &strip.response, press_id);
        }

        // Bottom border separates the strip from the body content below.
        let strip_rect = strip.response.rect;
        ui.painter().line_segment(
            [
                Pos2::new(strip_rect.min.x, strip_rect.max.y - 0.5),
                Pos2::new(strip_rect.max.x, strip_rect.max.y - 0.5),
            ],
            Stroke::new(1.0, p.border),
        );

        strip.inner
    }
}

/// Title bar behaviour for a press on the strip's empty area: a drag hands
/// the window to the platform to move, a double-click toggles maximised.
///
/// The strip senses clicks but not drags. Sensing drags would hand it
/// drags that start on click-only widgets such as buttons, which egui
/// routes to the drag-sensing widget behind them, so it tracks the drag
/// itself: `press_id` keys a flag in egui memory recording whether the
/// press began on empty area, consumed once the press becomes a drag.
fn move_window_from_strip(ctx: &Context, strip: &Response, press_id: Id) {
    let (pressed, dragging) = ctx.input(|i| {
        let pointer = &i.pointer;
        (
            pointer.primary_pressed(),
            pointer.primary_down() && pointer.is_decidedly_dragging(),
        )
    });
    if pressed {
        // egui hovers a click-only widget on a press only when it is the
        // click target, i.e. no widget in front of the strip claimed it.
        let on_empty = ctx.interaction_snapshot(|s| s.hovered.contains(&strip.id));
        ctx.data_mut(|d| d.insert_temp(press_id, on_empty));
    }
    if dragging && ctx.data_mut(|d| d.remove_temp::<bool>(press_id)) == Some(true) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    // egui's double-click check is timing only, so a click on a widget
    // followed quickly by one on empty area also toggles. Rare enough to
    // leave, as tracking the first click's target would add state.
    if strip.double_clicked_by(PointerButton::Primary) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}

/// Per-frame state stored in [`egui::Memory`] so the next frame can know
/// where each trigger sat and whether any of our menus was open.
#[derive(Clone, Default, Debug)]
struct MenuBarFrameState {
    triggers: Vec<(Id, Rect)>,
    any_open: bool,
}

/// The handle passed to a [`MenuBar::show`] closure for declaring menu
/// triggers. Each call to [`MenuBarUi::menu`] paints one trigger and its
/// dropdown. Custom widgets go inline via [`MenuBarUi::ui`] or at the
/// right edge via [`MenuBarUi::trailing`].
pub struct MenuBarUi<'u> {
    ui: &'u mut Ui,
    base_id: Id,
    next_idx: usize,
    any_open_prev: bool,
    any_open_now: bool,
    triggers: Vec<(Id, Rect)>,
    status: Option<&'u StatusContent>,
    /// The item spacing in effect before the strip zeroed it, restored for
    /// the trailing slot's content.
    item_spacing_x: f32,
    trailing_shown: bool,
}

impl<'u> std::fmt::Debug for MenuBarUi<'u> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuBarUi")
            .field("base_id", &self.base_id)
            .field("next_idx", &self.next_idx)
            .field("any_open_prev", &self.any_open_prev)
            .field("any_open_now", &self.any_open_now)
            .field("trailing_shown", &self.trailing_shown)
            .finish()
    }
}

impl<'u> MenuBarUi<'u> {
    /// The strip's [`Ui`], for custom widgets inline with the menu
    /// triggers. Widgets added here follow the triggers declared so far,
    /// left to right, vertically centred in the strip.
    ///
    /// Item spacing is zero, since the triggers carry their own padding;
    /// use [`Ui::add_space`] to separate custom widgets from their
    /// neighbours. For content pinned to the right edge, such as window
    /// controls, use [`MenuBarUi::trailing`] rather than a right-to-left
    /// layout here, which would claim the width the status slot needs.
    #[inline]
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }

    /// Pin content to the right edge of the strip, outside the status
    /// slot, which moves left to make room. Suited to window controls
    /// (close, maximise, minimise) when the bar serves as an app's title
    /// bar, or to an account avatar or settings button.
    ///
    /// The content is laid out right to left, as in egui's
    /// [`Layout::right_to_left`]: the first widget added sits at the right
    /// edge and each later one lands to its left. Item spacing is the
    /// caller's, as it was before [`MenuBar::show`] ran.
    ///
    /// Call this once, after the last menu: it claims the rest of the
    /// strip's width, so a trigger declared afterwards has nowhere to go.
    /// Both mistakes panic in debug builds.
    pub fn trailing<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        debug_assert!(
            !self.trailing_shown,
            "MenuBarUi::trailing called more than once; add all trailing content in one call"
        );
        self.trailing_shown = true;

        let status = self.status;
        let item_spacing_x = self.item_spacing_x;
        self.ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                let inner = ui
                    .scope(|ui| {
                        ui.spacing_mut().item_spacing.x = item_spacing_x;
                        add_contents(ui)
                    })
                    .inner;
                if let Some(status) = status {
                    ui.add_space(TRAILING_STATUS_GAP);
                    paint_status(ui, &Theme::current(ui.ctx()), status);
                }
                inner
            })
            .inner
    }

    /// Paint a single menu trigger with `label` and attach a dropdown
    /// populated by `body`. Clicking an item inside the dropdown dismisses
    /// the menu — the standard pattern for action-style menus (File / Edit
    /// / etc.). For settings-style menus that should stay open while the
    /// user toggles items, use [`MenuBarUi::menu_keep_open`].
    ///
    /// Returns `Some` with the body closure's return value while the
    /// dropdown is open, `None` while it's closed.
    pub fn menu<R>(
        &mut self,
        label: impl Into<WidgetText>,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        self.menu_inner(label, PopupCloseBehavior::CloseOnClick, body)
    }

    /// Like [`MenuBarUi::menu`], but the dropdown stays open while the
    /// user clicks items inside it. Useful for menus full of toggles
    /// (checkboxes, radio groups) where the user expects to see the state
    /// change without the menu vanishing. The menu still closes on click
    /// outside, on `Esc`, or when the user clicks the trigger again.
    pub fn menu_keep_open<R>(
        &mut self,
        label: impl Into<WidgetText>,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        self.menu_inner(label, PopupCloseBehavior::CloseOnClickOutside, body)
    }

    fn menu_inner<R>(
        &mut self,
        label: impl Into<WidgetText>,
        close_behavior: PopupCloseBehavior,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        debug_assert!(
            !self.trailing_shown,
            "MenuBarUi menu declared after MenuBarUi::trailing; declare every menu first"
        );
        let label: WidgetText = label.into();
        let theme = Theme::current(self.ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let idx = self.next_idx;
        self.next_idx += 1;
        let popup_id = self.base_id.with("__menu").with(idx);

        let galley =
            crate::theme::placeholder_galley(self.ui, label.text(), t.body, false, f32::INFINITY);
        let trigger_size = Vec2::new(
            galley.size().x + TRIGGER_PAD_X * 2.0,
            galley.size().y + TRIGGER_PAD_Y * 2.0,
        );
        let (rect, response) = self.ui.allocate_exact_size(trigger_size, Sense::click());
        self.triggers.push((popup_id, rect));

        let was_open = Popup::is_id_open(self.ui.ctx(), popup_id);
        let hovered = response.hovered();
        let clicked = response.clicked();

        // Decide what to do this frame.
        //   1. Click: toggle this menu open/closed.
        //   2. Hover while another menu of ours is already open: switch
        //      to this one. `Bool(true)` opens this id and closes others
        //      (egui memory only tracks one open popup at a time).
        //   3. Otherwise: leave the state as-is.
        let intent: Option<SetOpenCommand> = if clicked {
            Some(SetOpenCommand::Bool(!was_open))
        } else if self.any_open_prev && hovered && !was_open {
            Some(SetOpenCommand::Bool(true))
        } else {
            None
        };

        let will_be_open = matches!(intent, Some(SetOpenCommand::Bool(true)))
            || (was_open && !matches!(intent, Some(SetOpenCommand::Bool(false))));
        self.any_open_now |= will_be_open;

        if self.ui.is_rect_visible(rect) {
            let bg = if will_be_open {
                p.card
            } else if hovered {
                with_alpha(p.text, 14)
            } else {
                Color32::TRANSPARENT
            };
            if bg.a() > 0 {
                self.ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);
            }
            let text_color = if will_be_open || hovered {
                p.text
            } else {
                p.text_muted
            };
            let pos = Pos2::new(
                rect.min.x + TRIGGER_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            );
            self.ui.painter().galley(pos, galley, text_color);
        }

        // Dropdown panel. Top-left corner is square so the panel reads
        // visually flush with the trigger above it.
        let r = theme.card_radius as u8;
        let frame = Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius {
                nw: 0,
                ne: r,
                sw: r,
                se: r,
            })
            .inner_margin(Margin::same(4));

        let label_text = label.text().to_string();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label_text));

        let result = Popup::menu(&response)
            .id(popup_id)
            .open_memory(intent)
            .align(RectAlign::BOTTOM_START)
            .gap(0.0)
            .frame(frame)
            .close_behavior(close_behavior)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                body(ui)
            });

        result.map(|r| r.inner)
    }
}

fn paint_brand(ui: &mut Ui, theme: &Theme, brand: &Brand) {
    let p = &theme.palette;
    let t = &theme.typography;

    // Resolve the effective logo: an explicit override wins; otherwise the
    // library default applies when there is brand text to sit next to.
    // `Brand` is constructed only via `MenuBar::brand` or
    // `MenuBar::brand_logo`, each of which sets at least one of the two
    // fields — so the (None, None) state is unreachable.
    let effective_logo = match (&brand.logo, &brand.text) {
        (Some(logo), _) => logo.clone(),
        (None, Some(_)) => BrandLogo::default(),
        (None, None) => unreachable!("Brand requires at least one of text or logo"),
    };

    let logo_painted = paint_brand_logo(ui, theme, &effective_logo);

    if logo_painted && brand.text.is_some() {
        ui.add_space(8.0);
    }

    if let Some(text) = brand.text.as_ref() {
        let galley = crate::theme::placeholder_galley(ui, text.text(), t.body, true, f32::INFINITY);
        let label_size = Vec2::new(galley.size().x, galley.size().y + 4.0);
        let (rect, _) = ui.allocate_exact_size(label_size, Sense::hover());
        let pos = Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5);
        ui.painter().galley(pos, galley, p.text);
    }

    if logo_painted || brand.text.is_some() {
        ui.add_space(14.0);
    }
}

/// Paint the logo slot. Returns `true` if anything occupied the slot, so
/// the caller knows whether to insert the gap before the brand label.
fn paint_brand_logo(ui: &mut Ui, theme: &Theme, logo: &BrandLogo) -> bool {
    let p = &theme.palette;
    let size = Vec2::splat(BRAND_LOGO_SIZE);

    match logo {
        BrandLogo::None => false,
        BrandLogo::Square(accent) => {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), p.accent_fill(*accent));
            true
        }
        BrandLogo::Glyph(glyph) => {
            let galley =
                crate::theme::placeholder_galley(ui, glyph, BRAND_LOGO_SIZE, true, f32::INFINITY);
            // Allocate enough room for the actual glyph metrics — line
            // height at a 14pt body font typically exceeds BRAND_LOGO_SIZE,
            // so an undersized slot would lie to the surrounding layout.
            let slot = Vec2::new(
                galley.size().x.max(BRAND_LOGO_SIZE),
                galley.size().y.max(BRAND_LOGO_SIZE),
            );
            let (rect, _) = ui.allocate_exact_size(slot, Sense::hover());
            let pos = Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(pos, galley, p.text);
            true
        }
        BrandLogo::Image(src) => {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            egui::Image::new(src.clone())
                .fit_to_exact_size(size)
                .paint_at(ui, rect);
            true
        }
    }
}

fn paint_status(ui: &mut Ui, theme: &Theme, status: &StatusContent) {
    let p = &theme.palette;
    let t = &theme.typography;

    // Layout is right-to-left here, so allocations come from the right edge
    // inward — paint text first, then the dot to the left of it.
    ui.add_space(4.0);
    let galley =
        crate::theme::placeholder_galley(ui, status.text.text(), t.small, false, f32::INFINITY);
    let label_size = Vec2::new(galley.size().x, galley.size().y + 4.0);
    let (rect, _) = ui.allocate_exact_size(label_size, Sense::hover());
    let pos = Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, p.text_faint);

    if let Some(color) = status.dot {
        ui.add_space(6.0);
        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
        ui.painter().circle_filled(dot_rect.center(), 3.5, color);
    }
}
