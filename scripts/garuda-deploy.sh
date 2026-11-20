#!/usr/bin/env bash
set -euo pipefail

app_name="linux-live-wallpaper"
bin_dir="${HOME}/.local/bin"
unit_dir="${HOME}/.config/systemd/user"
unit_file="${unit_dir}/linux-live-wallpaper.service"
binary="${bin_dir}/${app_name}"
autostart_dir="${HOME}/.config/autostart"
autostart_file="${autostart_dir}/linux-live-wallpaper-session.desktop"
session_launcher="${bin_dir}/linux-live-wallpaper-session"

usage() {
  cat <<'EOF'
Usage: scripts/garuda-deploy.sh <preflight|install|remove|run|status|diagnose> [--url URL] [--monitor INDEX] [--allow-self-signed HOST]

preflight Check the active layer-shell protocol and native libraries without changes.
install  Build and install the binary for this user, then enable its systemd user service.
remove   Stop/disable the service and remove app files created by this script.
run      Run the app in the current Wayland session with the native controller.
status   Show the user service status.\ndiagnose Print login/session diagnostics and recent service logs.

Options:
  --url URL  install: add URL to persistent sources before enabling.
             run: use URL temporarily for this session.
             Without --url, existing saved sources are used.
  --monitor INDEX
             Select a zero-based GDK monitor for install or run.
  --allow-self-signed HOST
             Trust only unknown-CA certificate errors for HOST. Repeatable.
EOF
}

die() { printf '%s\n' "$*" >&2; exit 2; }

[[ $# -ge 1 ]] || { usage >&2; exit 2; }
action="$1"
shift
url=""
monitor_index=""
tls_hosts=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      [[ $# -ge 2 ]] || die "--url requires a URL"
      url="$2"
      shift 2
      ;;
    --monitor)
      [[ $# -ge 2 ]] || die "--monitor requires an index"
      monitor_index="$2"
      shift 2
      ;;
    --allow-self-signed)
      [[ $# -ge 2 ]] || die "--allow-self-signed requires a host"
      tls_hosts+=("$2")
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

if [[ -n "$monitor_index" && ! "$monitor_index" =~ ^[0-9]+$ ]]; then
  die "Invalid monitor index: $monitor_index"
fi

for host in "${tls_hosts[@]}"; do
  [[ "$host" =~ ^[A-Za-z0-9][A-Za-z0-9.-]*$ ]] || die "Invalid TLS host: $host"
done

check_wayland() {
  [[ "${XDG_SESSION_TYPE:-}" == wayland ]] || die "Start this from a Garuda Wayland session (XDG_SESSION_TYPE=wayland)."
  command -v wayland-info >/dev/null || die "wayland-info is missing; install dependencies with bash scripts/install-arch-deps.sh."
  local protocols
  protocols="$(wayland-info 2>/dev/null)" || die "Could not query the active Wayland compositor."
  [[ "$protocols" == *zwlr_layer_shell_v1* ]] || die "This compositor does not advertise zwlr_layer_shell_v1; Linux Live Wallpaper needs a layer-shell-capable Wayland session."
}

check_native_deps() {
  command -v pkg-config >/dev/null || die "pkg-config is missing; install dependencies with bash scripts/install-arch-deps.sh."
  local missing=()
  for package in gtk4 webkitgtk-6.0 gtk4-layer-shell; do
    pkg-config --exists "$package" || missing+=("$package")
  done
  (( ${#missing[@]} == 0 )) || die "Missing native development/runtime packages: ${missing[*]}. Run bash scripts/install-arch-deps.sh."
}

add_url() {
  [[ -n "$url" ]] || return 0
  "$binary" --add "$url"
}

case "$action" in
  preflight)
    check_wayland
    check_native_deps
    printf '%s\n' "Wayland layer shell and native build/runtime dependencies are ready."
    ;;
  install)
    check_wayland
    check_native_deps
    command -v cargo >/dev/null || die "Install Rust/Cargo first: bash scripts/install-arch-deps.sh"
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
    systemctl --user stop linux-live-wallpaper.service 2>/dev/null || true
    install -Dm755 "$built_binary" "$binary"
    install -d "$unit_dir"
    service_exec="$binary --background"
    if [[ -n "$monitor_index" ]]; then
      service_exec+=" --monitor $monitor_index"
    fi
    for host in "${tls_hosts[@]}"; do
      service_exec+=" --allow-self-signed $host"
    done
    cat > "$unit_file" <<EOF
[Unit]
Description=Linux Live Wallpaper
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
Environment=GDK_BACKEND=wayland
ExecStart=$service_exec
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
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
    # KDE desktop autostart runs after login with the live Wayland environment.
    # It imports session variables before starting the service to avoid stale
    # systemd user-manager environment left over from install or logout.
    install -d "$autostart_dir"
    cat > "$session_launcher" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${XDG_SESSION_TYPE:-}" == wayland ]] || exit 0
[[ -n "${WAYLAND_DISPLAY:-}" ]] || exit 0
names=()
for name in DISPLAY WAYLAND_DISPLAY XDG_SESSION_TYPE XDG_CURRENT_DESKTOP XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS; do
  [[ -n "${!name:-}" ]] && names+=("$name")
done
(( ${#names[@]} )) && systemctl --user import-environment "${names[@]}"
systemctl --user reset-failed linux-live-wallpaper.service 2>/dev/null || true
systemctl --user restart linux-live-wallpaper.service
EOF
    chmod 755 "$session_launcher"
    cat > "$autostart_file" <<EOF
[Desktop Entry]
Type=Application
Name=Linux Live Wallpaper Session Startup
Comment=Start the wallpaper once the Wayland desktop session is available
Exec=$session_launcher
TryExec=$session_launcher
Terminal=false
OnlyShowIn=KDE;
X-KDE-autostart-phase=2
EOF
    # Avoid startup before the compositor exists. The desktop autostart entry
    # starts the service once Plasma has established its Wayland session.
    systemctl --user disable linux-live-wallpaper.service 2>/dev/null || true
    "$session_launcher"
    printf 'Installed %s and configured KDE session autostart.\n' "$binary"
    printf 'Controller UI: run %s to open it interactively.\n' "$binary"
    ;;
  run)
    check_wayland
    check_native_deps
    [[ -x "$binary" ]] || die "Not installed; run scripts/garuda-deploy.sh install first."
    systemctl --user stop linux-live-wallpaper.service 2>/dev/null || true
    run_args=()
    if [[ -n "$url" ]]; then
      run_args+=(--url "$url")
    fi
    if [[ -n "$monitor_index" ]]; then
      run_args+=(--monitor "$monitor_index")
    fi
    for host in "${tls_hosts[@]}"; do
      run_args+=(--allow-self-signed "$host")
    done
    exec "$binary" "${run_args[@]}"
    ;;
  status)
    systemctl --user status linux-live-wallpaper.service
    ;;
  diagnose)
    printf 'Session type: %s\nWayland display: %s\nDesktop: %s\n' "${XDG_SESSION_TYPE:-unset}" "${WAYLAND_DISPLAY:-unset}" "${XDG_CURRENT_DESKTOP:-unset}"
    printf '\nAutostart entry:\n'
    if [[ -f "$autostart_file" ]]; then cat "$autostart_file"; else printf 'not installed\n'; fi
    printf '\nService unit state:\n'
    systemctl --user show linux-live-wallpaper.service -p LoadState -p ActiveState -p SubState -p Result -p ExecMainStatus 2>&1 || true
    printf '\nUser manager session variables:\n'
    systemctl --user show-environment 2>/dev/null | grep -E '^(WAYLAND_DISPLAY|XDG_SESSION_TYPE|XDG_CURRENT_DESKTOP|DISPLAY)=' || true
    printf '\nRecent logs:\n'
    journalctl --user -u linux-live-wallpaper.service -b -n 65 --no-pager 2>&1 || true
    ;;
  remove)
    systemctl --user disable --now linux-live-wallpaper.service 2>/dev/null || true
    rm -f "$unit_file" "$binary" "$autostart_file" "$session_launcher"
    systemctl --user daemon-reload 2>/dev/null || true
    printf 'Removed the user service and installed binary. Saved sources are preserved.\n'
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
