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

YAL supports lightweight **Lua** plugins. Plugins can add commands (e.g. Spotify controls, Bluetooth management, window actions, notes/Shortcuts automations via `osascript`) that appear in YAL's command palette.

### Where plugins live

- **Config file:** `~/.config/yal/plugins.lua`
- **Install directory:** `~/.local/share/yal/plugins/<plugin-name>/` (git-cloned plugins go here)

YAL's built-in plugin manager will hot-load plugins from the config file when changes are made (no need to restart YAL).

### Plugin config (`plugins.lua`)

Create `~/.config/yal/plugins.lua` and return an array of plugin entries. Each entry needs a `name` and either a `git` or `path` source:

```lua
return {
    -- Git shorthand: clones from https://github.com/<user>/<repo>.git
    {
        name = "spotify",
        git = "klaatu01/yal-spotify-plugin"
    },

    -- Full git URL
    {
        name = "my-remote-plugin",
        git = "https://github.com/user/repo.git"
    },

    -- Local path (supports ~ for home directory)
    {
        name = "bluetooth",
        path = "~/.local/share/yal/plugins/bluetooth"
    },

    -- Absolute local path
    {
        name = "my-dev-plugin",
        path = "/Users/me/dev/my-plugin"
    },

    -- Plugins can receive arbitrary config
    {
        name = "some-plugin",
        git = "user/some-plugin",
        config = {
            api_key = "...",
            option  = true,
        }
    },
}
```

| Field    | Type           | Description                                                                                       |
|----------|----------------|---------------------------------------------------------------------------------------------------|
| `name`   | string         | Plugin identifier. For git plugins this is also the install directory name.                        |
| `git`    | string or nil  | GitHub shorthand (`user/repo`), full HTTPS URL, or `git@` SSH URL. Cloned to the install directory. |
| `path`   | string or nil  | Local filesystem path to the plugin directory. `~` is expanded to your home directory.             |
| `config` | table or nil   | Free-form config table passed to the plugin's `init(config)` function.                            |

You must provide either `git` or `path` (not both). Path-based plugins are not cloned — YAL loads directly from that directory. This is useful for local development.

### Writing a plugin

Each plugin is a directory containing an `init.lua` that returns a module table with two functions:

- `init(config)` — called once at load time; returns plugin metadata and commands
- `execute(req)` — called when the user runs a command

#### Directory structure

```
my-plugin/
  init.lua          -- required entry point
  vendor/            -- optional: vendored Lua modules (require-able as "a.b.c")
```

#### Minimal example

```lua
-- init.lua
local M = {}

function M.init(config)
  return {
    name = "my-plugin",
    description = "My first YAL plugin",
    version = "0.1.0",
    author = "Me",
    commands = {
      { name = "hello", description = "Say hello" },
    },
  }
end

function M.execute(req)
  if req.command == "hello" then
    print("Hello from my-plugin!")
    return { hide = true }
  end
  return { hide = false }
end

return M
```

#### `init(config)` return value

| Field         | Type   | Description                                    |
|---------------|--------|------------------------------------------------|
| `name`        | string | Plugin name.                                   |
| `description` | string | Short description shown in the command palette. |
| `version`     | string | Semver version string.                         |
| `author`      | string | Author name.                                   |
| `commands`    | array  | List of `{ name, description }` tables.        |

The `config` argument is whatever was specified in `plugins.lua` (or `nil` if omitted).

#### `execute(req)` request shape

```lua
{
  command = "hello",        -- which command the user selected
  args    = ...,            -- optional args (usually nil)
  context = {               -- runtime context
    windows = { ... },      -- list of open windows
    displays = { ... },     -- list of displays
    current_display = { ... },
  },
}
```

Return `{ hide = true }` to dismiss YAL after success, or `{ hide = false }` to keep it open.

### Showing UI: prompts and forms

Plugins can show interactive prompts using the `yal.ui` module. A prompt displays a modal with content nodes and optional form fields.

```lua
local ui = require("yal.ui")

local p = ui.prompt({
  title   = "My Prompt",   -- optional window title
  width   = 60,            -- optional width (percentage, default 75)
  height  = 50,            -- optional height (percentage, default auto)
  content = { ... },       -- array of content nodes (see below)
  hotkeys = { ... },       -- optional hotkey definitions (see below)
})
```

The `prompt()` call returns a prompt handle with three methods:

| Method          | Returns                          | Description                                           |
|-----------------|----------------------------------|-------------------------------------------------------|
| `p:submission()` | table of form values             | Blocks until the user presses Enter. Errors on cancel. |
| `p:state()`      | table of current values, or nil  | Polls live form state. Returns `nil` after submit. Errors on cancel. Rate-limited to 100ms. |
| `p:cancel()`     | nothing                          | Programmatically closes the prompt.                   |

#### Content nodes

The `content` array supports these node types:

```lua
-- Plain text (with optional variant)
{ type = "text", text = "Hello", variant = "heading" }
-- variant: "muted", "caption", "code", "emphasis", "heading" (or omit for default)

-- Markdown
{ type = "markdown", md = "**bold** and _italic_" }

-- Raw HTML
{ type = "html", html = "<div>custom</div>" }

-- Image
{ type = "image", src = "https://...", alt = "desc", w = 100, h = 100 }

-- Layout containers (nestable)
{ type = "v_stack", gap = 8, children = { ... } }
{ type = "h_stack", gap = 8, children = { ... } }
{ type = "grid", cols = 2, gap = 8, children = { ... } }

-- Form with input fields
{ type = "form", name = "my-form", fields = { ... } }
```

#### Form fields

Forms support three field kinds:

```lua
-- Text input
{ kind = "text", name = "username", label = "Username", placeholder = "Enter name", max_length = 50 }

-- Select dropdown
{ kind = "select", name = "choice", label = "Pick one", options = {
    { label = "Option A", value = "a" },
    { label = "Option B", value = "b" },
}}

-- Slider
{ kind = "slider", name = "volume", label = "Volume", min = 0, max = 100, step = 1, value = 50, show_value = true }
```

Form values are returned as a table keyed by field `name` when the user submits (Enter) or when polled via `p:state()`.

#### Hotkeys

Prompts can register hotkeys that the user can press while the prompt is open. Hotkey presses are delivered through the `_hotkey` key in `p:state()` responses.

```lua
local p = ui.prompt({
  title = "Device Manager",
  hotkeys = {
    { key = "r", label = "refresh" },
    { key = "c", label = "connect" },
    { key = "x", label = "disconnect" },
  },
  content = { ... },
})

-- Poll for hotkey presses
while true do
  local state = p:state()
  if state == nil then break end  -- user submitted

  local hotkey = state["_hotkey"]
  if hotkey == "r" then
    -- handle refresh
  elseif hotkey == "c" then
    -- handle connect
  end
end
```

Registered hotkeys are displayed as hints at the bottom of the prompt. Each hotkey press is delivered exactly once — the `_hotkey` field is only present in the `state()` call immediately following the key press.

### Plugin standard library

Plugins have access to a set of built-in Lua modules beyond `yal.ui`. All modules are required as `yal.*`:

| Module     | Require path  | Key functions                                                |
|------------|---------------|--------------------------------------------------------------|
| UI         | `yal.ui`      | `prompt(form)` — show interactive prompts                    |
| JSON       | `yal.json`    | `encode(value)`, `decode(string)`                            |
| HTTP       | `yal.http`    | `request(opts)`, `get(url)`, `post_json(url, body)`          |
| Logging    | `yal.log`     | `debug(msg)`, `info(msg)`, `warn(msg)`, `error(msg)`         |
| Database   | `yal.db`      | `open(namespace)` — persistent key-value store               |
| Base64     | `yal.base64`  | `encode(input)`, `decode(b64)`                               |
| Socket     | `yal.socket`  | `bind(ip, port)` — TCP server/client                         |

Plugins also have access to Lua's full standard library (`io`, `os`, `string`, `table`, etc.).

See the [YAL Lua Library Reference](./docs/yal-std.md) for the full API.

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
