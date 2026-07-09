pub mod app_state;
pub mod commands;
pub mod core;

use app_state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance MUST be registered first so a second launch is
        // intercepted before any window is created.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.show();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let state = AppState::bootstrap(app_data_dir).map_err(|e| e.to_string())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ollama_health,
            commands::get_setting,
            commands::set_setting,
            commands::recent_feed,
            // Onboarding flow + vault location
            commands::onboarding::get_onboarding_state,
            commands::onboarding::set_onboarding_domains,
            commands::onboarding::complete_onboarding,
            commands::onboarding::get_default_vault_dir,
            commands::onboarding::set_vault_dir,
            // Ollama setup automation
            commands::ollama::ollama_detect,
            commands::ollama::ollama_recommended_models,
            commands::ollama::ollama_install,
            commands::ollama::ollama_ensure_running,
            commands::ollama::ollama_pull,
            // Google Calendar connection
            commands::google::google_connect,
            commands::google::google_status,
            commands::google::google_disconnect,
            commands::google::google_list_calendars,
            // ntfy phone push
            commands::notify::ntfy_get_config,
            commands::notify::ntfy_setup,
            commands::notify::ntfy_send_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
