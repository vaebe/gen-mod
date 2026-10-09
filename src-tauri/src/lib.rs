mod storage;
mod pi;
mod config;
mod changes;
mod files;
mod skills;
mod artifacts;
mod install;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(storage::mods::StorageState::default())
        .manage(pi::bridge::PiState::default())
        .manage(install::installer::InstallState::default())
        .invoke_handler(tauri::generate_handler![
            storage::mods::create_mod, storage::mods::list_mods,
            storage::mods::get_mod, storage::mods::rename_mod,
            storage::mods::save_mod_draft,
            pi::bridge::connect_pi, pi::bridge::send_pi_command,
            pi::bridge::disconnect_pi, pi::bridge::start_mod_task,
            config::get_configuration, config::open_configuration_directory,
            changes::list_task_changes, changes::get_task_file_diff,
            files::reveal_source_file, files::open_message_link,
            pi::sessions::list_mod_sessions, pi::sessions::read_session_history,
            pi::sessions::list_mod_tasks,
            pi::logs::read_session_runtime_logs,
            pi::sessions::rename_mod_session, pi::bridge::delete_mod_session,
            skills::registry::list_skills, skills::registry::import_skill,
            skills::registry::set_skill_enabled,
            install::installer::preview_install, install::installer::install_version,
            files::pick_directory, files::open_mod_directory,
            files::export_version, files::open_task_logs,
            artifacts::registry::delete_version])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) { pi::bridge::shutdown(app); }
        });
}
