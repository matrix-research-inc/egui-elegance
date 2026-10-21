//! Popover — a click-anchored floating panel that points at a trigger.
//!
//! A [`Popover`] opens a themed, bordered panel next to a trigger
//! [`Response`] when the trigger is clicked. The panel has an optional
//! title row, a free-form body (filled by the caller's closure), and a
//! small arrow that visually connects the panel to the trigger. Close
//! behaviour matches the platform convention: click outside, press
//! `Esc`, or click the trigger again.
//!
//! ```no_run
//! # use elegance::{Button, Popover, PopoverSide};
//! # egui::__run_test_ui(|ui| {
//! let trigger = ui.add(Button::new("Filter"));
//! Popover::new("filters")
//!     .side(PopoverSide::Bottom)
//!     .title("Filter results")
//!     .show(&trigger, |ui| {
//!         ui.label("…");
//!     });
//! # });
//! ```
//!
//! Popovers are lighter than [`Modal`](crate::Modal): they don't dim the
//! background or trap focus. Reach for a [`Modal`](crate::Modal) when the
//! user must respond before continuing; reach for a [`Popover`] for
//! inline settings, confirmations, or rich hover-cards.

use egui::{
    Color32, CornerRadius, Frame, Id, InnerResponse, Margin, Pos2, Rect, Response, Shape, Stroke,
    Ui, Vec2, WidgetText, emath::RectAlign,
};

use crate::theme::Theme;

/// Which side of the trigger the popover opens on.
///
/// The popover will try the requested side first and fall back to the
/// opposite side if the requested placement doesn't fit in the viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopoverSide {
    /// Opens above the trigger; arrow points down.
    Top,
    /// Opens below the trigger; arrow points up. The default.
    Bottom,
    /// Opens to the left of the trigger; arrow points right.
    Left,
    /// Opens to the right of the trigger; arrow points left.
    Right,
}

impl PopoverSide {
    fn to_rect_align(self) -> RectAlign {
        match self {
            PopoverSide::Top => RectAlign::TOP,
            PopoverSide::Bottom => RectAlign::BOTTOM,
            PopoverSide::Left => RectAlign::LEFT,
            PopoverSide::Right => RectAlign::RIGHT,
        }
    }
}

/// A click-to-toggle popover anchored to a trigger [`Response`].
///
/// Call [`Popover::show`] immediately after painting the trigger; the
/// popover toggles open on trigger clicks and closes on outside-click,
/// `Esc`, or a subsequent trigger click.
#[derive(Debug, Clone)]
#[must_use = "Call `.show(&trigger, |ui| ...)` to render the popover."]
pub struct Popover {
    id_salt: Id,
    side: PopoverSide,
    title: Option<WidgetText>,
    width: Option<f32>,
    min_width: f32,
    gap: f32,
    arrow: bool,
}

impl Popover {
    /// Create a popover keyed by `id_salt`. The salt is used to persist
    /// the open/closed state across frames and must be stable for the
    /// trigger it's attached to.
    pub fn new(id_salt: impl crate::IdSalt) -> Self {
        Self {
            id_salt: Self::popup_id(id_salt),
            side: PopoverSide::Bottom,
            title: None,
            width: None,
            min_width: 200.0,
            gap: 8.0,
            arrow: true,
        }
    }

    /// The internal popup id for a given `id_salt`.
    ///
    /// Use this with [`egui::Popup::open_id`] / [`egui::Popup::close_id`]
    /// to open or close a popover programmatically (for example, from a
    /// keyboard shortcut or a test harness).
    pub fn popup_id(id_salt: impl crate::IdSalt) -> Id {
        Id::unique(("elegance::popover", Id::unique(id_salt)))
    }

    /// Which side of the trigger to anchor on. Default: [`PopoverSide::Bottom`].
    #[inline]
    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    /// Add a strong title row above the body.
    #[inline]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Fix the popover's content width. When unset, the popover sizes
    /// itself to the content and its `min_width`.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Minimum content width in points. Default: 200.
    #[inline]
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Gap between the trigger and the popover, in points. Default: 8.
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Toggle the small arrow that points at the trigger. Default: on.
    #[inline]
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    /// Render the popover attached to `trigger`. Returns `Some` with the
    /// body closure's return value while the popover is open, `None`
    /// while it is closed.
    pub fn show<R>(
        self,
        trigger: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let theme = Theme::current(&trigger.ctx);
        let p = &theme.palette;

        let popup_id = self.id_salt;
        let side = self.side;
        let title = self.title;
        let arrow = self.arrow;
        let width = self.width;
        let min_width = self.min_width;

        let frame = Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(theme.card_radius as u8))
            .inner_margin(Margin::same(12));

        let mut popup = egui::Popup::from_toggle_button_response(trigger)
            .id(popup_id)
            .align(side.to_rect_align())
            .align_alternatives(&[])
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .gap(self.gap)
            .frame(frame);
        if let Some(w) = width {
            popup = popup.width(w);
        }

        let trigger_rect = trigger.rect;
        let trigger_ctx = trigger.ctx.clone();

        let response = popup.show(move |ui| {
            ui.set_min_width(min_width);
            if let Some(h) = &title {
                let t = Theme::current(ui.ctx());
                let rt = egui::RichText::new(h.text())
                    .color(t.palette.text)
                    .size(t.typography.body)
                    .strong();
                ui.add(egui::Label::new(rt).wrap_mode(egui::TextWrapMode::Extend));
                ui.add_space(4.0);
            }
            add_contents(ui)
        });

        let inner = response?;

        if arrow {
            // Determine the actual side used. When the popup runs out of
            // room on the requested side egui flips it; infer the side
            // from the popup rect vs. the trigger rect.
            let actual_side = detect_side(trigger_rect, inner.response.rect, side);
            paint_arrow(
                &trigger_ctx,
                inner.response.layer_id,
                inner.response.rect,
                trigger_rect,
                actual_side,
                p.card,
                p.border,
            );
        }

        Some(inner)
    }
}

fn detect_side(trigger: Rect, popup: Rect, requested: PopoverSide) -> PopoverSide {
    match requested {
        PopoverSide::Top | PopoverSide::Bottom => {
            if popup.center().y < trigger.center().y {
                PopoverSide::Top
            } else {
                PopoverSide::Bottom
            }
        }
        PopoverSide::Left | PopoverSide::Right => {
            if popup.center().x < trigger.center().x {
                PopoverSide::Left
            } else {
                PopoverSide::Right
            }
        }
    }
}

fn paint_arrow(
    ctx: &egui::Context,
    layer: egui::LayerId,
    popup: Rect,
    trigger: Rect,
    side: PopoverSide,
    fill: Color32,
    border: Color32,
) {
    let painter = ctx.layer_painter(layer);

    // Half-base and height of the isoceles arrow triangle.
    let half_base = 6.0;
    let depth = 6.0;
    let inset = 10.0; // Keep the arrow away from the rounded corner.

    // Axis along which the arrow's base runs (`base_center` lies on the
    // popup edge adjacent to the trigger), the perpendicular direction
    // (pointing from the popup toward the trigger) and the base-axis
    // direction used to lay out the triangle's footprint.
    let (base_center, perp, base_axis) = match side {
        PopoverSide::Bottom => {
            let cx = trigger
                .center()
                .x
                .clamp(popup.min.x + inset, popup.max.x - inset);
            (
                Pos2::new(cx, popup.min.y),
                Vec2::new(0.0, -1.0),
                Vec2::new(1.0, 0.0),
            )
        }
        PopoverSide::Top => {
            let cx = trigger
                .center()
                .x
                .clamp(popup.min.x + inset, popup.max.x - inset);
            (
                Pos2::new(cx, popup.max.y),
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 0.0),
            )
        }
        PopoverSide::Right => {
            let cy = trigger
                .center()
                .y
                .clamp(popup.min.y + inset, popup.max.y - inset);
            (
                Pos2::new(popup.min.x, cy),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, 1.0),
            )
        }
        PopoverSide::Left => {
            let cy = trigger
                .center()
                .y
                .clamp(popup.min.y + inset, popup.max.y - inset);
            (
                Pos2::new(popup.max.x, cy),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
            )
        }
    };

    let base_a = base_center + base_axis * half_base;
    let base_b = base_center - base_axis * half_base;
    let tip = base_center + perp * depth;

    // 1. Filled triangle in the popup fill colour, extending outside the
    //    popup edge toward the trigger.
    painter.add(Shape::convex_polygon(
        vec![base_a, tip, base_b],
        fill,
        Stroke::NONE,
    ));

    // 2. Cover the popup's border stroke where the arrow meets it: a
    //    short segment in the fill colour along the popup edge.
    painter.line_segment([base_a, base_b], Stroke::new(1.5, fill));

    // 3. Stroke the two outward edges of the triangle in the border colour.
    let stroke = Stroke::new(1.0, border);
    painter.line_segment([base_a, tip], stroke);
    painter.line_segment([base_b, tip], stroke);
}
