//! Text-layout regressions in widgets that paint their own text.

use egui::{Context, RawInput, Rect, Shape, Ui, Vec2, pos2};
use elegance::{Button, Callout, CalloutTone, MenuItem, Theme, Toast, Toasts, glyphs};

/// Walk a shape tree, since a `Ui`'s output nests painted shapes under
/// `Shape::Vec`. `dyn` rather than `impl Trait` so the recursion monomorphizes.
fn visit(shape: &Shape, f: &mut dyn FnMut(&Shape)) {
    match shape {
        Shape::Vec(shapes) => {
            for shape in shapes {
                visit(shape, f);
            }
        }
        other => f(other),
    }
}

/// Toast text must never be justified.
///
/// `Ui::put` imposes `Layout::centered_and_justified`, which `Label` copies into
/// `LayoutJob::justify`; justification then stretches every row but the last out
/// to the full wrap width, letter-spacing any row that holds a single long word.
/// The description below wraps exactly that way: the path cannot share row one.
#[test]
fn toast_text_is_never_justified() {
    let ctx = Context::default();
    Theme::slate().install(&ctx);

    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(600.0, 400.0))),
        ..Default::default()
    };
    let frame = |ui: &mut egui::Ui| {
        Toast::new("Export finished")
            .description("Saved /var/tmp/exports/quarterly-report-archive.tar.gz")
            .show(ui.ctx());
        Toasts::new().render(ui.ctx());
    };
    // Two frames: the first enqueues and lets the toast Area settle.
    //
    // `TexturesDelta` asserts on drop that its deltas were applied — a real
    // integration uploads them to the GPU. This test only inspects shapes, so
    // discard them explicitly instead.
    let mut output = ctx.run_ui(input.clone(), frame);
    output.textures_delta.clear();
    let mut output = ctx.run_ui(input, frame);
    output.textures_delta.clear();

    let mut wrapped = false;
    for clipped in &output.shapes {
        visit(&clipped.shape, &mut |shape| {
            if let Shape::Text(text) = shape {
                assert!(
                    !text.galley.job.justify,
                    "justified toast text: {:?}",
                    text.galley.job.text
                );
                wrapped |= text.galley.rows.len() > 1;
            }
        });
    }
    // Guards the case above: a description that stopped wrapping would leave
    // the test passing without ever exercising a non-final row.
    assert!(wrapped, "the toast description did not wrap");
}

/// Run one frame of `ui` (with whatever passes egui takes) and return each
/// painted text's string and layout bounds (not ink bounds, which vary with
/// each glyph's side bearing).
fn painted_text(mut add: impl FnMut(&mut Ui)) -> Vec<(String, Rect)> {
    let ctx = Context::default();
    Theme::slate().install(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(520.0, 400.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| add(ui));
    output.textures_delta.clear();

    let mut texts = Vec::new();
    for clipped in &output.shapes {
        visit(&clipped.shape, &mut |shape| {
            if let Shape::Text(text) = shape {
                let bounds = Rect::from_min_size(text.pos, text.galley.size());
                texts.push((text.galley.text().to_owned(), bounds));
            }
        });
    }
    texts
}

fn left_of(texts: &[(String, Rect)], text: &str) -> f32 {
    texts
        .iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text:?} was not painted"))
        .1
        .left()
}

/// One item with a glyph moves every label in its menu past the gutter, from
/// the first frame on, including items laid out above it and items in other
/// uis within the menu.
#[test]
fn menu_labels_line_up_when_one_item_has_a_glyph() {
    let menu = |icon: bool| {
        painted_text(move |ui| {
            let body = egui::UiBuilder::new()
                .ui_stack_info(egui::UiStackInfo::new(egui::UiKind::Menu))
                .layout(egui::Layout::top_down_justified(egui::Align::Min));
            ui.scope_builder(body, |ui| {
                ui.add(MenuItem::new("Cut"));
                let copy = MenuItem::new("Copy");
                // A nested ui, as `add_enabled_ui` or a scroll area makes.
                ui.add_enabled_ui(true, |ui| {
                    ui.add(if icon {
                        copy.icon(glyphs::COPY.to_string())
                    } else {
                        copy
                    })
                });
                ui.add(MenuItem::new("Paste"));
            });
        })
    };

    let plain = menu(false);
    let x = left_of(&plain, "Cut");
    assert_eq!(left_of(&plain, "Copy"), x);

    let mixed = menu(true);
    let x_gutter = left_of(&mixed, "Cut");
    assert!(x_gutter > x, "no gutter reserved: {x_gutter} vs {x}");
    assert_eq!(left_of(&mixed, "Copy"), x_gutter);
    assert_eq!(left_of(&mixed, "Paste"), x_gutter);
}

/// Long callout text stops short of the actions instead of running under
/// them, in the one-line and the multi-line layouts.
#[test]
fn callout_text_leaves_room_for_actions() {
    const LONG: &str = "A long line of text that would otherwise run all the way across the banner and under its buttons";
    for multiline in [false, true] {
        let mut button = Rect::NOTHING;
        let texts = painted_text(|ui| {
            let mut open = true;
            let mut callout = Callout::new(CalloutTone::Warning)
                .title(LONG)
                .body(LONG)
                .dismissable(&mut open);
            if multiline {
                callout = callout.multiline();
            }
            button = callout
                .show(ui, |ui| ui.add(Button::new("Save")).rect)
                .inner;
        });
        // The text on the button's row; a multi-line body below it may span
        // the full width.
        let beside = texts
            .iter()
            .filter(|(t, r)| t.starts_with("A long") && r.y_range().intersects(button.y_range()));
        let mut count = 0;
        for (text, rect) in beside {
            count += 1;
            assert!(
                rect.right() <= button.left(),
                "multiline={multiline}: {text:?} ends at {} but the button starts at {}",
                rect.right(),
                button.left()
            );
        }
        assert!(
            count > 0,
            "multiline={multiline}: no text beside the button"
        );
    }
}
