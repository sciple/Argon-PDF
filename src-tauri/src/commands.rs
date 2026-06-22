use pdfium_render::prelude::*;
use serde::Serialize;
use std::io::Cursor;

use crate::error::AppError;
use crate::state::{AppState, DocHandle, OpenDoc, PageSize, RenderKey, PDFIUM};
use crate::store::{self, Highlight, HighlightRect};

// ── Types returned to the frontend ──────────────────────────────────────────

#[derive(Serialize)]
pub struct DocumentInfo {
    pub doc_id: String,
    pub page_count: u32,
    pub has_text_layer: bool,
    pub page_sizes: Vec<PageSize>,
}

#[derive(Serialize)]
pub struct CharRect {
    pub char_index: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub char_val: String,
}

#[derive(Serialize)]
pub struct TextLayout {
    pub chars: Vec<CharRect>,
}

/// Largest page-bitmap dimension we will render, in pixels. Caps memory and
/// PNG-encode time at very high zoom × devicePixelRatio combinations.
const MAX_RENDER_DIM: i32 = 4096;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn compute_doc_id(path: &str) -> Result<String, AppError> {
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Err(AppError::InvalidPdf);
    }
    if !bytes.starts_with(b"%PDF") {
        return Err(AppError::InvalidPdf);
    }
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn map_pdf_err(e: PdfiumError) -> AppError {
    let msg = e.to_string();
    if msg.to_lowercase().contains("password") {
        AppError::NeedsPassword
    } else {
        AppError::Pdf(msg)
    }
}

/// Run `f` against a parsed document, loading and caching it on first use.
/// Holding the `pdf_cache` lock for the whole call serialises all PDFium access
/// (PDFium is not thread-safe).
fn with_document<F, R>(state: &AppState, doc_id: &str, f: F) -> Result<R, AppError>
where
    F: FnOnce(&PdfDocument) -> Result<R, AppError>,
{
    let path = {
        let docs = state.open_docs.lock().unwrap();
        docs.get(doc_id)
            .ok_or_else(|| AppError::DocNotFound(doc_id.to_string()))?
            .path
            .clone()
    };

    let mut cache = state.pdf_cache.lock().unwrap();
    if !cache.contains_key(doc_id) {
        let pdfium = PDFIUM
            .get()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;
        let doc = pdfium.load_pdf_from_file(&path, None).map_err(map_pdf_err)?;
        cache.insert(doc_id.to_string(), DocHandle(doc));
    }

    let handle = cache.get(doc_id).unwrap();
    f(&handle.0)
}

fn render_page_png(doc: &PdfDocument, page_index: u32, scale_pct: u32) -> Result<Vec<u8>, AppError> {
    let page = doc
        .pages()
        .get(page_index as u16)
        .map_err(|e| AppError::Pdf(e.to_string()))?;

    // scale_pct already folds in zoom × devicePixelRatio. 96 DPI base.
    let base_dpi: f32 = 96.0;
    let scale = scale_pct as f32 / 100.0;
    let mut target_width = ((page.width().value / 72.0) * base_dpi * scale) as i32;
    let mut target_height = ((page.height().value / 72.0) * base_dpi * scale) as i32;

    // Cap the longest edge to keep memory / encode time bounded.
    let longest = target_width.max(target_height);
    if longest > MAX_RENDER_DIM {
        let factor = MAX_RENDER_DIM as f32 / longest as f32;
        target_width = (target_width as f32 * factor) as i32;
        target_height = (target_height as f32 * factor) as i32;
    }
    target_width = target_width.max(1);
    target_height = target_height.max(1);

    let config = PdfRenderConfig::new()
        .set_target_width(target_width)
        .set_maximum_height(target_height);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| AppError::Pdf(e.to_string()))?;

    let img = bitmap.as_image();
    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| AppError::Pdf(e.to_string()))?;

    Ok(png_bytes)
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn open_document(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<DocumentInfo, AppError> {
    let doc_id = compute_doc_id(&path)?;

    // Return cached metadata if already open
    {
        let docs = state.open_docs.lock().unwrap();
        if let Some(doc) = docs.get(&doc_id) {
            return Ok(DocumentInfo {
                doc_id: doc_id.clone(),
                page_count: doc.page_count,
                has_text_layer: doc.has_text_layer,
                page_sizes: doc.page_sizes.clone(),
            });
        }
    }

    // Load the document once, compute metadata, and keep it open in the cache.
    let (page_count, has_text_layer, page_sizes) = {
        let pdfium = PDFIUM
            .get()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;

        let doc = pdfium.load_pdf_from_file(&path, None).map_err(map_pdf_err)?;

        let page_count = doc.pages().len() as u32;

        let mut page_sizes = Vec::with_capacity(page_count as usize);
        for idx in 0..page_count {
            let page = doc
                .pages()
                .get(idx as u16)
                .map_err(|e| AppError::Pdf(e.to_string()))?;
            page_sizes.push(PageSize {
                width_pts: page.width().value,
                height_pts: page.height().value,
            });
        }

        let has_text_layer = (0..page_count.min(5)).any(|idx| {
            doc.pages()
                .get(idx as u16)
                .ok()
                .map(|p| p.text().ok().map(|t| t.chars().len() > 0).unwrap_or(false))
                .unwrap_or(false)
        });

        // Keep the parsed document open for fast subsequent renders.
        state
            .pdf_cache
            .lock()
            .unwrap()
            .insert(doc_id.clone(), DocHandle(doc));

        (page_count, has_text_layer, page_sizes)
    };

    {
        let db = state.db.lock().unwrap();
        store::upsert_document(&db, &doc_id, &path, page_count)?;
    }

    let info = DocumentInfo {
        doc_id: doc_id.clone(),
        page_count,
        has_text_layer,
        page_sizes: page_sizes.clone(),
    };

    state.open_docs.lock().unwrap().insert(
        doc_id,
        OpenDoc { path, page_count, has_text_layer, page_sizes },
    );

    Ok(info)
}

#[tauri::command]
pub fn get_page_text_layout(
    doc_id: String,
    page_index: u32,
    state: tauri::State<'_, AppState>,
) -> Result<TextLayout, AppError> {
    with_document(&state, &doc_id, |doc| {
        let page = doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| AppError::Pdf(e.to_string()))?;

        let text_page = match page.text() {
            Ok(t) => t,
            Err(_) => return Ok(TextLayout { chars: vec![] }),
        };

        let char_count = text_page.chars().len();
        let mut chars = Vec::with_capacity(char_count as usize);
        for idx in 0..char_count {
            if let Ok(ch) = text_page.chars().get(idx) {
                if let Ok(bounds) = ch.loose_bounds() {
                    let c = ch.unicode_char().unwrap_or('\0');
                    chars.push(CharRect {
                        char_index: idx as u32,
                        x: bounds.left().value,
                        y: bounds.bottom().value,
                        width: bounds.right().value - bounds.left().value,
                        height: bounds.top().value - bounds.bottom().value,
                        char_val: c.to_string(),
                    });
                }
            }
        }
        Ok(TextLayout { chars })
    })
}

#[tauri::command]
pub fn add_highlight(
    doc_id: String,
    page_index: u32,
    start_char: u32,
    end_char: u32,
    color: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Highlight, AppError> {
    if start_char >= end_char {
        return Err(AppError::Pdf("Empty selection".into()));
    }

    let (rects, excerpt) = with_document(&state, &doc_id, |doc| {
        let page = doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| AppError::Pdf(e.to_string()))?;

        let text_page = page.text().map_err(|e| AppError::Pdf(e.to_string()))?;

        let mut rects: Vec<HighlightRect> = Vec::new();
        let mut excerpt_chars = String::new();

        for idx in start_char..end_char {
            if let Ok(ch) = text_page.chars().get(idx as usize) {
                if let Ok(bounds) = ch.loose_bounds() {
                    let r = HighlightRect {
                        x: bounds.left().value,
                        y: bounds.bottom().value,
                        width: bounds.right().value - bounds.left().value,
                        height: bounds.top().value - bounds.bottom().value,
                    };
                    if let Some(last) = rects.last_mut() {
                        let same_line = (last.y - r.y).abs() < 2.0 && r.x >= last.x;
                        if same_line {
                            let right = last.x + last.width;
                            if r.x + r.width > right {
                                last.width = (r.x + r.width) - last.x;
                            }
                            if let Some(c) = ch.unicode_char() {
                                excerpt_chars.push(c);
                            }
                            continue;
                        }
                    }
                    rects.push(r);
                }
                if let Some(c) = ch.unicode_char() {
                    excerpt_chars.push(c);
                }
            }
        }

        let excerpt: String = excerpt_chars.chars().take(120).collect();
        Ok((rects, excerpt))
    })?;

    let color = color.unwrap_or_else(|| "#FFFF00".to_string());
    let db = state.db.lock().unwrap();
    store::add_highlight(&db, &doc_id, page_index, start_char, end_char, &rects, &excerpt, &color)
}

#[tauri::command]
pub fn list_highlights(
    doc_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Highlight>, AppError> {
    let db = state.db.lock().unwrap();
    store::list_highlights(&db, &doc_id)
}

#[tauri::command]
pub fn delete_highlight(id: i64, state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.lock().unwrap();
    store::delete_highlight(&db, id)
}

// ── Render helper called by the protocol handler ──────────────────────────────

pub fn render_page_cached(
    state: &AppState,
    doc_id: &str,
    page_index: u32,
    scale_pct: u32,
) -> Result<Vec<u8>, AppError> {
    let key = RenderKey {
        doc_id: doc_id.to_string(),
        page: page_index,
        scale_pct,
    };

    if let Some(cached) = state.render_cache.lock().unwrap().get(&key) {
        return Ok(cached.clone());
    }

    let png = with_document(state, doc_id, |doc| render_page_png(doc, page_index, scale_pct))?;

    state.render_cache.lock().unwrap().put(key, png.clone());
    Ok(png)
}
