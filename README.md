# mortty

> This project is vibe coded ✨

Mortty is a next-generation GPU-accelerated terminal emulator built in Rust. It uses `wgpu` for hardware-accelerated rendering and `glyphon` for font rasterization, decoupling text rendering from the CPU entirely for seamless, fluid terminal updates.

![mortty screenshot](https://raw.githubusercontent.com/puggeraponanna/mortty/master/screenshot.png)

## Features

- **GPU-accelerated rendering** via `wgpu` + `glyphon` (cosmic-text)
- **Full ANSI color support**
  - 16 standard colors
  - 256-color palette (`38;5;N` / `48;5;N`)
  - 24-bit TrueColor (`38;2;R;G;B` / `48;2;R;G;B`)
- **Background color rendering** — colored prompt segments via native WGSL shader pipeline
- **Block character rendering** — `█`, `▀`, `▄` etc. rendered as GPU quads for pixel-perfect height
- **sRGB gamma-correct rendering** — background quads and text colors match exactly
- **Dynamic terminal sizing** — cols/rows computed from window size; PTY notified on resize
- **Powerline / Starship prompt support** — separators, icons, and colored segments render correctly
- **macOS `.app` bundle** support via `make bundle`

## Prerequisites

- Rust toolchain (via [rustup](https://rustup.rs))
- A [Nerd Font](https://www.nerdfonts.com/) (for Starship/powerline icons)

## Building & Running

```bash
# Dev build
make run

# Optimized release build
make run-release

# Bundle as macOS .app
make bundle
open Mortty.app
```

## Architecture

| Module | Responsibility |
|--------|---------------|
| `window.rs` | winit event loop, keyboard input, resize handling |
| `renderer.rs` | wgpu surface, glyphon text renderer, WGSL background pipeline |
| `terminal.rs` | 2D cell grid, VTE/ANSI parser (CSI, SGR, cursor ops) |
| `pty.rs` | PTY subprocess (portable-pty), async reader thread |
| `bg_shader.wgsl` | WGSL vertex/fragment shader for colored background quads |

## Dependencies

- [`winit`](https://github.com/rust-windowing/winit) — window management
- [`wgpu`](https://wgpu.rs) — GPU rendering
- [`glyphon`](https://github.com/grovesNL/glyphon) — GPU font rasterization
- [`vte`](https://github.com/alacritty/vte) — ANSI escape sequence parser
- [`portable-pty`](https://github.com/wezterm/wezterm/tree/main/pty) — cross-platform PTY
- [`bytemuck`](https://github.com/Lokathor/bytemuck) — safe GPU vertex casting
