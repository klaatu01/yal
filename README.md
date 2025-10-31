# YAL — Yet Another Launcher (for macOS)

A tiny, no-nonsense app launcher. Press `⌘ Space`, type a few letters, hit `Enter`. That’s it.

---

![yal-modes](https://github.com/user-attachments/assets/c83384ef-0f9a-46ee-a3a1-ab3b13c0f205)

---

## Features

- **Global hotkey**: toggles with `⌘ Space` (configurable in code).
- **Fuzzy search**: type fragments like `gc` → finds “Google Chrome”.
- **Multi‑monitor aware**: opens on the active display; plays nicely with separate Spaces.
- **Hot‑reload config**: edit `config.lua` and it live‑applies (colors, fonts, size).
- **Theme filtering & switching**: press `Ctrl‑T` to filter themes by name and apply instantly.
- **Lightweight**: ~20 MB RAM, instant launch.
- **Window switching**: list running app windows and jump to them (across Spaces).
- **Pluggable**: uses the built-in Lua plugin manager to add custom commands to the command palette. see [Plugins](#plugins).

---

## How it works (high level)

- **Backend**: [Tauri] + Rust.
- **Frontend**: Leptos (WASM) UI; communicates via `invoke`.
- **App discovery**: recursively scans:
  - `/Applications`
  - `/System/Applications`
  - `~/Applications`
- **Launching**: opens the selected `.app` bundle.
- **Switching**: focuses an existing app/window using Accessibility APIs plus a small amount of Mission Control key‑emulation (see below).

![yal](https://github.com/user-attachments/assets/6c192bcf-8431-4c55-b038-5f7070069bbd)

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

- **Accessibility** (to focus/raise windows and post Mission Control keys)
- **Screen Recording** (to read window metadata via CGWindow)

System Settings → **Privacy & Security** → **Accessibility** and **Screen Recording**.  
You **should** be prompted on first run. If not, enable them manually and restart YAL.

For switching to work effectively, make sure Mission Control shortcuts are turned on:

**System Settings** → **Keyboard** → **Keyboard Shortcuts…** → **Mission Control** → enable:
- **Move left a space** → `Control + Left Arrow`
- **Move right a space** → `Control + Right Arrow`
- **Switch to Desktop 1…10** → `Control + 1…0`

If you use multiple monitors, “Displays have separate Spaces” is recommended.

### Autostart

Since YAL is long‑running, consider adding it to **Login Items**.

### Disable Spotlight’s shortcut

Spotlight also binds `⌘ Space`. Pick one:

- System Settings → **Keyboard** → **Keyboard Shortcuts…** → **Spotlight** → disable or remap **Show Spotlight**, **or**
- Change YAL’s shortcut in code (plugin config).

---

## Usage

- `⌘ Space` — toggle YAL
- Type to search (fuzzy match)
- `Up/Down` or `Ctrl‑p` / `Ctrl‑n` — navigate
- `Enter` — launch selected app **or** switch to its window (if in switch mode)
- `Esc` — close YAL
- `Ctrl‑o` — toggle **App** mode
- `Ctrl‑f` — toggle **Switch** (windows) mode
- `Ctrl‑t` — toggle **Themes** mode (filter themes; `Enter` applies the highlighted theme)

> Theme switching is instant. Applied themes do not persist between restarts, you will need to hard code a chosen theme in `config.lua`.

---

## Configuration

YAL reads Lua files from your XDG config directory and hot‑reloads on change.

**Locations**

- `~/.config/yal/config.lua` (main app config)
- `~/.config/yal/themes.lua` (named theme definitions)

### Quick start: example files

**`~/.config/yal/themes.lua`**
```lua
-- Define one or more named themes. Keys are color hex strings.
-- You can reference any section name here from `config.lua`'s `theme` key.

return {
    {
       name = "catppuccin-mocha"
       bg_color      = "#1e1e2e"
       fg_color      = "#45475a"
       bg_font_color = "#cdd6f4"
       fg_font_color = "#cdd6f4"
    },
    {
       name = "custom",
       bg_color      = "#0f0f14"
       fg_color      = "#2f81f7"
       bg_font_color = "#e6e6e6"
       fg_font_color = "#ffffff"
    }
}
```

**`~/.config/yal/config.lua`**
```lua
-- Pick a theme by name (must exist in themes.lua).
return {
    theme = "catppuccin-mocha"
    font = {
        font      = "Fira Code"   -- CSS font stack allowed
        font_size = 12.0          -- px
    },
    window = {
        w_width     = 400.0       -- logical points
        w_height    = 250.0
        align_h     = "center"    -- left | center | right
        align_v     = "center"    -- top  | center | bottom
        line_height = 0.8
        padding     = 8
        w_radius    = 0
    }
}
```

> Any change you save will be applied live. If you change `theme = ...`, the UI updates immediately. `Ctrl‑T` in YAL lets you preview a theme only.

### Config reference

#### Theme (from `themes.lua`)

![yal-theme](https://github.com/user-attachments/assets/49cb1c21-b55a-4b4e-9587-2d3aa750978c)


Each **theme** is a `[name]` table with these keys:

| Key              | Type   | Description                                             |
|------------------|--------|---------------------------------------------------------|
| `bg_color`       | string | App background color (CSS hex or named color).         |
| `fg_color`       | string | Row highlight background for the selected item.        |
| `bg_font_color`  | string | Text color for normal rows (on `bg_color`).            |
| `fg_font_color`  | string | Text color on the highlighted row (on `fg_color`).     |

> Reference a theme in `config.lua` via `theme = "<name>"`.

#### Font (`[font]` in `config.lua`)

| Key          | Type   | Description                                                     |
|--------------|--------|-----------------------------------------------------------------|
| `font`       | string | CSS `font-family` stack applied to the UI.                      |
| `font_size`  | float  | Base font size in **px** (e.g., `14.0`).                        |

#### Window (`[window]` in `config.lua`)

| Key           | Type   | Description                                                                 |
|---------------|--------|-----------------------------------------------------------------------------|
| `w_width`     | float  | Window width in logical points.                                             |
| `w_height`    | float  | Window height in logical points.                                            |
| `align_h`     | enum   | Horizontal alignment: `"left"` \| `"center"` \| `"right"`.                  |
| `align_v`     | enum   | Vertical alignment: `"top"` \| `"center"` \| `"bottom"`.                    |
| `padding`     | float  | Inner padding (px).                                                         |
| `line_height` | float  | Line height multiplier for rows (e.g., `1.2`).                              |
| `w_radius`    | float  | Corner radius (px).                                                         |

---

## Plugins

YAL supports lightweight **Lua** plugins. Plugins can add commands (e.g. Spotify controls, window actions, notes/Shortcuts automations via `osascript`) that appear in YAL’s command palette.

### Where plugins live

- **Config file:** `~/.config/yal/plugins.lua`  
- **Install directory:** `~/.local/share/yal/plugins/<plugin-name>/` (git-cloned here)

YAL's built in plugin manager will hot-load plugins from the config file when changes are made (no need to restart YAL).

### Quick start

Create `~/.config/yal/plugins.lua`:

Yal uses the format `<plugin-name> = "<github-user>/<repo>"` to clone from GitHub.

```lua
return { 
    {
        name = "spotify",
        git = "klaatu01/yal-spotify-plugin"
    }
}
```

### Writing a plugin (Lua)

Each plugin is a folder with an `init.lua` that returns a table exposing two functions:

- `init()` → returns a JSON-serializable table describing the plugin and its commands
- `execute(req)` → runs a named command

**Minimal skeleton:**

```lua
-- ~/.local/share/yal/plugins/my-plugin/init.lua
local M = {}

function M.init()
  return {
    name = "my-plugin",
    description = "My first YAL plugin",
    version = "0.1.0",
    author = "Me",
    commands = {
      { name = "hello", description = "Say hello in the console" },
    },
  }
end

-- req: { command: string, context: PluginExecuteContext }
function M.execute(req)
  if req.command == "hello" then
    print("Hello from my-plugin!")
    return { hide = true }   -- tell YAL to hide after success
  end
  return { hide = false }    -- unknown command → keep UI open
end

return M
```

**Command visibility:** return `{ hide = true }` when your command succeeds and YAL should dismiss; `{ hide = false }` to keep the UI up (e.g., when nothing happened or you want to show an error result in the UI).

A proper guide is on its way.

### Plugin API

Plugins communicate with YAL via the built-in Lua yal std library. See the [YAL Lua Library Reference](./docs/yal-std.md) for details.

### Example plugins

- [yal-spotify-plugin](https://github.com/klaatu01/yal-spotify-plugin) — control Spotify playback

---

## Troubleshooting

- **`⌘ Space` doesn’t toggle YAL**  
  Disable/remap Spotlight’s shortcut, or change YAL’s shortcut in code.

- **YAL hides when I click elsewhere**  
  That’s intentional; it hides on blur. Press `⌘ Space` again.

- **Colors/fonts don’t change**  
  Confirm you’re editing files in `~/.config/yal/`. Save and give it a second—YAL hot‑reloads.

- **Window switching doesn’t work**  
  - Grant **Accessibility** and **Screen Recording** permissions.  
  - Ensure Mission Control shortcuts are enabled (see above).  
  - Quit and relaunch YAL after granting permissions.  
  - Some apps (or non‑standard windows) may not expose the right metadata.

---

## Under the hood: window detection & switching

YAL gathers a snapshot of displays → spaces → windows, then focuses the one you choose.

- **Space & window inventory (Skylight)**  
  Uses private SkyLight/CGS symbols via a small Rust layer (“Lightsky”) to:
  - list managed displays and their Spaces (`CGSCopyManagedDisplaySpaces`),
  - enumerate windows per Space (`SLSCopyWindowsWithOptionsAndTags` + iterators),
  - infer window type (normal/utility/fullscreen/minimized) from **level** and **tag** bits.  
    (Heuristics include flags like `TAG_HAS_TITLEBAR_LIKE`, and “minimized‑ish” masks observed on recent macOS builds.)

- **Metadata enrichment (CoreGraphics)**  
  Separately reads the public `CGWindowListCopyWindowInfo` snapshot to attach **PID**, **owner name**, and **title** to each window ID. This is also why YAL needs **Screen Recording** permission (macOS requires it to access full window metadata).

- **Space targeting**  
  To jump across Spaces, YAL identifies the **display** that contains the target Space, warps the cursor to that display’s center (so Mission Control shortcuts address the right display), then:
  - uses `Control + <digit>` for Desktops 1–10 when available, or  
  - `Control + Left/Right` to walk to the desired index.

- **Focusing the exact window (AX)**  
  After switching to the Space, YAL activates the target app (`NSRunningApplication.activate…`), then uses the Accessibility API to set the **AXFocusedWindow** and perform **AXRaise** for the specific `AXWindowNumber` that matches the CGS window id.

> Note: This relies on private symbols and brittle heuristics. Apple can (and does) change SkyLight internals between major macOS versions. YAL targets macOS 15+ and may need updates over time.

---

## Contributing

Contributions welcome! Open an issue or PR.

If you would like to help with development, please see the [roadmap](./ROADMAP.md) for ideas.

---

## License

MIT. Use it, fork it, rebind it, ship it.

[Tauri]: https://tauri.app/

## Disclaimer

Not affiliated in any way with https://github.com/srsholmes/yal.

Turns out it just a great name.
