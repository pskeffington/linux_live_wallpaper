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
cargo run --release -- --list-monitors
cargo run --release -- --url http://FEEDER_HOST:FEEDER_PORT/ --monitor 0
```

Replace `FEEDER_HOST` and `FEEDER_PORT` with the address configured for your local receiver. Replace monitor index `0` with the index from `--list-monitors` if you want another display. The app should use the Tar1090 dashboard's web root, not its `data/aircraft.json` endpoint.

For example, for a feeder with a root page at port 8504:

```sh
cargo run --release -- --url http://FEEDER_HOST:8504/
```

Close the foreground app with Ctrl+C. For per-user login startup, use the deployment helper below.


## Install and start at login

After the dependencies are installed, run the deployment helper from the repository root. It builds the release binary, installs it at `~/.local/bin/linux-live-wallpaper`, and enables a per-user systemd service for the graphical session:

```sh
bash scripts/garuda-deploy.sh install --url http://FEEDER_HOST:8504/
```

Replace `FEEDER_HOST` with the address of your Tar1090 host. The URL is added to the existing saved source list. The controller’s per-source zoom and reload interval are saved in `settings.tsv` beside `sources.txt`. If `graphical-session.target` is active, the service starts immediately; otherwise it is enabled for the next login. Review the current source list with:

```sh
~/.local/bin/linux-live-wallpaper --list
```

To open the interactive controller for the current session, run `bash scripts/garuda-deploy.sh run`. The helper stops the background service first, preventing two wallpaper surfaces from being created. Closing the interactive app leaves the service stopped until the next login. Manage it with:

```sh
bash scripts/garuda-deploy.sh status
systemctl --user stop linux-live-wallpaper.service
systemctl --user start linux-live-wallpaper.service
```

Remove the service and installed binary with `bash scripts/garuda-deploy.sh remove`; the saved source list is preserved. This helper modifies only files under your home directory and uses systemd user commands; it does not need sudo. The installer imports the active `DISPLAY`, `WAYLAND_DISPLAY`, session type, runtime directory, and D-Bus address into systemd’s user manager. The service is ordered after `graphical-session.target` and is conditioned on a Wayland session. If it does not start, inspect `systemctl --user status linux-live-wallpaper.service`, `journalctl --user -u linux-live-wallpaper.service`, and `systemctl --user show-environment`.

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
