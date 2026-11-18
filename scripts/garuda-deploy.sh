#!/usr/bin/env bash
set -euo pipefail

app_name="linux-live-wallpaper"
bin_dir="${HOME}/.local/bin"
unit_dir="${HOME}/.config/systemd/user"
unit_file="${unit_dir}/linux-live-wallpaper.service"
binary="${bin_dir}/${app_name}"

usage() {
  cat <<'EOF'
Usage: scripts/garuda-deploy.sh <install|remove|run|status> [--url URL]

install  Build and install the binary for this user, then enable its systemd user service.
remove   Stop/disable the service and remove app files created by this script.
run      Run the app in the current Wayland session with the native controller.
status   Show the user service status.

Options:
  --url URL  Add this URL to the persistent source list before install/run.
             Without --url, existing saved sources are used.
EOF
}

die() { printf '%s\n' "$*" >&2; exit 2; }

[[ $# -ge 1 ]] || { usage >&2; exit 2; }
action="$1"
shift
url=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      [[ $# -ge 2 ]] || die "--url requires a URL"
      url="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      die "Unknown argument: $1"
      ;;
  esac
done

check_wayland() {
  [[ "${XDG_SESSION_TYPE:-}" == wayland ]] || die "Start this from a Garuda Wayland session (XDG_SESSION_TYPE=wayland)."
}

add_url() {
  [[ -n "$url" ]] || return 0
  "$binary" --add "$url"
}

case "$action" in
  install)
    check_wayland
    command -v cargo >/dev/null || die "Install Rust/Cargo and system dependencies first: bash scripts/install-arch-deps.sh"
    command -v systemctl >/dev/null || die "systemctl is required for the user service."
    cargo build --release --locked
    install -Dm755 "target/release/$app_name" "$binary"
    add_url
    install -d "$unit_dir"
    cat > "$unit_file" <<EOF
[Unit]
Description=Linux Live Wallpaper
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$binary --background
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
EOF
    systemctl --user daemon-reload
    systemctl --user enable --now linux-live-wallpaper.service
    printf 'Installed %s and enabled the user service.\n' "$binary"
    printf 'Controller UI: run %s to open it interactively.\n' "$binary"
    ;;
  run)
    check_wayland
    [[ -x "$binary" ]] || die "Not installed; run scripts/garuda-deploy.sh install first."
    add_url
    exec "$binary"
    ;;
  status)
    systemctl --user status linux-live-wallpaper.service
    ;;
  remove)
    systemctl --user disable --now linux-live-wallpaper.service 2>/dev/null || true
    rm -f "$unit_file" "$desktop_file" "$binary"
    systemctl --user daemon-reload 2>/dev/null || true
    printf 'Removed the user service and installed binary. Saved sources are preserved.\n'
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
