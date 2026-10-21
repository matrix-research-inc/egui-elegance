//! Status indicator light — a small circle with three visual states.

use egui::{CornerRadius, Response, Role, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo};

use crate::theme::{Theme, with_alpha};

/// The three visual states of an [`Indicator`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndicatorState {
    /// Connected / healthy — solid green dot with a subtle glow.
    On,
    /// Disconnected / error — red horizontal bar inside a ring.
    Off,
    /// Transient / in-progress — amber ring only.
    Connecting,
}

/// A small status light.
#[derive(Debug, Clone, Copy)]
#[must_use = "Add with `ui.add(...)`."]
pub struct Indicator {
    state: IndicatorState,
    size: f32,
}

impl Indicator {
    /// Create an indicator in the given state. Default size: 10 points.
    pub fn new(state: IndicatorState) -> Self {
        Self { state, size: 10.0 }
    }

    /// Override the indicator diameter in points.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Widget for Indicator {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;

        let size = Vec2::splat(self.size);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let c = rect.center();
            let r = self.size * 0.5;

            match self.state {
                IndicatorState::On => {
                    // Soft outer glow.
                    painter.circle_filled(c, r + 1.5, with_alpha(p.success, 70));
                    painter.circle_filled(c, r, p.success);
                }
                IndicatorState::Off => {
                    painter.circle_stroke(c, r - 0.5, Stroke::new(1.0, p.danger));
                    // Horizontal bar inside.
                    let bar_w = self.size * 0.7;
                    let bar_h = 2.0;
                    let bar = egui::Rect::from_center_size(c, Vec2::new(bar_w, bar_h));
                    painter.rect_filled(bar, CornerRadius::same(1), p.danger);
                }
                IndicatorState::Connecting => {
                    painter.circle_stroke(c, r - 0.5, Stroke::new(1.8, p.warning));
                }
            }
        }

        response.widget_info(|| {
            WidgetInfo::labeled(
                Role::Unknown,
                true,
                match self.state {
                    IndicatorState::On => "status: on",
                    IndicatorState::Off => "status: off",
                    IndicatorState::Connecting => "status: connecting",
                },
            )
        });
        response
    }
}
