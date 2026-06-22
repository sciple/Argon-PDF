use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub id: i64,
    pub doc_id: String,
    pub page_index: u32,
    pub start_char: u32,
    pub end_char: u32,
    pub rects: Vec<HighlightRect>,
    pub text_excerpt: String,
    pub color: String,
}

pub fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS documents (
            doc_id      TEXT PRIMARY KEY,
            last_path   TEXT NOT NULL,
            page_count  INTEGER NOT NULL,
            created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS highlights (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            doc_id       TEXT NOT NULL REFERENCES documents(doc_id) ON DELETE CASCADE,
            page_index   INTEGER NOT NULL,
            start_char   INTEGER NOT NULL,
            end_char     INTEGER NOT NULL,
            rects_json   TEXT NOT NULL,
            text_excerpt TEXT NOT NULL DEFAULT '',
            color        TEXT NOT NULL DEFAULT '#FFFF00',
            created_at   INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE INDEX IF NOT EXISTS idx_highlights_doc ON highlights(doc_id);
        ",
    )
    .map_err(AppError::from)
}

pub fn upsert_document(
    conn: &Connection,
    doc_id: &str,
    path: &str,
    page_count: u32,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO documents (doc_id, last_path, page_count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(doc_id) DO UPDATE SET
             last_path  = excluded.last_path,
             page_count = excluded.page_count,
             updated_at = unixepoch()",
        params![doc_id, path, page_count],
    )
    .map(|_| ())
    .map_err(AppError::from)
}

pub fn add_highlight(
    conn: &Connection,
    doc_id: &str,
    page_index: u32,
    start_char: u32,
    end_char: u32,
    rects: &[HighlightRect],
    text_excerpt: &str,
    color: &str,
) -> Result<Highlight, AppError> {
    let rects_json = serde_json::to_string(rects)
        .map_err(|e| AppError::Db(format!("JSON serialization failed: {e}")))?;

    conn.execute(
        "INSERT INTO highlights (doc_id, page_index, start_char, end_char, rects_json, text_excerpt, color)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![doc_id, page_index, start_char, end_char, rects_json, text_excerpt, color],
    )
    .map_err(AppError::from)?;

    let id = conn.last_insert_rowid();
    Ok(Highlight {
        id,
        doc_id: doc_id.to_string(),
        page_index,
        start_char,
        end_char,
        rects: rects.to_vec(),
        text_excerpt: text_excerpt.to_string(),
        color: color.to_string(),
    })
}

pub fn list_highlights(conn: &Connection, doc_id: &str) -> Result<Vec<Highlight>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, doc_id, page_index, start_char, end_char, rects_json, text_excerpt, color
         FROM highlights WHERE doc_id = ?1 ORDER BY page_index, start_char",
    )?;

    let rows = stmt.query_map(params![doc_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u32>(3)?,
            row.get::<_, u32>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;

    let mut highlights = Vec::new();
    for row in rows {
        let (id, doc_id, page_index, start_char, end_char, rects_json, text_excerpt, color) =
            row.map_err(AppError::from)?;
        let rects: Vec<HighlightRect> = serde_json::from_str(&rects_json)
            .map_err(|e| AppError::Db(format!("JSON parse failed: {e}")))?;
        highlights.push(Highlight {
            id,
            doc_id,
            page_index,
            start_char,
            end_char,
            rects,
            text_excerpt,
            color,
        });
    }
    Ok(highlights)
}

pub fn delete_highlight(conn: &Connection, id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM highlights WHERE id = ?1", params![id])
        .map(|_| ())
        .map_err(AppError::from)
}
