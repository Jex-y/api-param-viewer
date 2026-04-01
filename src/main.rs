use std::io;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Terminal,
};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "api-param-viewer", about = "TUI viewer for LLM API param files")]
struct Cli {
    /// Path to the JSON API params file
    file: String,
}

#[derive(Clone)]
struct TreeRow {
    depth: usize,
    key: String,
    preview: String,
    expandable: bool,
    expanded: bool,
    path: Vec<PathSeg>,
    type_color: Color,
}

#[derive(Clone)]
enum PathSeg {
    Key(String),
    Index(usize),
}

struct App {
    root: Value,
    rows: Vec<TreeRow>,
    selected: usize,
    scroll_offset: usize,
    detail_scroll: usize,
    expanded_paths: std::collections::HashSet<String>,
    search_mode: bool,
    search_query: String,
    search_matches: Vec<usize>,
    search_match_idx: Option<usize>,
}

/// Build a rich preview string for JSON objects, pulling out semantically useful fields.
fn object_preview(obj: &serde_json::Map<String, Value>) -> String {
    // Priority fields to show as identifying info
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

    // For content blocks, show extra context
    if let Some(Value::String(t)) = obj.get("type") {
        match t.as_str() {
            "tool_use" => {
                if let Some(Value::String(name)) = obj.get("name") {
                    // name already captured above, but ensure it's there
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
                    let chars: usize = text.len();
                    parts.push(format!("{} chars", chars));
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

/// Build a label for an array child that includes semantic info from the item.
fn array_child_label(index: usize, item: &Value) -> String {
    if let Value::Object(obj) = item {
        // Build a short tag from key identifying fields
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

impl App {
    fn new(root: Value) -> Self {
        let mut app = App {
            root,
            rows: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            detail_scroll: 0,
            expanded_paths: std::collections::HashSet::new(),
            search_mode: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: None,
        };
        app.expanded_paths.insert(String::new());
        app.rebuild_rows();
        app
    }

    fn path_key(path: &[PathSeg]) -> String {
        path.iter()
            .map(|s| match s {
                PathSeg::Key(k) => k.clone(),
                PathSeg::Index(i) => i.to_string(),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        let root = self.root.clone();
        self.build_rows(&root, 0, &[], "root");
    }

    fn build_rows(&mut self, value: &Value, depth: usize, path: &[PathSeg], key: &str) {
        let pk = Self::path_key(path);
        let is_expanded = self.expanded_paths.contains(&pk);

        let (preview, expandable, type_color) = match value {
            Value::Null => ("null".into(), false, Color::DarkGray),
            Value::Bool(b) => (b.to_string(), false, Color::Yellow),
            Value::Number(n) => (n.to_string(), false, Color::Cyan),
            Value::String(s) => {
                let clean: String = s.chars().take(80).map(|c| if c == '\n' { ' ' } else { c }).collect();
                let display = if s.len() > 80 {
                    format!("\"{}...\" ({} chars)", clean, s.len())
                } else {
                    format!("\"{}\"", clean)
                };
                (display, false, Color::Green)
            }
            Value::Array(arr) => (format!("[{}]", arr.len()), true, Color::Magenta),
            Value::Object(obj) => {
                (object_preview(obj), true, Color::Blue)
            }
        };

        self.rows.push(TreeRow {
            depth,
            key: key.to_string(),
            preview,
            expandable,
            expanded: is_expanded,
            path: path.to_vec(),
            type_color,
        });

        if is_expanded {
            match value {
                Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSeg::Index(i));
                        let label = array_child_label(i, item);
                        self.build_rows(item, depth + 1, &child_path, &label);
                    }
                }
                Value::Object(obj) => {
                    for (k, v) in obj.iter() {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSeg::Key(k.clone()));
                        self.build_rows(v, depth + 1, &child_path, k);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_path(&self, path: &[PathSeg]) -> &Value {
        let mut current = &self.root;
        for seg in path {
            current = match seg {
                PathSeg::Key(k) => &current[k],
                PathSeg::Index(i) => &current[*i],
            };
        }
        current
    }

    fn toggle_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            if row.expandable {
                let pk = Self::path_key(&row.path);
                if self.expanded_paths.contains(&pk) {
                    let prefix = if pk.is_empty() {
                        String::new()
                    } else {
                        format!("{}/", pk)
                    };
                    self.expanded_paths
                        .retain(|p| p != &pk && !p.starts_with(&prefix));
                } else {
                    self.expanded_paths.insert(pk);
                }
                self.rebuild_rows();
                if self.selected >= self.rows.len() {
                    self.selected = self.rows.len().saturating_sub(1);
                }
            }
        }
        self.detail_scroll = 0;
    }

    fn expand_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            if row.expandable && !row.expanded {
                let pk = Self::path_key(&row.path);
                self.expanded_paths.insert(pk);
                self.rebuild_rows();
            }
        }
    }

    fn collapse_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            let pk = Self::path_key(&row.path);
            if row.expandable && row.expanded {
                let prefix = if pk.is_empty() {
                    String::new()
                } else {
                    format!("{}/", pk)
                };
                self.expanded_paths
                    .retain(|p| p != &pk && !p.starts_with(&prefix));
                self.rebuild_rows();
            } else if row.depth > 0 {
                let target_depth = row.depth - 1;
                for i in (0..self.selected).rev() {
                    if self.rows[i].depth == target_depth {
                        self.selected = i;
                        break;
                    }
                }
            }
        }
        self.detail_scroll = 0;
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
        self.detail_scroll = 0;
    }

    fn page_down(&mut self, page_size: usize) {
        self.selected = (self.selected + page_size).min(self.rows.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    fn update_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_matches.clear();
        self.search_match_idx = None;
        if query.is_empty() {
            return;
        }
        for (i, row) in self.rows.iter().enumerate() {
            if row.key.to_lowercase().contains(&query)
                || row.preview.to_lowercase().contains(&query)
            {
                self.search_matches.push(i);
            }
        }
        if !self.search_matches.is_empty() {
            self.search_match_idx = Some(0);
            self.selected = self.search_matches[0];
            self.detail_scroll = 0;
        }
    }

    fn next_search_match(&mut self) {
        if let Some(idx) = self.search_match_idx {
            let next = (idx + 1) % self.search_matches.len();
            self.search_match_idx = Some(next);
            self.selected = self.search_matches[next];
            self.detail_scroll = 0;
        }
    }

    fn prev_search_match(&mut self) {
        if let Some(idx) = self.search_match_idx {
            let prev = if idx == 0 {
                self.search_matches.len() - 1
            } else {
                idx - 1
            };
            self.search_match_idx = Some(prev);
            self.selected = self.search_matches[prev];
            self.detail_scroll = 0;
        }
    }

    fn selected_value(&self) -> Option<&Value> {
        self.rows.get(self.selected).map(|row| self.resolve_path(&row.path))
    }

    fn breadcrumb(&self) -> String {
        if let Some(row) = self.rows.get(self.selected) {
            let parts: Vec<String> = std::iter::once("$".to_string())
                .chain(row.path.iter().map(|s| match s {
                    PathSeg::Key(k) => k.clone(),
                    PathSeg::Index(i) => format!("[{}]", i),
                }))
                .collect();
            parts.join(".")
        } else {
            String::new()
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let content = std::fs::read_to_string(&cli.file)?;
    let root: Value = serde_json::from_str(&content)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(root);

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app.search_mode {
                    match key.code {
                        KeyCode::Esc => app.search_mode = false,
                        KeyCode::Enter => app.search_mode = false,
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.update_search();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.update_search();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                            app.expand_selected()
                        }
                        KeyCode::Left | KeyCode::Char('h') => app.collapse_selected(),
                        KeyCode::Char(' ') => app.toggle_selected(),
                        KeyCode::PageUp => app.page_up(20),
                        KeyCode::PageDown => app.page_down(20),
                        KeyCode::Home => {
                            app.selected = 0;
                            app.detail_scroll = 0;
                        }
                        KeyCode::End => {
                            app.selected = app.rows.len().saturating_sub(1);
                            app.detail_scroll = 0;
                        }
                        KeyCode::Char('/') => {
                            app.search_mode = true;
                            app.search_query.clear();
                            app.search_matches.clear();
                            app.search_match_idx = None;
                        }
                        KeyCode::Char('n') => app.next_search_match(),
                        KeyCode::Char('N') => app.prev_search_match(),
                        KeyCode::Char('d') => app.detail_scroll += 5,
                        KeyCode::Char('u') => {
                            app.detail_scroll = app.detail_scroll.saturating_sub(5)
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn draw(frame: &mut ratatui::Frame, app: &mut App) {
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
    let bc_line = Line::from(vec![
        Span::styled(" Path: ", Style::default().fg(Color::DarkGray)),
        Span::styled(breadcrumb, Style::default().fg(Color::White)),
    ]);
    frame.render_widget(Paragraph::new(bc_line), outer[0]);

    // Main split
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(outer[1]);

    draw_tree(frame, app, main_chunks[0]);
    draw_detail(frame, app, main_chunks[1]);

    // Bottom bar
    if app.search_mode {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
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
        frame.render_widget(Paragraph::new(search_line), outer[2]);
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
        frame.render_widget(Paragraph::new(help), outer[2]);
    }
}

fn draw_tree(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
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

/// Strip HTML tags. If `inline` is true, tags are simply removed (for table cells).
/// If false, block tags like </p> and <br/> insert newlines.
fn strip_html(s: &str, inline: bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();

    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            in_tag = true;
            tag_buf.clear();
        } else if c == '>' && in_tag {
            in_tag = false;
            if !inline {
                let tag_lower = tag_buf.to_lowercase();
                let tag_name = tag_lower.split_whitespace().next().unwrap_or("");
                match tag_name {
                    "br" | "br/" | "br /" => out.push('\n'),
                    "/p" => out.push_str("\n\n"),
                    "hr" | "hr/" => out.push_str("\n---\n"),
                    "/li" => out.push('\n'),
                    _ => {}
                }
            }
        } else if in_tag {
            tag_buf.push(c);
        } else if c == '&' {
            let mut entity = String::new();
            for ec in chars.by_ref() {
                if ec == ';' {
                    break;
                }
                entity.push(ec);
                if entity.len() > 10 {
                    break;
                }
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                "nbsp" => out.push(' '),
                _ => {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Render a markdown-ish line into styled spans.
fn render_markdown_line(line: &str) -> Line<'static> {
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
        spans.push(Span::styled(
            prefix,
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::styled("• ", Style::default().fg(Color::Cyan)));
    }

    let chars: Vec<char> = rest.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    while i < len {
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            // Flush buffer
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            // Find closing **
            i += 2;
            let mut bold_text = String::new();
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip closing **
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
            spans.push(Span::styled(
                code_text,
                Style::default().fg(Color::Green),
            ));
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

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 1
}

fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !is_table_row(trimmed) {
        return false;
    }
    trimmed[1..trimmed.len() - 1]
        .chars()
        .all(|c| c == '-' || c == '|' || c == ':' || c == ' ')
}

fn parse_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading/trailing pipe
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// Word-wrap text into lines of at most `width` characters, breaking on spaces.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }
    let mut current = String::new();
    for word in &words {
        if current.is_empty() {
            // Single word longer than width — just push it, it'll be padded/clipped
            current = word.to_string();
        } else if current.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn render_table(table_lines: &[&str], max_width: usize) -> Vec<Line<'static>> {
    if table_lines.is_empty() {
        return vec![];
    }

    let rows: Vec<Vec<String>> = table_lines
        .iter()
        .filter(|l| !is_separator_row(l))
        .map(|l| parse_table_cells(l))
        .collect();

    if rows.is_empty() {
        return vec![];
    }

    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return vec![];
    }

    // Border overhead: outer pipes (2) + inner pipes (num_cols - 1) + padding (2 per col)
    let overhead = 2 + num_cols.saturating_sub(1) + num_cols * 2;
    let content_budget = max_width.saturating_sub(overhead);

    // Distribute width among columns. Give each column a fair share, but let
    // short columns use less so long columns get more room.
    let mut col_widths: Vec<usize> = vec![0; num_cols];

    // First pass: find the natural (max content) width of each column
    let mut natural: Vec<usize> = vec![0; num_cols];
    for row in &rows {
        for (j, cell) in row.iter().enumerate() {
            if j < num_cols {
                natural[j] = natural[j].max(cell.len());
            }
        }
    }

    let total_natural: usize = natural.iter().sum();
    if total_natural <= content_budget || content_budget == 0 {
        col_widths = natural.iter().map(|w| (*w).max(1)).collect();
    } else {
        // Give each column at least a min, then distribute remaining space proportionally
        let min_col: usize = 5;
        let mut remaining = content_budget;
        let mut fixed = vec![false; num_cols];

        // First: columns that fit within min get their natural size
        for j in 0..num_cols {
            if natural[j] <= min_col {
                col_widths[j] = natural[j].max(1);
                fixed[j] = true;
                remaining = remaining.saturating_sub(col_widths[j]);
            }
        }

        // Distribute remaining among unfixed columns proportionally
        let unfixed_natural: usize = (0..num_cols)
            .filter(|j| !fixed[*j])
            .map(|j| natural[j])
            .sum();

        if unfixed_natural > 0 {
            for j in 0..num_cols {
                if !fixed[j] {
                    let share = (natural[j] as f64 / unfixed_natural as f64 * remaining as f64) as usize;
                    col_widths[j] = share.max(min_col);
                }
            }
        }
    }

    let border_style = Style::default().fg(Color::DarkGray);
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let cell_style = Style::default().fg(Color::White);

    let mut output: Vec<Line<'static>> = Vec::new();

    // Helper to build a horizontal border line
    let make_border = |left: &str, mid: &str, right: &str| -> Line<'static> {
        let mut s = String::from(left);
        for (i, &w) in col_widths.iter().enumerate() {
            s.push_str(&"─".repeat(w + 2));
            if i + 1 < num_cols {
                s.push_str(mid);
            }
        }
        s.push_str(right);
        Line::from(Span::styled(s, border_style))
    };

    output.push(make_border("┌", "┬", "┐"));

    for (row_idx, row) in rows.iter().enumerate() {
        let style = if row_idx == 0 { header_style } else { cell_style };

        // Word-wrap each cell and find the max number of visual lines in this row
        let wrapped: Vec<Vec<String>> = (0..num_cols)
            .map(|j| {
                let cell = row.get(j).map(|s| s.as_str()).unwrap_or("");
                wrap_text(cell, col_widths[j])
            })
            .collect();

        let num_visual_lines = wrapped.iter().map(|w| w.len()).max().unwrap_or(1);

        for vline in 0..num_visual_lines {
            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::styled("│", border_style));

            for (j, w) in col_widths.iter().enumerate() {
                let text = wrapped[j]
                    .get(vline)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                // Truncate if a single word is still wider than column
                let display: String = if text.len() > *w {
                    text.chars().take(w.saturating_sub(1)).collect::<String>() + "…"
                } else {
                    text.to_string()
                };
                let padded = format!(" {:<width$} ", display, width = w);
                spans.push(Span::styled(padded, style));
                spans.push(Span::styled("│", border_style));
            }
            output.push(Line::from(spans));
        }

        // After header: separator. Between data rows: light separator.
        if row_idx == 0 {
            output.push(make_border("├", "┼", "┤"));
        } else if row_idx + 1 < rows.len() {
            output.push(make_border("├", "┼", "┤"));
        }
    }

    output.push(make_border("└", "┴", "┘"));

    output
}

fn draw_detail(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;

    let title = if let Some(row) = app.rows.get(app.selected) {
        format!(" {} ", row.key)
    } else {
        " Detail ".to_string()
    };

    let val = app.selected_value();

    // For strings: render as formatted text with markdown/HTML support
    // For everything else: pretty-print JSON with syntax highlighting
    let all_lines: Vec<Line<'static>> = match val {
        Some(Value::String(s)) => {
            // Split on actual newlines in the raw string first, before any HTML processing.
            // This preserves table rows that contain HTML within cells.
            let raw_lines: Vec<&str> = s.lines().collect();
            let mut lines = Vec::new();
            let mut in_code_block = false;
            let mut i = 0;

            while i < raw_lines.len() {
                let raw_line = raw_lines[i];

                // Check for code blocks (on the raw line before HTML strip)
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

                // Detect markdown table on the raw lines (before HTML stripping).
                // Table rows start and end with | even if they contain HTML inside.
                if is_table_row(raw_line) {
                    let table_start = i;
                    while i < raw_lines.len() && is_table_row(raw_lines[i]) {
                        i += 1;
                    }
                    // Strip HTML inline (no newlines) for each table row
                    let cleaned_rows: Vec<String> = raw_lines[table_start..i]
                        .iter()
                        .map(|l| strip_html(l, true))
                        .collect();
                    let cleaned_refs: Vec<&str> = cleaned_rows.iter().map(|s| s.as_str()).collect();
                    lines.extend(render_table(&cleaned_refs, inner_width));
                    continue;
                }

                // For non-table lines, strip HTML with block-level newlines
                let cleaned = strip_html(raw_line, false);

                // The cleaned result may now contain multiple lines (from <br/>, </p>)
                for sub_line in cleaned.lines() {
                    if inner_width > 0 && sub_line.len() > inner_width {
                        let words: Vec<&str> = sub_line.split_whitespace().collect();
                        let mut current_line = String::new();
                        for word in words {
                            if current_line.is_empty() {
                                current_line = word.to_string();
                            } else if current_line.len() + 1 + word.len() > inner_width {
                                lines.push(render_markdown_line(&current_line));
                                current_line = word.to_string();
                            } else {
                                current_line.push(' ');
                                current_line.push_str(word);
                            }
                        }
                        if !current_line.is_empty() {
                            lines.push(render_markdown_line(&current_line));
                        }
                    } else {
                        lines.push(render_markdown_line(sub_line));
                    }
                }
                i += 1;
            }
            lines
        }
        Some(val) => {
            let pretty =
                serde_json::to_string_pretty(val).unwrap_or_else(|_| "Error".into());
            pretty
                .lines()
                .map(|line_str| {
                    let trimmed = line_str.trim_start();
                    if trimmed.starts_with('"') && trimmed.contains("\": ") {
                        Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else if trimmed.starts_with('"') {
                        Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(Color::Green),
                        ))
                    } else if trimmed == "null" || trimmed == "null," {
                        Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(Color::DarkGray),
                        ))
                    } else if trimmed == "true"
                        || trimmed == "true,"
                        || trimmed == "false"
                        || trimmed == "false,"
                    {
                        Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(Color::Yellow),
                        ))
                    } else if trimmed
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit() || c == '-')
                    {
                        Line::from(Span::styled(
                            line_str.to_string(),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else {
                        Line::from(line_str.to_string())
                    }
                })
                .collect()
        }
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
