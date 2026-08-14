use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const DOCUMENT_HEADER: &str = "# 语音纠正学习\n\n以下规则由自动学习生成，供后续语音转写参考。\n\n";
const DEFAULT_LEARNING_PROMPT: &str = include_str!("../prompts/voice-learning-prompt.md");
const LAST_DEFAULT_LEARNING_PROMPT: &str =
    include_str!("../prompts/legacy/voice-learning-prompt-1.16.1.md");
const LEGACY_DEFAULT_LEARNING_PROMPT: &str = r#"你负责从用户对语音转写结果的手动修改中提取简短纠错规则。

输入包含：

- original：最近一次最终展示文本。
- corrected：当前剪贴板文本。
- existingRules：已有学习规则。

要求：

1. 先判断corrected是否由original修改而来。主题或文本结构明显无关时，related为false，不输出任何规则。
2. 只学习语音识别造成的词语错误。忽略标点、排版、语气、增删内容和改写表达。
3. 每条规则只保留正确词及周围1至2个能说明语境的词，格式为「正确片段，不是错误片段」。例如：「项目技能，不是项目智能」。
4. 与existingRules完全重复时不要添加。学习规则只累加，不删除或替换已有规则，remove始终返回空数组。
5. 只返回JSON，不要使用Markdown或解释。格式必须是：{"related":true,"remove":[],"add":[]}。
"#;
const PREVIOUS_DEFAULT_LEARNING_PROMPT: &str = r#"你负责判断用户对语音转写结果的修改是否包含值得长期学习的内容，并提取可复用的学习项。

输入包含：

- original：最近一次最终展示文本。
- corrected：当前剪贴板文本。
- existingContent.transcriptionRules：当前使用的全部转录规则。
- existingContent.vocabulary：当前使用的全部专有词汇。
- existingContent.autoLearning：此前由自动学习追加的内容。

要求：

1. 由你完整判断corrected是否由original修改而来，以及修改中是否有内容值得长期学习。主题或文本结构明显无关时，related为false。
2. 从两个方面判断是否值得学习：
   - 专有词汇：人名、品牌名、产品名、行业术语、技术名词、固定写法等。
   - 转录规则：能在后续语音转写中重复使用的纠错、格式或表达转换规则。
3. 扫描existingContent中的全部内容，按含义比较。已有内容已经覆盖的词汇或规则不要重复添加；没有覆盖的内容才追加到add。
4. 只提取本次修改能够明确证明的内容，不猜测。每项写成可直接用于后续转写的简短指令，并标明「词汇」或「规则」。例如：「词汇：ByeType」「规则：将项目智能纠正为项目技能」。
5. 由你决定是否学习及学习哪些内容。add可以同时包含词汇和规则，也可以为空。已有内容只用于比较，不删除或改写，remove始终返回空数组。
6. 只返回JSON，不要使用Markdown或解释。格式必须是：{"related":true,"remove":[],"add":[]}。
"#;

pub struct VoiceLearningManager {
    latest_output: Mutex<Option<String>>,
    rules_path: PathBuf,
    prompt_path: PathBuf,
    document_lock: Mutex<()>,
    running: AtomicBool,
    draft: Mutex<Option<LearningDraft>>,
}

impl VoiceLearningManager {
    pub fn new(data_dir: &Path) -> Self {
        let rules_path = data_dir.join("prompts").join("voice-learning.md");
        let prompt_path = data_dir.join("prompts").join("voice-learning-prompt.md");
        if let Err(error) = recover_windows_backup(&rules_path) {
            eprintln!("[learning] {error}");
        }
        Self {
            latest_output: Mutex::new(None),
            rules_path,
            prompt_path,
            document_lock: Mutex::new(()),
            running: AtomicBool::new(false),
            draft: Mutex::new(None),
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

    fn ensure_prompt_document(&self) -> Result<PathBuf, String> {
        if self.prompt_path.exists() {
            let content = read_document(&self.prompt_path)?;
            if content == LEGACY_DEFAULT_LEARNING_PROMPT
                || content == PREVIOUS_DEFAULT_LEARNING_PROMPT
                || content == LAST_DEFAULT_LEARNING_PROMPT
            {
                write_document(&self.prompt_path, DEFAULT_LEARNING_PROMPT)?;
            }
            return Ok(self.prompt_path.clone());
        }
        write_document(&self.prompt_path, DEFAULT_LEARNING_PROMPT)?;
        Ok(self.prompt_path.clone())
    }

    fn prompt_content(&self) -> Result<String, String> {
        self.ensure_prompt_document()?;
        let content = read_document(&self.prompt_path)?;
        if content.trim().is_empty() {
            return Err("自动学习提示词不能为空".to_string());
        }
        Ok(content)
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

    fn draft(&self) -> Option<LearningDraft> {
        self.draft
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn set_draft(&self, draft: LearningDraft) {
        *self.draft.lock().unwrap_or_else(|error| error.into_inner()) = Some(draft);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningDocument {
    path: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningDraft {
    original: String,
    corrected: String,
    generated: String,
    notice: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningApplyResult {
    added: usize,
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
    #[cfg(test)]
    Unrelated,
    NoChange,
    Updated {
        added: usize,
        removed: usize,
    },
}

#[cfg(test)]
pub fn apply_analysis(path: &Path, analysis: LearningAnalysis) -> Result<LearningOutcome, String> {
    if !analysis.related {
        return Ok(LearningOutcome::Unrelated);
    }

    let existing = read_document(path)?;
    let additions: Vec<String> = analysis
        .add
        .iter()
        .filter_map(|rule| normalize_rule(rule))
        .collect();
    if additions.is_empty() {
        return Ok(LearningOutcome::NoChange);
    }

    let parent = path
        .parent()
        .ok_or_else(|| "学习文档路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("创建学习目录失败: {error}"))?;
    let document = append_learning_items(&existing, &additions);
    write_document(path, &document)?;

    Ok(LearningOutcome::Updated {
        added: additions.len(),
        removed: 0,
    })
}

fn apply_generated_text(path: &Path, generated: &str) -> Result<LearningOutcome, String> {
    let existing = read_document(path)?;
    let existing_rules = parse_rules(&existing);
    let mut additions = Vec::new();

    for item in parse_generated_items(generated)? {
        let item = item.into_document_text();
        if !existing_rules.contains(&item) && !additions.contains(&item) {
            additions.push(item);
        }
    }

    if additions.is_empty() {
        return Ok(LearningOutcome::NoChange);
    }

    write_document(path, &append_learning_items(&existing, &additions))?;
    Ok(LearningOutcome::Updated {
        added: additions.len(),
        removed: 0,
    })
}

#[derive(Debug, PartialEq, Eq)]
enum LearningItem {
    Vocabulary(String),
    Rule(String),
}

impl LearningItem {
    fn into_document_text(self) -> String {
        match self {
            Self::Vocabulary(value) => format!("词汇：{value}"),
            Self::Rule(value) => format!("规则：{value}"),
        }
    }
}

fn parse_generated_items(generated: &str) -> Result<Vec<LearningItem>, String> {
    let mut items = Vec::new();
    for (index, line) in generated.lines().enumerate() {
        let line = line.trim().strip_prefix('-').unwrap_or(line.trim()).trim();
        if line.is_empty() {
            continue;
        }
        let item = if let Some(value) = line.strip_prefix("词汇：") {
            LearningItem::Vocabulary(value.trim().to_string())
        } else if let Some(value) = line.strip_prefix("规则：") {
            LearningItem::Rule(value.trim().to_string())
        } else {
            return Err(format!(
                "学习结果第{}行格式不正确，请以「词汇：」或「规则：」开头",
                index + 1
            ));
        };
        let is_empty = match &item {
            LearningItem::Vocabulary(value) | LearningItem::Rule(value) => value.is_empty(),
        };
        if is_empty {
            return Err(format!("学习结果第{}行内容不能为空", index + 1));
        }
        items.push(item);
    }
    Ok(items)
}

fn append_learning_items(content: &str, additions: &[String]) -> String {
    let mut output = if content.is_empty() {
        DOCUMENT_HEADER.to_string()
    } else {
        content.to_string()
    };
    if !output.ends_with('\n') {
        output.push('\n');
    }
    for item in additions {
        output.push_str("- ");
        output.push_str(item);
        output.push('\n');
    }
    output
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
    } else if !missing.is_empty() && !output.ends_with('\n') {
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

fn build_learning_input(
    original: &str,
    corrected: &str,
    transcription_rules: &str,
    vocabulary: &str,
    auto_learning: &str,
) -> String {
    serde_json::json!({
        "original": original,
        "corrected": corrected,
        "existingContent": {
            "transcriptionRules": transcription_rules,
            "vocabulary": vocabulary,
            "autoLearning": auto_learning,
        },
    })
    .to_string()
}

async fn generate_draft(
    app: &AppHandle,
    original: String,
    corrected: String,
) -> Result<LearningDraft, String> {
    let manager = app.state::<VoiceLearningManager>();
    let _run = manager.begin()?;
    if corrected.trim().is_empty() {
        return Err("用户修订不能为空".to_string());
    }
    let config = app.state::<crate::config::ConfigManager>().get();
    let prompts_dir = crate::commands::resolve_prompts_dir_pub(app)?;
    let (transcription_rules, vocabulary) =
        crate::ai::prompt::load_transcription_reference_content(&config, &prompts_dir)?;
    let auto_learning = read_document(&manager.rules_path)?;
    let input = build_learning_input(
        &original,
        &corrected,
        &transcription_rules,
        &vocabulary,
        &auto_learning,
    );
    let learning_prompt = manager.prompt_content()?;
    if !crate::ai::models::supports_text(&config, &config.voice_learning.model_id)? {
        return Err("当前语音转写模型不支持文本分析，无法执行自动学习".to_string());
    }
    let client =
        crate::task::build_client(config.advanced.proxy_enabled, &config.advanced.proxy_url)?;
    let response = crate::ai::retry::with_retry(
        || {
            let client = client.clone();
            let input = input.clone();
            let learning_prompt = learning_prompt.clone();
            let config = config.clone();
            async move {
                crate::ai::analyze_correction(&client, &input, &learning_prompt, &config).await
            }
        },
        config.advanced.max_retries,
        config.advanced.optimize_timeout,
        |_| {},
    )
    .await?;
    let analysis = parse_analysis(&response)?;
    let (generated, notice) = if !analysis.related {
        (
            String::new(),
            "没有识别到可学习的修改，可以编辑前两栏后重新生成。".to_string(),
        )
    } else if analysis.add.is_empty() {
        (
            String::new(),
            "已有规则覆盖了本次修改，也可以在右栏手动填写。".to_string(),
        )
    } else {
        (
            analysis.add.join("\n"),
            format!("AI生成了{}条学习内容，确认后才会录入。", analysis.add.len()),
        )
    };
    Ok(LearningDraft {
        original,
        corrected,
        generated,
        notice,
    })
}

pub async fn learn_from_clipboard(app: &AppHandle) -> Result<(), String> {
    let manager = app.state::<VoiceLearningManager>();
    let original = manager.latest_output().unwrap_or_default();
    let corrected = arboard::Clipboard::new()
        .map_err(|error| format!("读取剪贴板失败：{error}"))?
        .get_text()
        .unwrap_or_default();
    if corrected.trim().is_empty() {
        let draft = LearningDraft {
            original,
            corrected: String::new(),
            generated: String::new(),
            notice: "请在中栏输入自然语言新增要求，再点击重新生成。".to_string(),
        };
        manager.set_draft(draft.clone());
        let _ = app.emit_to("learning", "voice-learning-draft", &draft);
        show_learning_window(app)?;
        return Ok(());
    }

    let draft = generate_draft(app, original, corrected).await?;
    manager.set_draft(draft.clone());
    let _ = app.emit_to("learning", "voice-learning-draft", &draft);
    show_learning_window(app)?;
    Ok(())
}

fn show_learning_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("learning")
        .ok_or_else(|| "学习窗口未初始化".to_string())?;
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    #[cfg(target_os = "windows")]
    let _ = window.set_skip_taskbar(false);
    let _ = window.center();
    window
        .show()
        .map_err(|error| format!("显示学习窗口失败：{error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("聚焦学习窗口失败：{error}"))
}

#[tauri::command]
pub fn get_voice_learning_draft(
    manager: State<'_, VoiceLearningManager>,
) -> Result<Option<LearningDraft>, String> {
    Ok(manager.draft())
}

#[tauri::command]
pub async fn regenerate_voice_learning(
    app: AppHandle,
    original: String,
    corrected: String,
) -> Result<LearningDraft, String> {
    let draft = generate_draft(&app, original, corrected).await?;
    app.state::<VoiceLearningManager>().set_draft(draft.clone());
    Ok(draft)
}

#[tauri::command]
pub fn apply_voice_learning_generated(
    app: AppHandle,
    manager: State<'_, VoiceLearningManager>,
    generated: String,
) -> Result<LearningApplyResult, String> {
    if generated.trim().is_empty() {
        return Err("学习内容不能为空".to_string());
    }
    let outcome = {
        let _guard = manager
            .document_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        apply_generated_text(&manager.rules_path, &generated)?
    };
    let added = match outcome {
        LearningOutcome::Updated { added, .. } => added,
        LearningOutcome::NoChange => 0,
        #[cfg(test)]
        LearningOutcome::Unrelated => 0,
    };
    if added > 0 {
        let _ = app.emit("voice-learning-updated", ());
    }
    Ok(LearningApplyResult { added })
}

#[tauri::command]
pub fn close_voice_learning_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("learning")
        .ok_or_else(|| "学习窗口未初始化".to_string())?;
    window
        .hide()
        .map_err(|error| format!("关闭学习窗口失败：{error}"))?;
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    #[cfg(target_os = "windows")]
    let _ = window.set_skip_taskbar(true);
    Ok(())
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

#[tauri::command]
pub fn get_voice_learning_prompt_path(
    manager: State<'_, VoiceLearningManager>,
) -> Result<String, String> {
    manager
        .ensure_prompt_document()
        .map(|path| path.to_string_lossy().to_string())
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
    fn appends_every_item_selected_by_ai() {
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
                added: 2,
                removed: 0
            }
        );
        assert_eq!(
            fs::read_to_string(&path).expect("learning document should exist"),
            "# 语音纠正学习\n\n以下规则由自动学习生成。\n\n- 项目智能，应为项目技能\n- 保持API大写\n- 项目技能，不是项目智能\n- 保持API大写\n"
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
    fn creates_editable_learning_prompt_on_request() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);

        let prompt_path = manager
            .ensure_prompt_document()
            .expect("learning prompt should be created");

        assert_eq!(
            fs::read_to_string(&prompt_path).expect("learning prompt should be readable"),
            DEFAULT_LEARNING_PROMPT
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn reads_user_edited_learning_prompt() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);
        let prompt_path = manager
            .ensure_prompt_document()
            .expect("learning prompt should be created");
        fs::write(&prompt_path, "用户修改后的学习提示词")
            .expect("learning prompt should be editable");

        assert_eq!(
            manager
                .prompt_content()
                .expect("learning prompt should load"),
            "用户修改后的学习提示词"
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn upgrades_legacy_default_learning_prompt() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);
        fs::create_dir_all(
            manager
                .prompt_path
                .parent()
                .expect("prompt should have parent"),
        )
        .expect("prompt directory should be created");
        fs::write(&manager.prompt_path, LEGACY_DEFAULT_LEARNING_PROMPT)
            .expect("legacy prompt should be written");

        let prompt_path = manager
            .ensure_prompt_document()
            .expect("legacy prompt should be upgraded");

        assert_eq!(
            fs::read_to_string(prompt_path).expect("prompt should be readable"),
            DEFAULT_LEARNING_PROMPT
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn upgrades_previous_default_learning_prompt() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);
        fs::create_dir_all(
            manager
                .prompt_path
                .parent()
                .expect("prompt should have parent"),
        )
        .expect("prompt directory should be created");
        fs::write(&manager.prompt_path, PREVIOUS_DEFAULT_LEARNING_PROMPT)
            .expect("previous prompt should be written");

        let prompt_path = manager
            .ensure_prompt_document()
            .expect("previous prompt should be upgraded");

        assert_eq!(
            fs::read_to_string(prompt_path).expect("prompt should be readable"),
            DEFAULT_LEARNING_PROMPT
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn upgrades_last_default_learning_prompt() {
        let path = test_file("placeholder");
        let data_dir = path.parent().expect("test path should have parent");
        let manager = VoiceLearningManager::new(data_dir);
        fs::create_dir_all(
            manager
                .prompt_path
                .parent()
                .expect("prompt should have parent"),
        )
        .expect("prompt directory should be created");
        fs::write(&manager.prompt_path, LAST_DEFAULT_LEARNING_PROMPT)
            .expect("last prompt should be written");

        let prompt_path = manager
            .ensure_prompt_document()
            .expect("last prompt should be upgraded");

        assert_eq!(
            fs::read_to_string(prompt_path).expect("prompt should be readable"),
            DEFAULT_LEARNING_PROMPT
        );
        fs::remove_dir_all(data_dir).expect("test directory should be removed");
    }

    #[test]
    fn applies_only_unique_lines_from_edited_generated_text() {
        let path = test_file("voice-learning.md");
        fs::create_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be created");
        fs::write(&path, format!("{DOCUMENT_HEADER}- 词汇：ByeType\n"))
            .expect("fixture should be written");

        let outcome = apply_generated_text(
            &path,
            "- 词汇：ByeType\n规则：项目智能→项目技能\n规则：项目智能→项目技能\n",
        )
        .expect("generated text should be applied");

        assert_eq!(
            outcome,
            LearningOutcome::Updated {
                added: 1,
                removed: 0,
            }
        );
        let content = fs::read_to_string(&path).expect("learning document should exist");
        assert_eq!(content.matches("词汇：ByeType").count(), 1);
        assert_eq!(content.matches("规则：项目智能→项目技能").count(), 1);
        fs::remove_dir_all(path.parent().expect("test path should have parent"))
            .expect("test directory should be removed");
    }

    #[test]
    fn rejects_untyped_generated_lines() {
        let error = parse_generated_items("帮我记住ByeType")
            .expect_err("untyped learning content should be rejected");

        assert_eq!(
            error,
            "学习结果第1行格式不正确，请以「词汇：」或「规则：」开头"
        );
    }

    #[test]
    fn learning_input_contains_all_reference_content() {
        let input = build_learning_input(
            "原始文本",
            "修正文本",
            "规则内容",
            "词汇内容",
            "自动学习内容",
        );
        let value: serde_json::Value =
            serde_json::from_str(&input).expect("learning input should be valid JSON");

        assert_eq!(value["original"], "原始文本");
        assert_eq!(value["corrected"], "修正文本");
        assert_eq!(value["existingContent"]["transcriptionRules"], "规则内容");
        assert_eq!(value["existingContent"]["vocabulary"], "词汇内容");
        assert_eq!(value["existingContent"]["autoLearning"], "自动学习内容");
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
