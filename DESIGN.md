# DESIGN

Crucible's UI should be minimalist, modern, and performance-focused. The editor should feel like a precision tool: calm, fast, dense enough for serious work, and visually restrained.

## Visual Direction

- Minimalist, not empty.
- Modern, not decorative.
- Quiet by default, expressive only where state or attention requires it.
- Prioritize workspace, viewport, and data over brand treatment.
- Use contrast, spacing, and type weight before using color.

## Layout

- First screen should be the usable editor, not a landing page.
- The viewport is the primary surface and should get the most space.
- Panels should be dockable in spirit: hierarchy, inspector, assets, console, profiler.
- Avoid nested cards. Use panels, splitters, tabs, rows, and toolbars.
- Keep controls compact and aligned to a clear grid.
- Favor predictable desktop-editor patterns over custom novelty.

## Color

Use a restrained neutral base with a single purposeful accent.

- Background: near-black or deep neutral gray.
- Panels: slightly lifted neutral surfaces.
- Borders: low-contrast separators.
- Text: high-contrast primary, muted secondary.
- Accent: one clear color for selection, focus, and active tools.
- Warnings/errors: semantic colors only.

Avoid large gradients, decorative orbs, heavy shadows, and one-note color themes.

## Typography

- Use a modern sans-serif UI font.
- Keep type compact and legible.
- Use medium weight for labels and section headers.
- Use monospace only for logs, code, shader text, metrics, and technical readouts.
- Do not scale font size with viewport width.
- Keep letter spacing at `0`.

## Controls

- Use icons for common editor actions: save, play, pause, stop, transform, rotate, scale, search, settings.
- Add text labels where ambiguity would slow the user down.
- Use tooltips for icon-only controls.
- Use tabs for major panel modes.
- Use toggles for binary state, sliders or numeric inputs for continuous values, and menus for option sets.
- Selection, hover, focus, disabled, loading, and error states must be visually distinct.

## Motion

- Motion should be short and functional.
- Use transitions for focus, hover, panel open/close, and selection changes.
- Avoid decorative animation in the editor shell.
- Game viewport motion belongs to the game/rendering layer, not the UI chrome.

## Performance Feel

The UI should reinforce the engine's performance goals:

- interactions should feel instant
- expensive work should show progress without blocking the shell
- logs and metrics should update without layout jumps
- viewport controls should never fight panel layout
- avoid UI effects that increase GPU or CPU cost without improving usability

## Accessibility

- Maintain readable contrast for text and controls.
- Do not rely on color alone for important states.
- Keep focus states visible.
- Ensure text fits inside controls at common desktop sizes.
- Prefer explicit labels for data-heavy inspector fields.
