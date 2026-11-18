# Linux Live Wallpaper

A Rust desktop application for running web content as an interactive live wallpaper, inspired by [Plash](https://github.com/sindresorhus/Plash). The initial compatibility target is Arch Linux and Arch-derived distributions.

## Status

Early prototype. The current design targets GTK 4, WebKitGTK 6, and GTK Layer Shell on Wayland. Desktop wallpaper surfaces are compositor-specific: this backend is intended for compositors that implement the layer-shell protocol. X11 and non-layer-shell compositors need separate backends.

## Build on Arch Linux

```sh
sudo pacman -S --needed base-devel cargo gtk4 gtk4-layer-shell webkitgtk-6.0
cargo build --release
cargo run --release -- --url https://example.com
```

## Goals

- Display arbitrary web pages as desktop backgrounds.
- Manage multiple web sources and per-display assignments.
- Provide reload, navigation, scaling, interaction and developer tools.
- Restore wallpaper state after login.
- Ship reproducible Arch package metadata and document compositor capabilities.

See [ROADMAP.md](ROADMAP.md) for feature scope and platform constraints. Current code is an early capability prototype, not yet validated on a target compositor.

## License

MIT. See [LICENSE](LICENSE).
