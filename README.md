# YAL — Yet Another Launcher (for macOS)

A tiny, no-nonsense app launcher. Press `⌘ Space`, type a few letters, hit `Enter`. That’s it.

---

![yal](https://github.com/user-attachments/assets/1b3bb73f-17df-4037-8c03-667692a3d87c)

---

## Features

- **Global hotkey**: toggles with `⌘ Space` (configurable in code).
- **Fuzzy search**: type fragments like `gc` → finds “Google Chrome”.
- **Multi-monitor aware**: opens on the active display; plays nicely with separate Spaces.
- **Hot-reload config**: edit `config.toml` and it live-applies (colors, fonts, size).
- **Lightweight**: ~20 MB RAM, instant launch.
- **Window switching**: list running app windows and jump to them (across Spaces).

---

## How it works (high level)

- **Backend**: [Tauri] + Rust.
- **Frontend**: Leptos (WASM) UI; communicates via `invoke`.
- **App discovery**: recursively scans:
  - `/Applications`
  - `/System/Applications`
  - `~/Applications`
- **Launching**: opens the selected `.app` bundle.
- **Switching**: focuses an existing app/window using Accessibility APIs plus a small amount of Mission Control key-emulation (see below).

---

## Under the hood: window detection & switching

YAL gathers a snapshot of displays → spaces → windows, then focuses the one you choose.

- **Space & window inventory (Skylight)**  
  Uses private SkyLight/CGS symbols via a small Rust layer (“Lightsky”) to:
  - list managed displays and their Spaces (`CGSCopyManagedDisplaySpaces`),
  - enumerate windows per Space (`SLSCopyWindowsWithOptionsAndTags` + iterators),
  - infer window type (normal/utility/fullscreen/minimized) from **level** and **tag** bits.  
    (Heuristics include flags like `TAG_HAS_TITLEBAR_LIKE`, and “minimized-ish” masks observed on recent macOS builds.)

- **Metadata enrichment (CoreGraphics)**  
  Separately reads the public `CGWindowListCopyWindowInfo` snapshot to attach **PID**, **owner name**, and **title** to each window ID. This is also why YAL needs **Screen Recording** permission (macOS requires it to access full window metadata).

- **Space targeting**  
  To jump across Spaces, YAL identifies the **display** that contains the target Space, warps the cursor to that display’s center (so Mission Control shortcuts address the right display), then:
  - uses `Control + <digit>` for Spaces 1–10 when available, or  
  - `Control + Left/Right` to walk to the desired index.

- **Focusing the exact window (AX)**  
  After switching to the Space, YAL activates the target app (`NSRunningApplication.activate…`), then uses the Accessibility API to set the **AXFocusedWindow** and perform **AXRaise** for the specific `AXWindowNumber` that matches the CGS window id.

> Note: This relies on private symbols and brittle heuristics. Apple can (and does) change SkyLight internals between major macOS versions. YAL targets macOS 15+ and may need updates over time.

---

## Installation

### Install from Homebrew

Not notarized yet, so you’ll need `--no-quarantine`:

```bash
brew install --cask --no-quarantine klaatu01/tap/yal
```

### Build from source

**Prereqs**

- Rust (stable)
- WASM target: `rustup target add wasm32-unknown-unknown`
- Tauri CLI & Trunk:
  ```bash
  cargo install tauri-cli trunk
  ```

**Run in dev**

```bash
cargo tauri dev
```

**Build a binary**

```bash
cargo tauri build
```

> If Trunk complains about config, add a minimal `Trunk.toml` next to your web `index.html`, or mirror the project’s structure.

---

## Permissions

### OS requirements

Tested on Apple Silicon with **macOS 15+**.

### Accessibility & Screen Recording

YAL needs both:

- **Accessibility** (to focus/raise windows)
- **Screen Recording** (to read window metadata via CGWindow)

System Settings → **Privacy & Security** → **Accessibility** and **Screen Recording**.  
You **should** be prompted on first run. If not, enable them manually and restart YAL.

For switching to work effectively, make sure Mission Control shortcuts are turned on:

**System Settings** → **Keyboard** → **Keyboard Shortcuts…** → **Mission Control** → enable:
- **Move left a space** → `Control + Left Arrow`
- **Move right a space** → `Control + Right Arrow`
- **Move to space 1…10** → `Control + 1…0`

If you use multiple monitors, “Displays have separate Spaces” is recommended.

### Autostart

Since YAL is long-running, consider adding it to **Login Items**.

### Disable Spotlight’s shortcut

Spotlight also binds `⌘ Space`. Pick one:

- System Settings → **Keyboard** → **Keyboard Shortcuts…** → **Spotlight** → disable or remap **Show Spotlight**, **or**
- Change YAL’s shortcut in code (plugin config).

---

## Usage

- `⌘ Space` — toggle YAL
- Type to search (fuzzy match)
- `Up/Down` or `Ctrl-p` / `Ctrl-n` — navigate
- `Enter` — launch selected app **or** switch to its window (if in switch mode)
- `Esc` — close YAL
- `Ctrl-o` / `Ctrl-f` — toggle between **app** and **switch** mode  
  _(YAL remembers the last mode you used.)_

---

## Configuration

YAL reads a TOML file and hot-reloads it on change.

**Location**

- `~/.config/yal/config.toml`

**Example**

```toml
# ~/.config/yal/config.toml

# UI
font = "ui-monospace, SFMono-Regular, Menlo, monospace"
font_size = 14.0

# Colors (CSS)
bg_color = "#111111"        # app background
fg_color = "#2a6ff0"        # highlight background for the selected row
bg_font_color = "#e6e6e6"   # normal text color (on bg_color)
fg_font_color = "#ffffff"   # text color on the highlighted row

# Window (logical points)
w_width = 720.0
w_height = 380.0

# Layout
align_h = "center"          # left | center | right
align_v = "top"             # top | center | bottom
margin_x = 12.0             # px inset for left/right align
margin_y = 12.0             # px inset for top/bottom align
padding  = 6.0              # inner padding
line_height = 1.2           # line height multiplier
w_radius = 10.0             # corner radius in px
```

### Config reference

| Key             | Type   | Description                                                                                           |
|-----------------|--------|-------------------------------------------------------------------------------------------------------|
| `font`          | string | CSS `font-family` stack applied to the UI.                                                            |
| `font_size`     | float  | Base font size in **px** (e.g., `14.0`).                                                              |
| `bg_color`      | string | App background color (CSS color).                                                                     |
| `fg_color`      | string | **Row highlight background** for the selected item.                                                   |
| `bg_font_color` | string | Text color for normal rows (text on `bg_color`).                                                      |
| `fg_font_color` | string | Text color for the selected row (text on `fg_color`).                                                 |
| `w_width`       | float  | Window width in logical points.                                                                       |
| `w_height`      | float  | Window height in logical points.                                                                      |
| `align_h`       | enum   | Horizontal alignment on the active display: `"left"` \| `"center"` \| `"right"`.                    |
| `align_v`       | enum   | Vertical alignment on the active display: `"top"` \| `"center"` \| `"bottom"`.                      |
| `margin_x`      | float  | Horizontal inset (in px) used when `align_h` is `"left"` or `"right"`.                                |
| `margin_y`      | float  | Vertical inset (in px) used when `align_v` is `"top"` or `"bottom"`.                                  |
| `padding`       | float  | Inner padding of the window (in px).                                                                  |
| `line_height`   | float  | Line height multiplier for rows (e.g., `1.2`).                                                        |
| `w_radius`      | float  | Window corner radius (in px).                                                                         |

> Any value you omit falls back to the built-in defaults. Save the file while YAL is open to see live updates.

---

## Troubleshooting

- **`⌘ Space` doesn’t toggle YAL**  
  Disable/remap Spotlight’s shortcut, or change YAL’s shortcut in code.

- **YAL hides when I click elsewhere**  
  That’s intentional; it hides on blur. Press `⌘ Space` again.

- **Colors/fonts don’t change**  
  Confirm you’re editing `~/.config/yal/config.toml` (or `$XDG_CONFIG_HOME/yal/config.toml`). Save and give it a second—YAL hot-reloads.

- **App doesn’t appear**  
  Make sure it’s an `.app` bundle in `/Applications`, `/System/Applications`, or `~/Applications`.

- **Window switching doesn’t work**  
  - Grant **Accessibility** and **Screen Recording** permissions.  
  - Ensure Mission Control shortcuts are enabled (see above).  
  - Quit and relaunch YAL after granting permissions.  
  - Some apps (or non-standard windows) may not expose the right metadata.

---

## License

MIT. Use it, fork it, rebind it, ship it.

[Tauri]: https://tauri.app/
