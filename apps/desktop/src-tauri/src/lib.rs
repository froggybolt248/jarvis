pub mod app_state;
pub mod commands;
pub mod core;

use app_state::AppState;
use tauri::Manager;

use crate::core::memory::embedder;
use crate::core::memory::watcher::VaultEventKind;
use crate::core::memory::Vault;

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

            // Full re-index on startup, then keep the vault index fresh as
            // files change. Both run in the background: a slow or failing
            // embeddings model (e.g. not pulled yet) must never block the
            // window from opening.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();

                // A std `RwLockReadGuard` is `!Send` and can't be held across
                // an `.await` in a task the runtime may move between
                // threads. So rather than borrowing the shared `Vault`, copy
                // out its root path (guard dropped immediately after) and
                // open a fresh, owned `Vault` for this long-lived background
                // task. `Vault::open` is idempotent and never overwrites
                // existing files.
                let vault_root = state.vault.read().expect("vault lock poisoned").root().to_path_buf();
                let vault = match Vault::open(&vault_root) {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("failed to open vault for background indexing: {err:#}");
                        return;
                    }
                };

                if let Err(err) = embedder::index_vault(&state.db, &state.provider, &vault).await {
                    eprintln!("startup vault indexing failed: {err:#}");
                }

                // Best-effort calendar sync: fine if Google isn't connected
                // yet (errors), must never block startup.
                if let Err(err) = crate::core::google::sync::sync_calendar(&state.db, "primary").await {
                    eprintln!("startup calendar sync failed: {err:#}");
                }

                let (watcher, mut rx) = match vault.watch() {
                    Ok(pair) => pair,
                    Err(err) => {
                        eprintln!("failed to start vault watcher: {err:#}");
                        return;
                    }
                };
                // Keep the watcher alive for as long as this task runs.
                let _watcher = watcher;

                while let Some(event) = rx.recv().await {
                    let rel = match event.path.strip_prefix(vault.root()) {
                        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                        Err(_) => continue,
                    };

                    match event.kind {
                        VaultEventKind::Created | VaultEventKind::Modified => match vault.read(&rel) {
                            Ok(content) => {
                                let updated_at = chrono::Utc::now().to_rfc3339();
                                if let Err(err) = embedder::index_source(
                                    &state.db,
                                    &state.provider,
                                    &rel,
                                    &content,
                                    &updated_at,
                                )
                                .await
                                {
                                    eprintln!("failed to re-index {rel}: {err:#}");
                                }
                            }
                            Err(err) => eprintln!("failed to read changed vault file {rel}: {err:#}"),
                        },
                        VaultEventKind::Removed => {
                            if let Err(err) = state.db.delete_chunks_for_source(&rel) {
                                eprintln!("failed to delete chunks for removed {rel}: {err:#}");
                            }
                        }
                    }
                }
            });

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
            // Agent chat
            commands::agent::chat,
            // Domain reads (diet, gym, study, calendar)
            commands::diet::diet_logs_for_date,
            commands::diet::diet_current_targets,
            commands::gym::gym_recent_sessions,
            commands::gym::gym_sets_for_exercise,
            commands::study::study_due_cards,
            commands::calendar::calendar_events_between,
            commands::calendar::calendar_sync_now,
            commands::calendar::calendar_create_event,
            // Knowledge (vault notes)
            commands::knowledge::vault_list_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
