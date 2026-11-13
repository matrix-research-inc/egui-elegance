//! Focus rings follow the web's `:focus-visible`: they show once the user
//! navigates by keyboard and stay hidden after pointer input.
//!
//! Modals and drawers focus their close button on open, and value widgets
//! take focus when grabbed, so a ring on every focused widget would mark the
//! close button of each dialog opened by mouse and every slider after a drag.
//!
//! Each test compares a frame painted with focus against one painted without:
//! "no ring" means focus changed nothing on screen, whatever the ring's style.

use std::cell::Cell;

use egui::epaint::ClippedShape;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Response, Ui};
use elegance::{Button, Slider, Theme};

/// One widget on a context, driven one frame at a time.
struct Rig {
    ctx: Context,
    add: fn(&mut Ui, &mut f32) -> Response,
    value: Cell<f32>,
    rect: Cell<Rect>,
}

impl Rig {
    fn new(add: fn(&mut Ui, &mut f32) -> Response, install_theme: bool) -> Self {
        let ctx = Context::default();
        if install_theme {
            Theme::slate().install(&ctx);
        }
        let rig = Self {
            ctx,
            add,
            value: Cell::new(0.0),
            rect: Cell::new(Rect::NOTHING),
        };
        rig.frame(Vec::new(), false);
        rig
    }

    /// Run one frame with `events`, optionally requesting focus on the widget,
    /// and return what it painted.
    fn frame(&self, events: Vec<Event>, request_focus: bool) -> Vec<ClippedShape> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 200.0))),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| {
            let mut value = self.value.get();
            let response = (self.add)(ui, &mut value);
            self.value.set(value);
            self.rect.set(response.rect);
            if request_focus {
                response.request_focus();
            }
        });
        output.textures_delta.clear();
        output.shapes
    }

    fn focused(&self) -> bool {
        self.ctx.memory(|m| m.focused().is_some())
    }

    /// After `events`, whether focus changes what the widget paints: compare
    /// a frame without focus against one with focus requested from code, as a
    /// modal does on open.
    fn ring_after(&self, events: Vec<Event>) -> bool {
        self.frame(events, false);
        self.ring_now()
    }

    /// Whether the widget's current focus changes what it paints.
    fn ring_now(&self) -> bool {
        self.ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        let unfocused = self.frame(Vec::new(), false);
        self.frame(Vec::new(), true);
        let focused = self.frame(Vec::new(), false);
        assert!(self.focused(), "the widget did not take focus");
        focused != unfocused
    }
}

fn button(ui: &mut Ui, _: &mut f32) -> Response {
    ui.add(Button::new("Close"))
}

fn outline_button(ui: &mut Ui, _: &mut f32) -> Response {
    ui.add(Button::new("Close").outline())
}

const BUTTONS: [fn(&mut Ui, &mut f32) -> Response; 2] = [button, outline_button];

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn pointer(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A click on empty space, away from the widget.
fn click() -> Vec<Event> {
    let pos = egui::pos2(350.0, 180.0);
    vec![pointer(pos, true), pointer(pos, false)]
}

#[test]
fn no_ring_before_any_keyboard_input() {
    for add in BUTTONS {
        assert!(!Rig::new(add, true).ring_after(Vec::new()));
    }
}

#[test]
fn shortcut_then_focus_from_code_shows_the_ring() {
    // A dialog opened by keyboard shortcut rings its close button.
    for add in BUTTONS {
        let rig = Rig::new(add, true);
        assert!(rig.ring_after(vec![key(Key::K, Modifiers::COMMAND)]));
    }
}

#[test]
fn tab_shows_the_ring() {
    for add in BUTTONS {
        let rig = Rig::new(add, true);
        let unfocused = rig.frame(Vec::new(), false);
        let tabbed = rig.frame(vec![key(Key::Tab, Modifiers::NONE)], false);
        assert!(rig.focused(), "Tab did not focus the button");
        assert_ne!(tabbed, unfocused, "Tab focused the button without a ring");
    }
}

#[test]
fn click_hides_the_ring_again() {
    for add in BUTTONS {
        let rig = Rig::new(add, true);
        assert!(rig.ring_after(vec![key(Key::K, Modifiers::NONE)]));
        // A click that opens a dialog, which then focuses its close button.
        assert!(!rig.ring_after(click()));
    }
}

#[test]
fn modifier_alone_is_not_keyboard_navigation() {
    // Shift-click, Ctrl-scroll and app switching press a modifier by itself.
    for add in BUTTONS {
        let rig = Rig::new(add, true);
        let mut events = click();
        events.push(key(Key::ShiftLeft, Modifiers::SHIFT));
        assert!(!rig.ring_after(events));
    }
}

#[test]
fn without_theme_install_the_ring_always_shows() {
    // Without the plugin there is no record of the input, so keep the old
    // behaviour rather than hide focus from keyboard users.
    for add in BUTTONS {
        assert!(Rig::new(add, false).ring_after(Vec::new()));
    }
}

#[test]
fn clicked_slider_shows_no_halo() {
    // A slider takes focus on press so arrow keys nudge it afterwards; that
    // focus alone must not draw the halo once the pointer lets go.
    let rig = Rig::new(|ui, value| ui.add(Slider::new(value, 0.0..=100.0)), true);
    let center = rig.rect.get().center();
    rig.frame(vec![Event::PointerMoved(center)], false);
    rig.frame(vec![pointer(center, true)], false);
    rig.frame(vec![pointer(center, false)], false);
    assert!(rig.focused(), "the click did not focus the slider");
    assert!(!rig.ring_now());
}
