use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::app::App;

pub fn draw_tree(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;

    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    } else if app.selected >= app.scroll_offset + inner_height {
        app.scroll_offset = app.selected - inner_height + 1;
    }

    let mut lines: Vec<Line> = Vec::new();
    let visible_end = (app.scroll_offset + inner_height).min(app.rows.len());

    for i in app.scroll_offset..visible_end {
        let row = &app.rows[i];
        let is_selected = i == app.selected;
        let is_search_match = app.search_matches.contains(&i);

        let indent = "  ".repeat(row.depth);
        let arrow = if row.expandable {
            if row.expanded {
                "▼ "
            } else {
                "▶ "
            }
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

        let line = Line::from(vec![
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
