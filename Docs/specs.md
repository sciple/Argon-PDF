# Argon-PDF
a simple pdf reader for windows 11 developed in RUST-TAURI.
THis is a  native app that opens a pdf file and displays it for reading. the goal of the app is to allow the user to read a page (and scroll through the pages normally), while also keeping another page viewer open from the same pdf file.
there are three main panels organized vertically:
left panel is the preview of the pages - it is a scrollable window where the user can scroll through the pages and click them to visualize them
the central panel is the page viewer (the user can zoom in-out to read the page)
the third panel on the right is another page viewer that allows the user to access (i.e. view) a second page of the same pdf at the same time (basically it is a duplicate viewer, but with its own scroll bar so that the two viewers are independent).
there is no save/edit of the text. Only highlighting of the text is necessary.
When the user highlights parts of the text (sentences/paragraphs). Every highlighted part of the text will be logged permanently, so that when the user clicks a button `notes` a window is opened (or a panel, we can discuss together the possible options), it can click on the notes and then visualize the page where the note was taken.
The user must be able also to delete (or remove) the highlights

Very important point: you must draft a test strategy so that I can confidently add/remove features from the app during its development, so that I am not afaraid of breaking or introducing any regression. 

## Design Decisions

These decisions resolve open questions in the spec above and keep the app lightweight and streamlined.

1. **Text-snapped highlighting.** Highlights snap to the selectable text of the PDF (the text layer), so the user selects sentences/paragraphs rather than drawing free-form boxes. A single highlight is stored as a *list* of rectangles (a multi-line selection produces multiple rects) plus the page number. Rectangles are stored in **PDF-coordinate space on the unrotated page** (not pixels) so they survive zoom, window resize, and DPI changes.

2. **Central SQLite persistence.** All highlights are stored permanently in a single SQLite database in `%APPDATA%` (not in per-file JSON sidecars). Documents are keyed by **content hash** (with the last-known file path kept as a hint), so highlights survive moving or renaming the PDF. Writes are transactional so a crash mid-write cannot corrupt the store.

3. **Highlighting requires a text layer.** If the opened PDF has no selectable text (e.g. a scanned, image-only document), highlighting is **disabled** and the app displays a clear message (e.g. "This document has no selectable text — highlighting is unavailable"). Viewing, scrolling, zoom, and the dual viewer still work normally. OCR is explicitly out of scope.

## Revised scope (current — supersedes parts of the above)

After early iterations, the app was re-scoped around its real use case — **working maths
exercises while checking the solution on a second page** — and around a hard requirement that
text be **crisp like Firefox/Adobe**.

- **Rendering pivoted to PDF.js (`pdfjs-dist`)**, rendering pages to `<canvas>` at the device
  pixel ratio (the same engine Firefox uses). The earlier "render to PNG in Rust (PDFium) and
  show in `<img>`" pipeline was removed because it was persistently blurry at fractional Windows
  display scaling. The Rust backend is now just a file-bytes reader.
- **Dropped: notes panel, text highlighting, and persistence (SQLite).** With those gone, the
  PDFium / SQLite / custom-image-protocol backend was deleted. (Design Decisions 1–3 above are
  retained as history but no longer implemented.)
- **Kept:** two independent page viewers (Main + Side), thumbnail strip, draggable divider,
  light theme, open via dialog + drag-drop.
- **Added:** a **page-number box** per viewer to jump straight to a page, **thumbnail → Main/Side**
  to open a page in either viewer, and fit-width-by-default zoom (manual zoom on +/−).
