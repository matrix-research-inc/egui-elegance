//! Styled multi-line text input — the multiline companion to
//! [`TextInput`](crate::TextInput).
//!
//! Wraps [`egui::TextEdit::multiline`] with the same visual treatment as
//! `TextInput`: the slate input background, a 1-px border that turns
//! focus-accent on focus, optional label, hint, dirty bar, and submit-
//! flash feedback via [`ResponseFlashExt`](crate::ResponseFlashExt).

use egui::{
    CornerRadius, FontId, FontSelection, Response, Role, Stroke, TextEdit, Ui, Vec2, Widget,
    WidgetInfo, WidgetText,
};

use crate::{flash, theme::Theme};

/// A styled multi-line text input.
///
/// ```no_run
/// # use elegance::TextArea;
/// # egui::__run_test_ui(|ui| {
/// let mut notes = String::new();
/// ui.add(
///     TextArea::new(&mut notes)
///         .label("Notes")
///         .hint("Jot anything down…")
///         .rows(6),
/// );
/// # });
/// ```
///
/// # Ids, focus, and flash state
///
/// As with [`TextInput`](crate::TextInput), pin a stable
/// [`id_salt`](Self::id_salt) on any TextArea you plan to flash — otherwise
/// the id is layout-dependent and in-flight flash/focus/cursor state is
/// lost if siblings above this widget appear or disappear between frames.
#[must_use = "Add with `ui.add(...)`."]
pub struct TextArea<'a> {
    text: &'a mut String,
    label: Option<WidgetText>,
    hint: Option<&'a str>,
    dirty: bool,
    rows: usize,
    desired_width: Option<f32>,
    monospace: bool,
    id_salt: Option<egui::Id>,
}

impl<'a> std::fmt::Debug for TextArea<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextArea")
            .field("dirty", &self.dirty)
            .field("rows", &self.rows)
            .field("monospace", &self.monospace)
            .field("desired_width", &self.desired_width)
            .finish()
    }
}

impl<'a> TextArea<'a> {
    /// Create a multi-line text input bound to `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            label: None,
            hint: None,
            dirty: false,
            rows: 5,
            desired_width: None,
            monospace: false,
            id_salt: None,
        }
    }

    /// Show a label above the text area.
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

    /// Minimum visible row count. Default: `5`.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// Desired width (points) for the editor portion of the widget.
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    /// Render the text in the theme's monospace font. Useful for code,
    /// JSON blobs, PEM keys, and similar fixed-width content.
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    /// Supply a stable id salt. Required if you plan to flash this widget
    /// via [`ResponseFlashExt`](crate::ResponseFlashExt).
    pub fn id_salt(mut self, id: impl crate::IdSalt) -> Self {
        self.id_salt = Some(egui::Id::unique(id));
        self
    }
}

impl<'a> Widget for TextArea<'a> {
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

            let id_salt = self.id_salt.unwrap_or_else(|| ui.next_auto_id());
            let widget_id = ui.make_persistent_id(egui::Id::unique(id_salt));

            let flash = flash::active_flash(ui.ctx(), widget_id);
            let bg_fill = flash::background_fill(&theme, p.input_bg, flash);

            let desired_width = self.desired_width.unwrap_or_else(|| ui.available_width());
            let margin = Vec2::new(theme.control_padding_x * 0.5, theme.control_padding_y);

            let font_id = if self.monospace {
                FontId::monospace(t.monospace)
            } else {
                FontId::proportional(t.body)
            };

            let response = crate::theme::with_themed_visuals(ui, |ui| {
                let v = ui.visuals_mut();
                crate::theme::themed_input_visuals(v, &theme, bg_fill);
                v.extreme_bg_color = bg_fill;
                v.selection.bg_fill = crate::theme::with_alpha(p.focus, 90);
                v.selection.stroke = Stroke::new(1.0, p.focus);

                let mut edit = TextEdit::multiline(self.text)
                    .id_salt(id_salt)
                    .font(FontSelection::FontId(font_id))
                    .text_color(p.text)
                    .margin(margin)
                    .desired_width(desired_width)
                    .desired_rows(self.rows);
                if let Some(hint) = self.hint {
                    edit = edit.hint_text(egui::RichText::new(hint).color(p.text_faint));
                }

                ui.add(edit)
            });

            if self.dirty && ui.is_rect_visible(response.rect) {
                // Inset by the frame stroke so the bar sits *inside* the border,
                // and use the frame's inner radius so the bar's rounded left edge
                // follows the same arc as the inside of the input's rounded corner.
                let stroke_w = 1.0;
                let bar_w = 3.0;
                let r = ((theme.control_radius - stroke_w).max(0.0)) as u8;
                let bar = egui::Rect::from_min_max(
                    egui::pos2(
                        response.rect.min.x + stroke_w,
                        response.rect.min.y + stroke_w,
                    ),
                    egui::pos2(
                        response.rect.min.x + stroke_w + bar_w,
                        response.rect.max.y - stroke_w,
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

            if let Some(label) = &self.label {
                let label = label.text().to_string();
                response.widget_info(|| WidgetInfo::labeled(Role::TextInput, true, &label));
            }

            response
        })
        .inner
    }
}
