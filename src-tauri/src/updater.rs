use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// Check for updates and prompt the user.
/// - `silent = true` (startup): no dialog on "up to date" or error
/// - `silent = false` (manual): shows dialog for all outcomes
pub async fn check_and_prompt_update(app: AppHandle, silent: bool) {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Updater init failed: {}", e);
            if !silent {
                app.dialog()
                    .message(format!("检查更新失败：{}", e))
                    .kind(MessageDialogKind::Error)
                    .title("更新")
                    .blocking_show();
            }
            return;
        }
    };

    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => {
            if !silent {
                app.dialog()
                    .message("当前已是最新版本")
                    .kind(MessageDialogKind::Info)
                    .title("更新")
                    .blocking_show();
            }
            return;
        }
        Err(e) => {
            eprintln!("Update check failed: {}", e);
            if !silent {
                app.dialog()
                    .message(format!("检查更新失败：{}", e))
                    .kind(MessageDialogKind::Error)
                    .title("更新")
                    .blocking_show();
            }
            return;
        }
    };

    // Ask user to confirm
    let version = &update.version;
    let confirmed = app
        .dialog()
        .message(format!("发现新版本 v{}，是否立即更新？", version))
        .title("更新")
        .buttons(MessageDialogButtons::OkCancelCustom("更新".into(), "取消".into()))
        .blocking_show();

    if !confirmed {
        return;
    }

    // Download and install
    println!("Downloading update v{}...", version);
    match update
        .download_and_install(|_, _| {}, || {})
        .await
    {
        Ok(()) => {
            app.dialog()
                .message("更新完成，应用即将重启")
                .kind(MessageDialogKind::Info)
                .title("更新")
                .blocking_show();
            app.restart();
        }
        Err(e) => {
            eprintln!("Update install failed: {}", e);
            app.dialog()
                .message(format!("更新失败：{}，请稍后重试", e))
                .kind(MessageDialogKind::Error)
                .title("更新")
                .blocking_show();
        }
    }
}
