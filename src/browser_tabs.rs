//! Browser-style closable tabs with a dirty indicator.
//!
//! [`BrowserTabs`] is an owned-state widget for an editor- or browser-style
//! strip of tabs. The caller stores the widget on their app struct (so the
//! tab list and selection survive across frames), pushes tabs in via
//! [`BrowserTabs::add_tab`], and reacts to user actions by draining
//! [`BrowserTabs::take_events`] each frame.
//!
//! Each [`BrowserTab`] carries a stable string id, a label, an optional icon
//! glyph, and a `dirty` flag. The dirty flag paints a small focus-accent dot to
//! signal unsaved changes. The active tab fills with the theme's card
//! colour so it visually merges with the panel below.
//!
//! # Example
//!
//! ```no_run
//! use elegance::{BrowserTab, BrowserTabs, BrowserTabsEvent};
//!
//! struct App { tabs: BrowserTabs, untitled: u32 }
//!
//! impl Default for App {
//!     fn default() -> Self {
//!         let tabs = BrowserTabs::new("editor")
//!             .with_tab(BrowserTab::new("readme", "README.md"))
//!             .with_tab(BrowserTab::new("theme", "theme.rs").dirty(true))
//!             .with_tab(BrowserTab::new("button", "widgets/button.rs"));
//!         Self { tabs, untitled: 0 }
//!     }
//! }
//!
//! # impl App {
//! fn ui(&mut self, ui: &mut egui::Ui) {
//!     self.tabs.show(ui);
//!     for ev in self.tabs.take_events() {
//!         if let BrowserTabsEvent::NewRequested = ev {
//!             self.untitled += 1;
//!             let id = format!("untitled-{}", self.untitled);
//!             let label = format!("Untitled-{}", self.untitled);
//!             self.tabs.add_tab(BrowserTab::new(id, label));
//!         }
//!     }
//! }
//! # }
//! ```

use egui::{CornerRadius, Rect, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2};

use crate::tab_paint::{self, TAB_GAP, TabContent, TabState};
use crate::theme::Theme;

/// A single tab cell rendered by [`BrowserTabs`].
#[derive(Clone, Debug)]
pub struct BrowserTab {
    /// Stable identifier used to track selection and emit events.
    pub id: String,
    /// Display label. Truncated with an ellipsis if it doesn't fit.
    pub label: String,
    /// Optional leading icon glyph. Pass any string; typically a single
    /// glyph from a font like Lucide via [`crate::glyphs`].
    pub icon: Option<String>,
    /// Show a small focus-accent dot to signal unsaved changes.
    pub dirty: bool,
}

impl BrowserTab {
    /// Create a new tab with a stable id and a display label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            dirty: false,
        }
    }

    /// Set a leading icon glyph (e.g. one from [`crate::glyphs`]).
    #[inline]
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Mark or clear the unsaved-changes indicator.
    #[inline]
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }
}

/// Events emitted by [`BrowserTabs`] for a frame.
///
/// Drain via [`BrowserTabs::take_events`]; the queue is cleared each call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserTabsEvent {
    /// The user changed the active tab. Carries the id of the newly-active tab.
    Activated(String),
    /// The user clicked the close (×) button. The widget has already
    /// removed the tab from its list when this fires; the caller can free
    /// any associated state.
    Closed(String),
    /// The user clicked the trailing "+" button. The widget does NOT add a
    /// tab automatically; the caller decides label / icon / id and calls
    /// [`BrowserTabs::add_tab`].
    NewRequested,
}

const STRIP_PAD_X: f32 = 8.0;
const STRIP_PAD_Y: f32 = 8.0;
const NEW_BTN_SIZE: f32 = 28.0;
const NEW_BTN_INNER: f32 = 14.0;
const NEW_BTN_RADIUS: u8 = 5;
const NEW_BTN_GAP: f32 = 4.0;

const DEFAULT_MIN_TAB_WIDTH: f32 = 120.0;
const DEFAULT_MAX_TAB_WIDTH: f32 = 220.0;

/// A horizontal strip of browser-style closable tabs.
///
/// See the module-level docs for an example.
#[must_use = "Call `.show(ui)` to render the widget."]
pub struct BrowserTabs {
    id_salt: egui::IdSalt,
    tabs: Vec<BrowserTab>,
    selected: Option<String>,
    show_new_button: bool,
    min_tab_width: f32,
    max_tab_width: f32,
    events: Vec<BrowserTabsEvent>,
}

impl Default for BrowserTabs {
    fn default() -> Self {
        Self::new("elegance::browser_tabs")
    }
}

impl std::fmt::Debug for BrowserTabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserTabs")
            .field("id_salt", &self.id_salt)
            .field("tabs", &self.tabs.len())
            .field("selected", &self.selected)
            .field("show_new_button", &self.show_new_button)
            .field("min_tab_width", &self.min_tab_width)
            .field("max_tab_width", &self.max_tab_width)
            .field("events", &self.events.len())
            .finish()
    }
}

impl BrowserTabs {
    /// Create an empty strip. `id_salt` scopes the widget's interaction
    /// state in egui memory; two `BrowserTabs` strips in the same `Ui`
    /// need distinct salts.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance::browser_tabs", id_salt)),
            tabs: Vec::new(),
            selected: None,
            show_new_button: true,
            min_tab_width: DEFAULT_MIN_TAB_WIDTH,
            max_tab_width: DEFAULT_MAX_TAB_WIDTH,
            events: Vec::new(),
        }
    }

    /// Push a tab at construction time (builder form).
    #[inline]
    pub fn with_tab(mut self, tab: BrowserTab) -> Self {
        self.add_tab(tab);
        self
    }

    /// Whether to show the trailing "+" button. Default: `true`.
    #[inline]
    pub fn show_new_button(mut self, show: bool) -> Self {
        self.show_new_button = show;
        self
    }

    /// Minimum width for a single tab in points. Default: `120.0`.
    #[inline]
    pub fn min_tab_width(mut self, w: f32) -> Self {
        self.min_tab_width = w.max(60.0);
        if self.max_tab_width < self.min_tab_width {
            self.max_tab_width = self.min_tab_width;
        }
        self
    }

    /// Maximum width for a single tab in points. Default: `220.0`.
    #[inline]
    pub fn max_tab_width(mut self, w: f32) -> Self {
        self.max_tab_width = w.max(self.min_tab_width);
        self
    }

    /// Append a tab. If no tab is currently selected, the new tab becomes
    /// the active one. Subsequent calls don't change the active tab unless
    /// the caller invokes [`Self::set_selected`].
    pub fn add_tab(&mut self, tab: BrowserTab) {
        if self.selected.is_none() {
            self.selected = Some(tab.id.clone());
        }
        self.tabs.push(tab);
    }

    /// Remove a tab by id. If the removed tab was active, selection moves
    /// to the next tab (or the previous one if it was the last). Returns
    /// `true` if a tab was actually removed.
    pub fn remove_tab(&mut self, id: &str) -> bool {
        let Some(pos) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };
        let was_selected = self.selected.as_deref() == Some(id);
        self.tabs.remove(pos);
        if was_selected {
            self.selected = self
                .tabs
                .get(pos)
                .or_else(|| self.tabs.get(pos.saturating_sub(1)))
                .map(|t| t.id.clone());
        }
        true
    }

    /// Currently active tab id, if any.
    #[inline]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Set the active tab. No-op if no tab with that id exists.
    pub fn set_selected(&mut self, id: impl Into<String>) {
        let id = id.into();
        if self.tabs.iter().any(|t| t.id == id) {
            self.selected = Some(id);
        }
    }

    /// Borrow the underlying tab list.
    #[inline]
    pub fn tabs(&self) -> &[BrowserTab] {
        &self.tabs
    }

    /// Find a tab by id.
    pub fn tab(&self, id: &str) -> Option<&BrowserTab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Find a tab by id (mutably) so the caller can flip `dirty` or rename it.
    pub fn tab_mut(&mut self, id: &str) -> Option<&mut BrowserTab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Drain events queued since the last call. Returns events in the order
    /// they were emitted.
    pub fn take_events(&mut self) -> Vec<BrowserTabsEvent> {
        std::mem::take(&mut self.events)
    }

    /// Render the strip. Call once per frame.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let base_id = ui.scope_id().with_salt(self.id_salt);

        let tab_height = tab_paint::tab_height(&theme);
        let strip_height = STRIP_PAD_Y + tab_height;

        let avail_w = ui.available_width();
        let (strip_rect, response) =
            ui.allocate_exact_size(Vec2::new(avail_w, strip_height), Sense::hover());

        if !ui.is_rect_visible(strip_rect) {
            response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "browser tabs"));
            return response;
        }

        let strip_bg = tab_paint::strip_bg(&theme);
        ui.painter()
            .rect_filled(strip_rect, CornerRadius::ZERO, strip_bg);

        let active_id = self.selected.clone();
        let mut active_rect: Option<Rect> = None;
        let mut activate_target: Option<String> = None;
        let mut close_target: Option<String> = None;
        let mut new_clicked = false;

        let tabs_top = strip_rect.min.y + STRIP_PAD_Y;
        let mut x = strip_rect.min.x + STRIP_PAD_X;

        for tab in self.tabs.iter() {
            let content = TabContent {
                label: &tab.label,
                icon: tab.icon.as_deref(),
                dirty: tab.dirty,
                closable: true,
            };
            let measured = tab_paint::measure_tab(
                ui,
                &theme,
                &content,
                self.min_tab_width,
                self.max_tab_width,
            );
            let tab_w = measured.width;
            let tab_rect = Rect::from_min_size(pos2(x, tabs_top), vec2(tab_w, tab_height));

            let tab_id = base_id.with(("tab", tab.id.as_str()));
            let resp = ui.interact(tab_rect, tab_id, Sense::click());

            let close_rect = tab_paint::close_rect(tab_rect);
            let close_id = base_id.with(("close", tab.id.as_str()));
            let close_resp = ui.interact(close_rect, close_id, Sense::click());

            let is_active = active_id.as_deref() == Some(tab.id.as_str());
            let state = TabState {
                active: is_active,
                hovered: resp.hovered() || close_resp.hovered(),
                close_hovered: close_resp.hovered(),
                close_enabled: true,
                strip_bg,
            };
            tab_paint::paint_tab(ui.painter(), &theme, tab_rect, &content, measured, state);

            if is_active {
                active_rect = Some(tab_rect);
            }

            let info_active = is_active;
            let info_label = tab.label.clone();
            resp.widget_info(move || {
                WidgetInfo::selected(Role::Button, true, info_active, &info_label)
            });
            let close_label = format!("Close {}", tab.label);
            close_resp.widget_info(move || WidgetInfo::labeled(Role::Button, true, &close_label));

            if close_resp.clicked() {
                close_target = Some(tab.id.clone());
            } else if resp.clicked() {
                activate_target = Some(tab.id.clone());
            }

            x += tab_w + TAB_GAP;
        }

        if self.show_new_button {
            x += NEW_BTN_GAP;
            let btn_y = tabs_top + tab_height - NEW_BTN_SIZE - 2.0;
            let new_rect = Rect::from_min_size(pos2(x, btn_y), Vec2::splat(NEW_BTN_SIZE));
            let new_id = base_id.with("new_tab");
            let new_resp = ui.interact(new_rect, new_id, Sense::click());

            let painter = ui.painter();
            let hovered = new_resp.hovered();
            if hovered {
                painter.rect_filled(
                    new_rect,
                    CornerRadius::same(NEW_BTN_RADIUS),
                    p.depth_tint(strip_bg, 0.06),
                );
            }
            let cross_color = if hovered { p.text } else { p.text_faint };
            let center = new_rect.center();
            let half = NEW_BTN_INNER * 0.5;
            let stroke = Stroke::new(1.5, cross_color);
            painter.line_segment(
                [
                    pos2(center.x, center.y - half),
                    pos2(center.x, center.y + half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center.x - half, center.y),
                    pos2(center.x + half, center.y),
                ],
                stroke,
            );
            new_resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, "New tab"));

            if new_resp.clicked() {
                new_clicked = true;
            }
        }

        tab_paint::paint_strip_border(ui.painter(), &theme, strip_rect, active_rect);

        if let Some(id) = activate_target
            && self.selected.as_deref() != Some(id.as_str())
        {
            self.selected = Some(id.clone());
            self.events.push(BrowserTabsEvent::Activated(id));
        }
        if let Some(id) = close_target
            && self.remove_tab(&id)
        {
            self.events.push(BrowserTabsEvent::Closed(id));
            if let Some(new_active) = self.selected.clone() {
                self.events.push(BrowserTabsEvent::Activated(new_active));
            }
        }
        if new_clicked {
            self.events.push(BrowserTabsEvent::NewRequested);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "browser tabs"));
        response
    }
}
