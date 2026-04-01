use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;

use super::html::strip_html;

/// Pretty-print a JSON value into styled lines, rendering string values with
/// real newlines instead of JSON-escaped `\n`.
pub fn pretty_print_value(val: &Value, indent: usize, max_width: usize) -> Vec<Line<'static>> {
    let indent_str = "  ".repeat(indent);
    let key_style = Style::default().fg(Color::Cyan);
    let str_style = Style::default().fg(Color::Green);
    let num_style = Style::default().fg(Color::Cyan);
    let bool_style = Style::default().fg(Color::Yellow);
    let null_style = Style::default().fg(Color::DarkGray);
    let brace_style = Style::default().fg(Color::DarkGray);

    match val {
        Value::Object(obj) => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("{}{{", indent_str),
                brace_style,
            )));
            let entries: Vec<_> = obj.iter().collect();
            for (i, (k, v)) in entries.iter().enumerate() {
                let comma = if i + 1 < entries.len() { "," } else { "" };
                match v {
                    Value::String(s) if s.contains('\n') || s.len() > 80 => {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}  ", indent_str), brace_style),
                            Span::styled(format!("\"{}\": ", k), key_style),
                        ]));
                        let text_indent = format!("{}    ", indent_str);
                        let text_width = max_width.saturating_sub(indent * 2 + 4);
                        let cleaned = strip_html(s, false);
                        lines.push(Line::from(Span::styled(
                            format!("{}┌─", text_indent),
                            Style::default().fg(Color::DarkGray),
                        )));
                        for text_line in cleaned.lines() {
                            if text_width > 0 && text_line.len() > text_width {
                                let words: Vec<&str> = text_line.split_whitespace().collect();
                                let mut current = String::new();
                                for word in words {
                                    if current.is_empty() {
                                        current = word.to_string();
                                    } else if current.len() + 1 + word.len() > text_width {
                                        lines.push(Line::from(vec![
                                            Span::styled(
                                                format!("{}│ ", text_indent),
                                                Style::default().fg(Color::DarkGray),
                                            ),
                                            Span::styled(current.clone(), str_style),
                                        ]));
                                        current = word.to_string();
                                    } else {
                                        current.push(' ');
                                        current.push_str(word);
                                    }
                                }
                                if !current.is_empty() {
                                    lines.push(Line::from(vec![
                                        Span::styled(
                                            format!("{}│ ", text_indent),
                                            Style::default().fg(Color::DarkGray),
                                        ),
                                        Span::styled(current, str_style),
                                    ]));
                                }
                            } else {
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        format!("{}│ ", text_indent),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::styled(text_line.to_string(), str_style),
                                ]));
                            }
                        }
                        lines.push(Line::from(Span::styled(
                            format!("{}└─{}", text_indent, comma),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    Value::Object(_) | Value::Array(_) => {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}  ", indent_str), brace_style),
                            Span::styled(format!("\"{}\": ", k), key_style),
                        ]));
                        lines.extend(pretty_print_value(v, indent + 1, max_width));
                        if !comma.is_empty() {
                            if let Some(last) = lines.last_mut() {
                                last.spans.push(Span::styled(",", brace_style));
                            }
                        }
                    }
                    _ => {
                        let val_span = match v {
                            Value::String(s) => Span::styled(format!("\"{}\"", s), str_style),
                            Value::Number(n) => Span::styled(n.to_string(), num_style),
                            Value::Bool(b) => Span::styled(b.to_string(), bool_style),
                            Value::Null => Span::styled("null", null_style),
                            _ => Span::raw(""),
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}  ", indent_str), brace_style),
                            Span::styled(format!("\"{}\": ", k), key_style),
                            val_span,
                            Span::styled(comma.to_string(), brace_style),
                        ]));
                    }
                }
            }
            lines.push(Line::from(Span::styled(
                format!("{}}}", indent_str),
                brace_style,
            )));
            lines
        }
        Value::Array(arr) => {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("{}[", indent_str),
                brace_style,
            )));
            for (i, item) in arr.iter().enumerate() {
                let comma = if i + 1 < arr.len() { "," } else { "" };
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        lines.extend(pretty_print_value(item, indent + 1, max_width));
                        if !comma.is_empty() {
                            if let Some(last) = lines.last_mut() {
                                last.spans.push(Span::styled(",", brace_style));
                            }
                        }
                    }
                    Value::String(s) => {
                        let display = if s.len() > 60 {
                            let clean: String = s
                                .chars()
                                .take(57)
                                .map(|c| if c == '\n' { ' ' } else { c })
                                .collect();
                            format!("\"{}...\"", clean)
                        } else {
                            format!("\"{}\"", s.replace('\n', "\\n"))
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}  ", indent_str), brace_style),
                            Span::styled(display, str_style),
                            Span::styled(comma.to_string(), brace_style),
                        ]));
                    }
                    _ => {
                        let val_span = match item {
                            Value::Number(n) => Span::styled(n.to_string(), num_style),
                            Value::Bool(b) => Span::styled(b.to_string(), bool_style),
                            Value::Null => Span::styled("null", null_style),
                            _ => Span::raw(""),
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("{}  ", indent_str), brace_style),
                            val_span,
                            Span::styled(comma.to_string(), brace_style),
                        ]));
                    }
                }
            }
            lines.push(Line::from(Span::styled(
                format!("{}]", indent_str),
                brace_style,
            )));
            lines
        }
        Value::String(s) => {
            vec![Line::from(Span::styled(
                format!("{}\"{}\"", indent_str, s),
                str_style,
            ))]
        }
        Value::Number(n) => {
            vec![Line::from(Span::styled(
                format!("{}{}", indent_str, n),
                num_style,
            ))]
        }
        Value::Bool(b) => {
            vec![Line::from(Span::styled(
                format!("{}{}", indent_str, b),
                bool_style,
            ))]
        }
        Value::Null => {
            vec![Line::from(Span::styled(
                format!("{}null", indent_str),
                null_style,
            ))]
        }
    }
}
