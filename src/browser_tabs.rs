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

use egui::{
    Color32, CornerRadius, FontId, FontSelection, Id, Rect, Response, RichText, Role, Sense,
    Stroke, TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText, pos2, vec2,
};

use crate::theme::{Theme, with_alpha};

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
const TAB_PAD_X: f32 = 10.0;
const TAB_PAD_Y: f32 = 7.0;
const TAB_GAP: f32 = 2.0;
const TAB_RADIUS: f32 = 7.0;
const ICON_SIZE: f32 = 12.0;
const INNER_GAP: f32 = 8.0;
const DIRTY_SIZE: f32 = 7.0;
const DIRTY_TO_CLOSE_GAP: f32 = 5.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_INNER: f32 = 9.0;
const CLOSE_RADIUS: u8 = 4;
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
    id_salt: Id,
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
    /// state in egui memory; two `BrowserTabs` strips on the same page
    /// need distinct salts.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: Id::unique(("elegance::browser_tabs", id_salt)),
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
        let t = &theme.typography;

        let label_size = t.small + 0.5;
        let tab_height = TAB_PAD_Y * 2.0 + label_size.max(ICON_SIZE);
        let strip_height = STRIP_PAD_Y + tab_height;

        let avail_w = ui.available_width();
        let (strip_rect, response) =
            ui.allocate_exact_size(Vec2::new(avail_w, strip_height), Sense::hover());

        if !ui.is_rect_visible(strip_rect) {
            response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "browser tabs"));
            return response;
        }

        let strip_bg = p.input_bg;
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
            let icon_w = if tab.icon.is_some() {
                ICON_SIZE + INNER_GAP
            } else {
                0.0
            };
            let dirty_block_w = if tab.dirty {
                INNER_GAP + DIRTY_SIZE + DIRTY_TO_CLOSE_GAP
            } else {
                INNER_GAP
            };
            let close_w = CLOSE_SIZE;
            let max_label_w =
                (self.max_tab_width - 2.0 * TAB_PAD_X - icon_w - dirty_block_w - close_w).max(0.0);

            let label_galley = WidgetText::from(
                RichText::new(&tab.label)
                    .size(label_size)
                    .color(Color32::PLACEHOLDER),
            )
            .into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                max_label_w,
                FontSelection::FontId(FontId::proportional(label_size)),
            );
            let label_w = label_galley.size().x.min(max_label_w);

            let mut tab_w = 2.0 * TAB_PAD_X + icon_w + label_w + dirty_block_w + close_w;
            tab_w = tab_w.clamp(self.min_tab_width, self.max_tab_width);
            let tab_rect = Rect::from_min_size(pos2(x, tabs_top), vec2(tab_w, tab_height));

            let tab_id = self.id_salt.with(("tab", tab.id.as_str()));
            let resp = ui.interact(tab_rect, tab_id, Sense::click());

            let close_center = pos2(
                tab_rect.max.x - TAB_PAD_X - CLOSE_SIZE * 0.5,
                tab_rect.center().y,
            );
            let close_rect = Rect::from_center_size(close_center, Vec2::splat(CLOSE_SIZE));
            let close_id = self.id_salt.with(("close", tab.id.as_str()));
            let close_resp = ui.interact(close_rect, close_id, Sense::click());

            let is_active = active_id.as_deref() == Some(tab.id.as_str());
            let any_hover = resp.hovered() || close_resp.hovered();

            let radius = CornerRadius {
                nw: TAB_RADIUS as u8,
                ne: TAB_RADIUS as u8,
                sw: 0,
                se: 0,
            };
            let painter = ui.painter();

            let (fill, label_color, icon_color) = if is_active {
                (p.card, p.text, p.focus)
            } else if any_hover {
                (p.depth_tint(strip_bg, 0.06), p.text, p.text_muted)
            } else {
                (p.depth_tint(strip_bg, 0.02), p.text_muted, p.text_faint)
            };
            painter.rect_filled(tab_rect, radius, fill);

            // Active tab gets a 1px top edge in the border colour plus side
            // highlights so it reads as the lifted "merged with the panel
            // below" cell. Inactive tabs get a suppressed outline (alpha
            // ~110/255) so each cell stays distinct without competing.
            let (top_color, side_color) = if is_active {
                (p.border, p.depth_tint(p.card, 0.04))
            } else {
                let outline = with_alpha(p.border, 110);
                (outline, outline)
            };
            painter.line_segment(
                [
                    pos2(tab_rect.min.x + TAB_RADIUS, tab_rect.min.y + 0.5),
                    pos2(tab_rect.max.x - TAB_RADIUS, tab_rect.min.y + 0.5),
                ],
                Stroke::new(1.0, top_color),
            );
            painter.line_segment(
                [
                    pos2(tab_rect.min.x + 0.5, tab_rect.min.y + TAB_RADIUS),
                    pos2(tab_rect.min.x + 0.5, tab_rect.max.y),
                ],
                Stroke::new(1.0, side_color),
            );
            painter.line_segment(
                [
                    pos2(tab_rect.max.x - 0.5, tab_rect.min.y + TAB_RADIUS),
                    pos2(tab_rect.max.x - 0.5, tab_rect.max.y),
                ],
                Stroke::new(1.0, side_color),
            );

            let mut cursor_x = tab_rect.min.x + TAB_PAD_X;
            let cy = tab_rect.center().y;

            if let Some(icon) = &tab.icon {
                let icon_galley = WidgetText::from(
                    RichText::new(icon)
                        .size(ICON_SIZE)
                        .color(Color32::PLACEHOLDER),
                )
                .into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    FontSelection::FontId(FontId::proportional(ICON_SIZE)),
                );
                let painter = ui.painter();
                painter.galley(
                    pos2(cursor_x, cy - icon_galley.size().y * 0.5),
                    icon_galley,
                    icon_color,
                );
                cursor_x += ICON_SIZE + INNER_GAP;
            }

            let painter = ui.painter();
            let label_pos = pos2(cursor_x, cy - label_galley.size().y * 0.5);
            painter.galley(label_pos, label_galley, label_color);

            if tab.dirty {
                let dot_x = close_rect.min.x - DIRTY_TO_CLOSE_GAP - DIRTY_SIZE * 0.5;
                painter.circle_filled(pos2(dot_x, cy), DIRTY_SIZE * 0.5, p.focus);
            }

            let close_visible = is_active || any_hover;
            if close_visible {
                if close_resp.hovered() {
                    let close_bg = p.depth_tint(if is_active { p.card } else { strip_bg }, 0.10);
                    painter.rect_filled(close_rect, CornerRadius::same(CLOSE_RADIUS), close_bg);
                }
                let cross_color = if close_resp.hovered() {
                    p.text
                } else if is_active {
                    p.text_muted
                } else {
                    p.text_faint
                };
                let half = CLOSE_INNER * 0.5;
                let stroke = Stroke::new(1.5, cross_color);
                painter.line_segment(
                    [
                        pos2(close_center.x - half, close_center.y - half),
                        pos2(close_center.x + half, close_center.y + half),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        pos2(close_center.x + half, close_center.y - half),
                        pos2(close_center.x - half, close_center.y + half),
                    ],
                    stroke,
                );
            }

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
            let new_id = self.id_salt.with("new_tab");
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
            let stroke = Stroke::new(2.0, cross_color);
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

        let border_y = strip_rect.bottom() - 0.5;
        let stroke = Stroke::new(1.0, p.border);
        let painter = ui.painter();
        if let Some(active) = active_rect {
            painter.line_segment(
                [
                    pos2(strip_rect.min.x, border_y),
                    pos2(active.min.x, border_y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(active.max.x, border_y),
                    pos2(strip_rect.max.x, border_y),
                ],
                stroke,
            );
        } else {
            painter.line_segment(
                [
                    pos2(strip_rect.min.x, border_y),
                    pos2(strip_rect.max.x, border_y),
                ],
                stroke,
            );
        }

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
