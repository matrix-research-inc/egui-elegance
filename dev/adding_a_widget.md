# Adding a Widget to Elegance

A practical, step-by-step guide for landing a new widget in the `elegance` crate. It walks through every file you'll touch, the conventions you're expected to follow, and the patterns the existing widgets establish.

If you're prototyping a complex widget, write a design doc first (see `node_editor_plan.md` for the format) — the process below assumes you already know what you're building.

---

## 1. Decide What Kind of Widget You're Building

Three patterns recur in this crate. Pick the one that matches the way your widget is used; the rest of this guide branches on it.

### 1a. Leaf widget — `impl Widget`, called via `ui.add(...)`

A self-contained control the user drops into a layout. Renders itself in one rectangle and returns a `Response`. The caller owns any bound value.

- **Stateless:** [`Button`](../src/button.rs), [`Badge`](../src/badge.rs), [`Spinner`](../src/spinner.rs), [`StatusPill`](../src/pill.rs), [`Indicator`](../src/indicator.rs).
- **Bound to caller state:** [`TextInput`](../src/input.rs), [`Switch`](../src/switch.rs), [`Slider`](../src/slider.rs), [`Checkbox`](../src/checkbox.rs), [`Select`](../src/select.rs).

### 1b. Container widget — `.show(ui, |ui| ...)` returning `InnerResponse<R>`

Wraps a region of the UI in elegance chrome (frame, padding, header). The body is a closure the caller fills with arbitrary widgets.

- [`Card`](../src/card.rs), [`CollapsingSection`](../src/collapsing.rs).

### 1c. Owned-state widget — caller holds the struct, calls `.show(ui)`

Heavier widgets that own a non-trivial chunk of state across frames (collections, interaction state machines, animation timers). The caller stores the widget itself in their app struct and mutates it directly.

- [`LogBar`](../src/log_bar.rs), [`Toasts`](../src/toast.rs), [`Pairing`](../src/pairing.rs), [`Modal`](../src/modal.rs).

If you're unsure: start with a leaf widget. Promote to owned-state only if you genuinely need state that survives across frames *and* doesn't fit naturally in egui's `ctx.data_mut()` keyed by an `id_salt`.

---

## 2. Create the Module

Each widget lives in its own file under `src/`. Filename = snake_case of the type, e.g. `src/text_input.rs` for `TextInput`.

Wire it up in `src/lib.rs`:

```rust
// src/lib.rs — add the module declaration in alphabetical order:
mod my_widget;

// ...and re-export the public API. Include any public enums the widget
// uses (size variants, kind/tone enums, etc.) — they don't need to exist
// if your widget doesn't have them.
pub use my_widget::{MyWidget, MyWidgetSize};
```

`#![deny(missing_docs)]` is set crate-wide — every public item must have a doc comment or the build fails.

---

## 3. Write the Widget

### 3.1 Struct + builder

Match the conventions in [`Button`](../src/button.rs):

```rust
//! One-line summary of the widget.
//!
//! A short paragraph describing the visual treatment, when to use it, and
//! anything callers need to know up front.

use egui::{Response, Role, Sense, Ui, Widget, WidgetInfo, WidgetText};

use crate::theme::Theme;

/// What the widget is, in one line.
///
/// ```no_run
/// # use elegance::MyWidget;
/// # egui::__run_test_ui(|ui| {
/// ui.add(MyWidget::new("Label"));
/// # });
/// ```
#[must_use = "Call `ui.add(...)` to render the widget."]
pub struct MyWidget {
    text: WidgetText,
    enabled: bool,
    // ...other fields, all with sensible defaults.
}

impl MyWidget {
    /// Create a new widget. Document the defaults here.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            enabled: true,
        }
    }

    /// Disable the widget. Default: enabled.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
```

Conventions to follow:

- **Builder methods take `mut self` and return `Self`.** Mark them `#[inline]`.
- **`#[must_use]` on the struct** with a message reminding the caller to actually render it.
- **Manual `Debug` impl** when fields don't all `impl Debug` themselves — `WidgetText`, `Galley`, closures, and `&mut T` borrows generally don't, so derive will fail. See `Button` for the pattern: list the configuration fields and skip the rest.
- **Bound state takes `&'a mut T`** and the struct gains a lifetime: `pub struct TextInput<'a> { text: &'a mut String, ... }`.
- **Defaults match the rest of the family.** `Accent::Blue`, `ButtonSize::Medium`, enabled, no min width, etc.

### 3.2 Implement `Widget` (leaf widgets)

```rust
impl Widget for MyWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::current(ui.ctx());

        // 1. Measure.
        let desired = /* compute size from theme padding, typography, content */;

        // 2. Allocate + sense.
        let sense = if self.enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(desired, sense);

        // 3. Paint.
        if ui.is_rect_visible(rect) {
            // Read state off `response`: hovered(), is_pointer_button_down_on(),
            // has_focus(). Resolve colors from `theme.palette`.
            ui.painter().rect(rect, /* radius */, /* fill */, /* stroke */, egui::StrokeKind::Inside);
            // Paint text/icons via ui.painter().galley(...).
        }

        // 4. Accessibility.
        response.widget_info(|| {
            WidgetInfo::labeled(Role::Button, self.enabled, self.text.text())
        });

        response
    }
}
```

The order — **measure, allocate, paint-if-visible, widget_info, return** — is the same in every leaf widget; follow it. `Button::ui` is the canonical reference.

### 3.3 Container widgets — `show(ui, add_contents)`

```rust
impl MyContainer {
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let theme = Theme::current(ui.ctx());
        let frame = egui::Frame::new()
            .fill(theme.palette.card)
            .stroke(egui::Stroke::new(1.0, theme.palette.border))
            .corner_radius(egui::CornerRadius::same(theme.card_radius as u8))
            .inner_margin(egui::Margin::same(theme.card_padding as i8));
        frame.show(ui, |ui| add_contents(ui))
    }
}
```

See [`Card`](../src/card.rs) for the full pattern, including how to render an optional heading inside the frame.

### 3.4 Owned-state widgets — caller holds the struct

The caller stores the widget in their app struct (`my_widget: MyWidget`) and renders it with `widget.show(ui)`. Internal state lives on the struct itself; transient *interaction* state (current drag, last hover) goes in `ctx.data_mut()` keyed by an `Id::unique(self.id_salt)`. See [`Pairing`](../src/pairing.rs) for the full treatment, and [`LogBar`](../src/log_bar.rs) for a simpler one.

Owned-state widgets typically take an `id_salt: impl Hash` in their constructor so multiple instances coexist.

---

## 4. Use the Theme — Don't Hardcode Anything Visual

Pull every color, size, and radius from the active theme. Hardcoded values will look wrong on at least one of the four shipped palettes.

```rust
let theme = Theme::current(ui.ctx());
let p = &theme.palette;
let t = &theme.typography;
```

The fields you'll reach for most often:

| Need | Use |
|---|---|
| Page background | `p.bg` |
| Card / panel fill | `p.card` |
| Input fill | `p.input_bg` |
| 1px border | `p.border` |
| Body text | `p.text` |
| Secondary text | `p.text_muted` |
| Tertiary text | `p.text_faint` |
| Accent fill (resting) | `p.accent_fill(accent)` |
| Accent fill (hover) | `p.accent_hover(accent)` |
| Focus ring | `p.sky` (typically with `with_alpha(p.sky, 180)`) |
| Success / danger / warning | `p.success`, `p.danger`, `p.warning` |
| Body / button / label / small / heading / monospace | `t.body`, `t.button`, `t.label`, `t.small`, `t.heading`, `t.monospace` |
| Control corner radius | `theme.control_radius` |
| Card corner radius | `theme.card_radius` |
| Control padding | `theme.control_padding_x`, `theme.control_padding_y` |
| Card padding | `theme.card_padding` |

Helpers in [`src/theme.rs`](../src/theme.rs):

- `mix(a, b, t)` — blend two colors.
- `with_alpha(c, alpha)` — set alpha on a `Color32`.
- `depth_tint(base, t)` — lift toward white on dark themes, deepen toward black on light themes. Use this for subtle hover/pressed shifts that work on every palette.
- `themed_input_visuals(visuals, theme)` — when wrapping an existing egui control (like `egui::TextEdit`), apply this to the visuals stack so the control inherits elegance styling.

**Light vs dark:** never branch on the theme name. Branch on `p.is_dark` if you must, but `depth_tint` and the palette accessors usually mean you don't need to.

---

## 5. Accessibility

Every widget that takes focus or accepts input must emit `WidgetInfo` so screen readers — and the kittest harness used by snapshot tests — can find it. The shape of that info depends on the widget.

**Stateless action (button, link):**

```rust
response.widget_info(|| {
    WidgetInfo::labeled(Role::Button, self.enabled, self.text.text())
});
```

**Toggle / checkbox / switch:** report the *current* selected state, not just the label.

```rust
response.widget_info(|| {
    WidgetInfo::selected(Role::CheckBox, self.enabled, *self.value, self.label)
});
```

`Switch` and `Checkbox` are good references — both call `selected(...)` so a screen reader announces "on" / "off" as the user toggles.

**Text-bearing input (text input, text area):** use `WidgetInfo::labeled` with `Role::TextInput` and supply the *label* as the third argument, not the input contents — the contents are spoken separately by the OS.

**Optional label:** a widget whose `.label(...)` is optional is unnamed without one, and `egui_kittest` fails any harness that leaves a widget unnamed. Return the response of the widget's accessible node (the one carrying its role) so callers can name it with egui's `Response::accessible_name` or `Response::labelled_by`, and add the widget to `unlabelled_widgets_take_an_accessible_name_from_their_response` in `tests/a11y.rs`.

**Overlay (modal, toast, menu):** set the matching accesskit role (`Dialog`, `AlertDialog`, `Menu`) on the containing `Area`/`Frame` so assistive tech treats the overlay as a focus boundary. `Modal` traps focus inside while open and restores it on close — mirror that if your overlay is dismissable.

**Keyboard interaction:** any widget that accepts pointer input also needs to work from the keyboard. At a minimum, `Sense::click()` already gives you Space/Enter activation when the widget is focused. For composite widgets (segmented buttons, tab bars, menus), handle arrow-key navigation between sub-elements explicitly — see `TabBar` and `SegmentedButton` for the pattern.

The full a11y test patterns — focus traversal, role assertions, focus-trap behavior — are in `tests/a11y.rs`. Read it before adding a widget that participates in any of those.

---

## 6. Add Snapshot Tests

Visual regression coverage lives in [`tests/visual.rs`](../tests/visual.rs). Every widget gets a render function and a `theme_tests!` invocation, which expands into one test per built-in theme.

### 6.1 Add a render function

```rust
// tests/visual.rs
fn my_widget_ui(ui: &mut egui::Ui) {
    ui.add(MyWidget::new("Default"));
    ui.add_space(8.0);
    ui.add(MyWidget::new("Disabled").enabled(false));
}
```

Keep render functions **deterministic** — declare any mutable state locally inside the function so the snapshot doesn't drift between runs. The harness already runs the UI a few times to settle layout.

### 6.2 Register the theme expansion

```rust
theme_tests!(my_widget, my_widget_ui);
```

This generates `my_widget::slate`, `my_widget::charcoal`, `my_widget::frost`, and `my_widget::paper` tests. Snapshots are written to `tests/snapshots/my_widget_{theme}.png`.

### 6.3 Snapshot interaction states (optional)

If your widget's hovered/focused/pressed states are visually distinct enough to be worth pinning, use `snap_with_setup`:

```rust
fn my_widget_focused(harness: &Harness<'_>) {
    harness.get_by_label("Default").focus();
}

#[test]
fn my_widget_focused_slate() {
    snap_with_setup(
        "my_widget_focused_slate",
        Theme::slate(),
        my_widget_ui,
        my_widget_focused,
    );
}
```

Naming: `{widget}_{state}_{theme}.png`. State words **stack alphabetically** when more than one applies, so the dirty + focused variant is `text_input_dirty_focused_*.png`, not `text_input_focused_dirty_*.png`. Keep ordering consistent so a glance at the snapshots directory is enough to tell what each file shows.

### 6.4 Generate the baseline images

```sh
UPDATE_SNAPSHOTS=true cargo test --test visual
```

Inspect the generated PNGs in `tests/snapshots/` and **commit them**. On any subsequent run a mismatch writes `*.new.png` and `*.diff.png` next to the baseline; review the diff before regenerating.

### 6.5 Accessibility tests (when relevant)

If your widget participates in focus management, screen-reader output, or keyboard nav, add a corresponding test in [`tests/a11y.rs`](../tests/a11y.rs).

---

## 7. Add the Widget to the Reference Example

The widgets reference in [`examples/widgets.rs`](../examples/widgets.rs) is the showcase users browse. It's organized into a `TabBar` of broad categories (Inputs / Display / Layout / Overlays at the time of writing — read the file for the current set, since the list grows).

To add your widget:

1. Pick the category your widget belongs to. If none of them fit cleanly, raise it in the PR — adding a tab is fine but not done lightly.
2. Add a `section_my_widget(&mut self, ui: &mut egui::Ui)` method on the example app, wrapping the demo in a `Card` with a heading.
3. Add any state your widget needs as fields on the app struct.
4. Call `self.section_my_widget(ui)` from inside the matching tab branch.

---

## 8. Document the Widget in the README

Add an entry to the widget catalog in [`README.md`](../README.md) following the existing format: a screenshot, a one-line description, and a code snippet. Extend [`examples/render_docs.rs`](../examples/render_docs.rs) with a `render_<widget>()` tile, register it in `main()`, run `cargo render-docs`, and commit the resulting `docs/images/<widget>.png`.

---

## 9. Pre-PR Checklist

- [ ] Widget module compiles with `#![deny(missing_docs)]` — every public item documented.
- [ ] No hardcoded colors, font sizes, radii, or spacing in the widget itself — everything reads from `Theme::current(ui.ctx())`. (Test/example code may use literal `add_space` values for layout.)
- [ ] `widget_info` emitted on the returned `Response` — `selected(...)` for toggles, `labeled(...)` otherwise.
- [ ] Re-exported from `src/lib.rs`.
- [ ] Snapshot tests added to `tests/visual.rs` and baselines committed for all four themes.
- [ ] If the widget uses `std::time::Instant`, `std::thread`, the filesystem, or other host-only APIs, verified `cargo build --target wasm32-unknown-unknown` still passes — see `LogBar` for how to gate timestamp code per target.
- [ ] Added to the appropriate tab in `examples/widgets.rs`.
- [ ] Tile added to `examples/render_docs.rs` and `docs/images/<widget>.png` committed.
- [ ] Documented in `README.md` with screenshot, description, and code snippet.
- [ ] `cargo test`, `cargo clippy`, `cargo fmt` clean.
- [ ] Verified visually in `cargo run --example widgets` on at least one dark and one light theme.

---

## Reference: Where to Crib From

| If your widget is like... | Read |
|---|---|
| A button or toggle | [`src/button.rs`](../src/button.rs), [`src/switch.rs`](../src/switch.rs) |
| A bound input | [`src/input.rs`](../src/input.rs), [`src/slider.rs`](../src/slider.rs) |
| A status badge | [`src/badge.rs`](../src/badge.rs), [`src/pill.rs`](../src/pill.rs), [`src/indicator.rs`](../src/indicator.rs) |
| A container | [`src/card.rs`](../src/card.rs), [`src/collapsing.rs`](../src/collapsing.rs) |
| A tabbed/segmented selector | [`src/tabs.rs`](../src/tabs.rs), [`src/segmented.rs`](../src/segmented.rs) |
| An overlay | [`src/modal.rs`](../src/modal.rs), [`src/toast.rs`](../src/toast.rs), [`src/menu.rs`](../src/menu.rs) |
| Owned, complex, with interaction state | [`src/pairing.rs`](../src/pairing.rs), [`src/log_bar.rs`](../src/log_bar.rs) |
