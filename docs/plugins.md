# Plugins

Yal has support for Lua plugins, which can extend the functionality of the application. Plugins are should be added to the `~/.config/yal/plugins.lua` file and will automatically be installed and loaded when yal detects a change to the file.

## Plugin API Reference

Yal supplies a Lua API for plugin developers to interact with the application. The following functions and objects are available:

...
