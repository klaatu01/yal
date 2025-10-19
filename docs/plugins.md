# Plugins

Yal has support for Lua plugins, which can extend the functionality of the application. Plugins are should be added to the `~/.config/yal/plugins.toml` file and will automatically be installed and loaded when yal detects a change to the file.

## Plugin API Reference

Yal supplies a Lua API for plugin developers to interact with the application. The following functions and objects are available:

...

## Architecture Reference

```mermaid
flowchart LR
    subgraph Host[YAL Core]
      UI[YAL Window]
      PM[Plugin Manager]
      LRT[Lua Runtime<br/>(per plugin)]
      ER[Event Reactor]
      DISC[Discovery Graph<br/>(Application Tree)]
    end

    CFG[plugins.toml<br/>(~/.config/yal/plugins.toml)]
    PL[Lua Plugin(s)]
    YLIB[YAL Lua Library<br/>(host-injected)]

    CFG -- watched --> PM
    PM -- load/refresh --> LRT
    LRT -- loads --> PL
    PM -. inject .-> YLIB

    UI -- open window --> PM
    PM -- init(config, ctx) --> PL
    PL -- PluginManifest --> PM
    PM -- register cmds --> UI

    %% Execution path
    UI -- command + args + exec_ctx --> PM
    PM --> PL
    PL -- call --> YLIB
    YLIB -- Event{op,payload,responder} --> ER
    ER -- handle op using DISC/system --> ER
    ER -- JSON reply --> YLIB
    YLIB --> PL
    PL -- PluginExecuteResponse --> PM
    PM --> UI
```
