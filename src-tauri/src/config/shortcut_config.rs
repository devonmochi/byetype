//! Shortcut-specific raw JSON handling: schema version parsing, Windows schema
//! v1 migration and field-level helpers.
//!
//! All functions operate on `serde_json::Value` so that unknown fields and the
//! original representation of untouched values are preserved. Only the four
//! shortcut fields and (on Windows) the shortcut schema marker are ever
//! rewritten.

use serde_json::{Map, Value};

use crate::shortcut::model::{
    self, build_conflict_graph, NormalizedShortcut, ShortcutField,
};

/// Highest Windows shortcut-schema version this build understands.
pub const SUPPORTED_WINDOWS_SHORTCUT_SCHEMA_VERSION: u64 = 1;
/// On-disk key for the shortcut-only schema version.
pub const SCHEMA_VERSION_KEY: &str = "windowsShortcutSchemaVersion";

/// Raw schema marker as found on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSchemaVersion {
    Missing,
    Version(u64),
    /// Non-integer, negative or otherwise not safely interpretable.
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutSchemaMode {
    /// Schema is understood and automatic (shortcut-only) maintenance is safe.
    Supported,
    /// Schema is newer than this build or of an unknown type: never auto-write.
    Future,
}

pub fn parse_schema_version(raw: &Value) -> RawSchemaVersion {
    let Some(value) = raw
        .get("general")
        .and_then(|general| general.get(SCHEMA_VERSION_KEY))
    else {
        return RawSchemaVersion::Missing;
    };
    match value {
        Value::Null => RawSchemaVersion::Missing,
        Value::Number(number) => match number.as_u64() {
            Some(version) => RawSchemaVersion::Version(version),
            None => RawSchemaVersion::Unsupported,
        },
        _ => RawSchemaVersion::Unsupported,
    }
}

pub fn schema_mode(raw: &Value) -> ShortcutSchemaMode {
    match parse_schema_version(raw) {
        RawSchemaVersion::Missing => ShortcutSchemaMode::Supported,
        RawSchemaVersion::Version(version)
            if version <= SUPPORTED_WINDOWS_SHORTCUT_SCHEMA_VERSION =>
        {
            ShortcutSchemaMode::Supported
        }
        RawSchemaVersion::Version(_) | RawSchemaVersion::Unsupported => ShortcutSchemaMode::Future,
    }
}

/// Platform default for one field.
///
/// Only Windows changes the voice shortcut default to the physical right Alt;
/// every other platform keeps the existing `F4`.
pub fn platform_default(field: ShortcutField, is_windows: bool) -> &'static str {
    match (field, is_windows) {
        (ShortcutField::Shortcut, true) => model::ALT_RIGHT,
        (ShortcutField::Shortcut, false) => "F4",
        (ShortcutField::Shortcut2, _) => "",
        (ShortcutField::ExtractShortcut, _) => "F6",
        (ShortcutField::ExtractShortcut2, _) => "",
    }
}

/// Read the raw JSON value for a field, or `None` when absent.
pub fn raw_field<'a>(raw: &'a Value, field: ShortcutField) -> Option<&'a Value> {
    raw.get("general").and_then(|general| general.get(field.key()))
}

/// Normalize the current raw value of a field for conflict/validation purposes.
/// Non-string values are treated as invalid (never registered) without being
/// rewritten.
pub fn normalized_field(raw: &Value, field: ShortcutField) -> NormalizedShortcut {
    match raw_field(raw, field) {
        None => NormalizedShortcut::Empty,
        Some(Value::Null) => NormalizedShortcut::Empty,
        Some(Value::String(value)) => model::normalize(value),
        Some(_) => NormalizedShortcut::Invalid("<非字符串值>".to_string()),
    }
}

pub fn normalized_fields(raw: &Value) -> [NormalizedShortcut; 4] {
    [
        normalized_field(raw, ShortcutField::Shortcut),
        normalized_field(raw, ShortcutField::Shortcut2),
        normalized_field(raw, ShortcutField::ExtractShortcut),
        normalized_field(raw, ShortcutField::ExtractShortcut2),
    ]
}

/// Ensure `raw` is an object and `raw["general"]` is an object, returning the
/// general map.
pub fn ensure_general_object(raw: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !raw.is_object() {
        *raw = Value::Object(Map::new());
    }
    let root = raw.as_object_mut().expect("just ensured object");
    let general = root
        .entry("general".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !general.is_object() {
        *general = Value::Object(Map::new());
    }
    Ok(general.as_object_mut().expect("just ensured object"))
}

/// Write a field value as a JSON string. Preserves everything else.
pub fn set_field(raw: &mut Value, field: ShortcutField, value: &str) -> Result<(), String> {
    let general = ensure_general_object(raw)?;
    general.insert(field.key().to_string(), Value::String(value.to_string()));
    Ok(())
}

/// Deep-merge `incoming` into `target`. Objects merge recursively; arrays and
/// scalars are replaced. Unknown keys already present in `target` survive.
fn merge_value(target: &mut Value, incoming: Value) {
    match (target, incoming) {
        (Value::Object(target_map), Value::Object(incoming_map)) => {
            for (key, value) in incoming_map {
                match target_map.get_mut(&key) {
                    Some(slot) => merge_value(slot, value),
                    None => {
                        target_map.insert(key, value);
                    }
                }
            }
        }
        (slot, replacement) => *slot = replacement,
    }
}

/// Merge a serialized `AppConfig` into the latest raw JSON.
///
/// The four shortcut fields and the shortcut schema marker are *always* taken
/// from `raw` (the authoritative latest persisted state); they are only ever
/// changed through the shortcut-specific update path. This prevents a stale
/// frontend `AppConfig` snapshot from overwriting shortcuts or downgrading a
/// higher schema.
pub fn merge_config_preserving_shortcuts(raw: &mut Value, incoming: Value) {
    let protected: Vec<(String, Option<Value>)> = std::iter::once(SCHEMA_VERSION_KEY.to_string())
        .chain(ShortcutField::ALL.iter().map(|field| field.key().to_string()))
        .map(|key| {
            let value = raw
                .get("general")
                .and_then(|general| general.get(&key))
                .cloned();
            (key, value)
        })
        .collect();

    merge_value(raw, incoming);

    if let Ok(general) = ensure_general_object(raw) {
        for (key, value) in protected {
            match value {
                Some(value) => {
                    general.insert(key, value);
                }
                None => {
                    general.remove(&key);
                }
            }
        }
    }
}

/// Produce a `deserialize`-friendly copy where non-string shortcut values are
/// replaced by `""` so a single malformed field cannot reset the whole config.
pub fn sanitized_for_deserialize(raw: &Value) -> Value {
    let mut sanitized = raw.clone();
    if let Ok(general) = ensure_general_object(&mut sanitized) {
        for field in ShortcutField::ALL {
            if let Some(value) = general.get(field.key()) {
                if !value.is_string() {
                    general.insert(field.key().to_string(), Value::String(String::new()));
                }
            }
        }
    }
    sanitized
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortcutMigrationOutcome {
    /// The candidate differs from the input and (when supported) should be
    /// persisted.
    pub changed: bool,
    /// Schema v1 check finished; on Windows the marker is set to `1`.
    pub schema_completed: bool,
    /// The disk schema is newer/unknown; nothing may be auto-written.
    pub future_mode: bool,
    pub f4_replaced: bool,
    pub f4_kept_conflict: bool,
    pub diagnostics: Vec<String>,
}

/// Run the shortcut part of raw JSON migration.
///
/// `schema_lt_1` must be computed from the *original* schema marker before this
/// call. When `is_windows` is false the Windows schema marker is never created
/// or advanced.
pub fn migrate_shortcuts(
    raw: &mut Value,
    is_windows: bool,
    schema_lt_1: bool,
) -> ShortcutMigrationOutcome {
    let mut outcome = ShortcutMigrationOutcome::default();

    if schema_mode(raw) == ShortcutSchemaMode::Future {
        outcome.future_mode = true;
        return outcome;
    }

    if let Err(error) = ensure_general_object(raw) {
        outcome.diagnostics.push(error);
        return outcome;
    }

    // 1. Normalize missing / null / blank / Backspace for the four fields.
    for field in ShortcutField::ALL {
        let existing = raw_field(raw, field).cloned();
        match existing {
            None => {
                let default = platform_default(field, is_windows);
                let _ = set_field(raw, field, default);
                outcome.changed = true;
                outcome
                    .diagnostics
                    .push(format!("字段 {} 缺失，补全为 {}", field.key(), value_label(default)));
            }
            Some(Value::Null) => {
                let _ = set_field(raw, field, "");
                outcome.changed = true;
                outcome
                    .diagnostics
                    .push(format!("字段 {} 为 null，规范化为空", field.key()));
            }
            Some(Value::String(value)) => match model::normalize(&value) {
                NormalizedShortcut::Empty => {
                    if !value.is_empty() {
                        let _ = set_field(raw, field, "");
                        outcome.changed = true;
                        outcome.diagnostics.push(format!(
                            "字段 {} 的空白或 Backspace 值规范化为空",
                            field.key()
                        ));
                    }
                }
                NormalizedShortcut::Valid(_) => {
                    // Preserve valid user values verbatim.
                }
                NormalizedShortcut::Invalid(raw_value) => {
                    // Preserve the original value; only record a diagnostic.
                    outcome.diagnostics.push(format!(
                        "字段 {} 含无法识别的值 {}，运行时将跳过注册",
                        field.key(),
                        value_label(&raw_value)
                    ));
                }
            },
            Some(other) => {
                outcome.diagnostics.push(format!(
                    "字段 {} 为非字符串值 {}，保留原始 JSON 并跳过注册",
                    field.key(),
                    other
                ));
            }
        }
    }

    // 2. Windows schema v1: F4 -> AltRight, checked against the full candidate.
    if is_windows && schema_lt_1 {
        let shortcut = normalized_field(raw, ShortcutField::Shortcut);
        if shortcut.canonical() == Some("F4") {
            let mut candidate = normalized_fields(raw);
            candidate[ShortcutField::Shortcut.index()] =
                NormalizedShortcut::Valid(model::ALT_RIGHT.to_string());
            if build_conflict_graph(&candidate).has_conflicts() {
                outcome.f4_kept_conflict = true;
                outcome.diagnostics.push(
                    "F4 → AltRight 会产生冲突，保留 F4 并保留其他用户快捷键".to_string(),
                );
            } else {
                let _ = set_field(raw, ShortcutField::Shortcut, model::ALT_RIGHT);
                outcome.changed = true;
                outcome.f4_replaced = true;
            }
        }
    }

    // 3. Advance the Windows schema marker only after a completed check.
    if is_windows && schema_lt_1 {
        if let Ok(general) = ensure_general_object(raw) {
            general.insert(
                SCHEMA_VERSION_KEY.to_string(),
                Value::Number(SUPPORTED_WINDOWS_SHORTCUT_SCHEMA_VERSION.into()),
            );
            outcome.changed = true;
            outcome.schema_completed = true;
        }
    }

    outcome
}

fn value_label(value: &str) -> String {
    if value.is_empty() {
        "空".to_string()
    } else {
        format!("\"{}\"", value)
    }
}

/// One-line summary of the on-disk shortcut state, for diagnostics.
pub fn raw_shortcut_summary(raw: &Value) -> String {
    let mut parts = Vec::new();
    for field in ShortcutField::ALL {
        let value = raw
            .get("general")
            .and_then(|general| general.get(field.key()));
        parts.push(format!(
            "{}={}",
            field.key(),
            value
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<missing>".to_string())
        ));
    }
    format!(
        "schema={:?} {}",
        parse_schema_version(raw),
        parts.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn general_shortcut(raw: &Value) -> Option<&str> {
        raw.get("general")
            .and_then(|g| g.get("shortcut"))
            .and_then(|v| v.as_str())
    }

    fn schema(raw: &Value) -> Option<u64> {
        raw.get("general")
            .and_then(|g| g.get(SCHEMA_VERSION_KEY))
            .and_then(|v| v.as_u64())
    }

    #[test]
    fn parses_missing_and_null_schema_as_version_zero() {
        assert_eq!(parse_schema_version(&json!({})), RawSchemaVersion::Missing);
        assert_eq!(
            parse_schema_version(&json!({ "general": { SCHEMA_VERSION_KEY: null } })),
            RawSchemaVersion::Missing
        );
        assert_eq!(
            parse_schema_version(&json!({ "general": { SCHEMA_VERSION_KEY: 1 } })),
            RawSchemaVersion::Version(1)
        );
        assert_eq!(
            parse_schema_version(&json!({ "general": { SCHEMA_VERSION_KEY: "x" } })),
            RawSchemaVersion::Unsupported
        );
    }

    #[test]
    fn m01_windows_new_config_defaults_to_alt_right() {
        let mut raw = json!({ "general": {} });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("AltRight"));
        assert_eq!(schema(&raw), Some(1));
        assert!(outcome.changed);
    }

    #[test]
    fn m02_f4_without_conflict_becomes_alt_right() {
        let mut raw = json!({ "general": { "shortcut": "F4", "extractShortcut": "F6" } });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("AltRight"));
        assert_eq!(schema(&raw), Some(1));
        assert!(outcome.f4_replaced);
    }

    #[test]
    fn m03_f4_with_alt_combo_is_kept() {
        let mut raw = json!({ "general": { "shortcut": "F4", "shortcut2": "Alt+A" } });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("F4"));
        assert_eq!(raw["general"]["shortcut2"], "Alt+A");
        assert_eq!(schema(&raw), Some(1));
        assert!(outcome.f4_kept_conflict);
    }

    #[test]
    fn m04_f4_with_existing_alt_right_is_kept() {
        let mut raw = json!({ "general": { "shortcut": "F4", "shortcut2": "AltRight" } });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("F4"));
        assert_eq!(schema(&raw), Some(1));
        assert!(outcome.f4_kept_conflict);
    }

    #[test]
    fn m05_f8_is_preserved() {
        let mut raw = json!({ "general": { "shortcut": "F8" } });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("F8"));
        assert_eq!(schema(&raw), Some(1));
        assert!(!outcome.f4_replaced);
    }

    #[test]
    fn m06_alt_right_is_preserved() {
        let mut raw = json!({ "general": { "shortcut": "AltRight" } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("AltRight"));
        assert_eq!(schema(&raw), Some(1));
    }

    #[test]
    fn m07_empty_is_preserved() {
        let mut raw = json!({ "general": { "shortcut": "" } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some(""));
        assert_eq!(schema(&raw), Some(1));
    }

    #[test]
    fn m08_null_becomes_empty() {
        let mut raw = json!({ "general": { "shortcut": null } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some(""));
    }

    #[test]
    fn m09_blank_becomes_empty() {
        let mut raw = json!({ "general": { "shortcut": "   " } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some(""));
    }

    #[test]
    fn m10_backspace_becomes_empty() {
        for value in ["Backspace", "Ctrl+Backspace", "Alt+Backspace"] {
            let mut raw = json!({ "general": { "shortcut": value } });
            migrate_shortcuts(&mut raw, true, true);
            assert_eq!(general_shortcut(&raw), Some(""), "value={value}");
        }
    }

    #[test]
    fn m11_unknown_value_is_preserved() {
        let mut raw = json!({ "general": { "shortcut": "NotAKey" } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(general_shortcut(&raw), Some("NotAKey"));
        assert_eq!(schema(&raw), Some(1));
    }

    #[test]
    fn m12_empty_extract_shortcut_is_never_restored() {
        let mut raw = json!({ "general": { "extractShortcut": "" } });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(raw["general"]["extractShortcut"], "");
    }

    #[test]
    fn m13_missing_extract_shortcut_defaults_to_f6() {
        let mut raw = json!({ "general": {} });
        migrate_shortcuts(&mut raw, true, true);
        assert_eq!(raw["general"]["extractShortcut"], "F6");
    }

    #[test]
    fn m16_schema_one_f4_is_not_migrated() {
        let mut raw = json!({ "general": { "shortcut": "F4", SCHEMA_VERSION_KEY: 1 } });
        let outcome = migrate_shortcuts(&mut raw, true, false);
        assert_eq!(general_shortcut(&raw), Some("F4"));
        assert!(!outcome.f4_replaced);
    }

    #[test]
    fn m17_future_schema_is_untouched() {
        let mut raw = json!({ "general": { "shortcut": "F4", SCHEMA_VERSION_KEY: 2 } });
        let before = raw.clone();
        let outcome = migrate_shortcuts(&mut raw, true, false);
        assert!(outcome.future_mode);
        assert_eq!(raw, before);
    }

    #[test]
    fn m18_non_windows_does_not_touch_schema() {
        let mut raw = json!({ "general": { "shortcut": "F4" } });
        let outcome = migrate_shortcuts(&mut raw, false, false);
        assert_eq!(general_shortcut(&raw), Some("F4"));
        assert_eq!(schema(&raw), None);
        assert!(!outcome.schema_completed);
    }

    #[test]
    fn non_string_value_is_preserved_but_sanitized_for_deserialize() {
        let mut raw = json!({ "general": { "shortcut": ["oops"] } });
        let outcome = migrate_shortcuts(&mut raw, true, true);
        assert_eq!(raw["general"]["shortcut"], json!(["oops"]));
        assert!(outcome.diagnostics.iter().any(|d| d.contains("非字符串")));
        let sanitized = sanitized_for_deserialize(&raw);
        assert_eq!(sanitized["general"]["shortcut"], "");
    }

    #[test]
    fn unknown_fields_survive_merge() {
        let mut raw = json!({
            "general": { "shortcut": "AltRight", SCHEMA_VERSION_KEY: 1, "futureField": 7 },
            "unknownSection": { "keep": true }
        });
        let incoming = json!({
            "general": { "shortcut": "F4", "launchAtLogin": true },
            "localApi": { "enabled": false, "port": 8765 }
        });
        merge_config_preserving_shortcuts(&mut raw, incoming);
        assert_eq!(general_shortcut(&raw), Some("AltRight"));
        assert_eq!(schema(&raw), Some(1));
        assert_eq!(raw["general"]["futureField"], 7);
        assert_eq!(raw["unknownSection"]["keep"], true);
        assert_eq!(raw["general"]["launchAtLogin"], true);
    }
}
