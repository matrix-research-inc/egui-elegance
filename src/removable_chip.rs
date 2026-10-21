//! A single editable, removable chip: a bordered inline text edit with an
//! optional non-editable prefix and an `×` close button.
//!
//! Use this when you have one optional value that should appear inline among
//! other content (e.g., an inline filter pill, a path-segment chip, an
//! editable tag in a single-tag form) and the user can clear it by clicking
//! `×` or pressing Escape on an empty input. For multi-value tag inputs,
//! see [`TagInput`](crate::TagInput).

use egui::{
    Color32, CornerRadius, FontId, FontSelection, Id, Rect, Response, Role, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, WidgetText, pos2, vec2,
};

use crate::Accent;
use crate::theme::{Theme, themed_input_visuals, with_alpha, with_themed_visuals};

/// A bordered inline text input with an `×` close button, bound to a single
/// `String`.
///
/// ```no_run
/// # use elegance::RemovableChip;
/// # egui::__run_test_ui(|ui| {
/// let mut suffix = String::from("run-1");
/// let resp = RemovableChip::new(&mut suffix)
///     .prefix("_")
///     .placeholder("run-1")
///     .show(ui);
/// if resp.removed {
///     // caller drops the field
/// }
/// # });
/// ```
///
/// The chip auto-sizes its editor to fit the current text, clamped to
/// [`auto_size`](Self::auto_size). The `removed` flag in the returned
/// [`RemovableChipResponse`] is set when the user clicks `×` or presses
/// Escape on an empty input; the caller decides whether to actually clear
/// or drop the binding.
#[must_use = "Call `.show(ui)` to render the chip."]
pub struct RemovableChip<'a> {
    text: &'a mut String,
    prefix: Option<WidgetText>,
    placeholder: Option<&'a str>,
    accent: Accent,
    enabled: bool,
    min_width: f32,
    max_width: f32,
    id_salt: Option<Id>,
    focus_on_render: bool,
    close_on_empty_blur: bool,
}

impl<'a> std::fmt::Debug for RemovableChip<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemovableChip")
            .field("prefix", &self.prefix.as_ref().map(|w| w.text()))
            .field("placeholder", &self.placeholder)
            .field("accent", &self.accent)
            .field("enabled", &self.enabled)
            .field("min_width", &self.min_width)
            .field("max_width", &self.max_width)
            .field("focus_on_render", &self.focus_on_render)
            .field("close_on_empty_blur", &self.close_on_empty_blur)
            .finish()
    }
}

impl<'a> RemovableChip<'a> {
    /// Create a chip bound to `text`. The chip's value mirrors this `String`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            prefix: None,
            placeholder: None,
            accent: Accent::Sky,
            enabled: true,
            min_width: 50.0,
            max_width: 240.0,
            id_salt: None,
            focus_on_render: false,
            close_on_empty_blur: false,
        }
    }

    /// Show non-editable text inside the chip, before the input. Useful for
    /// leading separators (e.g. `"_"` for a path-suffix chip) or for fixed
    /// labels that read as part of the value but aren't part of the binding.
    pub fn prefix(mut self, text: impl Into<WidgetText>) -> Self {
        self.prefix = Some(text.into());
        self
    }

    /// Placeholder text shown when the input is empty.
    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    /// Border / focus accent colour. Default: [`Accent::Sky`].
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Disable the chip. Disabled chips ignore typing and clicks on `×`.
    /// Default: enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Minimum and maximum width (points) for the editor portion. The chip
    /// measures the current text and sizes the editor within this range.
    /// Default: `50.0..=240.0`.
    pub fn auto_size(mut self, range: std::ops::RangeInclusive<f32>) -> Self {
        self.min_width = *range.start();
        self.max_width = *range.end();
        self
    }

    /// Stable id salt. Useful when several chips share a layout, or when
    /// you need to address the chip's state across frames.
    pub fn id_salt(mut self, id: impl crate::IdSalt) -> Self {
        self.id_salt = Some(Id::unique(id));
        self
    }

    /// Request focus for the chip's TextEdit on this render. Pass `true`
    /// on the frame the chip first appears (e.g. just after the user
    /// clicked an "+ add" button to surface it) so the input is focused
    /// the moment it shows up; pass `false` on subsequent frames so the
    /// user can click out and the focus request doesn't keep stealing it
    /// back. The chip calls `request_focus` on its TextEdit's id before
    /// the widget is added, so focus lands on the very first frame the
    /// chip is visible.
    pub fn focus(mut self, request: bool) -> Self {
        self.focus_on_render = request;
        self
    }

    /// Auto-fire `removed` when the editor loses focus while empty. Use
    /// this for "+ add" affordances where leaving the chip empty is
    /// equivalent to deciding not to add the field at all: the caller
    /// then drops the binding and the affordance reverts to its
    /// pre-click state. Pairs naturally with [`focus`](Self::focus).
    /// Default: `false`.
    pub fn close_on_empty_blur(mut self, on: bool) -> Self {
        self.close_on_empty_blur = on;
        self
    }

    /// Render the chip and return its response.
    pub fn show(self, ui: &mut Ui) -> RemovableChipResponse {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let id_salt = self
            .id_salt
            .unwrap_or_else(|| Id::unique(ui.next_auto_id()));
        let edit_id = ui.make_persistent_id(id_salt);

        // Honour `focus(true)` before the TextEdit is added so the
        // widget picks up focus the same frame it appears (request_focus
        // applied after add only takes effect the following frame).
        if self.focus_on_render && self.enabled {
            ui.memory_mut(|m| m.request_focus(edit_id));
        }

        let pad_x = 6.0;
        // Vertical padding chosen so the chip renders at the same total
        // height as `Button::size(ButtonSize::Small)` (≈22 pt). Anything
        // smaller and the chip sits 2 px shorter than its row neighbours,
        // which reads as a layout bug rather than a stylistic choice.
        let pad_y = 3.0;
        let close_diam = 16.0;
        let gap = 4.0;

        // Reserve the background shape index now so we can paint the fill
        // and border under the inner content once focus is known.
        let bg_idx = ui.painter().add(Shape::Noop);

        // Auto-size the editor by measuring the current text (or
        // placeholder when empty) at body-font size and clamping into the
        // user-supplied range.
        let measure_text = if self.text.is_empty() {
            self.placeholder.unwrap_or("")
        } else {
            self.text.as_str()
        };
        let measured = WidgetText::from(egui::RichText::new(measure_text).size(t.body))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                FontSelection::FontId(FontId::proportional(t.body)),
            );
        let editor_w = (measured.size().x + 6.0).clamp(self.min_width, self.max_width);

        let mut removed = false;

        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, 0.0);
            ui.add_space(pad_x);

            if let Some(prefix) = &self.prefix {
                let rich = egui::RichText::new(prefix.text())
                    .color(p.text_faint)
                    .size(t.body);
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
            }

            // The editor borrows the chip's outer chrome, so strip its
            // per-state strokes and bg fill.
            let edit_response = with_themed_visuals(ui, |ui| {
                let v = ui.visuals_mut();
                themed_input_visuals(v, &theme, Color32::TRANSPARENT);
                v.extreme_bg_color = Color32::TRANSPARENT;
                for w in [
                    &mut v.widgets.inactive,
                    &mut v.widgets.hovered,
                    &mut v.widgets.active,
                    &mut v.widgets.open,
                ] {
                    w.bg_stroke = Stroke::NONE;
                }
                v.selection.bg_fill = with_alpha(p.focus, 90);
                v.selection.stroke = Stroke::new(1.0, p.focus);

                let mut te = TextEdit::singleline(self.text)
                    .id(edit_id)
                    .font(FontSelection::FontId(FontId::proportional(t.body)))
                    .text_color(p.text)
                    .desired_width(editor_w)
                    // Centre text and hint in the frame; egui otherwise
                    // takes the parent layout's alignment.
                    .align(egui::Align2::LEFT_CENTER)
                    .frame(
                        egui::Frame::new().inner_margin(egui::Margin::symmetric(0, pad_y as i8)),
                    );
                if let Some(ph) = self.placeholder {
                    te = te.hint_text(egui::RichText::new(ph).color(p.text_faint));
                }
                ui.add_enabled(self.enabled, te)
            });

            // Reserve the (×) button's footprint now so the chip's
            // overall layout is stable whether the cross is currently
            // painted or hidden. Defer the paint + click handling until
            // after we know the chip's outer rect (and therefore
            // hover-on-chip), so the cross can be hidden when the chip
            // is at rest and revealed on hover or focus.
            let close_size = Vec2::splat(close_diam);
            let sense = if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let (close_rect, close_resp) = ui.allocate_exact_size(close_size, sense);

            ui.add_space((pad_x - gap).max(0.0));

            (edit_response, close_rect, close_resp)
        });

        let (edit_response, close_rect, close_resp) = inner.inner;
        let frame_rect = inner.response.rect;

        // The cross only renders when the chip is "active": editor has
        // focus, or the pointer is anywhere over the chip's frame. At
        // rest the chip is just a value pill with no affordance noise.
        let frame_hovered = ui.rect_contains_pointer(frame_rect);
        let chip_active = edit_response.has_focus() || frame_hovered;
        if self.enabled && chip_active {
            let close_bg = if close_resp.hovered() {
                with_alpha(p.danger, 32)
            } else {
                Color32::TRANSPARENT
            };
            ui.painter()
                .rect_filled(close_rect, CornerRadius::same(3), close_bg);
            let cross_color = if close_resp.hovered() {
                p.danger
            } else {
                p.text_muted
            };
            paint_cross(ui, close_rect, cross_color);

            if close_resp.clicked() {
                removed = true;
            }
        }

        // Escape on an empty editor signals "remove" to the caller.
        if self.enabled
            && edit_response.has_focus()
            && self.text.is_empty()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            removed = true;
        }

        // Empty editor losing focus also fires `removed` when the caller
        // opted in. Lets a freshly-surfaced chip auto-close if the user
        // clicks elsewhere without typing anything.
        if self.close_on_empty_blur
            && self.enabled
            && edit_response.lost_focus()
            && self.text.trim().is_empty()
        {
            removed = true;
        }

        // Paint the chip's frame underneath everything. Reuse the
        // already-computed `frame_hovered` from the close-button gating
        // above.
        let frame_focused = ui.memory(|m| m.has_focus(edit_id));
        let bg_fill = p.input_bg;
        let (border_w, border_color) = if !self.enabled {
            (1.0, with_alpha(p.border, 160))
        } else if frame_focused {
            (1.5, p.accent_fill(self.accent))
        } else if frame_hovered {
            (1.0, p.text_muted)
        } else {
            (1.0, p.border)
        };
        let radius = CornerRadius::same(theme.control_radius as u8);
        ui.painter()
            .set(bg_idx, Shape::rect_filled(frame_rect, radius, bg_fill));
        ui.painter().rect_stroke(
            frame_rect,
            radius,
            Stroke::new(border_w, border_color),
            StrokeKind::Inside,
        );

        // Speak the placeholder (or "Removable chip" when none is set) as
        // the field label. The current text is announced separately by the
        // OS, so the widget label should describe the field's purpose.
        let label_for_a11y = self
            .placeholder
            .map(str::to_owned)
            .unwrap_or_else(|| "Removable chip".to_string());
        let response = inner.response;
        response
            .widget_info(|| WidgetInfo::labeled(Role::TextInput, self.enabled, &label_for_a11y));

        RemovableChipResponse { response, removed }
    }
}

/// The result of rendering a [`RemovableChip`].
#[derive(Debug)]
pub struct RemovableChipResponse {
    /// Outer [`Response`] covering the whole chip rect. Use this to react
    /// to hover, click-outside, etc.
    pub response: Response,
    /// `true` when the user clicked the `×` button or pressed Escape on
    /// an empty editor. The caller decides whether to clear the binding,
    /// drop the chip, or otherwise react.
    pub removed: bool,
}

fn paint_cross(ui: &Ui, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = 3.0;
    let stroke = Stroke::new(1.5, color);
    ui.painter()
        .line_segment([pos2(c.x - s, c.y - s), pos2(c.x + s, c.y + s)], stroke);
    ui.painter()
        .line_segment([pos2(c.x - s, c.y + s), pos2(c.x + s, c.y - s)], stroke);
}
