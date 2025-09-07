# YAL (Yet Another Launcher)

YAL is a simple, lightweight and customizable application launcher for MacOS.

## Features
- Fast response time
- Fuzzy search
- Vim-like keybindings
- CMD+Space to open
- Minimalistic, yet customizable design

## Installation

At the moment the only way to install YAL is to build it from source.

1. Install Rust and Cargo from [here](https://www.rust-lang.org/tools/install)
2. Install Tauri Prerequisites from [here](https://tauri.app/v1/guides/getting-started/prerequisites)
3. Clone the respository and run 
```
cargo tauri build
```
4. Follow the Install the application by moving the built app from the dmg file to your Applications folder.

I would also go into keyboard settings and disable the default spotlight shortcut (CMD+Space).
