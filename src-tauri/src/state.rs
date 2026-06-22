use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Mutex, OnceLock};

use lru::LruCache;
use pdfium_render::prelude::{Pdfium, PdfDocument};

#[derive(Clone, Debug, serde::Serialize)]
pub struct PageSize {
    pub width_pts: f32,
    pub height_pts: f32,
}

pub struct OpenDoc {
    pub path: String,
    pub page_count: u32,
    pub has_text_layer: bool,
    pub page_sizes: Vec<PageSize>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct RenderKey {
    pub doc_id: String,
    pub page: u32,
    /// scale × 100 — e.g. 100 = 1.0×, 150 = 1.5×, 20 = 0.2× (thumbnail)
    pub scale_pct: u32,
}

/// Pdfium is a C-library wrapper that is not Send by default.
/// We gate every access behind a Mutex, so Send + Sync is safe.
pub struct PdfiumHandle(pub Pdfium);
unsafe impl Send for PdfiumHandle {}
unsafe impl Sync for PdfiumHandle {}

impl std::ops::Deref for PdfiumHandle {
    type Target = Pdfium;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The single, process-wide PDFium binding. Stored in a `static` so loaded
/// `PdfDocument`s can borrow it for `'static` and be cached across calls
/// (PDFium is never dropped — the library stays mapped until the process exits).
pub static PDFIUM: OnceLock<PdfiumHandle> = OnceLock::new();

/// A parsed PDF kept open in memory so we don't re-parse the file on every
/// page render. Borrows the `'static` PDFIUM binding above.
/// PDFium is not thread-safe, so every access is serialised behind `AppState.pdf_cache`.
pub struct DocHandle(pub PdfDocument<'static>);
unsafe impl Send for DocHandle {}
unsafe impl Sync for DocHandle {}

pub struct AppState {
    pub open_docs: Mutex<HashMap<String, OpenDoc>>,
    /// Live, parsed documents kept open. This mutex is also the single
    /// serialisation point for ALL PDFium calls (load / render / text).
    pub pdf_cache: Mutex<HashMap<String, DocHandle>>,
    /// PNG-encoded page images keyed by (doc_id, page, scale_pct).
    pub render_cache: Mutex<LruCache<RenderKey, Vec<u8>>>,
    pub db: Mutex<rusqlite::Connection>,
}

impl AppState {
    pub fn new(db: rusqlite::Connection) -> Self {
        Self {
            open_docs: Mutex::new(HashMap::new()),
            pdf_cache: Mutex::new(HashMap::new()),
            render_cache: Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())),
            db: Mutex::new(db),
        }
    }
}
