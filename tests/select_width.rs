//! Default sizing of `Select` when no explicit `.width(...)` is given.

use egui::{Context, RawInput, Rect, Vec2, pos2};
use elegance::{Select, Theme};

/// Lay out one unlabeled string `Select` and return its field width.
fn field_width(selected: &str, options: &[&'static str]) -> f32 {
    let ctx = Context::default();
    Theme::slate().install(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(600.0, 400.0))),
        ..Default::default()
    };
    let mut value = selected.to_owned();
    let mut width = 0.0;
    // Two frames so the installed theme's spacing is in effect. Texture
    // deltas are discarded: nothing here uploads them, and `TexturesDelta`
    // asserts on drop that they were applied.
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| {
            width = ui
                .add(Select::strings("unit", &mut value, options.iter().copied()))
                .rect
                .width();
        });
        output.textures_delta.clear();
    }
    width
}

/// The field must not resize as the selection changes, or it would shove its
/// neighbours around. egui's `ComboBox` widens the button to fit the selected
/// text, so this also catches a default that undercounts the widest label.
#[test]
fn default_width_is_stable_across_selections() {
    let options = ["s", "ms", "Development"];
    let widths: Vec<f32> = options.iter().map(|s| field_width(s, &options)).collect();
    assert!(
        widths.windows(2).all(|w| (w[0] - w[1]).abs() < 0.5),
        "width changed with the selection: {widths:?}"
    );
}

/// Short option lists get a compact field, not a fixed-size one.
#[test]
fn default_width_tracks_the_widest_option() {
    let short = field_width("ms", &["us", "ms", "s"]);
    let long = field_width("ms", &["us", "ms", "s", "Development"]);
    assert!(
        short < long,
        "a longer option should widen the field: short={short}, long={long}"
    );
    assert!(short < 100.0, "short options should stay compact: {short}");
}
