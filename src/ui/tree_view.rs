use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;

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

const TOKEN_COL_WIDTH: u16 = 16; // "  6.2k ████████"

pub fn draw_tree(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    // Draw the outer block first, then split the inner area
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Tree ({} nodes) ", app.rows.len()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_height = inner.height as usize;

    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    } else if app.selected >= app.scroll_offset + inner_height {
        app.scroll_offset = app.selected - inner_height + 1;
    }

    // Split inner area: tree content on left, token bars on right
    let col_width = TOKEN_COL_WIDTH.min(inner.width / 3);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(col_width),
        ])
        .split(inner);

    let tree_area = chunks[0];
    let token_area = chunks[1];
    let bar_max = (col_width as usize).saturating_sub(7); // label takes ~7 chars

    let total_tokens = app.context_limit.max(1);

    let mut tree_lines: Vec<Line> = Vec::new();
    let mut token_lines: Vec<Line> = Vec::new();
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

        // Tree content line
        tree_lines.push(Line::from(vec![
            Span::styled(
                format!("{}{}", indent, arrow),
                if is_selected {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
            Span::styled(format!("{}: ", row.key), key_style),
            Span::styled(row.preview.clone(), val_style),
        ]));

        // Token bar line
        let fraction = row.tokens as f64 / total_tokens as f64;
        let filled = (fraction * bar_max as f64).ceil() as usize;
        let empty = bar_max.saturating_sub(filled);
        let bar_color = heat_color(fraction);
        let label_style = if is_selected {
            Style::default().fg(bar_color).bg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let empty_style = if is_selected {
            Style::default().fg(Color::DarkGray).bg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        token_lines.push(Line::from(vec![
            Span::styled(format!("{:>6} ", format_tokens_short(row.tokens)), label_style),
            Span::styled("█".repeat(filled), Style::default().fg(bar_color).bg(if is_selected { Color::White } else { Color::Reset })),
            Span::styled("░".repeat(empty), empty_style),
        ]));
    }

    frame.render_widget(Paragraph::new(tree_lines), tree_area);
    frame.render_widget(Paragraph::new(token_lines), token_area);

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
