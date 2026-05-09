# Local Production Build Guide

## Quick Start

```bash
# Install system dependencies (one-time)
sudo apt-get install -y libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev clang libclang-dev

# Build
pnpm run build:npx

# Run
cd npx-cli && node bin/cli.js
```

## Build Command

| Command | What it does |
|---------|-------------|
| `pnpm run build:npx` | Full production build (web app + Rust workspace + npx-cli) |
| `pnpm run build:npx -- --desktop` | Includes Tauri desktop app |

The build script (`local-build.sh`) performs these steps:

1. Builds the web frontend (`packages/local-web` → Vite production build)
2. Builds all Rust workspace crates in release mode
3. Zips binaries (`server`, `vibe-kanban-mcp`, `review`) into `npx-cli/dist/<platform>/`
4. Installs npx-cli npm dependencies and bundles TypeScript

## System Dependencies

### Required

Two groups of system packages must be installed on Ubuntu/Debian:

**Group 1 — Tauri/GTK** (required by `crates/tauri-app` in the workspace):

| Package | Purpose |
|---------|---------|
| `libglib2.0-dev` | GLib C library headers (`glib-sys` crate build) |
| `libgtk-3-dev` | GTK3 widget toolkit headers |
| `libwebkit2gtk-4.1-dev` | WebKit webview for Tauri |
| `libappindicator3-dev` | System tray indicator |
| `librsvg2-dev` | SVG rendering |
| `libsoup-3.0-dev` | HTTP library (WebKit dependency) |
| `libjavascriptcoregtk-4.1-dev` | JavaScript engine (WebKit dependency) |
| `patchelf` | Tauri AppImage bundling |

**Group 2 — SQLite/bindgen** (required by `libsqlite3-sys`):

| Package | Purpose |
|---------|---------|
| `clang` | C compiler frontend |
| `libclang-dev` | Clang library headers (bindgen uses libclang to parse C headers) |

### Why bindgen needs clang

`libsqlite3-sys` bundles SQLite source and uses `bindgen` to generate Rust bindings from `sqlite3.h`. Bindgen needs libclang to parse C headers. The system's GCC is not sufficient — GCC's internal include path for `stdarg.h` is not discoverable by bindgen's clang-based parser.

## Build Time

| Phase | Time |
|-------|------|
| Rust release build (workspace, from scratch) | ~27 minutes |
| Rust release build (incremental) | ~1.5 seconds |
| npx-cli TypeScript bundle | ~13 milliseconds |

Full cold build with no cached artifacts: **~27 minutes**.

## Common Issues

### `cargo: no such command: watch` (dev mode only)

Install `cargo-watch`:
```bash
cargo install cargo-watch
```

### `glib-sys` build fails: `glib-2.0.pc` not found

Install Group 1 system packages (see above).

### `libsqlite3-sys` build fails: `'stdarg.h' file not found`

Install Group 2 system packages: `clang libclang-dev`.

## Artifacts

After a successful build:

```
npx-cli/dist/linux-x64/
├── vibe-kanban.zip         (~50 MB)   Main server binary
├── vibe-kanban-mcp.zip     (~9 MB)    MCP server binary
└── vibe-kanban-review.zip  (~4 MB)    Review CLI binary
```
