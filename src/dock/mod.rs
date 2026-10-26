//! Tabbed split panes: a docking layout with draggable tabs.
//!
//! A [`Dock`] arranges tab groups in a tree of resizable splits. Users can
//! reorder tabs, drag them between groups, and drop them on a group's edge to
//! split it. The arrangement lives in a [`DockLayout`] of the app's own tab
//! values; the app supplies titles and content through a [`DockViewer`].
//!
//! Floating windows are not supported.
//!
//! # Example
//!
//! ```no_run
//! use elegance::{Dock, DockLayout, DockNode, DockTab, DockViewer};
//!
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! enum Tab {
//!     Files,
//!     Channel(u8),
//! }
//!
//! struct Viewer;
//!
//! impl DockViewer for Viewer {
//!     type Tab = Tab;
//!
//!     fn tab(&mut self, tab: &Tab) -> DockTab {
//!         match tab {
//!             Tab::Files => DockTab::new("Files").closable(false),
//!             Tab::Channel(n) => DockTab::new(format!("Channel {n}")).scroll([false, true]),
//!         }
//!     }
//!     fn ui(&mut self, ui: &mut egui::Ui, tab: &Tab) {
//!         match tab {
//!             Tab::Files => ui.label("Files"),
//!             Tab::Channel(n) => ui.label(format!("Samples from channel {n}")),
//!         };
//!     }
//! }
//!
//! struct App {
//!     layout: DockLayout<Tab>,
//! }
//!
//! impl Default for App {
//!     fn default() -> Self {
//!         Self {
//!             layout: DockLayout::from_node(DockNode::row(
//!                 0.3,
//!                 DockNode::tabs([Tab::Files]),
//!                 DockNode::tabs([Tab::Channel(1), Tab::Channel(2)]),
//!             )),
//!         }
//!     }
//! }
//!
//! # impl App {
//! fn ui(&mut self, ui: &mut egui::Ui) {
//!     let out = Dock::new("workspace").show(ui, &mut self.layout, &mut Viewer);
//!     for event in out.events {
//!         // React to TabClosed, TabMoved, LayoutChanged, ...
//!         let _ = event;
//!     }
//! }
//! # }
//! ```

mod drag;
mod layout;
mod show;

pub use layout::{DockLayout, DockNode, SplitDirection};

use std::hash::Hash;

use egui::{Response, Ui, Vec2b};

use crate::tab_paint::TabContent;

/// How one tab appears in its group's tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockTab {
    /// Display label. Truncated with an ellipsis if it doesn't fit, in which
    /// case hovering the tab shows it in full.
    pub label: String,
    /// Optional leading icon glyph, typically from [`crate::glyphs`].
    pub icon: Option<String>,
    /// Show a small focus-accent dot to signal unsaved changes.
    pub dirty: bool,
    /// Show a close (×) button. Default: `true`.
    pub closable: bool,
    /// Open a context menu on right-click, filled by
    /// [`DockViewer::context_menu`]. Default: `false`.
    pub context_menu: bool,
    /// Scroll the tab's content on these axes when it outgrows the pane,
    /// instead of clipping it. Default: neither.
    pub scroll: Vec2b,
}

impl DockTab {
    /// A closable tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            dirty: false,
            closable: true,
            context_menu: false,
            scroll: Vec2b::FALSE,
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

    /// Show or hide the close button.
    #[inline]
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Enable or disable the right-click context menu. Its items come from
    /// [`DockViewer::context_menu`].
    #[inline]
    pub fn context_menu(mut self, context_menu: bool) -> Self {
        self.context_menu = context_menu;
        self
    }

    /// Scroll the content on these axes when it outgrows the pane: `true`
    /// for both, or `[horizontal, vertical]`. The scroll bars sit at the
    /// pane's edge, outside [`Dock::body_padding`].
    #[inline]
    pub fn scroll(mut self, scroll: impl Into<Vec2b>) -> Self {
        self.scroll = scroll.into();
        self
    }

    pub(crate) fn content(&self) -> TabContent<'_> {
        TabContent {
            label: &self.label,
            icon: self.icon.as_deref(),
            dirty: self.dirty,
            closable: self.closable,
        }
    }
}

/// Supplies the content and appearance of the tabs in a [`Dock`].
pub trait DockViewer {
    /// The tab type stored in the [`DockLayout`]. Each tab appears in the
    /// layout once. Its hash keys the tab's widgets and drag state, so
    /// distinct tabs should hash differently, as any derived `Hash` does.
    type Tab: Clone + Eq + Hash;

    /// Describe a tab. Called every frame for each tab in the layout.
    fn tab(&mut self, tab: &Self::Tab) -> DockTab;

    /// Draw the contents of the active tab of a group.
    ///
    /// The content is clipped to its pane unless the tab opts into scrolling
    /// with [`DockTab::scroll`].
    fn ui(&mut self, ui: &mut Ui, tab: &Self::Tab);

    /// Fill the right-click menu of a tab whose [`DockTab::context_menu`]
    /// is set. Add [`crate::MenuItem`]s and friends; clicking one closes
    /// the menu.
    ///
    /// The layout is borrowed while the dock is drawn, so record what the
    /// user picked and apply it to the [`DockLayout`] after [`Dock::show`]
    /// returns (for example with [`DockLayout::group_tabs`] and
    /// [`DockLayout::remove_tab`]).
    fn context_menu(&mut self, ui: &mut Ui, tab: &Self::Tab) {
        let _ = (ui, tab);
    }

    /// Draw the dock when the layout has no tabs. Defaults to a muted
    /// "No open tabs" message.
    fn empty_ui(&mut self, ui: &mut Ui) {
        let theme = crate::Theme::current(ui.ctx());
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No open tabs")
                    .color(theme.palette.text_faint)
                    .size(theme.typography.body),
            );
        });
    }
}

/// Something that happened in a [`Dock`] during a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DockEvent<T> {
    /// The user selected a different tab in a group.
    TabActivated(T),
    /// The user closed a tab. The dock has already removed it from the
    /// layout; the app can free any associated state.
    TabClosed(T),
    /// The user dragged a tab to a new position or group.
    TabMoved(T),
    /// The arrangement changed: a tab was activated, closed or moved, or a
    /// splitter was released. Useful as a signal to persist the layout.
    LayoutChanged,
}

/// The result of [`Dock::show`].
#[derive(Debug)]
pub struct DockResponse<T> {
    /// The response covering the whole dock area.
    pub response: Response,
    /// What the user did this frame, in order.
    pub events: Vec<DockEvent<T>>,
}

/// Tabbed split panes. See the module docs for an example.
///
/// Fills the available space.
#[must_use = "Call `.show(ui, layout, viewer)` to render the dock."]
#[derive(Debug)]
pub struct Dock {
    id_salt: egui::IdSalt,
    min_pane_size: f32,
    body_padding: f32,
}

impl Dock {
    /// Create a dock. `id_salt` scopes its interaction state in egui memory;
    /// two docks in the same `Ui` need distinct salts.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance::dock", id_salt)),
            min_pane_size: 96.0,
            body_padding: 8.0,
        }
    }

    /// Smallest width or height, in points, a splitter can shrink a pane to.
    /// Default: `96.0`.
    #[inline]
    pub fn min_pane_size(mut self, size: f32) -> Self {
        self.min_pane_size = size.max(0.0);
        self
    }

    /// Padding between a pane's border and its tab content. Default: `8.0`.
    #[inline]
    pub fn body_padding(mut self, padding: f32) -> Self {
        self.body_padding = padding.max(0.0);
        self
    }

    /// Render the dock into the available space.
    ///
    /// Mutations the user makes (activating, closing, moving tabs, resizing
    /// splits) are applied to `layout` before this returns.
    pub fn show<V: DockViewer>(
        self,
        ui: &mut Ui,
        layout: &mut DockLayout<V::Tab>,
        viewer: &mut V,
    ) -> DockResponse<V::Tab> {
        let base_id = ui.scope_id().with_salt(self.id_salt);
        let (response, events) = show::show(ui, base_id, &self, layout, viewer);
        DockResponse { response, events }
    }
}
