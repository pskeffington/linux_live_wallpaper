#!/usr/bin/env bash
set -euo pipefail

packages=(
  base-devel
  cargo
  gtk4
  gtk4-layer-shell
  webkitgtk-6.0
)

usage() {
  cat <<'EOF'
Usage: scripts/install-arch-deps.sh [--dry-run] [--help]

Install the system packages needed to build and run linux-live-wallpaper
on Arch Linux and Arch-based distributions.

Options:
  --dry-run  Print the pacman command without changing the system.
  --help     Show this help.

This script does not install the application, change desktop settings, or
start the wallpaper. Review the package list, then approve pacman in your
terminal when prompted.
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

if [[ ! -e /etc/arch-release ]]; then
  printf '%s\n' "This installer is intended for Arch Linux and Arch-based distributions." >&2
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

sudo pacman -S --needed "${packages[@]}"
