//! Shared focus policy: when to draw focus, and focus on press for value
//! widgets.

use egui::{Context, Event, Id, RawInput, Response, TouchPhase};

/// Whether to draw `response`'s focus indicator: it has focus, and the user
/// is navigating with the keyboard.
///
/// Like the web's `:focus-visible`. Focus also lands on widgets without the
/// keyboard: a modal focuses its close button when it opens, and a slider
/// takes focus when grabbed (see [`focus_on_press`]). A ring there, after a
/// mouse click, is noise; after `Tab` or a keyboard shortcut it shows where
/// the next key press goes.
///
/// Draw a widget's focus indicator only when this is true. Use plain
/// [`Response::has_focus`] for keyboard handling, which applies however the
/// widget got focus.
pub(crate) fn shows_focus(response: &Response) -> bool {
    response.has_focus() && keyboard_navigation(&response.ctx)
}

/// Whether the latest input was the keyboard (or an assistive technology)
/// rather than a pointer, as recorded by [`FocusVisible`].
///
/// Without the plugin, which [`Theme::install`](crate::Theme::install)
/// registers, there is no record, and focus always shows.
fn keyboard_navigation(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(keyboard_navigation_id()))
        .unwrap_or(true)
}

fn keyboard_navigation_id() -> Id {
    Id::unique("elegance::keyboard_navigation")
}

/// Records whether the latest input was the keyboard or a pointer, for
/// [`shows_focus`].
///
/// A plugin rather than a check inside widgets, because it has to see every
/// frame's input: the click that opens a modal arrives in a frame where the
/// modal's close button doesn't exist yet to look.
pub(crate) struct FocusVisible;

impl egui::Plugin for FocusVisible {
    fn debug_name(&self) -> &'static str {
        "elegance::FocusVisible"
    }

    fn setup(&mut self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(keyboard_navigation_id(), false));
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        let latest = input.events.iter().rev().find_map(|event| match event {
            Event::Key { pressed: true, .. } | Event::AccessKitActionRequest(_) => Some(true),
            Event::PointerButton { pressed: true, .. }
            | Event::Touch {
                phase: TouchPhase::Start,
                ..
            } => Some(false),
            _ => None,
        });
        if let Some(keyboard) = latest {
            ctx.data_mut(|d| d.insert_temp(keyboard_navigation_id(), keyboard));
        }
    }
}

/// Give a value widget keyboard focus when the pointer grabs it.
///
/// egui does not focus a widget on press: only `TextEdit` and `DragValue`
/// call [`Response::request_focus`] themselves, so `egui::Slider` and every
/// plain button stay unfocused after a click. For a button that is harmless,
/// because its keyboard path is the same activation as the click. For a value
/// widget it is not: the pointer sets a coarse value and the arrow keys refine
/// it, so a user who clicks a track and then presses an arrow key expects the
/// value to move, and instead the key press goes to focus navigation. Native
/// sliders and `<input type="range">` both focus on press for that reason.
///
/// Call this from any widget whose bound value responds to keys, passing the
/// response the pointer is actually down on, and call it before the widget's own
/// `has_focus` keyboard block so that block sees the focus in the same pass.
///
/// A widget that binds the arrow keys also needs `set_focus_lock_filter`, or
/// egui routes the key to spatial focus navigation as well and the focus this
/// grants lands on a neighbour after the first press.
pub(crate) fn focus_on_press(response: &Response) {
    if !response.is_pointer_button_down_on() {
        return;
    }

    // `is_pointer_button_down_on` stays true until the button is released, so a
    // widget grabbed while it was eligible keeps reporting the grab after it
    // stops being eligible — a slider disabled mid-drag, or one a `Modal` opened
    // over. egui surrenders such a widget's focus every pass (`Context::
    // create_widget`), but `Memory::request_focus` performs neither check, so
    // without these two guards we would take the focus straight back on every
    // frame of the drag and hold it against the modal's focus trap.
    if !response.enabled() {
        return;
    }
    if !response
        .ctx
        .memory(|m| m.allows_interaction(response.layer_id))
    {
        return;
    }

    // Read focus from memory rather than through `Response::has_focus`, which is
    // additionally false whenever the host window is unfocused. Re-requesting
    // focus we already hold is harmless to the `set_focus_lock_filter` the
    // keyboard handlers install — that is reinstalled later in the same pass,
    // before anything reads it — but it does interrupt IME composition, so skip
    // it for every frame of the drag after the first.
    let already_focused = response.ctx.memory(|m| m.has_focus(response.id));
    if !already_focused {
        response.request_focus();
    }
}
