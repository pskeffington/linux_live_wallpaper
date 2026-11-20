#!/usr/bin/env bash
# Single-entry Arch/Garuda installation.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
usage() {
  cat <<'EOF'
Usage: bash scripts/install-arch.sh [--url URL] [--monitor INDEX] [--allow-self-signed HOST] [--dry-run]
Runs the dependency installer, compositor preflight, Rust release build and
per-user systemd installation. Do not run this script with sudo.
Existing saved sources are reused when --url is omitted.
EOF
}
die() { printf 'Error: %s\n' "$*" >&2; exit 2; }
args=()
dry=0
while (( $# )); do
  case "$1" in
    --url|--monitor|--allow-self-signed)
      (( $# >= 2 )) || die "$1 requires a value"
      args+=("$1" "$2"); shift 2 ;;
    --dry-run) dry=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) die "Unknown argument: $1" ;;
  esac
done
[[ -e /etc/arch-release ]] && command -v pacman >/dev/null ||
  die "Requires Arch Linux or an Arch-based distribution."
(( EUID != 0 )) || die "Run as your desktop user, not root."
[[ -f scripts/garuda-deploy.sh && -f scripts/install-arch-deps.sh ]] ||
  die "Run from a complete checkout."
if (( dry )); then
  bash scripts/install-arch-deps.sh --dry-run
  printf 'Then: preflight, build release and install user service with arguments:'
  printf ' %q' "${args[@]}"
  printf '\n'
  exit 0
fi
[[ "${XDG_SESSION_TYPE:-}" == wayland ]] ||
  die "Log into a Wayland desktop session first."
bash scripts/install-arch-deps.sh
bash scripts/garuda-deploy.sh preflight
bash scripts/garuda-deploy.sh install "${args[@]}"
printf '\nInstalled. Service: bash scripts/garuda-deploy.sh status\n'
printf 'Logs: journalctl --user -u linux-live-wallpaper.service -n 50 --no-pager\n'
printf 'Remove: bash scripts/garuda-deploy.sh remove\n'
