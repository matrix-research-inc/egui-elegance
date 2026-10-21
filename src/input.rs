//! Styled single-line text input.
//!
//! Wraps [`egui::TextEdit`] with:
//!
//! * The slate input background
//! * A crisp 1-px border that turns the focus accent on focus
//! * An optional "dirty" indicator (a 3-px focus-accent bar down the left
//!   edge) to signal unsaved changes
//! * An optional label and optional hint text
//! * Success / error flash animations triggered via
//!   [`ResponseFlashExt`](crate::ResponseFlashExt)

use egui::{
    Color32, CornerRadius, FontId, FontSelection, Margin, Response, Role, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, Widget, WidgetInfo, WidgetText,
};

use crate::{flash, theme::Theme};

/// A styled single-line text input.
///
/// ```no_run
/// # use elegance::TextInput;
/// # egui::__run_test_ui(|ui| {
/// let mut email = String::new();
/// ui.add(TextInput::new(&mut email).label("Email").hint("you@example.com"));
/// # });
/// ```
///
/// # Ids, focus, and flash state
///
/// Flash animations, focus, and cursor state are keyed off the widget's
/// egui id. Without [`id_salt`](Self::id_salt), the id is derived from
/// egui's auto-id counter, which is layout-dependent — if a sibling
/// appears or disappears above this input between frames, the id shifts
/// and any in-flight flash, focus, or cursor state is lost. Any input
/// you flash via [`ResponseFlashExt`](crate::ResponseFlashExt) should pin
/// its id with [`id_salt`](Self::id_salt).
#[must_use = "Add with `ui.add(...)`."]
pub struct TextInput<'a> {
    text: &'a mut String,
    label: Option<WidgetText>,
    hint: Option<&'a str>,
    dirty: bool,
    password: bool,
    revealable: bool,
    desired_width: Option<f32>,
    id_salt: Option<egui::Id>,
    compact: bool,
}

impl<'a> std::fmt::Debug for TextInput<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("dirty", &self.dirty)
            .field("password", &self.password)
            .field("revealable", &self.revealable)
            .field("desired_width", &self.desired_width)
            .field("compact", &self.compact)
            .finish()
    }
}

impl<'a> TextInput<'a> {
    /// Create a text input bound to `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            label: None,
            hint: None,
            dirty: false,
            password: false,
            revealable: false,
            desired_width: None,
            id_salt: None,
            compact: false,
        }
    }

    /// Show a label above the input.
    ///
    /// The label also names the input for screen readers. Without one, name
    /// the returned response with [`egui::Response::accessible_name`], or with
    /// [`egui::Response::labelled_by`] when a caption is drawn beside it.
    pub fn label(mut self, text: impl Into<WidgetText>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// Show placeholder-style hint text when the field is empty.
    pub fn hint(mut self, text: &'a str) -> Self {
        self.hint = Some(text);
        self
    }

    /// Mark the input as having unsaved changes. Shows a focus-accent
    /// accent bar down the left side.
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Mask the text as a password field.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Show a reveal toggle (an eye icon) inside the right edge of the input
    /// that masks the text by default and lets the user reveal it. Use for
    /// password and passphrase fields where confirming what was typed avoids
    /// silent typos.
    ///
    /// A revealable input is masked by default, so this implies
    /// [`password`](Self::password) — you do not need to set both. The toggle
    /// is a real focusable control: it is operable by mouse, by keyboard
    /// (Space / Enter when focused), and by screen readers (it announces
    /// "Reveal password" / "Hide password").
    ///
    /// The revealed-vs-masked state is held in egui memory keyed off the
    /// widget id and starts masked each session. As with flash and cursor
    /// state, pin the id with [`id_salt`](Self::id_salt) if the input's
    /// auto-generated id is not stable across frames.
    ///
    /// The toggle sits inside the field and shares its width, so the editable
    /// area is a little narrower than a plain input of the same
    /// [`desired_width`](Self::desired_width). The eye glyph comes from the
    /// bundled symbols font, which [`Theme::install`](crate::Theme::install)
    /// registers — install a theme (as you would for any elegance widget) or
    /// the toggle renders as a missing-glyph box.
    pub fn revealable(mut self, revealable: bool) -> Self {
        self.revealable = revealable;
        self
    }

    /// Desired width (points) for the editor portion of the widget.
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Supply a stable id salt. Useful when two inputs share the same label.
    pub fn id_salt(mut self, id: impl crate::IdSalt) -> Self {
        self.id_salt = Some(egui::Id::unique(id));
        self
    }

    /// Render with reduced vertical padding so the input matches the
    /// height of [`RemovableChip`](crate::RemovableChip) and small-size
    /// controls. Useful for inline path / chip rows where a full-height
    /// input would dominate. Default: `false` (standard control height).
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl<'a> Widget for TextInput<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        ui.vertical(|ui| {
            if let Some(label) = &self.label {
                ui.add_space(2.0);
                let rich = egui::RichText::new(label.text())
                    .color(p.text_muted)
                    .size(t.label);
                ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
                ui.add_space(2.0);
            }

            // Pin a stable id_salt so the TextEdit id is predictable —
            // we need it to look up flash state before painting.
            //
            // `TextEdit::id_salt` internally wraps its input in
            // `Id::unique(...)` before calling `make_persistent_id`, so we
            // mirror that step here to get the *same* widget id.
            let id_salt = self.id_salt.unwrap_or_else(|| ui.next_auto_id());
            let widget_id = ui.make_persistent_id(egui::Id::unique(id_salt));

            let flash = flash::active_flash(ui.ctx(), widget_id);
            let bg_fill = flash::background_fill(&theme, p.input_bg, flash);

            let desired_width = self.desired_width.unwrap_or_else(|| ui.available_width());
            // Compact mode shrinks the vertical padding to match
            // RemovableChip's editor and Button::size(Small) at ~22 pt
            // total. Horizontal padding is unchanged so the caret has
            // room either way.
            let margin_y = if self.compact {
                3.0
            } else {
                theme.control_padding_y
            };

            // A revealable field is masked until the user reveals it; that
            // state lives in egui memory keyed off the widget id so the
            // caller never threads a bool. `revealable` therefore implies
            // masking — `masked` collapses to `self.password` only for the
            // plain (non-revealable) path.
            let reveal_id = widget_id.with("elegance-reveal");
            let revealed =
                self.revealable && ui.data(|d| d.get_temp::<bool>(reveal_id).unwrap_or(false));
            let masked = if self.revealable {
                !revealed
            } else {
                self.password
            };

            // `frame_rect` is the outer box (editor + any reveal toggle). It
            // equals the editor's own rect on the plain path, and the combined
            // row on the revealable path; the dirty bar and visibility check
            // hang off it either way.
            let (response, frame_rect) = if self.revealable {
                render_revealable(RevealableArgs {
                    ui,
                    theme: &theme,
                    text: self.text,
                    hint: self.hint,
                    widget_id,
                    reveal_id,
                    bg_fill,
                    desired_width,
                    margin_y,
                    masked,
                    revealed,
                })
            } else {
                let margin = Vec2::new(theme.control_padding_x * 0.5, margin_y);

                // Swap visuals so the TextEdit picks up our look, then restore.
                let response = crate::theme::with_themed_visuals(ui, |ui| {
                    let v = ui.visuals_mut();
                    crate::theme::themed_input_visuals(v, &theme, bg_fill);
                    v.extreme_bg_color = bg_fill;
                    v.selection.bg_fill = crate::theme::with_alpha(p.focus, 90);
                    v.selection.stroke = Stroke::new(1.0, p.focus);

                    let mut edit = TextEdit::singleline(self.text)
                        .id_salt(id_salt)
                        .font(FontSelection::FontId(FontId::proportional(t.body)))
                        .text_color(p.text)
                        .margin(margin)
                        .desired_width(desired_width);
                    if let Some(hint) = self.hint {
                        edit = edit.hint_text(egui::RichText::new(hint).color(p.text_faint));
                    }
                    if masked {
                        edit = edit.password(true);
                    }

                    ui.add(edit)
                });

                let rect = response.rect;
                (response, rect)
            };

            if self.dirty && ui.is_rect_visible(frame_rect) {
                // Hug the inside of the border with a fixed geometry that does
                // *not* change across hover/focus. The bar is a status
                // indicator, not an interactive element — jittering it with the
                // cursor reads as a bug. All input states use a 1 pt stroke
                // (only the colour changes on focus), so inset = 1.0 matches
                // the inner edge of the border in every state, and the bar's
                // 5 pt inner corner radius matches the border's inner arc.
                let stroke_w = 1.0;
                let bar_w = 3.0;
                let r = (theme.control_radius - stroke_w).max(0.0) as u8;
                let bar = egui::Rect::from_min_max(
                    egui::pos2(frame_rect.min.x + stroke_w, frame_rect.min.y + stroke_w),
                    egui::pos2(
                        frame_rect.min.x + stroke_w + bar_w,
                        frame_rect.max.y - stroke_w,
                    ),
                );
                let corner = CornerRadius {
                    nw: r,
                    sw: r,
                    ne: 0,
                    se: 0,
                };
                ui.painter().rect_filled(bar, corner, p.focus);
            }

            // Expose the field label via accesskit so screen readers announce
            // the field purpose. TextEdit sets its own widget_info (with the
            // current text value as the label); replacing it with ours makes
            // the field queryable by its semantic label instead.
            if let Some(label) = &self.label {
                let label = label.text().to_string();
                response.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &label));
            }

            response
        })
        .inner
    }
}

/// Inputs to [`render_revealable`], grouped to keep the call site readable.
struct RevealableArgs<'a, 'u> {
    ui: &'u mut Ui,
    theme: &'u Theme,
    text: &'a mut String,
    hint: Option<&'a str>,
    widget_id: egui::Id,
    reveal_id: egui::Id,
    bg_fill: Color32,
    desired_width: f32,
    margin_y: f32,
    masked: bool,
    revealed: bool,
}

/// Render a [`TextInput`] with a trailing reveal toggle. Returns the editor's
/// [`Response`] (widened to the whole box) together with the outer frame rect.
///
/// Unlike the plain path, the editor can't own its border here: the eye sits
/// *inside* the same box, so the editor's frame is stripped and the border /
/// fill are painted manually around the `[editor | eye]` row (the same
/// approach [`RemovableChip`](crate::RemovableChip) uses for its close
/// button). The eye is a separate, non-overlapping click widget, so it never
/// fights the editor for the pointer.
fn render_revealable(args: RevealableArgs<'_, '_>) -> (Response, egui::Rect) {
    let RevealableArgs {
        ui,
        theme,
        text,
        hint,
        widget_id,
        reveal_id,
        bg_fill,
        desired_width,
        margin_y,
        masked,
        revealed,
    } = args;

    let p = &theme.palette;
    let t = &theme.typography;

    // `pad_x` matches the plain path's text inset (control_padding_x * 0.5) so
    // a revealable field lines up with its neighbours. The eye is a square
    // sized just above the body text, separated from the editor by `gap`.
    let pad_x = theme.control_padding_x * 0.5;
    let gap = 6.0;
    let eye_diam = t.body + 6.0;
    let editor_w = (desired_width - pad_x * 2.0 - gap - eye_diam).max(40.0);

    // Reserve the fill shape now so it can be painted *under* the editor and
    // eye once focus / hover (and therefore the border colour) are known.
    let bg_idx = ui.painter().add(Shape::Noop);

    let inner = ui.horizontal(|ui| {
        // Drive all spacing explicitly so the row width lands on
        // `desired_width`: pad + editor + gap + eye + pad.
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.add_space(pad_x);

        // The editor borrows the outer box's chrome, so strip its per-state
        // border and fill; only the manual frame below is visible.
        let edit_response = crate::theme::with_themed_visuals(ui, |ui| {
            let v = ui.visuals_mut();
            crate::theme::themed_input_visuals(v, theme, Color32::TRANSPARENT);
            v.extreme_bg_color = Color32::TRANSPARENT;
            for w in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.open,
            ] {
                w.bg_stroke = Stroke::NONE;
            }
            v.selection.bg_fill = crate::theme::with_alpha(p.focus, 90);
            v.selection.stroke = Stroke::new(1.0, p.focus);

            // Pin the editor's id *explicitly* to `widget_id`. The plain path
            // gets this for free (the editor is added on the same `ui` whose
            // `widget_id` we precomputed), but here the editor lives inside a
            // nested horizontal child `ui` — `id_salt` would derive a *different*
            // id from the child's id, desyncing the editor from the flash lookup
            // and focus check that key off `widget_id`. RemovableChip pins its
            // editor id the same way for the same reason.
            let mut edit = TextEdit::singleline(text)
                .id(widget_id)
                .font(FontSelection::FontId(FontId::proportional(t.body)))
                .text_color(p.text)
                .desired_width(editor_w)
                // Pass an explicit `.frame(...)`, not `.margin(...)`. With the
                // default frame, a *focused* TextEdit paints its own border
                // from `selection.stroke` (which we set for text selection) —
                // that would draw a second ring around just the editor, inside
                // our box. A custom frame suppresses that stroke; our manual
                // border is the only one. (RemovableChip does the same.)
                .frame(egui::Frame::new().inner_margin(Margin {
                    left: 0,
                    right: 0,
                    top: margin_y.round() as i8,
                    bottom: margin_y.round() as i8,
                }));
            if let Some(hint) = hint {
                edit = edit.hint_text(egui::RichText::new(hint).color(p.text_faint));
            }
            if masked {
                edit = edit.password(true);
            }
            ui.add(edit)
        });

        ui.add_space(gap);
        let (eye_rect, eye_resp) = ui.allocate_exact_size(Vec2::splat(eye_diam), Sense::click());
        ui.add_space(pad_x);

        (edit_response, eye_rect, eye_resp)
    });

    let (edit_response, eye_rect, eye_resp) = inner.inner;
    let frame_rect = inner.response.rect;

    // Toggle on any activation (mouse, Space / Enter while focused, or an
    // AccessKit click action). The flip takes effect next frame, when `masked`
    // is recomputed from the stored flag.
    if eye_resp.clicked() {
        let now = !revealed;
        ui.data_mut(|d| d.insert_temp(reveal_id, now));
        ui.ctx().request_repaint();
    }

    // Eye affordance: a subtle hover plate, then the eye (masked, "click to
    // reveal") or eye-off (revealed, "click to hide") glyph from the bundled
    // symbols font.
    let eye_hovered = eye_resp.hovered();
    if eye_hovered {
        ui.painter().rect_filled(
            eye_rect,
            CornerRadius::same(3),
            crate::theme::with_alpha(p.text_muted, 28),
        );
    }
    let glyph = if revealed {
        crate::glyphs::EYE_OFF
    } else {
        crate::glyphs::EYE
    };
    let glyph_color = if eye_hovered { p.text } else { p.text_muted };
    let galley =
        ui.painter()
            .layout_no_wrap(glyph.to_string(), FontId::proportional(t.body), glyph_color);
    let glyph_pos = eye_rect.center() - galley.size() * 0.5;
    ui.painter().galley(glyph_pos, galley, glyph_color);

    // Announce the toggle (and its current state) to assistive tech. Pairing
    // a Button role with a state-reflecting label is what makes the control
    // both screen-reader-operable and unambiguous to query in tests.
    eye_resp.widget_info(|| {
        WidgetInfo::labeled(
            Role::Button,
            true,
            if revealed {
                "Hide password"
            } else {
                "Reveal password"
            },
        )
    });

    // Paint the box underneath everything. Match the plain path's stroke
    // convention: a 1 pt stroke in every state, only the colour changes
    // (border at rest, text_muted on hover, focus accent when focused). The
    // plain TextEdit draws its focused border from `selection.stroke` — also
    // 1 pt (set on `input.rs`) — so mirroring that width here, rather than the
    // 1.5 pt `active` stroke, keeps a revealable field pixel-consistent with a
    // plain one beside it (and matches the dirty-bar inset, which assumes 1 pt
    // in every state). Focus follows the editor *or* the eye so tabbing into
    // the toggle keeps the field looking active.
    let frame_focused = edit_response.has_focus() || eye_resp.has_focus();
    let frame_hovered = ui.rect_contains_pointer(frame_rect);
    let border_color = if frame_focused {
        p.focus
    } else if frame_hovered {
        p.text_muted
    } else {
        p.border
    };
    let radius = CornerRadius::same(theme.control_radius as u8);
    ui.painter()
        .set(bg_idx, Shape::rect_filled(frame_rect, radius, bg_fill));
    ui.painter().rect_stroke(
        frame_rect,
        radius,
        Stroke::new(1.0, border_color),
        StrokeKind::Inside,
    );

    // Report the editor's response, but widened to the whole box so callers
    // see the same `rect` the plain path returns (e.g. for anchoring a popup).
    // `with_new_rect` keeps the editor's id and `changed()` flag intact, so
    // edit detection and `ResponseFlashExt` still work.
    (edit_response.with_new_rect(frame_rect), frame_rect)
}
