//! Node-editor-style one-to-one pairing widget.
//!
//! See [`Pairing`] for the full interaction model and an example.

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, epaint::CubicBezierShape,
};

use crate::badge::BadgeTone;
use crate::theme::{Palette, Theme, Typography};

/// Maximum number of items supported per side. Layout uses fixed-size stack
/// buffers of this length so no heap allocation happens per frame; exceeding
/// this cap panics with a clear message.
const MAX_ROWS: usize = 64;

/// How a [`PairItem`]'s leading icon is tinted.
///
/// Set via [`PairItem::icon_tone`] or [`PairItem::icon_color`]; both write
/// this one field, so the last call wins.
///
/// `#[non_exhaustive]` for the same reason [`PairItem`] is: a future tint
/// kind should not break callers that match on this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IconTint {
    /// A status tone, resolved through [`BadgeTone::foreground`] against the
    /// palette in force when the node is painted.
    Tone(BadgeTone),
    /// An explicit colour, used as-is.
    Solid(Color32),
}

/// A single item rendered in either column of a [`Pairing`] widget.
///
/// Build with [`PairItem::new`] and the chained setters. The struct is
/// `#[non_exhaustive]`, so construct it that way rather than with a struct
/// literal.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PairItem {
    /// Stable identifier. Used as the link key when pairing items across sides.
    pub id: String,
    /// Primary label shown on the node.
    pub name: String,
    /// Optional secondary text rendered below the name.
    pub detail: Option<String>,
    /// Optional leading-edge glyph rendered in a small rounded box.
    pub icon: Option<String>,
    /// Optional tint for [`icon`](Self::icon). `None` paints it in the muted
    /// icon colour.
    pub icon_tint: Option<IconTint>,
}

impl PairItem {
    /// Create a new item with a stable id and display name.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            detail: None,
            icon: None,
            icon_tint: None,
        }
    }

    /// Set the secondary detail line.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the leading icon glyph.
    ///
    /// Rendered with the default proportional font; the glyph must be
    /// available in a registered font. The bundled `Elegance Symbols`
    /// fallback covers arrows (`← ↑ → ↓ ↩ ↲ ↵`), ellipses (`⋮ ⋯`), modifier
    /// keys (`⌃ ⌘ ⌥ ⌫ ⌦`), triangles (`▴ ▸ ▾ ◂`), `✓ ✗`, and the UI icon set
    /// in [`glyphs`](crate::glyphs) (including the status marks
    /// `CIRCLE_CHECK` / `CIRCLE_X` / `CIRCLE_ALERT`). Glyphs outside that set
    /// (e.g. `◈`, `↗`) may render as tofu unless the host app has registered
    /// a font that covers them.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Tint the leading icon with a status tone.
    ///
    /// The tone is resolved against the palette when the node is painted, so
    /// it tracks theme changes. Without this (or [`icon_color`]) the icon is
    /// painted in the muted icon colour, which is also what
    /// [`BadgeTone::Neutral`] resolves to.
    ///
    /// Pairs naturally with the bundled status glyphs:
    ///
    /// ```
    /// # use elegance::{BadgeTone, PairItem, glyphs};
    /// PairItem::new("c1", "worker-pool-a")
    ///     .detail("24 instances")
    ///     .icon(glyphs::CIRCLE_CHECK)
    ///     .icon_tone(BadgeTone::Ok);
    /// ```
    ///
    /// [`icon_color`]: Self::icon_color
    pub fn icon_tone(mut self, tone: BadgeTone) -> Self {
        self.icon_tint = Some(IconTint::Tone(tone));
        self
    }

    /// Tint the leading icon with an explicit colour, bypassing the palette.
    ///
    /// Use for one-off colours outside the status vocabulary (a per-category
    /// colour, say). Prefer [`icon_tone`](Self::icon_tone) for status, so the
    /// tint follows the theme.
    pub fn icon_color(mut self, color: impl Into<Color32>) -> Self {
        self.icon_tint = Some(IconTint::Solid(color.into()));
        self
    }
}

/// Which column a node lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Cross-frame state — just the currently-selected source node, if any.
/// Snap target is recomputed every frame from hover responses.
#[derive(Clone, Debug, Default)]
struct State {
    selection: Option<(Side, String)>,
}

impl State {
    fn clone_for_storage(&self) -> Self {
        self.clone()
    }
}

/// A widget that lets users connect items in two lists with 1:1 pairings,
/// drawn as bezier curves between ports on each node.
///
/// # Interaction
///
/// - **Click a port** to enter pairing mode — a dashed ghost line follows the cursor.
/// - **Click an opposite-side port** to complete the pairing. If that target was
///   already paired, its previous pairing is broken first (swap).
/// - **Hover an opposite-side node** while pairing and the ghost line latches
///   to that node's port.
/// - **Click a paired node** to break its connection *and* immediately start a
///   new pairing from it, so reconnecting is a single click.
/// - **Click a pair's line** to remove it.
/// - **Escape** or **click the background** cancels a pending selection.
///
/// # State
///
/// Pairings are stored in the caller-owned `Vec<(String, String)>` passed to
/// [`Pairing::new`]. Each element is `(left_id, right_id)`. The transient
/// selection state is stored in egui memory keyed by the widget's `id_salt`.
///
/// # Limits
///
/// Each side supports up to 64 items. Exceeding this panics — the widget
/// uses fixed-size stack buffers for zero-allocation layout. For larger
/// data sets, split the lists across multiple `Pairing` widgets.
///
/// # Example
///
/// ```no_run
/// # use elegance::{Pairing, PairItem};
/// # egui::__run_test_ui(|ui| {
/// let clients = vec![
///     PairItem::new("c1", "worker-pool-a").detail("24 instances"),
///     PairItem::new("c2", "edge-proxy-01").detail("8 instances"),
/// ];
/// let servers = vec![
///     PairItem::new("s1", "api-east-01").detail("10.0.1.5 · us-east"),
///     PairItem::new("s2", "api-west-01").detail("10.0.2.4 · us-west"),
/// ];
/// let mut pairs: Vec<(String, String)> = vec![];
/// Pairing::new("client-server", &clients, &servers, &mut pairs)
///     .left_label("Clients")
///     .right_label("Servers")
///     .show(ui);
/// # });
/// ```
#[must_use = "Call `.show(ui)` to render the pairing widget."]
pub struct Pairing<'a> {
    id_salt: egui::IdSalt,
    left: &'a [PairItem],
    right: &'a [PairItem],
    pairs: &'a mut Vec<(String, String)>,
    left_label: Option<String>,
    right_label: Option<String>,
    height: Option<f32>,
    align: Option<Side>,
}

impl<'a> std::fmt::Debug for Pairing<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pairing")
            .field("id_salt", &self.id_salt)
            .field("left", &self.left.len())
            .field("right", &self.right.len())
            .field("pairs", &self.pairs.len())
            .field("left_label", &self.left_label)
            .field("right_label", &self.right_label)
            .field("height", &self.height)
            .field("align", &self.align)
            .finish()
    }
}

impl<'a> Pairing<'a> {
    /// Create a new pairing widget.
    ///
    /// * `id_salt` — a stable salt for this widget's memory state. Different
    ///   `Pairing` widgets in the same `Ui` must use distinct salts.
    /// * `left`, `right` — items shown in each column.
    /// * `pairs` — the caller-owned list of `(left_id, right_id)` tuples. It
    ///   is mutated when the user creates or removes a pairing.
    pub fn new(
        id_salt: impl crate::IdSalt,
        left: &'a [PairItem],
        right: &'a [PairItem],
        pairs: &'a mut Vec<(String, String)>,
    ) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance_pairing", id_salt)),
            left,
            right,
            pairs,
            left_label: None,
            right_label: None,
            height: None,
            align: None,
        }
    }

    /// Set the label shown above the left column.
    pub fn left_label(mut self, label: impl Into<String>) -> Self {
        self.left_label = Some(label.into());
        self
    }

    /// Set the label shown above the right column.
    pub fn right_label(mut self, label: impl Into<String>) -> Self {
        self.right_label = Some(label.into());
        self
    }

    /// Override the total widget height. By default the widget sizes itself
    /// to fit the longer of the two columns.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Auto-arrange the left column so every pairing renders as a straight
    /// horizontal line. The right column keeps its caller-given order;
    /// unpaired items on the left fill the remaining slots in input order.
    pub fn align_left(mut self) -> Self {
        self.align = Some(Side::Left);
        self
    }

    /// Auto-arrange the right column so every pairing renders as a straight
    /// horizontal line. The left column keeps its caller-given order;
    /// unpaired items on the right fill the remaining slots in input order.
    pub fn align_right(mut self) -> Self {
        self.align = Some(Side::Right);
        self
    }

    /// Render the widget and handle interaction.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Pairing {
            id_salt,
            left,
            right,
            pairs,
            left_label,
            right_label,
            height,
            align,
        } = self;
        let base_id = ui.scope_id().with_salt(id_salt);

        assert!(
            left.len() <= MAX_ROWS && right.len() <= MAX_ROWS,
            "Pairing widget supports up to {} items per side (got left={}, right={})",
            MAX_ROWS,
            left.len(),
            right.len()
        );

        let theme = Theme::current(ui.ctx());

        const NODE_HEIGHT: f32 = 56.0;
        const NODE_GAP: f32 = 8.0;
        const LABEL_HEIGHT: f32 = 26.0;
        const PORT_RADIUS: f32 = 5.0;
        const MIN_COL_GAP: f32 = 80.0;
        const LINE_HIT_THRESHOLD: f32 = 6.0;

        // Layout.
        let has_label = left_label.is_some() || right_label.is_some();
        let rows = left.len().max(right.len());
        let content_h = (if has_label { LABEL_HEIGHT } else { 0.0 })
            + if rows > 0 {
                rows as f32 * (NODE_HEIGHT + NODE_GAP) - NODE_GAP
            } else {
                0.0
            };
        let widget_h = height.unwrap_or(content_h + theme.card_padding * 2.0);

        // Allocate the outer rect. Click on this (when no child consumes) =
        // background click = cancel.
        let (outer_rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), widget_h), Sense::click());

        let inner = outer_rect.shrink(theme.card_padding);
        let col_gap = MIN_COL_GAP.max(inner.width() * 0.12);
        let col_w = ((inner.width() - col_gap) * 0.5).max(120.0);
        let left_x = inner.left();
        let right_x = inner.right() - col_w;
        let nodes_top = if has_label {
            inner.top() + LABEL_HEIGHT
        } else {
            inner.top()
        };

        // Load persistent state and prune stale selections.
        let mut state: State = ui.ctx().data(|d| d.get_temp(base_id).unwrap_or_default());
        if let Some((side, id)) = state.selection.clone() {
            let exists = match side {
                Side::Left => left.iter().any(|i| i.id == id),
                Side::Right => right.iter().any(|i| i.id == id),
            };
            if !exists {
                state.selection = None;
            }
        }

        // Compute visual row positions. When no alignment is set we use
        // identity (row i → row i) without any buffer. When aligned, one
        // side writes its reordered positions into a stack buffer. Neither
        // path touches the heap.
        let mut left_buf = [0usize; MAX_ROWS];
        let mut right_buf = [0usize; MAX_ROWS];
        let left_positions: Option<&[usize]> = if align == Some(Side::Left) {
            compute_aligned_positions(left, right, pairs, false, &mut left_buf);
            Some(&left_buf[..left.len()])
        } else {
            None
        };
        let right_positions: Option<&[usize]> = if align == Some(Side::Right) {
            compute_aligned_positions(right, left, pairs, true, &mut right_buf);
            Some(&right_buf[..right.len()])
        } else {
            None
        };

        // Allocate node rects and collect interaction responses.
        let mut hits: Vec<NodeHit> = Vec::with_capacity(left.len() + right.len());
        for (i, item) in left.iter().enumerate() {
            let vis = left_positions.map_or(i, |p| p[i]);
            let top = nodes_top + vis as f32 * (NODE_HEIGHT + NODE_GAP);
            let r = Rect::from_min_size(Pos2::new(left_x, top), Vec2::new(col_w, NODE_HEIGHT));
            let port = Pos2::new(r.right(), r.center().y);
            let resp = ui.interact(r, base_id.with(("L", &item.id)), Sense::click());
            hits.push(NodeHit {
                side: Side::Left,
                id: item.id.clone(),
                rect: r,
                port,
                resp,
            });
        }
        for (i, item) in right.iter().enumerate() {
            let vis = right_positions.map_or(i, |p| p[i]);
            let top = nodes_top + vis as f32 * (NODE_HEIGHT + NODE_GAP);
            let r = Rect::from_min_size(Pos2::new(right_x, top), Vec2::new(col_w, NODE_HEIGHT));
            let port = Pos2::new(r.left(), r.center().y);
            let resp = ui.interact(r, base_id.with(("R", &item.id)), Sense::click());
            hits.push(NodeHit {
                side: Side::Right,
                id: item.id.clone(),
                rect: r,
                port,
                resp,
            });
        }

        // Snap target = opposite-side hovered node (only while a selection is active).
        let snap_target: Option<(Side, String)> = state.selection.as_ref().and_then(|(ss, _)| {
            let opp = ss.opposite();
            hits.iter()
                .find(|h| h.side == opp && h.resp.hovered())
                .map(|h| (h.side, h.id.clone()))
        });

        // Clicks.
        let node_click = hits
            .iter()
            .find(|h| h.resp.clicked())
            .map(|h| (h.side, h.id.clone()));
        if let Some((side, id)) = node_click {
            handle_node_click(&mut state, side, &id, pairs);
        } else {
            // Check for clicks on existing pair lines; if none, fall back to
            // background-click cancel.
            let pointer = ui.input(|i| i.pointer.hover_pos());
            let pressed = ui.input(|i| i.pointer.primary_clicked());
            let mut consumed = false;
            if pressed
                && let Some(m) = pointer
                && outer_rect.contains(m)
            {
                let mut remove = None;
                for (idx, (lid, rid)) in pairs.iter().enumerate() {
                    if let (Some(lp), Some(rp)) = (
                        port_of(&hits, Side::Left, lid),
                        port_of(&hits, Side::Right, rid),
                    ) && bezier_hit(m, lp, rp, LINE_HIT_THRESHOLD)
                    {
                        remove = Some(idx);
                        break;
                    }
                }
                if let Some(i) = remove {
                    pairs.remove(i);
                    state.selection = None;
                    consumed = true;
                }
            }
            if !consumed && response.clicked() {
                state.selection = None;
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.selection = None;
        }

        // Paint.
        if ui.is_rect_visible(outer_rect) {
            let painter = ui.painter();
            let palette = &theme.palette;
            let typo = &theme.typography;

            // Card background + border.
            painter.rect(
                outer_rect,
                CornerRadius::same(theme.card_radius as u8),
                palette.card,
                Stroke::new(1.0, palette.border),
                StrokeKind::Inside,
            );

            // Dot grid.
            paint_grid(painter, outer_rect, palette);

            // Column labels.
            if let Some(lbl) = &left_label {
                painter.text(
                    Pos2::new(left_x + 2.0, inner.top()),
                    Align2::LEFT_TOP,
                    lbl,
                    FontId::proportional(typo.label),
                    palette.text_muted,
                );
            }
            if let Some(lbl) = &right_label {
                painter.text(
                    Pos2::new(right_x + 2.0, inner.top()),
                    Align2::LEFT_TOP,
                    lbl,
                    FontId::proportional(typo.label),
                    palette.text_muted,
                );
            }

            // Existing pair lines (solid, focus accent).
            let line_stroke = Stroke::new(2.0, palette.focus);
            for (lid, rid) in pairs.iter() {
                if let (Some(lp), Some(rp)) = (
                    port_of(&hits, Side::Left, lid),
                    port_of(&hits, Side::Right, rid),
                ) {
                    paint_bezier(painter, lp, rp, line_stroke, false);
                }
            }

            // Ghost line while selecting.
            if let Some((sel_side, sel_id)) = &state.selection
                && let Some(src) = port_of(&hits, *sel_side, sel_id)
            {
                let end = snap_target
                    .as_ref()
                    .and_then(|(s, i)| port_of(&hits, *s, i))
                    .or_else(|| {
                        ui.input(|i| i.pointer.hover_pos())
                            .filter(|p| outer_rect.contains(*p))
                    });
                if let Some(e) = end {
                    let ghost_stroke = Stroke::new(1.75, with_alpha(palette.focus, 140));
                    paint_bezier(painter, src, e, ghost_stroke, true);
                    if snap_target.is_none() {
                        painter.circle_filled(e, 3.5, with_alpha(palette.text_muted, 165));
                    }
                }
                // Keep the ghost tracking the cursor.
                ui.ctx().request_repaint();
            }

            // Nodes on top of lines.
            for h in &hits {
                let item = match h.side {
                    Side::Left => left.iter().find(|i| i.id == h.id),
                    Side::Right => right.iter().find(|i| i.id == h.id),
                };
                let Some(item) = item else {
                    continue;
                };
                let selected = state
                    .selection
                    .as_ref()
                    .is_some_and(|(s, i)| *s == h.side && i == &h.id);
                let is_snap = snap_target
                    .as_ref()
                    .is_some_and(|(s, i)| *s == h.side && i == &h.id);
                let paired = is_paired(pairs, h.side, &h.id);
                paint_node(
                    painter,
                    h.rect,
                    h.port,
                    item,
                    selected,
                    is_snap,
                    paired,
                    h.resp.hovered(),
                    palette,
                    typo,
                    theme.control_radius,
                    PORT_RADIUS,
                );
            }
        }

        // Save state.
        ui.ctx()
            .data_mut(|d| d.insert_temp(base_id, state.clone_for_storage()));
        response
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct NodeHit {
    side: Side,
    id: String,
    rect: Rect,
    port: Pos2,
    resp: Response,
}

fn port_of(hits: &[NodeHit], side: Side, id: &str) -> Option<Pos2> {
    hits.iter()
        .find(|h| h.side == side && h.id == id)
        .map(|h| h.port)
}

/// Assign each aligned-side item to a visual row so paired items land on
/// the same row as their partner on the *other* side. Unpaired items on the
/// aligned side fill the remaining rows in their input order.
///
/// Writes the result into the first `aligned.len()` entries of `positions`.
/// Stack-only — uses no heap allocation.
fn compute_aligned_positions(
    aligned: &[PairItem],
    other: &[PairItem],
    pairs: &[(String, String)],
    aligned_is_right: bool,
    positions: &mut [usize; MAX_ROWS],
) {
    let n_aligned = aligned.len();
    let max_pos = n_aligned.max(other.len());

    // Sentinel for "not yet placed".
    for p in positions.iter_mut().take(n_aligned) {
        *p = usize::MAX;
    }

    let mut slot_taken = [false; MAX_ROWS];

    // Anchor paired aligned items to their partner's visual row.
    for (other_idx, other_item) in other.iter().enumerate() {
        let partner_id: Option<&String> = pairs.iter().find_map(|(l, r)| {
            if aligned_is_right {
                (l == &other_item.id).then_some(r)
            } else {
                (r == &other_item.id).then_some(l)
            }
        });
        if let Some(pid) = partner_id
            && let Some(ai) = aligned.iter().position(|a| &a.id == pid)
            && other_idx < max_pos
            && !slot_taken[other_idx]
            && positions[ai] == usize::MAX
        {
            positions[ai] = other_idx;
            slot_taken[other_idx] = true;
        }
    }

    // Fill remaining aligned items into free rows, preserving input order.
    let mut free_slots = (0..max_pos).filter(|s| !slot_taken[*s]);
    for pos in positions.iter_mut().take(n_aligned) {
        if *pos == usize::MAX {
            *pos = free_slots.next().unwrap_or(0);
        }
    }
}

fn is_paired(pairs: &[(String, String)], side: Side, id: &str) -> bool {
    match side {
        Side::Left => pairs.iter().any(|(l, _)| l == id),
        Side::Right => pairs.iter().any(|(_, r)| r == id),
    }
}

fn handle_node_click(state: &mut State, side: Side, id: &str, pairs: &mut Vec<(String, String)>) {
    let paired = is_paired(pairs, side, id);
    let sel = state.selection.clone();

    // Re-clicking the selected node cancels.
    if let Some((s, sid)) = &sel
        && *s == side
        && sid == id
    {
        state.selection = None;
        return;
    }

    // Opposite-side click → pair (swap if target was already paired).
    if let Some((sel_side, sel_id)) = &sel
        && *sel_side != side
    {
        if paired {
            pairs.retain(|(l, r)| match side {
                Side::Left => l != id,
                Side::Right => r != id,
            });
        }
        let pair = match side {
            Side::Left => (id.to_string(), sel_id.clone()),
            Side::Right => (sel_id.clone(), id.to_string()),
        };
        pairs.push(pair);
        state.selection = None;
        return;
    }

    // Otherwise (no selection, or same-side click): unpair if needed, then select.
    // One click breaks the existing connection AND starts a new one.
    if paired {
        pairs.retain(|(l, r)| match side {
            Side::Left => l != id,
            Side::Right => r != id,
        });
    }
    state.selection = Some((side, id.to_string()));
}

fn paint_grid(painter: &egui::Painter, rect: Rect, palette: &Palette) {
    let step = 22.0;
    let dot = with_alpha(palette.text, 12);
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        let mut x = rect.left() + step;
        while x < rect.right() {
            painter.circle_filled(Pos2::new(x, y), 0.75, dot);
            x += step;
        }
        y += step;
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node(
    painter: &egui::Painter,
    rect: Rect,
    port: Pos2,
    item: &PairItem,
    selected: bool,
    snap_target: bool,
    paired: bool,
    hovered: bool,
    palette: &Palette,
    typo: &Typography,
    radius: f32,
    port_radius: f32,
) {
    let r = CornerRadius::same(radius as u8);

    // Background + border (fold into one rect() call).
    let border = if selected || snap_target {
        palette.focus
    } else if hovered {
        palette.text_muted
    } else {
        palette.border
    };
    painter.rect(
        rect,
        r,
        palette.input_bg,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    // Content.
    let pad_x = 14.0;
    let mut content_x = rect.left() + pad_x;

    // Optional icon box.
    if let Some(icon) = &item.icon {
        let box_size = 28.0;
        let icon_rect = Rect::from_min_size(
            Pos2::new(content_x, rect.center().y - box_size * 0.5),
            Vec2::splat(box_size),
        );
        painter.rect(
            icon_rect,
            r,
            palette.card,
            Stroke::new(1.0, palette.border),
            StrokeKind::Inside,
        );
        let icon_color = match item.icon_tint {
            None => palette.text_muted,
            Some(IconTint::Tone(tone)) => tone.foreground(palette),
            Some(IconTint::Solid(color)) => color,
        };
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(13.0),
            icon_color,
        );
        content_x += box_size + 12.0;
    }

    // Name + optional detail.
    if let Some(detail) = &item.detail {
        painter.text(
            Pos2::new(content_x, rect.top() + 11.0),
            Align2::LEFT_TOP,
            &item.name,
            FontId::proportional(typo.body),
            palette.text,
        );
        painter.text(
            Pos2::new(content_x, rect.top() + 31.0),
            Align2::LEFT_TOP,
            detail,
            FontId::proportional(typo.small),
            palette.text_faint,
        );
    } else {
        painter.text(
            Pos2::new(content_x, rect.center().y),
            Align2::LEFT_CENTER,
            &item.name,
            FontId::proportional(typo.body),
            palette.text,
        );
    }

    // Port.
    let active = selected || snap_target || paired;
    let port_fill = if active {
        palette.focus
    } else {
        palette.input_bg
    };
    let port_stroke = if active || hovered {
        palette.focus
    } else {
        palette.border
    };
    painter.circle_filled(port, port_radius, port_fill);
    painter.circle_stroke(port, port_radius, Stroke::new(1.5, port_stroke));
    if active {
        painter.circle_stroke(
            port,
            port_radius + 3.0,
            Stroke::new(3.0, with_alpha(palette.focus, 46)),
        );
    }
}

fn paint_bezier(painter: &egui::Painter, start: Pos2, end: Pos2, stroke: Stroke, dashed: bool) {
    let mid_x = (start.x + end.x) * 0.5;
    let c1 = Pos2::new(mid_x, start.y);
    let c2 = Pos2::new(mid_x, end.y);

    if !dashed {
        let shape = CubicBezierShape::from_points_stroke(
            [start, c1, c2, end],
            false,
            Color32::TRANSPARENT,
            stroke,
        );
        painter.add(Shape::CubicBezier(shape));
        return;
    }

    // Dashed: sample the curve and draw short, alternating segment groups.
    const SAMPLES: usize = 40;
    const DASH_N: usize = 2; // segments per dash; gap = DASH_N segments
    let pts: Vec<Pos2> = (0..=SAMPLES)
        .map(|i| cubic_bezier(i as f32 / SAMPLES as f32, start, c1, c2, end))
        .collect();
    let period = DASH_N * 2;
    let mut i = 0;
    while i + DASH_N < pts.len() {
        for j in 0..DASH_N {
            painter.line_segment([pts[i + j], pts[i + j + 1]], stroke);
        }
        i += period;
    }
}

fn bezier_hit(point: Pos2, start: Pos2, end: Pos2, threshold: f32) -> bool {
    let mid_x = (start.x + end.x) * 0.5;
    let c1 = Pos2::new(mid_x, start.y);
    let c2 = Pos2::new(mid_x, end.y);
    const SAMPLES: usize = 30;
    let mut prev = start;
    for i in 1..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let p = cubic_bezier(t, start, c1, c2, end);
        if dist_to_segment(point, prev, p) < threshold {
            return true;
        }
        prev = p;
    }
    false
}

fn cubic_bezier(t: f32, p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2) -> Pos2 {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    Pos2::new(
        mt2 * mt * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t2 * t * p3.x,
        mt2 * mt * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t2 * t * p3.y,
    )
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
