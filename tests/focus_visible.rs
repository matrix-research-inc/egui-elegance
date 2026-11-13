//! Focus rings follow the web's `:focus-visible`: they show once the user
//! navigates by keyboard and stay hidden after pointer input.
//!
//! Modals and drawers focus their close button on open, so a ring on every
//! focused widget would mark that button each time a dialog opened by mouse.

use egui::{Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, Shape, Vec2, pos2};
use elegance::{Button, Theme};

/// The ring is the only 2px stroke a button paints.
const RING_WIDTH: f32 = 2.0;

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

/// A button on a themed context, driven one frame at a time.
struct Rig {
    ctx: Context,
    outline: bool,
}

impl Rig {
    fn new(outline: bool) -> Self {
        let ctx = Context::default();
        Theme::slate().install(&ctx);
        Self { ctx, outline }
    }

    /// Run one frame with `events`, optionally requesting focus on the button,
    /// and report whether the button painted a focus ring.
    fn frame(&self, events: Vec<Event>, request_focus: bool) -> bool {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(300.0, 200.0))),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, |ui| {
            let button = Button::new("Close");
            let button = if self.outline {
                button.outline()
            } else {
                button
            };
            let response = ui.add(button);
            if request_focus {
                response.request_focus();
            }
        });
        output.textures_delta.clear();

        let mut ringed = false;
        for clipped in &output.shapes {
            visit(&clipped.shape, &mut |shape| {
                if let Shape::Rect(rect) = shape {
                    ringed |= rect.stroke.width == RING_WIDTH;
                }
            });
        }
        ringed
    }

    /// Request focus, then run a frame so the button paints with it.
    fn focus(&self) -> bool {
        self.frame(Vec::new(), true);
        self.frame(Vec::new(), false)
    }
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A press on empty space, away from the button.
fn press() -> Event {
    Event::PointerButton {
        pos: pos2(250.0, 150.0),
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn programmatic_focus_shows_no_ring() {
    for outline in [false, true] {
        let rig = Rig::new(outline);
        rig.frame(Vec::new(), false);
        assert!(
            !rig.focus(),
            "outline={outline}: ring before any keyboard input"
        );
    }
}

#[test]
fn keyboard_input_shows_the_ring() {
    for outline in [false, true] {
        let rig = Rig::new(outline);
        rig.frame(Vec::new(), false);
        rig.frame(vec![key(Key::Tab)], false);
        assert!(rig.focus(), "outline={outline}: no ring after a key press");
    }
}

#[test]
fn pointer_input_hides_the_ring_again() {
    for outline in [false, true] {
        let rig = Rig::new(outline);
        rig.frame(Vec::new(), false);
        rig.frame(vec![key(Key::Tab)], false);
        assert!(rig.focus(), "outline={outline}: no ring after a key press");
        // A click that opens a dialog, which then focuses its close button.
        rig.frame(vec![press()], false);
        assert!(
            !rig.focus(),
            "outline={outline}: ring after a pointer press"
        );
    }
}

#[test]
fn tab_focus_shows_the_ring() {
    for outline in [false, true] {
        let rig = Rig::new(outline);
        rig.frame(Vec::new(), false);
        rig.frame(vec![key(Key::Tab)], false);
        let ringed = rig.frame(Vec::new(), false) | rig.frame(Vec::new(), false);
        assert!(ringed, "outline={outline}: Tab did not ring the button");
    }
}
