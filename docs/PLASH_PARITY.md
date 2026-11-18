# Plash feature parity

This project is an independent Linux implementation inspired by Plash. Plash's repository states that its app is no longer open source; this project does not copy its source code or macOS assets. The feature list below is based on Plash's public product page, accessed 2026-11-18: https://sindresorhus.com/plash

## Publicly documented Plash features

| Feature | Linux implementation target | Status |
|---|---|---|
| Remote or local website | WebKitGTK URL/file source | CLI supports http, https and file URLs |
| Screen-size URL placeholders | Substitute selected display dimensions in a URL | `[[screenWidth]]` and `[[screenHeight]]` resolve from selected monitor geometry multiplied by its GDK scale factor (default monitor index 0); fractional-scaling and runtime behavior need verification |
| Self-signed certificates | Host-scoped trust exception | Opt-in `--allow-self-signed HOST` accepts only unknown-CA errors for the named host during this run |
| Browsing mode | Temporarily enable pointer/keyboard input and navigation controls | Native controller toggles pointer input and on-demand keyboard focus, restores the browsing preference for saved-source launches, provides Back/Forward history buttons with sensitivity updated as history changes, and injects `is-plash-app` plus the live `plash-is-browsing-mode` class |
| Page zoom | Zoom page contents and remember the level per website | Controller saves and restores per-source zoom; `--zoom` supplies the default for sources without saved settings |
| Automatic reload interval | Per-source timer, pause while hidden or suspended | Controller saves a per-source interval; `--reload-seconds` supplies the default. Automatic reload pauses while the wallpaper surface is hidden; system suspend naturally stops the timer process. Controller status reports load start, response commit, completion, and failures |
| Multiple websites and switching | Source collection with active source | Persistent CLI and native controller add/remove/reorder, timed rotation, and Previous/Next controls; saved-source launches resume the last selected URL from `active-source.txt`, while explicit `--url` sessions remain temporary |
| Choose display | Select output and save assignment per source | CLI display listing and launch-time `--monitor INDEX`; controller accepts a connected GDK output connector per source, persists it, and follows assignments across monitor removal/reconnection with fallback |
| Invert colors | Optional injected stylesheet/filter | Launch-time `--invert-colors` |
| Custom CSS and JavaScript | Explicit per-source customization fields | Launch-time files supply defaults; controller editors save CSS and JavaScript per source and apply them on page switches |
| Custom user-agent | Per-source HTTP User-Agent override | Launch-time `--user-agent` supplies the default; the controller saves a per-source override and applies it on the next load |
| Opacity | Surface opacity where compositor supports it | Controller saves opacity per source; `--opacity` supplies the default for sources without saved settings |
| Transparent background | Web view transparency where supported | Launch-time `--transparent-background`; actual alpha rendering depends on compositor support |
| Deactivate on battery | Power state integration, with manual override | Opt-in `--deactivate-on-battery` polls Linux battery status every 30 seconds and hides the window while discharging |
| Mute audio | WebKit media policy | Muted by default |
| Aspect-fill single image | Image-source mode with crop-to-fill | CSS targets standalone image documents and fills the wallpaper viewport with `object-fit: cover`; not runtime-verified |
| Hide tray/menu icon | Optional background service indicator | No tray/service UI yet |
| Shortcuts / automation | CLI and desktop action interface | Focused controller shortcuts and CLI commands; no desktop-wide shortcut integration |
| Share extension | Linux browser share handler or URL import | Not implemented |

Plash documents that it shows only one website at a time; multiple simultaneous websites are out of scope. It describes multi-display support as choosing one display, with separate websites per display planned. This project should preserve those semantics by default rather than silently claiming broader parity.

Custom JavaScript is injected into every loaded page in the current run and executes with that page's privileges. Only supply trusted scripts. The Linux desktop cannot provide uniform wallpaper placement, input routing, power events, or tray behavior across all compositors. Each implementation must be marked against a tested compositor/backend matrix. This list tracks targets and known implementation state; it is not evidence of runtime support.
