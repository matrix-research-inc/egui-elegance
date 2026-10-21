//! A multi-option segmented control: a row of mutually-exclusive segments
//! sharing a single track.
//!
//! Use it for compact pickers where every option fits on one line and the
//! caller wants the choices visible at a glance: timeframes (1h / 6h / 24h),
//! density (Compact / Comfortable / Spacious), view modes (Dashboard /
//! Inbox / Calendar). Each segment can carry a label, an icon, a status
//! dot, and a count badge.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontId, FontSelection, Galley, Id, Rect, Response, Role, Sense, Stroke,
    StrokeKind, TextWrapMode, Ui, Vec2, Widget, WidgetInfo, WidgetText, pos2,
};

use crate::theme::{Accent, Theme, placeholder_galley, with_alpha};

/// Size variants for [`SegmentedControl`].
///
/// Sizes scale font, padding, track inset, and corner radii together so a
/// segmented control sits naturally next to other elegance controls of the
/// same size class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SegmentedSize {
    /// Compact, for toolbars and tight headers.
    Small,
    /// Default size.
    #[default]
    Medium,
    /// Chunkier — pairs with [`ButtonSize::Large`](crate::ButtonSize::Large)
    /// in mixed action rows.
    Large,
}

impl SegmentedSize {
    fn font_size(self, theme: &Theme) -> f32 {
        let t = &theme.typography;
        match self {
            Self::Small => t.small,
            Self::Medium => t.label,
            Self::Large => t.button,
        }
    }
    fn icon_size(self, theme: &Theme) -> f32 {
        self.font_size(theme)
    }
    fn pad_x(self) -> f32 {
        match self {
            Self::Small => 10.0,
            Self::Medium => 12.0,
            Self::Large => 16.0,
        }
    }
    fn pad_y(self) -> f32 {
        match self {
            Self::Small => 3.0,
            Self::Medium => 5.0,
            Self::Large => 7.0,
        }
    }
    fn track_pad(self) -> f32 {
        match self {
            Self::Small => 2.0,
            Self::Medium => 3.0,
            Self::Large => 4.0,
        }
    }
    fn track_radius(self) -> u8 {
        match self {
            Self::Small => 6,
            Self::Medium => 7,
            Self::Large => 8,
        }
    }
    fn segment_radius(self) -> u8 {
        match self {
            Self::Small => 4,
            Self::Medium => 5,
            Self::Large => 6,
        }
    }
    fn count_height(self) -> f32 {
        match self {
            Self::Small => 16.0,
            Self::Medium => 18.0,
            Self::Large => 20.0,
        }
    }
}

/// Status colour for the optional dot indicator inside a [`Segment`].
///
/// Maps to the palette's status accents (`success`, `warning`, `danger`,
/// `focus`) plus a neutral grey. Pick the variant that matches what the
/// segment represents (open / triaged / resolved / rejected, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentDot {
    /// Neutral grey — non-status segments or "all" buckets.
    Neutral,
    /// Sky — informational / in-progress.
    Sky,
    /// Amber — warning / open.
    Amber,
    /// Red — error / rejected.
    Red,
    /// Green — success / resolved.
    Green,
}

/// A single segment inside a [`SegmentedControl`].
///
/// Build with [`Segment::text`], [`Segment::icon`], or
/// [`Segment::icon_text`], then layer optional decorations with
/// [`Segment::dot`] and [`Segment::count`]. Mark unavailable segments
/// with [`Segment::enabled`].
///
/// ```no_run
/// # use elegance::{Segment, SegmentDot};
/// let seg = Segment::text("Open").dot(SegmentDot::Amber).count("12");
/// # let _ = seg;
/// ```
#[must_use = "Use with SegmentedControl::from_segments(...)"]
pub struct Segment {
    label: Option<WidgetText>,
    icon: Option<WidgetText>,
    count: Option<WidgetText>,
    dot: Option<SegmentDot>,
    enabled: bool,
    hover_text: Option<WidgetText>,
}

impl std::fmt::Debug for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Segment")
            .field("label", &self.label.as_ref().map(|l| l.text().to_string()))
            .field("icon", &self.icon.as_ref().map(|i| i.text().to_string()))
            .field("count", &self.count.as_ref().map(|c| c.text().to_string()))
            .field("dot", &self.dot)
            .field("enabled", &self.enabled)
            .field(
                "hover_text",
                &self.hover_text.as_ref().map(|t| t.text().to_string()),
            )
            .finish()
    }
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            label: None,
            icon: None,
            count: None,
            dot: None,
            enabled: true,
            hover_text: None,
        }
    }
}

impl Segment {
    /// Text-only segment.
    pub fn text(label: impl Into<WidgetText>) -> Self {
        Self {
            label: Some(label.into()),
            ..Self::default()
        }
    }

    /// Icon-only segment. The icon is any [`WidgetText`] — typically a
    /// glyph from [`elegance::glyphs`](crate::glyphs) wrapped in
    /// [`egui::RichText`].
    pub fn icon(icon: impl Into<WidgetText>) -> Self {
        Self {
            icon: Some(icon.into()),
            ..Self::default()
        }
    }

    /// Icon + label segment. The icon precedes the label.
    pub fn icon_text(icon: impl Into<WidgetText>, label: impl Into<WidgetText>) -> Self {
        Self {
            icon: Some(icon.into()),
            label: Some(label.into()),
            ..Self::default()
        }
    }

    /// Add a count badge that follows the label.
    #[inline]
    pub fn count(mut self, count: impl Into<WidgetText>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Add a leading status dot.
    #[inline]
    pub fn dot(mut self, dot: SegmentDot) -> Self {
        self.dot = Some(dot);
        self
    }

    /// Disable the segment. Disabled segments render dimmed and don't
    /// respond to clicks. Default: enabled.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Attach a tooltip shown when the user hovers this segment. Useful
    /// when segments are abbreviations, icons, or domain jargon (e.g.,
    /// "DEV" / "STG" / "PROD") and the short label can't carry the full
    /// meaning. Each segment can have its own; segments without a
    /// `hover_text` show no tooltip.
    #[inline]
    pub fn hover_text(mut self, text: impl Into<WidgetText>) -> Self {
        self.hover_text = Some(text.into());
        self
    }

    fn debug_label(&self) -> String {
        if let Some(l) = &self.label {
            l.text().to_string()
        } else if let Some(i) = &self.icon {
            i.text().to_string()
        } else {
            String::new()
        }
    }
}

/// A row of mutually-exclusive segments sharing a single rounded track.
///
/// Bind to a `&mut usize` index. Click selects; the selected index is
/// updated in place and the response is marked changed. Use
/// [`SegmentedControl::new`] for plain text segments and
/// [`SegmentedControl::from_segments`] when you need icons, dots, counts,
/// or per-segment disabled state.
///
/// ```no_run
/// # use elegance::{SegmentedControl, SegmentedSize};
/// # egui::__run_test_ui(|ui| {
/// let mut selected = 1usize;
/// ui.add(SegmentedControl::new(&mut selected, ["Day", "Week", "Month"]));
/// ui.add(
///     SegmentedControl::new(&mut selected, ["Compact", "Comfortable", "Spacious"])
///         .size(SegmentedSize::Small),
/// );
/// # });
/// ```
///
/// Rich segments with status dots and counts:
///
/// ```no_run
/// # use elegance::{Segment, SegmentDot, SegmentedControl};
/// # egui::__run_test_ui(|ui| {
/// let mut selected = 0usize;
/// ui.add(
///     SegmentedControl::from_segments(
///         &mut selected,
///         [
///             Segment::text("Open").dot(SegmentDot::Amber).count("12"),
///             Segment::text("Triaged").dot(SegmentDot::Neutral).count("84"),
///             Segment::text("Resolved").dot(SegmentDot::Green).count("1,204"),
///             Segment::text("Rejected").dot(SegmentDot::Red).count("31"),
///         ],
///     )
///     .fill(),
/// );
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct SegmentedControl<'a> {
    selection: Selection<'a>,
    segments: Vec<Segment>,
    size: SegmentedSize,
    fill: bool,
    accent: Option<Accent>,
    id_salt: Option<Id>,
}

/// Selection model for [`SegmentedControl`].
///
/// The default constructors ([`SegmentedControl::new`],
/// [`SegmentedControl::from_segments`]) produce a [`Selection::Single`]
/// control where exactly one segment is active at a time. Use
/// [`SegmentedControl::toggles`] to bind a parallel `&mut [bool]` and
/// allow each segment to be toggled on or off independently — useful when
/// "no selection" or "multiple selections" are valid states (e.g.,
/// Server / Client save targets where neither, either, or both can apply).
enum Selection<'a> {
    Single(&'a mut usize),
    Multi(&'a mut [bool]),
}

impl<'a> Selection<'a> {
    fn is_active(&self, i: usize) -> bool {
        match self {
            Selection::Single(idx) => **idx == i,
            Selection::Multi(states) => states.get(i).copied().unwrap_or(false),
        }
    }

    fn click(&mut self, i: usize) -> bool {
        match self {
            Selection::Single(idx) => {
                if **idx != i {
                    **idx = i;
                    true
                } else {
                    false
                }
            }
            Selection::Multi(states) => {
                if let Some(s) = states.get_mut(i) {
                    *s = !*s;
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl<'a> std::fmt::Debug for SegmentedControl<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("SegmentedControl");
        match &self.selection {
            Selection::Single(idx) => {
                d.field("mode", &"single");
                d.field("selected", &**idx);
            }
            Selection::Multi(states) => {
                d.field("mode", &"multi");
                d.field("states", states);
            }
        }
        d.field("segments", &self.segments)
            .field("size", &self.size)
            .field("fill", &self.fill)
            .finish()
    }
}

impl<'a> SegmentedControl<'a> {
    /// Build a text-only single-select segmented control bound to
    /// `selected` (a `&mut usize` index). Click-selects; the index is
    /// always within `0..items.len()` after rendering.
    pub fn new<I, S>(selected: &'a mut usize, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<WidgetText>,
    {
        Self {
            selection: Selection::Single(selected),
            segments: items.into_iter().map(Segment::text).collect(),
            size: SegmentedSize::default(),
            fill: false,
            accent: None,
            id_salt: None,
        }
    }

    /// Build a single-select segmented control from explicit [`Segment`]s.
    /// Use this when you need icons, dots, counts, or disabled states.
    pub fn from_segments(
        selected: &'a mut usize,
        segments: impl IntoIterator<Item = Segment>,
    ) -> Self {
        Self {
            selection: Selection::Single(selected),
            segments: segments.into_iter().collect(),
            size: SegmentedSize::default(),
            fill: false,
            accent: None,
            id_salt: None,
        }
    }

    /// Build a multi-select segmented control: each click toggles the
    /// segment's bool independently, so any combination of segments
    /// (including all on or all off) is a valid state. Visuals match the
    /// single-select track. `states.len()` and `items` should have the
    /// same length; extra labels render as always-off, extra states are
    /// ignored.
    ///
    /// Multi-select also supports click-and-drag paint: press on a
    /// segment to set its new state, then drag across neighbours to
    /// apply the same state to every segment the cursor crosses. Useful
    /// when the caller wants a quick "select a range" gesture without
    /// adding a separate range affordance. Pairs well with
    /// [`SegmentedControl::accent`] for a denser, channel-toggle look.
    ///
    /// ```no_run
    /// # use elegance::SegmentedControl;
    /// # egui::__run_test_ui(|ui| {
    /// let mut targets = [true, false]; // [server_on, client_on]
    /// ui.add(SegmentedControl::toggles(&mut targets, ["Server", "Client"]));
    /// # });
    /// ```
    pub fn toggles<I, S>(states: &'a mut [bool], items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<WidgetText>,
    {
        Self {
            selection: Selection::Multi(states),
            segments: items.into_iter().map(Segment::text).collect(),
            size: SegmentedSize::default(),
            fill: false,
            accent: None,
            id_salt: None,
        }
    }

    /// Pick a size preset. Default: [`SegmentedSize::Medium`].
    #[inline]
    pub fn size(mut self, size: SegmentedSize) -> Self {
        self.size = size;
        self
    }

    /// Force every segment to the same width and stretch the control to
    /// fill the available horizontal space. Useful as a row affordance.
    #[inline]
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Switch to a filled accent style: active segments paint with the
    /// chosen accent colour and white text, with hairline dividers between
    /// segments. The track stays a single rounded rectangle, like a
    /// connected button bar. Without `.accent(...)`, segments use the
    /// default raised-pill style.
    ///
    /// Pairs naturally with [`SegmentedControl::toggles`]; see also the
    /// click-and-drag paint behaviour available in multi-select mode.
    ///
    /// ```no_run
    /// # use elegance::{Accent, SegmentedControl};
    /// # egui::__run_test_ui(|ui| {
    /// let mut channels = [true, false, true, true, false, false, true, false];
    /// ui.add(
    ///     SegmentedControl::toggles(&mut channels, ["1", "2", "3", "4", "5", "6", "7", "8"])
    ///         .accent(Accent::Green),
    /// );
    /// # });
    /// ```
    #[inline]
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Pin a stable id salt for this control.
    ///
    /// Without it, the widget id is drawn from the auto id counter, which
    /// means a sibling widget added or removed conditionally above, or a
    /// parent layout that reflows between frames, can shift the id. When
    /// the id shifts at a stable screen rect, egui prints "Widget rect
    /// ... changed id between passes". Set an explicit salt for pickers
    /// that sit in regions whose siblings flip on user actions (mode
    /// toggles, conditional headers) so per-segment focus state and
    /// id-based diagnostics stay attached to the same control across
    /// frames.
    #[inline]
    pub fn id_salt(mut self, id: impl crate::IdSalt) -> Self {
        self.id_salt = Some(Id::unique(id));
        self
    }
}

struct Prepared {
    icon: Option<Arc<Galley>>,
    label: Option<Arc<Galley>>,
    count: Option<Arc<Galley>>,
    dot: Option<SegmentDot>,
    enabled: bool,
    hover_text: Option<WidgetText>,
    a11y: String,
    natural_w: f32,
    natural_h: f32,
}

const INNER_GAP: f32 = 6.0;
const DOT_SIZE: f32 = 6.0;

fn count_galley(ui: &Ui, text: &str, size: f32) -> Arc<Galley> {
    let rt = egui::RichText::new(text)
        .color(Color32::PLACEHOLDER)
        .size(size)
        .strong();
    egui::WidgetText::from(rt).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::FontId(FontId::monospace(size)),
    )
}

fn dot_color(dot: SegmentDot, theme: &Theme, active: bool) -> Color32 {
    let p = &theme.palette;
    match dot {
        SegmentDot::Neutral => {
            if active {
                p.focus
            } else {
                p.text_faint
            }
        }
        SegmentDot::Sky => p.focus,
        SegmentDot::Amber => p.warning,
        SegmentDot::Red => p.danger,
        SegmentDot::Green => p.success,
    }
}

impl<'a> Widget for SegmentedControl<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        let accent = self.accent;
        let filled_style = accent.is_some();
        let multi = matches!(self.selection, Selection::Multi(_));
        // Drag-paint applies the start segment's new state to every segment
        // the cursor passes over while the primary button is held. Reserved
        // for multi-select; single-select dragging would scrub the
        // selection across segments which feels surprising.
        let drag_paint = multi;

        let size = self.size;
        // Filled style snaps segments to the inside of the 1 px border so
        // the bar reads as a single connected control. The default style
        // keeps the per-size track_pad inset for the raised-pill look.
        let track_pad = if filled_style { 1.0 } else { size.track_pad() };
        let pad_x = size.pad_x();
        let pad_y = size.pad_y();
        let font_size = size.font_size(&theme);
        let icon_size = size.icon_size(&theme);
        let count_size = (font_size - 1.5).max(10.0);
        let count_h = size.count_height();

        // 1. Lay out each segment's content.
        let mut prepared: Vec<Prepared> = Vec::with_capacity(self.segments.len());
        for seg in self.segments.iter_mut() {
            let icon = seg
                .icon
                .as_ref()
                .map(|t| placeholder_galley(ui, t.text(), icon_size, false, f32::INFINITY));
            let label = seg
                .label
                .as_ref()
                .map(|t| placeholder_galley(ui, t.text(), font_size, true, f32::INFINITY));
            let count = seg
                .count
                .as_ref()
                .map(|t| count_galley(ui, t.text(), count_size));

            let mut content_w = 0.0_f32;
            let mut content_h = font_size;
            if seg.dot.is_some() {
                content_w += DOT_SIZE;
                content_h = content_h.max(DOT_SIZE);
            }
            if let Some(g) = &icon {
                if content_w > 0.0 {
                    content_w += INNER_GAP;
                }
                content_w += g.size().x;
                content_h = content_h.max(g.size().y);
            }
            if let Some(g) = &label {
                if content_w > 0.0 {
                    content_w += INNER_GAP;
                }
                content_w += g.size().x;
                content_h = content_h.max(g.size().y);
            }
            if let Some(g) = &count {
                if content_w > 0.0 {
                    content_w += INNER_GAP;
                }
                let pill_w = (g.size().x + 10.0).max(count_h);
                content_w += pill_w;
                content_h = content_h.max(count_h);
            }

            prepared.push(Prepared {
                icon,
                label,
                count,
                dot: seg.dot,
                enabled: seg.enabled,
                hover_text: seg.hover_text.take(),
                a11y: seg.debug_label(),
                natural_w: pad_x * 2.0 + content_w,
                natural_h: pad_y * 2.0 + content_h,
            });
        }

        // 2. Resolve cell widths.
        let segment_h = prepared
            .iter()
            .map(|s| s.natural_h)
            .fold(font_size + pad_y * 2.0, f32::max);

        let cell_widths: Vec<f32> = if self.fill && !prepared.is_empty() {
            let avail = (ui.available_width() - track_pad * 2.0).max(0.0);
            let max_natural = prepared.iter().map(|s| s.natural_w).fold(0.0_f32, f32::max);
            let cell_w = (avail / prepared.len() as f32).max(max_natural);
            prepared.iter().map(|_| cell_w).collect()
        } else {
            prepared.iter().map(|s| s.natural_w).collect()
        };

        let total_w = track_pad * 2.0 + cell_widths.iter().sum::<f32>();
        let total_h = track_pad * 2.0 + segment_h;

        // 3. Allocate the outer track rect. The base id seeds every
        //    per-segment interact id, so multiple SegmentedControls in the
        //    same parent never collide. When the caller pinned an
        //    `id_salt`, we honour it; otherwise we fall back to the auto
        //    id from `allocate_space`, matching the previous default.
        let desired = Vec2::new(total_w, total_h);
        let (auto_id, frame_rect) = ui.allocate_space(desired);
        let base_id = self
            .id_salt
            .map(|salt| ui.make_persistent_id(salt))
            .unwrap_or(auto_id);
        let track_rect = ui.layout().align_size_within_rect(desired, frame_rect);
        let mut response = ui.interact(frame_rect, base_id, Sense::hover());
        response.set_intrinsic_size(desired);

        // 4. Allocate per-segment interact rects (each is its own focus target).
        let mut x = track_rect.min.x + track_pad;
        let segment_y = track_rect.min.y + track_pad;
        let mut cell_rects: Vec<Rect> = Vec::with_capacity(prepared.len());
        let mut cell_responses: Vec<Response> = Vec::with_capacity(prepared.len());
        let mut selection_changed = false;
        let mut paint_target: Option<bool> = None;
        for (i, prep) in prepared.iter_mut().enumerate() {
            let cell_rect =
                Rect::from_min_size(pos2(x, segment_y), Vec2::new(cell_widths[i], segment_h));
            x += cell_widths[i];
            let sense = if !prep.enabled {
                Sense::hover()
            } else if drag_paint {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            let mut cell_resp = ui.interact(cell_rect, base_id.with(("seg", i)), sense);
            if prep.enabled {
                if cell_resp.clicked() && self.selection.click(i) {
                    selection_changed = true;
                }
                if drag_paint && cell_resp.drag_started() && self.selection.click(i) {
                    selection_changed = true;
                }
                // Anchor the paint target to the originating segment so the
                // drag never produces mixed flips: the start segment's new
                // state is what every subsequent segment under the cursor
                // gets set to.
                if drag_paint && (cell_resp.drag_started() || cell_resp.dragged()) {
                    paint_target = Some(self.selection.is_active(i));
                }
            }
            if let Some(text) = prep.hover_text.take() {
                cell_resp = cell_resp.on_hover_text(text);
            }
            cell_rects.push(cell_rect);
            cell_responses.push(cell_resp);
        }

        // Apply the paint target to any segment under the cursor. We
        // hit-test against cell_rects so the drag survives crossing the
        // gap between segments.
        if let (Some(target), Some(pos)) = (paint_target, ui.ctx().pointer_interact_pos())
            && let Selection::Multi(states) = &mut self.selection
        {
            for (i, cell_rect) in cell_rects.iter().enumerate() {
                if !prepared[i].enabled || !cell_rect.contains(pos) {
                    continue;
                }
                if let Some(s) = states.get_mut(i)
                    && *s != target
                {
                    *s = target;
                    selection_changed = true;
                }
            }
        }

        // Per-segment "is active" lookup, branched on selection model.
        // Disabled segments never paint as active, even if the binding
        // says they should.
        let is_active = |i: usize| -> bool {
            i < prepared.len() && prepared[i].enabled && self.selection.is_active(i)
        };
        let hovered_idx = cell_responses
            .iter()
            .zip(prepared.iter())
            .position(|(r, prep)| prep.enabled && r.hovered());

        // 5. Paint.
        if ui.is_rect_visible(track_rect) {
            let track_radius = CornerRadius::same(size.track_radius());
            let n = cell_rects.len();

            if filled_style {
                let acc = accent.expect("filled_style implies accent set");
                let accent_fill = p.accent_fill(acc);

                // Track background. The border is painted on top after the
                // fills so the rounded outline always reads crisp.
                ui.painter()
                    .rect_filled(track_rect, track_radius, p.input_bg);

                // Outer corner radius for first/last active segments. Sits
                // one pixel inside the track border so accent fills tuck
                // flush against the curved corner.
                let outer = (size.track_radius() as i32 - 1).max(0) as u8;
                let corners_for = |i: usize| -> CornerRadius {
                    let mut r = CornerRadius::same(0);
                    if i == 0 {
                        r.nw = outer;
                        r.sw = outer;
                    }
                    if i + 1 == n {
                        r.ne = outer;
                        r.se = outer;
                    }
                    r
                };

                // Hover overlay (non-active only).
                if let Some(h) = hovered_idx
                    && !is_active(h)
                {
                    let hover_fill = with_alpha(p.text, if p.is_dark { 14 } else { 18 });
                    ui.painter()
                        .rect_filled(cell_rects[h], corners_for(h), hover_fill);
                }

                // Active fills.
                for (i, cell_rect) in cell_rects.iter().enumerate() {
                    if !is_active(i) {
                        continue;
                    }
                    ui.painter()
                        .rect_filled(*cell_rect, corners_for(i), accent_fill);
                }

                // Hairline dividers between adjacent segments. The colour
                // depends on what's on either side: between two inactive
                // non-hovered cells we use the standard border tint; where
                // two active cells meet we cut through the accent with the
                // track fill so the bar reads as separate buttons rather
                // than one green block. Mixed pairs (active vs inactive,
                // or anything next to a hovered cell) need no divider
                // because the colour change already separates them.
                for (i, cell) in cell_rects.iter().enumerate().skip(1) {
                    let left_active = is_active(i - 1);
                    let right_active = is_active(i);
                    let left_hovered = hovered_idx == Some(i - 1);
                    let right_hovered = hovered_idx == Some(i);
                    let color = if left_active && right_active {
                        p.input_bg
                    } else if left_active || right_active || left_hovered || right_hovered {
                        continue;
                    } else {
                        with_alpha(p.border, 220)
                    };
                    let div_x = cell.min.x.round() - 0.5;
                    ui.painter().line_segment(
                        [pos2(div_x, cell.min.y), pos2(div_x, cell.max.y)],
                        Stroke::new(1.0, color),
                    );
                }

                // Track border on top of everything.
                ui.painter().rect_stroke(
                    track_rect,
                    track_radius,
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );
            } else {
                // ── Raised-pill style (default) ─────────────────────────
                ui.painter().rect(
                    track_rect,
                    track_radius,
                    p.input_bg,
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );

                // Dividers between adjacent inactive, non-hovered segments.
                for (i, cell) in cell_rects.iter().enumerate().skip(1) {
                    let left_busy = is_active(i - 1) || hovered_idx == Some(i - 1);
                    let right_busy = is_active(i) || hovered_idx == Some(i);
                    if left_busy || right_busy {
                        continue;
                    }
                    let div_x = cell.min.x.round() - 0.5;
                    let dy = (segment_h * 0.30).min(8.0);
                    ui.painter().line_segment(
                        [pos2(div_x, cell.min.y + dy), pos2(div_x, cell.max.y - dy)],
                        Stroke::new(1.0, with_alpha(p.border, 200)),
                    );
                }

                let segment_radius = CornerRadius::same(size.segment_radius());

                // Hovered fill (drawn before active, so an active+hover doesn't double-stack).
                if let Some(h) = hovered_idx
                    && !is_active(h)
                {
                    let hover_fill = with_alpha(p.text, if p.is_dark { 14 } else { 18 });
                    ui.painter().rect(
                        cell_rects[h].shrink(0.5),
                        segment_radius,
                        hover_fill,
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                }

                // Active fill(s): drop-shadow + card-coloured pill on every
                // active segment. Multi-select can have several at once.
                for (i, cell_rect) in cell_rects.iter().enumerate().take(prepared.len()) {
                    if !is_active(i) {
                        continue;
                    }
                    let cell = cell_rect.shrink(0.5);
                    let shadow = cell.translate(Vec2::new(0.0, 1.0));
                    ui.painter().rect(
                        shadow,
                        segment_radius,
                        with_alpha(Color32::BLACK, if p.is_dark { 70 } else { 28 }),
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                    ui.painter().rect(
                        cell,
                        segment_radius,
                        p.card,
                        Stroke::new(1.0, p.border),
                        StrokeKind::Inside,
                    );
                }
            }

            // Per-segment content (shared between styles).
            for (i, prep) in prepared.iter().enumerate() {
                let cell_rect = cell_rects[i];
                let active = is_active(i);
                let hovered = hovered_idx == Some(i) && !active;

                let text_color = if !prep.enabled {
                    with_alpha(p.text_faint, 160)
                } else if filled_style && active {
                    Color32::WHITE
                } else if active || hovered {
                    p.text
                } else {
                    p.text_muted
                };

                // Recompute content width to centre.
                let count_pill_w = prep
                    .count
                    .as_ref()
                    .map(|g| (g.size().x + 10.0).max(count_h));
                let mut content_w = 0.0_f32;
                if prep.dot.is_some() {
                    content_w += DOT_SIZE;
                }
                if let Some(g) = &prep.icon {
                    if content_w > 0.0 {
                        content_w += INNER_GAP;
                    }
                    content_w += g.size().x;
                }
                if let Some(g) = &prep.label {
                    if content_w > 0.0 {
                        content_w += INNER_GAP;
                    }
                    content_w += g.size().x;
                }
                if let Some(w) = count_pill_w {
                    if content_w > 0.0 {
                        content_w += INNER_GAP;
                    }
                    content_w += w;
                }

                let mut cx = cell_rect.center().x - content_w * 0.5;
                let cy = cell_rect.center().y;

                if let Some(dot) = prep.dot {
                    let mut col = dot_color(dot, &theme, active);
                    if !prep.enabled {
                        col = with_alpha(col, 120);
                    }
                    ui.painter()
                        .circle_filled(pos2(cx + DOT_SIZE * 0.5, cy), DOT_SIZE * 0.5, col);
                    cx += DOT_SIZE;
                }
                if let Some(icon) = &prep.icon {
                    if cx > cell_rect.center().x - content_w * 0.5 {
                        cx += INNER_GAP;
                    }
                    let pos = pos2(cx, cy - icon.size().y * 0.5);
                    ui.painter().galley(pos, icon.clone(), text_color);
                    cx += icon.size().x;
                }
                if let Some(label) = &prep.label {
                    if cx > cell_rect.center().x - content_w * 0.5 {
                        cx += INNER_GAP;
                    }
                    let pos = pos2(cx, cy - label.size().y * 0.5);
                    ui.painter().galley(pos, label.clone(), text_color);
                    cx += label.size().x;
                }
                if let (Some(g), Some(pill_w)) = (&prep.count, count_pill_w) {
                    if cx > cell_rect.center().x - content_w * 0.5 {
                        cx += INNER_GAP;
                    }
                    let pill_rect = Rect::from_min_size(
                        pos2(cx, cy - count_h * 0.5),
                        Vec2::new(pill_w, count_h),
                    );
                    let (pill_bg, pill_fg) = if active {
                        (with_alpha(p.focus, 50), p.focus)
                    } else if !prep.enabled {
                        (with_alpha(p.text_faint, 35), with_alpha(p.text_faint, 200))
                    } else {
                        (with_alpha(p.text_muted, 45), p.text_muted)
                    };
                    ui.painter().rect(
                        pill_rect,
                        CornerRadius::same(99),
                        pill_bg,
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                    let text_pos = pos2(
                        pill_rect.center().x - g.size().x * 0.5,
                        pill_rect.center().y - g.size().y * 0.5,
                    );
                    ui.painter().galley(text_pos, g.clone(), pill_fg);
                }
            }
        }

        // 6. Per-segment a11y info. Single-select reads as RadioButton(s)
        // in a RadioGroup; multi-select reads as Checkbox(es) in a Group
        // so a screen reader announces the right semantics.
        let segment_role = if multi {
            Role::CheckBox
        } else {
            Role::RadioButton
        };
        let group_role = if multi {
            Role::Unknown
        } else {
            Role::RadioGroup
        };
        for (i, (cell_resp, prep)) in cell_responses.iter().zip(prepared.iter()).enumerate() {
            let label = prep.a11y.clone();
            let enabled = prep.enabled;
            let selected = is_active(i);
            cell_resp.widget_info(|| WidgetInfo::selected(segment_role, enabled, selected, &label));
        }
        response.widget_info(|| WidgetInfo::labeled(group_role, true, "segmented control"));
        if selection_changed {
            response.mark_changed();
        }
        response
    }
}
