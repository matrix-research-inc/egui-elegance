//! Styled select (combo-box) widget.
//!
//! Wraps [`egui::ComboBox`] and paints it with the elegance palette: slate
//! input background, 1-px border, focus ring, and a matching chevron.

use std::borrow::Cow;

use egui::{
    Color32, ComboBox, CornerRadius, Pos2, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo,
    WidgetText, WidgetType,
};

use crate::theme::{Theme, with_alpha};

/// Gap between the field label and the unsaved dot.
const LABEL_DOT_GAP: f32 = 6.0;
/// Unsaved-dot diameter as a fraction of the label font size, so the mark
/// tracks the label if a caller rescales the typography.
const DOT_TO_LABEL: f32 = 0.46;

/// A styled drop-down select.
///
/// Bind the selection to any `PartialEq + Clone` type — an enum, an index,
/// or a `String` — and supply a list of `(value, label)` pairs. Labels
/// accept `&'static str`, `String`, or any `Cow<'a, str>`, so static option
/// lists don't allocate.
///
/// ```no_run
/// # use elegance::Select;
/// # egui::__run_test_ui(|ui| {
/// #[derive(Clone, PartialEq)]
/// enum Unit { Us, Ms, S }
///
/// let mut unit = Unit::Ms;
/// ui.add(Select::new("unit", &mut unit).options([
///     (Unit::Us, "μs"),
///     (Unit::Ms, "ms"),
///     (Unit::S,  "s"),
/// ]));
/// # });
/// ```
///
/// For string-valued selects where each option is both the value and the
/// label, use [`Select::strings`]:
///
/// ```no_run
/// # use elegance::Select;
/// # egui::__run_test_ui(|ui| {
/// let mut unit = String::from("ms");
/// ui.add(Select::strings("unit", &mut unit, ["us", "ms", "s"]));
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct Select<'a, T: PartialEq + Clone> {
    id_salt: egui::Id,
    value: &'a mut T,
    label: Option<WidgetText>,
    options: Vec<(T, Cow<'a, str>)>,
    width: Option<f32>,
    enabled: bool,
    /// Whether the selection has moved off the last committed value. Resolved
    /// by [`Select::saved`] at build time; `false` when it was never called.
    staged: bool,
}

impl<'a, T: PartialEq + Clone> std::fmt::Debug for Select<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels: Vec<&str> = self.options.iter().map(|(_, l)| l.as_ref()).collect();
        f.debug_struct("Select")
            .field("id_salt", &self.id_salt)
            .field("option_labels", &labels)
            .field("width", &self.width)
            .field("enabled", &self.enabled)
            .field("staged", &self.staged)
            .finish()
    }
}

impl<'a, T: PartialEq + Clone> Select<'a, T> {
    /// Create a select keyed by `id_salt` and bound to `value`.
    /// Add selectable options via [`Select::options`].
    pub fn new(id_salt: impl crate::IdSalt, value: &'a mut T) -> Self {
        Self {
            id_salt: egui::Id::new(id_salt),
            value,
            label: None,
            options: Vec::new(),
            width: None,
            enabled: true,
            staged: false,
        }
    }

    /// Show a label above the select.
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Supply the last committed value, so a *staged* select — one whose
    /// selection has moved but not yet been saved — marks itself with a
    /// focus-accent dot after the label. It is the same dot
    /// [`BrowserTab::dirty`](crate::BrowserTab::dirty) uses for the same
    /// meaning, sized to the field label rather than the tab strip, so a
    /// staged picker reads as unsaved beside a
    /// [`TextInput::dirty`](crate::TextInput::dirty) field in the same form.
    ///
    /// ```no_run
    /// # use elegance::Select;
    /// # egui::__run_test_ui(|ui| {
    /// # let mut region = String::from("us-east-1");
    /// let saved_region = String::from("eu-west-1");
    /// ui.add(
    ///     Select::strings("region", &mut region, ["eu-west-1", "us-east-1"])
    ///         .label("Region")
    ///         .saved(&saved_region),
    /// );
    /// # });
    /// ```
    ///
    /// The select takes the committed value rather than a `dirty: bool`
    /// because it already requires `T: PartialEq` and can make the
    /// comparison itself — leaving it to the caller invites the flag to
    /// drift out of sync with the value, and forces the comparison to be
    /// hoisted above the call anyway, since the select is holding
    /// `&mut value` by then.
    ///
    /// The mark rides the label, so this needs [`label`](Self::label): with
    /// no label there is nowhere for it to sit and nothing is painted.
    pub fn saved(mut self, saved: &T) -> Self {
        // Compare now rather than storing the reference. The select holds
        // `&mut value` for its whole lifetime, so nothing can change the
        // selection between here and `ui.add`, which makes the eager
        // comparison equivalent — and it keeps the committed value free to
        // be a shorter-lived temporary than the bound one.
        self.staged = saved != &*self.value;
        self
    }

    /// Set the selectable options as `(value, label)` pairs. Labels are
    /// carried as `Cow<'a, str>`, so `&'static str` labels never allocate.
    pub fn options<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = (T, S)>,
        S: Into<Cow<'a, str>>,
    {
        self.options = options.into_iter().map(|(v, l)| (v, l.into())).collect();
        self
    }

    /// Override the select width in points. By default the select is as
    /// wide as its widest option label plus padding and the chevron, so the
    /// width holds steady as the selection changes.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Enable or disable the select. A disabled select is dimmed and cannot
    /// be opened, mirroring [`Button::enabled`](crate::Button::enabled) — so a
    /// select can be greyed out inline without wrapping it in
    /// [`Ui::add_enabled_ui`](egui::Ui::add_enabled_ui). Default: `true`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a> Select<'a, String> {
    /// Convenience constructor for string-valued selects. Each item is used
    /// as both the value and the displayed label.
    ///
    /// ```no_run
    /// # use elegance::Select;
    /// # egui::__run_test_ui(|ui| {
    /// let mut unit = String::from("ms");
    /// ui.add(Select::strings("unit", &mut unit, ["us", "ms", "s"]));
    /// # });
    /// ```
    pub fn strings<I, S>(id_salt: impl crate::IdSalt, value: &'a mut String, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'a, str>>,
    {
        let options: Vec<(String, Cow<'a, str>)> = options
            .into_iter()
            .map(|s| {
                let label: Cow<'a, str> = s.into();
                let value = label.as_ref().to_owned();
                (value, label)
            })
            .collect();
        Self {
            id_salt: egui::Id::new(id_salt),
            value,
            label: None,
            options,
            width: None,
            enabled: true,
            staged: false,
        }
    }
}

impl<'a, T: PartialEq + Clone> Widget for Select<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let t = &theme.typography;

        let mut changed = false;
        let enabled = self.enabled;
        let staged = self.staged;

        let mut response = ui
            .vertical(|ui| {
                // Fade and lock the whole control (label + field). `disable`
                // multiplies the painter opacity and denies interaction, so the
                // combo can't be opened and our explicit override colours dim
                // along with everything else.
                if !enabled {
                    ui.disable();
                }
                if let Some(label) = &self.label {
                    // The unsaved mark rides the label, not the field. A bar
                    // inside the frame (the TextInput treatment) would have to
                    // negotiate with the chevron and the selected text; a dot
                    // beside the field's *name* leaves the select's own
                    // geometry untouched and stays put across hover, focus,
                    // and open states.
                    //
                    // Lay the row out by hand rather than reaching for
                    // `ui.horizontal`, which floors a row at `interact_size.y`
                    // and would drop the field below its unstaged neighbours
                    // in the same form row. Sized to the galley, the row is
                    // exactly as tall as a plain label whether or not the mark
                    // is showing, so a select that becomes staged mid-form
                    // never nudges its neighbours.
                    let galley = crate::theme::placeholder_galley(
                        ui,
                        label.text(),
                        t.label,
                        false,
                        f32::INFINITY,
                    );
                    let dot = t.label * DOT_TO_LABEL;
                    let reserved = if staged { LABEL_DOT_GAP + dot } else { 0.0 };
                    let size = Vec2::new(galley.size().x + reserved, galley.size().y);
                    let (rect, label_response) = ui.allocate_exact_size(size, Sense::hover());
                    // Painting the galley by hand skips the accesskit node
                    // `egui::Label` would have emitted, which would drop the
                    // field label out of the tree — announce it explicitly so
                    // the tree does not change shape with the staged state.
                    label_response.widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Label, enabled, label.text())
                    });
                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();
                        painter.galley(rect.min, galley, p.text_muted);
                        if staged {
                            painter.circle_filled(
                                Pos2::new(rect.max.x - dot * 0.5, rect.center().y),
                                dot * 0.5,
                                p.focus,
                            );
                        }
                    }
                    ui.add_space(2.0);
                }

                let width = self.width.unwrap_or_else(|| {
                    // Size to the widest option rather than the selected one,
                    // so picking a different value never resizes the field and
                    // shifts its neighbours. Mirrors the layout in egui's
                    // `ComboBox`: text, icon spacing, icon, then the button
                    // padding on both sides.
                    let widest = self
                        .options
                        .iter()
                        .map(|(_, l)| {
                            crate::theme::placeholder_galley(ui, l, t.body, false, f32::INFINITY)
                                .size()
                                .x
                        })
                        .fold(0.0, f32::max);
                    let sp = ui.spacing();
                    widest + sp.icon_spacing + sp.icon_width + 2.0 * sp.button_padding.x
                });
                let chevron_color = p.text_muted;

                // Resolve the displayed label for the current value. Owned so
                // it doesn't conflict with the mutable access to `self.value`
                // in the inner closure.
                let selected_label: String = self
                    .options
                    .iter()
                    .find(|(v, _)| v == &*self.value)
                    .map(|(_, l)| l.as_ref().to_owned())
                    .unwrap_or_default();
                let field_label = self.label.as_ref().map(|l| l.text().to_string());

                let response = crate::theme::with_themed_visuals(ui, |ui| {
                    let v = ui.visuals_mut();
                    crate::theme::themed_input_visuals(v, &theme, p.input_bg);
                    for w in [
                        &mut v.widgets.inactive,
                        &mut v.widgets.hovered,
                        &mut v.widgets.active,
                        &mut v.widgets.open,
                    ] {
                        w.fg_stroke = Stroke::new(1.0, p.text);
                    }
                    v.override_text_color = Some(p.text);

                    ComboBox::from_id_salt(self.id_salt)
                        .width(width)
                        .selected_text(
                            egui::RichText::new(&selected_label)
                                .color(p.text)
                                .size(t.body),
                        )
                        .icon(move |ui, rect, _visuals, is_popup_open| {
                            paint_chevron(ui, rect, chevron_color, is_popup_open);
                        })
                        .show_ui(ui, |ui| {
                            ui.set_min_width(width);
                            // Tight stacking. `select_option` handles its own padding.
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for (opt_value, opt_label) in self.options.iter() {
                                let selected = opt_value == &*self.value;
                                if select_option(ui, opt_label.as_ref(), selected, &theme).clicked()
                                    && opt_value != &*self.value
                                {
                                    *self.value = opt_value.clone();
                                    changed = true;
                                }
                            }
                        })
                        .response
                });

                if let Some(field_label) = field_label {
                    let selected_label = selected_label.clone();
                    response.widget_info(|| {
                        let mut info =
                            WidgetInfo::labeled(WidgetType::ComboBox, enabled, &field_label);
                        info.current_text_value = Some(selected_label.clone());
                        info
                    });
                }

                response
            })
            .inner;

        if changed {
            response.mark_changed();
        }
        response
    }
}

/// Paint a thin, centered chevron inside `rect`. Points down when the popup is
/// closed (hint to open) and flips up when the popup is open (hint to close).
fn paint_chevron(ui: &egui::Ui, rect: egui::Rect, color: Color32, is_popup_open: bool) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.4, color);

    let half_w = (rect.width() * 0.35).min(5.0);
    let half_h = (rect.height() * 0.18).min(3.0);
    let c = rect.center();

    let (left, right, tip) = if is_popup_open {
        (
            egui::pos2(c.x - half_w, c.y + half_h * 0.5),
            egui::pos2(c.x + half_w, c.y + half_h * 0.5),
            egui::pos2(c.x, c.y - half_h * 1.5),
        )
    } else {
        (
            egui::pos2(c.x - half_w, c.y - half_h * 0.5),
            egui::pos2(c.x + half_w, c.y - half_h * 0.5),
            egui::pos2(c.x, c.y + half_h * 1.5),
        )
    };

    painter.line_segment([left, tip], stroke);
    painter.line_segment([tip, right], stroke);
}

/// Render a single option row inside the Select popup. Keeps the text anchored
/// at a fixed offset across hover/selected/inactive states. egui's own
/// `ui.selectable_label` goes through `Button::selectable`, whose frame's
/// `expansion` changes between states, which shifts the text by ~1px on hover.
fn select_option(ui: &mut Ui, label: &str, selected: bool, theme: &Theme) -> Response {
    let p = &theme.palette;
    let t = &theme.typography;

    let pad_x = 10.0;
    let pad_y = 6.0;

    let galley = crate::theme::placeholder_galley(ui, label, t.body, false, f32::INFINITY);
    let content_w = galley.size().x;
    let desired = Vec2::new(
        ui.available_width().max(content_w + pad_x * 2.0),
        galley.size().y.max(t.body) + pad_y * 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let bg = if response.hovered() {
            with_alpha(p.focus, 60)
        } else if selected {
            with_alpha(p.focus, 40)
        } else {
            Color32::TRANSPARENT
        };
        if bg.a() > 0 {
            let radius = CornerRadius::same((theme.control_radius as u8).saturating_sub(2));
            ui.painter().rect_filled(rect, radius, bg);
        }
        let label_pos = Pos2::new(rect.min.x + pad_x, rect.center().y - galley.size().y * 0.5);
        ui.painter().galley(label_pos, galley, p.text);
    }
    response
}
