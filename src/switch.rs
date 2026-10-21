//! A sliding on/off switch — the boolean toggle for "turn this feature on."
//!
//! Visually distinct from [`Checkbox`](crate::Checkbox): a capsule track with
//! an animated knob that slides between off and on. Use it for settings and
//! feature flags; use `Checkbox` for "select this item in a list."
//!
//! ```no_run
//! # use elegance::{Accent, Switch};
//! # egui::__run_test_ui(|ui| {
//! let mut notify = true;
//! ui.add(Switch::new(&mut notify, "Notify on Slack").accent(Accent::Green));
//! # });
//! ```
//!
//! Clicking anywhere on the switch or its label toggles the bound boolean.
//! The knob transition is animated via [`egui::Context::animate_bool_responsive`].

use egui::{
    Color32, CornerRadius, FontSelection, Response, Role, Sense, Stroke, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, pos2, vec2,
};

use crate::Accent;
use crate::theme::{Theme, mix, with_alpha};

/// A sliding on/off switch bound to a `&mut bool`.
#[must_use = "Add this widget with `ui.add(...)`."]
pub struct Switch<'a> {
    state: &'a mut bool,
    label: WidgetText,
    accent: Accent,
    enabled: bool,
}

impl<'a> std::fmt::Debug for Switch<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switch")
            .field("state", &*self.state)
            .field("label", &self.label.text())
            .field("accent", &self.accent)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl<'a> Switch<'a> {
    /// Create a switch bound to `state` with the given label.
    ///
    /// Pass `""` for the label if the switch is rendered alongside a
    /// separately-laid-out caption (e.g., in a settings row).
    pub fn new(state: &'a mut bool, label: impl Into<WidgetText>) -> Self {
        Self {
            state,
            label: label.into(),
            accent: Accent::Sky,
            enabled: true,
        }
    }

    /// Colour the "on" state with the given accent. Default: [`Accent::Sky`].
    pub fn accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Disable the switch. Disabled switches do not respond to clicks and
    /// render with muted colours.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a> Widget for Switch<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let track_w: f32 = 32.0;
        let track_h: f32 = 18.0;
        let knob_pad: f32 = 2.0;
        let knob_d: f32 = track_h - knob_pad * 2.0;
        let gap: f32 = 8.0;

        let label_text = self.label.text();
        let has_label = !label_text.is_empty();

        let galley = has_label.then(|| {
            egui::WidgetText::from(egui::RichText::new(label_text).color(p.text).size(t.body))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    ui.available_width(),
                    FontSelection::FontId(egui::FontId::proportional(t.body)),
                )
        });

        let text_size = galley.as_ref().map_or(Vec2::ZERO, |g| g.size());
        let desired = vec2(
            track_w + if has_label { gap + text_size.x } else { 0.0 },
            track_h.max(text_size.y),
        );

        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(desired, sense);

        if self.enabled && response.clicked() {
            *self.state = !*self.state;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            let on = *self.state;
            let progress = ui.ctx().animate_bool_responsive(response.id, on);

            let track_rect = egui::Rect::from_min_size(
                pos2(rect.min.x, rect.center().y - track_h * 0.5),
                vec2(track_w, track_h),
            );

            let accent = p.accent_fill(self.accent);
            let hovered = self.enabled && response.hovered();

            let off_fill = p.input_bg;
            let track_fill = if !self.enabled {
                with_alpha(off_fill, 160)
            } else {
                mix(off_fill, accent, progress)
            };
            let stroke_color = if !self.enabled {
                with_alpha(p.border, 160)
            } else if progress > 0.05 {
                mix(p.border, accent, progress)
            } else if hovered {
                p.focus
            } else {
                p.border
            };

            ui.painter().rect(
                track_rect,
                CornerRadius::same((track_h * 0.5) as u8),
                track_fill,
                Stroke::new(1.0, stroke_color),
                egui::StrokeKind::Inside,
            );

            let travel = track_w - knob_d - knob_pad * 2.0;
            let knob_center = pos2(
                track_rect.min.x + knob_pad + knob_d * 0.5 + travel * progress,
                track_rect.center().y,
            );
            let knob_color = if self.enabled {
                if p.is_dark {
                    Color32::WHITE
                } else {
                    // Light themes: off-track is white, so a white knob would
                    // vanish. Fade a muted-grey knob to white as the switch
                    // turns on, where the track turns saturated.
                    mix(p.text_muted, Color32::WHITE, progress)
                }
            } else {
                p.text_muted
            };
            ui.painter()
                .circle_filled(knob_center, knob_d * 0.5, knob_color);

            if let Some(g) = galley {
                let text_pos = pos2(track_rect.max.x + gap, rect.center().y - text_size.y * 0.5);
                let color = if self.enabled { p.text } else { p.text_faint };
                ui.painter().galley(text_pos, g, color);
            }
        }

        response
            .widget_info(|| WidgetInfo::labeled(Role::CheckBox, self.enabled, self.label.text()));
        response
    }
}
