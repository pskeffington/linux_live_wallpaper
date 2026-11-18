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
- [x] Add configurable interval-based page reload.
- [x] Reject unsupported URL schemes.
- [x] Add launch-time custom CSS and JavaScript injection, invert colors, surface opacity, transparent background request and muted audio.
- [ ] Verify compilation and runtime on Arch Linux with a layer-shell-capable compositor.
- [ ] Add graceful errors for unsupported compositor/protocol and WebKit load failures.
- [ ] Provide a safe quit/recovery shortcut and prevent the wallpaper from obscuring desktop controls.

## Milestone 2 — Plash core workflow

- [x] Persist sources in a per-user text configuration file.
- [x] Add, remove and list sources from the command line.
- [x] Start with saved sources or pass temporary sources explicitly.
- [ ] Persist all page customization per source; zoom, opacity, and reload interval are implemented, while CSS, JavaScript, user-agent and other options remain launch-wide.
- [x] Native controller window with Previous/Next/Reload and browsing-mode toggle.
- [ ] Full settings window and tray/background service.
- [x] Switch among sources with native Previous/Next controls.
- [x] Add, remove and reorder sources from the native controller; active selection is not restored after restart.
- [x] Add an opt-in Garuda user-level systemd service that starts saved sources at login; automatic enablement remains installer-driven.
- [ ] Reload, navigation controls and web inspector/developer tools.
- [x] Configure launch-time page zoom and user-agent.
- [x] Persist zoom, opacity, and automatic reload interval per source.
- [x] List available displays and select one by its zero-based GDK monitor index.
- [ ] Persist per-source display assignments and handle multi-monitor hotplug.
- [x] Click-through wallpaper surface with a native browsing-mode toggle and on-demand keyboard focus.
- [x] Add WebKit Back/Forward history controls to the native controller.
- [ ] Remember browsing preferences and add navigation-state affordances.
- [x] Add opt-in battery-aware hiding based on Linux power-supply status.
- [ ] Add remaining parity items, including image aspect-fill and richer power-management integration.

## Milestone 3 — Linux desktop integration

- [ ] Wayland layer-shell backend with tested compositor compatibility matrix.
- [ ] X11 backend for desktops that do not support the layer-shell protocol.
- [x] Add a Garuda user service for desktop session startup; broader desktop autostart support remains open.
- [x] Add focused controller shortcuts for Previous/Next, Reload and browsing mode.
- [ ] Add desktop-wide shortcuts, notifications and native actions.
- [ ] Accessibility, localization and high-DPI behavior.

## Milestone 4 — Robustness and feature parity

- [ ] Compare every feature in the public parity matrix and document platform gaps.
- [ ] Resource controls: pause, refresh throttling, memory visibility and network status.
- [ ] Crash recovery, process isolation, logging controls and diagnostics export.
- [ ] UI and browser engine updates through distribution packages.

## Milestone 5 — Arch distribution quality

- [x] Generate and commit Cargo.lock; use locked builds.
- [x] Add an Arch dependency installer and Garuda Dr460nized deployment guide.
- [ ] Maintained PKGBUILD with a real release source and verified integrity checksum.
- [ ] AUR release workflow and signed tags.
- [ ] CI: formatting, clippy, unit tests and verified build on supported Arch image.
- [x] Add an Arch package build workflow; the workflow result is not yet confirmed.
- [ ] Package install, upgrade, removal, troubleshooting and compositor limitations for a release; Garuda helper instructions now cover per-user deploy/remove.
- [ ] Release candidate acceptance on clean Arch Wayland and at least one additional compositor.

## Technical constraints and risks

- Wayland does not provide one universal protocol for placing arbitrary application windows behind desktop icons. Layer-shell is an extension, not a core Wayland feature; compositor support varies.
- GNOME Shell/Mutter does not guarantee layer-shell support by default. Do not claim GNOME compatibility until tested with the needed support.
- WebKitGTK availability and crate bindings can change with Arch rolling releases; package versions must be verified continuously.
- X11 placement and input behavior differ from Wayland and require a separately tested backend.
- Exact parity is constrained where Linux has no equivalent API; track each difference in the parity matrix.
