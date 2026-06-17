use std::fs;
use std::path::Path;
use crate::backup::archive::{create_backup_zip, extract_backup_zip};

/// 备份到本地指定路径
pub fn backup_to_local(data_dir: &Path, save_path: &Path) -> Result<String, String> {
    let zip_data = create_backup_zip(data_dir)?;

    // 确保父目录存在
    if let Some(parent) = save_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    fs::write(save_path, &zip_data)
        .map_err(|e| format!("写入备份文件失败: {}", e))?;

    Ok(save_path.to_string_lossy().to_string())
}

/// 从本地 zip 恢复
/// 先备份当前 config.json 到 config.json.bak，恢复失败时回滚
pub fn restore_from_local(zip_path: &Path, data_dir: &Path) -> Result<(), String> {
    let zip_data = fs::read(zip_path)
        .map_err(|e| format!("读取备份文件失败: {}", e))?;

    // 备份当前 config.json
    let config_path = data_dir.join("config.json");
    let backup_config_path = data_dir.join("config.json.bak");
    let has_existing_config = config_path.exists();

    if has_existing_config {
        fs::copy(&config_path, &backup_config_path)
            .map_err(|e| format!("备份当前配置失败: {}", e))?;
    }

    // 解压到临时目录
    let temp_dir = data_dir.join(".restore_temp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("清理临时目录失败: {}", e))?;
    }

    match extract_backup_zip(&zip_data, &temp_dir) {
        Ok(()) => {
            // 用临时目录中的文件覆盖实际文件
            // config.json
            let temp_config = temp_dir.join("config.json");
            if temp_config.exists() {
                fs::copy(&temp_config, &config_path)
                    .map_err(|e| format!("覆盖 config.json 失败: {}", e))?;
            }

            // prompts/
            let temp_prompts = temp_dir.join("prompts");
            let actual_prompts = data_dir.join("prompts");
            if temp_prompts.exists() {
                if actual_prompts.exists() {
                    fs::remove_dir_all(&actual_prompts)
                        .map_err(|e| format!("清理旧 prompts 失败: {}", e))?;
                }
                copy_dir_recursive(&temp_prompts, &actual_prompts)
                    .map_err(|e| format!("恢复 prompts 失败: {}", e))?;
            }

            // 清理临时目录
            let _ = fs::remove_dir_all(&temp_dir);

            // 清理备份
            if has_existing_config {
                let _ = fs::remove_file(&backup_config_path);
            }

            Ok(())
        }
        Err(e) => {
            // 恢复失败，回滚
            let _ = fs::remove_dir_all(&temp_dir);
            if has_existing_config && backup_config_path.exists() {
                let _ = fs::copy(&backup_config_path, &config_path);
                let _ = fs::remove_file(&backup_config_path);
            }
            Err(format!("恢复失败（已回滚）: {}", e))
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
