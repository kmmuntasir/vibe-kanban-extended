# Local Production Build Guide — macOS

## Quick Start

```bash
# Install system dependencies (one-time)
xcode-select --install
brew install cmake

# Build
pnpm run build:npx

# Run
cd npx-cli && node bin/cli.js
```

## Build Command

| Command | What it does |
|---------|-------------|
| `pnpm run build:npx` | Full production build (web app + Rust workspace + npx-cli) |
| `pnpm run build:npx -- --desktop` | Includes Tauri desktop app (.app bundle) |

The build script (`local-build.sh`) auto-detects macOS and produces `macos-arm64` or `macos-x64` artifacts.

Build steps:
1. Builds the web frontend (`packages/local-web` → Vite production build)
2. Builds all Rust workspace crates in release mode
3. Zips binaries (`server`, `vibe-kanban-mcp`, `review`) into `npx-cli/dist/macos-<arch>/`
4. Installs npx-cli npm dependencies and bundles TypeScript

## System Dependencies

### Required

**Xcode Command Line Tools** — provides `cc`, `clang`, `libclang`, and macOS SDKs:

```bash
xcode-select --install
```

**cmake** — required by `aws-lc-sys` (AWS-LC crypto library used by rustls):

```bash
brew install cmake
```

### Not needed on macOS

These Linux packages have **no macOS equivalent** — the build works without them:

| Linux package | Why not needed on macOS |
|---------------|------------------------|
| `libglib2.0-dev` | GTK not used; Tauri uses native Cocoa/WebKit |
| `libgtk-3-dev` | GTK not used on macOS |
| `libwebkit2gtk-4.1-dev` | macOS has built-in WebKit framework |
| `libappindicator3-dev` | macOS uses native status bar |
| `librsvg2-dev` | Not used on macOS |
| `libsoup-3.0-dev` | macOS uses URLSession/CFNetwork |
| `libjavascriptcoregtk-4.1-dev` | macOS has built-in JavaScriptCore |
| `patchelf` | Linux ELF tool, not applicable to Mach-O |
| `clang` / `libclang-dev` | Xcode CLT already provides these |
| `nasm` | Only needed on x86_64; M2 is arm64 |

### Why macOS needs fewer dependencies

| Subsystem | Linux | macOS |
|-----------|-------|-------|
| **Tauri/WebView** | GTK3 + WebKitGTK (6 apt packages) | Built-in WebKit framework (0 packages) |
| **C compiler** | `clang` + `libclang-dev` (apt) | Xcode CLT (built-in) |
| **Crypto (TLS)** | Vendored OpenSSL (`openssl-sys`) | Security.framework (built-in) |
| **Crypto (aws-lc)** | `cmake` (apt) | `cmake` (Homebrew) |
| **SQLite** | Bundled from source (same) | Bundled from source (same) |

## Build Time

| Phase | Time (M2) |
|-------|-----------|
| Rust release build (workspace, from scratch) | ~15-20 minutes |
| Rust release build (incremental) | ~1.5 seconds |
| npx-cli TypeScript bundle | ~13 milliseconds |

Full cold build with no cached artifacts: **~15-20 minutes** on M2.

## Artifacts

After a successful build on Apple Silicon:

```
npx-cli/dist/macos-arm64/
├── vibe-kanban.zip         (~50 MB)   Main server binary
├── vibe-kanban-mcp.zip     (~9 MB)    MCP server binary
└── vibe-kanban-review.zip  (~4 MB)    Review CLI binary
```

Intel Macs produce `npx-cli/dist/macos-x64/`.

With `--desktop` flag, Tauri produces an `.app.tar.gz` in `npx-cli/dist/tauri/darwin-aarch64/`.

## Common Issues

### `xcode-select: error: tool 'xcodebuild' requires Xcode`

Install Command Line Tools:
```bash
xcode-select --install
```

### `cmake: command not found`

```bash
brew install cmake
```

### `libclang` not found (bindgen error)

`libsqlite3-sys` uses bindgen to generate SQLite bindings. If Xcode CLT libclang isn't found:

```bash
export LIBCLANG_PATH=$(xcrun --show-sdk-path)/usr/lib
```

Or point directly:
```bash
export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib
```

### macOS Gatekeeper blocks binary

After unzipping, macOS may refuse to run unsigned binaries:

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine vibe-kanban
```

Or allow in **System Settings → Privacy & Security**.

### `cargo: no such command: watch` (dev mode only)

```bash
cargo install cargo-watch
```

### Tauri desktop build: `"tauri" not found`

Install Tauri CLI:
```bash
cargo install tauri-cli --version "^2"
```
