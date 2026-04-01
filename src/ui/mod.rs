mod detail_view;
mod tree_view;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;

pub fn draw(frame: &mut ratatui::Frame, app: &mut App) {
    let size = frame.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    // Breadcrumb
    let breadcrumb = app.breadcrumb();
    let selected_tokens = format_k(app.selected_token_estimate());
    let total_tokens = format_k(app.token_estimate);
    let context_limit = format_k(app.context_limit);
    let bc_line = Line::from(vec![
        Span::styled(" Path: ", Style::default().fg(Color::DarkGray)),
        Span::styled(breadcrumb, Style::default().fg(Color::White)),
        Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("~{} tokens", selected_tokens), Style::default().fg(Color::Yellow)),
        Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("~{}/{} ctx ({})", total_tokens, context_limit, app.model_name), Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(bc_line), outer[0]);

    // Main split
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    tree_view::draw_tree(frame, app, main_chunks[0]);
    detail_view::draw_detail(frame, app, main_chunks[1]);

    // Bottom bar
    draw_status_bar(frame, app, outer[2]);
}

fn format_k(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn draw_status_bar(frame: &mut ratatui::Frame, app: &App, area: ratatui::layout::Rect) {
    if app.search_mode {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(app.search_query.clone()),
            Span::styled("_", Style::default().fg(Color::Yellow)),
            if !app.search_matches.is_empty() {
                Span::styled(
                    format!(
                        " ({}/{})",
                        app.search_match_idx.map(|i| i + 1).unwrap_or(0),
                        app.search_matches.len()
                    ),
                    Style::default().fg(Color::DarkGray),
                )
            } else if !app.search_query.is_empty() {
                Span::styled(" (no matches)", Style::default().fg(Color::Red))
            } else {
                Span::raw("")
            },
        ]);
        frame.render_widget(Paragraph::new(search_line), area);
    } else {
        let help = Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" nav  "),
            Span::styled("h/l", Style::default().fg(Color::Yellow)),
            Span::raw(" collapse/expand  "),
            Span::styled("space", Style::default().fg(Color::Yellow)),
            Span::raw(" toggle  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("n/N", Style::default().fg(Color::Yellow)),
            Span::raw(" next/prev  "),
            Span::styled("d/u", Style::default().fg(Color::Yellow)),
            Span::raw(" scroll detail  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]);
        frame.render_widget(Paragraph::new(help), area);
    }
}
