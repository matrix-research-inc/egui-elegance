//! Shared painting and measuring for editor-style tab cells.
//!
//! [`BrowserTabs`](crate::BrowserTabs) and the dock's per-group tab strips draw
//! identical cells. Interaction (ids, senses, events) stays with the caller;
//! this module only measures and paints.

use std::sync::Arc;

use egui::epaint::Shadow;
use egui::{
    Color32, CornerRadius, FontId, FontSelection, Galley, Painter, Pos2, Rect, RichText, Shape,
    Stroke, StrokeKind, TextWrapMode, Ui, Vec2, WidgetText, pos2,
};

use crate::theme::{Theme, mix, with_alpha};

pub(crate) const TAB_GAP: f32 = 2.0;

const TAB_PAD_X: f32 = 10.0;
const TAB_PAD_Y: f32 = 7.0;
const TAB_RADIUS: f32 = 7.0;
const ICON_SIZE: f32 = 12.0;
const INNER_GAP: f32 = 8.0;
const DIRTY_SIZE: f32 = 7.0;
const DIRTY_TO_CLOSE_GAP: f32 = 5.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_INNER: f32 = 8.0;
const CLOSE_STROKE: f32 = 1.25;
/// Segments used to stroke each rounded tab corner.
const ARC_STEPS: usize = 8;
const CLOSE_RADIUS: u8 = 4;

/// What a tab cell displays.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TabContent<'a> {
    pub label: &'a str,
    pub icon: Option<&'a str>,
    pub dirty: bool,
    pub closable: bool,
}

/// Interaction state that drives a tab cell's colours.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TabState {
    pub active: bool,
    /// The pointer is over the cell or its close button.
    pub hovered: bool,
    pub close_hovered: bool,
    /// The close button can be used. `false` hides it, e.g. when the tab is
    /// partly clipped by its strip.
    pub close_enabled: bool,
    /// Background of the strip the tab sits on; inactive fills tint from it.
    pub strip_bg: Color32,
}

/// A measured tab: its width and the pre-laid-out label and icon.
pub(crate) struct MeasuredTab {
    pub width: f32,
    pub label: Arc<Galley>,
    pub icon: Option<Arc<Galley>>,
}

/// The fill of a tab strip. It sits one step darker than the panel the active
/// tab merges into, so the active tab is the brightest cell in every theme.
pub(crate) fn strip_bg(theme: &Theme) -> Color32 {
    theme.palette.bg
}

/// Font size of a tab label.
pub(crate) fn label_size(theme: &Theme) -> f32 {
    theme.typography.small + 0.5
}

/// Height of a tab cell (excluding the strip's top padding).
pub(crate) fn tab_height(theme: &Theme) -> f32 {
    TAB_PAD_Y * 2.0 + label_size(theme).max(ICON_SIZE)
}

/// Lay out a tab's label and compute its width, clamped to `[min_w, max_w]`.
pub(crate) fn measure_tab(
    ui: &Ui,
    theme: &Theme,
    content: &TabContent<'_>,
    min_w: f32,
    max_w: f32,
) -> MeasuredTab {
    let label_size = label_size(theme);
    let icon = content.icon.map(|icon| {
        WidgetText::from(
            RichText::new(icon)
                .size(ICON_SIZE)
                .color(Color32::PLACEHOLDER),
        )
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            FontSelection::FontId(FontId::proportional(ICON_SIZE)),
        )
    });
    // The gap follows the glyph's real width, not a fixed slot.
    let icon_w = icon.as_ref().map_or(0.0, |g| g.size().x + INNER_GAP);
    let dirty_block_w = if content.dirty {
        INNER_GAP + DIRTY_SIZE + DIRTY_TO_CLOSE_GAP
    } else {
        INNER_GAP
    };
    let close_w = if content.closable { CLOSE_SIZE } else { 0.0 };
    let max_label_w = (max_w - 2.0 * TAB_PAD_X - icon_w - dirty_block_w - close_w).max(0.0);

    let label = WidgetText::from(
        RichText::new(content.label)
            .size(label_size)
            .color(Color32::PLACEHOLDER),
    )
    .into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        max_label_w,
        FontSelection::FontId(FontId::proportional(label_size)),
    );
    let label_w = label.size().x.min(max_label_w);
    let width = (2.0 * TAB_PAD_X + icon_w + label_w + dirty_block_w + close_w).clamp(min_w, max_w);
    MeasuredTab { width, label, icon }
}

/// The close button's hit rect for a tab occupying `tab_rect`.
pub(crate) fn close_rect(tab_rect: Rect) -> Rect {
    let center = pos2(
        tab_rect.max.x - TAB_PAD_X - CLOSE_SIZE * 0.5,
        tab_rect.center().y,
    );
    Rect::from_center_size(center, Vec2::splat(CLOSE_SIZE))
}

/// Paint one tab: its cell, then its contents.
///
/// Drawing goes to `painter`, so callers can target another layer or clip
/// rect.
pub(crate) fn paint_tab(
    painter: &Painter,
    theme: &Theme,
    tab_rect: Rect,
    content: &TabContent<'_>,
    measured: MeasuredTab,
    state: TabState,
) {
    paint_cell(painter, theme, tab_rect, state);
    paint_content(painter, theme, tab_rect, content, measured, state);
}

/// Paint a tab's fill and outline: a cell with rounded top corners that is
/// open at the bottom, so the active tab merges with the panel below.
pub(crate) fn paint_cell(painter: &Painter, theme: &Theme, tab_rect: Rect, state: TabState) {
    let p = &theme.palette;
    let radius = CornerRadius {
        nw: TAB_RADIUS as u8,
        ne: TAB_RADIUS as u8,
        sw: 0,
        se: 0,
    };
    let fill = if state.active {
        p.card
    } else if state.hovered {
        mix(state.strip_bg, p.card, 0.65)
    } else {
        mix(state.strip_bg, p.card, 0.3)
    };
    painter.rect_filled(tab_rect, radius, fill);

    // The active tab is outlined in the full border colour on its top and
    // sides; the strip's rule leaves a gap beneath it, so the outline closes
    // against the panel below. Inactive tabs get a suppressed outline (alpha
    // ~110/255) so each cell stays distinct without competing.
    let outline = if state.active {
        p.border
    } else {
        with_alpha(p.border, 110)
    };
    painter.add(Shape::line(
        top_outline(tab_rect),
        Stroke::new(1.0, outline),
    ));
    for x in [tab_rect.min.x + 0.5, tab_rect.max.x - 0.5] {
        painter.line_segment(
            [
                pos2(x, tab_rect.min.y + TAB_RADIUS),
                pos2(x, tab_rect.max.y),
            ],
            Stroke::new(1.0, outline),
        );
    }
}

/// Paint the cell of a tab being dragged: a fully outlined, rounded tab that
/// is lighter than the panel under it and casts a shadow, so it reads as
/// lifted off the dock.
pub(crate) fn paint_ghost_cell(painter: &Painter, theme: &Theme, rect: Rect) {
    let p = &theme.palette;
    let radius = CornerRadius::same(TAB_RADIUS as u8);
    let (fill, shadow) = if p.is_dark {
        (
            p.depth_tint(p.card, 0.06),
            Shadow {
                offset: [0, 6],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(110),
            },
        )
    } else {
        (
            p.input_bg,
            Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(45),
            },
        )
    };
    painter.add(shadow.as_shape(rect, radius));
    painter.rect(
        rect,
        radius,
        fill,
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
}

/// The top edge of a tab outline, including both rounded corners, as a
/// polyline inset half a pixel so a 1px stroke sits inside the fill.
fn top_outline(rect: Rect) -> Vec<Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};

    let r = TAB_RADIUS - 0.5;
    let arc = |center: Pos2, from: f32| {
        (0..=ARC_STEPS).map(move |i| {
            let a = from + FRAC_PI_2 * i as f32 / ARC_STEPS as f32;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
    };
    let left = pos2(rect.min.x + TAB_RADIUS, rect.min.y + TAB_RADIUS);
    let right = pos2(rect.max.x - TAB_RADIUS, rect.min.y + TAB_RADIUS);
    arc(left, PI).chain(arc(right, PI + FRAC_PI_2)).collect()
}

/// Paint a tab's icon, label, dirty dot and close glyph.
pub(crate) fn paint_content(
    painter: &Painter,
    theme: &Theme,
    tab_rect: Rect,
    content: &TabContent<'_>,
    measured: MeasuredTab,
    state: TabState,
) {
    let p = &theme.palette;
    let (label_color, icon_color) = if state.active {
        (p.text, p.focus)
    } else if state.hovered {
        (p.text, p.text_muted)
    } else {
        (p.text_muted, p.text_faint)
    };

    let mut cursor_x = tab_rect.min.x + TAB_PAD_X;
    let cy = tab_rect.center().y;

    if let Some(icon) = measured.icon {
        let width = icon.size().x;
        painter.galley(pos2(cursor_x, cy - icon.size().y * 0.5), icon, icon_color);
        cursor_x += width + INNER_GAP;
    }

    let label_pos = pos2(cursor_x, cy - measured.label.size().y * 0.5);
    painter.galley(label_pos, measured.label, label_color);

    // Content ends at the close slot when there is one, else at the padding.
    let content_right = if content.closable {
        close_rect(tab_rect).min.x
    } else {
        tab_rect.max.x - TAB_PAD_X
    };
    if content.dirty {
        let dot_x = content_right - DIRTY_TO_CLOSE_GAP - DIRTY_SIZE * 0.5;
        painter.circle_filled(pos2(dot_x, cy), DIRTY_SIZE * 0.5, p.focus);
    }

    if content.closable && state.close_enabled && (state.active || state.hovered) {
        let close_rect = close_rect(tab_rect);
        if state.close_hovered {
            let close_bg = p.depth_tint(if state.active { p.card } else { state.strip_bg }, 0.10);
            painter.rect_filled(close_rect, CornerRadius::same(CLOSE_RADIUS), close_bg);
        }
        let cross_color = if state.close_hovered {
            p.text
        } else if state.active {
            p.text_muted
        } else {
            p.text_faint
        };
        paint_cross(
            painter,
            close_rect.center(),
            CLOSE_INNER * 0.5,
            Stroke::new(CLOSE_STROKE, cross_color),
        );
    }
}

/// Paint the strip's 1px bottom rule, leaving a gap under the active tab so
/// it merges with the panel below.
pub(crate) fn paint_strip_border(
    painter: &Painter,
    theme: &Theme,
    strip_rect: Rect,
    active: Option<Rect>,
) {
    let border_y = strip_rect.bottom() - 0.5;
    let stroke = Stroke::new(1.0, theme.palette.border);
    if let Some(active) = active {
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
}

/// Paint a diagonal "×" centred on `center` with arm half-length `half`.
fn paint_cross(painter: &Painter, center: egui::Pos2, half: f32, stroke: Stroke) {
    painter.line_segment(
        [
            pos2(center.x - half, center.y - half),
            pos2(center.x + half, center.y + half),
        ],
        stroke,
    );
    painter.line_segment(
        [
            pos2(center.x + half, center.y - half),
            pos2(center.x - half, center.y + half),
        ],
        stroke,
    );
}
