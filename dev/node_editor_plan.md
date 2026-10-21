# Node Editor — Design Plan

A new `NodeEditor` widget for `egui-elegance`: a pannable/zoomable graph canvas with rounded, accent-tinted nodes and bezier connections, designed to feel like a native member of the elegance widget family rather than a bolt-on.

This document is a *proposal*. The **Open Questions** section at the end lists decisions that need your input before implementation begins; the rest assumes the most common choice and notes alternatives.

---

## 1. Goals

- **Visually native to elegance.** Nodes read as small Cards: the same `card_radius`, `card` fill, `border` stroke, accent-tinted heading bar, and sky focus ring on selection. Connections use the sky / palette accents so the graph looks like the rest of a slate or paper app, not a foreign embed.
- **One-import experience.** `use elegance::{NodeEditor, NodeId, ...}`, no extra crate, no separate theme.
- **Owned state, like `LogBar`.** Construct once, mutate from anywhere with `&mut self`, render with `editor.show(ui)`. Serialisable so apps can persist graphs without bridging through egui memory.
- **Generic over node payload.** Whatever the host wants stored per node (an enum, a trait object, a string id) is parameterised, the way `Select<T>` is generic over the bound value.
- **Composable interactions.** Pan, zoom, drag-to-move, drag-from-port-to-connect, marquee-select, delete — all out of the box, all themeable, all overrideable.
- **Accessible.** Each node and port emits `WidgetInfo`; keyboard navigation between nodes is possible (arrow keys / Tab); focus ring matches the rest of elegance.

## 2. Non-Goals

- **Not a graph engine.** No topological sort, dependency resolution, evaluation, or cycle detection in the core widget. Those belong in user code; the editor is the *view*.
- **Not a competitor to `egui_node_graph` / `egui-snarl` for every use case.** Those expose richer trait machinery for full visual scripting systems. Elegance's editor is a **first-class polished default**, not an extension framework.
- **Not a layout engine.** Auto-layout (e.g. Sugiyama) is out of scope for v1; node positions are user-owned and persisted.
- **No undo/redo built in.** Encouraged at the app layer (where the domain model lives); the editor exposes the events needed to record history.

## 3. Visual Design

Concrete commitments so the editor reads as elegance the moment it appears:

### 3.1 Node

```
┌─────────────────────────────────┐  ← card_radius, 1px palette.border
│ ▍ Heading                  • • •│  ← accent-tinted heading bar
├─────────────────────────────────┤
│ ◉ in_a              out ◉       │  ← ports (left = in, right = out)
│ ◉ in_b              gain ◉      │
│                                 │
│ ─────── body content ────────── │  ← optional widgets (Sliders, etc.)
│                                 │
└─────────────────────────────────┘
```

- **Frame:** `palette.card` fill, 1px `palette.border` stroke, `card_radius` corners. Selected: 2px `palette.sky` stroke, same fill.
- **Heading bar:** colored top strip (`Accent`-resolved fill, ~24pt tall) with bold heading text in `Color32::WHITE` (or `palette.text` on light themes — selected automatically per accent like `Button` does). Optional small icon left, optional menu trigger right.
- **Drop shadow:** Subtle, theme-aware. On dark palettes, a soft black shadow at low alpha; on light, a slightly stronger one. Matches the visual weight of `Modal`.
- **Hover:** Border subtly brightens to `text_muted` — same convention as `themed_input_visuals`.
- **Drag preview:** Slight `expansion` (1–2 px) and a sharper shadow.

### 3.2 Ports

- **Resting input:** outline circle, 8px diameter, `palette.border` stroke.
- **Resting output:** filled circle, 8px diameter, accent-tinted (port color or node accent).
- **Connected:** input fills with the same colour as its incoming edge.
- **Hover (any state):** halo ring grows ~2 px, sky highlight ring at ~40 alpha.
- **Drag-from:** the source port stays bright; cursor follows a dashed bezier to the pointer. Hovering a compatible target highlights it green; an incompatible target highlights red (similar to `MenuItem::danger`).
- **Type colours (if typed):** Map to the existing `Accent` palette so the user gets six obvious type slots without us inventing new colours.

### 3.3 Connections (Edges)

- **Default:** Cubic bezier from output to input, 2 px stroke. Control points horizontally offset by `0.5 * dx` so the curve flows naturally left-to-right (matching Blueprint, Blender, ComfyUI, n8n).
- **Color:** Output port colour at the source side, input port colour at the destination side, gradient blended along the curve. (If port colours are equal, just one colour.)
- **Hover:** stroke widens to 3 px, sky-tinted halo behind it.
- **Selected:** sky stroke at 3 px with a faint sky glow (same idea as the focus ring on `Button`).
- **Hit-testing:** generous — sample the curve at ~24 segments and reject if cursor distance > 6 px from any segment. Fast and forgiving.

### 3.4 Canvas

- **Background:** `palette.bg`.
- **Grid:** Optional dot grid at fixed world-space spacing (default 24 pt). Dot colour is `palette.depth_tint(palette.bg, 0.06)` — almost invisible but enough to give a sense of motion when panning.
- **At deep zoom-out:** grid fades to nothing to avoid moiré.
- **Selection marquee:** sky-tinted translucent rectangle with a 1 px sky border — same style as text-edit selection (`v.selection`).

### 3.5 Toolbar / Overlay

A small floating control cluster bottom-right by default:

```
┌─────────────┐
│  − 100% +  ⟲│   ← zoom out / level / zoom in / reset
└─────────────┘
```

Renders as a small rounded `Card`-style strip, semi-transparent (`with_alpha(p.card, 220)`). Hidden via `.toolbar(false)` if the host wants to roll its own.

### 3.6 Optional Minimap

A scaled-down view of the world bounded box, 160×100 in the bottom-left corner. Hidden by default; opt in with `.minimap(true)`.

## 4. Public API

Following the **owned-state, render-once** pattern of `LogBar`. The editor is generic over a payload type `N` for node data and an `E` for edge data (defaulting to `()`).

```rust
use elegance::{NodeEditor, NodeId, PortId, NodeStyle, Accent};

struct App {
    editor: NodeEditor<MyNodeData>,
}

impl App {
    fn new() -> Self {
        let mut editor = NodeEditor::<MyNodeData>::new("graph");
        let a = editor.add_node(
            NodeData::filter(),
            egui::pos2(80.0, 60.0),
        );
        let b = editor.add_node(
            NodeData::sink(),
            egui::pos2(360.0, 80.0),
        );
        editor.connect((a, "out"), (b, "in"));
        Self { editor }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        Theme::slate().install(ui.ctx());

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.editor
                .show(ui, |ui, ctx| {
                    // Per-frame node descriptor callback.
                    ctx.node(|node| {
                        node.heading(node.data().title())
                            .accent(node.data().accent())
                            .input("in", PortKind::Audio)
                            .input("gain", PortKind::Control)
                            .output("out", PortKind::Audio)
                            .body(|ui| {
                                ui.add(Slider::new(&mut node.data_mut().gain, 0.0..=1.0));
                            });
                    });
                });

            // Drain events emitted this frame.
            for ev in self.editor.events() {
                match ev {
                    NodeEvent::Connected { from, to } => { /* ... */ }
                    NodeEvent::Disconnected { edge } => { /* ... */ }
                    NodeEvent::NodeMoved { node, .. } => { /* ... */ }
                    NodeEvent::NodeRemoved { node } => { /* ... */ }
                    NodeEvent::SelectionChanged => { /* ... */ }
                }
            }
        });
    }
}
```

**Note:** the closure-based per-node descriptor above is one option — see Open Question Q3 for the alternative (trait-based) shape.

### 4.1 Core Types (sketch)

```rust
pub struct NodeEditor<N, E = ()> { /* ... */ }

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct NodeId(u64);
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct EdgeId(u64);

#[derive(Clone)]
pub struct PortRef {
    pub node: NodeId,
    pub port: PortId,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum PortId {
    Named(&'static str),
    Index(u16),
}

pub enum NodeEvent {
    NodeAdded { node: NodeId },
    NodeRemoved { node: NodeId },
    NodeMoved { node: NodeId, from: Pos2, to: Pos2 },
    Connected { edge: EdgeId, from: PortRef, to: PortRef },
    Disconnected { edge: EdgeId },
    SelectionChanged,
    Clicked { node: NodeId, modifiers: Modifiers },
    DoubleClicked { node: NodeId },
    ContextMenu { at: Pos2, target: ContextTarget },
}
```

### 4.2 Builder Methods

```rust
impl<N, E> NodeEditor<N, E> {
    pub fn new(id_salt: impl Hash) -> Self;

    // Visual options
    pub fn show_grid(self, show: bool) -> Self;
    pub fn show_toolbar(self, show: bool) -> Self;
    pub fn show_minimap(self, show: bool) -> Self;
    pub fn min_zoom(self, z: f32) -> Self;
    pub fn max_zoom(self, z: f32) -> Self;
    pub fn snap_to_grid(self, snap: Option<f32>) -> Self;

    // Connection style
    pub fn connection_style(self, s: ConnectionStyle) -> Self;
    // ConnectionStyle::Bezier (default) / Straight / Orthogonal

    // Behaviour toggles
    pub fn allow_self_loops(self, allow: bool) -> Self;
    pub fn allow_multiple_edges(self, allow: bool) -> Self;

    // Mutation API (state lives on the editor)
    pub fn add_node(&mut self, data: N, pos: Pos2) -> NodeId;
    pub fn remove_node(&mut self, id: NodeId);
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &N)>;
    pub fn nodes_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut N)>;
    pub fn connect(&mut self, from: impl Into<PortRef>, to: impl Into<PortRef>)
        -> Result<EdgeId, ConnectError>;
    pub fn disconnect(&mut self, edge: EdgeId);
    pub fn edges(&self) -> impl Iterator<Item = (EdgeId, &Edge<E>)>;

    pub fn selection(&self) -> &Selection;
    pub fn select(&mut self, sel: Selection);

    pub fn view(&self) -> &Viewport;          // pan + zoom
    pub fn view_mut(&mut self) -> &mut Viewport;
    pub fn fit_to_content(&mut self);

    pub fn events(&mut self) -> impl Iterator<Item = NodeEvent>;

    // Render
    pub fn show(&mut self, ui: &mut Ui, body: impl FnOnce(&mut Ui, &mut NodeContext<'_, N, E>));
}
```

### 4.3 Public Sub-Builders

- `NodeContext` — handed to the body closure each frame; lets the user describe how each node renders.
- `NodeBuilder` (or trait) — per-node configuration: heading text, accent, input/output port lists, body closure.
- `PortStyle` — colour, shape (circle / square / triangle), label position, type tag for connection validation.

## 5. Architecture

### 5.1 Module Layout

```
src/node_editor/
  mod.rs          // re-exports + NodeEditor entrypoint
  graph.rs        // NodeId, EdgeId, Graph<N, E> storage + invariants
  viewport.rs     // pan, zoom, world-vs-screen transforms
  interaction.rs  // input state machine (idle, panning, dragging,
                  //   connecting, marquee)
  paint/
    grid.rs       // background dots
    node.rs       // node frame + heading + ports
    edge.rs       // bezier curves + hit-test
    minimap.rs    // optional overview
    toolbar.rs    // zoom/reset chip
  events.rs       // NodeEvent enum + emission helper
  hit.rs          // pointer hit-tests across nodes/ports/edges
```

### 5.2 Coordinate Spaces

Two clearly separated spaces:

- **World space** — where nodes live, in points, persisted with the graph.
- **Screen space** — egui `Pos2` inside the canvas rect.

`Viewport { pan: Vec2, zoom: f32 }` provides `world_to_screen` / `screen_to_world`. Hit-testing happens in world space; painting converts at the last moment.

### 5.3 Interaction State Machine

A single enum, owned by the editor:

```rust
enum Interaction {
    Idle,
    Panning { from: Pos2 },
    DraggingNodes { offset_per_node: Vec<(NodeId, Vec2)> },
    Connecting { source: PortRef, cursor: Pos2 },
    Marquee { from: Pos2, to: Pos2 },
}
```

Transitions are driven from one place per frame, which makes behaviour easy to predict and to test.

### 5.4 Storage

`Graph<N, E>` uses `slotmap`-style id allocation but **without** the `slotmap` crate dependency unless we deem it worth it — a `Vec<Slot<T>>` with a free-list and a generation counter is small enough to inline, and avoids a public dependency leak. Open Question Q11.

### 5.5 Per-frame Cost

Targets:

- O(visible nodes + visible edges) per frame.
- Edge bezier sampling cached per edge while endpoints are stable (recomputed only when an endpoint moves or zoom changes).
- Hit-test uses a coarse AABB pre-filter on edges before the per-segment distance check.

### 5.6 Theme Integration

All colours via `Theme::current(ui.ctx())`. The editor adds **no new palette entries** in v1 — every visual derives from the existing palette and the existing `Accent` colours. This keeps custom palettes working without modification.

If a user wants to recolour just the editor (e.g. graph nodes use different accents than buttons), `NodeStyle` is the override knob:

```rust
let style = NodeStyle::default()
    .grid_visible(false)
    .edge_color_default(theme.palette.purple);
editor.style(style);
```

## 6. Interactions (default bindings)

| Action | Binding |
|---|---|
| Pan canvas | Middle-drag, or Space + left-drag, or two-finger drag |
| Zoom | Scroll wheel (cursor anchor), pinch on trackpad |
| Select node | Left click |
| Add to selection | Shift / Cmd + click |
| Marquee select | Left-drag on empty canvas |
| Move node(s) | Left-drag on selected node |
| Connect | Left-drag from output port to input port |
| Cancel pending connection | Esc, or release on empty canvas |
| Disconnect edge | Right-click edge → Disconnect, or select + Delete |
| Delete selection | Delete / Backspace |
| Frame all | `F` (or via toolbar reset) |
| Zoom 100% | `0` (or via toolbar) |
| Context menu | Right click — surfaces an elegance `Menu` populated by host |

## 7. Theming Hooks

Two layers:

1. **Theme tokens** (no API needed) — the editor reads the same `palette` and `Accent` values as everything else.
2. **NodeStyle** — per-editor overrides for grid visibility, edge style (Bezier / Straight / Orthogonal), edge thickness, port shape, drop shadow opacity. This is the escape hatch for users who want their graph editor to feel distinct from the rest of the app.

## 8. Accessibility

- Each rendered node calls `widget_info` with a labelled `Role::Unknown` (the heading text becomes the label).
- Each port emits its own `WidgetInfo` so screen readers can describe port directions.
- Tab cycles between nodes (sorted by world position, top-to-bottom then left-to-right).
- Arrow keys move the focused node by 8 px (or 1 px with Shift) when no input field is focused.
- Selected edge announced with "edge from {source-node}.{source-port} to {dest-node}.{dest-port}".
- Toolbar buttons reuse `Button` so they inherit its accessibility.

## 9. Persistence

`NodeEditor` and its `Graph<N, E>` derive `serde::Serialize` / `Deserialize` **gated behind a `serde` feature flag**. The viewport state (pan/zoom) is included by default; users who want positions but not view state can serialize `editor.graph()` directly.

```toml
[dependencies]
egui-elegance = { version = "0.2", features = ["serde"] }
```

Backwards-compatible by versioning a small header struct in the serialized form. Open Question Q9.

## 10. Testing

Same approach as the rest of the crate:

- **Visual regression:** `tests/visual.rs` adds a "node editor" tile with three nodes and two connections, rendered against all four palettes. PNG diffs catch drift.
- **Unit tests:** Pure-logic tests on `Graph`, `Viewport`, hit-testing, and the connection-validation rules — no egui needed.
- **Headless interaction tests:** `egui_kittest` Harness drives click/drag sequences and asserts on `editor.events()`. Same harness pattern as `cargo render-docs`.

## 11. Examples & Documentation

- **`examples/node_editor.rs`** — Minimal "audio routing" demo: source → filter → sink, with sliders inside each node body. Fits on one screen, exercises the full API.
- **A second `examples/` showcase** *(optional)* — Visual scripting flavour with conditional nodes and typed ports, to demonstrate the `Accent`-coded type system.
- **README section** — A new entry between `LogBar` and "Submit-flash feedback", with a screenshot in `docs/images/node_editor.png` generated by `examples/render_docs.rs`.

## 12. Implementation Phases

### Phase 1 — Skeleton (1–2 sessions)

- `Graph<N, E>` storage + id allocation
- `Viewport` + world/screen transforms
- Static rendering: nodes, ports, straight-line edges, no interaction
- Visual regression baseline for one palette

### Phase 2 — Core Interaction (2–3 sessions)

- Pan, zoom, single-select, drag-to-move
- Connect via port drag (with in-flight bezier preview)
- Delete key removes selection
- Bezier edge rendering + hit-test
- Toolbar (zoom in/out/reset)

### Phase 3 — Polish

- Multi-select + marquee
- Snap-to-grid (opt-in)
- Drop shadows, hover lifts
- Optional minimap
- Connection validation hooks
- Right-click context menu integration with `Menu`
- All four palette snapshots

### Phase 4 — Optional Extensions (possibly v0.3)

- Comment / region rectangles for grouping
- Subgraph collapse / expand
- Undo/redo helper (event-sourced)
- Auto-layout (Sugiyama)

---

## Open Questions

**These need your input before Phase 1 starts.** I've put a recommended default next to each, but every one is genuinely up for grabs.

### Q1 · Primary use case

What workflow are you targeting first? The answer changes priorities on typed ports, body widgets, evaluation hooks, and the shape of the demo example.

- (a) Visual scripting / shaders — typed ports critical
- (b) Audio / signal routing — body widgets critical
- (c) Workflow / pipeline (n8n / Zapier flavour) — heading icons + menu triggers critical
- (d) ML / data pipeline (ComfyUI flavour) — body widgets + previews
- (e) Something else

> **Suggested default:** (a) + (b) — covers most node editors and drives a richer baseline.

### Q2 · Typed ports?

Should connections be type-checked at the editor level, or is connection validity entirely the host's call?

- (a) Untyped — anything connects to anything; host validates after the fact via events.
- (b) Typed via `Accent` enum — six built-in slots; compatible iff same accent.
- (c) Typed via host-provided trait — `PortType: PartialEq + Clone` generic parameter; `Accent` is just one possible `PortType`.

> **Suggested default:** (c) — most flexible, costs little, lets users map types to accents themselves.

### Q3 · Per-node descriptor pattern

How should the host tell the editor what each node looks like?

- (a) Closure-based, as in the example above. Inversion of control, feels familiar to egui users (`ui.add`-ish).
- (b) Trait-based: `impl NodeView for MyNodeData { fn heading() …; fn ports() …; fn body(ui) … }`. More boilerplate but more discoverable and easier to derive.
- (c) Both — the trait provides defaults, the closure is the override.

> **Suggested default:** (a) for v1 — closures match the rest of the elegance API surface (`Card::show`, `Modal::show`). Trait can come later as an ergonomic layer.

### Q4 · Body widgets inside a node

Can node bodies host arbitrary egui widgets (sliders, text inputs, plots), or are they display-only with text labels?

- (a) Full widgets — body closure receives a `&mut Ui` clipped to the node's content rect.
- (b) Display-only — body returns a `WidgetText` or a list of rows.
- (c) Both — display by default, opt into a `Ui` via a method.

> **Suggested default:** (a). Restriction would feel limiting in a library that's otherwise generous with composition.
> **Catch:** mixing zoom with embedded widgets is hard — see Q5.

### Q5 · Zoom and embedded widgets

How should embedded widgets behave under canvas zoom?

- (a) Widgets render at native size regardless of zoom; node frames scale but body stays crisp. Simpler, but bodies "pop" out of scale.
- (b) Widgets scale with zoom via `ctx.set_pixels_per_point`-like treatment per node. Visually consistent but expensive and prone to hit-test drift.
- (c) Hybrid — bodies scale down with zoom but cap at 100%; below a threshold, body collapses to a heading-only badge.

> **Suggested default:** (c). Matches what mature editors (Blender, Figma) actually do.

### Q6 · Connection style

- (a) Cubic bezier (default in most editors)
- (b) Straight lines
- (c) Orthogonal (right-angle, like circuit boards)
- (d) All three, switchable via `ConnectionStyle`

> **Suggested default:** (d), with bezier as the default.

### Q7 · Multi-output → multi-input?

Can one output port connect to multiple inputs? Can one input port accept multiple incoming edges?

- (a) Output → many inputs, but each input takes only one edge. (Most common — Unreal Blueprint, Blender shaders, ComfyUI.)
- (b) Both sides allow many.
- (c) Configurable per port (`PortStyle::max_connections`).

> **Suggested default:** (c) with (a) as the default cap.

### Q8 · Minimap & toolbar by default?

- (a) Both visible by default.
- (b) Toolbar yes, minimap no.
- (c) Neither — opt-in.

> **Suggested default:** (b). Toolbar is universally useful; minimap is graph-size-dependent.

### Q9 · Persistence

- (a) Provide `serde` feature, derive on the public types.
- (b) Provide `serde` plus a stable JSON schema we commit to evolve (i.e. file format guarantees).
- (c) No serde — users serialise their own node data; positions live in a `HashMap<NodeId, Pos2>` they manage.

> **Suggested default:** (a). Format-stability promises (b) cost a lot for a 0.x library.

### Q10 · Undo / redo

- (a) Out of scope for the editor; document the events that allow the host to roll their own.
- (b) Built-in linear undo of editor-level mutations only (move, connect, disconnect, add, remove). Doesn't see body-widget changes.
- (c) Built-in event-sourced helper that records `NodeEvent`s and replays them.

> **Suggested default:** (a). Undo of *graph* changes without undo of *node-content* changes is confusing; the host owns the merged history naturally.

### Q11 · Slotmap dependency

- (a) Inline a small slotmap implementation (no new public dep).
- (b) Add `slotmap = "1"` (extra dependency, but battle-tested).

> **Suggested default:** (a). Elegance has zero non-egui runtime deps today and this would be the first.

### Q12 · Naming

- `NodeEditor` (descriptive)
- `Graph` (clashes with petgraph and is too generic)
- `NodeCanvas` (emphasises the canvas surface, downplays interaction)
- `Patchbay` / `Patchwork` (audio flavour, cute, less obvious)

> **Suggested default:** `NodeEditor`.

### Q13 · Demo example

Which demo would you most enjoy building/showcasing? It influences node payload examples and the README screenshot.

- (a) Audio routing (oscillator → filter → output)
- (b) Build pipeline (sources → transforms → sinks)
- (c) Visual scripting (literal → operator → output)
- (d) ML pipeline (load → preprocess → infer → display)

> **Suggested default:** (a) — quick to grok, visually rich, plays well with sliders inside nodes.

### Q14 · Scope of v1

Confirming the cut line for the first ship — is everything in **Phases 1–3** in scope, with Phase 4 deferred?

> **Suggested default:** Yes. Phases 1–3 produce a polished, demo-able widget; Phase 4 are nice-to-haves that can ride a later version without breaking changes.
