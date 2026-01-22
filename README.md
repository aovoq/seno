<div align="center">

# Seno

_**[seɪ-no]** — from Japanese「せーの！」meaning "ready, set, go!" — a phrase to start together in sync._

**Chat with Claude, ChatGPT, and Gemini — all at once.**

A native desktop app that displays three AI services side-by-side with a unified input field.

<br>

![Seno Screenshot](screenshot/ss.png)

<br>

[![Download](https://img.shields.io/github/v/release/aovoq/seno?label=Download&style=flat&color=6366f1)](https://github.com/aovoq/seno/releases/latest)
[![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)](https://www.apple.com/macos/)
[![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat&logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)](https://www.linux.org/)
[![Tauri](https://img.shields.io/badge/Tauri_v2-24C8D8?style=flat&logo=tauri&logoColor=white)](https://tauri.app/)

</div>

---

## Features

- **Unified Input** — Type once, send to all three AI services simultaneously
- **Completion Notifications** — Get notified when AI responses are ready (visual + sound)
- **Persistent Sessions** — Stay logged in across app restarts (macOS)
- **Auto Update** — Automatic update checking and installation
- **Customizable Titlebar** — Show/hide and reorder titlebar elements via Settings
- **Memory Monitoring** — Track memory usage in the titlebar
- **Zoom Control** — Adjust AI panel size from 50% to 200%
- **Dark Mode** — Automatic system theme detection
- **Cross Platform** — Available for macOS, Windows, and Linux

## Installation

### Download

Pre-built binaries are available on the [Releases page](https://github.com/aovoq/seno/releases/latest).

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `Seno_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Seno_x.x.x_x64.dmg` |
| Windows | `Seno_x.x.x_x64-setup.exe` |
| Linux (Debian/Ubuntu) | `Seno_x.x.x_amd64.deb` |
| Linux (Fedora/RHEL) | `Seno-x.x.x-1.x86_64.rpm` |
| Linux (Other) | `Seno_x.x.x_amd64.AppImage` |

### Build from Source

Requirements: [Bun](https://bun.sh/) (or npm/pnpm), [Rust](https://rustup.rs/)

```bash
git clone https://github.com/aovoq/seno.git
cd seno
bun install
bun tauri dev      # Development mode
bun tauri build    # Release build
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘ Enter` | Send message to all |
| `⌘ N` | New chat (all services) |
| `⌘ R` | Reload all |
| `⌘ ,` | Open Settings |
| `⌘ +` | Zoom in |
| `⌘ -` | Zoom out |
| `⌘ 0` | Reset zoom |

## Architecture

```
┌─────────────────────────────────────────────┐
│              Titlebar WebView               │
├──────────────┬──────────────┬───────────────┤
│    Claude    │   ChatGPT    │    Gemini     │
│   WebView    │   WebView    │   WebView     │
├──────────────┴──────────────┴───────────────┤
│              Unified Input Bar              │
└─────────────────────────────────────────────┘
```

Five WebViews in a single native window — each AI service maintains its own isolated session.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Tauri v2 |
| Frontend | Vanilla TypeScript |
| Backend | Rust |
| Build | Vite |

## Notes

- **Session persistence** works on macOS only (uses WebKit's `data_store_identifier`)
- **Gemini stability**: Due to WKWebView limitations, occasional errors may occur. Use `⌘ R` to reload if needed

## License

MIT
