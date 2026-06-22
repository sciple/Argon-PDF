// Reads a PDF file from disk and returns its raw bytes to the frontend.
// `tauri::ipc::Response` transfers the bytes as an ArrayBuffer (efficient),
// instead of serialising a Vec<u8> as a JSON number array.
#[tauri::command]
fn read_pdf(path: String) -> Result<tauri::ipc::Response, String> {
    std::fs::read(&path)
        .map(tauri::ipc::Response::new)
        .map_err(|e| format!("Failed to read '{path}': {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![read_pdf])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
