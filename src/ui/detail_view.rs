use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use serde_json::Value;

use crate::app::App;
use crate::render::{render_markdown_line, render_table, strip_html, wrap_text};
use crate::render::json::pretty_print_value;
use crate::render::table::is_table_row;

pub fn draw_detail(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;

    let title = if let Some(row) = app.rows.get(app.selected) {
        format!(" {} ", row.key)
    } else {
        " Detail ".to_string()
    };

    let val = app.selected_value();

    let all_lines: Vec<Line<'static>> = match val {
        Some(Value::String(s)) => render_string_content(s, inner_width),
        Some(val) => pretty_print_value(val, 0, inner_width),
        None => vec![],
    };

    let total_lines = all_lines.len();
    let display_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(app.detail_scroll)
        .take(inner_height)
        .collect();

    let scroll_info = format!(
        " {}/{} ",
        (app.detail_scroll + 1).min(total_lines.max(1)),
        total_lines.max(1)
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(scroll_info);
    let detail_widget = Paragraph::new(display_lines).block(block);
    frame.render_widget(detail_widget, area);

    if total_lines > inner_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(inner_height))
            .position(app.detail_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

fn render_string_content(s: &str, max_width: usize) -> Vec<Line<'static>> {
    let raw_lines: Vec<&str> = s.lines().collect();
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut i = 0;

    while i < raw_lines.len() {
        let raw_line = raw_lines[i];

        let trimmed_raw = raw_line.trim_start();
        if trimmed_raw.starts_with("```") {
            in_code_block = !in_code_block;
            lines.push(Line::from(Span::styled(
                "─── code ───",
                Style::default().fg(Color::DarkGray),
            )));
            i += 1;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                raw_line.to_string(),
                Style::default().fg(Color::Green),
            )));
            i += 1;
            continue;
        }

        if is_table_row(raw_line) {
            let table_start = i;
            while i < raw_lines.len() && is_table_row(raw_lines[i]) {
                i += 1;
            }
            let cleaned_rows: Vec<String> = raw_lines[table_start..i]
                .iter()
                .map(|l| strip_html(l, true))
                .collect();
            let cleaned_refs: Vec<&str> = cleaned_rows.iter().map(|s| s.as_str()).collect();
            lines.extend(render_table(&cleaned_refs, max_width));
            continue;
        }

        let cleaned = strip_html(raw_line, false);

        for sub_line in cleaned.lines() {
            if max_width > 0 && sub_line.len() > max_width {
                let wrapped = wrap_text(sub_line, max_width);
                for wl in wrapped {
                    lines.push(render_markdown_line(&wl));
                }
            } else {
                lines.push(render_markdown_line(sub_line));
            }
        }
        i += 1;
    }
    lines
}
