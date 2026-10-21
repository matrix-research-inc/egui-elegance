//! Collapsing section — a clickable trigger row with a rotating chevron
//! that hides or reveals a body of content.

use egui::{
    Color32, InnerResponse, Pos2, Response, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetText,
    pos2,
};

use crate::theme::Theme;

/// A collapsible content section.
///
/// ```no_run
/// # use elegance::CollapsingSection;
/// # egui::__run_test_ui(|ui| {
/// CollapsingSection::new("advanced", "Show advanced options").show(ui, |ui| {
///     ui.label("…hidden until expanded…");
/// });
/// # });
/// ```
#[must_use = "Call `.show(ui, |ui| ...)` to render."]
pub struct CollapsingSection<'a> {
    id_salt: egui::IdSalt,
    label: WidgetText,
    open: Option<&'a mut bool>,
    default_open: bool,
}

impl<'a> std::fmt::Debug for CollapsingSection<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollapsingSection")
            .field("id_salt", &self.id_salt)
            .field("label", &self.label.text())
            .field("open", &self.open.as_deref().copied())
            .field("default_open", &self.default_open)
            .finish()
    }
}

impl<'a> CollapsingSection<'a> {
    /// Create a collapsing section keyed by `id_salt` with the given header label.
    /// The section is closed by default.
    pub fn new(id_salt: impl crate::IdSalt, label: impl Into<WidgetText>) -> Self {
        Self {
            id_salt: egui::IdSalt::new(("elegance_collapsing", id_salt)),
            label: label.into(),
            open: None,
            default_open: false,
        }
    }

    /// Bind the open state to a `&mut bool` the caller owns. If omitted,
    /// the section remembers its state in egui's temp storage.
    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Starting state when no prior state exists. Default: closed.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Render the trigger row, and the body when open. Returns an
    /// [`InnerResponse`] whose `response` is the trigger row (useful for
    /// checking `clicked()`, `hovered()`, etc.) and whose `inner` is the
    /// body closure's return value, or `None` if the section is closed.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let theme = Theme::current(ui.ctx());
        let state_id = ui.scope_id().with_salt(self.id_salt);

        let mut is_open = match self.open.as_deref() {
            Some(flag) => *flag,
            None => ui
                .ctx()
                .data(|d| d.get_temp::<bool>(state_id).unwrap_or(self.default_open)),
        };

        let trigger = trigger_row(ui, self.label.text(), &theme, is_open);
        let label_text = self.label.text().to_string();
        trigger.widget_info(|| {
            WidgetInfo::selected(Role::DisclosureTriangle, true, is_open, &label_text)
        });
        if trigger.clicked() {
            is_open = !is_open;
        }

        match self.open {
            Some(flag) => *flag = is_open,
            None => {
                ui.ctx().data_mut(|d| d.insert_temp(state_id, is_open));
            }
        }

        let inner = if is_open { Some(add_body(ui)) } else { None };
        InnerResponse::new(inner, trigger)
    }
}

fn trigger_row(ui: &mut Ui, label: &str, theme: &Theme, open: bool) -> Response {
    let p = &theme.palette;
    let t = &theme.typography;

    const PAD_X: f32 = 4.0;
    const PAD_Y: f32 = 4.0;
    const CHEVRON: f32 = 12.0;
    const GAP: f32 = 8.0;

    let galley = crate::theme::placeholder_galley(ui, label, t.label, false, f32::INFINITY);

    let content_w = CHEVRON + GAP + galley.size().x;
    let desired = Vec2::new(content_w + PAD_X * 2.0, galley.size().y + PAD_Y * 2.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let label_color = if hovered { p.text } else { p.text_muted };
        let chevron_color = if hovered { p.focus } else { p.text_muted };

        let chev_center = pos2(rect.min.x + PAD_X + CHEVRON * 0.5, rect.center().y);
        draw_chevron(ui, chev_center, CHEVRON, chevron_color, open);

        let text_pos = pos2(
            rect.min.x + PAD_X + CHEVRON + GAP,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, label_color);
    }

    resp
}

fn draw_chevron(ui: &mut Ui, center: Pos2, size: f32, color: Color32, open: bool) {
    let half = size * 0.3;
    let points: Vec<Pos2> = if open {
        // ▾ — pointing down
        vec![
            pos2(center.x - half, center.y - half * 0.55),
            pos2(center.x + half, center.y - half * 0.55),
            pos2(center.x, center.y + half * 0.75),
        ]
    } else {
        // ▸ — pointing right
        vec![
            pos2(center.x - half * 0.55, center.y - half),
            pos2(center.x - half * 0.55, center.y + half),
            pos2(center.x + half * 0.75, center.y),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}
