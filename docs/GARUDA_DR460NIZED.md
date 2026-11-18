# Garuda Dr460nized deployment

Garuda Dr460nized uses KDE Plasma. This project’s first backend is GTK 4 with GTK Layer Shell on Wayland. The upstream gtk4-layer-shell project lists KDE Plasma on Wayland among supported compositors, but this application still needs runtime validation on the installed KWin version. X11 is not supported by this prototype.

## Install dependencies

Run from the repository root in a terminal:

```sh
bash scripts/install-arch-deps.sh
```

The script installs `base-devel`, `cargo`, `git`, `gtk4`, `gtk4-layer-shell`, and `webkitgtk-6.0` with `pacman -S --needed`. Review and confirm the package transaction in your terminal. To preview the command without changing the system:

```sh
bash scripts/install-arch-deps.sh --dry-run
```

## Build and run

Confirm the current session is Wayland, then build and open a Tar1090 web root:

```sh
test "${XDG_SESSION_TYPE:-}" = wayland
cargo build --release --locked
cargo run --release -- --url http://FEEDER_HOST:FEEDER_PORT/
```

Replace `FEEDER_HOST` and `FEEDER_PORT` with the address configured for your local receiver. The app should use the Tar1090 dashboard's web root, not its `data/aircraft.json` endpoint.

For example, for a feeder with a root page at port 8504:

```sh
cargo run --release -- --url http://FEEDER_HOST:8504/
```

Close the foreground app with Ctrl+C. The app does not yet install an autostart entry.

## Layer-shell support

GTK Layer Shell requires a compositor implementing `zwlr_layer_shell_v1`. The upstream [gtk4-layer-shell supported-compositors list](https://github.com/wmww/gtk4-layer-shell#supported-compositors) includes KDE Plasma on Wayland. Runtime placement and stacking still need to be checked on the particular Garuda/KWin session; a successful compile does not prove the wallpaper sits behind desktop icons.

To inspect the session and available Wayland protocols, install `wayland-utils` if needed and run:

```sh
printf 'session=%s\n' "${XDG_SESSION_TYPE:-unset}"
wayland-info | rg zwlr_layer_shell_v1
```

If the protocol is absent, the app cannot create a background layer in that session. Use a Wayland session with layer-shell support; do not expect the prototype to work under X11.

## Known Garuda-specific limits

- The Dr460nized theme does not alter the layer-shell protocol, but theme colors can affect transparent surfaces and GTK controls.
- The app uses WebKitGTK from the Arch package repository and does not bundle a private browser engine.
- Transparent output depends on both WebKitGTK and KWin alpha handling.
- GPU acceleration, multiple monitors, sleep/wake behavior, autostart, and persistent per-source display settings have not been validated on this machine.
