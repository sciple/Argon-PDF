# Argon-PDF — Implementation Plan

## Context

Argon-PDF is a greenfield Windows 11 native PDF reader (Rust + Tauri). The repo currently
contains only [specs.md](specs.md) and [../CLAUDE.md](../CLAUDE.md) — no code yet. This plan
turns the spec + locked design decisions into a buildable, incremental scaffold focused on a
lightweight, streamlined reading experience.

**Defining feature:** two independent page viewers over the same PDF, plus persistent
text-snapped highlighting surfaced in a notes panel.

### Locked decisions (from spec + planning conversation)
- **Frontend:** Svelte 5 (+ Vite + TypeScript). Reactive state across 4 views (two viewers,
  thumbnail strip, notes) is the core problem; Svelte gives that while compiling to a tiny bundle.
- **PDF engine:** `pdfium-render` (PDFium native lib). Needed for per-glyph text rects.
- **Persistence:** single SQLite DB in `%APPDATA%` via `rusqlite`, keyed by content hash.
- **Highlighting:** text-snapped (selects real text), stored as a *list* of rects + page, in
  PDF-coordinate space. Disabled with a banner when the PDF has no text layer (scanned docs). No OCR.
- **Paging:** continuous vertical scroll per viewer, virtualized (render only visible pages).
- **Page assignment:** hovering a thumbnail (or a note row) reveals **Center / Right** buttons that
  load that page into the chosen viewer. No hidden "active viewer" mode.
- **Open file:** toolbar Open button (native picker) + drag-and-drop. No `.pdf` file association in v1.

---

## Architecture

### Process boundary
- **Rust core (`src-tauri/`)** — PDFium binding, page rendering, text layout + rect-for-range,
  content hashing, SQLite CRUD. Exposed via `#[tauri::command]` and a custom image protocol.
- **Svelte frontend (`src/`)** — three-panel layout, virtualized viewers, selection/highlight
  overlays, notes panel. Talks to Rust via `invoke()` and `<img>` URLs.

### How a page is shown (two stacked layers per page)
1. **Raster layer** — PDFium renders the page to a bitmap, served to an `<img>` via a custom Tauri
   protocol `argon://render/{doc_id}/{page}/{scale}` so the **browser caches** images and lazy-loads
   them. One render path is shared by center, right, and thumbnails (just different `scale`).
2. **Text + highlight overlay** — absolutely-positioned transparent spans (one per word/char, from
   a `get_page_text_layout` command) sit over the bitmap so **native browser text selection** works.
   Existing highlights are drawn as semi-transparent divs from stored rects, scaled to current zoom.

### Highlight flow (text-snapped, minimal frontend logic)
1. User selects text → on `mouseup`, read start/end **char index** from the selected spans'
   `data-char` attributes.
2. `add_highlight(doc_id, page, start_char, end_char)` → Rust uses PDFium
   (`FPDFText_CountRects` / `GetRect`) to compute the covering rects (handles multi-line wraps),
   persists to SQLite, returns the `Highlight`.
3. Frontend draws the returned rects and the notes panel updates reactively.
Rust owns all coordinate math; the frontend only passes char indices. Empty/zero-width selections
(`start == end`) are ignored.

### State (Svelte stores in `src/lib/stores.ts`)
- `document` — `{ doc_id, page_count, has_text_layer, page_sizes[] }` (page_sizes drives virtualized layout without rendering).
- `centerViewer`, `rightViewer` — **separate** stores: `{ scrollTop, zoom }`. Independence is the headline feature; no shared mutable page state.
- `highlights` — array, drives both on-page overlays and the notes list.

---

## Rust command surface (`src-tauri/src/commands.rs`)

| Command | Purpose |
|---|---|
| `open_document(path)` | Load via PDFium, compute blake3 content hash, detect text layer, return `{ doc_id, page_count, has_text_layer, page_sizes }`. Cache the open doc in app state. Returns a typed error for encrypted/corrupt/non-PDF. |
| `get_page_text_layout(doc_id, page)` | Per-page word/char rects (PDF coords) for the selection overlay. |
| `add_highlight(doc_id, page, start_char, end_char, color)` | Compute rects via PDFium, persist, return `Highlight`. |
| `list_highlights(doc_id)` | All highlights for the doc (on open). |
| `delete_highlight(id)` | Remove one highlight. |
| Custom protocol `argon://render/{doc_id}/{page}/{scale}` | Render-on-demand bitmap (used by viewers + thumbnails); browser-cached. |

**App state (`state.rs`):** managed struct holding the `Pdfium` binding, a map of open documents,
a render LRU cache, and the SQLite connection.

### Persistence (`src-tauri/src/store/`)
SQLite at `%APPDATA%/Argon-PDF/argon.db` (resolve via Tauri path API). Migrations run on startup.
```
documents(doc_id PK, last_path, page_count, created_at, updated_at)
highlights(id PK, doc_id FK, page_index, start_char, end_char, rects_json, text_excerpt, color, created_at)
```
All writes in a transaction (crash-safe). Keyed by `doc_id` (blake3 hash) so highlights survive
move/rename; `last_path` kept as a hint.

---

## Frontend structure (`src/`)

```
src/
  App.svelte                 # CSS-grid 3-panel layout + top bar (Open, Notes toggle)
  lib/
    api.ts                   # typed invoke() wrappers + protocol URL builder
    stores.ts                # document, centerViewer, rightViewer, highlights
    components/
      TopBar.svelte          # Open button, drag-drop target, Notes toggle
      ThumbnailStrip.svelte  # virtualized list
      Thumbnail.svelte       # page img + hover overlay (Center / Right buttons)
      PageViewer.svelte      # reusable; prop = which viewer store. Virtualized scroll + zoom
      SelectionLayer.svelte  # transparent text spans, selection -> char range
      HighlightLayer.svelte  # draws stored rects, scaled to zoom; click to delete
      NotesPanel.svelte      # highlight list, Center/Right navigate, delete
```

**Virtualized continuous scroll:** container height = Σ page heights at current zoom (from
`page_sizes`); render only pages intersecting the viewport (+ buffer) as absolutely-positioned
slots; each slot loads its `<img>` from the render protocol. Zoom re-renders at a new scale bucket
(debounced), never bitmap-stretches.

---

## Corner cases baked in (from earlier analysis)
- **Encrypted PDF:** `open_document` returns a `NeedsPassword` error → frontend prompts once; hard
  failure shows a clean message. **Never `unwrap()` on PDF/IO paths.**
- **Corrupt / non-PDF / 0-byte:** validate + return typed error, degrade to an error state, no panic.
- **No text layer:** `has_text_layer=false` → selection overlay disabled + banner
  ("This document has no selectable text — highlighting is unavailable"); viewing still works.
- **Rotated / mixed page sizes:** use per-page `page_sizes` and PDFium's rotation-aware rects.
- **Huge PDFs:** virtualization + lazy `<img>` + render LRU keep memory/CPU flat.
- **File moved after open:** session uses the in-memory doc; reopen matches by hash.

---

## Build / tooling notes
- Scaffold with `cargo create-tauri-app` (Tauri v2, Svelte-TS template).
- **`pdfium.dll` is NOT vendored by Cargo** — bundle it as a Tauri resource/`externalBin` and ensure
  it loads in dev. A missing/ABI-mismatched DLL is the most likely first-run failure; verify it
  before debugging anything else.
- CI gate (per CLAUDE.md): `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
  frontend `vitest`, and E2E must pass before merge.

---

## Milestones (incremental — each independently runnable)

- **M0 — Scaffold:** create-tauri-app (Svelte+TS), add `pdfium-render` + `rusqlite` + `blake3`,
  bundle `pdfium.dll`, one hello-world command, CI skeleton (fmt/clippy/test).
- **M1 — Open & view (center only):** `open_document` (hash, metadata, text-layer detect), render
  protocol, virtualized continuous-scroll center viewer + zoom. Open button + drag-drop.
- **M2 — Thumbnails + right viewer:** virtualized strip; hover overlay Center/Right buttons assign a
  page to a viewer store; wire the independent right viewer.
- **M3 — Highlighting:** selection overlay → `add_highlight` (rects via PDFium) → persist → draw
  rects; no-text-layer banner.
- **M4 — Notes panel:** list highlights, Center/Right navigate, delete.
- **M5 — Test coverage + packaging:** fill the 4 test layers, fixtures, then `cargo tauri build`.

---

## Test strategy (4 layers, per CLAUDE.md)

1. **Rust unit** — hash determinism; text-layer detection; rect-for-char-range on a known fixture;
   highlight serde; SQLite CRUD against an in-memory DB.
2. **Tauri command integration** — `open_document` / `add` / `list` / `delete` against a small
   fixture PDF + temp DB.
3. **Svelte component (Vitest + @testing-library/svelte)** — center/right store independence;
   thumbnail hover overlay routes to the correct viewer; highlight rects scale with zoom; notes list
   renders/navigates/deletes.
4. **E2E (Playwright via tauri-driver)** — open → scroll → select text → highlight → reopen (same
   hash) → highlight persists → delete.

**Fixtures (`fixtures/`):** small text PDF, scanned/image-only (no text), encrypted, and a
rotated/mixed-size PDF.

---

## Verification (end-to-end)
1. `cargo tauri dev` → confirm `pdfium.dll` loads (no native-lib error).
2. Open the text fixture (button **and** drag-drop) → continuous scroll + zoom in center viewer.
3. Hover a thumbnail → click **Right** → page loads in the right viewer; scroll both — confirm independence.
4. Select a sentence → it highlights and appears in the Notes panel.
5. Close and reopen the same file → highlight persists (hash match). Delete it from a viewer and from Notes.
6. Open the scanned fixture → highlighting disabled + banner; viewing still works.
7. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npm run test`, E2E suite all green.
