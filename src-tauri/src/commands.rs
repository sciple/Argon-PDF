use pdfium_render::prelude::*;
use serde::Serialize;
use std::io::Cursor;

use crate::error::AppError;
use crate::state::{AppState, OpenDoc, PageSize, RenderKey};
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

fn render_page_png(
    pdfium: &Pdfium,
    path: &str,
    page_index: u32,
    scale_pct: u32,
) -> Result<Vec<u8>, AppError> {
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| {
            let msg = e.to_string();
            if msg.to_lowercase().contains("password") {
                AppError::NeedsPassword
            } else {
                AppError::Pdf(msg)
            }
        })?;

    let page = doc
        .pages()
        .get(page_index as u16)
        .map_err(|e| AppError::Pdf(e.to_string()))?;

    let base_dpi: f32 = 96.0;
    let scale = scale_pct as f32 / 100.0;
    let target_width = ((page.width().value / 72.0) * base_dpi * scale) as i32;
    let target_height = ((page.height().value / 72.0) * base_dpi * scale) as i32;

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

    // Return cached info if already loaded
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

    // Load and extract all data inside a block so pdfium_guard is dropped before DB write
    let (page_count, has_text_layer, page_sizes) = {
        let pdfium_guard = state.pdfium.lock().unwrap();
        let pdfium = pdfium_guard
            .as_ref()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;

        let doc = pdfium
            .load_pdf_from_file(&path, None)
            .map_err(|e| {
                let msg = e.to_string();
                if msg.to_lowercase().contains("password") {
                    AppError::NeedsPassword
                } else {
                    AppError::Pdf(msg)
                }
            })?;

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

        // Detect text layer by checking the first few pages
        let has_text_layer = (0..page_count.min(5)).any(|idx| {
            doc.pages()
                .get(idx as u16)
                .ok()
                .map(|p| {
                    p.text()
                        .ok()
                        .map(|t| t.chars().len() > 0)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        });

        // doc and pdfium_guard drop here
        (page_count, has_text_layer, page_sizes)
    };

    // Persist document record
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
    let path = {
        let docs = state.open_docs.lock().unwrap();
        docs.get(&doc_id)
            .ok_or_else(|| AppError::DocNotFound(doc_id.clone()))?
            .path
            .clone()
    };

    let chars = {
        let pdfium_guard = state.pdfium.lock().unwrap();
        let pdfium = pdfium_guard
            .as_ref()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;

        let doc = pdfium
            .load_pdf_from_file(&path, None)
            .map_err(|e| AppError::Pdf(e.to_string()))?;

        let page = doc
            .pages()
            .get(page_index as u16)
            .map_err(|e| AppError::Pdf(e.to_string()))?;

        let text_page = match page.text() {
            Ok(t) => t,
            Err(_) => return Ok(TextLayout { chars: vec![] }),
        };

        let char_count = text_page.chars().len();
        let mut chars = Vec::with_capacity(char_count);
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
        // doc, text_page, page, pdfium_guard all dropped here
        chars
    };

    Ok(TextLayout { chars })
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

    let path = {
        let docs = state.open_docs.lock().unwrap();
        docs.get(&doc_id)
            .ok_or_else(|| AppError::DocNotFound(doc_id.clone()))?
            .path
            .clone()
    };

    let (rects, excerpt) = {
        let pdfium_guard = state.pdfium.lock().unwrap();
        let pdfium = pdfium_guard
            .as_ref()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;

        let doc = pdfium
            .load_pdf_from_file(&path, None)
            .map_err(|e| AppError::Pdf(e.to_string()))?;

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
                    // Merge consecutive chars on the same line into one rect
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
        // doc, text_page, page, pdfium_guard dropped here
        (rects, excerpt)
    };

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
pub fn delete_highlight(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
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

    let path = {
        let docs = state.open_docs.lock().unwrap();
        docs.get(doc_id)
            .ok_or_else(|| AppError::DocNotFound(doc_id.to_string()))?
            .path
            .clone()
    };

    let png = {
        let pdfium_guard = state.pdfium.lock().unwrap();
        let pdfium = pdfium_guard
            .as_ref()
            .ok_or_else(|| AppError::Pdf("PDFium library not loaded".into()))?;
        render_page_png(&pdfium.0, &path, page_index, scale_pct)?
        // pdfium_guard dropped here
    };

    state.render_cache.lock().unwrap().put(key, png.clone());
    Ok(png)
}
