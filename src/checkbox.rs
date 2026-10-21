//! A checkbox with a focus accent.

use egui::{
    Color32, CornerRadius, FontSelection, Response, Role, Sense, Stroke, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, vec2,
};

use crate::theme::Theme;

/// A styled checkbox.
///
/// ```no_run
/// # use elegance::Checkbox;
/// # egui::__run_test_ui(|ui| {
/// let mut enabled = false;
/// ui.add(Checkbox::new(&mut enabled, "Enable feature"));
/// # });
/// ```
#[must_use = "Add this widget with `ui.add(...)`."]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: WidgetText,
}

impl<'a> std::fmt::Debug for Checkbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("checked", &*self.checked)
            .field("label", &self.label.text())
            .finish()
    }
}

impl<'a> Checkbox<'a> {
    /// Create a checkbox bound to `checked` with the given label.
    pub fn new(checked: &'a mut bool, label: impl Into<WidgetText>) -> Self {
        Self {
            checked,
            label: label.into(),
        }
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let box_size = 14.0;
        let gap = 6.0;

        let galley = egui::WidgetText::from(
            egui::RichText::new(self.label.text())
                .color(p.text)
                .size(t.body),
        )
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            ui.available_width(),
            FontSelection::FontId(egui::FontId::proportional(t.body)),
        );

        let text_size = galley.size();
        let desired = vec2(box_size + gap + text_size.x, box_size.max(text_size.y));
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let checked = *self.checked;
            let is_hovered = response.hovered();

            let box_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - box_size * 0.5),
                Vec2::splat(box_size),
            );

            let (fill, stroke) = if checked {
                (p.focus, Stroke::new(1.0, p.focus))
            } else if is_hovered {
                (p.input_bg, Stroke::new(1.0, p.focus))
            } else {
                (p.input_bg, Stroke::new(1.0, p.border))
            };

            ui.painter().rect(
                box_rect,
                CornerRadius::same(3),
                fill,
                stroke,
                egui::StrokeKind::Inside,
            );

            if checked {
                // Draw a crisp checkmark.
                let m = box_rect.min;
                let s = box_size;
                let a = egui::pos2(m.x + s * 0.22, m.y + s * 0.52);
                let b = egui::pos2(m.x + s * 0.44, m.y + s * 0.72);
                let c = egui::pos2(m.x + s * 0.78, m.y + s * 0.30);
                let stroke = Stroke::new(1.6, Color32::WHITE);
                ui.painter().line_segment([a, b], stroke);
                ui.painter().line_segment([b, c], stroke);
            }

            let text_pos = egui::pos2(box_rect.max.x + gap, rect.center().y - text_size.y * 0.5);
            ui.painter().galley(text_pos, galley, p.text);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::CheckBox, true, self.label.text()));
        response
    }
}
