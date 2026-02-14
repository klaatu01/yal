# YAL — Yet Another Launcher

A macOS app launcher built with Tauri (Rust backend) + Leptos (WASM frontend). Global hotkey toggles the UI, user types to fuzzy-search apps/windows/plugins, Enter to launch/switch.

## Architecture

```
src/                          # Leptos frontend (WASM)
  app.rs                      # Root App component — signals, key handlers, renders search + results + prompt overlay
  app/filtering.rs            # Fuzzy search/filtering logic for commands
  app/list.rs                 # ResultsList component
  prompt.rs                   # PromptView component — renders plugin prompts, form fields, hotkey hints
  prompt/fields.rs            # Form field renderers (text, select, slider, button)
  prompt/form.rs              # RenderForm component
  prompt/markdown.rs          # Markdown renderer
  prompt/render_node.rs       # Recursive node renderer for prompt content tree
  bridge/events.rs            # Tauri event listeners (prompt:show, prompt:state, prompt:cancel, config, theme, commands)
  bridge/invoke.rs            # Tauri invoke wrappers (api_respond, hide_window, run_cmd, etc.)
  ui/theme.rs                 # Theme/font/window config application to DOM
  utils/focus.rs              # Focus management helpers (raf_focus_search, focus_move, slider nudge)
  utils/keys.rs               # Key combo normalization

crates/yal-core/src/          # Shared types (used by both frontend and backend)
  prompt.rs                   # Prompt, Hotkey, Node, Form, Field, PromptResponse, etc.
  lib.rs                      # Re-exports everything (pub use prompt::*, command::*, config::*, etc.)
  command.rs                  # Command enum (App, Plugin, Switch, Theme), CommandKind
  config.rs                   # AppConfig, WindowConfig, FontConfig, KeysConfig, Shortcut

crates/yal-plugin/src/        # Plugin runtime
  plugin.rs                   # LuaPlugin — bootstraps mlua VM, loads init.lua, caches execute fn
  manager.rs                  # PluginManager — install (git clone), load_plugins, run_command
  manager/config.rs           # PluginConfigEntry — name, git (optional), path (optional), config
  deps.rs                     # install_all — registers all yal.* Lua modules
  deps/ui.rs                  # yal.ui module — prompt() function
  deps/ui/prompt.rs           # Prompt userdata — state(), submission(), cancel() methods
  deps/http.rs                # yal.http module
  deps/json.rs                # yal.json module
  deps/db.rs                  # yal.db module (persistent KV store)
  deps/log.rs                 # yal.log module
  deps/socket.rs              # yal.socket module
  deps/base64.rs              # yal.base64 module
  deps/vendor.rs              # Vendor searcher for plugin vendor/ directories
  backend.rs                  # Backend trait — prompt(), prompt_state(), prompt_submission(), prompt_cancel()
  protocol.rs                 # PluginExecuteRequest, PluginExecuteResponse, PluginInitResponse, etc.

crates/yal-config/src/        # Config loader — evaluates Lua files with mlua, deserializes to Rust types
  lib.rs                      # load_config<T>(path) generic function

src-tauri/                    # Tauri backend (Rust)
  src/main.rs                 # Tauri app setup, commands, event handling
```

## Key Concepts

### Signal flow (frontend)
- App creates signals: `query`, `selected`, `filter`, `prompt`, `form_values`, `hotkey`
- `init_api_listener` listens for Tauri events (`api://prompt:show`, `api://prompt:state`, `api://prompt:cancel`)
- When a plugin calls `ui.prompt()`, the backend emits `api://prompt:show` → sets `prompt` signal → `PromptView` renders
- Plugin polls `state()` → backend emits `api://prompt:state` → frontend responds with current form values (+ `_hotkey` if pressed)
- On Enter/Escape the frontend sends `PromptResponse::Submit`/`Cancel` back via `api_respond`

### Plugin system
- Plugins are Lua scripts loaded in isolated `mlua::Lua` VMs (one per plugin)
- Each VM gets full Lua stdlib (`io`, `os`, etc.) plus `yal.*` modules
- Plugin config lives at `~/.config/yal/plugins.lua` — array of `{ name, git?, path?, config? }`
- Git plugins are cloned to `~/.local/share/yal/plugins/<name>/`
- Path-based plugins load directly from the specified directory (supports `~/` expansion)
- Entry point is always `init.lua` returning a module with `init(config)` and `execute(req)` functions

### Prompt content model
- A `Prompt` has `title`, `width`, `height`, `content: Vec<Node>`, `hotkeys: Option<Vec<Hotkey>>`
- Nodes are tagged union: `v_stack`, `h_stack`, `grid`, `text`, `markdown`, `html`, `image`, `form`
- Forms contain fields: `text`, `select`, `slider` (tagged by `kind`)
- Hotkeys: declared on prompt, rendered as hint bar, delivered via `_hotkey` key in `state()` responses (one-shot)

## Build & Run

```bash
# Prerequisites
rustup target add wasm32-unknown-unknown
cargo install tauri-cli trunk

# Dev
cargo tauri dev

# Build
cargo tauri build

# Check compilation (frontend WASM crate)
cargo build
```

## Conventions

- Frontend uses Leptos 0.7+ (signals via `signal()`, `Memo::new`, `Effect::new`, `leptos::task::spawn_local`)
- Serde tags: nodes use `#[serde(tag = "type", rename_all = "snake_case")]`, fields use `#[serde(tag = "kind", ...)]`
- Config files are Lua scripts that `return` a table (evaluated by mlua, deserialized via serde)
- Plugin Lua modules use `require("yal.ui")`, `require("yal.json")`, etc.
- Pre-existing warning: `set_form_values` is unused in `init_api_listener` (events.rs) — this is known
