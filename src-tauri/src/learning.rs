use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const DOCUMENT_HEADER: &str = "# 语音纠正学习\n\n以下规则由自动学习生成，供后续语音转写参考。\n\n";
const LEARNING_SYSTEM_PROMPT: &str = r#"你负责从用户对语音转写结果的手动修改中提取简短纠错规则。

输入包含：original（最近一次最终展示文本）、corrected（当前剪贴板文本）、existingRules（已有学习规则）。

要求：
1. 先判断corrected是否由original修改而来。主题或文本结构明显无关时，related为false，不输出任何规则操作。
2. 只学习语音识别造成的词语错误。忽略标点、排版、语气、增删内容和改写表达。
3. 每条规则只保留正确词及周围1至2个能说明语境的词，格式为“正确片段，不是错误片段”。例如：“项目技能，不是项目智能”。
4. 与existingRules完全重复时不要添加。学习规则只累加，不删除或替换已有规则，remove始终返回空数组。
5. 只返回JSON，不要使用Markdown或解释。格式必须是：{"related":true,"remove":[],"add":[]}。"#;

pub struct VoiceLearningManager {
    latest_output: Mutex<Option<String>>,
    rules_path: PathBuf,
    document_lock: Mutex<()>,
    running: AtomicBool,
}

impl VoiceLearningManager {
    pub fn new(data_dir: &Path) -> Self {
        let rules_path = data_dir.join("prompts").join("voice-learning.md");
        if let Err(error) = recover_windows_backup(&rules_path) {
            eprintln!("[learning] {error}");
        }
        Self {
            latest_output: Mutex::new(None),
            rules_path,
            document_lock: Mutex::new(()),
            running: AtomicBool::new(false),
        }
    }

    pub fn record_output(&self, text: &str) {
        *self
            .latest_output
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(text.to_string());
    }

    pub fn rules_content(&self) -> String {
        read_document(&self.rules_path).unwrap_or_else(|error| {
            eprintln!("[learning] {error}");
            String::new()
        })
    }

    fn ensure_document_unlocked(&self) -> Result<PathBuf, String> {
        if self.rules_path.exists() {
            read_document(&self.rules_path)?;
            return Ok(self.rules_path.clone());
        }

        let parent = self
            .rules_path
            .parent()
            .ok_or_else(|| "学习文档路径无效".to_string())?;
        fs::create_dir_all(parent).map_err(|error| format!("创建学习目录失败: {error}"))?;
        fs::write(&self.rules_path, DOCUMENT_HEADER)
            .map_err(|error| format!("创建学习文档失败: {error}"))?;
        Ok(self.rules_path.clone())
    }

    fn document(&self) -> Result<LearningDocument, String> {
        let _guard = self
            .document_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.ensure_document_unlocked()?;
        Ok(LearningDocument {
            path: self.rules_path.to_string_lossy().to_string(),
            content: read_document(&self.rules_path)?,
        })
    }

    fn save_document(&self, content: &str, base_content: &str) -> Result<LearningDocument, String> {
        let _guard = self
            .document_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.ensure_document_unlocked()?;
        let current = read_document(&self.rules_path)?;
        let merged = if current == base_content {
            content.to_string()
        } else {
            merge_user_document(base_content, content, &current)
        };
        write_document(&self.rules_path, &merged)?;
        Ok(LearningDocument {
            path: self.rules_path.to_string_lossy().to_string(),
            content: merged,
        })
    }

    fn latest_output(&self) -> Option<String> {
        self.latest_output
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn begin(&self) -> Result<LearningRun<'_>, String> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "自动学习正在进行，请稍候".to_string())?;
        Ok(LearningRun(&self.running))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningDocument {
    path: String,
    content: String,
}

struct LearningRun<'a>(&'a AtomicBool);

impl Drop for LearningRun<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct LearningAnalysis {
    pub related: bool,
    #[serde(default)]
    pub remove: Vec<String>,
    #[serde(default)]
    pub add: Vec<String>,
}

pub fn parse_analysis(response: &str) -> Result<LearningAnalysis, String> {
    let trimmed = response.trim();
    let json = if let Some(fenced) = trimmed.strip_prefix("```") {
        let fenced = fenced.strip_prefix("json").unwrap_or(fenced);
        fenced.strip_suffix("```").unwrap_or(fenced).trim()
    } else {
        trimmed
    };

    serde_json::from_str(json).map_err(|error| format!("学习结果解析失败: {error}"))
}

#[derive(Debug, PartialEq, Eq)]
pub enum LearningOutcome {
    Unrelated,
    NoChange,
    Updated { added: usize, removed: usize },
}

pub fn apply_analysis(path: &Path, analysis: LearningAnalysis) -> Result<LearningOutcome, String> {
    if !analysis.related {
        return Ok(LearningOutcome::Unrelated);
    }

    let existing = read_document(path)?;
    let mut rules = parse_rules(&existing);
    let original_rules = rules.clone();

    let additions: Vec<String> = analysis
        .add
        .iter()
        .filter_map(|rule| normalize_rule(rule))
        .collect();
    let mut added = 0;
    for rule in additions {
        if !rules.contains(&rule) {
            rules.push(rule);
            added += 1;
        }
    }

    if rules == original_rules {
        return Ok(LearningOutcome::NoChange);
    }

    let parent = path
        .parent()
        .ok_or_else(|| "学习文档路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("创建学习目录失败: {error}"))?;
    let document = update_rule_lines(&existing, &original_rules, &rules);
    write_document(path, &document)?;

    Ok(LearningOutcome::Updated { added, removed: 0 })
}

fn write_document(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "学习文档路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("创建学习目录失败: {error}"))?;
    let temp_path = path.with_extension("md.tmp");
    fs::write(&temp_path, content).map_err(|error| format!("写入学习文档失败: {error}"))?;
    replace_document(&temp_path, path)
}

fn update_rule_lines(content: &str, old_rules: &[String], new_rules: &[String]) -> String {
    let removed: Vec<String> = old_rules
        .iter()
        .filter(|rule| !new_rules.contains(rule))
        .cloned()
        .collect();
    let mut present = Vec::new();
    let mut output = String::new();

    for line in content.lines() {
        let rule = line.trim().strip_prefix("- ").and_then(normalize_rule);
        if rule.as_ref().is_some_and(|rule| removed.contains(rule)) {
            continue;
        }
        if let Some(rule) = rule {
            present.push(rule);
        }
        output.push_str(line);
        output.push('\n');
    }

    let missing: Vec<&String> = new_rules
        .iter()
        .filter(|rule| !present.contains(rule))
        .collect();
    if output.is_empty() && !missing.is_empty() {
        output.push_str(DOCUMENT_HEADER);
    } else if !missing.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
    for rule in missing {
        output.push_str("- ");
        output.push_str(rule);
        output.push('\n');
    }
    output
}

fn merge_user_document(base: &str, edited: &str, current: &str) -> String {
    let base_rules = parse_rules(base);
    let edited_rules = parse_rules(edited);
    let current_rules = parse_rules(current);
    let mut merged_rules = current_rules.clone();

    merged_rules.retain(|rule| !base_rules.contains(rule) || edited_rules.contains(rule));
    for rule in edited_rules
        .iter()
        .filter(|rule| !base_rules.contains(rule))
    {
        if !merged_rules.contains(rule) {
            merged_rules.push(rule.clone());
        }
    }

    update_rule_lines(edited, &edited_rules, &merged_rules)
}

#[cfg(not(target_os = "windows"))]
fn replace_document(temp_path: &Path, path: &Path) -> Result<(), String> {
    if let Err(error) = fs::rename(temp_path, path) {
        let _ = fs::remove_file(temp_path);
        return Err(format!("更新学习文档失败: {error}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_document(temp_path: &Path, path: &Path) -> Result<(), String> {
    let backup_path = path.with_extension("md.bak");
    if backup_path.exists() {
        if !path.exists() {
            return Err("检测到尚未恢复的学习文档备份，已停止写入".to_string());
        }
        fs::remove_file(&backup_path).map_err(|error| format!("清理学习文档备份失败: {error}"))?;
    }
    if path.exists() {
        fs::rename(path, &backup_path).map_err(|error| format!("备份学习文档失败: {error}"))?;
    }

    if let Err(error) = fs::rename(temp_path, path) {
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(temp_path);
        return Err(format!("更新学习文档失败: {error}"));
    }
    let _ = fs::remove_file(backup_path);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn recover_windows_backup(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn recover_windows_backup(path: &Path) -> Result<(), String> {
    let backup_path = path.with_extension("md.bak");
    if !path.exists() && backup_path.exists() {
        fs::rename(backup_path, path).map_err(|error| format!("恢复学习文档备份失败: {error}"))?;
    }
    Ok(())
}

fn normalize_rule(rule: &str) -> Option<String> {
    let trimmed = rule.trim();
    let trimmed = trimmed.strip_prefix('-').unwrap_or(trimmed).trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn read_document(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("读取学习文档失败: {error}")),
    }
}

fn parse_rules(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter_map(normalize_rule)
        .collect()
}

fn load_rules(path: &Path) -> Result<Vec<String>, String> {
    read_document(path).map(|content| parse_rules(&content))
}

pub async fn learn_from_clipboard(app: &AppHandle) -> Result<LearningOutcome, String> {
    let manager = app.state::<VoiceLearningManager>();
    let _run = manager.begin()?;
    let original = manager
        .latest_output()
        .ok_or_else(|| "还没有可学习的语音转写结果".to_string())?;

    let corrected = arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|error| format!("读取剪贴板失败: {error}"))?;
    if corrected.trim().is_empty() {
        return Err("剪贴板中没有文本".to_string());
    }
    if corrected.trim() == original.trim() {
        return Ok(LearningOutcome::NoChange);
    }

    let input = serde_json::json!({
        "original": original,
        "corrected": corrected,
        "existingRules": load_rules(&manager.rules_path)?,
    })
    .to_string();
    let config = app.state::<crate::config::ConfigManager>().get();
    if !crate::ai::models::supports_text(&config, &config.transcribe.model_id)? {
        return Err("当前语音转写模型不支持文本分析，无法执行自动学习".to_string());
    }
    let client =
        crate::task::build_client(config.advanced.proxy_enabled, &config.advanced.proxy_url)?;
    let response = crate::ai::retry::with_retry(
        || {
            let client = client.clone();
            let input = input.clone();
            let config = config.clone();
            async move {
                crate::ai::analyze_correction(&client, &input, LEARNING_SYSTEM_PROMPT, &config)
                    .await
            }
        },
        config.advanced.max_retries,
        config.advanced.optimize_timeout,
        |_| {},
    )
    .await?;
    let analysis = parse_analysis(&response)?;
    let outcome = {
        let _guard = manager
            .document_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        apply_analysis(&manager.rules_path, analysis)?
    };
    if matches!(outcome, LearningOutcome::Updated { .. }) {
        let _ = app.emit("voice-learning-updated", ());
    }
    Ok(outcome)
}

#[tauri::command]
pub fn get_voice_learning_document(
    manager: State<'_, VoiceLearningManager>,
) -> Result<LearningDocument, String> {
    manager.document()
}

#[tauri::command]
pub fn save_voice_learning_document(
    manager: State<'_, VoiceLearningManager>,
    content: String,
    base_content: String,
) -> Result<LearningDocument, String> {
    manager.save_document(&content, &base_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_file(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be available")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("byetype-learning-{nonce}"))
            .join(name)
    }

    #[test]
    fn parses_json_wrapped_in_markdown_fence() {
        let response = r#"```json
{"related":true,"remove":["项目智能，应为项目技能"],"add":["项目技能，不是项目智能"]}
```"#;

        let analysis = parse_analysis(response).expect("response should parse");

        assert_eq!(
            analysis,
            LearningAnalysis {
                related: true,
                remove: vec!["项目智能，应为项目技能".to_string()],
                add: vec!["项目技能，不是项目智能".to_string()],
            }
        );
    }

    #[test]
    fn appends_new_rule_and_deduplicates_additions() {
        let path = test_file("voice-learning.md");
        fs::create_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be created");
        fs::write(
            &path,
            "# 语音纠正学习\n\n以下规则由自动学习生成。\n\n- 项目智能，应为项目技能\n- 保持API大写\n",
        )
        .expect("fixture should be written");

        let outcome = apply_analysis(
            &path,
            LearningAnalysis {
                related: true,
                remove: vec!["项目智能，应为项目技能".to_string()],
                add: vec![
                    "项目技能，不是项目智能".to_string(),
                    "保持API大写".to_string(),
                ],
            },
        )
        .expect("analysis should be applied");

        assert_eq!(
            outcome,
            LearningOutcome::Updated {
                added: 1,
                removed: 0
            }
        );
        assert_eq!(
            fs::read_to_string(&path).expect("learning document should exist"),
            "# 语音纠正学习\n\n以下规则由自动学习生成。\n\n- 项目智能，应为项目技能\n- 保持API大写\n\n- 项目技能，不是项目智能\n"
        );

        fs::remove_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be removed");
    }

    #[test]
    fn unrelated_text_does_not_create_learning_document() {
        let path = test_file("voice-learning.md");

        let outcome = apply_analysis(
            &path,
            LearningAnalysis {
                related: false,
                remove: vec![],
                add: vec!["无关规则".to_string()],
            },
        )
        .expect("unrelated analysis should be ignored");

        assert_eq!(outcome, LearningOutcome::Unrelated);
        assert!(!path.exists());
    }

    #[test]
    fn read_failure_stops_before_overwriting_learning_document() {
        let path = test_file("voice-learning.md");
        fs::create_dir_all(&path).expect("fixture directory should be created");

        let error = apply_analysis(
            &path,
            LearningAnalysis {
                related: true,
                remove: vec![],
                add: vec!["项目技能，不是项目智能".to_string()],
            },
        )
        .expect_err("unreadable learning document should stop the update");

        assert!(error.starts_with("读取学习文档失败"));
        assert!(path.is_dir());
        fs::remove_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be removed");
    }

    #[test]
    fn creates_editable_learning_document_on_request() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);

        let document_path = PathBuf::from(
            manager
                .document()
                .expect("learning document should be created")
                .path,
        );

        assert_eq!(
            fs::read_to_string(&document_path).expect("learning document should be readable"),
            DOCUMENT_HEADER
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn learning_update_preserves_user_markdown() {
        let path = test_file("voice-learning.md");
        fs::create_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be created");
        fs::write(
            &path,
            "# 我的学习规则\n\n这段说明由用户维护。\n\n- 智能，不是技能\n\n## 备注\n\n保留这里。\n",
        )
        .expect("fixture should be written");

        apply_analysis(
            &path,
            LearningAnalysis {
                related: true,
                remove: vec!["智能，不是技能".to_string()],
                add: vec!["项目技能，不是项目智能".to_string()],
            },
        )
        .expect("analysis should be applied");

        let content = fs::read_to_string(&path).expect("learning document should exist");
        assert!(content.contains("# 我的学习规则"));
        assert!(content.contains("这段说明由用户维护。"));
        assert!(content.contains("## 备注\n\n保留这里。"));
        assert!(content.contains("- 智能，不是技能"));
        assert!(content.contains("- 项目技能，不是项目智能"));
        fs::remove_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be removed");
    }

    #[test]
    fn concurrent_manual_save_merges_external_rule_update() {
        let base = "# 语音纠正学习\n\n用户说明。\n\n- 原规则\n";
        let edited = "# 我的规则\n\n用户改过的说明。\n";
        let current = "# 语音纠正学习\n\n用户说明。\n\n- 原规则\n- AI新增规则\n";

        let merged = merge_user_document(base, edited, current);

        assert!(merged.contains("# 我的规则"));
        assert!(merged.contains("用户改过的说明。"));
        assert!(!merged.contains("- 原规则"));
        assert!(merged.contains("- AI新增规则"));
    }

    #[test]
    fn unrelated_existing_rule_is_not_removed_when_adding_a_new_rule() {
        let path = test_file("voice-learning.md");
        fs::create_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be created");
        fs::write(&path, format!("{DOCUMENT_HEADER}- 李氏，不是李四\n"))
            .expect("fixture should be written");

        apply_analysis(
            &path,
            LearningAnalysis {
                related: true,
                remove: vec!["李氏，不是李四".to_string()],
                add: vec!["宠物组，不是宠物族".to_string()],
            },
        )
        .expect("analysis should be applied");

        let content = fs::read_to_string(&path).expect("learning document should exist");
        assert!(content.contains("- 李氏，不是李四"));
        assert!(content.contains("- 宠物组，不是宠物族"));
        fs::remove_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be removed");
    }
}
