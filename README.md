# YAL — Yet Another Launcher (for macOS)

A tiny, no-nonsense app launcher. Press `⌘ Space`, type a few letters, hit `Enter`. That’s it.

---

![yal](https://github.com/user-attachments/assets/1b3bb73f-17df-4037-8c03-667692a3d87c)

---

## Features

- **Global hotkey**: toggles with `⌘ Space` (configurable in code).
- **Fuzzy search**: type fragments like `gc` → finds “Google Chrome”.
- **Monitor support**: works with multiple monitors.
- **Config hot-reload**: update `config.toml` and it live-applies (colors, fonts, size) without restarting.

---

## How it works (high level)

- **Backend**: [Tauri] + Rust.
- **Frontend**: WASM (Leptos) UI, talks to Tauri via `invoke`.
- **App discovery**: recursively scans:
  - `/Applications`
  - `/System/Applications`
  - `~/Applications`
- **Open app**: launches the selected `.app` bundle.

---

## Installation

Build from source

**Prereqs**

- Rust (stable)  
- The wasm target: `rustup target add wasm32-unknown-unknown`  
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

> If your setup uses Trunk and you see an error about missing config, add a minimal `Trunk.toml` next to your web `index.html`, or follow the project’s existing structure.

Install from Homebew

```bash
brew install --cask --no-quarantine klaatu01/tap/yal
```

---

## Usage

- Press **`⌘ Space`** to toggle.
- Type to filter. Use **↑/↓** to move, **Enter** to launch.
- Press **Esc** to hide.

### Disable Spotlight’s shortcut (macOS)

Spotlight also uses `⌘ Space`. Pick one:

- System Settings → **Keyboard** → **Keyboard Shortcuts…** → **Spotlight** → disable or remap **Show Spotlight**.  
- Or change YAL’s shortcut in code (plugin config).

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

# Colors (CSS color values)
bg_color = "#111111"        # background behind everything
fg_color = "#2a6ff0"        # highlight background for the selected row
font_bg_color = "#e6e6e6"   # normal text color (on bg_color)
font_fg_color = "#ffffff"   # text color on the highlight row

# Window (logical points)
w_width = 720.0
w_height = 380.0
```

### Config reference

| Key             | Type   |  Description                                                                        |
| --------------- | ------ |  ---------------------------------------------------------------------------------- |
| `font`          | string |  CSS `font-family` stack applied to the UI.                                         |
| `font_size`     | float  |  Base font size in **px** (e.g., `14.0`).                                           |
| `bg_color`      | string |  Background color of the app (CSS color).                                           |
| `fg_color`      | string |  **Highlight background** for the selected list item.                               |
| `font_bg_color` | string |  Text color for normal rows (text on `bg_color`).                                   |
| `font_fg_color` | string |  Text color for the selected row (text on `fg_color`).                              |
| `font_color`    | string |  **Legacy fallback** for `font_bg_color` if that’s unset.                           |
| `w_width`       | float  |  Window width in logical points.                                                    |
| `w_height`      | float  |  Window height in logical points.                                                   |
| `align_h`       | string |  Horizontal alignment on the active display (`left`\|`center`\|`right`).            |
| `align_v`       | string |  Vertical alignment on the active display (`top`\|`center`\|`bottom`).              |
| `margin_x`      | float  |  Inset from the left/right screen edge (used with `align_h = "left"` or `"right"`). |
| `margin_y`      | float  |  Inset from the top/bottom screen edge (used with `align_v = "top"` or `"bottom"`). |

> Any value you omit falls back to the built-in theme. Change the file while YAL is open to see it live-update.

---

## Troubleshooting

- **`⌘ Space` doesn’t toggle YAL**  
  Disable or remap Spotlight’s shortcut, or change YAL’s shortcut in code.

- **YAL vanishes when I click elsewhere**  
  That’s by design. It hides on blur. Hit `⌘ Space` again.

- **Colors/fonts don’t change**  
  Check you’re editing the right file (`~/.config/yal/config.toml` or `$XDG_CONFIG_HOME/yal/config.toml`). Save and wait a second; YAL hot-reloads.

- **App doesn’t appear in the list**  
  Make sure it’s a proper `.app` bundle in `/Applications`, `/System/Applications`, or `~/Applications`.

---

## License

MIT. Use it, fork it, rebind it, ship it.
