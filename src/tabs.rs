//! Tab bar with a focus-accent underline on the active tab.

use egui::{Response, Role, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo, WidgetText, vec2};

use crate::theme::Theme;

/// A horizontal tab bar. The active tab is indicated by a focus-accent
/// underline and an inactive tab lights up on hover.
///
/// ```no_run
/// # use elegance::TabBar;
/// # egui::__run_test_ui(|ui| {
/// let mut selected = 0usize;
/// ui.add(TabBar::new(&mut selected, ["Overview", "Settings", "Activity"]));
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct TabBar<'a> {
    selected: &'a mut usize,
    tabs: Vec<WidgetText>,
}

impl<'a> std::fmt::Debug for TabBar<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabBar")
            .field("selected", &*self.selected)
            .field(
                "tabs",
                &self
                    .tabs
                    .iter()
                    .map(|t| t.text().to_string())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl<'a> TabBar<'a> {
    /// Build a tab bar. Accepts any iterator of items that convert into
    /// [`WidgetText`], so both static arrays and dynamic collections work:
    ///
    /// ```no_run
    /// # use elegance::TabBar;
    /// # egui::__run_test_ui(|ui| {
    /// let mut selected = 0usize;
    /// ui.add(TabBar::new(&mut selected, ["Overview", "Settings"]));
    /// let dynamic: Vec<String> = vec!["A".into(), "B".into()];
    /// ui.add(TabBar::new(&mut selected, dynamic));
    /// # });
    /// ```
    pub fn new<I, S>(selected: &'a mut usize, tabs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<WidgetText>,
    {
        Self {
            selected,
            tabs: tabs.into_iter().map(Into::into).collect(),
        }
    }
}

impl<'a> Widget for TabBar<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let egui::InnerResponse {
            mut response,
            inner: changed,
        } = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let mut changed = false;
            for (idx, name) in self.tabs.iter().enumerate() {
                let is_selected = idx == *self.selected;
                let galley = crate::theme::placeholder_galley(
                    ui,
                    name.text(),
                    t.button,
                    true,
                    f32::INFINITY,
                );

                let pad_x = theme.control_padding_x;
                // Intentionally taller than `theme.control_padding_y` — tabs
                // read as chunkier than standard controls.
                let pad_y = 10.0;
                let size = Vec2::new(galley.size().x + 2.0 * pad_x, galley.size().y + 2.0 * pad_y);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

                if resp.clicked() && *self.selected != idx {
                    *self.selected = idx;
                    changed = true;
                }

                let text_color = if is_selected {
                    p.focus
                } else if resp.hovered() {
                    p.text
                } else {
                    p.text_faint
                };
                let text_pos =
                    egui::pos2(rect.min.x + pad_x, rect.center().y - galley.size().y * 0.5);
                ui.painter().galley(text_pos, galley, text_color);

                let bottom = rect.bottom();
                if is_selected {
                    let a = egui::pos2(rect.min.x + 4.0, bottom - 1.0);
                    let b = egui::pos2(rect.max.x - 4.0, bottom - 1.0);
                    ui.painter().line_segment([a, b], Stroke::new(2.0, p.focus));
                }
            }
            changed
        });

        if ui.is_rect_visible(response.rect) {
            let bottom = response.rect.bottom();
            let a = egui::pos2(response.rect.min.x, bottom - 0.5);
            let b = egui::pos2(response.rect.right(), bottom - 0.5);
            ui.painter()
                .line_segment([a, b], Stroke::new(1.0, p.border));
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "tab bar"));
        if changed {
            response.mark_changed();
        }
        response
    }
}
