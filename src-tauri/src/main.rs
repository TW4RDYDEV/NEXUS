#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::Manager;
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let executable = std::env::current_exe()?;
            let beside = executable
                .parent()
                .ok_or_else(|| std::io::Error::other("Cannot locate application directory"))?;
            let root = if let Some(path) = std::env::var_os("NEXUS_DATA_DIR") {
                std::path::PathBuf::from(path)
            } else if beside.join("portable.flag").exists() {
                beside.join("data")
            } else {
                app.path().app_local_data_dir()?
            };
            let api = nexus_core::core::Api::new(root.clone()).map_err(std::io::Error::other)?;
            app.manage(std::sync::Mutex::new(api));
            let window = app
                .config()
                .app
                .windows
                .first()
                .ok_or_else(|| std::io::Error::other("Missing main window configuration"))?;
            tauri::WebviewWindowBuilder::from_config(app, window)?
                .data_directory(root.join("webview"))
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![nexus_core::commands::nexus])
        .run(tauri::generate_context!())
        .expect("NEXUS desktop runtime failed");
}
