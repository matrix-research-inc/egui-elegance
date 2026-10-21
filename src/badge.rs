//! Small rounded badges — used for status labels like "OK", "Connected",
//! "Pending", "Error".

use egui::{
    Color32, CornerRadius, FontSelection, Response, Role, Sense, Stroke, Ui, Vec2, Widget,
    WidgetInfo, WidgetText,
};

use crate::theme::{Palette, Theme, with_alpha};

/// Colour tones for a [`Badge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeTone {
    /// Success — green.
    Ok,
    /// Caution — amber.
    Warning,
    /// Error — red.
    Danger,
    /// Informational — focus accent.
    Info,
    /// Neutral grey, for status that isn't success/warning/error.
    Neutral,
}

impl BadgeTone {
    /// The tone's foreground colour — the badge label, and the tint used by
    /// other widgets that mark status with this vocabulary (for example
    /// [`PairItem::icon_tone`](crate::PairItem::icon_tone)).
    ///
    /// These are the colours to paint a glyph or label *on* a surface with;
    /// [`Palette::accent_fill`](crate::Palette::accent_fill) is for painting a
    /// filled shape with content on top. The two coincide in the light
    /// palettes, but the dark ones lighten `success` / `warning` / `danger`
    /// away from their button fills so a thin stroke still reads against a
    /// card.
    ///
    /// ```
    /// # use elegance::{BadgeTone, Theme};
    /// let palette = Theme::slate().palette;
    /// assert_eq!(BadgeTone::Ok.foreground(&palette), palette.success);
    /// // `Neutral` is the muted default an untinted mark already paints in.
    /// assert_eq!(BadgeTone::Neutral.foreground(&palette), palette.text_muted);
    /// ```
    pub fn foreground(self, palette: &Palette) -> Color32 {
        match self {
            BadgeTone::Ok => palette.success,
            BadgeTone::Warning => palette.warning,
            BadgeTone::Danger => palette.danger,
            BadgeTone::Info => palette.focus,
            BadgeTone::Neutral => palette.text_muted,
        }
    }

    fn colours(self, theme: &Theme) -> (Color32, Color32) {
        let p = &theme.palette;
        let bg = match self {
            BadgeTone::Ok => with_alpha(p.green, 64),
            BadgeTone::Warning => with_alpha(p.amber, 64),
            BadgeTone::Danger => with_alpha(p.red, 64),
            BadgeTone::Info => with_alpha(p.focus, 64),
            BadgeTone::Neutral => with_alpha(p.text_muted, 40),
        };
        (bg, self.foreground(p))
    }
}

/// A compact rounded status badge.
///
/// Badge text is upper-cased by default, which fits status labels like
/// "OK" or "Warning". For identifiers (branch names, file paths, user
/// handles) where casing carries meaning, call [`Badge::preserve_case`]
/// to render the text exactly as supplied.
///
/// ```no_run
/// # use elegance::{Badge, BadgeTone};
/// # egui::__run_test_ui(|ui| {
/// ui.add(Badge::new("OK", BadgeTone::Ok));
/// ui.add(Badge::new("feature/login", BadgeTone::Info).preserve_case());
/// # });
/// ```
#[must_use = "Add with `ui.add(...)`."]
pub struct Badge {
    text: WidgetText,
    tone: BadgeTone,
    preserve_case: bool,
}

impl std::fmt::Debug for Badge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Badge")
            .field("text", &self.text.text())
            .field("tone", &self.tone)
            .field("preserve_case", &self.preserve_case)
            .finish()
    }
}

impl Badge {
    /// Create a badge with the given text and tone.
    pub fn new(text: impl Into<WidgetText>, tone: BadgeTone) -> Self {
        Self {
            text: text.into(),
            tone,
            preserve_case: false,
        }
    }

    /// Render the label as supplied instead of upper-casing it.
    ///
    /// Use this when the text is an identifier rather than a status
    /// label: branch names, file paths, user handles, version strings
    /// where the original casing is meaningful.
    pub fn preserve_case(mut self) -> Self {
        self.preserve_case = true;
        self
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let t = &theme.typography;
        let (bg, fg) = self.tone.colours(&theme);

        let font = egui::FontId::proportional(t.small);
        let label = if self.preserve_case {
            self.text.text().to_string()
        } else {
            self.text.text().to_uppercase()
        };
        let galley =
            egui::WidgetText::from(egui::RichText::new(label).color(fg).size(t.small).strong())
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    FontSelection::FontId(font),
                );

        let pad = Vec2::new(9.0, 3.0);
        let desired = galley.size() + pad * 2.0;
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.painter().rect(
                rect,
                CornerRadius::same(99),
                bg,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
            let text_pos = egui::pos2(rect.min.x + pad.x, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, fg);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, self.text.text()));
        response
    }
}
