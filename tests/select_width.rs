//! Sizing of `Select`: its default width, a fixed `.width(...)`, and
//! `.size(...)`.

use egui::{Context, RawInput, Rect, Response, Ui, Vec2, pos2};
use elegance::{ButtonSize, SegmentedButton, Select, Theme};

/// Lay out `add` alone in a row at `pixels_per_point` and return the rect of
/// the response it gives.
fn laid_out_at(pixels_per_point: f32, mut add: impl FnMut(&mut Ui) -> Response) -> Rect {
    let ctx = Context::default();
    Theme::slate().install(&ctx);
    ctx.set_pixels_per_point(pixels_per_point);
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

fn laid_out(add: impl FnMut(&mut Ui) -> Response) -> Rect {
    laid_out_at(1.0, add)
}

/// The field width of one unlabeled string `Select` with `selected` picked,
/// after `style` has had its say.
fn field_width(
    selected: &str,
    options: &[&'static str],
    style: impl Fn(Select<'_, String>) -> Select<'_, String>,
) -> f32 {
    let mut value = selected.to_owned();
    laid_out(|ui| {
        ui.add(style(Select::strings(
            "unit",
            &mut value,
            options.iter().copied(),
        )))
    })
    .width()
}

/// Every option of `options` picked in turn must leave the field one width.
fn assert_width_is_stable(
    options: &[&'static str],
    style: impl Fn(Select<'_, String>) -> Select<'_, String>,
) {
    let widths: Vec<f32> = options
        .iter()
        .map(|s| field_width(s, options, &style))
        .collect();
    assert!(
        widths.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-3),
        "width changed with the selection: {widths:?}"
    );
}

/// The field must not resize as the selection changes, or it would shove its
/// neighbours around. egui's `ComboBox` widens the button to fit the selected
/// text, so this also catches a default that undercounts the widest label.
#[test]
fn default_width_is_stable_across_selections() {
    assert_width_is_stable(&["s", "ms", "Development"], |s| s);
}

/// The same holds for a sized select, whose padding is a button's and need
/// not be a whole number of points.
#[test]
fn sized_default_width_is_stable_across_selections() {
    for size in [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large] {
        assert_width_is_stable(&["s", "ms", "Development"], |s| s.size(size));
    }
}

/// Short option lists get a compact field, not a fixed-size one.
#[test]
fn default_width_tracks_the_widest_option() {
    let short = field_width("ms", &["us", "ms", "s"], |s| s);
    let long = field_width("ms", &["us", "ms", "s", "Development"], |s| s);
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
    let options = ["s", "A much longer option label"];
    for selected in options {
        let width = field_width(selected, &options, |s| s.width(80.0));
        assert!((width - 80.0).abs() < 1e-3, "{selected:?}: {width}");
    }
}

/// A field label wider than the fixed width does not widen the field either.
#[test]
fn fixed_width_holds_under_a_wide_label() {
    let options = ["s", "A much longer option label"];
    for selected in options {
        let mut value = selected.to_owned();
        // The response is the field's, not the label's.
        let field = laid_out(|ui| {
            ui.add(
                Select::strings("unit", &mut value, options)
                    .label("A field label far wider than the field")
                    .width(80.0),
            )
        });
        assert!(
            (field.width() - 80.0).abs() < 1e-3,
            "{selected:?}: {}",
            field.width()
        );
    }
}

/// A sized select is exactly as tall as a segmented button of the same size,
/// so it can sit flush in a strip of them, at any display scale.
#[test]
fn sized_select_matches_a_segmented_button() {
    for pixels_per_point in [1.0, 1.5, 2.0] {
        for size in [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large] {
            let mut value = String::from("ms");
            let select = laid_out_at(pixels_per_point, |ui| {
                ui.add(Select::strings("unit", &mut value, ["us", "ms"]).size(size))
            });
            let mut on = false;
            let button = laid_out_at(pixels_per_point, |ui| {
                ui.add(SegmentedButton::new(&mut on, "Run").size(size))
            });
            assert!(
                (select.height() - button.height()).abs() < 1e-3,
                "{size:?} at {pixels_per_point}x: select {} vs button {}",
                select.height(),
                button.height()
            );
        }
    }
}
