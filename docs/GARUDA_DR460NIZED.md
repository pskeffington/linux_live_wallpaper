# Garuda Dr460nized deployment

This guide targets Garuda Dr460nized's KDE Plasma desktop on Wayland. The application uses GTK 4, WebKitGTK 6, and GTK Layer Shell to place web content as a wallpaper. KWin must expose the Wayland layer-shell protocol; the tested Garuda session advertises it, but wallpaper placement and rendering remain unverified. X11 is unsupported.

## Validation snapshot — 2026-11-18

- **PASS — Arch build:** GitHub Actions run [44283177523](https://github.com/pskeffington/linux_live_wallpaper/actions/runs/44283177523) completed `cargo build --release --locked` successfully on current Arch packages at source revision `71b7ced`.
- **PASS — Wayland protocol:** `wayland-info` exited 0 on the Garuda Dragonized session and listed `zwlr_layer_shell_v1` (version 5).
- **PASS — Tar1090 endpoint:** `curl -sSI --max-time 8 http://172.20.30.64:8504/` exited 0 and returned HTTP 200.
- **BLOCKED — local runtime:** GTK 4.24.1 is installed, but `webkitgtk-6.0` and `gtk4-layer-shell` are absent. The dependency installer needs interactive sudo authorization before the app can be built and displayed here. Layer placement, Tar1090 rendering, and session startup remain unverified.

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

The session should report `wayland`, and the protocol query should return `zwlr_layer_shell_v1`. The Garuda deployment helper repeats this check and refuses to build or launch when the active compositor does not advertise layer shell. It also checks that GTK 4, WebKitGTK 6, and GTK Layer Shell development metadata is installed before building or running.

## Try the live Tar1090 feed

The configured test feed is `172.20.30.64:8504`. Build the release binary, list monitor indices, then run the dashboard on the chosen monitor:

```sh
cargo build --release --locked
cargo run --release -- --list-monitors
cargo run --release -- --url http://172.20.30.64:8504/ --monitor 0
```

Replace `0` with the desired zero-based index from the monitor listing. The deployment helper accepts the same index with `--monitor 0`, preserving the selected output when its login service starts. In the controller, enter the GDK connector name (for example, `DP-1`) in the active page's display field and save settings to assign that source to an output. The assignment is stored in `settings.tsv`; unplugging that output falls back to the launch output or first connected display, and reconnecting it restores the assignment. The app needs the dashboard web root, not Tar1090's `data/aircraft.json` endpoint. If the feed does not load, first check that `http://172.20.30.64:8504/` opens from the same desktop session.

Close the foreground app with Ctrl+C. For a self-signed HTTPS dashboard, pass `--allow-self-signed HOST`; the exception accepts only unknown-CA errors for that host, and the certificate hostname must still match.

## Install and start at login

The helper builds and installs the app to `~/.local/bin/linux-live-wallpaper`, then enables a per-user systemd service:

```sh
bash scripts/garuda-deploy.sh install --url http://172.20.30.64:8504/ --monitor 0
```

The source is saved to the user's config directory. When launching from saved sources, the most recently selected URL is restored after restart; explicit URLs passed to a direct app invocation remain temporary. If `graphical-session.target` is active, the service starts immediately; otherwise, it is enabled for the next login. The controller saves per-source zoom, opacity, reload interval, custom CSS, custom JavaScript, user-agent, and display connector in `settings.tsv` beside the source list. Saving CSS, JavaScript, or user-agent changes reloads the active page so they take effect immediately.

Review the saved sources with:

```sh
~/.local/bin/linux-live-wallpaper --list
```

To open the interactive controller, run:

```sh
bash scripts/garuda-deploy.sh run --monitor 0
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
