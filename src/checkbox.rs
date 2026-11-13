//! A checkbox with a focus accent.

use egui::{
    Color32, CornerRadius, FontSelection, Response, Role, Sense, Stroke, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, accesskit, vec2,
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
///
/// To draw the box alone, as in a table's header row, give it a label and
/// call [`Checkbox::hide_label`], so it still has an accessible name.
#[must_use = "Add this widget with `ui.add(...)`."]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: WidgetText,
    hide_label: bool,
    indeterminate: bool,
}

impl<'a> std::fmt::Debug for Checkbox<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("checked", &*self.checked)
            .field("label", &self.label.text())
            .field("hide_label", &self.hide_label)
            .field("indeterminate", &self.indeterminate)
            .finish()
    }
}

impl<'a> Checkbox<'a> {
    /// Create a checkbox bound to `checked` with the given label.
    pub fn new(checked: &'a mut bool, label: impl Into<WidgetText>) -> Self {
        Self {
            checked,
            label: label.into(),
            hide_label: false,
            indeterminate: false,
        }
    }

    /// Draw the box without its label. The label is still the box's
    /// accessible name, so pass what it controls ("Select all rows").
    pub fn hide_label(mut self) -> Self {
        self.hide_label = true;
        self
    }

    /// Show the box as partly selected, with a dash instead of a check, as
    /// a "select all" box does when only some items are selected.
    ///
    /// This only changes how the box looks: a click still toggles
    /// `checked`. For a "select all" box, pass `checked` as "all selected",
    /// so a click on a partly selected box selects everything.
    ///
    /// ```no_run
    /// # use elegance::Checkbox;
    /// # egui::__run_test_ui(|ui| {
    /// # let mut rows = vec![true, false, true];
    /// let selected = rows.iter().filter(|&&on| on).count();
    /// let mut all = selected == rows.len();
    /// let partly = selected > 0 && !all;
    /// let select_all = Checkbox::new(&mut all, "Select all rows")
    ///     .hide_label()
    ///     .indeterminate(partly);
    /// if ui.add(select_all).changed() {
    ///     rows.iter_mut().for_each(|on| *on = all);
    /// }
    /// # });
    /// ```
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let box_size = 14.0;
        let gap = 6.0;

        let label = self.label.text();
        let galley = (!self.hide_label && !label.is_empty()).then(|| {
            egui::WidgetText::from(egui::RichText::new(label).color(p.text).size(t.body))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    ui.available_width(),
                    FontSelection::FontId(egui::FontId::proportional(t.body)),
                )
        });
        let desired = match &galley {
            Some(g) => vec2(box_size + gap + g.size().x, box_size.max(g.size().y)),
            None => Vec2::splat(box_size),
        };
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());

        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let filled = *self.checked || self.indeterminate;
            let is_hovered = response.hovered();

            let box_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - box_size * 0.5),
                Vec2::splat(box_size),
            );

            let (fill, stroke) = if filled {
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

            let m = box_rect.min;
            let s = box_size;
            let mark = Stroke::new(1.6, Color32::WHITE);
            if self.indeterminate {
                ui.painter()
                    .hline(m.x + s * 0.25..=m.x + s * 0.75, box_rect.center().y, mark);
            } else if *self.checked {
                // Draw a crisp checkmark.
                let a = egui::pos2(m.x + s * 0.22, m.y + s * 0.52);
                let b = egui::pos2(m.x + s * 0.44, m.y + s * 0.72);
                let c = egui::pos2(m.x + s * 0.78, m.y + s * 0.30);
                ui.painter().line_segment([a, b], mark);
                ui.painter().line_segment([b, c], mark);
            }

            if let Some(galley) = galley {
                let text_pos = egui::pos2(
                    box_rect.max.x + gap,
                    rect.center().y - galley.size().y * 0.5,
                );
                ui.painter().galley(text_pos, galley, p.text);
            }
        }

        if self.indeterminate {
            // Neither checked nor unchecked, so no selected state in the
            // output event; accesskit has a state for it.
            response.widget_info(|| WidgetInfo::labeled(Role::CheckBox, ui.is_enabled(), label));
            response.ctx.accesskit_node_builder(response.id, |node| {
                node.set_toggled(accesskit::Toggled::Mixed);
            });
        } else {
            response.widget_info(|| {
                WidgetInfo::selected(Role::CheckBox, ui.is_enabled(), *self.checked, label)
            });
        }
        response
    }
}
