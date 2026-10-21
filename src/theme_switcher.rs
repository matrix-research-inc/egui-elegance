//! Theme picker — a one-line drop-in for switching between the four
//! built-in themes. Saves every app from re-implementing the
//! `Select::strings`-over-name-strings boilerplate.

use egui::{Id, Response, Ui, Widget};

use crate::Select;
use crate::theme::BuiltInTheme;

/// A drop-in picker for the four built-in elegance themes.
///
/// Renders a small [`Select`](crate::Select) of the four built-in themes
/// and — by default — installs the chosen theme into the context each
/// frame. Bind it to a [`BuiltInTheme`] held anywhere in your app state.
///
/// ```no_run
/// # use elegance::{BuiltInTheme, ThemeSwitcher};
/// # egui::__run_test_ui(|ui| {
/// let mut theme = BuiltInTheme::Slate;
/// ui.add(ThemeSwitcher::new(&mut theme));
/// # });
/// ```
///
/// # Installation
///
/// By default the widget calls [`Theme::install`](crate::Theme::install) on
/// the selected theme on every frame. If your app already installs a
/// theme elsewhere (for instance from a larger preference store), call
/// [`ThemeSwitcher::auto_install`]`(false)` to suppress that.
#[must_use = "Add with `ui.add(...)`."]
pub struct ThemeSwitcher<'a> {
    current: &'a mut BuiltInTheme,
    id_salt: Id,
    width: f32,
    auto_install: bool,
}

impl<'a> std::fmt::Debug for ThemeSwitcher<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeSwitcher")
            .field("current", &*self.current)
            .field("id_salt", &self.id_salt)
            .field("width", &self.width)
            .field("auto_install", &self.auto_install)
            .finish()
    }
}

impl<'a> ThemeSwitcher<'a> {
    /// Create a switcher bound to a mutable [`BuiltInTheme`] slot.
    pub fn new(current: &'a mut BuiltInTheme) -> Self {
        Self {
            current,
            id_salt: Id::unique("elegance_theme_switcher"),
            width: 110.0,
            auto_install: true,
        }
    }

    /// Override the id salt. Only needed if multiple switchers coexist
    /// in the same UI.
    pub fn id_salt(mut self, id_salt: impl crate::IdSalt) -> Self {
        self.id_salt = Id::unique(id_salt);
        self
    }

    /// Override the switcher width in points. Default: `110.0`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Whether to call [`Theme::install`](crate::Theme::install) on the
    /// selected theme every frame. Default: `true`.
    ///
    /// Set to `false` if the caller installs a theme elsewhere and just
    /// wants the picker UI.
    pub fn auto_install(mut self, auto_install: bool) -> Self {
        self.auto_install = auto_install;
        self
    }
}

impl Widget for ThemeSwitcher<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            current,
            id_salt,
            width,
            auto_install,
        } = self;

        let options = BuiltInTheme::all().into_iter().map(|t| (t, t.label()));
        // Reborrow so `current` remains usable after the Select is consumed,
        // which lets us install the (possibly just-updated) theme below.
        let response = ui.add(
            Select::new(id_salt, &mut *current)
                .options(options)
                .width(width),
        );

        if auto_install {
            current.theme().install(ui.ctx());
        }

        response
    }
}
