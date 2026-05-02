# Crucible

Crucible is a Rust-first 3D game engine project focused on high-throughput rendering, modular systems, and an editor that stays fast under heavy scenes.

## Stack

- `wgpu` for the GPU backend. It is Rust-native, safe, cross-platform, and targets Vulkan, Metal, DirectX 12, OpenGL, WebGPU, and WebGL2.
- `winit` for native window/event-loop integration.
- `crucible-core` for engine lifecycle, frame timing, and module boundaries.
- `crucible-render` for the renderer abstraction and surface management.
- `crucible-scripting` for a Rust-native scripting host foundation.
- `crucible-editor` for the initial desktop editor/runtime shell.

GPUI remains a good candidate for future editor UI work because it is the Rust UI framework behind Zed, but it is not used as the 3D renderer. The engine render path starts directly on `wgpu` so game rendering is not tied to an editor UI toolkit.

## Commands

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p crucible-editor
```

## Layout

```text
apps/
  crucible-editor/       Native editor/runtime shell.
crates/
  crucible-core/         Engine lifecycle, modules, timing.
  crucible-render/       wgpu renderer backend.
  crucible-scripting/    Rust-native script host contracts.
docs/
  architecture.md        Engine direction and module boundaries.
```
