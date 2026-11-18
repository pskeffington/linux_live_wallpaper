# Roadmap

The product goal is an independent Linux implementation of Plash's user-visible live-wallpaper workflow, with Arch Linux as the first-class build and packaging target. The original Plash app is no longer open source; parity targets are derived from its public product documentation in [PLASH_PARITY.md](docs/PLASH_PARITY.md). macOS-specific integrations must be reimplemented with native Linux equivalents.

## Milestone 0 — Project and compatibility contract

- [x] Establish a Rust application and Arch dependency baseline.
- [x] Record initial Wayland layer-shell architecture and constraints.
- [ ] Pin and verify GTK, WebKitGTK and gtk4-layer-shell API versions against current Arch packages.
- [ ] Define the supported compositor/version matrix and automated build environment.

## Milestone 1 — Visible wallpaper prototype

- [x] Implement a full-output Wayland background layer with a WebKitGTK view (not yet build-verified).
- [x] Accept one or more web URLs from the command line.
- [x] Add opt-in timed switching between configured URLs.
- [x] Reject unsupported URL schemes.
- [ ] Verify compilation and runtime on Arch Linux with a layer-shell-capable compositor.
- [ ] Add graceful errors for unsupported compositor/protocol and WebKit load failures.
- [ ] Provide a safe quit/recovery shortcut and prevent the wallpaper from obscuring desktop controls.

## Milestone 2 — Plash core workflow

- [x] Persist sources in a per-user text configuration file.
- [x] Add, remove and list sources from the command line.
- [x] Start with saved sources or pass temporary sources explicitly.
- [ ] Native settings window and tray/background service.
- [ ] Reorder sources and switch among them interactively.
- [ ] Restore wallpaper state after login.
- [ ] Reload, navigate, zoom, user-agent and page load controls.
- [ ] Web inspector/developer tools toggle.
- [ ] Choose a display and handle multi-monitor hotplug.
- [ ] Interactive browsing mode toggle, click-through mode and navigation controls.
- [ ] Per-site settings, custom CSS/JavaScript, opacity, transparency and invert colors.
- [ ] Configurable reload interval, mute audio and safe permission controls.
- [ ] Automatic battery-aware deactivation and image aspect-fill mode.
- [ ] CLI/desktop automation actions and browser URL import/share handler.

## Milestone 3 — Linux desktop integration

- [ ] Wayland layer-shell backend with tested compositor compatibility matrix.
- [ ] X11 backend for desktops that do not support the layer-shell protocol.
- [ ] Desktop autostart and session restoration.
- [ ] Native desktop menu/actions, notifications and keyboard shortcuts.
- [ ] Accessibility, localization and high-DPI behavior.

## Milestone 4 — Robustness and feature parity

- [ ] Compare every feature in the public parity matrix and document platform gaps.
- [ ] Resource controls: pause, refresh throttling, memory visibility and network status.
- [ ] Crash recovery, process isolation, logging controls and diagnostics export.
- [ ] UI and browser engine updates through distribution packages.

## Milestone 5 — Arch distribution quality

- [ ] Generate and commit Cargo.lock; use locked builds.
- [ ] Maintained PKGBUILD with a real release source and verified integrity checksum.
- [ ] AUR release workflow and signed tags.
- [ ] CI: formatting, clippy, unit tests and build on supported Arch image.
- [ ] Document install, upgrade, removal, troubleshooting and compositor limitations.
- [ ] Release candidate acceptance on clean Arch Wayland and at least one additional compositor.

## Technical constraints and risks

- Wayland does not provide one universal protocol for placing arbitrary application windows behind desktop icons. Layer-shell is an extension, not a core Wayland feature; compositor support varies.
- GNOME Shell/Mutter does not guarantee layer-shell support by default. Do not claim GNOME compatibility until tested with the needed support.
- WebKitGTK availability and crate bindings can change with Arch rolling releases; package versions must be verified continuously.
- X11 placement and input behavior differ from Wayland and require a separately tested backend.
- Exact parity is constrained where Linux has no equivalent API; track each difference in the parity matrix.
