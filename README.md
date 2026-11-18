# Linux Live Wallpaper

A Rust desktop application for running web content as an interactive live wallpaper, inspired by [Plash](https://github.com/sindresorhus/Plash). The initial compatibility target is Arch Linux and Arch-derived distributions.

## Status

Early prototype. The current design targets GTK 4, WebKitGTK 6, and GTK Layer Shell on Wayland. Desktop wallpaper surfaces are compositor-specific: this backend requires a compositor that implements the layer-shell protocol. X11 and non-layer-shell compositors need separate backends. The code has not yet been verified on a supported compositor.

## Build on Arch Linux

```sh
bash scripts/install-arch-deps.sh
cargo build --release --locked
```

Add and inspect persistent sources:

```sh
cargo run -- --add https://example.com
cargo run -- --add https://news.example --add https://status.example
cargo run -- --list
cargo run -- --remove https://news.example
```

Start the wallpaper from saved sources or provide temporary sources directly. Repeat `--url` for each source:

```sh
cargo run --release
cargo run --release -- --url https://one.example --url https://two.example --rotate-seconds 300 --reload-seconds 900 --monitor 0
```

Per-run page options include a custom stylesheet, script, color inversion, and surface opacity:

```sh
cargo run --release -- --url https://example.com --css-file wallpaper.css --js-file wallpaper.js --invert-colors --opacity 0.85 --transparent-background --deactivate-on-battery
```

Custom JavaScript executes in each loaded page at document start. Only use scripts you trust. Web audio is muted by default. Opacity must be between 0.0 and 1.0.

Sources are stored at `$XDG_CONFIG_HOME/linux-live-wallpaper/sources.txt`, or `~/.config/linux-live-wallpaper/sources.txt` when `XDG_CONFIG_HOME` is unset. Use `--config PATH` to select another file. Only `http://`, `https://`, and `file://` URLs are accepted. Rotation is disabled unless `--rotate-seconds` is greater than zero.

List displays with `cargo run --release -- --list-monitors`, then select one by its zero-based GDK index with `--monitor INDEX`. Monitor ordering can change when displays are reconnected.\n\nFor a local Tar1090 dashboard, point the wallpaper at its web root:

```sh
cargo run --release -- --url http://feeder-host:port/
```

## Goals

- Display arbitrary web pages as desktop backgrounds.
- Manage multiple web sources and per-display assignments.
- Provide reload, navigation, scaling, interaction and developer tools.
- Restore wallpaper state after login.
- Ship reproducible Arch package metadata and document compositor capabilities.

See [ROADMAP.md](ROADMAP.md), [docs/PLASH_PARITY.md](docs/PLASH_PARITY.md), and the [Garuda Dr460nized deployment guide](docs/GARUDA_DR460NIZED.md) for feature scope and platform constraints.

## License

MIT. See [LICENSE](LICENSE).
