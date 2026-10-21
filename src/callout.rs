//! Callout — a full-width inline banner announcing persistent context.
//!
//! A [`Callout`] is a strip of chrome that sits inline in a layout to flag
//! important context to the reader: experimental features, unsaved changes,
//! failed builds, maintenance windows. Unlike [`Toast`](crate::Toast) it does
//! not auto-dismiss, and unlike [`FlashKind`](crate::FlashKind) it's a whole
//! surface rather than a pulse on another widget.
//!
//! Two visual treatments are available. The default is a `card`-colored
//! banner with a 3px accent stripe on the leading edge — quiet enough to
//! read as inline page chrome. Calling [`Callout::tinted`] swaps that for a
//! severity-tinted background with a matching tinted border and rounded
//! corners; this reads louder and works well when the banner needs to feel
//! like a discrete alert rather than part of the surrounding card.

use egui::{
    Align, Color32, CornerRadius, InnerResponse, Layout, Margin, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetText,
};

use crate::theme::Theme;

/// Semantic tones for a [`Callout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalloutTone {
    /// Informational — focus accent. For neutral announcements.
    Info,
    /// Success — green accent. For affirmative context ("Deploy complete").
    Success,
    /// Caution — amber accent. For conditions the user should notice.
    Warning,
    /// Error or destructive — red accent. For failures or dangerous state.
    Danger,
    /// Grey — for announcements that have no urgency at all.
    Neutral,
}

impl CalloutTone {
    fn stripe(self, theme: &Theme) -> Color32 {
        let p = &theme.palette;
        match self {
            Self::Info => p.focus,
            Self::Success => p.green,
            Self::Warning => p.amber,
            Self::Danger => p.red,
            Self::Neutral => p.text_muted,
        }
    }

    fn icon_color(self, theme: &Theme) -> Color32 {
        let p = &theme.palette;
        match self {
            Self::Info => p.focus,
            Self::Success => p.success,
            Self::Warning => p.warning,
            Self::Danger => p.danger,
            Self::Neutral => p.text_muted,
        }
    }

    fn default_icon(self) -> char {
        use crate::glyphs;
        match self {
            Self::Info => glyphs::INFO,
            Self::Success => glyphs::CIRCLE_CHECK,
            Self::Warning => glyphs::TRIANGLE_ALERT,
            Self::Danger => glyphs::CIRCLE_X,
            // No Lucide mark for a tone with no urgency; a plain dot reads
            // as "just an announcement".
            Self::Neutral => '•',
        }
    }
}

/// A full-width inline banner in the elegance style.
///
/// ```no_run
/// # use elegance::{Callout, CalloutTone};
/// # egui::__run_test_ui(|ui| {
/// Callout::new(CalloutTone::Warning)
///     .title("Unsaved changes.")
///     .body("You have 3 edits that haven't been written to disk.")
///     .show(ui, |_| {});
/// # });
/// ```
#[must_use = "Call `.show(ui, ...)` to render the callout."]
pub struct Callout<'a> {
    tone: CalloutTone,
    tinted: bool,
    multiline: bool,
    title: Option<WidgetText>,
    body: Option<WidgetText>,
    icon: Option<WidgetText>,
    open: Option<&'a mut bool>,
}

impl std::fmt::Debug for Callout<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callout")
            .field("tone", &self.tone)
            .field("tinted", &self.tinted)
            .field("multiline", &self.multiline)
            .field("title", &self.title.as_ref().map(|t| t.text()))
            .field("body", &self.body.as_ref().map(|b| b.text()))
            .field("icon", &self.icon.as_ref().map(|i| i.text()))
            .field("dismissable", &self.open.is_some())
            .finish()
    }
}

impl<'a> Callout<'a> {
    /// Create a new callout with the given tone.
    pub fn new(tone: CalloutTone) -> Self {
        Self {
            tone,
            tinted: false,
            multiline: false,
            title: None,
            body: None,
            icon: None,
            open: None,
        }
    }

    /// Set the bolded title text rendered inline at the left of the banner.
    #[inline]
    pub fn title(mut self, text: impl Into<WidgetText>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Set the muted body text, rendered to the right of the title.
    #[inline]
    pub fn body(mut self, text: impl Into<WidgetText>) -> Self {
        self.body = Some(text.into());
        self
    }

    /// Override the icon glyph. Defaults to a tone-dependent symbol.
    #[inline]
    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Swap the default stripe-on-card treatment for a severity-tinted
    /// background with a matching tinted border and rounded corners.
    ///
    /// The fill is the tone color at low alpha and the border is the same
    /// color at moderate alpha; the leading accent stripe is dropped. Use
    /// this when the banner should read as a discrete alert rather than as
    /// inline page chrome.
    #[inline]
    pub fn tinted(mut self) -> Self {
        self.tinted = true;
        self
    }

    /// Render a trailing × button that sets `*open = false` when clicked.
    ///
    /// The caller is responsible for gating the `.show(...)` call on the
    /// same `bool` so the banner disappears after dismissal.
    #[inline]
    pub fn dismissable(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Switch from the default inline single-row layout to a stacked layout
    /// where the body wraps to multiple lines below the title.
    ///
    /// Use this when the body is longer than ~one short sentence. The
    /// default layout puts title and body on the same row and truncates the
    /// body to fit, which keeps short banners ("Unsaved changes. You have 3
    /// edits...") tidy alongside right-aligned action buttons but cuts off
    /// any longer prose. In multiline mode the row reads:
    ///
    /// ```text
    /// ⓘ  Title.                                       [Actions] [×]
    ///    Body wraps to as many lines as it needs to fit
    ///    the available width.
    /// ```
    ///
    /// Combine freely with [`Callout::tinted`], [`Callout::dismissable`],
    /// and the `add_actions` closure passed to [`Callout::show`].
    #[inline]
    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    /// Render the callout and return the closure's result.
    ///
    /// `add_actions` is invoked with a **right-to-left** layout so buttons
    /// you add slot into the action area between the body text and the
    /// dismiss button. Inside the closure the first widget added appears
    /// furthest right, so **add your primary action first**:
    ///
    /// ```no_run
    /// # use elegance::{Accent, Button, Callout, CalloutTone};
    /// # egui::__run_test_ui(|ui| {
    /// Callout::new(CalloutTone::Warning)
    ///     .title("Unsaved changes.")
    ///     .show(ui, |ui| {
    ///         ui.add(Button::new("Save now").accent(Accent::Amber)); // rightmost
    ///         ui.add(Button::new("Discard").outline());              // to its left
    ///     });
    /// # });
    /// ```
    ///
    /// Pass `|_| {}` when no actions are needed.
    pub fn show<R>(self, ui: &mut Ui, add_actions: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        const STRIPE_WIDTH: f32 = 3.0;
        const TINT_RADIUS: u8 = 8;
        // ~10% / ~28% — matches the bordered-tinted variant in
        // mockups/banner/banner.html.
        const TINT_BG_ALPHA: u8 = 26;
        const TINT_BORDER_ALPHA: u8 = 71;

        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let body_size = theme.typography.body;
        let stripe = self.tone.stripe(&theme);
        let icon_color = self.tone.icon_color(&theme);
        let default_icon = self.tone.default_icon();

        let a11y_label = self
            .title
            .as_ref()
            .or(self.body.as_ref())
            .map(|w| w.text().to_string())
            .unwrap_or_else(|| "callout".to_string());

        let Self {
            tone: _,
            tinted,
            multiline,
            title,
            body,
            icon,
            open,
        } = self;

        let frame = if tinted {
            let bg =
                Color32::from_rgba_unmultiplied(stripe.r(), stripe.g(), stripe.b(), TINT_BG_ALPHA);
            let border = Color32::from_rgba_unmultiplied(
                stripe.r(),
                stripe.g(),
                stripe.b(),
                TINT_BORDER_ALPHA,
            );
            egui::Frame::new()
                .fill(bg)
                .stroke(Stroke::new(1.0, border))
                .corner_radius(CornerRadius::same(TINT_RADIUS))
                .inner_margin(Margin {
                    left: 14,
                    right: 14,
                    top: 10,
                    bottom: 10,
                })
        } else {
            // Left inner margin accounts for the 3px stripe (24 pt from
            // stripe to content, matching the HTML mockups).
            egui::Frame::new().fill(p.card).inner_margin(Margin {
                left: (STRIPE_WIDTH as i8) + 18,
                right: 16,
                top: 10,
                bottom: 10,
            })
        };

        let icon_str = icon
            .as_ref()
            .map(|w| w.text().to_string())
            .unwrap_or_else(|| default_icon.to_string());

        let frame_response: InnerResponse<R> = frame.show(ui, |ui| {
            if multiline {
                // Stacked layout: icon on the left, then a vertical column
                // with [title + actions row] on top and [wrapped body] below.
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

                    // Icon (top-aligned with the title row).
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&icon_str)
                                .color(icon_color)
                                .size(body_size + 1.0),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );

                    ui.vertical(|ui| {
                        // Claim the rest of the row so the body wraps to it.
                        ui.set_width(ui.available_width());

                        let actions_inner = ui
                            .horizontal(|ui| {
                                if let Some(title) = title {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(title.text())
                                                .color(p.text)
                                                .size(body_size)
                                                .strong(),
                                        )
                                        .wrap(),
                                    );
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if let Some(open) = open
                                        && dismiss_button(ui, &theme).clicked()
                                    {
                                        *open = false;
                                    }
                                    add_actions(ui)
                                })
                                .inner
                            })
                            .inner;

                        if let Some(body) = body {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(body.text())
                                        .color(p.text_muted)
                                        .size(body_size),
                                )
                                .wrap(),
                            );
                        }

                        actions_inner
                    })
                    .inner
                })
                .inner
            } else {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

                    // Icon.
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&icon_str)
                                .color(icon_color)
                                .size(body_size + 1.0),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );

                    // Title (strong).
                    if let Some(title) = title {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(title.text())
                                    .color(p.text)
                                    .size(body_size)
                                    .strong(),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    }

                    // Body (muted).
                    if let Some(body) = body {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(body.text())
                                    .color(p.text_muted)
                                    .size(body_size),
                            )
                            .wrap_mode(egui::TextWrapMode::Truncate),
                        );
                    }

                    // Right-aligned action slot and optional dismiss button.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(open) = open
                            && dismiss_button(ui, &theme).clicked()
                        {
                            *open = false;
                        }
                        add_actions(ui)
                    })
                    .inner
                })
                .inner
            }
        });

        // Stripe + bottom hairline are only for the default treatment;
        // tinted draws its own border via the frame stroke.
        if !tinted {
            let rect = frame_response.response.rect;
            let painter = ui.painter();
            painter.rect(
                Rect::from_min_max(
                    rect.left_top(),
                    egui::pos2(rect.left() + STRIPE_WIDTH, rect.bottom()),
                ),
                CornerRadius::ZERO,
                stripe,
                Stroke::NONE,
                StrokeKind::Inside,
            );
            painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.border));
        }

        frame_response
            .response
            .widget_info(|| WidgetInfo::labeled(Role::Unknown, true, &a11y_label));

        frame_response
    }
}

/// Compact × dismiss button. Renders as two diagonal strokes so it doesn't
/// depend on the presence of a specific "×" glyph in the active font.
fn dismiss_button(ui: &mut Ui, theme: &Theme) -> Response {
    let size = Vec2::splat(theme.typography.body + 8.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if response.hovered() || response.has_focus() {
            theme.palette.text
        } else {
            theme.palette.text_faint
        };
        let painter = ui.painter();
        let c = rect.center();
        let r = 4.5;
        let stroke = Stroke::new(1.5, color);
        painter.line_segment([c + Vec2::new(-r, -r), c + Vec2::new(r, r)], stroke);
        painter.line_segment([c + Vec2::new(r, -r), c + Vec2::new(-r, r)], stroke);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Dismiss"));
    response
}
