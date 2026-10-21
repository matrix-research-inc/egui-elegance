//! Drag-and-drop sortable list — reorder rows by dragging their grip.
//!
//! See [`SortableList`] for the full interaction model and an example.

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Pos2, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
};

use crate::badge::BadgeTone;
use crate::theme::{Palette, Theme, with_alpha};

const ROW_HEIGHT: f32 = 50.0;
const ROW_GAP: f32 = 6.0;
const ROW_PAD_X: f32 = 12.0;
const GRIP_W: f32 = 18.0;
const ICON_BOX: f32 = 28.0;
const COLUMN_GAP: f32 = 12.0;
const PILL_PAD_X: f32 = 9.0;
const PILL_PAD_Y: f32 = 3.0;
const PILL_DOT: f32 = 6.0;
const PILL_GAP: f32 = 6.0;
const PILL_TEXT: f32 = 11.5;

/// One row in a [`SortableList`].
#[derive(Clone, Debug)]
pub struct SortableItem {
    /// Stable identifier — used as the row id for hit testing and as the
    /// caller-facing way to look up which item moved.
    pub id: String,
    /// Primary label.
    pub title: String,
    /// Optional secondary line below the title.
    pub subtitle: Option<String>,
    /// Optional leading glyph rendered in a small rounded box.
    pub icon: Option<String>,
    /// Optional trailing status pill.
    pub status: Option<SortableStatus>,
}

/// A trailing status pill on a [`SortableItem`].
#[derive(Clone, Debug)]
pub struct SortableStatus {
    /// Pill text — kept lower-case to match the design language.
    pub label: String,
    /// Tone used for the pill's dot, border and text colours.
    pub tone: BadgeTone,
}

impl SortableItem {
    /// Create an item with a stable id and title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            icon: None,
            status: None,
        }
    }

    /// Set the secondary line below the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the leading icon glyph.
    ///
    /// Rendered with the default proportional font; the glyph must be
    /// available in a registered font (the bundled `Elegance Symbols`
    /// fallback covers arrows, modifier keys, and a small set of UI icons).
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the trailing status pill.
    pub fn status(mut self, label: impl Into<String>, tone: BadgeTone) -> Self {
        self.status = Some(SortableStatus {
            label: label.into(),
            tone,
        });
        self
    }
}

/// Cross-frame drag state stored in [`egui::Context`] memory.
#[derive(Clone, Debug)]
struct DragState {
    /// Index of the item the drag started on.
    origin_idx: usize,
    /// Current insertion index — the slot opens just before items[target_idx]
    /// (or at the trailing position when `target_idx == items.len()`).
    target_idx: usize,
    /// Pointer offset within the source row at drag start, used to keep the
    /// ghost row anchored under the cursor.
    grab_offset: Vec2,
    /// Width and height of the source row at drag start, used to size the
    /// ghost and the drop slot.
    row_size: Vec2,
}

/// A vertical list of rows that can be reordered by dragging their grips.
///
/// # Interaction
///
/// - **Press on a row's grip** to start dragging. The source row collapses
///   out of layout and a ghost copy of the row floats under the cursor.
/// - **Move** the cursor — a focus-tinted slot opens at the predicted drop
///   position, and the surrounding rows shift to reveal it.
/// - **Release** to commit the move. If the slot lands at the source's own
///   position the order is left untouched.
/// - **Escape** cancels the drag without reordering.
///
/// # State
///
/// Items are stored in a caller-owned `Vec<SortableItem>` passed by mutable
/// reference; the widget reorders the vec in place on a successful drop.
/// Transient drag state (origin, target, grab offset) lives in egui memory
/// keyed by the widget's `id_salt`.
///
/// # Example
///
/// ```no_run
/// # use elegance::{SortableList, SortableItem, BadgeTone};
/// # egui::__run_test_ui(|ui| {
/// let mut items = vec![
///     SortableItem::new("api", "api-east-01")
///         .subtitle("10.0.1.5 · us-east-1")
///         .status("live", BadgeTone::Ok),
///     SortableItem::new("worker", "worker-pool-a")
///         .subtitle("24 instances · autoscale")
///         .status("idle", BadgeTone::Neutral),
/// ];
/// SortableList::new("deployment-targets", &mut items).show(ui);
/// # });
/// ```
#[must_use = "Call `.show(ui)` to render the sortable list."]
pub struct SortableList<'a> {
    id_salt: egui::IdSalt,
    items: &'a mut Vec<SortableItem>,
}

impl<'a> std::fmt::Debug for SortableList<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortableList")
            .field("id_salt", &self.id_salt)
            .field("items", &self.items.len())
            .finish()
    }
}

impl<'a> SortableList<'a> {
    /// Create a sortable list.
    ///
    /// * `id_salt` — stable salt for this widget's transient drag state.
    ///   Different `SortableList` widgets in the same `Ui` must use
    ///   distinct salts.
    /// * `items` — caller-owned list of rows. Reordered in place on drop.
    pub fn new(id_salt: impl crate::IdSalt, items: &'a mut Vec<SortableItem>) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance_sortable_list", id_salt)),
            items,
        }
    }

    /// Render the list and handle drag interaction.
    pub fn show(self, ui: &mut Ui) -> Response {
        let SortableList { id_salt, items } = self;
        let theme = Theme::current(ui.ctx());
        let base_id = ui.scope_id().with_salt(id_salt);

        // Load and validate cross-frame drag state.
        let mut drag: Option<DragState> = ui.ctx().data(|d| d.get_temp(base_id));
        if let Some(s) = &drag
            && s.origin_idx >= items.len()
        {
            drag = None;
        }

        // Cancel on Escape.
        if drag.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            drag = None;
        }

        // Drop on pointer release.
        let pointer_down = ui.input(|i| i.pointer.primary_down());
        let commit_drop = drag.is_some() && !pointer_down;

        let n = items.len();

        // Build the visual sequence: the source row is hidden during a drag,
        // and a slot is inserted at the current target index.
        let drag_origin = drag.as_ref().map(|d| d.origin_idx);
        let drag_target = drag.as_ref().map(|d| d.target_idx);
        let mut sequence: Vec<DisplayKind> = Vec::with_capacity(n + 1);
        for i in 0..n {
            if drag_target == Some(i) {
                sequence.push(DisplayKind::Slot);
            }
            if drag_origin == Some(i) {
                continue;
            }
            sequence.push(DisplayKind::Row(i));
        }
        if drag_target == Some(n) {
            sequence.push(DisplayKind::Slot);
        }

        // Allocate the list rect.
        let total_w = ui.available_width();
        let total_h = if sequence.is_empty() {
            0.0
        } else {
            sequence.len() as f32 * ROW_HEIGHT + (sequence.len() - 1) as f32 * ROW_GAP
        };
        let (list_rect, response) =
            ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::hover());

        // Compute per-element rects.
        let mut row_rects: Vec<(usize, Rect)> = Vec::with_capacity(n);
        let mut slot_rect: Option<Rect> = None;
        let mut y = list_rect.top();
        for kind in &sequence {
            let r = Rect::from_min_size(
                Pos2::new(list_rect.left(), y),
                Vec2::new(total_w, ROW_HEIGHT),
            );
            match kind {
                DisplayKind::Slot => slot_rect = Some(r),
                DisplayKind::Row(i) => row_rects.push((*i, r)),
            }
            y += ROW_HEIGHT + ROW_GAP;
        }

        // Render rows and detect drag start.
        let mut new_drag: Option<DragState> = None;
        for (i, rect) in &row_rects {
            let item = &items[*i];
            let row_id = base_id.with(("row", &item.id));

            // Whole-row hover for the border tint, plus a smaller grip rect
            // that actually starts the drag — clicking the title or pill
            // shouldn't grab the row.
            let row_resp = ui.interact(*rect, row_id, Sense::hover());
            let grip_rect = grip_rect(*rect);
            let grip_resp = ui.interact(grip_rect, row_id.with("grip"), Sense::click_and_drag());

            if drag.is_none() && grip_resp.drag_started() {
                let pointer = ui
                    .input(|inp| inp.pointer.interact_pos())
                    .unwrap_or(rect.left_top());
                new_drag = Some(DragState {
                    origin_idx: *i,
                    target_idx: *i,
                    grab_offset: pointer - rect.left_top(),
                    row_size: rect.size(),
                });
            }

            if grip_resp.hovered() && drag.is_none() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }

            if ui.is_rect_visible(*rect) {
                paint_row(ui, *rect, item, &theme, row_resp.hovered(), false);
            }

            row_resp.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, &item.title));
        }

        // Apply drag started this frame so the slot/ghost render immediately.
        if drag.is_none() {
            drag = new_drag;
        }

        // Render the drop slot.
        if let Some(rect) = slot_rect
            && ui.is_rect_visible(rect)
        {
            paint_slot(ui, rect, &theme);
        }

        // Update target index and render the ghost row.
        if let Some(s) = drag.as_mut() {
            let pointer_pos = ui.input(|inp| inp.pointer.interact_pos());

            if let Some(p) = pointer_pos {
                let mut new_target = n;
                for (i, rect) in &row_rects {
                    if p.y < rect.center().y {
                        new_target = *i;
                        break;
                    }
                }
                s.target_idx = new_target;

                let ghost_rect = Rect::from_min_size(p - s.grab_offset, s.row_size);
                paint_ghost(ui, ghost_rect, &items[s.origin_idx], &theme, base_id);
            }

            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            ui.ctx().request_repaint();
        }

        // Commit a drop or clear cancelled state.
        if commit_drop && let Some(s) = drag.take() {
            let mut final_idx = s.target_idx.min(n);
            if final_idx > s.origin_idx {
                final_idx -= 1;
            }
            if final_idx != s.origin_idx && final_idx < items.len() {
                let moved = items.remove(s.origin_idx);
                items.insert(final_idx, moved);
            }
        }

        // Persist drag state (or remove on cancel/drop).
        ui.ctx().data_mut(|d| match drag {
            Some(s) => {
                d.insert_temp(base_id, s);
            }
            None => {
                d.remove::<DragState>(base_id);
            }
        });

        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "sortable list"));
        response
    }
}

#[derive(Clone, Copy)]
enum DisplayKind {
    Slot,
    Row(usize),
}

fn grip_rect(row: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(row.left() + ROW_PAD_X, row.top()),
        Vec2::new(GRIP_W, row.height()),
    )
}

fn paint_row(ui: &Ui, rect: Rect, item: &SortableItem, theme: &Theme, hovered: bool, ghost: bool) {
    let p = &theme.palette;
    let t = &theme.typography;
    let painter = ui.painter();
    let radius = CornerRadius::same(theme.control_radius as u8);

    let (fill, border, grip_color) = if ghost {
        (p.card, with_alpha(p.focus, 115), p.focus)
    } else if hovered {
        (p.input_bg, p.text_muted, p.text_muted)
    } else {
        (p.input_bg, p.border, p.text_faint)
    };
    painter.rect(
        rect,
        radius,
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    let mid_y = rect.center().y;
    let mut x = rect.left() + ROW_PAD_X;

    paint_grip(painter, Pos2::new(x + GRIP_W * 0.5, mid_y), grip_color);
    x += GRIP_W + COLUMN_GAP;

    if let Some(icon) = &item.icon {
        let icon_rect =
            Rect::from_center_size(Pos2::new(x + ICON_BOX * 0.5, mid_y), Vec2::splat(ICON_BOX));
        painter.rect(
            icon_rect,
            radius,
            p.card,
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(13.0),
            p.text_muted,
        );
        x += ICON_BOX + COLUMN_GAP;
    }

    let pill_size = item
        .status
        .as_ref()
        .map(|s| measure_pill(ui, &s.label))
        .unwrap_or(Vec2::ZERO);
    let pill_x = rect.right() - ROW_PAD_X - pill_size.x;

    if let Some(sub) = &item.subtitle {
        painter.text(
            Pos2::new(x, rect.top() + 9.0),
            Align2::LEFT_TOP,
            &item.title,
            FontId::proportional(t.body),
            p.text,
        );
        painter.text(
            Pos2::new(x, rect.top() + 9.0 + t.body + 2.0),
            Align2::LEFT_TOP,
            sub,
            FontId::proportional(t.small),
            p.text_muted,
        );
    } else {
        painter.text(
            Pos2::new(x, mid_y),
            Align2::LEFT_CENTER,
            &item.title,
            FontId::proportional(t.body),
            p.text,
        );
    }

    if let Some(s) = &item.status {
        let pill_rect =
            Rect::from_min_size(Pos2::new(pill_x, mid_y - pill_size.y * 0.5), pill_size);
        paint_pill(ui, pill_rect, &s.label, s.tone, p);
    }
}

fn paint_grip(painter: &egui::Painter, center: Pos2, color: Color32) {
    for col in 0..2 {
        for row in 0..3 {
            let cx = center.x - 2.0 + col as f32 * 4.0;
            let cy = center.y - 5.0 + row as f32 * 5.0;
            painter.circle_filled(Pos2::new(cx, cy), 1.3, color);
        }
    }
}

fn measure_pill(ui: &Ui, label: &str) -> Vec2 {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        FontId::proportional(PILL_TEXT),
        Color32::WHITE,
    );
    Vec2::new(
        PILL_PAD_X * 2.0 + PILL_DOT + PILL_GAP + galley.size().x,
        (galley.size().y + PILL_PAD_Y * 2.0).max(PILL_DOT + PILL_PAD_Y * 2.0),
    )
}

fn paint_pill(ui: &Ui, rect: Rect, label: &str, tone: BadgeTone, palette: &Palette) {
    let painter = ui.painter();
    let (bg, border, fg) = pill_colours(tone, palette);
    painter.rect(
        rect,
        CornerRadius::same(99),
        bg,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let dot_x = rect.left() + PILL_PAD_X + PILL_DOT * 0.5;
    painter.circle_filled(Pos2::new(dot_x, rect.center().y), PILL_DOT * 0.5, fg);

    let text_x = rect.left() + PILL_PAD_X + PILL_DOT + PILL_GAP;
    let galley = painter.layout_no_wrap(label.to_string(), FontId::proportional(PILL_TEXT), fg);
    let text_y = rect.center().y - galley.size().y * 0.5;
    painter.galley(Pos2::new(text_x, text_y), galley, fg);
}

fn pill_colours(tone: BadgeTone, p: &Palette) -> (Color32, Color32, Color32) {
    match tone {
        BadgeTone::Ok => (with_alpha(p.green, 26), with_alpha(p.green, 64), p.success),
        BadgeTone::Warning => (with_alpha(p.amber, 26), with_alpha(p.amber, 64), p.warning),
        BadgeTone::Danger => (with_alpha(p.red, 26), with_alpha(p.red, 64), p.danger),
        BadgeTone::Info => (with_alpha(p.focus, 26), with_alpha(p.focus, 64), p.focus),
        BadgeTone::Neutral => (
            with_alpha(p.text_muted, 20),
            with_alpha(p.text_muted, 51),
            p.text_muted,
        ),
    }
}

fn paint_slot(ui: &Ui, rect: Rect, theme: &Theme) {
    let painter = ui.painter();
    let p = &theme.palette;
    let radius = CornerRadius::same(theme.control_radius as u8);
    painter.rect(
        rect,
        radius,
        with_alpha(p.focus, 13),
        Stroke::NONE,
        StrokeKind::Inside,
    );
    let pts = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    let stroke = Stroke::new(1.0, with_alpha(p.focus, 102));
    painter.extend(egui::Shape::dashed_line(&pts, stroke, 6.0, 4.0));
}

fn paint_ghost(ui: &Ui, rect: Rect, item: &SortableItem, theme: &Theme, base_id: Id) {
    let layer = LayerId::new(Order::Tooltip, base_id.with("ghost"));
    let painter = ui.ctx().layer_painter(layer);
    let p = &theme.palette;
    let t = &theme.typography;
    let radius = CornerRadius::same(theme.control_radius as u8);

    let shadow = egui::epaint::Shadow {
        offset: [0, 14],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    painter.add(shadow.as_shape(rect, radius));
    painter.rect(
        rect,
        radius,
        p.card,
        Stroke::new(1.0, with_alpha(p.focus, 115)),
        StrokeKind::Inside,
    );

    let mid_y = rect.center().y;
    let mut x = rect.left() + ROW_PAD_X;
    paint_grip(&painter, Pos2::new(x + GRIP_W * 0.5, mid_y), p.focus);
    x += GRIP_W + COLUMN_GAP;

    if let Some(icon) = &item.icon {
        let icon_rect =
            Rect::from_center_size(Pos2::new(x + ICON_BOX * 0.5, mid_y), Vec2::splat(ICON_BOX));
        painter.rect(
            icon_rect,
            radius,
            p.card,
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(13.0),
            p.text_muted,
        );
        x += ICON_BOX + COLUMN_GAP;
    }

    let pill_size = item
        .status
        .as_ref()
        .map(|s| measure_pill(ui, &s.label))
        .unwrap_or(Vec2::ZERO);
    let pill_x = rect.right() - ROW_PAD_X - pill_size.x;

    if let Some(sub) = &item.subtitle {
        painter.text(
            Pos2::new(x, rect.top() + 9.0),
            Align2::LEFT_TOP,
            &item.title,
            FontId::proportional(t.body),
            p.text,
        );
        painter.text(
            Pos2::new(x, rect.top() + 9.0 + t.body + 2.0),
            Align2::LEFT_TOP,
            sub,
            FontId::proportional(t.small),
            p.text_muted,
        );
    } else {
        painter.text(
            Pos2::new(x, mid_y),
            Align2::LEFT_CENTER,
            &item.title,
            FontId::proportional(t.body),
            p.text,
        );
    }

    if let Some(s) = &item.status {
        let pill_rect =
            Rect::from_min_size(Pos2::new(pill_x, mid_y - pill_size.y * 0.5), pill_size);
        paint_ghost_pill(&painter, pill_rect, &s.label, s.tone, p);
    }
}

fn paint_ghost_pill(
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    tone: BadgeTone,
    palette: &Palette,
) {
    let (bg, border, fg) = pill_colours(tone, palette);
    painter.rect(
        rect,
        CornerRadius::same(99),
        bg,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let dot_x = rect.left() + PILL_PAD_X + PILL_DOT * 0.5;
    painter.circle_filled(Pos2::new(dot_x, rect.center().y), PILL_DOT * 0.5, fg);
    let text_x = rect.left() + PILL_PAD_X + PILL_DOT + PILL_GAP;
    let galley = painter.layout_no_wrap(label.to_string(), FontId::proportional(PILL_TEXT), fg);
    let text_y = rect.center().y - galley.size().y * 0.5;
    painter.galley(Pos2::new(text_x, text_y), galley, fg);
}
