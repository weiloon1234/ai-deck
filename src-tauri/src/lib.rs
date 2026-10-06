pub mod app_core;
pub mod cli_adapters;
pub mod codex_model_analyzer;
pub mod commands;
pub mod credential_store;
pub mod deployment_reconciler;
pub mod deployment_service;
pub mod enabled_endpoint_service;
pub mod error;
pub mod hugging_face_client;
pub mod model_analysis;
pub mod model_catalog;
pub mod model_import_service;
mod process_tree;
pub mod runpod_client;
pub mod runtime_client;
pub mod session_service;
pub mod state_store;
pub mod terminal_process;
pub mod types;

use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager};

pub fn run() {
    let app = tauri::Builder::default()
        .manage(commands::ExitControl::default())
        .setup(|app| {
            let store = Arc::new(state_store::StateStore::open(app.path().app_data_dir()?)?);
            let credentials: Arc<dyn credential_store::CredentialStore> =
                Arc::new(credential_store::OsCredentialStore);
            let handle = app.handle().clone();
            let core = Arc::new(app_core::AppCore {
                store,
                provider: Arc::new(runpod_client::RunpodClient::new(credentials.clone())?),
                runtime: Arc::new(runtime_client::RuntimeClient::new(credentials.clone())?),
                credentials,
                terminals: Arc::new(terminal_process::TerminalManager::default()),
                operations: tokio::sync::Mutex::new(()),
                model_imports: Default::default(),
                notify: Arc::new(move |event, data| {
                    let _ = handle.emit(event, data);
                }),
            });
            app.manage(core.clone());
            tauri::async_runtime::spawn(async move {
                loop {
                    let _ = core.background_tick().await;
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::save_runpod_key,
            commands::remove_runpod_key,
            commands::save_settings,
            commands::detect_clis,
            commands::choose_executable,
            commands::choose_project,
            commands::remove_project,
            commands::hardware,
            commands::provision,
            commands::reconcile,
            commands::enable,
            commands::finish,
            commands::confirm_absent_creation,
            commands::launch_session,
            commands::resume_session,
            commands::close_session,
            commands::rename_session,
            commands::terminal_input,
            commands::terminal_resize,
            commands::terminal_replay,
            commands::import_catalog,
            commands::import_hugging_face_model,
            commands::cancel_model_import,
            commands::refresh_model_prices,
            commands::remove_imported_model,
            commands::export_diagnostics,
            commands::open_external,
            commands::request_quit
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("Unable to start AI Deck. Existing state was not reset.");
    app.run(|handle, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            if !handle
                .state::<commands::ExitControl>()
                .0
                .load(Ordering::Acquire)
            {
                api.prevent_exit();
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = handle.emit("quit-requested", ());
            }
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        _ => (),
    });
}
