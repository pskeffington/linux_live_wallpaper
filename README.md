# Linux Live Wallpaper

A Rust desktop application for running web content as an interactive live wallpaper, inspired by [Plash](https://github.com/sindresorhus/Plash). The initial compatibility target is Arch Linux and Arch-derived distributions.

## Status

Early prototype. The current design targets GTK 4, WebKitGTK 6, and GTK Layer Shell on Wayland. Desktop wallpaper surfaces are compositor-specific: this backend requires a compositor that implements the layer-shell protocol. X11 and non-layer-shell compositors need separate backends. The code has not yet been verified on a supported compositor.

## Build on Arch Linux

```sh
sudo pacman -S --needed base-devel cargo gtk4 gtk4-layer-shell webkitgtk-6.0
cargo build --release
```

Run one or more web sources; repeat `--url` to supply several:

```sh
cargo run --release -- --url https://example.com
cargo run --release -- --url https://one.example --url https://two.example --rotate-seconds 300
```

Only `http://`, `https://`, and `file://` URLs are accepted. Rotation is disabled unless `--rotate-seconds` is greater than zero.

## Goals

- Display arbitrary web pages as desktop backgrounds.
- Manage multiple web sources and per-display assignments.
- Provide reload, navigation, scaling, interaction and developer tools.
- Restore wallpaper state after login.
- Ship reproducible Arch package metadata and document compositor capabilities.

See [ROADMAP.md](ROADMAP.md) and [docs/PLASH_PARITY.md](docs/PLASH_PARITY.md) for feature scope and platform constraints.

## License

MIT. See [LICENSE](LICENSE).
