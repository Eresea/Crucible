# Crucible Architecture

## Direction

Crucible should feel lightweight to build with, but its runtime should be biased toward explicit data flow, predictable scheduling, and low frame overhead. The current setup keeps the core engine independent from rendering, scripting, and editor code so each layer can be optimized or replaced independently.

## Crates

- `crucible-core`: owns the engine lifecycle, frame clock, module registration, shutdown requests, and shared frame context.
- `crucible-render`: owns GPU device/surface setup and frame submission through `wgpu`.
- `crucible-scripting`: starts with Rust-native scripts so gameplay behavior can be ergonomic without embedding a slower language runtime too early.
- `crucible-editor`: hosts the native app shell, window loop, renderer startup, and later designer/editor workflows.

## Rendering Choice

The renderer starts on `wgpu` rather than GPUI. GPUI is designed for fast application UI, while `wgpu` is a lower-level GPU API suited to game rendering and compute. This keeps the game frame path close to the GPU and avoids coupling engine rendering to an editor UI abstraction.

The initial renderer already requests a high-performance adapter and configures the surface with a low-latency present-mode preference where the platform supports it. Future renderer milestones should add:

- render graph scheduling
- bindless material/resource tables where supported
- async pipeline and asset preparation
- visibility, culling, and draw-call batching
- GPU timing captures behind a profiling feature

## Editor Direction

The editor should be a thin orchestration layer over engine services. GPUI can be evaluated for panels, inspectors, asset browser, scene hierarchy, and command palette style workflows. Game viewport rendering should continue to be owned by `crucible-render`.

## Scripting Direction

The first scripting layer is a Rust-native trait API. This supports high-performance gameplay modules immediately and keeps ABI/runtime decisions open. Later options:

- hot-reloadable Rust dynamic plugins for power users
- a visual graph layer for designers
- a constrained embedded language only where iteration speed matters more than raw runtime speed
