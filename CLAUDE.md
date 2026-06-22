# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Argon-PDF is a Windows 11 native PDF reader built with **Rust + Tauri**, focused on one
workflow: reading a document in a **main viewer** while checking another page of the same
document **side-by-side** (e.g. working a maths exercise while viewing its solution).

Core UX:
- Left: scrollable page **thumbnail strip**; hovering a thumbnail reveals **Main / Side** buttons
  that open that page in the chosen viewer.
- **Main viewer** and **Side viewer**: two independent continuous-scroll page viewers, each with
  its own page position, fit-width/zoom, and a **page-number box** to jump to a page.
- A **draggable divider** resizes main vs side; the side viewer can be toggled on/off.

Rendering quality is a hard requirement: pages must be **crisp like Firefox/Adobe**.

## Tech Stack

- **Frontend**: Svelte 5 + SvelteKit (SPA, `adapter-static`) + Vite + TypeScript. Owns all
  PDF handling.
- **PDF rendering**: **PDF.js (`pdfjs-dist`)** — the engine inside Firefox — rendered to
  `<canvas>` at `devicePixelRatio` and re-rendered on zoom. This is crisp by construction.
  **Do NOT** pre-render pages to images in Rust and display them in `<img>`; that approach is
  persistently blurry at fractional Windows display scaling (this was the original design and
  was removed for that reason).
- **Backend (Rust/Tauri)**: minimal. Its only job is reading a chosen file's bytes
  (`read_pdf` command) and the native file-open dialog. No PDFium, no database.

## Common Commands

```bash
# Development (run the app)
npm run tauri dev

# Production build (installer)
npm run tauri build

# Frontend type-check (Svelte + TS)
npm run check

# Rust checks
cargo build            # from src-tauri/ (or: cargo check)
cargo clippy -- -D warnings
cargo fmt --check
```

## Architecture

### Process boundary
- **Rust core** ([src-tauri/src/lib.rs](src-tauri/src/lib.rs)): one command,
  `read_pdf(path) -> tauri::ipc::Response`, returning raw file bytes as an ArrayBuffer
  (efficient — not a JSON number array). Registers the dialog plugin. That's it.
- **Webview frontend** (`src/`): loads the bytes into PDF.js, renders pages to canvases, and
  implements the whole UI.

### Frontend layout
- [src/lib/pdf.ts](src/lib/pdf.ts): PDF.js worker setup (`workerSrc` from a Vite `?url` import),
  `loadPdf(bytes)`, a shared per-document page cache `getPage()`, and `renderPageToCanvas()`
  (device-pixel-exact, cancels superseded renders).
- [src/lib/stores.ts](src/lib/stores.ts): `pdfDoc` (the shared `PDFDocumentProxy` + page size +
  count), `mainViewer` / `sideViewer` (`{ targetPage, mode: 'fit'|'manual', manualZoom }`),
  and `sideOpen`.
- [src/lib/components/PageViewer.svelte](src/lib/components/PageViewer.svelte): reusable
  virtualized continuous-scroll viewer (IntersectionObserver renders only near-viewport pages);
  fit-width by default via a `ResizeObserver` on the pane, manual zoom on +/−, page-number box.
  Used for **both** main and side.
- `ThumbnailStrip.svelte` / `Thumbnail.svelte`: virtualized thumbnail canvases + hover Main/Side.
- [src/routes/+page.svelte](src/routes/+page.svelte): three-pane layout + the draggable divider
  (ARIA `separator`, pointer-capture drag, keyboard-resizable).

### Rendering rules (the crisp part)
Canvas backing store = `viewport(scale) × devicePixelRatio`; canvas CSS box = logical px. Fit-width
computes `scale` from the measured pane width. During a divider drag the canvas is CSS-scaled live
and the backing store is re-rendered after a short debounce (crisp on settle). See
[[rendering-must-be-pdfjs-not-raster]] in project memory.

## Notes / gotchas
- **PDF.js worker**: imported via `pdfjs-dist/build/pdf.worker.min.mjs?url` and assigned to
  `GlobalWorkerOptions.workerSrc`. CSP in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json)
  must allow `worker-src 'self' blob:`.
- **Drag-and-drop**: Tauri intercepts OS file drops at the window level, so the path comes from
  `getCurrentWebview().onDragDropEvent(...)` (the webview never gets an HTML drop event).
- **Mixed page sizes**: layout uses page 1's size for all slots (maths textbooks are uniform).
  Each page still renders at its own true viewport; only the slot box may differ on odd pages.

## Out of scope (intentionally removed)
Text highlighting, a notes panel, and persistence (SQLite) were dropped to keep the app lean and
fast. Don't reintroduce them — or a server-side raster pipeline — unless explicitly asked.
