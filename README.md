# Linux Live Wallpaper

A Rust desktop application for running web content as an interactive live wallpaper, inspired by [Plash](https://github.com/sindresorhus/Plash). The initial compatibility target is Arch Linux and Arch-derived distributions, with a setup guide for Garuda Dr460nized (KDE Plasma).

## Status

Early prototype. The current design targets GTK 4, WebKitGTK 6, and GTK Layer Shell on Wayland. Desktop wallpaper surfaces are compositor-specific: this backend requires a compositor that implements the layer-shell protocol. X11 and non-layer-shell compositors need separate backends. The code has not yet been verified on a supported compositor.

## Build on Arch Linux

```sh
bash scripts/install-arch-deps.sh
cargo build --release --locked
```

For Garuda Dr460nized, start with the [Garuda setup guide](docs/GARUDA_DR460NIZED.md). It covers dependency installation, Wayland protocol checks, and the KDE user service. To try the configured Tar1090 test feed:

```sh
cargo run --release -- --url http://172.20.30.64:8504/ --monitor 0
```

To install it for the current user and enable startup at login:

```sh
bash scripts/garuda-deploy.sh install --url http://172.20.30.64:8504/ --monitor 0
```

Add and inspect persistent sources:

```sh
cargo run -- --add https://example.com
cargo run -- --add https://news.example --add https://status.example
cargo run -- --list
cargo run -- --remove https://news.example
```

Add at least one source before starting from saved sources, or provide temporary sources directly. Repeat `--url` for each source:

```sh
cargo run --release
cargo run --release -- --url https://one.example --url https://two.example --rotate-seconds 300 --reload-seconds 900 --monitor 0
```

A native controller window opens with Previous, Next, Reload, browsing-mode, browser history, and persistent source controls. Add, remove, and reorder sources there; changes are saved to the source list. The controller also saves zoom, opacity, automatic reload interval, custom CSS, custom JavaScript, user-agent, and GDK output connector for each page. While browsing mode is enabled, right-click for Back, Forward, Reload, and Zoom In/Out, pinch with two touch points to magnify, or choose Inspect Page to open WebKit’s inspector; zoom changes save automatically. Use the connector shown by `--list-monitors`; assignments follow the output across source switches, with fallback when it is disconnected. The wallpaper surface is click-through by default; browsing mode enables pointer input and on-demand keyboard focus; Back and Forward navigate the active page’s WebKit history. Pages receive Plash-compatible `is-plash-app` and `plash-is-browsing-mode` classes on the root HTML element. With the controller focused, use Alt+Left/Right to switch sources, Ctrl+R to reload, and Ctrl+B to toggle browsing mode. Closing the controller exits the wallpaper.

Per-run page options include custom CSS and JavaScript, color inversion, surface opacity, page zoom, a custom user-agent, and a host-scoped exception for self-signed TLS certificates:

```sh
cargo run --release -- --url https://dashboard.example --css-file wallpaper.css --js-file wallpaper.js --invert-colors --opacity 0.85 --zoom 1.15 --user-agent 'Mozilla/5.0' --allow-self-signed dashboard.example --transparent-background --deactivate-on-battery
```

Custom JavaScript executes in each loaded page at document start. Only use scripts you trust. Web audio is muted by default. `--allow-self-signed HOST` is opt-in and accepts only unknown-certificate-authority errors for that host; hostname mismatches, expired certificates, and other TLS errors still fail. Standalone image URLs are styled to fill the wallpaper viewport while preserving aspect ratio. Opacity must be between 0.0 and 1.0.

Sources are stored at `$XDG_CONFIG_HOME/linux-live-wallpaper/sources.txt`, or `~/.config/linux-live-wallpaper/sources.txt` when `XDG_CONFIG_HOME` is unset. Per-source zoom, opacity, reload, CSS, JavaScript, user-agent, and output connector settings are stored alongside them in `settings.tsv`; `active-source.txt` remembers the last selected URL when starting from saved sources. Use `--config PATH` to select another file. Only `http://`, `https://`, and `file://` URLs are accepted. Rotation is disabled unless `--rotate-seconds` is greater than zero.

List displays with `cargo run --release -- --list-monitors`, then select one by its zero-based GDK index with `--monitor INDEX`. Monitor ordering can change when displays are reconnected.

Use `[[screenWidth]]` and `[[screenHeight]]` in a URL to substitute the selected monitor’s dimensions. With no `--monitor`, output index `0` is selected. For example:

```sh
cargo run --release -- --url 'https://picsum.photos/[[screenWidth]]/[[screenHeight]]' --monitor 0
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
