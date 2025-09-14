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
- **Lightweight**: ~20 MB RAM, instant launch.
- **Window Switching**: filter for currently running instances of apps and switch to them automatically.

---

## How it works (high level)

- **Backend**: [Tauri] + Rust.
- **Frontend**: WASM (Leptos) UI, talks to Tauri via `invoke`.
- **App discovery**: recursively scans:
  - `/Applications`
  - `/System/Applications`
  - `~/Applications`
- **Open app**: launches the selected `.app` bundle.
- **Window switching**: Uses a mixture of Accessibility APIs and Private and *unstable* Skylight APIs to find and focus running app windows. (This is fragile and may break in future macOS versions.) 


---

## Installation

### Install from Homebew

I haven't sorted out notarization yet, so you need to use `--no-quarantine`:

```bash
brew install --cask --no-quarantine klaatu01/tap/yal
```

### Build from source

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

---

## Permissions

### OS Requirements

Only tested on an M4 Mac and requires at least macOS 15+

### Accessibility & Screen Recording (macOS)

Requires **Accessibility** Permissions and **Screen Recording** (for window switching).
Grant them in System Settings → **Privacy & Security** → **Accessibility** and **Screen Recording**.
_you shoudd be prompted to do this on first run._

For window switching to work affectively, `yal` emulates keypresses to Mission Control, you need to make sure the following shortcuts are enabled.

**System Settings** → **Keyboard** → **Keyboard Shortcuts…** → **Mission Control** → enable:
    - **Move left a space** -> `Control + Left Arrow`
    - **Move right a space** -> `Control + Right Arrow`
    - **Move to space 1** -> `Control + 1`
    - **Move to space 2** -> `Control + 2`
    - **Move to space 3** -> `Control + 3`
    - **Move to space 4** -> `Control + 4`
    - **Move to space 5** -> `Control + 5`
    - **Move to space 6** -> `Control + 6`
    - **Move to space 7** -> `Control + 7`
    - **Move to space 8** -> `Control + 8`
    - **Move to space 9** -> `Control + 9`
    - **Move to space 10** -> `Control + 0`

### Autostart

As `yal` is a long running process, you may want to add it to launch at login.

### Disable Spotlight’s shortcut (macOS)

Spotlight also uses `⌘ Space`. Pick one:

- System Settings → **Keyboard** → **Keyboard Shortcuts…** → **Spotlight** → disable or remap **Show Spotlight**.  
- Or change YAL’s shortcut in code (plugin config).

---

## Usage/Controls

- `⌘ Space`: toggle YAL
- Type to search (fuzzy match)
- `up/down arrow` or `Ctrl-p/Ctrl-n`: navigate results
- `Enter`: launch selected app or switch to its window
- `Esc`: close YAL
- `Ctrl-o/Ctrl-f` Toggles you between "app" and "switch" mode respectively, _yal will remember the last mode you used when it opens next time._

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

- **Window switching doesn’t work**
    - Make sure you granted **Accessibility** and **Screen Recording** permissions in System Settings → **Privacy & Security**.
    - Make sure the Mission Control shortcuts are enabled (see above).
    - Try quitting and restarting YAL after granting permissions.
    - It may not work with all apps, especially non-native ones.

---

## License

MIT. Use it, fork it, rebind it, ship it.
