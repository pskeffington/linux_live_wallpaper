# Roadmap

The product goal is a Linux implementation of Plash's user-visible live-wallpaper workflow, with Arch Linux as the first-class packaging and build target. “Replica” means matching the user-facing capabilities where Linux desktop protocols permit; macOS-specific APIs and desktop integration must be reimplemented with native Linux equivalents.

## Milestone 0 — Project and compatibility contract

- [x] Establish a Rust application and Arch dependency baseline.
- [x] Record initial Wayland layer-shell architecture and constraints.
- [ ] Pin and verify GTK, WebKitGTK and gtk4-layer-shell API versions against current Arch packages.
- [ ] Define the supported compositor/version matrix and automated build environment.

## Milestone 1 — Visible wallpaper prototype

- [x] Launch a WebKitGTK view in a full-screen Wayland background layer.
- [x] Accept an initial URL on the command line.
- [ ] Verify compilation and runtime on Arch Linux with GNOME/Mutter plus a layer-shell-capable compositor.
- [ ] Add graceful errors for unsupported compositor/protocol and WebKit load failures.
- [ ] Provide a safe quit/recovery shortcut and prevent the wallpaper from obscuring desktop controls.

## Milestone 2 — Plash core workflow

- [ ] Native settings window and tray/background service.
- [ ] Add, remove, reorder, enable and switch among multiple web sources.
- [ ] Persist settings and restore wallpaper state after login.
- [ ] Reload, navigate, zoom, user-agent and page load controls.
- [ ] Web inspector/developer tools toggle.
- [ ] Per-display assignment, scaling, positioning and independent refresh.
- [ ] Interactive mode toggle: click-through wallpaper by default, temporarily interactive on request.
- [ ] Remember per-site options and fail safely on invalid or unreachable URLs.

## Milestone 3 — Linux desktop integration

- [ ] Wayland layer-shell backend with tested GNOME/KDE/wlroots compatibility matrix.
- [ ] X11 backend for desktops that do not support the layer-shell protocol.
- [ ] Desktop autostart and session restoration.
- [ ] Native desktop menu/actions, notifications and keyboard shortcuts.
- [ ] Multi-monitor hotplug and resolution-change handling.
- [ ] Accessibility, localization and high-DPI behavior.

## Milestone 4 — Feature parity and robustness

- [ ] Compare every Plash release feature against a tracked parity matrix.
- [ ] Web content permissions controls and safe defaults for camera, microphone, location, popups and downloads.
- [ ] Resource controls: pause, refresh throttling, memory visibility, network status.
- [ ] Crash recovery, process isolation, logging controls and diagnostics export.
- [ ] UI and browser engine updates through distribution packages.

## Milestone 5 — Arch distribution quality

- [ ] Maintained PKGBUILD with source integrity checks and reproducible package build.
- [ ] AUR release workflow and signed tags.
- [ ] CI: formatting, clippy, unit tests, package metadata and build on supported Arch image.
- [ ] Document install, upgrade, removal, troubleshooting, portal permissions and compositor limitations.
- [ ] Release candidate acceptance on clean Arch GNOME Wayland and at least one additional compositor.

## Technical constraints and risks

- Wayland does not provide one universal protocol for placing arbitrary application windows behind desktop icons. Layer-shell is an extension, not a core Wayland feature; compositor support varies.
- GNOME Shell/Mutter support for layer-shell is not generally guaranteed by default. The initial prototype must not claim GNOME compatibility until tested with the required extension or compositor support.
- WebKitGTK availability and crate bindings can change with Arch rolling releases; package versions must be verified continuously.
- X11 placement and input behavior differ from Wayland and require a separately tested backend.
- Exact macOS feature parity is not achievable where Linux has no equivalent API; document each difference in the parity matrix.
