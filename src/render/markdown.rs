use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Render a single line of markdown-ish text into styled spans.
pub fn render_markdown_line(line: &str) -> Line<'static> {
    let trimmed = line.trim_start();

    // Headers
    if trimmed.starts_with("### ") {
        return Line::from(Span::styled(
            format!("   {}", &trimmed[4..]),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if trimmed.starts_with("## ") {
        return Line::from(Span::styled(
            format!("  {}", &trimmed[3..]),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if trimmed.starts_with("# ") {
        return Line::from(Span::styled(
            trimmed[2..].to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Horizontal rule
    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return Line::from(Span::styled(
            "────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Bullet lists
    let (prefix, rest) = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let indent = line.len() - trimmed.len();
        (
            format!("{}  ", " ".repeat(indent)),
            trimmed[2..].to_string(),
        )
    } else {
        (String::new(), line.to_string())
    };

    // Inline formatting: **bold**, *italic*, `code`
    let mut spans: Vec<Span<'static>> = Vec::new();
    if !prefix.is_empty() {
        spans.push(Span::styled(prefix, Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled("• ", Style::default().fg(Color::Cyan)));
    }

    let chars: Vec<char> = rest.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    while i < len {
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 2;
            let mut bold_text = String::new();
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            spans.push(Span::styled(
                bold_text,
                Style::default().add_modifier(Modifier::BOLD),
            ));
        } else if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 1;
            let mut code_text = String::new();
            while i < len && chars[i] != '`' {
                code_text.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push(Span::styled(code_text, Style::default().fg(Color::Green)));
        } else if chars[i] == '*' && i + 1 < len && chars[i + 1] != ' ' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 1;
            let mut italic_text = String::new();
            while i < len && chars[i] != '*' {
                italic_text.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push(Span::styled(
                italic_text,
                Style::default().add_modifier(Modifier::ITALIC),
            ));
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }

    Line::from(spans)
}
