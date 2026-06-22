# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Argon-PDF is a Windows 11 native PDF reader built with **Rust + Tauri**. Its defining feature is a dual-page viewer: two independent page viewers for the same PDF can be open simultaneously.

Core capabilities from [Docs/specs.md](Docs/specs.md):
- Left panel: scrollable page thumbnail strip; clicking a thumbnail focuses it in a viewer
- Center panel: primary page viewer with zoom in/out
- Right panel: secondary independent page viewer (its own scroll position, independent of center)
- Text highlighting that persists permanently per-document
- Notes panel: lists all highlights; clicking a note navigates to that page
- Users can delete highlights

## Tech Stack

- **Backend**: Rust (PDF parsing, highlight storage, file I/O)
- **Frontend**: Tauri webview (HTML/CSS/JS or a framework like React/Svelte — TBD)
- **PDF rendering**: likely `pdfium-render` or `pdf-rs` crate (decide before scaffolding)
- **Persistence**: highlights stored locally (SQLite via `rusqlite` or JSON sidecar — TBD)

## Common Commands (once scaffolded)

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build

# Run Rust unit tests
cargo test

# Run a single test
cargo test <test_name>

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

## Architecture Intent

### Process boundary
Tauri splits work across two processes:
- **Rust core** (`src-tauri/`): PDF page rendering, highlight CRUD, file management. Exposed to the frontend via Tauri commands (`#[tauri::command]`).
- **Webview frontend** (`src/` or `ui/`): Three-panel layout, zoom controls, highlight overlay, notes UI. Communicates with Rust via `invoke()`.

### Dual-viewer design
Each viewer is an independent component with its own current-page state. The thumbnail strip is a shared source of truth for page count/order but does not own viewer scroll state.

### Highlight model
Highlights are PDF-coordinate-space rectangles (not pixel positions) so they survive zoom changes and window resizes. Stored persistently alongside the PDF (keyed by file path or hash).

## Test Strategy

Per the spec, a test strategy is required to allow fearless feature addition/removal without regression risk. Planned layers:

1. **Rust unit tests** — pure functions: PDF page count, highlight serialization/deserialization, coordinate math.
2. **Tauri command integration tests** — test each `#[tauri::command]` handler in isolation using Tauri's test utilities.
3. **UI component tests** — test viewer and panel components independently (e.g., Vitest + Testing Library if using a JS framework).
4. **End-to-end tests** — WebdriverIO or Playwright with Tauri's WebDriver support; cover: open PDF → navigate pages → add highlight → reopen → verify highlight persists → delete highlight.

Regression gate: CI must pass all four layers before merging any feature branch.
