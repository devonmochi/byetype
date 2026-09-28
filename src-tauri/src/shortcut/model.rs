//! Pure shortcut domain logic: field identities, normalization, validation and
//! conflict detection.
//!
//! This module deliberately has no dependency on Tauri or on the Windows
//! backend so that it can be unit-tested on any platform and reused by both the
//! config migration layer and the runtime registration layer.

use serde::{Deserialize, Serialize};

/// Canonical internal representation of the physical right Alt key.
pub const ALT_RIGHT: &str = "AltRight";
/// Canonical internal representation of the physical left Alt key.
///
/// It is intentionally distinct from [`ALT_RIGHT`]. This requirement does not
/// add a dedicated left-Alt single-key feature, but keeping the identity lets
/// conflict detection and diagnostics tell the two apart.
#[allow(dead_code)]
pub const ALT_LEFT: &str = "AltLeft";

/// The four shortcut fields managed by this feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShortcutField {
    #[serde(rename = "shortcut")]
    Shortcut,
    #[serde(rename = "shortcut2")]
    Shortcut2,
    #[serde(rename = "extractShortcut")]
    ExtractShortcut,
    #[serde(rename = "extractShortcut2")]
    ExtractShortcut2,
}

impl ShortcutField {
    pub const ALL: [ShortcutField; 4] = [
        ShortcutField::Shortcut,
        ShortcutField::Shortcut2,
        ShortcutField::ExtractShortcut,
        ShortcutField::ExtractShortcut2,
    ];

    /// Field name as it appears in the on-disk / JavaScript config.
    pub fn key(self) -> &'static str {
        match self {
            ShortcutField::Shortcut => "shortcut",
            ShortcutField::Shortcut2 => "shortcut2",
            ShortcutField::ExtractShortcut => "extractShortcut",
            ShortcutField::ExtractShortcut2 => "extractShortcut2",
        }
    }

    /// Field name as it appears in the Rust `GeneralConfig` struct.
    #[allow(dead_code)]
    pub fn rust_key(self) -> &'static str {
        match self {
            ShortcutField::Shortcut => "shortcut",
            ShortcutField::Shortcut2 => "shortcut2",
            ShortcutField::ExtractShortcut => "extract_shortcut",
            ShortcutField::ExtractShortcut2 => "extract_shortcut2",
        }
    }

    /// User facing label, used in conflict diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ShortcutField::Shortcut => "语音输入 1",
            ShortcutField::Shortcut2 => "语音输入 2",
            ShortcutField::ExtractShortcut => "截图取词",
            ShortcutField::ExtractShortcut2 => "截图翻译",
        }
    }

    pub fn parse_key(key: &str) -> Option<ShortcutField> {
        ShortcutField::ALL.into_iter().find(|f| f.key() == key)
    }

    pub fn index(self) -> usize {
        match self {
            ShortcutField::Shortcut => 0,
            ShortcutField::Shortcut2 => 1,
            ShortcutField::ExtractShortcut => 2,
            ShortcutField::ExtractShortcut2 => 3,
        }
    }
}

/// Result of normalizing one raw shortcut value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedShortcut {
    /// Blank, `null`, or a Backspace-primary shortcut. Never registered.
    Empty,
    /// Syntactically valid shortcut, stored in canonical form.
    Valid(String),
    /// Non-empty but not understood. Preserved on disk, never registered.
    Invalid(String),
}

impl NormalizedShortcut {
    pub fn canonical(&self) -> Option<&str> {
        match self {
            NormalizedShortcut::Valid(v) => Some(v.as_str()),
            _ => None,
        }
    }
}

/// A modifier token found while parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

/// Normalize a raw shortcut string.
///
/// Order of operations matches requirement §8.1:
/// trim → Backspace primary-key handling → modifier alias unification →
/// modifier ordering → main-key representation → syntax validation.
pub fn normalize(raw: &str) -> NormalizedShortcut {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return NormalizedShortcut::Empty;
    }

    let tokens: Vec<&str> = trimmed.split('+').map(|t| t.trim()).collect();
    if tokens.iter().any(|t| t.is_empty()) {
        return NormalizedShortcut::Invalid(trimmed.to_string());
    }

    let mut modifiers: Vec<Modifier> = Vec::new();
    let mut main_key: Option<String> = None;

    for token in tokens {
        if let Some(main) = to_main_key(token) {
            // A primary key was already found: two primary keys is invalid.
            if main_key.is_some() {
                return NormalizedShortcut::Invalid(trimmed.to_string());
            }
            if main.eq_ignore_ascii_case("BACKSPACE") {
                // Backspace as primary key means "delete this shortcut", no
                // matter which modifiers accompany it (requirement §7.1).
                return NormalizedShortcut::Empty;
            }
            main_key = Some(main);
            continue;
        }

        match to_modifier(token) {
            Some(modifier) => {
                // Modifiers must appear before the primary key.
                if main_key.is_some() {
                    return NormalizedShortcut::Invalid(trimmed.to_string());
                }
                if !modifiers.contains(&modifier) {
                    modifiers.push(modifier);
                }
            }
            None => return NormalizedShortcut::Invalid(trimmed.to_string()),
        }
    }

    let Some(main) = main_key else {
        // Modifier-only input is not a registrable shortcut.
        return NormalizedShortcut::Invalid(trimmed.to_string());
    };

    if main == ALT_RIGHT && !modifiers.is_empty() {
        // The native right-Alt backend only implements the bare key. Do not
        // invent new combination formats for this requirement (§5.1).
        return NormalizedShortcut::Invalid(trimmed.to_string());
    }

    NormalizedShortcut::Valid(join_canonical(&modifiers, &main))
}

fn join_canonical(modifiers: &[Modifier], main: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    // Fixed, platform independent ordering so that "Ctrl+Shift+A" and
    // "Shift+Ctrl+A" collapse to the same value.
    if modifiers.contains(&Modifier::Ctrl) {
        parts.push("Ctrl");
    }
    if modifiers.contains(&Modifier::Alt) {
        parts.push("Alt");
    }
    if modifiers.contains(&Modifier::Shift) {
        parts.push("Shift");
    }
    if modifiers.contains(&Modifier::Super) {
        parts.push("Super");
    }
    parts.push(main);
    parts.join("+")
}

fn to_modifier(token: &str) -> Option<Modifier> {
    match token.to_ascii_uppercase().as_str() {
        "CTRL" | "CONTROL" | "CONTROLLEFT" | "CONTROLRIGHT" => Some(Modifier::Ctrl),
        "ALT" | "OPTION" | "ALTLEFT" | "LEFTALT" | "LALT" => Some(Modifier::Alt),
        "SHIFT" | "SHIFTLEFT" | "SHIFTRIGHT" => Some(Modifier::Shift),
        "SUPER" | "CMD" | "COMMAND" | "META" | "WIN" | "WINDOWS" | "METALEFT" | "METARIGHT"
        | "SUPERLEFT" | "SUPERRIGHT" => Some(Modifier::Super),
        // CommandOrControl resolves per platform: Super on macOS, Ctrl elsewhere.
        "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
            #[cfg(target_os = "macos")]
            {
                Some(Modifier::Super)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Some(Modifier::Ctrl)
            }
        }
        _ => None,
    }
}

/// Map a token to its canonical primary-key name, or `None` if the token is not
/// a known primary key. `"BACKSPACE"` is returned verbatim so the caller can
/// apply the delete semantics.
fn to_main_key(token: &str) -> Option<String> {
    let upper = token.to_ascii_uppercase();
    let canonical = match upper.as_str() {
        "ALTRIGHT" | "RIGHTALT" | "RALT" => ALT_RIGHT,
        "BACKSPACE" => return Some("BACKSPACE".to_string()),
        "BACKQUOTE" | "`" => "Backquote",
        "BACKSLASH" | "\\" => "Backslash",
        "BRACKETLEFT" | "[" => "BracketLeft",
        "BRACKETRIGHT" | "]" => "BracketRight",
        "PAUSE" | "PAUSEBREAK" => "Pause",
        "COMMA" | "," => "Comma",
        "DIGIT0" | "0" => "0",
        "DIGIT1" | "1" => "1",
        "DIGIT2" | "2" => "2",
        "DIGIT3" | "3" => "3",
        "DIGIT4" | "4" => "4",
        "DIGIT5" | "5" => "5",
        "DIGIT6" | "6" => "6",
        "DIGIT7" | "7" => "7",
        "DIGIT8" | "8" => "8",
        "DIGIT9" | "9" => "9",
        "EQUAL" | "=" => "Equal",
        "MINUS" | "-" => "Minus",
        "PERIOD" | "." => "Period",
        "QUOTE" | "'" => "Quote",
        "SEMICOLON" | ";" => "Semicolon",
        "SLASH" | "/" => "Slash",
        "CAPSLOCK" => "CapsLock",
        "ENTER" | "RETURN" => "Enter",
        "SPACE" | "SPACEBAR" => "Space",
        "TAB" => "Tab",
        "DELETE" | "DEL" => "Delete",
        "END" => "End",
        "HOME" => "Home",
        "INSERT" | "INS" => "Insert",
        "PAGEDOWN" => "PageDown",
        "PAGEUP" => "PageUp",
        "PRINTSCREEN" => "PrintScreen",
        "SCROLLLOCK" => "ScrollLock",
        "ARROWDOWN" | "DOWN" => "ArrowDown",
        "ARROWLEFT" | "LEFT" => "ArrowLeft",
        "ARROWRIGHT" | "RIGHT" => "ArrowRight",
        "ARROWUP" | "UP" => "ArrowUp",
        "NUMLOCK" => "NumLock",
        "NUMPAD0" | "NUM0" => "Numpad0",
        "NUMPAD1" | "NUM1" => "Numpad1",
        "NUMPAD2" | "NUM2" => "Numpad2",
        "NUMPAD3" | "NUM3" => "Numpad3",
        "NUMPAD4" | "NUM4" => "Numpad4",
        "NUMPAD5" | "NUM5" => "Numpad5",
        "NUMPAD6" | "NUM6" => "Numpad6",
        "NUMPAD7" | "NUM7" => "Numpad7",
        "NUMPAD8" | "NUM8" => "Numpad8",
        "NUMPAD9" | "NUM9" => "Numpad9",
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => "NumpadAdd",
        "NUMPADDECIMAL" | "NUMDECIMAL" => "NumpadDecimal",
        "NUMPADDIVIDE" | "NUMDIVIDE" => "NumpadDivide",
        "NUMPADENTER" | "NUMENTER" => "NumpadEnter",
        "NUMPADEQUAL" | "NUMEQUAL" => "NumpadEqual",
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => "NumpadMultiply",
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => "NumpadSubtract",
        "ESCAPE" | "ESC" => "Escape",
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => "AudioVolumeDown",
        "AUDIOVOLUMEUP" | "VOLUMEUP" => "AudioVolumeUp",
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => "AudioVolumeMute",
        "MEDIAPLAY" => "MediaPlay",
        "MEDIAPAUSE" => "MediaPause",
        "MEDIAPLAYPAUSE" => "MediaPlayPause",
        "MEDIASTOP" => "MediaStop",
        "MEDIATRACKNEXT" => "MediaTrackNext",
        "MEDIATRACKPREV" | "MEDIATRACKPREVIOUS" => "MediaTrackPrevious",
        _ => {
            // Single ASCII letters and F1..F24.
            if upper.len() == 1 && upper.as_bytes()[0].is_ascii_uppercase() {
                return Some(upper);
            }
            if let Some(rest) = upper.strip_prefix('F') {
                if let Ok(n) = rest.parse::<u8>() {
                    if (1..=24).contains(&n) {
                        return Some(format!("F{n}"));
                    }
                }
            }
            return None;
        }
    };
    Some(canonical.to_string())
}

/// Whether a canonical shortcut uses the generic Alt modifier.
pub fn has_alt_modifier(canonical: &str) -> bool {
    canonical.split('+').any(|part| part == "Alt")
}

/// Whether a canonical shortcut is the bare right-Alt key.
pub fn is_alt_right(canonical: &str) -> bool {
    canonical == ALT_RIGHT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Two fields normalize to the same non-empty shortcut.
    Duplicate,
    /// An `AltRight` field coexists with a field that uses the Alt modifier.
    AltPrefix,
}

impl ConflictKind {
    pub fn code(self) -> &'static str {
        match self {
            ConflictKind::Duplicate => "SHORTCUT_CONFLICT",
            ConflictKind::AltPrefix => "ALT_PREFIX_CONFLICT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictPair {
    pub left: ShortcutField,
    pub right: ShortcutField,
    pub kind: ConflictKind,
}

/// Deterministic conflict graph built from the four normalized candidate
/// values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConflictGraph {
    /// Per-field flag: the field participates in at least one conflict and must
    /// therefore be skipped at registration time.
    pub skipped: [bool; 4],
    pub pairs: Vec<ConflictPair>,
}

impl ConflictGraph {
    /// True when at least one pair conflicts.
    pub fn has_conflicts(&self) -> bool {
        !self.pairs.is_empty()
    }
}

/// Build the conflict graph. Only syntactically valid, non-empty values take
/// part: invalid values are never registered, so they cannot conflict.
///
/// The result does not depend on iteration order or hashing: pairs are emitted
/// in field order.
pub fn build_conflict_graph(values: &[NormalizedShortcut; 4]) -> ConflictGraph {
    let mut graph = ConflictGraph::default();
    for i in 0..4 {
        for j in (i + 1)..4 {
            let (Some(a), Some(b)) = (values[i].canonical(), values[j].canonical()) else {
                continue;
            };
            let kind = if a == b {
                Some(ConflictKind::Duplicate)
            } else if (is_alt_right(a) && has_alt_modifier(b))
                || (is_alt_right(b) && has_alt_modifier(a))
            {
                Some(ConflictKind::AltPrefix)
            } else {
                None
            };
            if let Some(kind) = kind {
                let left = ShortcutField::ALL[i];
                let right = ShortcutField::ALL[j];
                graph.skipped[i] = true;
                graph.skipped[j] = true;
                graph.pairs.push(ConflictPair { left, right, kind });
            }
        }
    }
    graph
}

/// Validate a set of candidate values that the user explicitly submitted.
///
/// `changed` marks the fields the user actually edited. Only changed fields are
/// required to be syntactically valid; unchanged fields keep their existing
/// (possibly unknown) value. Conflicts are always evaluated across all four
/// normalized values.
pub fn validate_candidates(
    values: &[NormalizedShortcut; 4],
    changed: &[bool; 4],
) -> Result<(), ShortcutValidationError> {
    for i in 0..4 {
        if changed[i] {
            if let NormalizedShortcut::Invalid(raw) = &values[i] {
                return Err(ShortcutValidationError {
                    code: "INVALID_SHORTCUT",
                    message: format!("无法识别的快捷键：{}", raw),
                    fields: vec![ShortcutField::ALL[i]],
                });
            }
        }
    }

    let graph = build_conflict_graph(values);
    if let Some(pair) = graph.pairs.first() {
        let message = match pair.kind {
            ConflictKind::Duplicate => format!(
                "{} 与 {} 使用了相同的快捷键，请设置不同的快捷键",
                pair.left.label(),
                pair.right.label()
            ),
            ConflictKind::AltPrefix => format!(
                "{} 与 {} 存在“右 Alt”前缀冲突",
                pair.left.label(),
                pair.right.label()
            ),
        };
        return Err(ShortcutValidationError {
            code: pair.kind.code(),
            message,
            fields: vec![pair.left, pair.right],
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutValidationError {
    pub code: &'static str,
    pub message: String,
    pub fields: Vec<ShortcutField>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(s: &str) -> NormalizedShortcut {
        NormalizedShortcut::Valid(s.to_string())
    }

    fn norm(s: &str) -> NormalizedShortcut {
        normalize(s)
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(norm("  F4  "), valid("F4"));
        assert_eq!(norm("   "), NormalizedShortcut::Empty);
        assert_eq!(norm(""), NormalizedShortcut::Empty);
    }

    #[test]
    fn unifies_modifier_order_and_aliases() {
        assert_eq!(norm("Ctrl+Shift+A"), norm("Shift+Ctrl+A"));
        assert_eq!(norm("Ctrl+Shift+A"), valid("Ctrl+Shift+A"));
        assert_eq!(norm("control+A"), valid("Ctrl+A"));
        assert_eq!(norm("Option+F8"), valid("Alt+F8"));
        assert_eq!(norm("cmd+A"), valid("Super+A"));
        assert_eq!(norm("Win+A"), valid("Super+A"));
    }

    #[test]
    fn normalizes_main_key_representation() {
        assert_eq!(norm("f4"), valid("F4"));
        assert_eq!(norm("esc"), valid("Escape"));
        assert_eq!(norm("space"), valid("Space"));
        assert_eq!(norm("numpad1"), valid("Numpad1"));
    }

    #[test]
    fn alt_right_aliases_normalize() {
        assert_eq!(norm("AltRight"), valid("AltRight"));
        assert_eq!(norm("RightAlt"), valid("AltRight"));
        assert_eq!(norm("RAlt"), valid("AltRight"));
    }

    #[test]
    fn alt_left_is_not_alt_right() {
        assert_eq!(norm("AltLeft"), NormalizedShortcut::Invalid("AltLeft".to_string()));
        assert_ne!(norm("AltRight"), norm("AltLeft"));
    }

    #[test]
    fn alt_right_with_modifiers_is_unsupported() {
        assert!(matches!(norm("Ctrl+AltRight"), NormalizedShortcut::Invalid(_)));
    }

    #[test]
    fn backspace_primary_key_is_empty_regardless_of_modifiers() {
        for raw in [
            "Backspace",
            "backspace",
            "Ctrl+Backspace",
            "Alt+Backspace",
            "Shift+Backspace",
            "Meta+Backspace",
            "Ctrl+Shift+Backspace",
            "Ctrl+Alt+Backspace",
        ] {
            assert_eq!(norm(raw), NormalizedShortcut::Empty, "raw={raw}");
        }
    }

    #[test]
    fn modifier_only_and_multi_key_are_invalid() {
        assert!(matches!(norm("Ctrl"), NormalizedShortcut::Invalid(_)));
        assert!(matches!(norm("Ctrl+Alt"), NormalizedShortcut::Invalid(_)));
        assert!(matches!(norm("Ctrl+A+B"), NormalizedShortcut::Invalid(_)));
        assert!(matches!(norm("A+Ctrl"), NormalizedShortcut::Invalid(_)));
        assert!(matches!(norm("Ctrl++A"), NormalizedShortcut::Invalid(_)));
    }

    #[test]
    fn unknown_non_empty_is_invalid() {
        assert!(matches!(norm("NotAKey"), NormalizedShortcut::Invalid(_)));
    }

    #[test]
    fn duplicate_conflict_flags_both_nodes() {
        let values = [valid("F8"), valid("F8"), NormalizedShortcut::Empty, NormalizedShortcut::Empty];
        let graph = build_conflict_graph(&values);
        assert_eq!(graph.skipped, [true, true, false, false]);
        assert_eq!(graph.pairs.len(), 1);
        assert_eq!(graph.pairs[0].kind, ConflictKind::Duplicate);
    }

    #[test]
    fn multiple_empty_is_allowed() {
        let values = [
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        assert!(!build_conflict_graph(&values).has_conflicts());
    }

    #[test]
    fn alt_right_prefix_conflict() {
        let values = [
            valid("AltRight"),
            valid("Alt+A"),
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        let graph = build_conflict_graph(&values);
        assert_eq!(graph.skipped, [true, true, false, false]);
        assert_eq!(graph.pairs[0].kind, ConflictKind::AltPrefix);
    }

    #[test]
    fn ctrl_alt_also_conflicts_with_alt_right() {
        let values = [
            valid("AltRight"),
            valid("Ctrl+Alt+A"),
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        assert!(build_conflict_graph(&values).has_conflicts());
    }

    #[test]
    fn conflict_graph_is_order_independent() {
        let a = build_conflict_graph(&[
            valid("AltRight"),
            valid("Alt+A"),
            valid("F8"),
            valid("F8"),
        ]);
        let b = build_conflict_graph(&[
            valid("AltRight"),
            valid("Alt+A"),
            valid("F8"),
            valid("F8"),
        ]);
        assert_eq!(a, b);
        assert!(a.skipped[0] && a.skipped[1] && a.skipped[2] && a.skipped[3]);
    }

    #[test]
    fn invalid_and_empty_never_conflict() {
        let values = [
            NormalizedShortcut::Invalid("Bad".to_string()),
            NormalizedShortcut::Invalid("Bad".to_string()),
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        assert!(!build_conflict_graph(&values).has_conflicts());
    }

    #[test]
    fn validate_rejects_invalid_changed_field() {
        let values = [
            NormalizedShortcut::Invalid("Bad".to_string()),
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        let err = validate_candidates(&values, &[true, false, false, false]).unwrap_err();
        assert_eq!(err.code, "INVALID_SHORTCUT");
    }

    #[test]
    fn validate_allows_unchanged_invalid_field() {
        let values = [
            NormalizedShortcut::Invalid("Bad".to_string()),
            valid("F8"),
            NormalizedShortcut::Empty,
            NormalizedShortcut::Empty,
        ];
        assert!(validate_candidates(&values, &[false, true, false, false]).is_ok());
    }

    #[test]
    fn validate_reports_duplicate() {
        let values = [valid("F8"), valid("F8"), NormalizedShortcut::Empty, NormalizedShortcut::Empty];
        let err = validate_candidates(&values, &[true, false, false, false]).unwrap_err();
        assert_eq!(err.code, "SHORTCUT_CONFLICT");
    }
}
