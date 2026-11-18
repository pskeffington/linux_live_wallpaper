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
  --url URL  install: add URL to persistent sources before enabling.
             run: use URL temporarily for this session.
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
    built_binary="target/release/$app_name"
    [[ -x "$built_binary" ]] || die "The release binary was not produced by cargo build."
    if [[ -n "$url" ]]; then
      "$built_binary" --add "$url"
    fi
    sources="$("$built_binary" --list)"
    if [[ -z "${sources//[[:space:]]/}" ]]; then
      die "No saved sources. Re-run with --url http://FEEDER_HOST:8504/ or add a source first."
    fi
    install -Dm755 "$built_binary" "$binary"
    install -d "$unit_dir"
    cat > "$unit_file" <<EOF
[Unit]
Description=Linux Live Wallpaper
After=graphical-session.target
PartOf=graphical-session.target
ConditionEnvironment=XDG_SESSION_TYPE=wayland
ConditionEnvironment=WAYLAND_DISPLAY

[Service]
Type=simple
Environment=GDK_BACKEND=wayland
ExecStart=$binary --background
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
EOF
    systemctl --user daemon-reload
    import_vars=(DISPLAY WAYLAND_DISPLAY XDG_SESSION_TYPE XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS)
    import_args=()
    for name in "${import_vars[@]}"; do
      if [[ -n "${!name:-}" ]]; then
        import_args+=("$name")
      fi
    done
    if (( ${#import_args[@]} )); then
      systemctl --user import-environment "${import_args[@]}"
    fi
    if ! systemctl --user is-active --quiet graphical-session.target; then
      printf '%s\n' "The graphical-session.target is not active yet; enabling service for the next login."
      systemctl --user enable linux-live-wallpaper.service
    else
      systemctl --user enable --now linux-live-wallpaper.service
    fi
    printf 'Installed %s and enabled the user service.\n' "$binary"
    printf 'Controller UI: run %s to open it interactively.\n' "$binary"
    ;;
  run)
    check_wayland
    [[ -x "$binary" ]] || die "Not installed; run scripts/garuda-deploy.sh install first."
    systemctl --user stop linux-live-wallpaper.service 2>/dev/null || true
    if [[ -n "$url" ]]; then
      exec "$binary" --url "$url"
    fi
    exec "$binary"
    ;;
  status)
    systemctl --user status linux-live-wallpaper.service
    ;;
  remove)
    systemctl --user disable --now linux-live-wallpaper.service 2>/dev/null || true
    rm -f "$unit_file" "$binary"
    systemctl --user daemon-reload 2>/dev/null || true
    printf 'Removed the user service and installed binary. Saved sources are preserved.\n'
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
