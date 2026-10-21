//! Profile avatars: circular tiles with initials, an optional presence dot,
//! and an [`AvatarGroup`] for stacked overlap with a `+N` overflow indicator.

use egui::{
    Color32, FontSelection, Response, Role, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo, WidgetText,
};

use crate::theme::{Theme, with_alpha};

/// Diameter preset for an [`Avatar`].
///
/// Maps to fixed point sizes so groups of avatars line up cleanly across
/// surfaces. Scale the font and presence-dot proportionally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarSize {
    /// 20 pt. For dense lists or inline chips.
    XSmall,
    /// 28 pt. Inline with body text.
    Small,
    /// 36 pt. The default.
    Medium,
    /// 48 pt. Card headers and identity rows.
    Large,
    /// 64 pt. Profile headers and feature screens.
    XLarge,
}

impl AvatarSize {
    fn diameter(self) -> f32 {
        match self {
            AvatarSize::XSmall => 20.0,
            AvatarSize::Small => 28.0,
            AvatarSize::Medium => 36.0,
            AvatarSize::Large => 48.0,
            AvatarSize::XLarge => 64.0,
        }
    }

    fn font_size(self) -> f32 {
        match self {
            AvatarSize::XSmall => 10.0,
            AvatarSize::Small => 11.5,
            AvatarSize::Medium => 13.0,
            AvatarSize::Large => 16.0,
            AvatarSize::XLarge => 22.0,
        }
    }

    fn dot_diameter(self) -> f32 {
        match self {
            AvatarSize::XSmall | AvatarSize::Small => 7.0,
            AvatarSize::Medium => 10.0,
            AvatarSize::Large => 12.0,
            AvatarSize::XLarge => 14.0,
        }
    }

    fn dot_border(self) -> f32 {
        match self {
            AvatarSize::XSmall | AvatarSize::Small => 1.5,
            _ => 2.0,
        }
    }
}

/// Background tone for an [`Avatar`]'s initials variant.
///
/// When the user passes `None` (the default), the tone is derived
/// deterministically from the initials text so the same person gets the
/// same colour everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarTone {
    /// Sky blue.
    Sky,
    /// Green.
    Green,
    /// Amber.
    Amber,
    /// Red.
    Red,
    /// Purple.
    Purple,
    /// Theme-neutral grey. Also used for the `+N` overflow tile.
    Neutral,
}

const AUTO_TONES: [AvatarTone; 5] = [
    AvatarTone::Sky,
    AvatarTone::Green,
    AvatarTone::Amber,
    AvatarTone::Red,
    AvatarTone::Purple,
];

impl AvatarTone {
    /// Pick a tone deterministically from a name or initials.
    pub fn from_text(s: &str) -> Self {
        // FNV-1a — stable across runs, unlike the hashing in `std::collections`.
        let mut h: u32 = 0x811c_9dc5;
        for b in s.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        AUTO_TONES[(h as usize) % AUTO_TONES.len()]
    }

    fn colours(self, theme: &Theme) -> (Color32, Color32) {
        let p = &theme.palette;
        match self {
            AvatarTone::Sky => (with_alpha(p.focus, 51), p.focus),
            AvatarTone::Green => (with_alpha(p.green, 46), p.success),
            AvatarTone::Amber => (with_alpha(p.warning, 51), p.warning),
            AvatarTone::Red => (with_alpha(p.danger, 51), p.danger),
            AvatarTone::Purple => (with_alpha(p.purple, 51), p.purple),
            AvatarTone::Neutral => (with_alpha(p.text_muted, 40), p.text_muted),
        }
    }
}

/// Presence-dot state painted at the avatar's bottom-right corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarPresence {
    /// Connected and active — green.
    Online,
    /// Do-not-disturb — red.
    Busy,
    /// Idle / away — amber.
    Away,
    /// Disconnected — neutral grey.
    Offline,
}

impl AvatarPresence {
    fn colour(self, theme: &Theme) -> Color32 {
        let p = &theme.palette;
        match self {
            AvatarPresence::Online => p.success,
            AvatarPresence::Busy => p.danger,
            AvatarPresence::Away => p.warning,
            AvatarPresence::Offline => p.text_faint,
        }
    }
}

/// A circular profile avatar with initials and an optional presence dot.
///
/// ```no_run
/// # use elegance::{Avatar, AvatarPresence, AvatarSize, AvatarTone};
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     Avatar::new("AL")
///         .size(AvatarSize::Large)
///         .tone(AvatarTone::Sky)
///         .presence(AvatarPresence::Online),
/// );
/// # });
/// ```
#[must_use = "Add the avatar with `ui.add(...)`."]
pub struct Avatar {
    initials: WidgetText,
    size: AvatarSize,
    tone: Option<AvatarTone>,
    presence: Option<AvatarPresence>,
    surface: Option<Color32>,
    ring: bool,
}

impl std::fmt::Debug for Avatar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Avatar")
            .field("initials", &self.initials.text())
            .field("size", &self.size)
            .field("tone", &self.tone)
            .field("presence", &self.presence)
            .field("ring", &self.ring)
            .finish()
    }
}

impl Avatar {
    /// Create an avatar that displays the given initials. One or two
    /// uppercase glyphs read best (`"AL"`, `"MR"`, `"??"`).
    pub fn new(initials: impl Into<WidgetText>) -> Self {
        Self {
            initials: initials.into(),
            size: AvatarSize::Medium,
            tone: None,
            presence: None,
            surface: None,
            ring: false,
        }
    }

    /// Pick a size preset. Default: [`AvatarSize::Medium`].
    #[inline]
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Pin the tone explicitly. When unset, the tone is derived from the
    /// initials so the same name always gets the same colour.
    #[inline]
    pub fn tone(mut self, tone: AvatarTone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Render a presence dot in the bottom-right corner. Default: none.
    #[inline]
    pub fn presence(mut self, presence: AvatarPresence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// Override the surface colour the avatar is painted against. Drives
    /// the inner border on the presence dot and the colour of the outer
    /// ring (when [`Avatar::ring`] is set). Defaults to the page
    /// background; pass `theme.palette.card` when placing an avatar inside
    /// a [`Card`](crate::Card) so the presence dot punches cleanly out of
    /// the card surface.
    #[inline]
    pub fn surface(mut self, color: Color32) -> Self {
        self.surface = Some(color);
        self
    }

    /// Paint a 2 pt outer ring in the surface colour around the disc. Used
    /// by [`AvatarGroup`] to separate overlapping members; rarely needed
    /// for solo avatars. Default: off.
    #[inline]
    pub fn ring(mut self, ring: bool) -> Self {
        self.ring = ring;
        self
    }

    fn paint(&self, ui: &mut Ui, rect: egui::Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let theme = Theme::current(ui.ctx());
        let p = &theme.palette;
        let surface = self.surface.unwrap_or(p.bg);
        let initials_text = self.initials.text();
        let tone = self
            .tone
            .unwrap_or_else(|| AvatarTone::from_text(initials_text));
        let (bg, fg) = tone.colours(&theme);

        let painter = ui.painter();
        let center = rect.center();
        let r = self.size.diameter() * 0.5;

        painter.circle_filled(center, r, bg);

        if !initials_text.is_empty() {
            let font_size = self.size.font_size();
            let galley = WidgetText::from(
                egui::RichText::new(initials_text)
                    .color(fg)
                    .size(font_size)
                    .strong(),
            )
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                FontSelection::FontId(egui::FontId::proportional(font_size)),
            );
            let pos = center - galley.size() * 0.5;
            ui.painter().galley(pos, galley, fg);
        }

        if self.ring {
            ui.painter()
                .circle_stroke(center, r + 1.0, Stroke::new(2.0, surface));
        }

        if let Some(presence) = self.presence {
            let dot_d = self.size.dot_diameter();
            let border_w = self.size.dot_border();
            let off = r + 1.0 - dot_d * 0.5;
            let dot_center = center + Vec2::splat(off);
            let outer_r = dot_d * 0.5 + border_w;
            ui.painter().circle_filled(dot_center, outer_r, surface);
            ui.painter()
                .circle_filled(dot_center, dot_d * 0.5, presence.colour(&theme));
        }
    }
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let diameter = self.size.diameter();
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
        self.paint(ui, rect);

        let label = self.initials.text();
        let owned = if label.is_empty() {
            "avatar".to_string()
        } else {
            label.to_string()
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &owned));
        response
    }
}

/// A row of overlapping avatars with an optional `+N` overflow tile.
///
/// All members share the group's size and surface so the punch-out ring
/// reads cleanly. Pass an explicit surface when placing the group on a
/// card or non-default background.
///
/// ```no_run
/// # use elegance::{Avatar, AvatarGroup, AvatarSize, AvatarTone};
/// # egui::__run_test_ui(|ui| {
/// ui.add(
///     AvatarGroup::new()
///         .size(AvatarSize::Medium)
///         .item(Avatar::new("AL").tone(AvatarTone::Sky))
///         .item(Avatar::new("MR").tone(AvatarTone::Green))
///         .item(Avatar::new("JK").tone(AvatarTone::Amber))
///         .overflow(7),
/// );
/// # });
/// ```
#[must_use = "Add the group with `ui.add(...)`."]
pub struct AvatarGroup {
    items: Vec<Avatar>,
    overflow: Option<usize>,
    overlap: f32,
    surface: Option<Color32>,
    size: AvatarSize,
}

impl std::fmt::Debug for AvatarGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AvatarGroup")
            .field("items", &self.items.len())
            .field("overflow", &self.overflow)
            .field("overlap", &self.overlap)
            .field("size", &self.size)
            .finish()
    }
}

impl Default for AvatarGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarGroup {
    /// Create an empty group at the default medium size.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            overflow: None,
            overlap: 10.0,
            surface: None,
            size: AvatarSize::Medium,
        }
    }

    /// Append an avatar. The group's size and surface override anything
    /// set on the passed avatar so members share a uniform diameter.
    #[inline]
    pub fn item(mut self, avatar: Avatar) -> Self {
        self.items.push(avatar);
        self
    }

    /// Show a trailing `+N` neutral tile. Counts beyond the on-screen
    /// items.
    #[inline]
    pub fn overflow(mut self, n: usize) -> Self {
        self.overflow = Some(n);
        self
    }

    /// Pixels of overlap between adjacent avatars. Default: 10 pt.
    #[inline]
    pub fn overlap(mut self, overlap: f32) -> Self {
        self.overlap = overlap;
        self
    }

    /// Pin the size for every member of the group. Default:
    /// [`AvatarSize::Medium`].
    #[inline]
    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Override the surface colour. Drives the punch-out ring around each
    /// member and the inner border on any presence dots. Defaults to the
    /// page background.
    #[inline]
    pub fn surface(mut self, color: Color32) -> Self {
        self.surface = Some(color);
        self
    }
}

impl Widget for AvatarGroup {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());
        let surface = self.surface.unwrap_or(theme.palette.bg);
        let diameter = self.size.diameter();
        let count = self.items.len() + usize::from(self.overflow.is_some());
        if count == 0 {
            let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            return response;
        }
        let total_w = diameter * count as f32 - self.overlap * (count.saturating_sub(1)) as f32;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(total_w, diameter), Sense::hover());

        let mut x = rect.left();
        for avatar in self.items {
            let avatar = avatar.size(self.size).surface(surface).ring(true);
            let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::splat(diameter));
            avatar.paint(ui, cell);
            x += diameter - self.overlap;
        }
        if let Some(n) = self.overflow {
            let label = format!("+{}", n);
            let avatar = Avatar::new(label)
                .size(self.size)
                .tone(AvatarTone::Neutral)
                .surface(surface)
                .ring(true);
            let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::splat(diameter));
            avatar.paint(ui, cell);
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "avatar group"));
        response
    }
}
