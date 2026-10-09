mod commands;
mod store;
mod theme;

use std::sync::Arc;

use tauri::Manager;

use store::local::Local;
use store::sync::Sync;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            theme::watch(app.handle().clone());

            let path = dirs::data_dir().ok_or("no data directory")?.join("oche/oche.db");
            let local = tauri::async_runtime::block_on(Local::open(&path))
                .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
            let sync = Arc::new(Sync::new(Arc::new(local), store::remote::load_config()));
            sync.clone().spawn(app.handle().clone());
            app.manage(sync);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            theme::get_theme,
            commands::players_list,
            commands::player_named,
            commands::game_save,
            commands::game_delete,
            commands::game_load,
            commands::games_list,
            commands::player_stats,
            commands::remote_get,
            commands::remote_test,
            commands::remote_connect,
            commands::remote_disconnect,
            commands::remote_import,
            commands::sync_status,
            commands::sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
