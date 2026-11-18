# Plash feature parity

This project is an independent Linux implementation inspired by Plash. Plash's repository states that its app is no longer open source; this project does not copy its source code or macOS assets. The feature list below is based on Plash's public product page, accessed 2026-11-18: https://sindresorhus.com/plash

## Publicly documented Plash features

| Feature | Linux implementation target | Status |
|---|---|---|
| Remote or local website | WebKitGTK URL/file source | CLI supports http, https and file URLs |
| Browsing mode | Temporarily enable pointer/keyboard input and navigation controls | Native controller toggles pointer input and on-demand keyboard focus, with Back/Forward browser history buttons |
| Page zoom | Zoom page contents and remember the level per website | Controller saves and restores per-source zoom; `--zoom` supplies the default for sources without saved settings |
| Automatic reload interval | Per-source timer, pause while hidden or suspended | Controller saves a per-source interval; `--reload-seconds` supplies the default. Reload currently continues while hidden or suspended |
| Multiple websites and switching | Source collection with active source | Persistent CLI and native controller add/remove/reorder, timed rotation, and Previous/Next controls; active source is not restored |
| Choose display | Monitor picker and per-source assignment | CLI display listing and launch-time `--monitor INDEX`; no saved assignment or settings UI |
| Invert colors | Optional injected stylesheet/filter | Launch-time `--invert-colors` |
| Custom CSS and JavaScript | Explicit per-source customization fields | Launch-time files via `--css-file` and `--js-file`; not saved per source |
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
