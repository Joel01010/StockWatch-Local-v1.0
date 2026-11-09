use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

pub fn send_native<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
    sound: bool,
) -> Result<(), String> {
    let mut builder = app.notification().builder().title(title).body(body);
    if !sound {
        builder = builder.silent();
    }
    builder.show().map_err(|error| error.to_string())
}
