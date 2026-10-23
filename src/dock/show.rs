//! Layout and painting of a [`Dock`](super::Dock) for one frame.

use egui::{
    Align, CornerRadius, CursorIcon, Id, Key, LayerId, Layout, Order, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::drag::{self, DragState, GroupGeom};
use super::layout::{DockLayout, GroupId, Node, SplitDirection, TabGroup};
use super::{Dock, DockEvent, DockTab, DockViewer};
use crate::tab_paint::{self, TAB_GAP, TabContent, TabState};
use crate::theme::{Theme, with_alpha};

/// Space between two panes; the splitter's hit area.
const HANDLE: f32 = 6.0;
/// Thickness of the splitter line shown on hover or drag.
const HANDLE_LINE: f32 = 2.0;
const STRIP_PAD_X: f32 = 4.0;
pub(super) const STRIP_PAD_Y: f32 = 4.0;
const TAB_MIN_WIDTH: f32 = 72.0;
const TAB_MAX_WIDTH: f32 = 200.0;
/// Tabs never shrink below this when a strip overflows. It is wide enough
/// for a tab's padding, icon, dirty dot and close button without overlap.
const TAB_SHRUNK_WIDTH: f32 = 72.0;
const FALLBACK_SIZE: Vec2 = vec2(480.0, 320.0);

pub(super) fn show(
    ui: &mut Ui,
    base_id: Id,
    dock: &Dock,
    layout: &mut DockLayout,
    viewer: &mut dyn DockViewer,
) -> (Response, Vec<DockEvent>) {
    layout.sanitize();
    let theme = Theme::current(ui.ctx());

    let avail = ui.available_size();
    let size = vec2(
        if avail.x.is_finite() {
            avail.x
        } else {
            FALLBACK_SIZE.x
        },
        if avail.y.is_finite() {
            avail.y
        } else {
            FALLBACK_SIZE.y
        },
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "dock"));
    if !ui.is_rect_visible(rect) {
        return (response, Vec::new());
    }

    let drag_id = base_id.with("drag");
    let pass_nr = ui.ctx().cumulative_pass_nr();
    let mut drag: Option<DragState> = ui.ctx().data(|d| d.get_temp(drag_id));
    // A drag is refreshed every pass, so one that missed a pass (the dock was
    // hidden or not shown) is stale and must not commit a drop later.
    if let Some(d) = &drag
        && (d.pass + 1 < pass_nr
            || !layout.contains(&d.tab)
            || ui.input(|i| i.key_pressed(Key::Escape)))
    {
        drag = None;
    }

    ui.painter().rect_filled(rect, 0.0, theme.palette.bg);
    if layout.root.is_none() {
        // Give an empty dock the same bounds a pane would have.
        ui.painter().rect(
            rect,
            pane_radius(&theme),
            theme.palette.card,
            Stroke::new(1.0, theme.palette.border),
            StrokeKind::Inside,
        );
    }

    let Outcome {
        geoms,
        mut events,
        closes,
        new_drag,
        focus,
        mut changed,
    } = {
        let mut pass = Pass {
            ui: &mut *ui,
            theme: &theme,
            viewer: &mut *viewer,
            dock,
            base_id,
            dragging: drag.is_some(),
            geoms: Vec::new(),
            events: Vec::new(),
            closes: Vec::new(),
            new_drag: None,
            focus: None,
            changed: false,
        };
        match layout.root.as_mut() {
            Some(root) => pass.node(root, rect, 1),
            None => {
                let mut child = pass.ui.new_child(
                    UiBuilder::new()
                        .max_rect(rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                pass.viewer.empty_ui(&mut child);
            }
        }
        Outcome {
            geoms: pass.geoms,
            events: pass.events,
            closes: pass.closes,
            new_drag: pass.new_drag,
            focus: pass.focus,
            changed: pass.changed,
        }
    };

    if let Some(id) = focus {
        layout.set_focused(id);
    }
    for id in closes {
        if layout.remove_tab(&id) {
            events.push(DockEvent::TabClosed(id));
            changed = true;
        }
    }

    if drag.is_none() {
        drag = new_drag;
    }
    if let Some(state) = drag.take() {
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hit = pointer.and_then(|p| drag::hit_test(&geoms, p, &state));
        if ui.input(|i| i.pointer.primary_down()) {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            ui.ctx().request_repaint();
            let layer = LayerId::new(Order::Foreground, base_id.with("preview"));
            let painter = ui.ctx().layer_painter(layer).with_clip_rect(rect);
            if let Some(hit) = hit {
                drag::paint_preview(&painter, &theme, hit.preview);
            }
            if let Some(p) = pointer {
                paint_ghost(ui, base_id, &theme, viewer, &state, p);
            }
            drag = Some(state);
        } else if let Some(hit) = hit
            && layout.move_tab(&state.tab, hit.target)
        {
            events.push(DockEvent::TabMoved(state.tab));
            changed = true;
        }
    }
    ui.ctx().data_mut(|d| match drag {
        Some(state) => {
            d.insert_temp(
                drag_id,
                DragState {
                    pass: pass_nr,
                    ..state
                },
            );
        }
        None => {
            d.remove_temp::<DragState>(drag_id);
        }
    });

    if changed {
        events.push(DockEvent::LayoutChanged);
    }
    (response, events)
}

fn pane_radius(theme: &Theme) -> CornerRadius {
    CornerRadius::same(theme.control_radius as u8)
}

/// What a layout pass hands back to [`show`].
struct Outcome {
    geoms: Vec<GroupGeom>,
    events: Vec<DockEvent>,
    closes: Vec<String>,
    new_drag: Option<DragState>,
    focus: Option<GroupId>,
    changed: bool,
}

/// State threaded through the recursive layout pass.
struct Pass<'a> {
    ui: &'a mut Ui,
    theme: &'a Theme,
    viewer: &'a mut dyn DockViewer,
    dock: &'a Dock,
    base_id: Id,
    /// A tab drag was already in progress when the frame began.
    dragging: bool,
    geoms: Vec<GroupGeom>,
    events: Vec<DockEvent>,
    closes: Vec<String>,
    new_drag: Option<DragState>,
    focus: Option<GroupId>,
    changed: bool,
}

impl Pass<'_> {
    /// `path` identifies a node's position in the tree (a leading 1 followed
    /// by one bit per branch taken), which keeps splitter ids stable.
    fn node(&mut self, node: &mut Node, rect: Rect, path: u64) {
        match node {
            Node::Group(group) => self.group(group, rect),
            Node::Split {
                direction,
                fraction,
                first,
                second,
            } => {
                let geometry = split_geometry(rect, *direction, *fraction, self.dock.min_pane_size);
                let handle_id = self.base_id.with(("split", path));
                let handle = self.ui.interact(geometry.handle, handle_id, Sense::drag());

                if !self.dragging && (handle.hovered() || handle.dragged()) {
                    self.ui.ctx().set_cursor_icon(match direction {
                        SplitDirection::Row => CursorIcon::ResizeHorizontal,
                        SplitDirection::Column => CursorIcon::ResizeVertical,
                    });
                }
                let mut geometry = geometry;
                if handle.dragged()
                    && geometry.total > 0.0
                    && let Some(pointer) = handle.interact_pointer_pos()
                {
                    // Follow the pointer rather than accumulating deltas, so
                    // the handle stays under it after hitting a limit.
                    let (pos, origin) = match direction {
                        SplitDirection::Row => (pointer.x, rect.min.x),
                        SplitDirection::Column => (pointer.y, rect.min.y),
                    };
                    let first_len = (pos - origin - HANDLE * 0.5).clamp(geometry.lo, geometry.hi);
                    *fraction = first_len / geometry.total;
                    geometry = split_geometry(rect, *direction, *fraction, self.dock.min_pane_size);
                }
                if handle.double_clicked() {
                    *fraction = 0.5;
                    geometry = split_geometry(rect, *direction, *fraction, self.dock.min_pane_size);
                    self.changed = true;
                }
                if handle.drag_stopped() {
                    self.changed = true;
                }

                if handle.hovered() || handle.dragged() {
                    let p = &self.theme.palette;
                    let color = if handle.dragged() {
                        p.focus
                    } else {
                        with_alpha(p.focus, 150)
                    };
                    let line = match direction {
                        SplitDirection::Row => Rect::from_center_size(
                            geometry.handle.center(),
                            vec2(HANDLE_LINE, geometry.handle.height()),
                        ),
                        SplitDirection::Column => Rect::from_center_size(
                            geometry.handle.center(),
                            vec2(geometry.handle.width(), HANDLE_LINE),
                        ),
                    };
                    self.ui
                        .painter()
                        .rect_filled(line, CornerRadius::same(1), color);
                }
                handle.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "splitter"));

                self.node(first, geometry.first, path << 1);
                self.node(second, geometry.second, (path << 1) | 1);
            }
        }
    }

    fn group(&mut self, group: &mut TabGroup, rect: Rect) {
        let theme = self.theme;
        let p = &theme.palette;
        let tab_height = tab_paint::tab_height(theme);
        let strip = Rect::from_min_size(rect.min, vec2(rect.width(), STRIP_PAD_Y + tab_height));
        let body = Rect::from_min_max(pos2(rect.min.x, strip.max.y), rect.max);
        let painter = self.ui.painter_at(rect);

        let radius = pane_radius(theme);
        let strip_bg = tab_paint::strip_bg(theme);
        painter.rect_filled(
            body,
            CornerRadius {
                nw: 0,
                ne: 0,
                ..radius
            },
            p.card,
        );
        painter.rect_filled(
            strip,
            CornerRadius {
                sw: 0,
                se: 0,
                ..radius
            },
            strip_bg,
        );

        // Describe every tab, then fit them to the strip: natural widths when
        // there's room, otherwise shrunk proportionally.
        let tabs: Vec<_> = group.tabs.iter().map(|id| self.viewer.tab(id)).collect();
        let contents: Vec<TabContent<'_>> = tabs.iter().map(DockTab::content).collect();
        let natural: Vec<f32> = contents
            .iter()
            .map(|c| tab_paint::measure_tab(self.ui, theme, c, TAB_MIN_WIDTH, TAB_MAX_WIDTH).width)
            .collect();
        let gaps = TAB_GAP * natural.len().saturating_sub(1) as f32;
        let room = (strip.width() - 2.0 * STRIP_PAD_X - gaps).max(0.0);
        let total: f32 = natural.iter().sum();
        let scale = if total > room && total > 0.0 {
            room / total
        } else {
            1.0
        };

        let mut x = strip.min.x + STRIP_PAD_X;
        let tabs_top = strip.min.y + STRIP_PAD_Y;
        let mut tab_rects = Vec::with_capacity(tabs.len());
        let mut active_rect = None;
        let mut activate = None;

        for (i, (tab, content)) in tabs.iter().zip(&contents).enumerate() {
            let id = group.tabs[i].as_str();
            let width = (natural[i] * scale).max(TAB_SHRUNK_WIDTH);
            let tab_rect = Rect::from_min_size(pos2(x, tabs_top), vec2(width, tab_height));
            x += width + TAB_GAP;
            tab_rects.push(tab_rect);

            // Tabs pushed past the strip's edge stay unreachable.
            let visible = tab_rect.intersect(strip);
            // A tab that is mostly cut off is skipped rather than left as a
            // clipped sliver.
            if visible.width() < width * 0.5 {
                continue;
            }
            let measured = tab_paint::measure_tab(self.ui, theme, content, width, width);

            let resp = self.ui.interact(
                visible,
                self.base_id.with(("tab", id)),
                Sense::click_and_drag(),
            );
            let close_rect = tab_paint::close_rect(tab_rect);
            let close_resp = (tab.closable && strip.contains_rect(close_rect)).then(|| {
                self.ui
                    .interact(close_rect, self.base_id.with(("close", id)), Sense::click())
            });
            let close_hovered = close_resp.as_ref().is_some_and(Response::hovered);

            let is_active = group.active == i;
            let state = TabState {
                active: is_active,
                hovered: !self.dragging && (resp.hovered() || close_hovered),
                close_hovered: !self.dragging && close_hovered,
                close_enabled: close_resp.is_some(),
                strip_bg,
            };
            tab_paint::paint_tab(&painter, theme, tab_rect, content, measured, state);
            if is_active {
                active_rect = Some(visible);
            }

            let label = tab.label.clone();
            resp.widget_info(move || WidgetInfo::selected(Role::Button, true, is_active, &label));
            if let Some(close) = &close_resp {
                let label = format!("Close {}", tab.label);
                close.widget_info(move || WidgetInfo::labeled(Role::Button, true, &label));
            }

            if close_resp.as_ref().is_some_and(Response::clicked) {
                self.closes.push(id.to_owned());
            } else if resp.drag_started() {
                activate = Some(i);
                let grab = resp.interact_pointer_pos().unwrap_or(tab_rect.center());
                self.new_drag = Some(DragState {
                    tab: id.to_owned(),
                    source: group.id,
                    grab_offset: grab - tab_rect.min,
                    tab_width: width,
                    ..DragState::default()
                });
            } else if resp.clicked() {
                activate = Some(i);
            }
        }

        if let Some(i) = activate {
            if group.active != i {
                group.active = i;
                self.events
                    .push(DockEvent::TabActivated(group.tabs[i].clone()));
                self.changed = true;
            }
            self.focus = Some(group.id);
        }
        tab_paint::paint_strip_border(&painter, theme, strip, active_rect);

        if let Some(id) = group.active_tab() {
            let inner = body.shrink(self.dock.body_padding);
            let mut child = self.ui.new_child(
                UiBuilder::new()
                    .id_salt((self.base_id, "body", id))
                    .max_rect(inner)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(body.intersect(self.ui.clip_rect()));
            self.viewer.ui(&mut child, id);
        }

        let pressed_inside = self.ui.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
        });
        if pressed_inside {
            self.focus = Some(group.id);
        }

        painter.rect_stroke(rect, radius, Stroke::new(1.0, p.border), StrokeKind::Inside);

        self.geoms.push(GroupGeom {
            id: group.id,
            rect,
            strip,
            body,
            tabs: tab_rects,
        });
    }
}

/// Rects for the two halves of a split and the gutter between them.
struct SplitGeometry {
    first: Rect,
    handle: Rect,
    second: Rect,
    /// Length of the split axis available to the two panes.
    total: f32,
    /// Bounds on `first_len` that keep both panes above the minimum size.
    lo: f32,
    hi: f32,
}

fn split_geometry(
    rect: Rect,
    direction: SplitDirection,
    fraction: f32,
    min_pane_size: f32,
) -> SplitGeometry {
    let axis_len = match direction {
        SplitDirection::Row => rect.width(),
        SplitDirection::Column => rect.height(),
    };
    let total = (axis_len - HANDLE).max(0.0);
    let lo = min_pane_size.min(total * 0.5);
    let hi = total - lo;
    let first_len = (total * fraction).clamp(lo, hi);

    let (first, handle, second) = match direction {
        SplitDirection::Row => {
            let handle_x = rect.min.x + first_len;
            (
                Rect::from_min_max(rect.min, pos2(handle_x, rect.max.y)),
                Rect::from_min_max(
                    pos2(handle_x, rect.min.y),
                    pos2(handle_x + HANDLE, rect.max.y),
                ),
                Rect::from_min_max(pos2(handle_x + HANDLE, rect.min.y), rect.max),
            )
        }
        SplitDirection::Column => {
            let handle_y = rect.min.y + first_len;
            (
                Rect::from_min_max(rect.min, pos2(rect.max.x, handle_y)),
                Rect::from_min_max(
                    pos2(rect.min.x, handle_y),
                    pos2(rect.max.x, handle_y + HANDLE),
                ),
                Rect::from_min_max(pos2(rect.min.x, handle_y + HANDLE), rect.max),
            )
        }
    };
    SplitGeometry {
        first,
        handle,
        second,
        total,
        lo,
        hi,
    }
}

/// Paint the dragged tab following the pointer.
fn paint_ghost(
    ui: &Ui,
    base_id: Id,
    theme: &Theme,
    viewer: &mut dyn DockViewer,
    drag: &DragState,
    pointer: egui::Pos2,
) {
    let tab = viewer.tab(&drag.tab);
    let content = TabContent {
        closable: false,
        ..tab.content()
    };
    let measured = tab_paint::measure_tab(ui, theme, &content, drag.tab_width, drag.tab_width);
    let rect = Rect::from_min_size(
        pointer - drag.grab_offset,
        vec2(drag.tab_width, tab_paint::tab_height(theme)),
    );
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, base_id.with("ghost")));
    let state = TabState {
        active: true,
        hovered: false,
        close_hovered: false,
        close_enabled: false,
        strip_bg: tab_paint::strip_bg(theme),
    };
    tab_paint::paint_ghost_cell(&painter, theme, rect);
    tab_paint::paint_content(&painter, theme, rect, &content, measured, state);
}
