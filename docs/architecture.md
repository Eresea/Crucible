# Crucible Architecture

## Direction

Crucible should feel lightweight to build with, but its runtime should be biased toward explicit data flow, predictable scheduling, and low frame overhead. The current setup keeps the core engine independent from rendering, scripting, and editor code so each layer can be optimized or replaced independently.

## Crates

- `crucible-core`: owns the engine lifecycle, frame clock, module registration, shutdown requests, and shared frame context.
- `crucible-render`: owns GPU device/surface setup and frame submission through `wgpu`.
- `crucible-scripting`: starts with Rust-native scripts so gameplay behavior can be ergonomic without embedding a slower language runtime too early.
- `crucible-ui`: owns editor state plus GPUI Component panel composition, dock persistence, asset browser state, and script workspace wiring.
- `crucible-editor`: hosts the native GPUI application shell and later designer/editor workflows.

## Rendering Choice

The game renderer starts on `wgpu`. GPUI Component is used for editor UI, while `wgpu` remains the lower-level GPU API for game rendering and compute. This keeps the game frame path close to the GPU and avoids coupling runtime rendering to an editor UI abstraction.

The initial renderer already requests a high-performance adapter and configures the surface with a low-latency present-mode preference where the platform supports it. Future renderer milestones should add:

- render graph scheduling
- bindless material/resource tables where supported
- async pipeline and asset preparation
- visibility, culling, and draw-call batching
- GPU timing captures behind a profiling feature

## Editor Direction

The editor is a thin orchestration layer over engine services plus a GPUI Component UI built in `crucible-ui`. The first editor shell uses dockable/resizable panels for viewport, scene outline, inspector, asset manager, and script editor.

Game viewport rendering should continue to be owned by `crucible-render`. The current viewport panel is a GPUI placeholder shell; the next render milestone should embed or bridge the `wgpu` viewport into that dock panel without moving runtime rendering into UI code.

## Scripting Direction

The first scripting layer is a Rust-native trait API. This supports high-performance gameplay modules immediately and keeps ABI/runtime decisions open. Later options:

- hot-reloadable Rust dynamic plugins for power users
- a visual graph layer for designers
- a constrained embedded language only where iteration speed matters more than raw runtime speed
