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
/// 先备份当前 config.json 到 config.json.bak，prompts/ 到 prompts.bak/，
/// 恢复失败时回滚。所有 Ok 分支中的错误也会触发回滚。
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

    // 备份当前 prompts 目录
    let prompts_path = data_dir.join("prompts");
    let backup_prompts_path = data_dir.join("prompts.bak");
    let has_existing_prompts = prompts_path.exists();

    if has_existing_prompts {
        if backup_prompts_path.exists() {
            fs::remove_dir_all(&backup_prompts_path)
                .map_err(|e| format!("清理旧 prompts 备份失败: {}", e))?;
        }
        copy_dir_recursive(&prompts_path, &backup_prompts_path)
            .map_err(|e| format!("备份当前 prompts 失败: {}", e))?;
    }

    // 解压到临时目录
    let temp_dir = data_dir.join(".restore_temp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("清理临时目录失败: {}", e))?;
    }

    // 内部恢复逻辑：所有失败都返回 Err，由外层统一回滚
    let restore_result = (|| -> Result<(), String> {
        extract_backup_zip(&zip_data, &temp_dir)?;

        // 用临时目录中的文件覆盖实际文件
        // config.json（使用临时文件 + rename 保证原子性）
        let temp_config = temp_dir.join("config.json");
        if temp_config.exists() {
            let temp_write_path = data_dir.join("config.json.new");
            fs::copy(&temp_config, &temp_write_path)
                .map_err(|e| format!("写入 config.json 临时文件失败: {}", e))?;
            fs::rename(&temp_write_path, &config_path)
                .map_err(|e| {
                    let _ = fs::remove_file(&temp_write_path);
                    format!("重命名 config.json 失败: {}", e)
                })?;
        }

        // prompts/
        let temp_prompts = temp_dir.join("prompts");
        if temp_prompts.exists() {
            if prompts_path.exists() {
                fs::remove_dir_all(&prompts_path)
                    .map_err(|e| format!("清理旧 prompts 失败: {}", e))?;
            }
            copy_dir_recursive(&temp_prompts, &prompts_path)
                .map_err(|e| format!("恢复 prompts 失败: {}", e))?;
        }

        Ok(())
    })();

    // 统一清理临时目录
    let _ = fs::remove_dir_all(&temp_dir);

    match restore_result {
        Ok(()) => {
            // 成功：清理备份
            if has_existing_config {
                let _ = fs::remove_file(&backup_config_path);
            }
            if has_existing_prompts {
                let _ = fs::remove_dir_all(&backup_prompts_path);
            }
            Ok(())
        }
        Err(e) => {
            // 失败：回滚 config.json
            if has_existing_config && backup_config_path.exists() {
                let _ = fs::copy(&backup_config_path, &config_path);
                let _ = fs::remove_file(&backup_config_path);
            }
            // 失败：回滚 prompts 目录
            if has_existing_prompts && backup_prompts_path.exists() {
                if prompts_path.exists() {
                    let _ = fs::remove_dir_all(&prompts_path);
                }
                let _ = fs::rename(&backup_prompts_path, &prompts_path);
            } else if !has_existing_prompts && prompts_path.exists() {
                // 原本没有 prompts，恢复过程中创建了部分内容，清理掉
                let _ = fs::remove_dir_all(&prompts_path);
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
