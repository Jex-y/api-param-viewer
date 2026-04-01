mod app;
mod render;
mod tree;
mod ui;

use std::io;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

#[derive(Parser)]
#[command(name = "api-param-viewer", about = "TUI viewer for LLM API param files")]
struct Cli {
    /// Path to the JSON API params file
    file: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let content = std::fs::read_to_string(&cli.file)?;
    let root: serde_json::Value = serde_json::from_str(&content)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(root);

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
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
                            KeyCode::Char('c')
                                if key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
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
                Event::Mouse(mouse) if !app.search_mode => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.detail_scroll = app.detail_scroll.saturating_sub(3)
                    }
                    MouseEventKind::ScrollDown => app.detail_scroll += 3,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
