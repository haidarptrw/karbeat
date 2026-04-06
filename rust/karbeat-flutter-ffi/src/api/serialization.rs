use karbeat_core::
    api::project_api
;
use crate::api::project::UiApplicationState;

/// Save the currrent project to path_name
pub fn save_project(path_name: &str) -> Result<(), String> {
    project_api::save_project(path_name).map_err(|e| e.to_string())?;
    
    log::info!("Successfully saved project to {}", path_name);
    Ok(())
}

/// Load the `.karbeat` project.
pub fn load_project(path_name: &str) -> Result<crate::api::project::UiApplicationState, String> {
    let ui_state = project_api::load_project(path_name, |app| {
        UiApplicationState::from(app.clone()) 
    }).map_err(|e| e.to_string())?;

    log::info!("Successfully loaded the project {}", path_name);
    Ok(ui_state)
}
