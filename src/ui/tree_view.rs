use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;

/// Format a token count compactly: 1234 → "1.2k", 12345 → "12k", 123456 → "123k"
fn format_tokens_short(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 10_000 {
        format!("{}k", n / 1_000)
    } else if n >= 1_000 {
        format!("{}.{}k", n / 1_000, (n % 1_000) / 100)
    } else {
        n.to_string()
    }
}

/// Map a fraction (0.0–1.0) to a heat color from dark to bright.
fn heat_color(fraction: f64) -> Color {
    if fraction > 0.5 {
        Color::Red
    } else if fraction > 0.2 {
        Color::Yellow
    } else if fraction > 0.05 {
        Color::Green
    } else {
        Color::DarkGray
    }
}

const BAR_WIDTH: usize = 10;
// token count label (e.g. " 12.3k") + 1 space + bar
const TOKEN_COL_WIDTH: usize = 7 + 1 + BAR_WIDTH;

pub fn draw_tree(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize; // inside borders

    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    } else if app.selected >= app.scroll_offset + inner_height {
        app.scroll_offset = app.selected - inner_height + 1;
    }

    let total_tokens = app.token_estimate.max(1);

    let mut lines: Vec<Line> = Vec::new();
    let visible_end = (app.scroll_offset + inner_height).min(app.rows.len());

    for i in app.scroll_offset..visible_end {
        let row = &app.rows[i];
        let is_selected = i == app.selected;
        let is_search_match = app.search_matches.contains(&i);

        let indent = "  ".repeat(row.depth);
        let arrow = if row.expandable {
            if row.expanded { "▼ " } else { "▶ " }
        } else {
            "  "
        };

        let key_style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_search_match {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let val_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::White)
        } else if is_search_match {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(row.type_color)
        };

        // Build left side: indent + arrow + key + preview
        let left_prefix = format!("{}{}", indent, arrow);
        let key_text = format!("{}: ", row.key);
        let left_len = left_prefix.len() + key_text.len() + row.preview.len();

        // Build right side: token bar
        let fraction = row.tokens as f64 / total_tokens as f64;
        let filled = (fraction * BAR_WIDTH as f64).ceil() as usize;
        let empty = BAR_WIDTH.saturating_sub(filled);
        let bar_filled: String = "█".repeat(filled);
        let bar_empty: String = "░".repeat(empty);
        let token_label = format!("{:>6} ", format_tokens_short(row.tokens));

        // Pad between left content and right-aligned token column
        let pad_len = inner_width
            .saturating_sub(left_len + TOKEN_COL_WIDTH);
        let padding = " ".repeat(pad_len);

        let bar_color = if is_selected {
            Color::Black
        } else {
            heat_color(fraction)
        };
        let dim_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Line::from(vec![
            Span::styled(
                left_prefix,
                if is_selected {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(key_text, key_style),
            Span::styled(row.preview.clone(), val_style),
            Span::styled(padding, dim_style),
            Span::styled(token_label, dim_style),
            Span::styled(bar_filled, Style::default().fg(bar_color)),
            Span::styled(bar_empty, Style::default().fg(Color::DarkGray)),
        ]);
        lines.push(line);
    }

    let title = format!(" Tree ({} nodes) ", app.rows.len());
    let block = Block::default().borders(Borders::ALL).title(title);
    let tree_widget = Paragraph::new(lines).block(block);
    frame.render_widget(tree_widget, area);

    if app.rows.len() > inner_height {
        let mut scrollbar_state = ScrollbarState::new(app.rows.len().saturating_sub(inner_height))
            .position(app.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}
