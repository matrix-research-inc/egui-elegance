# Changelog

Notable changes to `egui-elegance`. Where a version has [GitHub release notes](https://github.com/stephenberry/egui-elegance/releases), its heading links to them for the full detail including migration examples.

Versions before 0.11.0 predate tagging; see the git history for those.

## Unreleased

### Changed

- **`Select` sizes to its widest option by default.** Without `.width(...)`, every select was a fixed 160pt wide, contradicting its docs. It now fits its longest option label plus the chevron, and its width doesn't change with the selection. Pass `.width(160.0)` to keep the old size.

## [0.16.1](https://github.com/stephenberry/egui-elegance/releases/tag/v0.16.1) — 2026-09-30

### Added

- **`Button::icon(glyph, label)`** — a square icon-only button showing one of the bundled Lucide `glyphs`. A glyph passed to `Button::new` was laid out at the size preset's *label* size inside a box padded for text, so it inked under a third of a wide rectangle at every `ButtonSize`. An icon button is as tall as a text button of the same size, so the two line up in a row, and sizes its glyph off that box instead. `label` is the accessible name rather than a painted label, and is required: an icon alone announces nothing to a screen reader. Every other builder method applies unchanged.
- **Custom content in `MenuBar`.** `MenuBarUi::ui()` exposes the strip's `Ui` for widgets inline with the menu triggers, and `MenuBarUi::trailing(|ui| …)` pins content to the right edge, outside the status slot. The trailing slot suits window controls when the bar serves as an app's title bar.
- **`MenuBar::title_bar(true)`** lets the strip stand in for a hidden native title bar: dragging its empty area moves the window, and double-clicking it toggles maximised. Menu triggers and custom widgets keep their own clicks.

### Changed

- `Button::new` now documents that it uses only the string content of its argument. It takes `impl Into<WidgetText>` to match egui's own signature, but the label is always laid out at the size preset in the button's state colour, so a `RichText`'s size, colour, and style are silently dropped. Reach for `Button::icon` instead of sizing the text.

## [0.16.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.16.0) — 2026-08-10

### Added

- **Commit signal.** New `ResponseCommitExt::committed()` reports the frame a value adjustment settles: on pointer release after a drag or a click (any mouse button, plus touch long-press), immediately on a keyboard nudge, and — for the `Knob`'s scroll wheel — once when the scroll stops rather than once per frame of its smoothed delta. `changed()` still fires on every intermediate value, which is what a live preview wants; `committed()` is for reactions you would not want to repeat dozens of times for one gesture, such as a network write or a disk persist. A `MetricSlider` in `stops` mode previously fired `changed()` once per stop a drag crossed, with no affordance to tell an intermediate value from a settled one. Works on `MetricSlider`, `PercentSlider`, `Slider`, `RangeSlider`, and `Knob`.
- **Per-item icon tint on `Pairing`.** `PairItem::icon_tone(BadgeTone)` tints a node's leading icon with the crate's status vocabulary; `PairItem::icon_color(impl Into<Color32>)` takes an explicit colour. Untinted icons are unchanged.
- `BadgeTone::foreground(&Palette)` is now public — the resolver behind badge labels and icon tones.
- **`glyphs::FOLDER` and `glyphs::FOLDER_OPEN`** (Lucide `folder` / `folder-open`, `U+E023`–`U+E024`), for directories, collections, and breadcrumb roots, with the open variant for the expanded or current node. Apps were falling back to the 📁 emoji, which renders in the system emoji font and reads as a different icon set beside elegance's Lucide glyphs.
- **`Select::saved(&T)`** marks a *staged* select — one whose selection has moved off the last committed value — with the focus-accent dot `BrowserTab` already uses for unsaved work, so a picker can read as unsaved beside a dirty `TextInput` in the same form. Pass the last committed value rather than a `dirty` flag and the select makes the comparison itself. Needs `.label(…)`, since the dot rides the label.
- **`ColorPicker` is keyboard-operable.** Its saturation/value plane, hue strip, and alpha strip were pointer-only: `Tab` landed on them, because they were focusable, but they answered no keys, so the arrows navigated straight back off and a keyboard user could not pick a colour at all. The plane now takes `←`/`→` for saturation and `↑`/`↓` for value; the two strips take `←`/`→` plus `Home`/`End`; `Shift` is a 10x nudge throughout. Each surface draws a focus ring, announces itself to screen readers (previously three unlabelled targets), and records to the recents row on a keyboard adjustment just as it does on a pointer release — one entry per run of presses, so holding an arrow key down cannot flood the row.

### Changed

- **Grabbing a value widget now gives it keyboard focus.** Clicking a `MetricSlider`, `PercentSlider`, `Slider`, or `Knob` and then pressing an arrow key did nothing: the widget never took focus on press, so the key went to focus navigation instead. You can now set a coarse value with the pointer and refine it with the arrow keys, without tabbing to the widget first. `RangeSlider` already did this for a click on the track, and now does it for a press that lands directly on a thumb too.
- **`Knob` no longer resets to its `default` on Space**, only on `0` (and the documented Alt+click / double-click). egui synthesises a click from Space on any focused widget, and knobs are now focused by the pointer, so the old binding would have wiped the value of a knob you had just set by hand.

### Fixed

- Arrow keys on a focused `Knob` moved the value *and* handed focus to the neighbouring widget, because the knob never claimed the arrows the way the sliders do. A knob next to another control nudged once and then drove its neighbour; a knob above a slider became unreachable by keyboard entirely. `Tab` and `Esc` still move focus away.
- `ColorPicker` now adds to its recents list on a non-primary click or a touch long-press of the SV plane, hue strip, or alpha slider, matching what those controls already did for a primary click or drag.
- **Breaking:** `PairItem` gained a public `icon_tint` field and is now `#[non_exhaustive]`, as is the new `IconTint` enum. Construct with `PairItem::new(...)` plus the builder methods; struct literals, functional record update (`..other`), and destructuring without `..` no longer compile. Doing this once now avoids a second break the next time the struct grows.

## [0.15.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.15.0) — 2026-08-06

- **Breaking:** targets egui 0.36, so your app must move to 0.36 as well. MSRV 1.92 → 1.95.
- **Breaking:** `FileDropResponse::dropped_files` is now `Vec<egui::DroppedFileHandle>`; `path` and `bytes` are methods rather than fields.
- Fixed overlay layering: a foreground `egui::Window` rendered above an open `Modal`'s backdrop and stayed clickable ([#13](https://github.com/stephenberry/egui-elegance/issues/13)). `Esc` now dismisses only the topmost overlay, and focus restoration works when the overlay is drawn before the widget that held focus.
- `Toast` no longer justifies wrapped title and description text.

## [0.14.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.14.0) — 2026-07-13

- **Breaking:** targets egui 0.35, so your app must move to 0.35 as well.
- Added the `IdSalt` trait, blanket-implemented over every `Hash + Debug` type, so the id-salt bound lives in one place. Additive: existing id-salt arguments still compile.

## [0.13.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.13.0) — 2026-06-18

- Added `TextInput::revealable(bool)`: a masked field with a trailing eye toggle, operable by mouse, keyboard, and screen reader. Implies masking, so `.password(true)` is not also needed.

## [0.12.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.12.0) — 2026-06-17

- Added 23 Lucide glyphs to the bundled Elegance Symbols font (U+E00C–U+E022) covering actions, navigation, status, and editing. All are exposed as `glyphs` constants.
- `Callout`'s per-tone icons now use a uniform Lucide set, replacing base-font marks that sat on a different baseline and weight.

## [0.11.2](https://github.com/stephenberry/egui-elegance/releases/tag/v0.11.2) — 2026-06-17

- Fixed filled sub-frames painting square corners over rounded cards: the modal footer fill and accordion row highlight/focus ring both squared off the card they sat in ([#7](https://github.com/stephenberry/egui-elegance/issues/7)).
- Fixed `Modal` focus restoration for callers that drop the modal the instant it closes.
- A disabled `Select` now reports as disabled to screen readers.
- Added `Modal::closable(bool)` and `Select::enabled(bool)`.

## 0.11.1 — 2026-06-01

- Added the `trash` and `pencil` glyphs to the Symbols font.

## [0.11.0](https://github.com/stephenberry/egui-elegance/releases/tag/v0.11.0) — 2026-05-30

- **Breaking:** `Palette::sky` renamed to `Palette::focus`. It drives every structural focus, active, and selection treatment, and the old name collided with the unrelated `Accent::Sky` button variant. Migration: replace `palette.sky` with `palette.focus`. The `Accent::Sky`, `AvatarTone::Sky`, and `SegmentDot::Sky` variants are unchanged.
- Documented the two accent roles: the structural `palette.focus` field versus the semantic `Accent` enum, with a worked re-theming example.
