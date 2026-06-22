mod commands;
mod error;
mod state;
mod store;

use pdfium_render::prelude::*;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load pdfium.dll once into the process-wide PDFIUM static.
    // Search order: next to exe, current dir, system PATH.
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();

    match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&exe_dir))
        .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")))
        .or_else(|_| Pdfium::bind_to_system_library())
    {
        Ok(binding) => {
            let _ = state::PDFIUM.set(state::PdfiumHandle(Pdfium::new(binding)));
        }
        Err(_) => {
            eprintln!(
                "WARNING: pdfium.dll not found — PDF rendering will be unavailable. \
                 Place pdfium.dll next to the executable."
            );
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            // Open (or create) the SQLite database in %APPDATA%/Argon-PDF/argon.db
            let app_data_dir = app.path().app_data_dir()
                .expect("could not resolve AppData dir");
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("argon.db");

            let db = rusqlite::Connection::open(&db_path)
                .expect("failed to open SQLite database");
            store::run_migrations(&db).expect("database migration failed");

            let app_state = AppState::new(db);
            app.manage(app_state);
            Ok(())
        })
        .register_uri_scheme_protocol("argon", |ctx, request| {
            // URL format: argon://render/{doc_id}/{page_index}/{scale_pct}
            let uri = request.uri().to_string();
            let response = handle_render_protocol(ctx.app_handle(), &uri);
            match response {
                Ok(bytes) => tauri::http::Response::builder()
                    .header("Content-Type", "image/png")
                    .header("Cache-Control", "max-age=3600")
                    .status(200)
                    .body(bytes)
                    .unwrap(),
                Err(e) => tauri::http::Response::builder()
                    .status(500)
                    .body(e.to_string().into_bytes())
                    .unwrap(),
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_document,
            commands::get_page_text_layout,
            commands::add_highlight,
            commands::list_highlights,
            commands::delete_highlight,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_render_protocol(
    app: &tauri::AppHandle,
    uri: &str,
) -> Result<Vec<u8>, error::AppError> {
    // The URL ends in `.../render/{doc_id}/{page}/{scale}`, but its scheme/host
    // differ by platform (Windows: `http://argon.localhost/render/...`,
    // macOS/Linux: `argon://localhost/render/...`). Parse robustly by taking the
    // last three path segments, which are always doc_id / page / scale.
    let clean = uri.split(['?', '#']).next().unwrap_or(uri);
    let segments: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() < 3 {
        return Err(error::AppError::Pdf(format!("Invalid render URL: {uri}")));
    }

    let n = segments.len();
    let doc_id = segments[n - 3];
    let page_index: u32 = segments[n - 2]
        .parse()
        .map_err(|_| error::AppError::Pdf("Invalid page index".into()))?;
    let scale_pct: u32 = segments[n - 1]
        .parse()
        .map_err(|_| error::AppError::Pdf("Invalid scale".into()))?;

    let state = app.state::<AppState>();
    commands::render_page_cached(&state, doc_id, page_index, scale_pct)
}
