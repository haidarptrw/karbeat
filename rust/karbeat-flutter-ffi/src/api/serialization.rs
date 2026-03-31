use std::path::Path;

use karbeat_core::{
    audio::render_state::broadcast_plugin_state_loading, commands::AudioCommand, context::utils::send_audio_command, core::file_manager::project_loader::{load_karbeat_project, save_karbeat_project}, lock::{get_app_read, get_app_write}
};

use crate::broadcast_state_change;

pub fn save_project(path_name: &str) -> Result<(), String> {
    {
        let app = get_app_read();
        save_karbeat_project(Path::new(path_name), &app).map_err(|e| e.to_string())?;
    }

    log::info!("Successfully saved project to {}", path_name);
    Ok(())
}

pub fn load_project(path_name: &str) -> Result<crate::api::project::UiApplicationState, String> {
    
    let loaded_app = load_karbeat_project(Path::new(path_name)).map_err(|e| e.to_string())?;

    // HALT THE ENGINE
    send_audio_command(AudioCommand::StopAndReset);

    let ui_state = {
        let mut app = get_app_write();
        *app = loaded_app.clone();
        crate::api::project::UiApplicationState::from(loaded_app)
    };

    broadcast_state_change();
    broadcast_plugin_state_loading();

    log::info!("Successfully load the project {}", path_name);
    Ok(ui_state)
}
