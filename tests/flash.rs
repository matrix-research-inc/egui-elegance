//! Submit-feedback flashes on text fields.
//!
//! `flash_success` / `flash_error` key their state by the `Response` id, and
//! the field reads it back while painting. Both ids have to agree, or the
//! flash is stored and never drawn. They are derived independently, so these
//! tests render the field and check the tint actually reaches the screen.

use eframe::egui;
use egui_kittest::Harness;
use elegance::{ResponseFlashExt, TextArea, TextInput, Theme};

struct Probe {
    text: String,
    flash: bool,
    rect: egui::Rect,
}

fn field_harness(
    add_field: fn(&mut egui::Ui, &mut String) -> egui::Response,
) -> Harness<'static, Probe> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(320.0, 160.0))
        .wgpu()
        .build_ui_state(
            move |ui, probe: &mut Probe| {
                Theme::slate().install(ui.ctx());
                let response = add_field(ui, &mut probe.text);
                probe.rect = response.rect;
                if std::mem::take(&mut probe.flash) {
                    response.flash_success();
                }
            },
            Probe {
                text: String::new(),
                flash: false,
                rect: egui::Rect::NOTHING,
            },
        );
    harness.run();
    harness
}

/// The field's fill, sampled inside its left padding so no text is under it.
fn fill_color(harness: &mut Harness<'_, Probe>) -> image::Rgba<u8> {
    let rect = harness.state().rect;
    let ppp = harness.ctx.pixels_per_point();
    let image = harness.render().expect("render");
    let (x, y) = (rect.left() + 3.0, rect.center().y);
    *image.get_pixel((x * ppp) as u32, (y * ppp) as u32)
}

fn assert_flash_tints(add_field: fn(&mut egui::Ui, &mut String) -> egui::Response) {
    let mut harness = field_harness(add_field);
    let resting = fill_color(&mut harness);

    harness.state_mut().flash = true;
    // Step rather than run: running to idle would play the flash out.
    harness.step();
    harness.step();
    let flashed = fill_color(&mut harness);

    assert_ne!(
        resting, flashed,
        "flash_success left the field fill unchanged"
    );
}

#[test]
fn text_input_shows_a_flash() {
    assert_flash_tints(|ui, text| ui.add(TextInput::new(text)).accessible_name("Field"));
}

#[test]
fn text_input_with_an_id_salt_shows_a_flash() {
    assert_flash_tints(|ui, text| {
        ui.add(TextInput::new(text).id_salt("flash_field"))
            .accessible_name("Field")
    });
}

#[test]
fn revealable_text_input_shows_a_flash() {
    assert_flash_tints(|ui, text| {
        ui.add(TextInput::new(text).revealable(true))
            .accessible_name("Field")
    });
}

#[test]
fn text_area_shows_a_flash() {
    assert_flash_tints(|ui, text| ui.add(TextArea::new(text)).accessible_name("Field"));
}
