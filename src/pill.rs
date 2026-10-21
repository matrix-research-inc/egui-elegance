//! Status pill — a rounded capsule of small labelled status lights.

use egui::{
    CornerRadius, Margin, Response, Role, Stroke, Ui, Widget, WidgetInfo, WidgetText, vec2,
};

use crate::{
    indicator::{Indicator, IndicatorState},
    theme::Theme,
};

/// A capsule-shaped row of `(label, state)` status items.
///
/// ```no_run
/// # use elegance::{StatusPill, IndicatorState};
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     StatusPill::new()
///         .item("UI", IndicatorState::On)
///         .item("API", IndicatorState::Connecting)
///         .item("DB", IndicatorState::Off),
/// );
/// # });
/// ```
#[derive(Default)]
#[must_use = "Add with `ui.add(...)`."]
pub struct StatusPill {
    items: Vec<(WidgetText, IndicatorState)>,
}

impl std::fmt::Debug for StatusPill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusPill")
            .field(
                "items",
                &self
                    .items
                    .iter()
                    .map(|(l, s)| (l.text().to_string(), *s))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl StatusPill {
    /// Create an empty status pill. Add rows with [`StatusPill::item`].
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Append a `(label, state)` row to the pill.
    pub fn item(mut self, label: impl Into<WidgetText>, state: IndicatorState) -> Self {
        self.items.push((label.into(), state));
        self
    }
}

impl Widget for StatusPill {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let frame = egui::Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(99))
            .inner_margin(Margin::symmetric(12, 4));

        let response = frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
                    for (i, (label, state)) in self.items.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(4.0);
                        }
                        ui.add(Indicator::new(*state));
                        let rt = egui::RichText::new(label.text())
                            .color(p.text_faint)
                            .size(t.small);
                        ui.add(egui::Label::new(rt).wrap_mode(egui::TextWrapMode::Extend));
                    }
                });
            })
            .response;

        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "status pill"));
        response
    }
}
