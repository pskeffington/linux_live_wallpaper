# Garuda Dr460nized deployment

This guide targets Garuda Dr460nized's KDE Plasma desktop on Wayland. The application uses GTK 4, WebKitGTK 6, and GTK Layer Shell to place web content as a wallpaper. KWin must expose the Wayland layer-shell protocol; the current prototype has not yet been runtime-validated on a Garuda machine. X11 is unsupported.

## Prepare Garuda

Update the rolling system through Garuda's normal update workflow before installing build dependencies. Then open a terminal in the repository and install the project packages:

```sh
bash scripts/install-arch-deps.sh
```

The script installs `base-devel`, `cargo`, `gtk4`, `gtk4-layer-shell`, `wayland-utils`, and `webkitgtk-6.0` through pacman. It does not run a system upgrade or change desktop settings. Preview the package command first with:

```sh
bash scripts/install-arch-deps.sh --dry-run
```

Log in to the **Plasma (Wayland)** session at SDDM. Check the session and protocol:

```sh
printf 'session=%s\n' "${XDG_SESSION_TYPE:-unset}"
wayland-info | rg zwlr_layer_shell_v1
```

The session should report `wayland`, and the protocol query should return `zwlr_layer_shell_v1`. If the protocol is absent, this backend cannot create the wallpaper layer in that session.

## Try the live Tar1090 feed

The configured test feed is `172.20.30.64:8504`. Build the release binary, list monitor indices, then run the dashboard on the chosen monitor:

```sh
cargo build --release --locked
cargo run --release -- --list-monitors
cargo run --release -- --url http://172.20.30.64:8504/ --monitor 0
```

Replace `0` with the desired zero-based index from the monitor listing. The app needs the dashboard web root, not Tar1090's `data/aircraft.json` endpoint. If the feed does not load, first check that `http://172.20.30.64:8504/` opens from the same desktop session.

Close the foreground app with Ctrl+C. For a self-signed HTTPS dashboard, pass `--allow-self-signed HOST`; the exception accepts only unknown-CA errors for that host, and the certificate hostname must still match.

## Install and start at login

The helper builds and installs the app to `~/.local/bin/linux-live-wallpaper`, then enables a per-user systemd service:

```sh
bash scripts/garuda-deploy.sh install --url http://172.20.30.64:8504/
```

The source is saved to the user's config directory. If `graphical-session.target` is active, the service starts immediately; otherwise, it is enabled for the next login. Review the saved sources with:

```sh
~/.local/bin/linux-live-wallpaper --list
```

To open the interactive controller, run:

```sh
bash scripts/garuda-deploy.sh run
```

The helper stops the background service first. Closing the interactive app leaves the service stopped until the next login. Manage the service with:

```sh
bash scripts/garuda-deploy.sh status
systemctl --user stop linux-live-wallpaper.service
systemctl --user start linux-live-wallpaper.service
```

Remove the service and installed binary with `bash scripts/garuda-deploy.sh remove`. The saved sources remain. The helper changes files only under your home directory and uses systemd user commands; it does not need sudo. It imports the active Wayland and D-Bus session variables into the user manager. If startup fails, inspect:

```sh
systemctl --user status linux-live-wallpaper.service
journalctl --user -u linux-live-wallpaper.service
systemctl --user show-environment
```

## Garuda and KDE notes

- GTK Layer Shell requires a compositor implementing `zwlr_layer_shell_v1`; protocol presence does not guarantee correct placement behind desktop icons.
- The Dr460nized theme can affect GTK controls and transparent surfaces; it does not change the layer-shell protocol.
- The application uses Garuda's system WebKitGTK package and does not bundle a browser engine.
- Transparent output depends on both WebKitGTK and KWin alpha handling.
- Multiple displays, GPU acceleration, sleep/wake, and startup behavior still need runtime validation on Garuda/KWin.
