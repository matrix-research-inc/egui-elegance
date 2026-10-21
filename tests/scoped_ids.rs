//! Widget id salts are scoped, not app-wide.
//!
//! A salt only has to be unique among its siblings: two widgets built with the
//! same salt under different parents keep separate state. Each test shows the
//! same widget twice, under `push_id` scopes "a" and "b", acts on the first,
//! and checks the second did not follow.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use elegance::{
    Accordion, BrowserTab, BrowserTabs, Button, CollapsingSection, ColorPicker, ContextMenu, Menu,
    MenuBar, MenuItem, Popover, Theme,
};

const SALT: &str = "shared";
const SIDES: [&str; 2] = ["a", "b"];

fn harness(mut app: impl FnMut(&mut egui::Ui, &str) + 'static) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(640.0, 480.0))
        .build_ui(move |ui| {
            Theme::slate().install(ui.ctx());
            ui.horizontal_top(|ui| {
                for side in SIDES {
                    ui.push_id(side, |ui| ui.vertical(|ui| app(ui, side)));
                }
            });
        });
    harness.run();
    harness
}

fn assert_only_a_shows(harness: &Harness<'_>, what: &str) {
    assert!(
        harness.query_by_label(&format!("{what} a")).is_some(),
        "{what} a should show"
    );
    assert!(
        harness.query_by_label(&format!("{what} b")).is_none(),
        "{what} b shares only a salt with a, so it should stay closed"
    );
}

#[test]
fn popovers_on_different_triggers_open_separately() {
    let mut harness = harness(|ui, side| {
        let trigger = ui.add(Button::new(format!("Trigger {side}")));
        Popover::new(SALT).show(&trigger, |ui| ui.label(format!("Popover {side}")));
    });
    harness.get_by_label("Trigger a").click();
    harness.run();
    assert_only_a_shows(&harness, "Popover");
}

#[test]
fn menus_on_different_triggers_open_separately() {
    let mut harness = harness(|ui, side| {
        let trigger = ui.add(Button::new(format!("Trigger {side}")));
        Menu::new(SALT).show_below(&trigger, |ui| ui.add(MenuItem::new(format!("Menu {side}"))));
    });
    harness.get_by_label("Trigger a").click();
    harness.run();
    assert_only_a_shows(&harness, "Menu");
}

#[test]
fn context_menus_on_different_targets_open_separately() {
    let mut harness = harness(|ui, side| {
        let target = ui.add(Button::new(format!("Target {side}")));
        ContextMenu::new(SALT).show(&target, |ui| {
            ui.add(MenuItem::new(format!("Context menu {side}")))
        });
    });
    harness.get_by_label("Target a").click_secondary();
    harness.run();
    assert_only_a_shows(&harness, "Context menu");
}

#[test]
fn collapsing_sections_open_separately() {
    let mut harness = harness(|ui, side| {
        CollapsingSection::new(SALT, format!("Section {side}"))
            .show(ui, |ui| ui.label(format!("Body {side}")));
    });
    harness.get_by_label("Section a").click();
    harness.run();
    assert_only_a_shows(&harness, "Body");
}

#[test]
fn accordion_items_open_separately() {
    let mut harness = harness(|ui, side| {
        Accordion::new(SALT).show(ui, |acc| {
            acc.item(format!("Item {side}"))
                .show(|ui| ui.label(format!("Body {side}")));
        });
    });
    harness.get_by_label("Item a").click();
    harness.run();
    assert_only_a_shows(&harness, "Body");
}

#[test]
fn menu_bars_open_separately() {
    let mut harness = harness(|ui, side| {
        MenuBar::new(SALT).show(ui, |bar| {
            bar.menu(format!("File {side}"), |ui| {
                ui.add(MenuItem::new(format!("Quit {side}")));
            });
        });
    });
    harness.get_by_label("File a").click();
    harness.run();
    assert_only_a_shows(&harness, "Quit");
}

#[test]
fn color_pickers_open_separately() {
    let mut harness = harness(|ui, side| {
        let mut color = egui::Color32::from_rgb(0x38, 0xbd, 0xf8);
        ui.add(ColorPicker::new(SALT, &mut color).label(format!("Color {side}")));
    });
    harness
        .get_by_role_and_label(egui::accesskit::Role::ColorWell, "Color a")
        .click();
    harness.run();
    let open = harness
        .query_all_by_role_and_label(egui::accesskit::Role::Slider, "Hue")
        .count();
    assert_eq!(open, 1, "only the clicked picker's popover should open");
}

#[test]
fn browser_tab_strips_select_separately() {
    let strips = std::rc::Rc::new(std::cell::RefCell::new(SIDES.map(|side| {
        BrowserTabs::new(SALT)
            .with_tab(BrowserTab::new("one", format!("One {side}")))
            .with_tab(BrowserTab::new("two", format!("Two {side}")))
    })));
    let shown = strips.clone();
    let mut harness = harness(move |ui, side| {
        let index = SIDES.iter().position(|s| *s == side).unwrap();
        shown.borrow_mut()[index].show(ui);
    });
    let before = strips.borrow()[1].selected().map(str::to_owned);

    harness.get_by_label("Two a").click();
    harness.run();

    let strips = strips.borrow();
    assert_eq!(strips[0].selected(), Some("two"));
    assert_eq!(
        strips[1].selected().map(str::to_owned),
        before,
        "clicking a tab in strip a should leave strip b's selection alone"
    );
}
