//! Color picker — a swatch-button trigger that opens a themed popover.
//!
//! Bind to a [`Color32`] and drop it into any layout. The trigger renders
//! as a small swatch plus the current hex value; clicking opens a popover
//! containing any combination of:
//!
//! * a curated palette grid,
//! * a recents row that auto-tracks the user's last picks,
//! * a continuous saturation/value plane plus hue slider,
//! * an alpha slider, and
//! * a hex input.
//!
//! ```no_run
//! # use elegance::ColorPicker;
//! # use egui::Color32;
//! # egui::__run_test_ui(|ui| {
//! let mut accent = Color32::from_rgb(0x38, 0xbd, 0xf8);
//! ui.add(
//!     ColorPicker::new("brand_accent", &mut accent)
//!         .palette(ColorPicker::default_palette())
//!         .continuous(true),
//! );
//! # });
//! ```

use egui::{
    Color32, CornerRadius, Event, EventFilter, FontSelection, Id, Key, Pos2, Rect, Response, Role,
    Sense, Shape, Stroke, StrokeKind, TextEdit, Ui, Vec2, Widget, WidgetInfo, WidgetText,
    ecolor::{Hsva, HsvaGamma},
    epaint::Mesh,
    lerp, pos2, vec2,
};

use crate::commit::ResponseCommitExt;
use crate::popover::{Popover, PopoverSide};
use crate::theme::{Theme, with_alpha};

const HEX_BUF_SUFFIX: &str = "color_picker::hex_buf";
const HSV_CACHE_SUFFIX: &str = "color_picker::hsv_cache";
const RECENTS_SUFFIX: &str = "color_picker::recents";
const KEY_RUN_SUFFIX: &str = "color_picker::key_run";

/// A click-to-open color picker bound to a [`Color32`].
///
/// The widget paints a compact swatch-and-hex trigger; the picker UI lives
/// in a [`Popover`](crate::Popover). Configure which sub-controls the
/// popover shows via the builder. By default the popover contains a
/// continuous picker (saturation/value plane plus hue slider), an alpha
/// slider, and a hex input.
#[must_use = "Add with `ui.add(...)`."]
pub struct ColorPicker<'a> {
    id_salt: egui::IdSalt,
    color: &'a mut Color32,
    label: Option<WidgetText>,
    palette: Vec<Color32>,
    palette_columns: usize,
    show_continuous: bool,
    show_alpha: bool,
    show_hex_input: bool,
    show_hex_label: bool,
    show_recents: bool,
    recents_max: usize,
    side: PopoverSide,
}

impl<'a> std::fmt::Debug for ColorPicker<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColorPicker")
            .field("id_salt", &self.id_salt)
            .field("color", &*self.color)
            .field("palette_len", &self.palette.len())
            .field("palette_columns", &self.palette_columns)
            .field("show_continuous", &self.show_continuous)
            .field("show_alpha", &self.show_alpha)
            .field("show_hex_input", &self.show_hex_input)
            .field("show_hex_label", &self.show_hex_label)
            .field("show_recents", &self.show_recents)
            .field("recents_max", &self.recents_max)
            .field("side", &self.side)
            .finish()
    }
}

impl<'a> ColorPicker<'a> {
    /// Create a color picker keyed by `id_salt` and bound to `color`.
    /// Defaults: continuous picker on, alpha slider on, hex input on,
    /// recents tracked, opens below the trigger.
    pub fn new(id_salt: impl crate::IdSalt, color: &'a mut Color32) -> Self {
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            color,
            label: None,
            palette: Vec::new(),
            palette_columns: 10,
            show_continuous: true,
            show_alpha: true,
            show_hex_input: true,
            show_hex_label: true,
            show_recents: true,
            recents_max: 10,
            side: PopoverSide::Bottom,
        }
    }

    /// Show a label above the trigger.
    ///
    /// The label also names the picker for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Supply a curated palette grid above the recents row. When unset
    /// (the default) no palette is shown. Use [`ColorPicker::default_palette`]
    /// for a 30-swatch starter palette inspired by Tailwind.
    pub fn palette(mut self, palette: impl IntoIterator<Item = Color32>) -> Self {
        self.palette = palette.into_iter().collect();
        self
    }

    /// Number of columns in the palette grid. Default: 10.
    #[inline]
    pub fn palette_columns(mut self, n: usize) -> Self {
        self.palette_columns = n.max(1);
        self
    }

    /// Toggle the continuous saturation/value plane and hue slider. Default: on.
    #[inline]
    pub fn continuous(mut self, on: bool) -> Self {
        self.show_continuous = on;
        self
    }

    /// Toggle the alpha slider. Default: on. Disable for opaque-only colors.
    #[inline]
    pub fn alpha(mut self, on: bool) -> Self {
        self.show_alpha = on;
        self
    }

    /// Toggle the hex input row inside the popover. Default: on.
    #[inline]
    pub fn hex_input(mut self, on: bool) -> Self {
        self.show_hex_input = on;
        self
    }

    /// Show or hide the hex string next to the swatch on the trigger
    /// button. Default: shown.
    #[inline]
    pub fn hex_label(mut self, on: bool) -> Self {
        self.show_hex_label = on;
        self
    }

    /// Toggle the recents row. The recent picks are kept in egui's
    /// temporary memory, keyed by the picker's `id_salt` within its `Ui`, so
    /// they last for the session. Default: on.
    #[inline]
    pub fn recents(mut self, on: bool) -> Self {
        self.show_recents = on;
        self
    }

    /// Maximum number of recent picks remembered. Default: 10.
    #[inline]
    pub fn recents_max(mut self, n: usize) -> Self {
        self.recents_max = n.max(1);
        self
    }

    /// Which side of the trigger the popover opens on. Default: below.
    #[inline]
    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    /// A 30-swatch curated palette: a row of neutrals, a row of cool
    /// accents, and a row of warm accents. Pass to [`ColorPicker::palette`].
    pub fn default_palette() -> Vec<Color32> {
        vec![
            // Neutrals.
            Color32::from_rgb(0xe2, 0xe8, 0xf0),
            Color32::from_rgb(0x94, 0xa3, 0xb8),
            Color32::from_rgb(0x64, 0x74, 0x8b),
            Color32::from_rgb(0x47, 0x55, 0x69),
            Color32::from_rgb(0x33, 0x41, 0x55),
            Color32::from_rgb(0x1e, 0x29, 0x3b),
            Color32::from_rgb(0x0f, 0x17, 0x2a),
            Color32::from_rgb(0x0b, 0x11, 0x20),
            Color32::from_rgb(0x11, 0x1a, 0x2e),
            Color32::from_rgb(0x18, 0x24, 0x38),
            // Cools.
            Color32::from_rgb(0x38, 0xbd, 0xf8),
            Color32::from_rgb(0x0e, 0xa5, 0xe9),
            Color32::from_rgb(0x25, 0x63, 0xeb),
            Color32::from_rgb(0x63, 0x66, 0xf1),
            Color32::from_rgb(0xc0, 0x84, 0xfc),
            Color32::from_rgb(0xa8, 0x55, 0xf7),
            Color32::from_rgb(0xf4, 0x72, 0xb6),
            Color32::from_rgb(0xfb, 0x71, 0x85),
            Color32::from_rgb(0xf8, 0x71, 0x71),
            Color32::from_rgb(0xef, 0x44, 0x44),
            // Warms / greens.
            Color32::from_rgb(0xf5, 0x9e, 0x0b),
            Color32::from_rgb(0xfb, 0xbf, 0x24),
            Color32::from_rgb(0xfa, 0xcc, 0x15),
            Color32::from_rgb(0xd9, 0x77, 0x06),
            Color32::from_rgb(0xa3, 0xe6, 0x35),
            Color32::from_rgb(0x86, 0xef, 0xac),
            Color32::from_rgb(0x4a, 0xde, 0x80),
            Color32::from_rgb(0x22, 0xc5, 0x5e),
            Color32::from_rgb(0x14, 0xb8, 0xa6),
            Color32::from_rgb(0x22, 0xd3, 0xee),
        ]
    }
}

impl<'a> Widget for ColorPicker<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let label = self.label.clone();
        let picker_id = ui.scope_id().with_salt(self.id_salt);
        let side = self.side;
        let show_palette = !self.palette.is_empty();
        let show_recents = self.show_recents;
        let show_continuous = self.show_continuous;
        let show_alpha = self.show_alpha;
        let show_hex_input = self.show_hex_input;
        let palette = self.palette.clone();
        let palette_columns = self.palette_columns;
        let recents_max = self.recents_max;

        ui.vertical(|ui| {
            if let Some(l) = &label {
                let rich = egui::RichText::new(l.text())
                    .color(p.text_muted)
                    .size(t.label);
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                ui.add_space(2.0);
            }

            let mut response =
                paint_trigger(ui, &theme, picker_id, *self.color, self.show_hex_label);

            // `discrete_pick` is set when the user makes a deliberate choice
            // (palette swatch, recents swatch, hex committed). `continuous_pick`
            // is set while a continuous control (SV plane / hue / alpha) is
            // being dragged. `settled` says how a continuous control finished
            // this frame, if it did. Recents are pushed on discrete picks and
            // on continuous settles, but not on each intermediate frame of a
            // drag — otherwise a single SV-plane sweep would carpet-bomb the
            // recents row.
            let mut discrete_pick: Option<Color32> = None;
            let mut continuous_pick: Option<Color32> = None;
            let mut settled = Settled::No;

            // Close an open keyboard run once its surface has lost focus, so
            // the next nudge opens a fresh recents entry rather than amending
            // one the user has since moved on from.
            let focused = ui.ctx().memory(|m| m.focused());
            if let Some(surface) = ui.ctx().data(|d| d.get_temp::<Id>(key_run_id(picker_id)))
                && Some(surface) != focused
            {
                end_key_run(ui.ctx(), picker_id);
            }

            Popover::new("elegance::color_picker")
                .side(side)
                .arrow(false)
                .min_width(248.0)
                .show(&response, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 8.0);

                    let cur = *self.color;
                    let mut hsv = current_hsv(ui.ctx(), picker_id, cur);

                    if show_palette {
                        ui.add(small_label(&theme, "Theme palette"));
                        if let Some(picked) =
                            paint_palette_grid(ui, &theme, cur, &palette, palette_columns)
                        {
                            discrete_pick = Some(picked);
                        }
                    }

                    if show_recents {
                        ui.add(small_label(&theme, "Recent"));
                        let recents: Vec<Color32> = ui
                            .ctx()
                            .data(|d| d.get_temp(recents_id(picker_id)))
                            .unwrap_or_default();
                        if let Some(picked) = paint_recents_row(
                            ui,
                            &theme,
                            cur,
                            &recents,
                            palette_columns,
                            recents_max,
                        ) {
                            discrete_pick = Some(picked);
                        }
                    }

                    if show_continuous {
                        let (changed, how) = paint_sv_plane(ui, &theme, &mut hsv);
                        if changed {
                            continuous_pick = Some(Color32::from(hsv));
                        }
                        if how != Settled::No {
                            settled = how;
                        }
                        let (changed, how) = paint_hue_strip(ui, &theme, &mut hsv);
                        if changed {
                            continuous_pick = Some(Color32::from(hsv));
                        }
                        if how != Settled::No {
                            settled = how;
                        }
                    }

                    if show_alpha {
                        let (changed, how) = paint_alpha_slider(ui, &theme, &mut hsv);
                        if changed {
                            continuous_pick = Some(Color32::from(hsv));
                        }
                        if how != Settled::No {
                            settled = how;
                        }
                    }

                    if show_hex_input
                        && let Some(picked) = paint_hex_input(ui, &theme, picker_id, cur)
                    {
                        discrete_pick = Some(picked);
                    }

                    if let Some(next) = discrete_pick.or(continuous_pick) {
                        // For discrete picks the chosen color is authoritative;
                        // re-derive HSV so the cache matches. For continuous
                        // picks the mutated HSV is authoritative; preserve it
                        // so dragging value to zero doesn't collapse the hue.
                        if discrete_pick.is_some() {
                            hsv = HsvaGamma::from(next);
                        }
                        set_hsv(ui.ctx(), picker_id, hsv);
                    }
                });

            let next_color = discrete_pick.or(continuous_pick);
            if let Some(picked) = next_color
                && picked != *self.color
            {
                *self.color = picked;
                response.mark_changed();
            }
            // Push to recents on a deliberate commit. A discrete pick and a
            // pointer release are each one whole gesture, so each gets its own
            // entry; a keyboard run gets one entry between all of its presses.
            // The current bound color is the right value to record either way.
            let settled = if discrete_pick.is_some() {
                Settled::Gesture
            } else {
                settled
            };
            match settled {
                Settled::No => {}
                Settled::Gesture => {
                    end_key_run(ui.ctx(), picker_id);
                    push_recent(ui.ctx(), picker_id, *self.color, recents_max);
                }
                Settled::KeyRun(surface) => {
                    record_key_run(ui.ctx(), picker_id, surface, *self.color, recents_max);
                }
            }

            let label_text = label
                .as_ref()
                .map(|l| l.text().to_string())
                .unwrap_or_else(|| "Color".to_string());
            response.widget_info(|| WidgetInfo::labeled(Role::ColorWell, true, &label_text));
            response
        })
        .inner
    }
}

// --- trigger ---------------------------------------------------------------

fn paint_trigger(
    ui: &mut Ui,
    theme: &Theme,
    picker_id: Id,
    color: Color32,
    show_hex_label: bool,
) -> Response {
    let p = &theme.palette;
    let t = &theme.typography;

    let pad_outer = vec2(5.0, 5.0);
    let swatch_size = 22.0;
    let inner_gap = 8.0;

    let hex_text = format_hex(color);
    let galley = if show_hex_label {
        Some(crate::theme::placeholder_galley(
            ui,
            &hex_text,
            t.small,
            false,
            f32::INFINITY,
        ))
    } else {
        None
    };

    let hex_w = galley.as_ref().map(|g| g.size().x).unwrap_or(0.0);
    let hex_h = galley.as_ref().map(|g| g.size().y).unwrap_or(0.0);
    let content_w = swatch_size
        + galley
            .as_ref()
            .map(|_| inner_gap + hex_w + 5.0)
            .unwrap_or(0.0);
    let content_h = swatch_size.max(hex_h);

    let desired = vec2(content_w + 2.0 * pad_outer.x, content_h + 2.0 * pad_outer.y);

    let id = picker_id.with("trigger");
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let response = ui.interact(rect, id, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(theme.control_radius as u8);
        let stroke_color = if response.has_focus() {
            with_alpha(p.focus, 200)
        } else if response.hovered() {
            p.text_muted
        } else {
            p.border
        };
        painter.rect(
            rect,
            radius,
            p.input_bg,
            Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        let swatch_rect = Rect::from_min_size(
            pos2(
                rect.min.x + pad_outer.x,
                rect.center().y - swatch_size * 0.5,
            ),
            Vec2::splat(swatch_size),
        );
        paint_swatch(painter, swatch_rect, color, 4, p.is_dark, p.input_bg);

        if let Some(g) = galley {
            let text_x = swatch_rect.max.x + inner_gap;
            let text_y = rect.center().y - g.size().y * 0.5;
            painter.galley(pos2(text_x, text_y), g, p.text);
        }
    }

    response
}

// --- palette grid ----------------------------------------------------------

fn paint_palette_grid(
    ui: &mut Ui,
    theme: &Theme,
    current: Color32,
    palette: &[Color32],
    columns: usize,
) -> Option<Color32> {
    let p = &theme.palette;
    let gap = 5.0;
    let avail = ui.available_width();
    let cols = columns.max(1);
    let cell = ((avail - gap * (cols - 1) as f32) / cols as f32).max(8.0);
    let rows = palette.len().div_ceil(cols);
    let total_h = rows as f32 * cell + (rows.saturating_sub(1)) as f32 * gap;
    let (rect, _) = ui.allocate_exact_size(vec2(avail, total_h), Sense::hover());

    let mut picked = None;
    for (i, color) in palette.iter().copied().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let x = rect.min.x + col as f32 * (cell + gap);
        let y = rect.min.y + row as f32 * (cell + gap);
        let cell_rect = Rect::from_min_size(pos2(x, y), Vec2::splat(cell));
        let id = ui
            .scope_id()
            .with(("color_picker_palette", i, color.r(), color.g(), color.b()));
        let resp = ui.interact(cell_rect, id, Sense::click());
        let selected = color == current;
        paint_palette_swatch(ui, cell_rect, color, selected, &resp, p.is_dark, theme);
        if resp.clicked() {
            picked = Some(color);
        }
    }
    picked
}

fn paint_recents_row(
    ui: &mut Ui,
    theme: &Theme,
    current: Color32,
    recents: &[Color32],
    columns: usize,
    max: usize,
) -> Option<Color32> {
    let p = &theme.palette;
    let cols = columns.max(1).max(max);
    let gap = 5.0;
    let avail = ui.available_width();
    let cell = ((avail - gap * (cols - 1) as f32) / cols as f32).max(8.0);
    let total_h = cell;
    let (rect, _) = ui.allocate_exact_size(vec2(avail, total_h), Sense::hover());

    let mut picked = None;
    for col in 0..cols {
        let x = rect.min.x + col as f32 * (cell + gap);
        let y = rect.min.y;
        let cell_rect = Rect::from_min_size(pos2(x, y), Vec2::splat(cell));
        if let Some(color) = recents.get(col).copied() {
            let id = ui.scope_id().with(("color_picker_recents", col));
            let resp = ui.interact(cell_rect, id, Sense::click());
            let selected = color == current;
            paint_palette_swatch(ui, cell_rect, color, selected, &resp, p.is_dark, theme);
            if resp.clicked() {
                picked = Some(color);
            }
        } else {
            paint_recents_empty(ui, cell_rect, theme);
        }
    }
    picked
}

fn paint_palette_swatch(
    ui: &Ui,
    rect: Rect,
    color: Color32,
    selected: bool,
    resp: &Response,
    is_dark: bool,
    theme: &Theme,
) {
    let painter = ui.painter();
    let radius_n: u8 = 4;
    let radius = CornerRadius::same(radius_n);
    if color.is_opaque() {
        painter.rect_filled(rect, radius, color);
    } else {
        paint_checkers(painter, rect, radius);
        painter.rect_filled(rect, radius, color);
        paint_rounded_corner_mask(painter, rect, radius_n as f32, theme.palette.card);
    }
    let inset_color = if is_dark {
        Color32::from_rgba_unmultiplied(15, 23, 42, 130)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 60)
    };
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, inset_color),
        StrokeKind::Inside,
    );

    if selected {
        let outer = rect.expand(2.0);
        let painter = ui.painter();
        painter.rect_stroke(
            outer,
            CornerRadius::same(5),
            Stroke::new(2.0, theme.palette.focus),
            StrokeKind::Outside,
        );
    } else if resp.hovered() {
        let outer = rect.expand(1.0);
        ui.painter().rect_stroke(
            outer,
            CornerRadius::same(5),
            Stroke::new(1.0, with_alpha(theme.palette.text, 110)),
            StrokeKind::Outside,
        );
    }
}

fn paint_recents_empty(ui: &Ui, rect: Rect, theme: &Theme) {
    let p = &theme.palette;
    let painter = ui.painter();
    let radius = CornerRadius::same(4);
    painter.rect_filled(rect, radius, p.input_bg);
    paint_dashed_rect(
        painter,
        rect,
        radius,
        with_alpha(p.text_faint, 160),
        1.0,
        3.0,
        3.0,
    );
}

fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: Rect,
    _radius: CornerRadius,
    color: Color32,
    width: f32,
    dash: f32,
    gap: f32,
) {
    let stroke = Stroke::new(width, color);
    let segments = |from: Pos2, to: Pos2| -> Vec<[Pos2; 2]> {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return Vec::new();
        }
        let step = dash + gap;
        let n = (len / step).floor() as usize;
        let mut out = Vec::with_capacity(n + 1);
        let mut t = 0.0_f32;
        while t < len {
            let t_end = (t + dash).min(len);
            let a = pos2(from.x + dx * (t / len), from.y + dy * (t / len));
            let b = pos2(from.x + dx * (t_end / len), from.y + dy * (t_end / len));
            out.push([a, b]);
            t += step;
        }
        out
    };
    for seg in segments(rect.left_top(), rect.right_top()) {
        painter.line_segment(seg, stroke);
    }
    for seg in segments(rect.right_top(), rect.right_bottom()) {
        painter.line_segment(seg, stroke);
    }
    for seg in segments(rect.right_bottom(), rect.left_bottom()) {
        painter.line_segment(seg, stroke);
    }
    for seg in segments(rect.left_bottom(), rect.left_top()) {
        painter.line_segment(seg, stroke);
    }
}

// --- continuous picker -----------------------------------------------------

/// One arrow-key step on a `0..=1` colour channel, as a fraction of the
/// channel. `Shift` multiplies it by ten, the same 10x the crate's sliders use.
/// How a continuous surface settled on a value this pass, as the recents row
/// needs to see it.
///
/// A pointer gesture settles once, on release, so it stands alone. A key press
/// settles the instant it lands — right for
/// [`committed()`](crate::ResponseCommitExt::committed), whose job is to mark
/// every value worth reacting to, but wrong for a history list: at a typical
/// auto-repeat rate a held arrow key would push tens of near-identical entries
/// per second and evict everything the user had collected. Carrying the
/// surface's id lets a run of presses amend the one entry it opened.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Settled {
    /// Not this pass.
    No,
    /// A whole gesture: a pointer release, or a discrete pick.
    Gesture,
    /// One press in a keyboard run on the surface with this id.
    KeyRun(Id),
}

const CHANNEL_STEP: f32 = 0.01;

/// One arrow-key step on hue, which reads in degrees rather than as a fraction
/// of an arbitrary range: one press is one degree, `Shift` a ten-degree jump.
const HUE_STEP: f32 = 1.0 / 360.0;

/// Focus ring for a continuous surface, drawn just outside it.
///
/// Outside rather than as an accented border because these surfaces paint
/// saturated gradients edge to edge: an inset ring on the 14pt hue strip
/// disappears into the rainbow behind it. This sits in the 8pt gap between
/// controls, on the popover surface, where it reads against every theme — and
/// it is where [`Knob`](crate::Knob) already puts its ring.
fn paint_focus_ring(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    p: &crate::theme::Palette,
    response: &Response,
) {
    if !response.has_focus() {
        return;
    }
    const GAP: f32 = 2.0;
    painter.rect_stroke(
        rect.expand(GAP),
        CornerRadius::same((radius + GAP) as u8),
        Stroke::new(2.0, p.focus),
        StrokeKind::Outside,
    );
}

/// Arrow-key handling for a horizontal `0..=1` strip.
///
/// Claims only the horizontal arrows, leaving the vertical pair to egui's focus
/// navigation so the user can move between the stacked controls — the same
/// split the crate's sliders make, and the opposite of the SV plane above,
/// which needs all four. `Home` jumps to 0; `End` jumps to `end`, which is 1 for
/// a channel whose two ends differ and one step short of it for hue, where 1 and
/// 0 are both red and `End` would otherwise be a synonym for `Home`.
///
/// Returns whether the channel moved.
fn strip_keys(ui: &mut Ui, response: &Response, channel: &mut f32, step: f32, end: f32) -> bool {
    if !response.has_focus() {
        return false;
    }
    ui.memory_mut(|m| {
        m.set_focus_lock_filter(
            response.id,
            EventFilter {
                horizontal_arrows: true,
                ..Default::default()
            },
        );
    });

    let mut next = *channel;
    for ev in ui.input(|i| i.events.clone()) {
        if let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = ev
        {
            let d = if modifiers.shift { step * 10.0 } else { step };
            match key {
                Key::ArrowRight => next += d,
                Key::ArrowLeft => next -= d,
                Key::Home => next = 0.0,
                Key::End => next = end,
                _ => {}
            }
        }
    }

    // Clamped, not wrapped, including for hue: the strip draws a line with two
    // ends, and the pointer path clamps, so the keyboard matching it keeps the
    // two ways of driving the same control honest.
    let next = next.clamp(0.0, 1.0);
    if (next - *channel).abs() <= f32::EPSILON {
        return false;
    }
    *channel = next;
    crate::commit::mark_commit(ui.ctx(), response.id);
    true
}

/// Classify a surface's settle for the recents row. `by_key` says the keyboard
/// moved the value this pass; a key nudge marks its own commit, so
/// [`committed()`](crate::ResponseCommitExt::committed) is true either way and
/// the keyboard has to be asked about first.
fn settled_by(response: &Response, by_key: bool) -> Settled {
    if by_key {
        Settled::KeyRun(response.id)
    } else if response.committed() {
        Settled::Gesture
    } else {
        Settled::No
    }
}

fn paint_sv_plane(ui: &mut Ui, theme: &Theme, hsv: &mut HsvaGamma) -> (bool, Settled) {
    let p = &theme.palette;
    let avail = ui.available_width();
    let height = 150.0;
    let (rect, response) = ui.allocate_exact_size(vec2(avail, height), Sense::click_and_drag());
    let mut changed = false;
    let mut by_key = false;

    crate::focus::focus_on_press(&response);

    if let Some(pos) = response.interact_pointer_pos()
        && response.is_pointer_button_down_on()
    {
        let s = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
        let v = 1.0 - ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0);
        hsv.s = s;
        hsv.v = v;
        changed = true;
    }

    // The one two-dimensional surface here: horizontal arrows move saturation,
    // vertical arrows move value. Both axes have to be claimed or egui routes
    // the key to spatial focus navigation as well and the plane hands focus to
    // a neighbour after one press. `Home` / `End` are deliberately unbound —
    // "the end" of a plane is a corner, not a value, and the strips below use
    // them for something unambiguous.
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });

        let (mut s, mut v) = (hsv.s, hsv.v);
        // Read `Shift` off each key event rather than `InputState::modifiers`,
        // which only tracks `ModifiersChanged`. Same as the crate's sliders.
        for ev in ui.input(|i| i.events.clone()) {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = ev
            {
                let d = if modifiers.shift {
                    CHANNEL_STEP * 10.0
                } else {
                    CHANNEL_STEP
                };
                match key {
                    Key::ArrowRight => s += d,
                    Key::ArrowLeft => s -= d,
                    Key::ArrowUp => v += d,
                    Key::ArrowDown => v -= d,
                    _ => {}
                }
            }
        }
        let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        if (s - hsv.s).abs() > f32::EPSILON || (v - hsv.v).abs() > f32::EPSILON {
            hsv.s = s;
            hsv.v = v;
            changed = true;
            by_key = true;
            crate::commit::mark_commit(ui.ctx(), response.id);
        }
    }

    // Classified after the keyboard block: a nudge reports its own commit
    // through `mark_commit`, and `committed()` has to see it in the same pass
    // for the colour to reach the recents row.
    let settled = settled_by(&response, by_key);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(6);

        // Build a NxN mesh of hsv-sampled colors.
        let n: usize = 24;
        let mut mesh = Mesh::default();
        for yi in 0..=n {
            for xi in 0..=n {
                let s = xi as f32 / n as f32;
                let v = 1.0 - yi as f32 / n as f32;
                let c: Color32 = HsvaGamma {
                    h: hsv.h,
                    s,
                    v,
                    a: 1.0,
                }
                .into();
                let x = lerp(rect.left()..=rect.right(), s);
                let y = lerp(rect.top()..=rect.bottom(), 1.0 - v);
                mesh.colored_vertex(pos2(x, y), c);
            }
        }
        let stride = (n + 1) as u32;
        for yi in 0..n {
            for xi in 0..n {
                let i = (yi * (n + 1) + xi) as u32;
                mesh.add_triangle(i, i + 1, i + stride);
                mesh.add_triangle(i + 1, i + stride, i + stride + 1);
            }
        }
        painter.add(Shape::mesh(mesh));

        // Cover the rectangular mesh's corner overflow with the popover
        // surface so the rounded shape reads cleanly.
        paint_rounded_corner_mask(painter, rect, 6.0, p.card);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, p.border), StrokeKind::Inside);
        paint_focus_ring(painter, rect, 6.0, p, &response);

        // Reticle.
        let cx = lerp(rect.left()..=rect.right(), hsv.s);
        let cy = lerp(rect.top()..=rect.bottom(), 1.0 - hsv.v);
        let center = pos2(cx, cy);
        ui.painter().circle(
            center,
            6.0,
            Color32::TRANSPARENT,
            Stroke::new(2.0, Color32::WHITE),
        );
        ui.painter().circle_stroke(
            center,
            7.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180)),
        );
    }

    // Two channels, so there is no single value to announce; the label carries
    // what the surface adjusts.
    response.widget_info(|| WidgetInfo::labeled(Role::Slider, true, "Saturation and value"));

    (changed, settled)
}

fn paint_hue_strip(ui: &mut Ui, theme: &Theme, hsv: &mut HsvaGamma) -> (bool, Settled) {
    let p = &theme.palette;
    let avail = ui.available_width();
    let height = 14.0;
    let (rect, response) = ui.allocate_exact_size(vec2(avail, height), Sense::click_and_drag());
    let mut changed = false;

    crate::focus::focus_on_press(&response);

    if let Some(pos) = response.interact_pointer_pos()
        && response.is_pointer_button_down_on()
    {
        let h = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
        hsv.h = h;
        changed = true;
    }

    // `End` stops one step short of 1: the hue circle joins there, so 1 is the
    // same red as 0, and jumping to it would leave `ArrowRight` dead as well.
    let by_key = strip_keys(ui, &response, &mut hsv.h, HUE_STEP, 1.0 - HUE_STEP);
    changed |= by_key;
    let settled = settled_by(&response, by_key);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same((rect.height() * 0.5) as u8);

        let n: usize = 36;
        let mut mesh = Mesh::default();
        for i in 0..=n {
            let h = i as f32 / n as f32;
            let c: Color32 = HsvaGamma {
                h,
                s: 1.0,
                v: 1.0,
                a: 1.0,
            }
            .into();
            let x = lerp(rect.left()..=rect.right(), h);
            mesh.colored_vertex(pos2(x, rect.top()), c);
            mesh.colored_vertex(pos2(x, rect.bottom()), c);
            if i < n {
                let base = (i * 2) as u32;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 2, base + 3);
            }
        }
        painter.add(Shape::mesh(mesh));
        paint_rounded_corner_mask(painter, rect, rect.height() * 0.5, p.card);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, p.border), StrokeKind::Inside);
        paint_focus_ring(painter, rect, rect.height() * 0.5, p, &response);

        let thumb_x = lerp(rect.left()..=rect.right(), hsv.h);
        let thumb_center = pos2(thumb_x, rect.center().y);
        let thumb_color: Color32 = HsvaGamma {
            h: hsv.h,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        }
        .into();
        painter.circle(
            thumb_center,
            7.0,
            thumb_color,
            Stroke::new(2.0, Color32::WHITE),
        );
        painter.circle_stroke(
            thumb_center,
            8.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 100)),
        );
    }

    // Degrees, which is how hue is read everywhere else. The far end of the
    // strip is 360, which names the same red as 0, so report it as 0.
    let degrees = (hsv.h * 360.0).round() as f64 % 360.0;
    response.widget_info(|| WidgetInfo::slider(true, degrees, "Hue"));

    (changed, settled)
}

fn paint_alpha_slider(ui: &mut Ui, theme: &Theme, hsv: &mut HsvaGamma) -> (bool, Settled) {
    let p = &theme.palette;
    let avail = ui.available_width();
    let height = 14.0;
    let (rect, response) = ui.allocate_exact_size(vec2(avail, height), Sense::click_and_drag());
    let mut changed = false;

    crate::focus::focus_on_press(&response);

    if let Some(pos) = response.interact_pointer_pos()
        && response.is_pointer_button_down_on()
    {
        let a = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
        hsv.a = a;
        changed = true;
    }

    let by_key = strip_keys(ui, &response, &mut hsv.a, CHANNEL_STEP, 1.0);
    changed |= by_key;
    let settled = settled_by(&response, by_key);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same((rect.height() * 0.5) as u8);

        // Checkers under the gradient.
        paint_checkers(painter, rect, radius);

        // Gradient from transparent → opaque (current hue/sat/value).
        let opaque: Color32 = HsvaGamma { a: 1.0, ..*hsv }.into();
        let [r, g, b, _] = opaque.to_srgba_unmultiplied();
        let transparent = Color32::from_rgba_unmultiplied(r, g, b, 0);
        let mut mesh = Mesh::default();
        let n = 12u32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let c = lerp_color(transparent, opaque, t);
            let x = lerp(rect.left()..=rect.right(), t);
            mesh.colored_vertex(pos2(x, rect.top()), c);
            mesh.colored_vertex(pos2(x, rect.bottom()), c);
            if i < n {
                let base = i * 2;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 2, base + 3);
            }
        }
        painter.add(Shape::mesh(mesh));
        paint_rounded_corner_mask(painter, rect, rect.height() * 0.5, p.card);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, p.border), StrokeKind::Inside);
        paint_focus_ring(painter, rect, rect.height() * 0.5, p, &response);

        let thumb_x = lerp(rect.left()..=rect.right(), hsv.a);
        let thumb_center = pos2(thumb_x, rect.center().y);
        painter.circle(thumb_center, 7.0, p.text, Stroke::new(2.0, p.card));
    }

    // Percent, matching how the alpha strip reads to a sighted user.
    let percent = (hsv.a * 100.0).round() as f64;
    response.widget_info(|| WidgetInfo::slider(true, percent, "Alpha"));

    (changed, settled)
}

// --- hex input -------------------------------------------------------------

fn paint_hex_input(ui: &mut Ui, theme: &Theme, picker_id: Id, current: Color32) -> Option<Color32> {
    let p = &theme.palette;
    let t = &theme.typography;

    let buf_id = picker_id.with(HEX_BUF_SUFFIX);
    let edit_id = picker_id.with("color_picker::hex_edit");

    // Sync the buffer to the current color when the input doesn't have focus.
    let has_focus = ui.memory(|m| m.has_focus(edit_id));
    let mut buf: String = ui.ctx().data(|d| d.get_temp(buf_id)).unwrap_or_default();
    if !has_focus {
        buf = format_hex(current);
    }

    let mut picked = None;
    ui.horizontal(|ui| {
        // Preview swatch on the left.
        let preview_size = Vec2::splat(28.0);
        let (preview_rect, _) = ui.allocate_exact_size(preview_size, Sense::hover());
        let radius_n: u8 = 5;
        let radius = CornerRadius::same(radius_n);
        if current.is_opaque() {
            ui.painter().rect_filled(preview_rect, radius, current);
        } else {
            paint_checkers(ui.painter(), preview_rect, radius);
            ui.painter().rect_filled(preview_rect, radius, current);
            paint_rounded_corner_mask(ui.painter(), preview_rect, radius_n as f32, p.card);
        }
        ui.painter().rect_stroke(
            preview_rect,
            radius,
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );

        ui.add_space(8.0);

        // Hex text edit.
        let response = crate::theme::with_themed_visuals(ui, |ui| {
            let v = ui.visuals_mut();
            crate::theme::themed_input_visuals(v, theme, p.input_bg);
            v.extreme_bg_color = p.input_bg;
            v.selection.bg_fill = with_alpha(p.focus, 90);
            v.selection.stroke = Stroke::new(1.0, p.focus);

            let edit = TextEdit::singleline(&mut buf)
                .id(edit_id)
                .font(FontSelection::FontId(egui::FontId::monospace(t.body)))
                .text_color(p.text)
                .margin(vec2(8.0, 4.0))
                .desired_width(ui.available_width());
            ui.add(edit).accessible_name("Hex color")
        });

        // Try to parse on every change. Accept on commit only.
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(c) = parse_hex(&buf) {
                picked = Some(c);
                buf = format_hex(c);
            } else {
                // Reject: revert buffer.
                buf = format_hex(current);
            }
        } else if !response.has_focus() && response.changed() {
            // External update path — let the syncing above reset the buffer.
        } else if response.has_focus() {
            // Live-parse if the user has typed a complete value, but don't
            // commit until they press enter. Keep the buffer untouched.
            let _ = parse_hex(&buf);
        }

        if !response.has_focus() {
            // Don't store transient buffer when focus is elsewhere.
            ui.ctx().data_mut(|d| d.remove::<String>(buf_id));
        } else {
            ui.ctx().data_mut(|d| d.insert_temp(buf_id, buf.clone()));
        }
    });

    picked
}

// --- helpers ---------------------------------------------------------------

fn small_label(theme: &Theme, text: &str) -> egui::Label {
    let rich = egui::RichText::new(text.to_uppercase())
        .color(theme.palette.text_faint)
        .size(theme.typography.small - 1.0);
    egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend)
}

fn paint_swatch(
    painter: &egui::Painter,
    rect: Rect,
    color: Color32,
    radius: u8,
    is_dark: bool,
    surround: Color32,
) {
    let r = CornerRadius::same(radius);
    if color.is_opaque() {
        painter.rect_filled(rect, r, color);
    } else {
        paint_checkers(painter, rect, r);
        painter.rect_filled(rect, r, color);
        paint_rounded_corner_mask(painter, rect, radius as f32, surround);
    }
    let inset = if is_dark {
        Color32::from_rgba_unmultiplied(15, 23, 42, 110)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 50)
    };
    painter.rect_stroke(rect, r, Stroke::new(1.0, inset), StrokeKind::Inside);
}

/// Cover the area between the bounding `rect` and its rounded interior
/// with `mask_color`. Use after painting a rectangular mesh or unrounded
/// pattern that overflows the intended rounded shape, so the parent
/// surface fills the corner cells instead of the overflow showing through.
fn paint_rounded_corner_mask(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    mask_color: Color32,
) {
    if radius <= 0.5 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let n: usize = 12;
    let half_pi = std::f32::consts::FRAC_PI_2;
    let pi = std::f32::consts::PI;
    let corners: [(Pos2, Pos2, f32); 4] = [
        // Top-left: arc spans angles π → 3π/2 around (left+r, top+r)
        (rect.left_top(), pos2(rect.left() + r, rect.top() + r), pi),
        // Top-right: 3π/2 → 2π around (right-r, top+r)
        (
            rect.right_top(),
            pos2(rect.right() - r, rect.top() + r),
            1.5 * pi,
        ),
        // Bottom-right: 0 → π/2 around (right-r, bottom-r)
        (
            rect.right_bottom(),
            pos2(rect.right() - r, rect.bottom() - r),
            0.0,
        ),
        // Bottom-left: π/2 → π around (left+r, bottom-r)
        (
            rect.left_bottom(),
            pos2(rect.left() + r, rect.bottom() - r),
            half_pi,
        ),
    ];
    for (corner, center, start_angle) in corners {
        let mut mesh = Mesh::default();
        mesh.colored_vertex(corner, mask_color);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let theta = start_angle + half_pi * t;
            let p = pos2(center.x + r * theta.cos(), center.y + r * theta.sin());
            mesh.colored_vertex(p, mask_color);
        }
        for i in 0..n {
            mesh.add_triangle(0, (1 + i) as u32, (2 + i) as u32);
        }
        painter.add(Shape::mesh(mesh));
    }
}

fn paint_checkers(painter: &egui::Painter, rect: Rect, radius: CornerRadius) {
    let dark = Color32::from_gray(40);
    let light = Color32::from_gray(96);
    let cell = (rect.height() * 0.5).max(2.0);
    painter.rect_filled(rect, radius, dark);
    let cols = (rect.width() / cell).ceil() as i32;
    let rows = (rect.height() / cell).ceil() as i32;
    let mut tiles: Vec<Shape> = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            if (i + j) % 2 == 0 {
                continue;
            }
            let x0 = rect.min.x + i as f32 * cell;
            let y0 = rect.min.y + j as f32 * cell;
            let x1 = (x0 + cell).min(rect.max.x);
            let y1 = (y0 + cell).min(rect.max.y);
            tiles.push(Shape::rect_filled(
                Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)),
                CornerRadius::ZERO,
                light,
            ));
        }
    }
    // Clip to the radius by stacking under another rect with no fill but
    // matching radius. Simpler: just paint the tiles; the slight over-paint
    // at corners is hidden by the swatch's rounded fill drawn on top.
    painter.extend(tiles);
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 {
        let xf = x as f32;
        let yf = y as f32;
        (xf + (yf - xf) * t).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

fn current_hsv(ctx: &egui::Context, picker_id: Id, color: Color32) -> HsvaGamma {
    let cache_id = picker_id.with(HSV_CACHE_SUFFIX);
    let cached: Option<HsvaGamma> = ctx.data(|d| d.get_temp(cache_id));
    if let Some(c) = cached
        && Color32::from(c) == color
    {
        return c;
    }
    HsvaGamma::from(Hsva::from(color))
}

fn set_hsv(ctx: &egui::Context, picker_id: Id, hsv: HsvaGamma) {
    let cache_id = picker_id.with(HSV_CACHE_SUFFIX);
    ctx.data_mut(|d| d.insert_temp(cache_id, hsv));
}

fn recents_id(picker_id: Id) -> Id {
    picker_id.with(RECENTS_SUFFIX)
}

fn key_run_id(picker_id: Id) -> Id {
    picker_id.with(KEY_RUN_SUFFIX)
}

/// Record a keyboard nudge in the recents row, amending the entry this run has
/// already opened rather than adding a second one beside it.
///
/// Without this a run of presses would push one entry each: at auto-repeat
/// speed a single held arrow key fills the whole row with shades a user cannot
/// tell apart and evicts everything they had collected. The pointer path
/// records one entry per gesture, and a keyboard run is one gesture too.
fn record_key_run(ctx: &egui::Context, picker_id: Id, surface: Id, color: Color32, max: usize) {
    let run = key_run_id(picker_id);
    if ctx.data(|d| d.get_temp::<Id>(run)) == Some(surface) {
        // Drop the head this run put there; the re-push below replaces it with
        // the colour the run has now reached.
        let id = recents_id(picker_id);
        let mut list: Vec<Color32> = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
        if !list.is_empty() {
            list.remove(0);
            ctx.data_mut(|d| d.insert_temp(id, list));
        }
    } else {
        ctx.data_mut(|d| d.insert_temp(run, surface));
    }
    push_recent(ctx, picker_id, color, max);
}

fn end_key_run(ctx: &egui::Context, picker_id: Id) {
    ctx.data_mut(|d| d.remove::<Id>(key_run_id(picker_id)));
}

fn push_recent(ctx: &egui::Context, picker_id: Id, color: Color32, max: usize) {
    let id = recents_id(picker_id);
    let mut list: Vec<Color32> = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    list.retain(|c| *c != color);
    list.insert(0, color);
    list.truncate(max);
    ctx.data_mut(|d| d.insert_temp(id, list));
}

fn format_hex(color: Color32) -> String {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

fn parse_hex(text: &str) -> Option<Color32> {
    let s = text.trim().trim_start_matches('#');
    let bytes: Vec<u8> = s
        .chars()
        .filter_map(|c| c.to_digit(16).map(|d| d as u8))
        .collect();
    match bytes.len() {
        3 => Some(Color32::from_rgb(
            bytes[0] * 17,
            bytes[1] * 17,
            bytes[2] * 17,
        )),
        4 => Some(Color32::from_rgba_unmultiplied(
            bytes[0] * 17,
            bytes[1] * 17,
            bytes[2] * 17,
            bytes[3] * 17,
        )),
        6 => Some(Color32::from_rgb(
            (bytes[0] << 4) | bytes[1],
            (bytes[2] << 4) | bytes[3],
            (bytes[4] << 4) | bytes[5],
        )),
        8 => Some(Color32::from_rgba_unmultiplied(
            (bytes[0] << 4) | bytes[1],
            (bytes[2] << 4) | bytes[3],
            (bytes[4] << 4) | bytes[5],
            (bytes[6] << 4) | bytes[7],
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Color32::from_rgb(0x38, 0xbd, 0xf8);
        assert_eq!(format_hex(c), "#38BDF8");
        assert_eq!(parse_hex("#38BDF8"), Some(c));
        assert_eq!(parse_hex("38bdf8"), Some(c));
        assert_eq!(parse_hex("#38B"), Some(Color32::from_rgb(0x33, 0x88, 0xbb)));
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#zzzzzz"), None);
    }

    #[test]
    fn hex_round_trip_alpha() {
        let c = Color32::from_rgba_unmultiplied(0x38, 0xbd, 0xf8, 0xc0);
        assert_eq!(format_hex(c), "#38BDF8C0");
        assert_eq!(parse_hex("#38BDF8C0"), Some(c));
    }
}
