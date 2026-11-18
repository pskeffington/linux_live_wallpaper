# Plash feature parity

This project is an independent Linux implementation inspired by Plash. Plash's repository states that its app is no longer open source; this project does not copy its source code or macOS assets. The feature list below is based on Plash's public product page, accessed 2026-11-18: https://sindresorhus.com/plash

## Publicly documented Plash features

| Feature | Linux implementation target | Status |
|---|---|---|
| Remote or local website | WebKitGTK URL/file source | Prototype accepts remote URL only |
| Browsing mode | Temporarily enable pointer/keyboard input and navigation controls | Not implemented |
| Automatic reload interval | Per-source timer, pause while hidden or suspended | Not implemented |
| Multiple websites and switching | Source collection with active source | Not implemented |
| Choose display | Monitor picker and per-source assignment | Not implemented |
| Invert colors | Optional injected stylesheet/filter | Not implemented |
| Custom CSS and JavaScript | Explicit per-source customization fields | Not implemented |
| Opacity | Surface opacity where compositor supports it | Not implemented |
| Transparent background | Web view transparency where supported | Not implemented |
| Deactivate on battery | Power state integration, with manual override | Not implemented |
| Mute audio | WebKit media policy | Not implemented |
| Aspect-fill single image | Image-source mode with crop-to-fill | Not implemented |
| Hide tray/menu icon | Optional background service indicator | Not implemented |
| Shortcuts / automation | CLI and desktop action interface | Not implemented |
| Share extension | Linux browser share handler or URL import | Not implemented |

Plash documents that it shows only one website at a time; multiple simultaneous websites are out of scope. It describes multi-display support as choosing one display, with separate websites per display planned. This project should preserve those semantics by default rather than silently claiming broader parity.

The Linux desktop cannot provide uniform wallpaper placement, input routing, power events, or tray behavior across all compositors. Each implementation must be marked against a tested compositor/backend matrix. The feature list is a product target, not evidence that Linux behavior has been completed.
