use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

/// 将 config.json 和 prompts/ 目录下的 .md 文件打包为 zip 字节
pub fn create_backup_zip(data_dir: &Path) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        // 添加 config.json
        let config_path = data_dir.join("config.json");
        if config_path.exists() {
            let config_content = fs::read(&config_path)
                .map_err(|e| format!("读取 config.json 失败: {}", e))?;
            zip.start_file("config.json", options)
                .map_err(|e| format!("写入 zip 失败: {}", e))?;
            zip.write_all(&config_content)
                .map_err(|e| format!("写入 zip 失败: {}", e))?;
        }

        // 添加 prompts/ 目录下的所有文件
        let prompts_dir = data_dir.join("prompts");
        if prompts_dir.exists() {
            let entries = collect_files(&prompts_dir)
                .map_err(|e| format!("遍历 prompts 目录失败: {}", e))?;
            for (relative_path, full_path) in entries {
                let content = fs::read(&full_path)
                    .map_err(|e| format!("读取文件 {} 失败: {}", relative_path, e))?;
                zip.start_file(format!("prompts/{}", relative_path), options)
                    .map_err(|e| format!("写入 zip 失败: {}", e))?;
                zip.write_all(&content)
                    .map_err(|e| format!("写入 zip 失败: {}", e))?;
            }
        }

        zip.finish()
            .map_err(|e| format!("完成 zip 失败: {}", e))?;
    }

    Ok(buffer)
}

/// 递归收集目录下所有文件，返回 (相对路径, 完整路径) 列表
fn collect_files(dir: &Path) -> std::io::Result<Vec<(String, std::path::PathBuf)>> {
    let mut result = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let sub = collect_files(&path)?;
            for (rel, full) in sub {
                let parent_name = entry.file_name().to_string_lossy().to_string();
                result.push((format!("{}/{}", parent_name, rel), full));
            }
        } else {
            let name = entry.file_name().to_string_lossy().to_string();
            result.push((name, path));
        }
    }
    Ok(result)
}

/// 解压 zip 数据到目标目录
pub fn extract_backup_zip(zip_data: &[u8], dest_dir: &Path) -> Result<(), String> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("打开 zip 失败: {}", e))?;

    // 防止 Zip Bomb：限制条目数量
    const MAX_ENTRIES: usize = 10000;
    if archive.len() > MAX_ENTRIES {
        return Err(format!("备份文件无效：条目数量超过上限 {}", MAX_ENTRIES));
    }

    // 校验：zip 中必须包含 config.json
    let has_config = (0..archive.len()).any(|i| {
        archive.by_index(i)
            .map(|f| f.name().to_string() == "config.json")
            .unwrap_or(false)
    });
    if !has_config {
        return Err("备份文件无效：缺少 config.json".to_string());
    }

    // 防止 Zip Bomb：限制单个文件未压缩大小
    const MAX_FILE_SIZE: u64 = 256 * 1024 * 1024;

    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| format!("读取 zip 条目失败: {}", e))?;

        // 校验未压缩大小，避免解压时无限膨胀内存或磁盘
        let file_name = file.name().to_string();
        if file.size() > MAX_FILE_SIZE {
            return Err(format!("文件过大: {}", file_name));
        }

        let path = match file.enclosed_name() {
            Some(p) => dest_dir.join(p),
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&path)
                .map_err(|e| format!("创建目录失败: {}", e))?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
            let mut out = fs::File::create(&path)
                .map_err(|e| format!("创建文件失败: {}", e))?;
            // 限制读取大小，防止实际解压数据超出声明大小
            let mut buf = Vec::new();
            file.take(MAX_FILE_SIZE + 1)
                .read_to_end(&mut buf)
                .map_err(|e| format!("读取 zip 条目失败: {}", e))?;
            if buf.len() as u64 > MAX_FILE_SIZE {
                return Err(format!("文件过大: {}", file_name));
            }
            out.write_all(&buf)
                .map_err(|e| format!("写入文件失败: {}", e))?;
        }
    }

    Ok(())
}
