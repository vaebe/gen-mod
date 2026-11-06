mod cloud;
mod games;
mod local_resources;
mod project_context;
mod project_lifecycle;
mod storage;
mod pi;
mod config;
mod changes;
mod files;
mod source_files;
mod skills;
mod artifacts;
mod install;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(cloud::auth::CloudState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(storage::mods::StorageState::default())
        .manage(config::ConfigurationState::default())
        .manage(pi::bridge::PiState::default())
        .manage(pi::workspace::Workspaces::default())
        .manage(install::installer::InstallState::default())
        .setup(|app| { storage::database::initialize(app.handle()).map_err(std::io::Error::other)?; Ok(()) })
        .invoke_handler({ let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
            games::list_games, games::read_game_cover, games::read_game_icon,
            cloud::auth::cloud_register,cloud::auth::cloud_login,cloud::auth::cloud_logout,cloud::auth::cloud_me,cloud::auth::cloud_initialize,
            cloud::catalog::cloud_check_updates,cloud::catalog::cloud_confirm_updates,cloud::catalog::cloud_games,
            project_lifecycle::set_mod_archived,project_lifecycle::list_archived_mods,project_lifecycle::delete_mod,
            knowledge::get_long_term_memory, knowledge::list_long_term_memories,
            knowledge::save_long_term_memory_entry, knowledge::set_long_term_memory_status,
            source_files::pick_source_import_files, source_files::import_source_files,
            source_files::move_source_file, source_files::create_source_directory,
            assets::list_legacy_assets,assets::restore_legacy_asset,
            game_inspection::inspect_supported_game, game_inspection::list_inspections, game_inspection::apply_inspection,
            verification::list_verification, verification::edit_feedback_draft, verification::confirm_feedback_draft, verification::reject_feedback_draft,
            artifacts::delivery_metadata::get_delivery_metadata,
            project_context::get_project_context,
            storage::mods::create_mod, storage::mods::list_mods,
            storage::mods::get_mod, storage::mods::rename_mod,
            storage::mods::save_mod_draft,
            pi::bridge::save_session_thinking, pi::bridge::select_session_model, storage::repository::read_session_metadata, storage::repository::list_migration_errors,
            storage::repository::list_recovery_issues,
            pi::bridge::create_mod_session, pi::bridge::connect_pi, pi::bridge::send_pi_command,
            pi::bridge::disconnect_pi, pi::bridge::start_mod_task,
            pi::bridge::reserve_pi_subscription, pi::bridge::read_pi_runtime, pi::submissions::read_submission_result,
            pi::bridge::steer_mod_task, storage::repository::list_steering_inputs,
            config::get_configuration, config::save_configuration, config::open_configuration_directory,
            changes::list_task_operations, changes::list_task_changes, changes::get_task_file_diff,
            files::reveal_source_file, files::open_message_link,
            files::list_source_directory, files::read_source_file,
            files::read_source_image,
            pi::sessions::list_mod_sessions, pi::sessions::read_session_history,
            pi::sessions::list_mod_tasks,
            pi::logs::read_session_runtime_logs,
            pi::sessions::rename_mod_session, pi::bridge::delete_mod_session,
            skills::registry::list_skills, skills::registry::import_skill,
            skills::registry::set_skill_enabled,
            skills::registry::read_skill_content, skills::registry::reveal_skill_file,
            install::installer::preview_install, install::installer::install_version,
            files::pick_directory, files::open_mod_directory,
            files::export_version, files::open_task_logs,
            artifacts::registry::delete_version];
            handler
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) { pi::bridge::shutdown(app); }
        });
}

mod verification;

mod knowledge;

mod game_inspection;

mod assets;
