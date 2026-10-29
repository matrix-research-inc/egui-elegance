//! `Theme::install` must theme both of egui's styles, since egui swaps
//! between them when the OS switches dark/light mode.

use elegance::Theme;

fn assert_both_styles_match(ctx: &egui::Context, theme: &Theme) {
    for slot in [egui::Theme::Dark, egui::Theme::Light] {
        let visuals = &ctx.style_of(slot).visuals;
        assert_eq!(visuals.panel_fill, theme.palette.bg, "{slot:?} style");
        assert_eq!(visuals.dark_mode, theme.palette.is_dark, "{slot:?} style");
    }
}

#[test]
fn install_themes_both_styles() {
    let ctx = egui::Context::default();
    let slate = Theme::slate();
    slate.clone().install(&ctx);
    assert_both_styles_match(&ctx, &slate);
}

#[test]
fn install_survives_an_os_theme_switch() {
    let ctx = egui::Context::default();
    let slate = Theme::slate();
    slate.clone().install(&ctx);
    ctx.set_theme(egui::Theme::Light);
    // Reinstalling the same theme is skipped, so the light style must
    // already carry it.
    slate.clone().install(&ctx);
    assert_eq!(ctx.global_style().visuals.panel_fill, slate.palette.bg);
}
