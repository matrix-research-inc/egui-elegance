//! The dock's layout tree: pure data, no egui types.
//!
//! A layout is a binary tree whose leaves are tab groups and whose interior
//! nodes are resizable splits. Tabs are values of the app's own type, so the
//! layout never needs to know what a tab contains.

use std::borrow::Borrow;
use std::collections::HashSet;
use std::hash::Hash;

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
/// let layout: DockLayout<String> = DockLayout::from_node(DockNode::row(
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
pub enum DockNode<T> {
    /// A group of tabs. The first tab starts active.
    Tabs(Vec<T>),
    /// Two nodes divided by a draggable splitter.
    Split {
        /// How the children are arranged.
        direction: SplitDirection,
        /// Share of the space given to `first`, clamped to `[0, 1]`. Panes
        /// never shrink below [`Dock::min_pane_size`](crate::Dock::min_pane_size).
        fraction: f32,
        /// Left (row) or top (column) child.
        first: Box<DockNode<T>>,
        /// Right (row) or bottom (column) child.
        second: Box<DockNode<T>>,
    },
}

impl<T> DockNode<T> {
    /// A tab group holding `tabs` in order.
    pub fn tabs<I>(tabs: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<T>,
    {
        Self::Tabs(tabs.into_iter().map(Into::into).collect())
    }

    /// Two nodes side by side. `fraction` is the share given to `first`.
    pub fn row(fraction: f32, first: DockNode<T>, second: DockNode<T>) -> Self {
        Self::split(SplitDirection::Row, fraction, first, second)
    }

    /// Two nodes stacked vertically. `fraction` is the share given to `first`.
    pub fn column(fraction: f32, first: DockNode<T>, second: DockNode<T>) -> Self {
        Self::split(SplitDirection::Column, fraction, first, second)
    }

    fn split(
        direction: SplitDirection,
        fraction: f32,
        first: DockNode<T>,
        second: DockNode<T>,
    ) -> Self {
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
pub(crate) struct TabGroup<T> {
    pub id: GroupId,
    pub tabs: Vec<T>,
    pub active: usize,
}

impl<T> TabGroup<T> {
    pub fn active_tab(&self) -> Option<&T> {
        self.tabs.get(self.active)
    }

    /// Remove the tab at `pos`, keeping the same tab active where possible.
    fn remove_at(&mut self, pos: usize) -> T {
        let tab = self.tabs.remove(pos);
        if pos < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        tab
    }

    fn position<Q>(&self, tab: &Q) -> Option<usize>
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.tabs.iter().position(|t| t.borrow() == tab)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum Node<T> {
    Split {
        direction: SplitDirection,
        fraction: f32,
        first: Box<Node<T>>,
        second: Box<Node<T>>,
    },
    Group(TabGroup<T>),
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
/// `T` is the app's tab type: a `String` id, or an enum whose variants carry
/// what each tab shows (a document path, a channel number). Tabs are compared
/// for equality, so each value may appear in the layout only once. The
/// [`DockViewer`](crate::DockViewer) turns a tab into its label and content.
///
/// The app keeps the layout across frames and passes it to
/// [`Dock::show`](crate::Dock::show). With the `serde` feature, it persists
/// whenever `T` does.
///
/// Lookups take any borrowed form of `T`, as `HashMap` does, so a
/// `DockLayout<String>` accepts `&str`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DockLayout<T> {
    pub(crate) root: Option<Node<T>>,
    next_group: GroupId,
    /// The group that most recently received user interaction. New tabs land
    /// here.
    focused: Option<GroupId>,
}

impl<T> Default for DockLayout<T> {
    fn default() -> Self {
        Self {
            root: None,
            next_group: 0,
            focused: None,
        }
    }
}

impl<T: Clone + Eq + Hash> DockLayout<T> {
    /// An empty layout. [`Self::add_tab`] creates the first group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a layout from a declarative description.
    ///
    /// A tab that appears more than once keeps its first occurrence, and
    /// empty groups are dropped.
    pub fn from_node(node: DockNode<T>) -> Self {
        let mut layout = Self::new();
        layout.root = Some(layout.build(node));
        layout.sanitize();
        layout
    }

    /// A layout with a single group holding `tabs`.
    pub fn single<I>(tabs: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<T>,
    {
        Self::from_node(DockNode::tabs(tabs))
    }

    fn build(&mut self, node: DockNode<T>) -> Node<T> {
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

    /// Every tab, in layout order (left to right, top to bottom).
    pub fn tabs(&self) -> Vec<&T> {
        let mut tabs = Vec::new();
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| tabs.extend(&g.tabs));
        }
        tabs
    }

    /// The active tab of every group, in layout order.
    pub fn active_tabs(&self) -> Vec<&T> {
        let mut tabs = Vec::new();
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| tabs.extend(g.active_tab()));
        }
        tabs
    }

    /// The tabs in the same group as `tab`, including `tab` itself, in strip
    /// order. Empty if `tab` is not in the layout.
    pub fn group_tabs<Q>(&self, tab: &Q) -> &[T]
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.group_of(tab)
            .and_then(|id| self.group(id))
            .map_or(&[], |g| &g.tabs)
    }

    /// Whether `tab` is in the layout.
    pub fn contains<Q>(&self, tab: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.group_of(tab).is_some()
    }

    /// Add a tab to the focused group (the first group if none has focus) and
    /// make it active. A tab already in the layout is activated instead.
    pub fn add_tab(&mut self, tab: impl Into<T>) {
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

    /// Remove a tab and return it, or `None` if it was not in the layout.
    /// Emptied groups are dropped and their space goes to the sibling.
    pub fn remove_tab<Q>(&mut self, tab: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let (id, pos) = self.locate(tab)?;
        let group = self.group_mut(id)?;
        let removed = group.remove_at(pos);
        if group.tabs.is_empty() {
            self.root = self.root.take().and_then(prune);
        }
        Some(removed)
    }

    /// Put `new` in place of the tab `old`, keeping its position and whether
    /// it is active, and return `old`'s value. Use it when a tab's identity
    /// changes, such as a viewer switching channels.
    ///
    /// If `new` is already elsewhere in the layout, `old` is removed and
    /// `new` activated instead, since a tab appears only once. Returns `None`
    /// and changes nothing if `old` is not in the layout.
    pub fn replace_tab<Q>(&mut self, old: &Q, new: impl Into<T>) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let new = new.into();
        let (id, pos) = self.locate(old)?;
        if <T as Borrow<Q>>::borrow(&new) != old && self.contains::<T>(&new) {
            let removed = self.remove_tab(old);
            self.activate::<T>(&new);
            return removed;
        }
        let group = self.group_mut(id)?;
        Some(std::mem::replace(&mut group.tabs[pos], new))
    }

    /// Keep only the tabs for which `keep` returns `true`, such as dropping
    /// documents that no longer exist after loading a saved layout. A group
    /// whose active tab is removed activates the tab that followed it, and
    /// emptied groups are dropped, their space going to the sibling.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        *self = std::mem::take(self).filter_map(|tab| keep(&tab).then_some(tab));
    }

    /// Make `tab` the active tab of its group and focus that group. Returns
    /// `false` if the tab is not in the layout.
    pub fn activate<Q>(&mut self, tab: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let Some((id, pos)) = self.locate(tab) else {
            return false;
        };
        if let Some(group) = self.group_mut(id) {
            group.active = pos;
        }
        self.focused = Some(id);
        true
    }

    pub(crate) fn group_of<Q>(&self, tab: &Q) -> Option<GroupId>
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.locate(tab).map(|(id, _)| id)
    }

    /// The group holding `tab`, and the tab's index in it.
    fn locate<Q>(&self, tab: &Q) -> Option<(GroupId, usize)>
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let mut found = None;
        if let Some(root) = &self.root {
            root.for_each_group(&mut |g| {
                if found.is_none()
                    && let Some(pos) = g.position(tab)
                {
                    found = Some((g.id, pos));
                }
            });
        }
        found
    }

    pub(crate) fn group_mut(&mut self, id: GroupId) -> Option<&mut TabGroup<T>> {
        self.root.as_mut().and_then(|root| root.group_mut(id))
    }

    pub(crate) fn group(&self, id: GroupId) -> Option<&TabGroup<T>> {
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
    pub(crate) fn move_tab<Q>(&mut self, tab: &Q, target: DropTarget) -> bool
    where
        T: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let Some((source, from)) = self.locate(tab) else {
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
                let Some(moved) = self.remove_tab(tab) else {
                    return false;
                };
                // The target group survives: it differs from the source.
                if let Some(g) = self.group_mut(group) {
                    let at = index.min(g.tabs.len());
                    g.tabs.insert(at, moved);
                    g.active = at;
                }
            }
            DropTarget::Edge { group, side } => {
                if source == group && target_len == 1 {
                    return false;
                }
                let Some(moved) = self.remove_tab(tab) else {
                    return false;
                };
                // The target group survives: either it differs from the
                // source, or it kept at least one other tab.
                let new_id = self.alloc_group();
                if let Some(node) = self.root.as_mut().and_then(|r| r.group_node_mut(group)) {
                    split_node(node, side, moved, new_id);
                }
                self.focused = Some(new_id);
                return true;
            }
        }
        self.focused = Some(target_group);
        true
    }

    /// Restore invariants after construction or deserialization: drop
    /// duplicate tabs and empty groups, collapse single-child splits, clamp
    /// `active` and split fractions, and keep group ids unique and ahead of
    /// the allocator.
    pub(crate) fn sanitize(&mut self) {
        let Some(mut root) = self.root.take() else {
            return;
        };
        let mut next = self.next_group;
        root.for_each_group(&mut |g| next = next.max(g.id + 1));

        // Duplicates only come from `from_node` or deserialization, so look
        // for one before cloning every tab to remove them.
        let mut has_duplicates = false;
        {
            let mut seen = HashSet::new();
            root.for_each_group(&mut |g| {
                has_duplicates |= g.tabs.iter().any(|t| !seen.insert(t));
            });
        }

        let mut seen_tabs = HashSet::new();
        let mut seen_ids = HashSet::new();
        root.for_each_group_mut(&mut |g| {
            if has_duplicates {
                g.tabs.retain(|t| seen_tabs.insert(t.clone()));
            }
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

impl<T> DockLayout<T> {
    /// Convert every tab with `f`, dropping the tabs it maps to `None`. The
    /// layout keeps its shape, as [`Self::retain`] does, and tabs that
    /// convert to the same value keep only the first.
    ///
    /// This migrates a saved layout whose tabs have changed form, such as
    /// names saved by an older version.
    #[cfg_attr(
        feature = "serde",
        doc = "To simply skip tabs that no longer deserialize, use [`Self::deserialize_lenient`]."
    )]
    ///
    /// ```
    /// use elegance::DockLayout;
    ///
    /// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    /// enum Tab {
    ///     Files,
    ///     Console,
    /// }
    ///
    /// // Saved by an older version that also had a "history" tab.
    /// let saved = DockLayout::<String>::single(["files", "history", "console"]);
    /// let layout = saved.filter_map(|name| match name.as_str() {
    ///     "files" => Some(Tab::Files),
    ///     "console" => Some(Tab::Console),
    ///     _ => None,
    /// });
    /// assert_eq!(layout.tabs(), [&Tab::Files, &Tab::Console]);
    /// ```
    pub fn filter_map<U: Clone + Eq + Hash>(
        self,
        mut f: impl FnMut(T) -> Option<U>,
    ) -> DockLayout<U> {
        let mut layout = DockLayout {
            root: self.root.map(|root| root.filter_map(&mut f)),
            next_group: self.next_group,
            focused: self.focused,
        };
        layout.sanitize();
        layout
    }
}

#[cfg(feature = "serde")]
impl<T: Clone + Eq + Hash> DockLayout<T> {
    /// Deserialize a layout, dropping tabs that don't deserialize as `T`
    /// instead of failing the whole layout. A saved layout can then name a
    /// tab the app has since removed. The layout keeps its shape, as
    /// [`Self::retain`] does. Errors in the layout's own structure still
    /// fail.
    ///
    /// Each tab is buffered before it is read, so this needs a
    /// self-describing format such as JSON or RON, not bincode.
    ///
    /// Use it on a field of persisted app state with
    /// `#[serde(deserialize_with = "DockLayout::deserialize_lenient")]`, or
    /// call it directly:
    ///
    /// ```
    /// use elegance::DockLayout;
    ///
    /// #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
    /// enum Tab {
    ///     Files,
    ///     Console,
    /// }
    ///
    /// // Saved by an older version that also had a `History` tab.
    /// let json = r#"{"root":{"Group":{"id":0,"tabs":["Files",{"History":3},"Console"],"active":0}},"next_group":1,"focused":0}"#;
    /// let mut de = serde_json::Deserializer::from_str(json);
    /// let layout = DockLayout::<Tab>::deserialize_lenient(&mut de)?;
    /// assert_eq!(layout.tabs(), [&Tab::Files, &Tab::Console]);
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn deserialize_lenient<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: serde::Deserialize<'de>,
    {
        let loose: DockLayout<MaybeTab<T>> = serde::Deserialize::deserialize(deserializer)?;
        Ok(loose.filter_map(|tab| match tab {
            MaybeTab::Known(tab) => Some(tab),
            MaybeTab::Unknown(_) => None,
        }))
    }
}

/// A saved tab that is `T` if it deserializes as one. Untagged enums buffer
/// the value first, so a failed `T` leaves the input intact for the fallback.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum MaybeTab<T> {
    Known(T),
    Unknown(serde::de::IgnoredAny),
}

impl<T> Node<T> {
    pub(crate) fn for_each_group<'a>(&'a self, f: &mut impl FnMut(&'a TabGroup<T>)) {
        match self {
            Self::Group(g) => f(g),
            Self::Split { first, second, .. } => {
                first.for_each_group(f);
                second.for_each_group(f);
            }
        }
    }

    fn for_each_group_mut(&mut self, f: &mut impl FnMut(&mut TabGroup<T>)) {
        match self {
            Self::Group(g) => f(g),
            Self::Split { first, second, .. } => {
                first.for_each_group_mut(f);
                second.for_each_group_mut(f);
            }
        }
    }

    /// Convert every tab, leaving empty groups for [`prune`]. A group whose
    /// active tab is dropped activates the next surviving tab.
    fn filter_map<U>(self, f: &mut impl FnMut(T) -> Option<U>) -> Node<U> {
        match self {
            Self::Group(g) => {
                let mut tabs = Vec::with_capacity(g.tabs.len());
                let mut active = 0;
                for (i, tab) in g.tabs.into_iter().enumerate() {
                    if i == g.active {
                        active = tabs.len();
                    }
                    tabs.extend(f(tab));
                }
                Node::Group(TabGroup {
                    id: g.id,
                    tabs,
                    active,
                })
            }
            Self::Split {
                direction,
                fraction,
                first,
                second,
            } => Node::Split {
                direction,
                fraction,
                first: Box::new(first.filter_map(f)),
                second: Box::new(second.filter_map(f)),
            },
        }
    }

    fn group(&self, id: GroupId) -> Option<&TabGroup<T>> {
        match self {
            Self::Group(g) => (g.id == id).then_some(g),
            Self::Split { first, second, .. } => first.group(id).or_else(|| second.group(id)),
        }
    }

    fn group_mut(&mut self, id: GroupId) -> Option<&mut TabGroup<T>> {
        match self {
            Self::Group(g) => (g.id == id).then_some(g),
            Self::Split { first, second, .. } => {
                first.group_mut(id).or_else(|| second.group_mut(id))
            }
        }
    }

    /// The leaf node holding group `id`.
    fn group_node_mut(&mut self, id: GroupId) -> Option<&mut Self> {
        if matches!(self, Self::Group(g) if g.id == id) {
            return Some(self);
        }
        match self {
            Self::Group(_) => None,
            Self::Split { first, second, .. } => first
                .group_node_mut(id)
                .or_else(|| second.group_node_mut(id)),
        }
    }
}

/// Drop empty groups and collapse splits left with one child, so the space
/// goes to the sibling. Also normalises split fractions into `[0, 1]`.
fn prune<T>(node: Node<T>) -> Option<Node<T>> {
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

/// Replace the leaf `node` with an even split of it and a new group `new_id`
/// holding `tab` on `side`.
fn split_node<T>(node: &mut Node<T>, side: Side, tab: T, new_id: GroupId) {
    let added = Node::Group(TabGroup {
        id: new_id,
        tabs: vec![tab],
        active: 0,
    });
    // Swap a placeholder in so the existing leaf can move into the split.
    let existing = std::mem::replace(
        node,
        Node::Group(TabGroup {
            id: new_id,
            tabs: Vec::new(),
            active: 0,
        }),
    );
    let (direction, first, second) = match side {
        Side::Left => (SplitDirection::Row, added, existing),
        Side::Right => (SplitDirection::Row, existing, added),
        Side::Top => (SplitDirection::Column, added, existing),
        Side::Bottom => (SplitDirection::Column, existing, added),
    };
    *node = Node::Split {
        direction,
        fraction: 0.5,
        first: Box::new(first),
        second: Box::new(second),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_pane() -> DockLayout<String> {
        DockLayout::<String>::from_node(DockNode::row(
            0.25,
            DockNode::tabs(["files"]),
            DockNode::column(
                0.7,
                DockNode::tabs(["editor", "preview"]),
                DockNode::tabs(["console"]),
            ),
        ))
    }

    #[test]
    fn group_tabs_lists_the_tabs_sharing_a_group() {
        let layout = three_pane();
        assert_eq!(layout.group_tabs("preview"), vec!["editor", "preview"]);
        assert_eq!(layout.group_tabs("files"), vec!["files"]);
        assert!(layout.group_tabs("missing").is_empty());
    }

    #[test]
    fn replace_tab_swaps_in_place() {
        let mut layout = three_pane();
        layout.activate("preview");
        assert_eq!(
            layout.replace_tab("preview", "diff").as_deref(),
            Some("preview")
        );
        assert_eq!(layout.group_tabs("diff"), vec!["editor", "diff"]);
        assert_eq!(layout.active_tabs(), vec!["files", "diff", "console"]);
        assert!(layout.replace_tab("missing", "x").is_none());
        assert!(!layout.contains("x"));
        assert_eq!(layout.replace_tab("diff", "diff").as_deref(), Some("diff"));
    }

    #[test]
    fn replace_tab_onto_a_present_tab_activates_it() {
        let mut layout = three_pane();
        assert_eq!(
            layout.replace_tab("console", "editor").as_deref(),
            Some("console")
        );
        assert!(!layout.contains("console"));
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.active_tabs(), vec!["files", "editor"]);
    }

    /// Tabs can be any `Clone + Eq + Hash` type, so they carry their own data.
    #[test]
    fn tabs_can_be_an_app_enum() {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        enum Tab {
            Log,
            Channel(u8),
        }
        let mut layout = DockLayout::from_node(DockNode::row(
            0.5,
            DockNode::tabs([Tab::Log]),
            DockNode::tabs([Tab::Channel(1), Tab::Channel(2)]),
        ));
        assert!(layout.contains(&Tab::Channel(2)));
        assert_eq!(
            layout.replace_tab(&Tab::Channel(1), Tab::Channel(7)),
            Some(Tab::Channel(1))
        );
        assert_eq!(
            layout.tabs(),
            vec![&Tab::Log, &Tab::Channel(7), &Tab::Channel(2)]
        );
        assert_eq!(layout.remove_tab(&Tab::Log), Some(Tab::Log));
        assert_eq!(layout.group_count(), 1);
    }

    #[test]
    fn from_node_builds_groups_in_order() {
        let layout = three_pane();
        assert_eq!(layout.group_count(), 3);
        assert_eq!(layout.tabs(), vec!["files", "editor", "preview", "console"]);
        assert_eq!(layout.active_tabs(), vec!["files", "editor", "console"]);
    }

    #[test]
    fn from_node_drops_duplicates_and_empty_groups() {
        let layout = DockLayout::<String>::from_node(DockNode::row(
            0.5,
            DockNode::tabs(["a", "b"]),
            DockNode::tabs(["b", "c"]),
        ));
        assert_eq!(layout.tabs(), vec!["a", "b", "c"]);

        let layout = DockLayout::<String>::from_node(DockNode::row(
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
            let layout = DockLayout::<String>::from_node(DockNode::row(
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
        assert_eq!(layout.remove_tab("console").as_deref(), Some("console"));
        assert_eq!(layout.group_count(), 2);
        // `editor`'s group now fills the old column.
        let Some(Node::Split { second, .. }) = &layout.root else {
            panic!("expected root split");
        };
        assert!(matches!(**second, Node::Group(_)));
        assert!(layout.remove_tab("console").is_none());
    }

    #[test]
    fn removing_the_only_tab_empties_the_layout() {
        let mut layout = DockLayout::<String>::single(["a"]);
        assert!(layout.remove_tab("a").is_some());
        assert!(layout.is_empty());
    }

    #[test]
    fn removing_a_tab_keeps_the_active_tab() {
        let mut layout = DockLayout::<String>::single(["a", "b", "c"]);
        layout.activate("c");
        layout.remove_tab("a");
        assert_eq!(layout.active_tabs(), vec!["c"]);
        layout.remove_tab("c");
        assert_eq!(layout.active_tabs(), vec!["b"]);
    }

    #[test]
    fn add_tab_goes_to_focused_group_and_activates() {
        let mut layout = three_pane();
        layout.activate("console");
        layout.add_tab("output");
        assert_eq!(layout.group_tabs("output"), vec!["console", "output"]);
        assert_eq!(layout.active_tabs(), vec!["files", "editor", "output"]);

        layout.add_tab("files");
        assert_eq!(layout.tabs().len(), 5, "existing id is not duplicated");
        assert_eq!(layout.active_tabs()[0], "files");
    }

    #[test]
    fn add_tab_to_empty_layout_creates_a_group() {
        let mut layout = DockLayout::<String>::new();
        layout.add_tab("a");
        assert_eq!(layout.tabs(), vec!["a"]);
    }

    #[test]
    fn move_within_group_reorders() {
        let mut layout = DockLayout::<String>::single(["a", "b", "c"]);
        let group = layout.group_of("a").unwrap();
        // Dropping `a` in the gap after `c`.
        assert!(layout.move_tab("a", DropTarget::Group { group, index: 3 }));
        assert_eq!(layout.group_tabs("a"), vec!["b", "c", "a"]);
        assert_eq!(layout.active_tabs(), vec!["a"]);

        // Dropping `c` back into the gap right after itself is a no-op.
        assert!(!layout.move_tab("c", DropTarget::Group { group, index: 2 }));
        assert!(!layout.move_tab("c", DropTarget::Group { group, index: 1 }));
        assert_eq!(layout.group_tabs("a"), vec!["b", "c", "a"]);
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
            layout.group_tabs("files"),
            vec!["editor", "files", "preview"]
        );
        // The emptied source group is gone.
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.active_tabs()[0], "files");
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
        assert_eq!(layout.group_tabs("preview"), vec!["preview"]);
        assert_eq!(layout.group_tabs("editor"), vec!["editor"]);
        // `files` above, `preview` below.
        let ids = layout.tabs();
        let pos = |t| ids.iter().position(|x| *x == t).unwrap();
        assert!(pos("files") < pos("preview"));
    }

    #[test]
    fn move_to_edge_of_own_group_splits_it() {
        let mut layout = DockLayout::<String>::single(["a", "b"]);
        let group = layout.group_of("a").unwrap();
        assert!(layout.move_tab(
            "b",
            DropTarget::Edge {
                group,
                side: Side::Left,
            }
        ));
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.tabs(), vec!["b", "a"]);
    }

    #[test]
    fn move_sole_tab_to_own_edge_is_a_no_op() {
        let mut layout = DockLayout::<String>::single(["a"]);
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
        let mut layout = DockLayout::<String>::single(["a", "b", "c"]);
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

    #[test]
    fn retain_drops_tabs_and_collapses_emptied_groups() {
        let mut layout = three_pane();
        layout.retain(|t| t != "editor" && t != "console");
        assert_eq!(layout.tabs(), vec!["files", "preview"]);
        assert_eq!(layout.group_count(), 2);
        assert_eq!(layout.active_tabs(), vec!["files", "preview"]);

        layout.retain(|_| false);
        assert!(layout.is_empty());
    }

    #[test]
    fn retain_keeps_the_active_tab_where_it_survives() {
        let mut layout = DockLayout::<String>::single(["a", "b", "c", "d"]);
        layout.activate("c");
        layout.retain(|t| t != "a");
        assert_eq!(layout.active_tabs(), vec!["c"]);
        layout.retain(|t| t != "c");
        assert_eq!(layout.active_tabs(), vec!["d"]);
        layout.retain(|t| t != "d");
        assert_eq!(layout.active_tabs(), vec!["b"]);
    }

    #[test]
    fn filter_map_converts_tabs_and_drops_duplicates() {
        let layout = three_pane().filter_map(|t| match t.as_str() {
            "files" => None,
            "editor" | "preview" => Some(1),
            _ => Some(2),
        });
        assert_eq!(layout.tabs(), vec![&1, &2]);
        assert_eq!(layout.group_count(), 2);
    }

    /// A layout saved with a tab variant the app no longer has loads with
    /// that tab dropped.
    #[cfg(feature = "serde")]
    #[test]
    fn lenient_load_drops_unknown_tabs() {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        enum Old {
            Files,
            History(u32),
            Console,
        }
        #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
        enum New {
            Files,
            Console,
        }
        let saved = DockLayout::<Old>::from_node(DockNode::row(
            0.3,
            DockNode::tabs([Old::Files, Old::History(3)]),
            DockNode::tabs([Old::Console]),
        ));
        let json = serde_json::to_string(&saved).unwrap();
        assert!(serde_json::from_str::<DockLayout<New>>(&json).is_err());

        let mut de = serde_json::Deserializer::from_str(&json);
        let layout = DockLayout::<New>::deserialize_lenient(&mut de).unwrap();
        assert_eq!(layout.tabs(), vec![&New::Files, &New::Console]);
        assert_eq!(layout.group_count(), 2);

        // The same, as a field of persisted app state.
        #[derive(serde::Deserialize)]
        struct State {
            #[serde(deserialize_with = "DockLayout::deserialize_lenient")]
            dock: DockLayout<New>,
        }
        let state: State = serde_json::from_str(&format!(r#"{{"dock":{json}}}"#)).unwrap();
        assert_eq!(state.dock, layout);

        // Structural errors are not skipped.
        let broken = json.replacen("\"next_group\"", "\"next_group\":\"x\",\"_\"", 1);
        let mut de = serde_json::Deserializer::from_str(&broken);
        assert!(DockLayout::<New>::deserialize_lenient(&mut de).is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let mut layout = three_pane();
        layout.activate("preview");
        let json = serde_json::to_string(&layout).unwrap();
        let back: DockLayout<String> = serde_json::from_str(&json).unwrap();
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
        let mut layout = DockLayout::<String> {
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
        assert_eq!(layout.group_tabs("a"), vec!["b", "a"]);
    }

    #[test]
    fn sanitize_repairs_a_hand_edited_layout() {
        let mut layout = DockLayout::<String> {
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
        assert_eq!(layout.tabs(), vec!["a"]);
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
