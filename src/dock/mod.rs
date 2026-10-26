//! Tabbed split panes: a docking layout with draggable tabs.
//!
//! A [`Dock`] arranges tab groups in a tree of resizable splits. Users can
//! reorder tabs, drag them between groups, and drop them on a group's edge to
//! split it. The arrangement lives in a [`DockLayout`], which holds only tab
//! ids; the app supplies titles and content through a [`DockViewer`].
//!
//! Floating windows are not supported.
//!
//! # Example
//!
//! ```no_run
//! use elegance::{Dock, DockLayout, DockNode, DockTab, DockViewer};
//!
//! struct Viewer;
//!
//! impl DockViewer for Viewer {
//!     fn tab(&mut self, id: &str) -> DockTab {
//!         DockTab::new(id.to_uppercase())
//!     }
//!     fn ui(&mut self, ui: &mut egui::Ui, id: &str) {
//!         ui.label(format!("Contents of {id}"));
//!     }
//! }
//!
//! struct App {
//!     layout: DockLayout,
//! }
//!
//! impl Default for App {
//!     fn default() -> Self {
//!         Self {
//!             layout: DockLayout::from_node(DockNode::row(
//!                 0.3,
//!                 DockNode::tabs(["files"]),
//!                 DockNode::tabs(["editor", "preview"]),
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

use egui::{Response, Ui};

use crate::tab_paint::TabContent;

/// How one tab appears in its group's tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockTab {
    /// Display label. Truncated with an ellipsis if it doesn't fit.
    pub label: String,
    /// Optional leading icon glyph, typically from [`crate::glyphs`].
    pub icon: Option<String>,
    /// Show a small focus-accent dot to signal unsaved changes.
    pub dirty: bool,
    /// Show a close (×) button. Default: `true`.
    pub closable: bool,
}

impl DockTab {
    /// A closable tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            dirty: false,
            closable: true,
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
    /// Describe the tab with this id. Called every frame for each tab.
    fn tab(&mut self, id: &str) -> DockTab;

    /// Draw the contents of the active tab of a group.
    ///
    /// The content is clipped to its pane. Wrap it in an
    /// [`egui::ScrollArea`] if it can be larger than the pane.
    fn ui(&mut self, ui: &mut Ui, id: &str);

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
pub enum DockEvent {
    /// The user selected a different tab in a group. Carries its id.
    TabActivated(String),
    /// The user closed a tab. The dock has already removed it from the
    /// layout; the app can free any associated state.
    TabClosed(String),
    /// The user dragged a tab to a new position or group. Carries its id.
    TabMoved(String),
    /// The arrangement changed: a tab was activated, closed or moved, or a
    /// splitter was released. Useful as a signal to persist the layout.
    LayoutChanged,
}

/// The result of [`Dock::show`].
#[derive(Debug)]
pub struct DockResponse {
    /// The response covering the whole dock area.
    pub response: Response,
    /// What the user did this frame, in order.
    pub events: Vec<DockEvent>,
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
        layout: &mut DockLayout,
        viewer: &mut V,
    ) -> DockResponse {
        let base_id = ui.scope_id().with_salt(self.id_salt);
        let (response, events) = show::show(ui, base_id, &self, layout, viewer);
        DockResponse { response, events }
    }
}
