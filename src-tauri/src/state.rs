use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;
use pdfium_render::prelude::Pdfium;

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

pub struct AppState {
    /// None when pdfium.dll was not found at startup.
    pub pdfium: Mutex<Option<PdfiumHandle>>,
    pub open_docs: Mutex<HashMap<String, OpenDoc>>,
    /// PNG-encoded page images keyed by (doc_id, page, scale_pct).
    pub render_cache: Mutex<LruCache<RenderKey, Vec<u8>>>,
    pub db: Mutex<rusqlite::Connection>,
}

impl AppState {
    pub fn new(pdfium: Option<PdfiumHandle>, db: rusqlite::Connection) -> Self {
        Self {
            pdfium: Mutex::new(pdfium),
            open_docs: Mutex::new(HashMap::new()),
            render_cache: Mutex::new(LruCache::new(NonZeroUsize::new(128).unwrap())),
            db: Mutex::new(db),
        }
    }
}
