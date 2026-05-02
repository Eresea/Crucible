# Crucible

Crucible is a Rust-first 3D game engine project focused on high-throughput rendering, modular systems, and an editor that stays fast under heavy scenes.

## Stack

- `wgpu` for the GPU backend. It is Rust-native, safe, cross-platform, and targets Vulkan, Metal, DirectX 12, OpenGL, WebGPU, and WebGL2.
- `winit` for native window/event-loop integration.
- `crucible-core` for engine lifecycle, frame timing, and module boundaries.
- `crucible-render` for the renderer abstraction and surface management.
- `gpui` and `gpui-component` for the editor UI shell, dock layout, panel controls, text input, and code editor foundation.
- `crucible-ui` for editor state, GPUI Component panel composition, asset browser state, and script workspace wiring.
- `crucible-scripting` for a Rust-native scripting host foundation.
- `crucible-editor` for the initial desktop editor/runtime shell.

GPUI Component is used for the editor interface because it already provides performant desktop UI primitives, dock panels, inputs, and code editing on top of GPUI. The engine render path still starts directly on `wgpu` so game rendering is not tied to an editor UI toolkit.

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
  crucible-ui/           GPUI Component editor shell and panel state.
docs/
  architecture.md        Engine direction and module boundaries.
```
