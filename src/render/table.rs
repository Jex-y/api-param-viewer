use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Word-wrap text into lines of at most `width` characters, breaking on spaces.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in &words {
        if current.is_empty() {
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

pub fn is_table_row(line: &str) -> bool {
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
    let inner = &trimmed[1..trimmed.len() - 1];
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

pub fn render_table(table_lines: &[&str], max_width: usize) -> Vec<Line<'static>> {
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

    let overhead = 2 + num_cols.saturating_sub(1) + num_cols * 2;
    let content_budget = max_width.saturating_sub(overhead);

    let mut col_widths: Vec<usize> = vec![0; num_cols];
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
        let min_col: usize = 5;
        let mut remaining = content_budget;
        let mut fixed = vec![false; num_cols];

        for j in 0..num_cols {
            if natural[j] <= min_col {
                col_widths[j] = natural[j].max(1);
                fixed[j] = true;
                remaining = remaining.saturating_sub(col_widths[j]);
            }
        }

        let unfixed_natural: usize = (0..num_cols)
            .filter(|j| !fixed[*j])
            .map(|j| natural[j])
            .sum();

        if unfixed_natural > 0 {
            for j in 0..num_cols {
                if !fixed[j] {
                    let share =
                        (natural[j] as f64 / unfixed_natural as f64 * remaining as f64) as usize;
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
        let style = if row_idx == 0 {
            header_style
        } else {
            cell_style
        };

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
                let text = wrapped[j].get(vline).map(|s| s.as_str()).unwrap_or("");
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

        if row_idx == 0 {
            output.push(make_border("├", "┼", "┤"));
        } else if row_idx + 1 < rows.len() {
            output.push(make_border("├", "┼", "┤"));
        }
    }

    output.push(make_border("└", "┴", "┘"));

    output
}
