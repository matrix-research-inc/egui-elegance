//! File drop zone — a dashed-bordered surface that accepts dropped files.
//!
//! A [`FileDropZone`] renders a click-and-drop target with a cloud icon,
//! prompt text, and a hint line. The zone responds visually when files are
//! dragged over it and reports any files dropped on its rect via the
//! returned [`FileDropResponse`]. The widget itself is stateless: the caller
//! either consumes the dropped files immediately or stores them in their
//! own app state.

use egui::{
    Color32, CornerRadius, DroppedFileHandle, FontSelection, Pos2, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetText, pos2, vec2,
};

use crate::glyphs::UPLOAD as UPLOAD_GLYPH;
use crate::theme::{Theme, with_alpha};

/// A click-and-drop file target.
///
/// ```no_run
/// # use elegance::FileDropZone;
/// # egui::__run_test_ui(|ui| {
/// let drop = FileDropZone::new().show(ui);
/// if drop.response.clicked() {
///     // Open a native file picker, e.g. via the `rfd` crate.
/// }
/// for file in &drop.dropped_files {
///     // Inspect `file.path()`, or read the contents with `file.bytes()`
///     // (`file.bytes_async()` on the web).
///     let _ = file.path();
/// }
/// # });
/// ```
#[must_use = "Call `.show(ui)` to render the drop zone."]
pub struct FileDropZone {
    prompt: Option<WidgetText>,
    action_word: Option<String>,
    hint: Option<WidgetText>,
    min_height: f32,
    enabled: bool,
}

impl std::fmt::Debug for FileDropZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileDropZone")
            .field("prompt", &self.prompt.as_ref().map(|p| p.text()))
            .field("action_word", &self.action_word)
            .field("hint", &self.hint.as_ref().map(|h| h.text()))
            .field("min_height", &self.min_height)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl Default for FileDropZone {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDropZone {
    /// Create a drop zone with the default prompt, action word, and minimum
    /// height.
    pub fn new() -> Self {
        Self {
            prompt: None,
            action_word: None,
            hint: None,
            min_height: 120.0,
            enabled: true,
        }
    }

    /// Override the prompt text. Defaults to "Drop files here, or browse",
    /// where the action word is rendered in the focus accent.
    #[inline]
    pub fn prompt(mut self, prompt: impl Into<WidgetText>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Override the action word rendered in the focus accent inside the
    /// prompt. Defaults to `"browse"`. Ignored if [`Self::prompt`] is set.
    #[inline]
    pub fn action_word(mut self, word: impl Into<String>) -> Self {
        self.action_word = Some(word.into());
        self
    }

    /// Set the hint line under the prompt (e.g. accepted formats and size
    /// limits). Defaults to no hint.
    #[inline]
    pub fn hint(mut self, hint: impl Into<WidgetText>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Override the minimum height of the zone in points. Defaults to 120.
    #[inline]
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = h;
        self
    }

    /// Disable the zone. Defaults to enabled.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Render the zone and return the response plus any files dropped on
    /// its rect this frame.
    pub fn show(self, ui: &mut Ui) -> FileDropResponse {
        let theme = Theme::current(ui.ctx());

        let action_word = self.action_word.as_deref().unwrap_or("browse");
        let prompt_text = self
            .prompt
            .as_ref()
            .map(|w| w.text().to_string())
            .unwrap_or_else(|| format!("Drop files here, or {action_word}"));
        let hint_text = self.hint.as_ref().map(|w| w.text().to_string());
        let a11y_label = prompt_text.clone();

        let desired = vec2(ui.available_width(), self.min_height);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(desired, sense);

        let (files_dragging, pointer) = ui
            .ctx()
            .input(|i| (!i.raw.hovered_files.is_empty(), i.pointer.interact_pos()));
        let pointer_in_rect = pointer.is_some_and(|pos| rect.contains(pos));
        let dragover = self.enabled && files_dragging && pointer_in_rect;

        let dropped_files = if self.enabled {
            ui.ctx().input(|i| {
                if i.raw.dropped_files.is_empty() {
                    return Vec::new();
                }
                if pointer_in_rect {
                    i.raw.dropped_files.clone()
                } else {
                    Vec::new()
                }
            })
        } else {
            Vec::new()
        };
        if !dropped_files.is_empty() {
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            paint_zone(
                ui,
                &theme,
                rect,
                &response,
                dragover,
                self.enabled,
                &prompt_text,
                action_word,
                hint_text.as_deref(),
            );
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Button, self.enabled, &a11y_label));

        FileDropResponse {
            response,
            dropped_files,
        }
    }
}

/// The result of rendering a [`FileDropZone`].
#[derive(Debug)]
pub struct FileDropResponse {
    /// The underlying egui [`Response`]: use `.clicked()` to open a picker
    /// and `.has_focus()` for keyboard handling.
    pub response: Response,
    /// Files dropped on the zone this frame. Empty when nothing was
    /// dropped or the drop landed outside the zone's rect.
    ///
    /// Each handle is an `Arc<dyn DroppedFile>` owned by the integration:
    /// read the contents with [`egui::DroppedFile::bytes`] on native, or
    /// `bytes_async` on the web.
    pub dropped_files: Vec<DroppedFileHandle>,
}

#[allow(clippy::too_many_arguments)]
fn paint_zone(
    ui: &Ui,
    theme: &Theme,
    rect: Rect,
    response: &Response,
    dragover: bool,
    enabled: bool,
    prompt: &str,
    action_word: &str,
    hint: Option<&str>,
) {
    let p = &theme.palette;
    let t = &theme.typography;

    let radius = CornerRadius::same(theme.card_radius as u8);
    let painter = ui.painter();

    let hovered = enabled && response.hovered();
    let focused = enabled && response.has_focus();

    // Background fill. Subtle by default; focus-tinted while a file is dragged
    // over the zone.
    let fill = if !enabled {
        Color32::TRANSPARENT
    } else if dragover {
        with_alpha(p.focus, 26)
    } else {
        p.depth_tint(p.card, 0.015)
    };
    painter.rect(rect, radius, fill, Stroke::NONE, StrokeKind::Inside);

    // Dashed border. Sharp corners on the dashed polyline read fine against
    // the rounded fill underneath.
    let border_color = if !enabled {
        with_alpha(p.border, 160)
    } else if dragover {
        p.focus
    } else if hovered || focused {
        p.text_muted
    } else {
        p.border
    };
    let border_stroke = Stroke::new(1.5, border_color);
    let pts = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    painter.extend(egui::Shape::dashed_line(&pts, border_stroke, 6.0, 4.0));

    if focused {
        // Outer focus ring, mirroring the rest of the elegance widgets.
        painter.rect_stroke(
            rect.expand(2.0),
            radius,
            Stroke::new(2.0, with_alpha(p.focus, 180)),
            StrokeKind::Outside,
        );
    }

    // Lay out the icon, prompt, and hint as a centered stack.
    let icon_diameter = 44.0;
    let icon_gap = 12.0;
    let prompt_gap = 4.0;

    let prompt_color = if !enabled {
        p.text_muted
    } else if dragover {
        p.focus
    } else {
        p.text
    };
    let prompt_galley =
        egui::WidgetText::from(egui::RichText::new(prompt).color(prompt_color).size(t.body))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                rect.width() - 24.0,
                FontSelection::FontId(egui::FontId::proportional(t.body)),
            );

    let hint_galley = hint.map(|h| {
        egui::WidgetText::from(egui::RichText::new(h).color(p.text_faint).size(t.small))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                rect.width() - 24.0,
                FontSelection::FontId(egui::FontId::proportional(t.small)),
            )
    });

    let total_h = icon_diameter
        + icon_gap
        + prompt_galley.size().y
        + hint_galley
            .as_ref()
            .map(|g| prompt_gap + g.size().y)
            .unwrap_or(0.0);
    let mut cursor_y = rect.center().y - total_h * 0.5;

    // Icon circle.
    let icon_center = pos2(rect.center().x, cursor_y + icon_diameter * 0.5);
    let icon_color = if !enabled {
        p.text_faint
    } else if dragover {
        p.focus
    } else {
        p.text_muted
    };
    let icon_bg = if dragover {
        with_alpha(p.focus, 30)
    } else {
        p.input_bg
    };
    let icon_stroke_color = if dragover {
        with_alpha(p.focus, 115)
    } else {
        p.border
    };
    painter.circle(
        icon_center,
        icon_diameter * 0.5,
        icon_bg,
        Stroke::new(1.0, icon_stroke_color),
    );
    let glyph_size = icon_diameter * 0.7;
    let font_id = egui::FontId::proportional(glyph_size);
    let galley = painter.layout_no_wrap(UPLOAD_GLYPH.to_string(), font_id, icon_color);
    // Center the actual ink bounding box (`mesh_bounds`) on `icon_center`
    // rather than the line-box (`galley.size()`) — the line-box includes
    // empty descender space that throws cap-height-aligned icon glyphs
    // off visual center.
    let ink_center = galley.mesh_bounds.center();
    let pos = pos2(icon_center.x - ink_center.x, icon_center.y - ink_center.y);
    painter.galley(pos, galley, icon_color);
    cursor_y += icon_diameter + icon_gap;

    // Prompt text. If we generated the default prompt, draw the action
    // word in the focus accent instead of the body colour.
    let prompt_size = prompt_galley.size();
    let prompt_pos = pos2(rect.center().x - prompt_size.x * 0.5, cursor_y);
    if enabled && !dragover {
        if let Some((before, after)) = split_around(prompt, action_word) {
            paint_split_prompt(
                ui,
                theme,
                prompt_pos,
                prompt_size,
                before,
                action_word,
                after,
            );
        } else {
            painter.galley(prompt_pos, prompt_galley, p.text);
        }
    } else {
        painter.galley(prompt_pos, prompt_galley, prompt_color);
    }
    cursor_y += prompt_size.y + prompt_gap;

    if let Some(hint_g) = hint_galley {
        let hint_size = hint_g.size();
        painter.galley(
            pos2(rect.center().x - hint_size.x * 0.5, cursor_y),
            hint_g,
            p.text_faint,
        );
    }
}

fn split_around<'a>(prompt: &'a str, word: &str) -> Option<(&'a str, &'a str)> {
    let idx = prompt.find(word)?;
    Some((&prompt[..idx], &prompt[idx + word.len()..]))
}

fn paint_split_prompt(
    ui: &Ui,
    theme: &Theme,
    base: Pos2,
    full_size: Vec2,
    before: &str,
    accent_word: &str,
    after: &str,
) {
    let p = &theme.palette;
    let size = theme.typography.body;
    let font = egui::FontId::proportional(size);

    let layout = |s: &str, color: Color32| {
        egui::WidgetText::from(egui::RichText::new(s).color(color).size(size)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            FontSelection::FontId(font.clone()),
        )
    };

    let before_g = layout(before, p.text);
    let word_g = layout(accent_word, p.focus);
    let after_g = layout(after, p.text);

    let baseline_y = base.y + (full_size.y - before_g.size().y) * 0.5;
    let mut x = base.x;
    let painter = ui.painter();
    painter.galley(pos2(x, baseline_y), before_g.clone(), p.text);
    x += before_g.size().x;
    painter.galley(pos2(x, baseline_y), word_g.clone(), p.focus);
    x += word_g.size().x;
    painter.galley(pos2(x, baseline_y), after_g, p.text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_around_works() {
        assert_eq!(
            split_around("Drop files here, or browse", "browse"),
            Some(("Drop files here, or ", ""))
        );
        assert_eq!(
            split_around("Click to browse files", "browse"),
            Some(("Click to ", " files"))
        );
        assert_eq!(split_around("nothing here", "missing"), None);
    }
}
