//! Tab drag-and-drop: hit-testing the pointer against the laid-out groups and
//! painting the drop preview.

use egui::{CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2};

use super::layout::{DropTarget, GroupId, Side};
use super::show::STRIP_PAD_Y;
use crate::theme::{Theme, with_alpha};

/// A drag in progress, kept in egui memory across frames.
#[derive(Clone, Debug, Default)]
pub(crate) struct DragState {
    pub tab: String,
    pub source: GroupId,
    /// Pointer position relative to the dragged tab's top-left at drag start.
    pub grab_offset: Vec2,
    pub tab_width: f32,
    /// Egui pass this state was last refreshed in; see `show`.
    pub pass: u64,
}

/// Where a group was laid out this frame.
#[derive(Clone, Debug)]
pub(crate) struct GroupGeom {
    pub id: GroupId,
    pub rect: Rect,
    pub strip: Rect,
    pub body: Rect,
    /// Every tab's rect in order, including tabs clipped by overflow.
    pub tabs: Vec<Rect>,
}

/// The visual cue for a drop target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Preview {
    /// A thin insertion bar between tabs.
    Marker(Rect),
    /// A tinted region showing where the tab will land.
    Region(Rect),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    pub target: DropTarget,
    pub preview: Preview,
}

/// Share of a body, per axis, near each edge that counts as an edge drop.
const EDGE_ZONE: f32 = 0.25;
const MARKER_WIDTH: f32 = 2.0;

/// Resolve what a tab dragged to `pos` would do, or `None` if dropping there
/// would be meaningless (outside every group, or splitting a lone tab off its
/// own group).
pub(crate) fn hit_test(geoms: &[GroupGeom], pos: Pos2, drag: &DragState) -> Option<Hit> {
    let geom = geoms.iter().find(|g| g.rect.contains(pos))?;

    if geom.strip.contains(pos) {
        let index = geom.tabs.iter().filter(|r| r.center().x < pos.x).count();
        return Some(Hit {
            target: DropTarget::Group {
                group: geom.id,
                index,
            },
            preview: Preview::Marker(marker_rect(geom, index)),
        });
    }

    let body = geom.body;
    let rel = pos2(
        (pos.x - body.min.x) / body.width().max(1.0),
        (pos.y - body.min.y) / body.height().max(1.0),
    );
    let edges = [
        (rel.x, Side::Left),
        (1.0 - rel.x, Side::Right),
        (rel.y, Side::Top),
        (1.0 - rel.y, Side::Bottom),
    ];
    let (distance, side) = edges
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("four candidates");

    if distance < EDGE_ZONE {
        let lone_tab_in_own_group = geom.id == drag.source && geom.tabs.len() == 1;
        if lone_tab_in_own_group {
            return None;
        }
        return Some(Hit {
            target: DropTarget::Edge {
                group: geom.id,
                side,
            },
            preview: Preview::Region(edge_region(geom.body, side)),
        });
    }

    // Dropping a tab back into the middle of its own group changes nothing.
    if geom.id == drag.source {
        return None;
    }
    Some(Hit {
        target: DropTarget::Group {
            group: geom.id,
            index: geom.tabs.len(),
        },
        preview: Preview::Region(body),
    })
}

fn marker_rect(geom: &GroupGeom, index: usize) -> Rect {
    let x = match (
        index.checked_sub(1).and_then(|i| geom.tabs.get(i)),
        geom.tabs.get(index),
    ) {
        (Some(prev), Some(next)) => (prev.max.x + next.min.x) * 0.5,
        (None, Some(next)) => next.min.x - 1.0,
        (Some(prev), None) => prev.max.x + 1.0,
        (None, None) => geom.strip.min.x + MARKER_WIDTH,
    };
    let x = x.clamp(geom.strip.min.x, geom.strip.max.x - MARKER_WIDTH);
    Rect::from_min_max(
        pos2(x - MARKER_WIDTH * 0.5, geom.strip.min.y + STRIP_PAD_Y),
        pos2(x + MARKER_WIDTH * 0.5, geom.strip.max.y - 1.0),
    )
}

fn edge_region(rect: Rect, side: Side) -> Rect {
    let c = rect.center();
    match side {
        Side::Left => Rect::from_min_max(rect.min, pos2(c.x, rect.max.y)),
        Side::Right => Rect::from_min_max(pos2(c.x, rect.min.y), rect.max),
        Side::Top => Rect::from_min_max(rect.min, pos2(rect.max.x, c.y)),
        Side::Bottom => Rect::from_min_max(pos2(rect.min.x, c.y), rect.max),
    }
}

/// Paint the drop cue for `preview`.
pub(crate) fn paint_preview(painter: &Painter, theme: &Theme, preview: Preview) {
    let p = &theme.palette;
    match preview {
        Preview::Marker(rect) => {
            painter.rect_filled(rect, CornerRadius::same(1), p.focus);
        }
        Preview::Region(rect) => {
            let rect = rect.shrink(2.0);
            painter.rect(
                rect,
                CornerRadius::same(2),
                with_alpha(p.focus, 22),
                Stroke::new(1.0, with_alpha(p.focus, 200)),
                StrokeKind::Inside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    fn geom(id: GroupId, origin: Pos2, tab_count: usize) -> GroupGeom {
        let rect = Rect::from_min_size(origin, vec2(300.0, 200.0));
        let strip = Rect::from_min_size(origin, vec2(300.0, 32.0));
        let body = Rect::from_min_max(pos2(origin.x, origin.y + 32.0), rect.max);
        let tabs = (0..tab_count)
            .map(|i| {
                Rect::from_min_size(
                    pos2(origin.x + 4.0 + i as f32 * 82.0, origin.y + 4.0),
                    vec2(80.0, 28.0),
                )
            })
            .collect();
        GroupGeom {
            id,
            rect,
            strip,
            body,
            tabs,
        }
    }

    fn drag(tab: &str, source: GroupId) -> DragState {
        DragState {
            tab: tab.into(),
            source,
            grab_offset: Vec2::ZERO,
            tab_width: 80.0,
            pass: 0,
        }
    }

    #[test]
    fn strip_gives_an_insertion_index() {
        let geoms = [geom(0, pos2(0.0, 0.0), 3)];
        let d = drag("x", 9);
        let at = |x| hit_test(&geoms, pos2(x, 16.0), &d).unwrap().target;
        assert_eq!(at(10.0), DropTarget::Group { group: 0, index: 0 });
        assert_eq!(at(60.0), DropTarget::Group { group: 0, index: 1 });
        assert_eq!(at(290.0), DropTarget::Group { group: 0, index: 3 });
    }

    #[test]
    fn body_center_appends_and_edges_split() {
        let geoms = [geom(0, pos2(0.0, 0.0), 2)];
        let d = drag("x", 9);
        let at = |x, y| hit_test(&geoms, pos2(x, y), &d).unwrap().target;
        assert_eq!(at(150.0, 120.0), DropTarget::Group { group: 0, index: 2 });
        assert_eq!(
            at(10.0, 120.0),
            DropTarget::Edge {
                group: 0,
                side: Side::Left
            }
        );
        assert_eq!(
            at(290.0, 120.0),
            DropTarget::Edge {
                group: 0,
                side: Side::Right
            }
        );
        assert_eq!(
            at(150.0, 40.0),
            DropTarget::Edge {
                group: 0,
                side: Side::Top
            }
        );
        assert_eq!(
            at(150.0, 195.0),
            DropTarget::Edge {
                group: 0,
                side: Side::Bottom
            }
        );
    }

    #[test]
    fn lone_tab_cannot_split_its_own_group() {
        let geoms = [geom(0, pos2(0.0, 0.0), 1)];
        assert!(hit_test(&geoms, pos2(10.0, 120.0), &drag("x", 0)).is_none());
        assert!(hit_test(&geoms, pos2(10.0, 120.0), &drag("x", 5)).is_some());
    }

    #[test]
    fn own_body_centre_is_no_target() {
        let geoms = [geom(0, pos2(0.0, 0.0), 3)];
        assert!(hit_test(&geoms, pos2(150.0, 120.0), &drag("x", 0)).is_none());
        assert!(hit_test(&geoms, pos2(150.0, 120.0), &drag("x", 5)).is_some());
    }

    #[test]
    fn outside_every_group_is_no_target() {
        let geoms = [geom(0, pos2(0.0, 0.0), 1)];
        assert!(hit_test(&geoms, pos2(500.0, 500.0), &drag("x", 5)).is_none());
    }
}
