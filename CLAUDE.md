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
- **PDF rendering**: `pdfium-render` (PDFium). Chosen over `pdf-rs` because text-snapped highlighting needs reliable per-glyph text bounding boxes, which PDFium provides. Note: PDFium is a native dynamic library that must be bundled with the app (see Build notes).
- **Persistence**: single SQLite database in `%APPDATA%` via `rusqlite`. Not per-file JSON sidecars.

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

### Build notes
`pdfium-render` links against the PDFium native library (`pdfium.dll` on Windows), which is **not** vendored by Cargo. The DLL must be available at dev/runtime and bundled into the Tauri installer (via `tauri.conf.json` resources / `externalBin`). A missing or ABI-mismatched `pdfium.dll` is the most likely first-run failure — confirm it loads before debugging anything else.

## Architecture Intent

### Process boundary
Tauri splits work across two processes:
- **Rust core** (`src-tauri/`): PDF page rendering, highlight CRUD, file management. Exposed to the frontend via Tauri commands (`#[tauri::command]`).
- **Webview frontend** (`src/` or `ui/`): Three-panel layout, zoom controls, highlight overlay, notes UI. Communicates with Rust via `invoke()`.

### Dual-viewer design
Each viewer is an independent component with its own current-page state. The thumbnail strip is a shared source of truth for page count/order but does not own viewer scroll state.

### Highlight model
Highlighting is **text-snapped**: the user selects text from the PDF's text layer (via PDFium text extraction), not free-form boxes. A single highlight is a *list* of rectangles (a multi-line selection spans several rects) plus a page number — model it as a list, not one box. Rectangles are stored in **PDF-coordinate space on the unrotated page** (not pixels) so they survive zoom changes, window resizes, and DPI changes.

If a PDF has no selectable text (e.g. a scanned/image-only document), highlighting is **disabled** and the UI shows a message; viewing/scrolling/zoom/dual-viewer still work. OCR is out of scope.

### Persistence model
Highlights live in one SQLite DB in `%APPDATA%`. Documents are keyed by **content hash** (with last-known file path stored as a hint) so highlights survive moving/renaming the PDF. Writes are transactional so a crash mid-write cannot corrupt the store.

## Test Strategy

Per the spec, a test strategy is required to allow fearless feature addition/removal without regression risk. Planned layers:

1. **Rust unit tests** — pure functions: PDF page count, highlight serialization/deserialization, coordinate math.
2. **Tauri command integration tests** — test each `#[tauri::command]` handler in isolation using Tauri's test utilities.
3. **UI component tests** — test viewer and panel components independently (e.g., Vitest + Testing Library if using a JS framework).
4. **End-to-end tests** — WebdriverIO or Playwright with Tauri's WebDriver support; cover: open PDF → navigate pages → add highlight → reopen → verify highlight persists → delete highlight.

Regression gate: CI must pass all four layers before merging any feature branch.
