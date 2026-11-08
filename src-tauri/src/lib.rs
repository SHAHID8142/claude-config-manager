// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_includes_name() {
        let out = greet("fixture-user");
        assert!(out.contains("fixture-user"));
    }

    #[test]
    fn greet_rejects_nothing_empty_is_safe() {
        // Negative case: empty input must not panic and must stay a plain string.
        let out = greet("");
        assert!(out.starts_with("Hello, ") && out.ends_with('!'));
        assert!(!out.contains('\n'));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
