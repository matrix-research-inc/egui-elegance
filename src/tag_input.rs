//! A token / chip / pill input bound to a `Vec<String>`.
//!
//! Renders existing tags as small accent-tinted pills and a free text
//! field for adding more. Enter and comma commit the buffer as a new
//! tag; with [`commit_on_space`](TagInput::commit_on_space) enabled,
//! whitespace commits too. Backspace on an empty buffer arms the last
//! pill (red highlight) and a second Backspace removes it; clicking a
//! pill's `×` removes that pill. Pasted text containing commas or
//! whitespace splits into multiple tags. Duplicates are folded
//! case-insensitively.
//!
//! The caller owns the `Vec<String>`; transient typing state (current
//! buffer, armed flag, last validation error) lives in egui memory keyed
//! by the supplied `id_salt`.

use egui::{
    Color32, CornerRadius, Event, FontId, FontSelection, Id, Key, Rect, Response, Role, Sense,
    Stroke, StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, WidgetText, pos2, vec2,
};

use crate::Accent;
use crate::theme::{Theme, themed_input_visuals, with_alpha, with_themed_visuals};

/// Boxed validator closure: `Ok(())` accepts the value, `Err(msg)` rejects
/// it and surfaces `msg` as an inline error.
type Validator<'a> = Box<dyn Fn(&str) -> Result<(), String> + 'a>;

/// A pill-list text input bound to a `Vec<String>`.
///
/// ```no_run
/// # use elegance::TagInput;
/// # egui::__run_test_ui(|ui| {
/// let mut tags: Vec<String> = vec!["rust".into(), "egui".into()];
/// TagInput::new("tags", &mut tags)
///     .label("Tags")
///     .placeholder("Add a tag…")
///     .show(ui);
/// # });
/// ```
///
/// # Email-style validator
///
/// Pass a [`validator`](Self::validator) closure to reject malformed
/// commits. The widget keeps the offending text in the buffer, switches
/// the border to the danger colour, and renders an inline error line
/// below.
///
/// ```no_run
/// # use elegance::TagInput;
/// # egui::__run_test_ui(|ui| {
/// let mut to: Vec<String> = Vec::new();
/// TagInput::new("recipients", &mut to)
///     .label("Recipients")
///     .placeholder("Add an email…")
///     .commit_on_space(true)
///     .validator(|v| {
///         if v.contains('@') && v.contains('.') {
///             Ok(())
///         } else {
///             Err(format!("\"{v}\" isn't a valid email."))
///         }
///     })
///     .show(ui);
/// # });
/// ```
#[must_use = "Call `.show(ui)` to render the input."]
pub struct TagInput<'a> {
    id_salt: Id,
    tags: &'a mut Vec<String>,
    label: Option<WidgetText>,
    placeholder: Option<String>,
    accent: Accent,
    enabled: bool,
    commit_on_space: bool,
    desired_width: Option<f32>,
    validator: Option<Validator<'a>>,
}

impl<'a> std::fmt::Debug for TagInput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TagInput")
            .field("tags", &self.tags)
            .field("label", &self.label.as_ref().map(|w| w.text()))
            .field("placeholder", &self.placeholder)
            .field("accent", &self.accent)
            .field("enabled", &self.enabled)
            .field("commit_on_space", &self.commit_on_space)
            .field("desired_width", &self.desired_width)
            .finish()
    }
}

impl<'a> TagInput<'a> {
    /// Create a tag input bound to `tags`. The `id_salt` keys the buffer,
    /// armed flag, and last-error in egui memory. Use a unique salt per
    /// instance.
    pub fn new(id_salt: impl crate::IdSalt, tags: &'a mut Vec<String>) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            tags,
            label: None,
            placeholder: None,
            accent: Accent::Sky,
            enabled: true,
            commit_on_space: false,
            desired_width: None,
            validator: None,
        }
    }

    /// Show a label above the input.
    pub fn label(mut self, text: impl Into<WidgetText>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// Placeholder shown inside the field when empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Pill accent colour. Default: [`Accent::Sky`].
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Disable the input. Disabled inputs ignore clicks, keystrokes, and
    /// the `×` buttons. Default: enabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Treat whitespace as a commit key, in addition to Enter and comma.
    /// Useful for email recipient fields where users frequently type
    /// addresses separated by spaces. Default: `false`.
    pub fn commit_on_space(mut self, on: bool) -> Self {
        self.commit_on_space = on;
        self
    }

    /// Desired width (points) of the framed input. Default: full
    /// available width.
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Reject a candidate tag with an inline error. The closure returns
    /// `Ok(())` for accepted values or `Err(msg)` to display `msg`
    /// underneath the input. Rejected text stays in the buffer so the
    /// user can fix and retry.
    pub fn validator(mut self, f: impl Fn(&str) -> Result<(), String> + 'a) -> Self {
        self.validator = Some(Box::new(f));
        self
    }

    /// Render the input.
    pub fn show(self, ui: &mut Ui) -> TagInputResponse {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let widget_id = ui.make_persistent_id(self.id_salt);
        let edit_id = widget_id.with("edit");

        let mut state: State = ui
            .ctx()
            .data(|d| d.get_temp::<State>(widget_id))
            .unwrap_or_default();

        let label_text = self.label.as_ref().map(|w| w.text().to_string());

        let outer = ui
            .vertical(|ui| {
                if let Some(label) = self.label.as_ref() {
                    ui.add_space(2.0);
                    let rich = egui::RichText::new(label.text())
                        .color(p.text_muted)
                        .size(t.label);
                    ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                    ui.add_space(2.0);
                }

                let pill_fill = with_alpha(p.accent_fill(self.accent), 41);
                let armed_fill = with_alpha(p.danger, 56);
                let armed_stroke = Stroke::new(1.0, with_alpha(p.danger, 153));
                let pad_x = 6.0;
                let pad_y = 4.0;
                let row_gap_x = 6.0;
                let row_gap_y = 4.0;

                let total_width = self
                    .desired_width
                    .unwrap_or_else(|| ui.available_width())
                    .max(120.0);

                // Reserve a slot for the frame fill so we can paint it
                // *under* the inner content once we know the focus state.
                let bg_idx = ui.painter().add(egui::Shape::Noop);

                let mut to_remove: Option<usize> = None;
                let inner = ui.allocate_ui_with_layout(
                    vec2(total_width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // `Frame::inner_margin` indents every wrapped row
                        // by `pad_x` on each side. `add_space` only
                        // affects the first row, so the TextEdit hugged
                        // the left edge after wrapping.
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(row_gap_x, row_gap_y);

                                    for (i, tag) in self.tags.iter().enumerate() {
                                        let armed = state.armed && i + 1 == self.tags.len();
                                        let close_clicked = paint_pill(
                                            ui,
                                            tag,
                                            &theme,
                                            if armed { armed_fill } else { pill_fill },
                                            if armed { armed_stroke } else { Stroke::NONE },
                                            self.enabled,
                                        );
                                        if close_clicked {
                                            to_remove = Some(i);
                                        }
                                    }

                                    // The editor borrows the outer frame's
                                    // chrome, so strip its per-state strokes.
                                    let avail = ui.available_width().max(80.0);
                                    with_themed_visuals(ui, |ui| {
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

                                        let mut te = TextEdit::singleline(&mut state.buffer)
                                            .id(edit_id)
                                            .font(FontSelection::FontId(FontId::proportional(
                                                t.body,
                                            )))
                                            .text_color(p.text)
                                            .desired_width(avail)
                                            // Centre text and hint in the frame; egui otherwise
                                            // takes the parent layout's alignment.
                                            .align(egui::Align2::LEFT_CENTER)
                                            // `.margin()` is ignored when
                                            // a custom frame is set, so
                                            // bake the indent into the
                                            // frame itself. The 8 px
                                            // matches the pill's internal
                                            // left padding so buffer text
                                            // aligns with pill text.
                                            .frame(
                                                egui::Frame::new()
                                                    .inner_margin(egui::Margin::symmetric(8, 2)),
                                            );
                                        if let Some(ph) = self.placeholder.as_deref() {
                                            te = te.hint_text(
                                                egui::RichText::new(ph).color(p.text_faint),
                                            );
                                        }
                                        ui.add_enabled(self.enabled, te)
                                    })
                                })
                                .inner
                            })
                            .inner
                    },
                );

                let edit_response = inner.inner;
                let frame_rect = inner.response.rect;

                let buffer_was_empty = state.buffer.is_empty();
                let focused = edit_response.has_focus();
                let lost_focus = edit_response.lost_focus();

                let (enter_pressed, backspace_pressed, other_key_pressed) = ui.input(|i| {
                    let mut any_other = false;
                    for ev in &i.events {
                        if let Event::Key {
                            pressed: true, key, ..
                        } = ev
                        {
                            if !matches!(key, Key::Backspace | Key::Enter) {
                                any_other = true;
                                break;
                            }
                        } else if matches!(ev, Event::Text(_)) {
                            any_other = true;
                            break;
                        }
                    }
                    (
                        i.key_pressed(Key::Enter),
                        i.key_pressed(Key::Backspace),
                        any_other,
                    )
                });

                let separators: &[char] = if self.commit_on_space {
                    &[',', ' ', '\t', '\n']
                } else {
                    &[',', '\n']
                };

                if self.enabled {
                    // No frame-wide click sensor here: an outer click
                    // sense at `frame_rect` would shadow the per-pill `×`
                    // hit-tests (egui picks the topmost interact, which
                    // is the latest-registered one). Users still focus
                    // the editor by clicking on the editor row directly.

                    // 1. Backspace on empty buffer: arm or remove.
                    if focused && backspace_pressed {
                        if buffer_was_empty && !self.tags.is_empty() {
                            if state.armed {
                                self.tags.pop();
                                state.armed = false;
                            } else {
                                state.armed = true;
                            }
                        } else {
                            state.armed = false;
                        }
                    } else if state.armed && (other_key_pressed || enter_pressed) {
                        state.armed = false;
                    }

                    // 2. Separator characters (typed or pasted) split
                    //    the buffer and commit each piece.
                    if state.buffer.contains(|c: char| separators.contains(&c)) {
                        let pieces: Vec<String> = state
                            .buffer
                            .split(|c: char| separators.contains(&c))
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                        state.buffer.clear();
                        for raw in pieces {
                            commit_value(&raw, self.tags, self.validator.as_deref(), &mut state);
                            if state.error.is_some() {
                                state.buffer = raw;
                                break;
                            }
                        }
                    }

                    // 3. Enter (delivered as `lost_focus`): commit the
                    //    trimmed buffer, then re-focus so the user can
                    //    keep typing more tags.
                    if lost_focus && enter_pressed {
                        let raw = state.buffer.trim().to_string();
                        if !raw.is_empty() {
                            commit_value(&raw, self.tags, self.validator.as_deref(), &mut state);
                            if state.error.is_none() {
                                state.buffer.clear();
                            }
                        }
                        ui.memory_mut(|m| m.request_focus(edit_id));
                    } else if lost_focus && !state.buffer.trim().is_empty() {
                        // Tab / click-away: commit in-progress buffer
                        // without re-focusing.
                        let raw = state.buffer.trim().to_string();
                        commit_value(&raw, self.tags, self.validator.as_deref(), &mut state);
                        if state.error.is_none() {
                            state.buffer.clear();
                        }
                    }

                    // 4. Typing past an error clears it.
                    if state.error.is_some() && !state.buffer.is_empty() && other_key_pressed {
                        state.error = None;
                    }
                }

                if let Some(i) = to_remove {
                    if i < self.tags.len() {
                        self.tags.remove(i);
                    }
                    state.armed = false;
                    state.error = None;
                }

                // Resolve the frame chrome now that we know the state.
                let frame_focused = ui.memory(|m| m.has_focus(edit_id));
                let frame_hovered = ui.rect_contains_pointer(frame_rect);
                let bg_fill = p.input_bg;
                let (border_stroke_w, border_color) = if !self.enabled {
                    (1.0, with_alpha(p.border, 160))
                } else if state.error.is_some() {
                    (1.5, p.danger)
                } else if frame_focused {
                    (1.5, p.focus)
                } else if frame_hovered {
                    (1.0, p.text_muted)
                } else {
                    (1.0, p.border)
                };
                let radius = CornerRadius::same(theme.control_radius as u8);
                ui.painter().set(
                    bg_idx,
                    egui::Shape::rect_filled(frame_rect, radius, bg_fill),
                );
                ui.painter().rect_stroke(
                    frame_rect,
                    radius,
                    Stroke::new(border_stroke_w, border_color),
                    StrokeKind::Inside,
                );

                if let Some(err) = state.error.as_deref() {
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(egui::RichText::new(err).color(p.danger).size(t.small))
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );
                }

                edit_response
            })
            .inner;

        ui.ctx()
            .data_mut(|d| d.insert_temp(widget_id, state.clone()));

        if let Some(label) = label_text {
            outer.widget_info(|| WidgetInfo::labeled(Role::TextInput, self.enabled, &label));
        }

        TagInputResponse {
            response: outer,
            error: state.error,
        }
    }
}

/// The result of rendering a [`TagInput`].
#[derive(Debug)]
pub struct TagInputResponse {
    /// Underlying egui [`Response`] for the inner text editor — use
    /// `.lost_focus()`, `.has_focus()`, etc.
    pub response: Response,
    /// If a validator rejected the most recent commit attempt, the error
    /// message it returned; otherwise `None`.
    pub error: Option<String>,
}

#[derive(Clone, Default)]
struct State {
    buffer: String,
    armed: bool,
    error: Option<String>,
}

type ValidatorRef<'a> = &'a (dyn Fn(&str) -> Result<(), String> + 'a);

fn commit_value(
    raw: &str,
    tags: &mut Vec<String>,
    validator: Option<ValidatorRef<'_>>,
    state: &mut State,
) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    if let Some(v) = validator
        && let Err(msg) = v(raw)
    {
        state.error = Some(msg);
        return;
    }
    let lower = raw.to_lowercase();
    if tags.iter().any(|t| t.to_lowercase() == lower) {
        // Silent dedup. The HTML mockup flashes the existing pill; in
        // egui we just drop the duplicate.
        state.error = None;
        return;
    }
    tags.push(raw.to_owned());
    state.error = None;
    state.armed = false;
}

fn paint_pill(
    ui: &mut Ui,
    tag: &str,
    theme: &Theme,
    fill: Color32,
    extra_stroke: Stroke,
    enabled: bool,
) -> bool {
    let p = &theme.palette;
    let t = &theme.typography;

    let label_size = t.label;
    let close_diam = 16.0;
    let pad_left = 8.0;
    let pad_right = 2.0;
    let pad_y = 2.0;
    let gap = 4.0;

    let label_galley =
        egui::WidgetText::from(egui::RichText::new(tag).color(p.text).size(label_size))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                FontSelection::FontId(FontId::proportional(label_size)),
            );

    let inner_h = label_galley.size().y.max(close_diam);
    let total = vec2(
        pad_left + label_galley.size().x + gap + close_diam + pad_right,
        pad_y * 2.0 + inner_h,
    );
    let (rect, _resp) = ui.allocate_exact_size(total, Sense::hover());

    if !ui.is_rect_visible(rect) {
        return false;
    }

    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        fill,
        extra_stroke,
        StrokeKind::Inside,
    );

    let label_pos = pos2(
        rect.min.x + pad_left,
        rect.center().y - label_galley.size().y * 0.5,
    );
    ui.painter().galley(label_pos, label_galley, p.text);

    let close_rect = Rect::from_center_size(
        pos2(rect.max.x - pad_right - close_diam * 0.5, rect.center().y),
        Vec2::splat(close_diam),
    );
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let close_resp = ui.interact(close_rect, ui.scope_id().with(("tag_close", tag)), sense);

    let close_bg = if close_resp.hovered() && enabled {
        with_alpha(p.text, 24)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(close_rect, CornerRadius::same(3), close_bg);

    let cross_color = if !enabled {
        p.text_faint
    } else if close_resp.hovered() {
        p.text
    } else {
        p.text_muted
    };
    paint_cross(ui, close_rect, cross_color);

    close_resp.clicked()
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
