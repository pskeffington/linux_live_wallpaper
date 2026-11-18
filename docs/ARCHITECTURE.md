# Architecture

## Initial backend

The prototype uses Rust for application orchestration, GTK 4 for the native window, WebKitGTK 6 for rendering, and gtk4-layer-shell to request a Wayland background layer. The web engine and GTK libraries are native system dependencies; Rust does not replace those components.

The initial command-line entry accepts one URL and constructs a full-output layer surface with keyboard focus disabled. This is a capability probe and architecture seed, not yet a validated wallpaper manager.

## Planned modules

- `app`: lifecycle, CLI, settings and profile management.
- `web`: WebKit view, navigation policy, per-site permissions and reload.
- `desktop`: backend interface; Wayland layer-shell and X11 implementations.
- `display`: monitor discovery and wallpaper-to-monitor assignment.
- `ui`: settings and source management.
- `storage`: versioned configuration and migration.

Keep desktop protocol code behind a backend trait so unsupported compositors can fail with actionable diagnostics and alternate backends can evolve independently.

## Security and usability

Arbitrary webpages execute active content. Restrict privileged browser features by default, keep downloads and popups explicit, and provide an always-available way to stop or hide the wallpaper. Do not embed credentials in saved URLs or diagnostics.
