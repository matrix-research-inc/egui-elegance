//! The dock's layout tree: pure data, no egui types.
//!
//! A layout is a binary tree whose leaves are tab groups and whose interior
//! nodes are resizable splits. Tabs are identified by caller-supplied string
//! ids, so the layout never needs to know what a tab contains.

/// Which way a split arranges its two children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitDirection {
    /// Children sit side by side, first on the left.
    Row,
    /// Children are stacked, first on top.
    Column,
}

/// A declarative description of a dock layout, used to build a
/// [`DockLayout`].
///
/// ```
/// use elegance::{DockLayout, DockNode};
///
/// let layout = DockLayout::from_node(DockNode::row(
///     0.25,
///     DockNode::tabs(["files"]),
///     DockNode::column(
///         0.7,
///         DockNode::tabs(["editor", "preview"]),
///         DockNode::tabs(["console"]),
///     ),
/// ));
/// assert_eq!(layout.group_count(), 3);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
    /// A group of tabs. The first tab starts active.
    Tabs(Vec<String>),
    /// Two nodes divided by a draggable splitter.
    Split {
        /// How the children are arranged.
        direction: SplitDirection,
        /// Share of the space given to `first`, clamped to `[0, 1]`. Panes
        /// never shrink below [`Dock::min_pane_size`](crate::Dock::min_pane_size).
        fraction: f32,
        /// Left (row) or top (column) child.
        first: Box<DockNode>,
        /// Right (row) or bottom (column) child.
        second: Box<DockNode>,
    },
}

impl DockNode {
    /// A tab group holding `ids` in order.
    pub fn tabs<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Tabs(ids.into_iter().map(Into::into).collect())
    }

    /// Two nodes side by side. `fraction` is the share given to `first`.
    pub fn row(fraction: f32, first: DockNode, second: DockNode) -> Self {
        Self::split(SplitDirection::Row, fraction, first, second)
    }

    /// Two nodes stacked vertically. `fraction` is the share given to `first`.
    pub fn column(fraction: f32, first: DockNode, second: DockNode) -> Self {
        Self::split(SplitDirection::Column, fraction, first, second)
    }

    fn split(direction: SplitDirection, fraction: f32, first: DockNode, second: DockNode) -> Self {
        Self::Split {
            direction,
            fraction,
            first: Box::new(first),
            second: Box::new(second),
        }
    }
}

/// Stable identity of a tab group within one [`DockLayout`].
pub(crate) type GroupId = u64;

/// A leaf of the layout tree.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TabGroup {
    pub id: GroupId,
    pub tabs: Vec<String>,
    pub active: usize,
}

impl TabGroup {
    pub fn active_tab(&self) -> Option<&str> {
        self.tabs.get(self.active).map(String::as_str)
    }

    /// Remove the tab at `pos`, keeping the same tab active where possible.
    fn remove_at(&mut self, pos: usize) {
        self.tabs.remove(pos);
        if pos < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum Node {
    Split {
        direction: SplitDirection,
        fraction: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
    Group(TabGroup),
}

/// Which edge of a group a tab was dropped on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// Where a dragged tab lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropTarget {
    /// Insert into a group at a gap index (`0..=tabs.len()`, measured before
    /// the dragged tab is removed).
    Group { group: GroupId, index: usize },
    /// Split the group and place the tab in the new half on `side`.
    Edge { group: GroupId, side: Side },
}

impl DropTarget {
    pub fn group(self) -> GroupId {
        match self {
            Self::Group { group, .. } | Self::Edge { group, .. } => group,
        }
    }
}

/// The arrangement of tabs in a [`Dock`](crate::Dock).
///
/// The layout owns only tab ids, so it is cheap to clone and, with the
/// `serde` feature, easy to persist. The app keeps it across frames and
/// passes it to [`Dock::show`](crate::Dock::show).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockLayout {
    pub(crate) root: Option<Node>,
    next_group: GroupId,
    /// The group that most recently received user interaction. New tabs land
    /// here.
    focused: Option<GroupId>,
}

impl DockLayout {
    /// An empty layout. [`Self::add_tab`] creates the first group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a layout from a declarative description.
    ///
    /// A tab id that appears more than once keeps its first occurrence, and
    /// empty groups are dropped.
    pub fn from_node(node: DockNode) -> Self {
        let mut layout = Self::new();
        layout.root = Some(layout.build(node));
        layout.sanitize();
        layout
    }

    /// A layout with a single group holding `ids`.
    pub fn single<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::from_node(DockNode::tabs(ids))
    }

    fn build(&mut self, node: DockNode) -> Node {
        match node {
            DockNode::Tabs(tabs) => Node::Group(TabGroup {
                id: self.alloc_group(),
                tabs,
                active: 0,
            }),
            DockNode::Split {
                direction,
                fraction,
                first,
                second,
            } => {
                let first = self.build(*first);
                let second = self.build(*second);
                Node::Split {
                    direction,
                    fraction,
                    first: Box::new(first),
                    second: Box::new(second),
                }
            }
        }
    }

    /// Whether the layout holds no tabs.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Number of tab groups.
    pub fn group_count(&self) -> usize {
        let mut n = 0;
        if let Some(root) = &self.root {
            root.for_each_group(&mut |_| n += 1);
        }
        n
    }

    /// Every tab id, in layout order (left to right, top to bottom).
    pub fn tab_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| ids.extend(g.tabs.iter().map(String::as_str)));
        }
        ids
    }

    /// The active tab id of every group, in layout order.
    pub fn active_tab_ids(&self) -> Vec<&str> {
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| ids.extend(g.active_tab()));
        }
        ids
    }

    /// Whether a tab with this id is in the layout.
    pub fn contains(&self, tab: &str) -> bool {
        self.group_of(tab).is_some()
    }

    /// Add a tab to the focused group (the first group if none has focus) and
    /// make it active. An existing tab with the same id is activated instead.
    pub fn add_tab(&mut self, tab: impl Into<String>) {
        let tab = tab.into();
        if self.activate(&tab) {
            return;
        }
        let target = self.focused_group();
        match target.and_then(|id| self.group_mut(id)) {
            Some(group) => {
                group.tabs.push(tab);
                group.active = group.tabs.len() - 1;
            }
            None => {
                let id = self.alloc_group();
                self.root = Some(Node::Group(TabGroup {
                    id,
                    tabs: vec![tab],
                    active: 0,
                }));
                self.focused = Some(id);
            }
        }
    }

    /// Remove a tab. Emptied groups are dropped and their space goes to the
    /// sibling. Returns `true` if the tab was present.
    pub fn remove_tab(&mut self, tab: &str) -> bool {
        let mut removed = false;
        if let Some(root) = &mut self.root {
            root.for_each_group_mut(&mut |g| {
                if let Some(pos) = g.tabs.iter().position(|t| t == tab) {
                    g.remove_at(pos);
                    removed = true;
                }
            });
        }
        if removed {
            self.root = self.root.take().and_then(prune);
        }
        removed
    }

    /// Make `tab` the active tab of its group and focus that group. Returns
    /// `false` if the tab is not in the layout.
    pub fn activate(&mut self, tab: &str) -> bool {
        let Some(id) = self.group_of(tab) else {
            return false;
        };
        if let Some(group) = self.group_mut(id) {
            group.active = group.tabs.iter().position(|t| t == tab).unwrap_or(0);
        }
        self.focused = Some(id);
        true
    }

    pub(crate) fn group_of(&self, tab: &str) -> Option<GroupId> {
        let mut found = None;
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| {
                if found.is_none() && g.tabs.iter().any(|t| t == tab) {
                    found = Some(g.id);
                }
            });
        }
        found
    }

    pub(crate) fn group_mut(&mut self, id: GroupId) -> Option<&mut TabGroup> {
        self.root.as_mut().and_then(|root| root.group_mut(id))
    }

    pub(crate) fn group(&self, id: GroupId) -> Option<&TabGroup> {
        self.root.as_ref().and_then(|root| root.group(id))
    }

    /// The focused group, falling back to the first group in layout order.
    pub(crate) fn focused_group(&self) -> Option<GroupId> {
        if let Some(id) = self.focused
            && self.group(id).is_some()
        {
            return Some(id);
        }
        let mut first = None;
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| {
                first.get_or_insert(g.id);
            });
        }
        first
    }

    pub(crate) fn set_focused(&mut self, id: GroupId) {
        self.focused = Some(id);
    }

    fn alloc_group(&mut self) -> GroupId {
        let id = self.next_group;
        self.next_group += 1;
        id
    }

    /// Apply a completed drag. Returns `true` if the layout changed.
    pub(crate) fn move_tab(&mut self, tab: &str, target: DropTarget) -> bool {
        let Some(source) = self.group_of(tab) else {
            return false;
        };
        let target_group = target.group();
        let Some(target_len) = self.group(target_group).map(|g| g.tabs.len()) else {
            return false;
        };

        match target {
            DropTarget::Group { group, index } if source == group => {
                let Some(g) = self.group_mut(group) else {
                    return false;
                };
                let from = g.tabs.iter().position(|t| t == tab).unwrap_or(0);
                let mut to = index.min(target_len);
                if from < to {
                    to -= 1;
                }
                if from == to {
                    g.active = from;
                    self.focused = Some(group);
                    return false;
                }
                let moved = g.tabs.remove(from);
                g.tabs.insert(to, moved);
                g.active = to;
            }
            DropTarget::Group { group, index } => {
                self.remove_tab(tab);
                // The target group survives: it differs from the source.
                if let Some(g) = self.group_mut(group) {
                    let at = index.min(g.tabs.len());
                    g.tabs.insert(at, tab.to_owned());
                    g.active = at;
                }
            }
            DropTarget::Edge { group, side } => {
                if source == group && target_len == 1 {
                    return false;
                }
                self.remove_tab(tab);
                let new_id = self.alloc_group();
                if let Some(root) = self.root.take() {
                    self.root = Some(split_group(root, group, side, tab, new_id));
                }
                self.focused = Some(new_id);
                return true;
            }
        }
        self.focused = Some(target_group);
        true
    }

    /// Restore invariants after construction or deserialization: drop
    /// duplicate tab ids and empty groups, collapse single-child splits, clamp
    /// `active` and split fractions, and keep group ids unique and ahead of
    /// the allocator.
    pub(crate) fn sanitize(&mut self) {
        let Some(mut root) = self.root.take() else {
            return;
        };
        let mut next = self.next_group;
        root.for_each_group(&mut |g| next = next.max(g.id + 1));

        let mut seen_tabs = std::collections::HashSet::new();
        let mut seen_ids = std::collections::HashSet::new();
        root.for_each_group_mut(&mut |g| {
            g.tabs.retain(|t| seen_tabs.insert(t.clone()));
            g.active = g.active.min(g.tabs.len().saturating_sub(1));
            if !seen_ids.insert(g.id) {
                g.id = next;
                next += 1;
                seen_ids.insert(g.id);
            }
        });
        self.next_group = next;
        self.root = prune(root);
    }
}

impl Node {
    pub(crate) fn for_each_group<'a>(&'a self, f: &mut impl FnMut(&'a TabGroup)) {
        match self {
            Self::Group(g) => f(g),
            Self::Split { first, second, .. } => {
                first.for_each_group(f);
                second.for_each_group(f);
            }
        }
    }

    fn for_each_group_mut(&mut self, f: &mut impl FnMut(&mut TabGroup)) {
        match self {
            Self::Group(g) => f(g),
            Self::Split { first, second, .. } => {
                first.for_each_group_mut(f);
                second.for_each_group_mut(f);
            }
        }
    }

    fn group(&self, id: GroupId) -> Option<&TabGroup> {
        match self {
            Self::Group(g) => (g.id == id).then_some(g),
            Self::Split { first, second, .. } => first.group(id).or_else(|| second.group(id)),
        }
    }

    fn group_mut(&mut self, id: GroupId) -> Option<&mut TabGroup> {
        match self {
            Self::Group(g) => (g.id == id).then_some(g),
            Self::Split { first, second, .. } => {
                first.group_mut(id).or_else(|| second.group_mut(id))
            }
        }
    }
}

/// Drop empty groups and collapse splits left with one child, so the space
/// goes to the sibling. Also normalises split fractions into `[0, 1]`.
fn prune(node: Node) -> Option<Node> {
    match node {
        Node::Group(g) => (!g.tabs.is_empty()).then_some(Node::Group(g)),
        Node::Split {
            direction,
            fraction,
            first,
            second,
        } => match (prune(*first), prune(*second)) {
            (Some(first), Some(second)) => Some(Node::Split {
                direction,
                fraction: if fraction.is_finite() {
                    fraction.clamp(0.0, 1.0)
                } else {
                    0.5
                },
                first: Box::new(first),
                second: Box::new(second),
            }),
            (only, None) | (None, only) => only,
        },
    }
}

fn split_group(node: Node, target: GroupId, side: Side, tab: &str, new_id: GroupId) -> Node {
    match node {
        Node::Group(g) if g.id == target => {
            let added = Node::Group(TabGroup {
                id: new_id,
                tabs: vec![tab.to_owned()],
                active: 0,
            });
            let existing = Node::Group(g);
            let (direction, first, second) = match side {
                Side::Left => (SplitDirection::Row, added, existing),
                Side::Right => (SplitDirection::Row, existing, added),
                Side::Top => (SplitDirection::Column, added, existing),
                Side::Bottom => (SplitDirection::Column, existing, added),
            };
            Node::Split {
                direction,
                fraction: 0.5,
                first: Box::new(first),
                second: Box::new(second),
            }
        }
        Node::Group(g) => Node::Group(g),
        Node::Split {
            direction,
            fraction,
            first,
            second,
        } => Node::Split {
            direction,
            fraction,
            first: Box::new(split_group(*first, target, side, tab, new_id)),
            second: Box::new(split_group(*second, target, side, tab, new_id)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_pane() -> DockLayout {
        DockLayout::from_node(DockNode::row(
            0.25,
            DockNode::tabs(["files"]),
            DockNode::column(
                0.7,
                DockNode::tabs(["editor", "preview"]),
                DockNode::tabs(["console"]),
            ),
        ))
    }

    fn group_tabs(layout: &DockLayout, tab: &str) -> Vec<String> {
        let id = layout.group_of(tab).expect("tab present");
        layout.group(id).expect("group present").tabs.clone()
    }

    #[test]
    fn from_node_builds_groups_in_order() {
        let layout = three_pane();
        assert_eq!(layout.group_count(), 3);
        assert_eq!(
            layout.tab_ids(),
            vec!["files", "editor", "preview", "console"]
        );
        assert_eq!(layout.active_tab_ids(), vec!["files", "editor", "console"]);
    }

    #[test]
    fn from_node_drops_duplicates_and_empty_groups() {
        let layout = DockLayout::from_node(DockNode::row(
            0.5,
            DockNode::tabs(["a", "b"]),
            DockNode::tabs(["b", "c"]),
        ));
        assert_eq!(layout.tab_ids(), vec!["a", "b", "c"]);

        let layout = DockLayout::from_node(DockNode::row(
            0.5,
            DockNode::tabs(["a"]),
            DockNode::Tabs(Vec::new()),
        ));
        assert_eq!(layout.group_count(), 1);
        assert!(matches!(layout.root, Some(Node::Group(_))));
    }

    #[test]
    fn fractions_are_normalised_into_unit_range() {
        let fraction_of = |f: f32| {
            let layout = DockLayout::from_node(DockNode::row(
                f,
                DockNode::tabs(["a"]),
                DockNode::tabs(["b"]),
            ));
            let Some(Node::Split { fraction, .. }) = layout.root else {
                panic!("expected a split");
            };
            fraction
        };
        assert_eq!(fraction_of(2.0), 1.0);
        assert_eq!(fraction_of(-1.0), 0.0);
        assert_eq!(fraction_of(f32::NAN), 0.5);
        // Small fractions survive: the pane minimum is enforced in pixels at
        // layout time, not here.
        assert_eq!(fraction_of(0.02), 0.02);
    }

    #[test]
    fn removing_last_tab_collapses_the_split() {
        let mut layout = three_pane();
        assert!(layout.remove_tab("console"));
        assert_eq!(layout.group_count(), 2);
        // `editor`'s group now fills the old column.
        let Some(Node::Split { second, .. }) = &layout.root else {
            panic!("expected root split");
        };
        assert!(matches!(**second, Node::Group(_)));
        assert!(!layout.remove_tab("console"));
    }

    #[test]
    fn removing_the_only_tab_empties_the_layout() {
        let mut layout = DockLayout::single(["a"]);
        assert!(layout.remove_tab("a"));
        assert!(layout.is_empty());
    }

    #[test]
    fn removing_a_tab_keeps_the_active_tab() {
        let mut layout = DockLayout::single(["a", "b", "c"]);
        layout.activate("c");
        layout.remove_tab("a");
        assert_eq!(layout.active_tab_ids(), vec!["c"]);
        layout.remove_tab("c");
        assert_eq!(layout.active_tab_ids(), vec!["b"]);
    }

    #[test]
    fn add_tab_goes_to_focused_group_and_activates() {
        let mut layout = three_pane();
        layout.activate("console");
        layout.add_tab("output");
        assert_eq!(group_tabs(&layout, "output"), vec!["console", "output"]);
        assert_eq!(layout.active_tab_ids(), vec!["files", "editor", "output"]);

        layout.add_tab("files");
        assert_eq!(layout.tab_ids().len(), 5, "existing id is not duplicated");
        assert_eq!(layout.active_tab_ids()[0], "files");
    }

    #[test]
    fn add_tab_to_empty_layout_creates_a_group() {
        let mut layout = DockLayout::new();
        layout.add_tab("a");
        assert_eq!(layout.tab_ids(), vec!["a"]);
    }

    #[test]
    fn move_within_group_reorders() {
        let mut layout = DockLayout::single(["a", "b", "c"]);
        let group = layout.group_of("a").unwrap();
        // Dropping `a` in the gap after `c`.
        assert!(layout.move_tab("a", DropTarget::Group { group, index: 3 }));
        assert_eq!(group_tabs(&layout, "a"), vec!["b", "c", "a"]);
        assert_eq!(layout.active_tab_ids(), vec!["a"]);

        // Dropping `c` back into the gap right after itself is a no-op.
        assert!(!layout.move_tab("c", DropTarget::Group { group, index: 2 }));
        assert!(!layout.move_tab("c", DropTarget::Group { group, index: 1 }));
        assert_eq!(group_tabs(&layout, "a"), vec!["b", "c", "a"]);
    }

    #[test]
    fn move_between_groups_inserts_at_index() {
        let mut layout = three_pane();
        let editor_group = layout.group_of("editor").unwrap();
        assert!(layout.move_tab(
            "files",
            DropTarget::Group {
                group: editor_group,
                index: 1,
            }
        ));
        assert_eq!(
            group_tabs(&layout, "files"),
            vec!["editor", "files", "preview"]
        );
        // The emptied source group is gone.
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.active_tab_ids()[0], "files");
    }

    #[test]
    fn move_to_edge_creates_a_split() {
        let mut layout = three_pane();
        let files_group = layout.group_of("files").unwrap();
        assert!(layout.move_tab(
            "preview",
            DropTarget::Edge {
                group: files_group,
                side: Side::Bottom,
            }
        ));
        assert_eq!(layout.group_count(), 4);
        assert_eq!(group_tabs(&layout, "preview"), vec!["preview"]);
        assert_eq!(group_tabs(&layout, "editor"), vec!["editor"]);
        // `files` above, `preview` below.
        let ids = layout.tab_ids();
        let pos = |t| ids.iter().position(|x| *x == t).unwrap();
        assert!(pos("files") < pos("preview"));
    }

    #[test]
    fn move_to_edge_of_own_group_splits_it() {
        let mut layout = DockLayout::single(["a", "b"]);
        let group = layout.group_of("a").unwrap();
        assert!(layout.move_tab(
            "b",
            DropTarget::Edge {
                group,
                side: Side::Left,
            }
        ));
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.tab_ids(), vec!["b", "a"]);
    }

    #[test]
    fn move_sole_tab_to_own_edge_is_a_no_op() {
        let mut layout = DockLayout::single(["a"]);
        let group = layout.group_of("a").unwrap();
        let before = layout.clone();
        assert!(!layout.move_tab(
            "a",
            DropTarget::Edge {
                group,
                side: Side::Right,
            }
        ));
        assert_eq!(layout, before);
    }

    #[test]
    fn group_ids_stay_unique_after_splits() {
        let mut layout = DockLayout::single(["a", "b", "c"]);
        for (tab, side) in [("b", Side::Right), ("c", Side::Bottom)] {
            let group = layout.group_of("a").unwrap();
            layout.move_tab(tab, DropTarget::Edge { group, side });
        }
        let mut ids = Vec::new();
        layout
            .root
            .as_ref()
            .unwrap()
            .for_each_group(&mut |g| ids.push(g.id));
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn move_unknown_tab_or_group_is_rejected() {
        let mut layout = three_pane();
        let before = layout.clone();
        assert!(!layout.move_tab("nope", DropTarget::Group { group: 0, index: 0 }));
        assert!(!layout.move_tab(
            "files",
            DropTarget::Group {
                group: 999,
                index: 0
            }
        ));
        assert_eq!(layout, before);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let mut layout = three_pane();
        layout.activate("preview");
        let json = serde_json::to_string(&layout).unwrap();
        let back: DockLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(back, layout);
    }

    #[test]
    fn sanitize_makes_duplicate_group_ids_unique() {
        let group = |id, tab: &str| {
            Box::new(Node::Group(TabGroup {
                id,
                tabs: vec![tab.into()],
                active: 0,
            }))
        };
        let mut layout = DockLayout {
            root: Some(Node::Split {
                direction: SplitDirection::Row,
                fraction: 0.5,
                first: group(4, "a"),
                second: group(4, "b"),
            }),
            next_group: 0,
            focused: None,
        };
        layout.sanitize();
        let mut ids = Vec::new();
        layout
            .root
            .as_ref()
            .unwrap()
            .for_each_group(&mut |g| ids.push(g.id));
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        // Dropping into the second pane must reach the second pane.
        let second = layout.group_of("b").unwrap();
        assert!(layout.move_tab(
            "a",
            DropTarget::Group {
                group: second,
                index: 1
            }
        ));
        assert_eq!(group_tabs(&layout, "a"), vec!["b", "a"]);
    }

    #[test]
    fn sanitize_repairs_a_hand_edited_layout() {
        let mut layout = DockLayout {
            root: Some(Node::Split {
                direction: SplitDirection::Row,
                fraction: f32::NAN,
                first: Box::new(Node::Group(TabGroup {
                    id: 7,
                    tabs: vec!["a".into(), "a".into()],
                    active: 9,
                })),
                second: Box::new(Node::Group(TabGroup {
                    id: 8,
                    tabs: Vec::new(),
                    active: 0,
                })),
            }),
            next_group: 0,
            focused: None,
        };
        layout.sanitize();
        assert_eq!(layout.tab_ids(), vec!["a"]);
        assert!(matches!(layout.root, Some(Node::Group(_))));
        layout.add_tab("b");
        layout.activate("b");
        let g = layout.group_of("a").unwrap();
        let moved = layout.move_tab(
            "b",
            DropTarget::Edge {
                group: g,
                side: Side::Right,
            },
        );
        assert!(moved);
        assert!(layout.next_group > 8, "allocator moved past existing ids");
    }
}
