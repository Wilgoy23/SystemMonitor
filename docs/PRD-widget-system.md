# PRD: Widget Dashboard ("Bubbles")

**Status:** Draft · **Owner:** willg · **Date:** 2026-07-17
**Repo context:** `sysmon` — Rust, egui/eframe 0.28, tabbed `Panel` architecture

---

## 1. Summary

Add an Apple-style widget dashboard to SystemMonitor: a grid of small rounded
"bubble" cards, each showing one glanceable piece of system information (CPU
sparkline, memory ring, network throughput, ...). Users can add, remove,
rearrange, and resize widgets, and the layout persists across runs.

The dashboard becomes the app's home view. The existing full-size panels stay
as the "detail" layer — clicking a widget jumps to its corresponding panel.

Think **macOS Notification Center / iOS home screen widgets**: compact,
rounded-corner cards in fixed size classes on a snapping grid, with an explicit
edit mode for arranging and a gallery for adding new ones.

## 2. Background & current state

Today the app is a tab bar of nine full-screen panels (`System`, `CPU`,
`Memory`, `Disks`, `Network`, `Processes`, `Temps`, `GPU`, `Explorer`), each
implementing the `Panel` trait (`src/panels/mod.rs`): `name()`, `refresh(&SysHandles)`,
`ui(&mut Ui)`. A single `Source` (`src/metrics.rs`) refreshes sysinfo handles on
a user-configurable interval and fans data out to every panel. Tab order,
hidden panels, refresh interval, and active tab persist via eframe storage
(`Settings` in `src/app.rs`). Tabs are already drag-reorderable.

What's missing is a *glanceable* view: to check CPU, memory, and network at
once you must flip between three tabs. A widget dashboard solves that, and
also creates a home for future non-system info (clock, weather, shortcuts).

## 3. Goals

1. **Glanceability** — one screen shows the user's chosen key metrics at once.
2. **Direct manipulation** — drag widgets to rearrange; changes feel physical
   (snap, push-aside reflow, subtle animation).
3. **Composability** — add/remove widgets from a gallery; multiple instances
   of the same widget kind with different configs (e.g. one Disk widget per
   drive, one Network widget per interface).
4. **Persistence** — layout, sizes, and per-widget config survive restarts,
   using the existing eframe storage mechanism.
5. **Extensibility** — adding a new widget kind is one file + one registry
   line, mirroring how `default_panels()` works today. Widget content is not
   limited to system metrics (future: clock, weather, notes).

### Non-goals (v1)

- OS-level desktop widgets (separate always-on-top windows). Tracked as a
  stretch goal via egui multi-viewport — see §10.
- Freeform pixel-perfect placement. Widgets snap to a grid; no overlapping.
- Third-party / scripted widgets (plugin system). Registry is compile-time.
- Multiple named dashboard pages. One dashboard in v1; the model shouldn't
  preclude pages later.

## 4. Users & key scenarios

- **Glance**: user opens the app (or leaves it on a second monitor) and reads
  CPU, RAM, GPU, and network at a glance without any interaction.
- **Curate**: user removes the Temps widget, adds a second Network widget
  pinned to their Ethernet interface, and drags GPU next to CPU.
- **Drill in**: user sees a CPU spike on the widget, clicks it, and lands on
  the full CPU panel with per-core detail.
- **Recover**: user messes up their layout and hits "Reset layout" to get the
  sensible default back.

## 5. UX specification

### 5.1 Placement in the app

- A new **Dashboard** entry appears first in the tab bar and is the default
  `active_tab` for fresh installs. Existing users keep their saved active tab.
- The dashboard fills the central panel with a vertically scrollable widget
  grid. The existing controls bar (pause / refresh interval) applies to it
  like any other view.

### 5.2 Grid & size classes

- The canvas is a uniform grid of square-ish cells (~150 px at 1.0 scale,
  responsive: column count = floor(available width / cell width), min 2).
- Widgets come in Apple-style size classes, expressed in cells:
  - **Small** — 1×1 (single stat + tiny sparkline or ring)
  - **Medium** — 2×1 (stat + chart, or two stats)
  - **Large** — 2×2 (chart + breakdown list)
  - (Wide 4×1 / XL 4×2 reserved for later; the layout model supports any
    w×h so this is only a catalog decision.)
- Layout is **flow-packed**: widgets have a user-defined order and are packed
  left-to-right, top-to-bottom into the column grid (skyline packing). This is
  how Apple does it — you order widgets, you don't pin exact coordinates — and
  it makes window resizing trivial (reflow, order preserved).
- Every widget renders as a rounded-rect card ("bubble"): corner radius ~16 px,
  subtle fill from theme visuals, 1 px stroke, consistent inner padding, title
  row (small icon + name, weak text) above the content area.

### 5.3 Rearranging (drag & drop)

- **Drag anywhere on a widget** to pick it up (same click-and-drag single
  widget technique already proven in the tab bar — see the comment in
  `src/app.rs`; do not layer a drag widget over a click widget).
- While dragging: the card lifts (slight scale/shadow), other widgets show a
  drop indicator at the insertion point; on drop the order list updates and
  the grid reflows. Positions animate to their new spots
  (`ctx.animate_value_with_time`).
- Click without drag = open the widget's linked panel (if any).

### 5.4 Edit mode

- Toggled by an **Edit** button on the dashboard (and Esc / "Done" to leave).
- In edit mode each bubble shows:
  - **✕ remove** badge (top-left, Apple-style) — removes instantly; a toast
    with **Undo** appears for ~5 s (no confirmation dialog).
  - **Size toggle** (bottom-right) — cycles the widget through the size
    classes its kind supports.
  - **⚙ configure** (top-right, only if the kind has options) — opens a small
    popup: e.g. Disk widget → which mount; Network widget → which interface;
    chart widgets → history length.
- Normal mode is deliberately chrome-free: no badges, no jiggle, just data.
  (Optional subtle wiggle animation in edit mode is a polish item, not a
  requirement.)

### 5.5 Adding widgets — the gallery

- **＋ Add widget** button on the dashboard (always visible, not only in edit
  mode) opens the **widget gallery**: a modal/window listing every registered
  widget kind with name, description, and a live-ish preview rendered at
  Small size using current data.
- Choosing a kind adds an instance (default size, default config) at the end
  of the layout and enters edit mode so the user can place it.
- Kinds that support multiple instances (Disk, Network) can be added
  repeatedly; single-instance kinds (e.g. Uptime) show as "already added".

### 5.6 Usability details

- **Reset layout** action (in the gallery footer or a dashboard ⋯ menu)
  restores the default widget set and order.
- **Keyboard**: in edit mode, arrow keys move the focused widget within the
  order; Delete removes it. (Accessibility baseline; full a11y later.)
- **Empty state**: if all widgets are removed, show a friendly hint + Add
  button in the center.
- Widget content must degrade gracefully when data is unavailable (no NVIDIA
  GPU, no temp sensors): the widget shows an inline "n/a" state, and its
  gallery entry is marked unavailable rather than hidden.
- Respect the global pause and refresh interval exactly as panels do.

## 6. Initial widget catalog (v1)

| Widget | Sizes | Content | Config | Links to panel |
|---|---|---|---|---|
| CPU | S, M, L | usage % + sparkline; L adds per-core bars | history length | CPU |
| Memory | S, M | used/total ring (S), + swap and sparkline (M) | — | Memory |
| GPU | S, M | utilization + VRAM; M adds temp/power | GPU index | GPU |
| Network | S, M | ↑/↓ rates; M adds sparkline | interface (default: aggregate) | Network |
| Disk | S, M | usage bar for one mount; M adds R/W rates | mount point | Disks |
| Temps | S, M | hottest sensor (S); M lists top sensors | sensor filter | Temps |
| Top processes | M, L | top 3 (M) / top 8 (L) by CPU or memory | sort key | Processes |
| Uptime / system | S, M | uptime (S); M adds host, OS, kernel | — | System |
| Clock | S, M | time + date — first non-system widget, proves generality | 12/24 h | — |

Each row reuses the existing per-panel data extraction (`History`,
`charts.rs` helpers) rather than re-querying sysinfo.

## 7. Technical design

### 7.1 New module layout

```
src/
  dashboard/
    mod.rs        // Dashboard view: grid layout, edit mode, gallery, dnd
    layout.rs     // flow packing: Vec<WidgetInstance> + columns -> rects
    registry.rs   // WidgetKind registry (id, name, desc, sizes, factory)
  widgets/
    mod.rs        // Widget trait + shared card frame helper
    cpu.rs, memory.rs, gpu.rs, network.rs, disk.rs,
    temps.rs, processes.rs, system.rs, clock.rs
```

### 7.2 Core trait

```rust
/// One live widget instance on the dashboard. Mirrors `Panel` but renders
/// into a fixed-size card and carries per-instance config.
pub trait Widget {
    /// Stable kind id, e.g. "cpu" — used for (de)serialization.
    fn kind(&self) -> &'static str;
    /// Title shown in the card header.
    fn title(&self) -> String;               // may include config, e.g. "Disk C:"
    /// Size classes this kind supports (first = default).
    fn supported_sizes(&self) -> &'static [WidgetSize];
    /// Pull data on the shared tick — same contract as Panel::refresh.
    fn refresh(&mut self, h: &SysHandles);
    /// Render into a card body of the given size. Must not exceed the rect.
    fn ui(&mut self, ui: &mut egui::Ui, size: WidgetSize);
    /// Panel name to open on click, if any.
    fn linked_panel(&self) -> Option<&'static str> { None }
    /// Serialize per-instance config (interface name, mount, ...).
    fn config(&self) -> serde_json::Value { serde_json::Value::Null }
    fn set_config(&mut self, v: &serde_json::Value) {}
    /// Optional config UI shown from the ⚙ badge in edit mode.
    fn config_ui(&mut self, ui: &mut egui::Ui) -> bool { false } // true = changed
}
```

`WidgetSize` is an enum `{ Small, Medium, Large }` with `fn cells() -> (u8, u8)`.

### 7.3 Registry

`registry.rs` holds a static table of `WidgetKindInfo { id, name, description,
factory: fn() -> Box<dyn Widget>, multi_instance: bool }` — the widget-world
analog of `default_panels()`. The gallery iterates it; deserialization looks
up factories by `id` and skips unknown ids (forward compatibility).

### 7.4 Persistence

Extend `Settings` (kept in the same eframe storage blob, `#[serde(default)]`
already protects old installs):

```rust
struct WidgetEntry {
    kind: String,             // registry id
    id: u64,                  // instance id, unique within layout
    size: WidgetSize,
    config: serde_json::Value,
}
struct Settings {
    // ...existing fields...
    widgets: Vec<WidgetEntry>,   // dashboard order == vec order
    dashboard_edit_hint_seen: bool,
}
```

On startup: if `widgets` is empty (fresh install), populate the default set
(CPU-S, Memory-S, GPU-S, Network-M, Disk-S per first mount, Uptime-S). New
dependency: `serde_json` (tiny, already transitively present via other crates'
ecosystems; add explicitly to `Cargo.toml`).

### 7.5 Refresh & data flow

No change to `Source`. In `App::update`, the same tick that refreshes panels
also refreshes widget instances (`for w in &mut widgets { w.refresh(&handles) }`).
Widgets keep their own `History` buffers like panels do. To avoid duplicated
sampling cost, widgets read only from `SysHandles` — same rule as panels.
(Deduplicating history buffers between a panel and its widget is a later
optimization; the buffers are 60 f64s, so duplication is fine in v1.)

### 7.6 Layout & drag-and-drop mechanics

- `layout.rs` computes, per frame: given available width → column count →
  a `Vec<Rect>` per widget via skyline packing over the order list. Pure
  function; unit-testable without egui.
- Each card is one `ui.allocate_rect` + `Sense::click_and_drag()` interaction
  (single widget for click+drag, per the tab-bar lesson). Drag payload is the
  instance `id` via `dnd_set_drag_payload`, matching the existing tab code.
- Drop target = nearest insertion index from pointer position; show an
  indicator; on release, reorder `settings.widgets`.
- Position animation: animate each card's rect toward its target with
  `ctx.animate_value_with_time` keyed by instance id. During active drag,
  request continuous repaint; otherwise keep the existing
  `request_repaint_after(refresh_ms)` economy.

### 7.7 Risks & constraints

- **egui 0.28 dnd quirks** — already navigated once in the tab bar; reuse the
  exact same pattern. Biggest risk is drop-index math during reflow; mitigate
  by unit-testing `layout.rs` and keeping dnd state minimal.
- **Small-size legibility** — 1×1 cards fit ~2 lines + a tiny chart. Catalog
  entries must be designed per size, not shrunk panels. Mitigation: shared
  card-frame helper enforces padding/typography; review each widget at 1.0
  and 1.5 UI scale.
- **Perf** — 10–15 widgets each drawing a small plot per repaint is well
  within egui's budget at 1.5 s refresh; keep the pause/interval behavior and
  avoid per-frame allocation in `layout.rs` hot path.
- **Settings migration** — additive fields + `#[serde(default)]` means old
  blobs load cleanly; unknown widget kinds in future downgrades are skipped.

## 8. Milestones

**M1 — Static dashboard (foundation)**
Widget trait, registry, card frame, `layout.rs` packing + tests, Dashboard
tab rendering the default widget set (CPU, Memory, Network, Uptime) at fixed
sizes. No editing yet.
*Accept:* fresh install opens on a dashboard showing live data; window resize
reflows; pause/interval respected.

**M2 — Arrange & persist**
Drag-to-reorder with drop indicator and animation; click-through to linked
panel; layout order/size persisted and restored.
*Accept:* rearrange, restart, layout identical; click CPU widget lands on CPU
panel; drag never triggers a click.

**M3 — Add, remove, edit mode**
Edit mode with ✕ remove + undo toast; widget gallery with previews; size
toggle; reset layout; empty state; remaining catalog widgets (GPU, Disk,
Temps, Processes) including n/a states.
*Accept:* full add→arrange→remove→undo→reset loop works; unavailable
hardware shows "n/a", never a panic.

**M4 — Per-widget config & multi-instance**
Config popup (⚙), `config`/`set_config` persistence, multi-instance kinds
(two Network widgets on different interfaces; Disk per mount); Clock widget
as the first general-info widget.
*Accept:* two differently-configured instances of one kind survive restart.

**M5 — Polish**
Animations tuning, keyboard controls in edit mode, first-run hint, visual
pass on both themes, 1.5× scale audit.

## 9. Success criteria

- Default view answers "how's my system doing" with **zero clicks**.
- Add + place a new widget in **≤ 3 interactions**.
- Layout persistence is 100 % reliable across restarts and window sizes.
- No regression to existing panels, tab reordering, or settings.
- Adding a new widget kind touches ≤ 2 files (its module + registry line).

## 10. Future directions (explicitly out of v1 scope)

- **Floating bubbles**: pop a widget out into a small always-on-top OS window
  via egui multi-viewport (supported since 0.27+) — the closest thing to true
  Apple desktop widgets and a natural M6.
- Multiple dashboard pages / profiles (the `Vec<WidgetEntry>` model extends
  to `Vec<Page>` cleanly).
- General-info widgets beyond Clock: weather, calendar, shortcuts, notes.
- Alert thresholds (widget turns red above X% and can notify).
- Plugin/scripted widgets (would require sandboxing decisions — far future).
