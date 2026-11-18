#!/usr/bin/env bash
set -euo pipefail

packages=(
  base-devel
  cargo
  gtk4
  gtk4-layer-shell
  wayland-utils
  webkitgtk-6.0
)

usage() {
  cat <<'EOF'
Usage: scripts/install-arch-deps.sh [--dry-run] [--help]

Install build and runtime packages for linux-live-wallpaper on Arch Linux,
including Garuda Dr460nized. The Wayland tools package is included so the
layer-shell protocol can be checked before launch.

Options:
  --dry-run  Print the pacman command without changing the system.
  --help     Show this help.

This script does not upgrade the system, install the application, change
desktop settings, or start the wallpaper. On Garuda, keep the rolling system
updated with Garuda's normal update tool before installing packages.
Review the package list; pacman will ask for confirmation.
EOF
}

if (( $# > 1 )); then
  usage >&2
  exit 2
fi

dry_run=0
case "${1-}" in
  "")
    ;;
  --dry-run)
    dry_run=1
    ;;
  --help|-h)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

if ! command -v pacman >/dev/null 2>&1 || [[ ! -e /etc/arch-release ]]; then
  printf '%s\n' "This installer requires Arch Linux or an Arch-based distribution with pacman." >&2
  exit 1
fi

if (( dry_run )); then
  printf 'sudo pacman -S --needed'
  printf ' %q' "${packages[@]}"
  printf '\n'
  exit 0
fi

if [[ ! -t 0 || ! -t 1 ]]; then
  printf '%s\n' "Run this script from a terminal so sudo and pacman can request confirmation." >&2
  exit 1
fi

if (( EUID == 0 )); then
  pacman -S --needed "${packages[@]}"
else
  if ! command -v sudo >/dev/null 2>&1; then
    printf '%s\n' "sudo is required when this script is not run as root." >&2
    exit 1
  fi
  sudo pacman -S --needed "${packages[@]}"
fi
