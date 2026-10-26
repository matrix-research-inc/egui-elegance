//! Layout and painting of a [`Dock`](super::Dock) for one frame.

use std::hash::Hash;

use egui::{
    Align, CornerRadius, CursorIcon, Frame, Id, Key, LayerId, Layout, Order, Rect, Response, Role,
    ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::drag::{self, DragState, GroupGeom};
use super::layout::{DockLayout, GroupId, Node, SplitDirection, TabGroup};
use super::{Dock, DockEvent, DockTab, DockViewer};
use crate::tab_paint::{self, TAB_GAP, TabContent, TabState};
use crate::theme::{Theme, with_alpha};
use crate::{ContextMenu, Menu, MenuItem};

/// Space between two panes; the splitter's hit area.
const HANDLE: f32 = 6.0;
/// Thickness of the splitter line shown on hover or drag.
const HANDLE_LINE: f32 = 2.0;
const STRIP_PAD_X: f32 = 4.0;
pub(super) const STRIP_PAD_Y: f32 = 4.0;
/// Tabs with short labels are padded out to this width.
const SHORT_TAB_WIDTH: f32 = 72.0;
/// Width of the button that opens a menu of every tab in an overflowing strip.
const OVERFLOW_BUTTON_WIDTH: f32 = 24.0;
const OVERFLOW_BUTTON_RADIUS: u8 = 4;
/// Distance from either end of an overflowing strip within which a dragged
/// tab scrolls it, and how fast, in points per second.
const STRIP_AUTOSCROLL_EDGE: f32 = 32.0;
const STRIP_AUTOSCROLL_SPEED: f32 = 600.0;
/// Width of the fade at an end of an overflowing strip with tabs hidden past it.
const STRIP_FADE_WIDTH: f32 = 24.0;
const FALLBACK_SIZE: Vec2 = vec2(480.0, 320.0);

pub(super) fn show<T: Clone + Eq + Hash>(
    ui: &mut Ui,
    base_id: Id,
    dock: &Dock,
    layout: &mut DockLayout<T>,
    viewer: &mut dyn DockViewer<Tab = T>,
) -> (Response, Vec<DockEvent<T>>) {
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
            || find_tab(layout, d.tab).is_none()
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
    for tab in closes {
        if let Some(removed) = layout.remove_tab(&tab) {
            events.push(DockEvent::TabClosed(removed));
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
            if let Some(p) = pointer
                && let Some(tab) = find_tab(layout, state.tab)
            {
                paint_ghost(ui, base_id, &theme, &viewer.tab(&tab), &state, p);
            }
            drag = Some(state);
        } else if let Some(hit) = hit
            && let Some(tab) = find_tab(layout, state.tab)
            && layout.move_tab(&tab, hit.target)
        {
            events.push(DockEvent::TabMoved(tab));
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

/// The tab whose [`drag::tab_key`] is `key`.
fn find_tab<T: Clone + Eq + Hash>(layout: &DockLayout<T>, key: u64) -> Option<T> {
    layout
        .tabs()
        .into_iter()
        .find(|tab| drag::tab_key(*tab) == key)
        .cloned()
}

fn pane_radius(theme: &Theme) -> CornerRadius {
    CornerRadius::same(theme.control_radius as u8)
}

/// What a layout pass hands back to [`show`].
struct Outcome<T> {
    geoms: Vec<GroupGeom>,
    events: Vec<DockEvent<T>>,
    closes: Vec<T>,
    new_drag: Option<DragState>,
    focus: Option<GroupId>,
    changed: bool,
}

/// State threaded through the recursive layout pass.
struct Pass<'a, T> {
    ui: &'a mut Ui,
    theme: &'a Theme,
    viewer: &'a mut dyn DockViewer<Tab = T>,
    dock: &'a Dock,
    base_id: Id,
    /// A tab drag was already in progress when the frame began.
    dragging: bool,
    geoms: Vec<GroupGeom>,
    events: Vec<DockEvent<T>>,
    closes: Vec<T>,
    new_drag: Option<DragState>,
    focus: Option<GroupId>,
    changed: bool,
}

impl<T: Clone + Eq + Hash> Pass<'_, T> {
    /// `path` identifies a node's position in the tree (a leading 1 followed
    /// by one bit per branch taken), which keeps splitter ids stable.
    fn node(&mut self, node: &mut Node<T>, rect: Rect, path: u64) {
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

    fn group(&mut self, group: &mut TabGroup<T>, rect: Rect) {
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
        // there's room, otherwise shrunk toward a floor. Past that, the strip
        // scrolls and an overflow menu lists every tab.
        let tabs: Vec<DockTab> = group.tabs.iter().map(|t| self.viewer.tab(t)).collect();
        let contents: Vec<TabContent<'_>> = tabs.iter().map(DockTab::content).collect();
        let (min_width, max_width) = (self.dock.min_tab_width, self.dock.max_tab_width);
        let short_width = SHORT_TAB_WIDTH.min(max_width);
        let natural: Vec<f32> = contents
            .iter()
            .map(|c| tab_paint::measure_tab(self.ui, theme, c, short_width, max_width).width)
            .collect();
        let gaps = TAB_GAP * natural.len().saturating_sub(1) as f32;
        let tabs_top = strip.min.y + STRIP_PAD_Y;
        let row = Rect::from_min_max(
            pos2(strip.min.x + STRIP_PAD_X, tabs_top),
            pos2(strip.max.x - STRIP_PAD_X, tabs_top + tab_height),
        );
        let widths = fit_widths(&natural, min_width, (row.width() - gaps).max(0.0));
        let content_width = widths.iter().sum::<f32>() + gaps;
        let overflow_button = (content_width > row.width() + 0.5).then(|| {
            Rect::from_min_max(pos2(row.max.x - OVERFLOW_BUTTON_WIDTH, row.min.y), row.max)
        });
        let tab_clip = match overflow_button {
            // A pane narrower than the button keeps an empty, not inverted, clip.
            Some(button) => Rect::from_min_max(
                row.min,
                pos2((button.min.x - TAB_GAP).max(row.min.x), row.max.y),
            ),
            None => row,
        };
        let offset = self.strip_offset(
            group,
            strip,
            tab_clip,
            &widths,
            overflow_button.is_some().then_some(content_width),
        );

        let tab_painter = painter.with_clip_rect(tab_clip.intersect(painter.clip_rect()));
        let mut x = tab_clip.min.x - offset;
        let mut tab_rects = Vec::with_capacity(tabs.len());
        let mut active_rect = None;
        let mut activate = None;

        for (i, (tab, content)) in tabs.iter().zip(&contents).enumerate() {
            let id = &group.tabs[i];
            let key = drag::tab_key(id);
            let width = widths[i];
            let tab_rect = Rect::from_min_size(pos2(x, tabs_top), vec2(width, tab_height));
            x += width + TAB_GAP;
            tab_rects.push(tab_rect);

            // Tabs scrolled fully out of view get no widget; the overflow
            // menu reaches them.
            let visible = tab_rect.intersect(tab_clip);
            if !visible.is_positive() {
                continue;
            }
            let measured = tab_paint::measure_tab(self.ui, theme, content, width, width);
            let truncated = measured.label.elided;

            let mut resp = self.ui.interact(
                visible,
                self.base_id.with(("tab", key)),
                Sense::click_and_drag(),
            );
            let close_rect = tab_paint::close_rect(tab_rect);
            let close_resp = (tab.closable && tab_clip.contains_rect(close_rect)).then(|| {
                self.ui.interact(
                    close_rect,
                    self.base_id.with(("close", key)),
                    Sense::click(),
                )
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
            tab_paint::paint_tab(&tab_painter, theme, tab_rect, content, measured, state);
            if is_active {
                active_rect = Some(visible);
            }

            let label = tab.label.clone();
            resp.widget_info(move || WidgetInfo::selected(Role::Button, true, is_active, &label));
            if let Some(close) = &close_resp {
                let label = format!("Close {}", tab.label);
                close.widget_info(move || WidgetInfo::labeled(Role::Button, true, &label));
            }
            if truncated && !self.dragging {
                resp = resp.on_hover_text(&tab.label);
            }

            if tab.context_menu {
                let viewer = &mut *self.viewer;
                ContextMenu::new("tab").show(&resp, |ui| viewer.context_menu(ui, id));
            }

            if close_resp.as_ref().is_some_and(Response::clicked) {
                self.closes.push(id.clone());
            } else if resp.drag_started() {
                activate = Some(i);
                let grab = resp.interact_pointer_pos().unwrap_or(tab_rect.center());
                self.new_drag = Some(DragState {
                    tab: key,
                    source: group.id,
                    grab_offset: grab - tab_rect.min,
                    tab_width: width,
                    pass: 0,
                });
            } else if resp.clicked() {
                activate = Some(i);
            }
        }

        if overflow_button.is_some() {
            let hidden_right = offset + tab_clip.width() < content_width - 0.5;
            paint_strip_fades(&tab_painter, tab_clip, strip_bg, offset > 0.5, hidden_right);
        }
        if let Some(button) = overflow_button
            && let Some(i) = self.overflow_menu(group, &tabs, button, strip_bg)
        {
            activate = Some(i);
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
            let scroll = tabs[group.active].scroll;
            let padding = self.dock.body_padding;
            // A scrolling body spans the pane, inside its border, so the
            // scroll bars sit at the edge and the padding scrolls with the
            // content.
            let content_rect = if scroll.any() {
                body.shrink(1.0)
            } else {
                body.shrink(padding)
            };
            let mut child = self.ui.new_child(
                UiBuilder::new()
                    .id_salt((self.base_id, "body", drag::tab_key(id)))
                    .max_rect(content_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(body.intersect(self.ui.clip_rect()));
            if scroll.any() {
                let viewer = &mut *self.viewer;
                ScrollArea::new(scroll)
                    .auto_shrink(false)
                    .show(&mut child, |ui| {
                        Frame::NONE
                            .inner_margin((padding - 1.0).max(0.0))
                            .show(ui, |ui| viewer.ui(ui, id));
                    });
            } else {
                self.viewer.ui(&mut child, id);
            }
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
            tab_clip,
        });
    }

    /// Update and return the horizontal scroll offset of a group's tab strip.
    /// `overflow_width` is the width of all its tabs when they don't fit.
    ///
    /// The wheel scrolls an overflowing strip, a dragged tab held near either
    /// end scrolls it, and a newly active tab, or the active tab of a strip
    /// that was resized, is scrolled into view.
    fn strip_offset(
        &mut self,
        group: &TabGroup<T>,
        strip: Rect,
        tab_clip: Rect,
        widths: &[f32],
        overflow_width: Option<f32>,
    ) -> f32 {
        let memory_id = self.base_id.with(("strip_scroll", group.id));
        let Some(overflow_width) = overflow_width else {
            // Forget the state so the active tab is revealed when the strip
            // next overflows.
            self.ui
                .ctx()
                .data_mut(|d| d.remove_temp::<StripScroll>(memory_id));
            return 0.0;
        };
        let mut scroll: StripScroll = self
            .ui
            .ctx()
            .data(|d| d.get_temp(memory_id))
            .unwrap_or_default();
        let view_width = tab_clip.width();

        if !self.dragging && self.ui.rect_contains_pointer(strip) {
            // A vertical wheel scrolls the strip too, as most mice have no
            // horizontal wheel.
            let delta = self.ui.input(|i| i.smooth_scroll_delta);
            if delta != Vec2::ZERO {
                scroll.offset -= delta.x + delta.y;
                self.ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        if self.dragging
            && let Some(pointer) = self.ui.input(|i| i.pointer.interact_pos())
            && strip.contains(pointer)
        {
            let step = STRIP_AUTOSCROLL_SPEED * self.ui.input(|i| i.stable_dt).min(0.1);
            if pointer.x < tab_clip.min.x + STRIP_AUTOSCROLL_EDGE {
                scroll.offset -= step;
            } else if pointer.x > tab_clip.max.x - STRIP_AUTOSCROLL_EDGE {
                scroll.offset += step;
            }
        }

        let active = group.active_tab().map(drag::tab_key);
        let settled = scroll
            .revealed
            .is_some_and(|(tab, width)| Some(tab) == active && (width - view_width).abs() < 0.5);
        if !settled {
            if let Some(&width) = widths.get(group.active) {
                let left =
                    widths[..group.active].iter().sum::<f32>() + TAB_GAP * group.active as f32;
                // Keep the tab clear of the edge fades, which the clamp
                // below removes at either end of the strip.
                let margin = STRIP_FADE_WIDTH.min(((view_width - width) / 2.0).max(0.0));
                if left - margin < scroll.offset {
                    scroll.offset = left - margin;
                } else if left + width + margin > scroll.offset + view_width {
                    scroll.offset = left + width + margin - view_width;
                }
            }
            scroll.revealed = active.map(|tab| (tab, view_width));
        }

        scroll.offset = scroll
            .offset
            .clamp(0.0, (overflow_width - view_width).max(0.0));
        self.ui.ctx().data_mut(|d| d.insert_temp(memory_id, scroll));
        scroll.offset
    }

    /// Draw the overflow button at the end of a strip and its menu of every
    /// tab. Returns the index of a tab picked from the menu.
    fn overflow_menu(
        &mut self,
        group: &TabGroup<T>,
        tabs: &[DockTab],
        button: Rect,
        strip_bg: egui::Color32,
    ) -> Option<usize> {
        let p = &self.theme.palette;
        let resp = self.ui.interact(
            button,
            self.base_id.with(("overflow", group.id)),
            Sense::click(),
        );
        resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, "All tabs"));
        let hovered = !self.dragging && resp.hovered();
        if hovered {
            self.ui.painter().rect_filled(
                button.shrink(2.0),
                CornerRadius::same(OVERFLOW_BUTTON_RADIUS),
                p.depth_tint(strip_bg, 0.10),
            );
        }
        paint_chevron_down(
            self.ui.painter(),
            button.center(),
            if hovered { p.text } else { p.text_muted },
        );

        let mut picked = None;
        Menu::new("tabs").show_below(&resp, |ui| {
            for (i, tab) in tabs.iter().enumerate() {
                let item = MenuItem::new(tab.label.as_str()).radio(i == group.active);
                if ui.add(item).clicked() {
                    picked = Some(i);
                }
            }
        });
        picked
    }
}

/// Horizontal scroll of a group's tab strip, kept in egui memory.
#[derive(Clone, Copy, Debug, Default)]
struct StripScroll {
    offset: f32,
    /// The active tab and the visible strip width when that tab was last
    /// scrolled into view.
    revealed: Option<(u64, f32)>,
}

/// Fit tabs into `room`: each shrinks in proportion to its natural width,
/// but none below `floor` (nor grows to reach it). The widths add up to more
/// than `room` only when the floors alone do.
fn fit_widths(natural: &[f32], floor: f32, room: f32) -> Vec<f32> {
    let total: f32 = natural.iter().sum();
    if total <= room {
        return natural.to_vec();
    }
    let floor_of = |w: f32| w.min(floor);
    // Pin tabs at their floor until the rest can share what's left.
    let mut pinned = vec![false; natural.len()];
    loop {
        let fixed: f32 = (0..natural.len())
            .filter(|&i| pinned[i])
            .map(|i| floor_of(natural[i]))
            .sum();
        let free: f32 = (0..natural.len())
            .filter(|&i| !pinned[i])
            .map(|i| natural[i])
            .sum();
        let scale = if free > 0.0 {
            ((room - fixed) / free).max(0.0)
        } else {
            0.0
        };
        let mut newly_pinned = false;
        for (i, &w) in natural.iter().enumerate() {
            if !pinned[i] && w * scale < floor_of(w) {
                pinned[i] = true;
                newly_pinned = true;
            }
        }
        if !newly_pinned {
            return natural
                .iter()
                .zip(&pinned)
                .map(|(&w, &pin)| if pin { floor_of(w) } else { w * scale })
                .collect();
        }
    }
}

/// Fade the tabs into the strip background at either end of `clip` with
/// tabs scrolled out of view past it.
fn paint_strip_fades(
    painter: &egui::Painter,
    clip: Rect,
    strip_bg: egui::Color32,
    left: bool,
    right: bool,
) {
    let width = STRIP_FADE_WIDTH.min(clip.width() / 2.0);
    let mut mesh = egui::Mesh::default();
    let mut fade = |outer: f32, inner: f32| {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(pos2(outer, clip.min.y), strip_bg);
        mesh.colored_vertex(pos2(outer, clip.max.y), strip_bg);
        mesh.colored_vertex(pos2(inner, clip.min.y), egui::Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(inner, clip.max.y), egui::Color32::TRANSPARENT);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 2, base + 3);
    };
    if left {
        fade(clip.min.x, clip.min.x + width);
    }
    if right {
        fade(clip.max.x, clip.max.x - width);
    }
    if !mesh.is_empty() {
        painter.add(mesh);
    }
}

/// Paint a small downward chevron centred on `center`.
fn paint_chevron_down(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let (half_w, half_h) = (4.0, 2.0);
    painter.add(egui::Shape::line(
        vec![
            pos2(center.x - half_w, center.y - half_h),
            pos2(center.x, center.y + half_h),
            pos2(center.x + half_w, center.y - half_h),
        ],
        Stroke::new(1.4, color),
    ));
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
    tab: &DockTab,
    drag: &DragState,
    pointer: egui::Pos2,
) {
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

#[cfg(test)]
mod tests {
    use super::fit_widths;

    #[test]
    fn tabs_that_fit_keep_their_natural_width() {
        assert_eq!(fit_widths(&[100.0, 80.0], 72.0, 200.0), [100.0, 80.0]);
    }

    #[test]
    fn wide_tabs_absorb_the_shrink_once_narrow_ones_reach_the_floor() {
        // Proportional scaling would put the 80pt tab at 64pt; pinning it at
        // the floor takes the rest from the wide tab, so the row still fits.
        let widths = fit_widths(&[200.0, 80.0], 72.0, 224.0);
        assert_eq!(widths, [152.0, 72.0]);
    }

    #[test]
    fn tabs_overflow_only_when_the_floors_do() {
        assert_eq!(fit_widths(&[100.0, 100.0, 100.0], 72.0, 150.0), [72.0; 3]);
    }

    #[test]
    fn tabs_narrower_than_the_floor_never_grow() {
        assert_eq!(fit_widths(&[40.0, 200.0], 72.0, 150.0), [40.0, 110.0]);
    }
}
