//! Sizing of `Select`: its default width, a fixed `.width(...)`, and
//! `.size(...)`.

use egui::{Context, RawInput, Rect, Response, Ui, Vec2, pos2};
use elegance::{ButtonSize, SegmentedButton, Select, Theme};

/// Lay out `add` alone and return the rect of the response it gives.
fn laid_out(mut add: impl FnMut(&mut Ui) -> Response) -> Rect {
    let ctx = Context::default();
    Theme::slate().install(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(600.0, 400.0))),
        ..Default::default()
    };
    let mut rect = Rect::NOTHING;
    // Two frames so the installed theme's spacing is in effect. Texture
    // deltas are discarded: nothing here uploads them, and `TexturesDelta`
    // asserts on drop that they were applied.
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| {
            ui.horizontal(|ui| rect = add(ui).rect);
        });
        output.textures_delta.clear();
    }
    rect
}

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

/// A fixed width holds even when the selected label is wider than it: the
/// label truncates rather than the field growing over its neighbours.
#[test]
fn fixed_width_holds_for_a_long_selection() {
    let width = |selected: &str| {
        let mut value = selected.to_owned();
        laid_out(|ui| {
            ui.add(
                Select::strings("unit", &mut value, ["s", "A much longer option label"])
                    .width(80.0),
            )
        })
        .width()
    };
    let (short, long) = (width("s"), width("A much longer option label"));
    assert!((short - 80.0).abs() < 0.5, "short selection: {short}");
    assert!(
        (long - 80.0).abs() < 0.5,
        "long selection grew the field: {long}"
    );
}

/// A sized select is as tall as a segmented button of the same size, so it
/// can sit in a strip of them.
#[test]
fn sized_select_matches_a_segmented_button() {
    for size in [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large] {
        let mut value = String::from("ms");
        let select =
            laid_out(|ui| ui.add(Select::strings("unit", &mut value, ["us", "ms"]).size(size)));
        let mut on = false;
        let button = laid_out(|ui| ui.add(SegmentedButton::new(&mut on, "Run").size(size)));
        assert!(
            (select.height() - button.height()).abs() < 0.5,
            "{size:?}: select {} vs button {}",
            select.height(),
            button.height()
        );
    }
}
