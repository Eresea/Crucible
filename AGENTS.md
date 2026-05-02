# AGENTS

This file defines working rules for coding agents contributing to Crucible.

## Project Goal

Crucible is a Rust-first 3D game engine focused on performance, modularity, and a fast editor experience. Runtime rendering should stay close to the GPU. Editor and designer tooling should be productive without adding overhead to game execution.

## Working Rules

- Prefer small, composable crates over large shared modules.
- Keep engine runtime code independent from editor UI code.
- Keep the render path explicit, measurable, and allocation-aware.
- Favor Rust-native APIs before embedding external runtimes.
- Avoid broad refactors unless they are required for the task.
- Keep public APIs narrow until repeated use proves the shape.
- Do not introduce `unsafe` without a clear performance or FFI reason and a comment explaining the invariant.

## Workspace Layout

- `apps/crucible-editor`: native editor/runtime shell.
- `crates/crucible-core`: lifecycle, modules, frame timing, shared engine contracts.
- `crates/crucible-render`: GPU backend and rendering abstractions.
- `crates/crucible-ui`: GPUI Component editor shell, docked panels, editor state, asset browser state, script workspace wiring.
- `crates/crucible-scripting`: gameplay scripting contracts and native script host.
- `docs`: architecture notes and long-form design decisions.

## Commands

Run these before handing off engine changes:

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
```

Use the editor shell for manual rendering checks:

```powershell
cargo run -p crucible-editor
```

## Rendering

`wgpu` is the primary game GPU backend. Keep game rendering owned by `crucible-render`; GPUI Component is for editor chrome, panels, inputs, and designer tooling.

Renderer changes should preserve:

- high-performance adapter selection by default
- clear surface lifecycle handling
- explicit resize and frame acquisition paths
- separation between GPU resource setup and frame submission

## Editor

The editor should feel immediate, quiet, and efficient. Build tools for repeated professional use: scene hierarchy, inspector, asset browser, script editor, command palette, logs, profiler, and viewport controls should prioritize scanability over decoration.

Editor UI work should start in `crucible-ui` unless it is specifically native-window setup in `crucible-editor`. Prefer `gpui-component` primitives for dock panels, controls, inputs, and code-editing surfaces before building custom widgets.

Follow `DESIGN.md` for visual direction.

## Documentation

Update `README.md` for commands or project structure changes. Update `docs/architecture.md` when boundaries, renderer strategy, scripting strategy, or editor strategy change.
