use tauri::AppHandle;

#[tauri::command]
pub fn get_kubo_path(app: AppHandle) -> Result<String, String> {
    let kubo_path = crate::helpers::paths::get_kubo_path(&app);
    Ok(kubo_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn hello_world(name: String) -> String {
    format!("Hello, {}! This is Planet Desktop.", name)
}