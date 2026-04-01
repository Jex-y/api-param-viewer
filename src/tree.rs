use ratatui::style::Color;
use serde_json::Value;

#[derive(Clone)]
pub struct TreeRow {
    pub depth: usize,
    pub key: String,
    pub preview: String,
    pub expandable: bool,
    pub expanded: bool,
    pub path: Vec<PathSeg>,
    pub type_color: Color,
    pub tokens: usize,
}

pub fn estimate_tokens(value: &Value) -> usize {
    serde_json::to_string(value).unwrap_or_default().len() / 4
}

#[derive(Clone)]
pub enum PathSeg {
    Key(String),
    Index(usize),
}

pub fn path_key(path: &[PathSeg]) -> String {
    path.iter()
        .map(|s| match s {
            PathSeg::Key(k) => k.clone(),
            PathSeg::Index(i) => i.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn value_preview(value: &Value) -> (String, bool, Color) {
    match value {
        Value::Null => ("null".into(), false, Color::DarkGray),
        Value::Bool(b) => (b.to_string(), false, Color::Yellow),
        Value::Number(n) => (n.to_string(), false, Color::Cyan),
        Value::String(s) => {
            let clean: String = s
                .chars()
                .take(80)
                .map(|c| if c == '\n' { ' ' } else { c })
                .collect();
            let display = if s.len() > 80 {
                format!("\"{}...\" ({} chars)", clean, s.len())
            } else {
                format!("\"{}\"", clean)
            };
            (display, false, Color::Green)
        }
        Value::Array(arr) => (format!("[{}]", arr.len()), true, Color::Magenta),
        Value::Object(obj) => (object_preview(obj), true, Color::Blue),
    }
}

fn object_preview(obj: &serde_json::Map<String, Value>) -> String {
    let label_fields = ["name", "role", "type", "id", "model", "title"];

    let mut parts: Vec<String> = Vec::new();
    for field in &label_fields {
        if let Some(Value::String(v)) = obj.get(*field) {
            let display = if v.len() > 30 {
                format!("{}...", &v[..27])
            } else {
                v.clone()
            };
            parts.push(format!("{}={}", field, display));
        }
    }

    if let Some(Value::String(t)) = obj.get("type") {
        match t.as_str() {
            "tool_use" => {
                if let Some(Value::String(name)) = obj.get("name") {
                    if !parts.iter().any(|p| p.starts_with("name=")) {
                        parts.push(format!("name={}", name));
                    }
                }
            }
            "tool_result" => {
                if let Some(Value::String(id)) = obj.get("tool_use_id") {
                    let short = if id.len() > 20 { &id[..20] } else { id };
                    if !parts.iter().any(|p| p.starts_with("id=")) {
                        parts.push(format!("tool_id={}...", short));
                    }
                }
            }
            "text" => {
                if let Some(Value::String(text)) = obj.get("text") {
                    parts.push(format!("{} chars", text.len()));
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        let keys: Vec<_> = obj.keys().take(3).cloned().collect();
        let suffix = if obj.len() > 3 { ", ..." } else { "" };
        format!("{{{}{}}}", keys.join(", "), suffix)
    } else {
        format!("{{{}}}", parts.join(", "))
    }
}

pub fn array_child_label(index: usize, item: &Value) -> String {
    if let Value::Object(obj) = item {
        let mut tag_parts: Vec<String> = Vec::new();

        if let Some(Value::String(role)) = obj.get("role") {
            tag_parts.push(role.clone());
        }
        if let Some(Value::String(t)) = obj.get("type") {
            tag_parts.push(t.clone());
            if t == "tool_use" {
                if let Some(Value::String(name)) = obj.get("name") {
                    tag_parts.push(name.clone());
                }
            }
        }
        if let Some(Value::String(name)) = obj.get("name") {
            if !tag_parts.contains(name) {
                tag_parts.push(name.clone());
            }
        }

        if tag_parts.is_empty() {
            format!("[{}]", index)
        } else {
            format!("[{}] {}", index, tag_parts.join(" "))
        }
    } else {
        format!("[{}]", index)
    }
}
