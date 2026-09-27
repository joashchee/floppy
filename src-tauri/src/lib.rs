mod amiga;
mod cd;
mod cli;
mod commands;
mod discs;
mod documents;
mod dos;
#[cfg(test)]
mod e2e;
mod emulator;
mod findings;
mod handlers;
mod known_files;
mod library;
mod media;
mod request;
mod sha1;
mod mac;
#[cfg(test)]
mod testutil;
mod verify;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use commands::{AppState, StartupImport};
use library::Library;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Info and up: Trace/Debug floods the terminal with windowing
        // (tao) events.
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to resolve app data dir");
            let library = Library::new(app_data_dir.join("library"));
            let args: Vec<String> = std::env::args().skip(1).collect();
            let startup = match cli::parse(&args) {
                cli::Cli::None => None,
                cli::Cli::Import { os, path } => Some(match library.import(os, &path) {
                    Ok(a) => StartupImport { app: Some(a), document: None, error: None },
                    Err(e) => StartupImport { app: None, document: None, error: Some(e) },
                }),
                cli::Cli::Open { os, path } => Some(match library.import_document(os, &path) {
                    Ok(d) => StartupImport { app: None, document: Some(d), error: None },
                    Err(e) => StartupImport { app: None, document: None, error: Some(e) },
                }),
                cli::Cli::Error(e) => Some(StartupImport { app: None, document: None, error: Some(e) }),
            };
            let media = Arc::new(media::MediaWatch::default());
            #[cfg(target_os = "macos")]
            {
                use tauri::Emitter;
                let handle = app.handle().clone();
                media::watch(media.clone(), move |list| {
                    let _ = handle.emit("old-media-changed", list);
                });
            }
            app.manage(media);
            app.manage(request::Opened::default());
            app.manage(AppState {
                library,
                running: Arc::new(Mutex::new(HashMap::new())),
                startup: Mutex::new(startup),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::import_app,
            commands::remove_app,
            commands::set_program,
            commands::rename_app,
            commands::app_folder,
            commands::guest_statuses,
            commands::set_system_file,
            commands::set_guest_model,
            commands::locate_emulator,
            commands::write_missing_list,
            commands::import_setup_files,
            commands::setup_sources,
            commands::add_setup_report,
            commands::import_from_downloads,
            commands::import_files_disc,
            commands::record_verification,
            commands::handler_tests,
            commands::findings_summary,
            commands::export_findings,
            commands::set_app_errors,
            commands::set_app_share_errors,
            commands::forget_handler_tests,
            commands::write_wanted_apps,
            commands::import_apps_disc,
            commands::setup_tracking,
            commands::forget_ignored_files,
            commands::ask_again,
            commands::running_apps,
            commands::take_startup_import,
            commands::launch_app,
            commands::open_document,
            commands::import_item,
            commands::list_documents,
            commands::document_openers,
            commands::remove_document,
            commands::set_app_opens,
            commands::set_app_identity,
            commands::set_favorite_app,
            commands::handler_catalog,
            commands::library_file,
            commands::export_file,
            commands::old_media,
            commands::dismiss_media,
            commands::copy_old_media,
            commands::diskette_running,
            commands::request_summary,
            commands::ask_diskette,
            commands::is_burn_disc,
            commands::import_disc,
            commands::take_opened_files,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            // A file opened with Floppy: Diskette sending back a disc it
            // made for a request (request.rs), or Open With in Finder.
            // Queued for the window, which may not be listening yet.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = event {
                use tauri::Emitter;
                let paths: Vec<std::path::PathBuf> = urls.iter().filter_map(|u| u.to_file_path().ok()).collect();
                if let (false, Some(opened)) = (paths.is_empty(), handle.try_state::<request::Opened>()) {
                    opened.0.lock().unwrap_or_else(|p| p.into_inner()).extend(paths);
                    let _ = handle.emit("files-opened", ());
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (handle, event);
        });
}
