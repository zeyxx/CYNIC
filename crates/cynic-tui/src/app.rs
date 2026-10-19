//! Fullscreen dashboard loop. Owns the terminal, never domain files.

use std::io::stdout;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::types::{Dashboard, Panel};

/// Run the fullscreen dashboard. `refresh` rebuilds the snapshot on demand
/// (first paint and every `r` press).
pub fn run(refresh: &dyn Fn() -> Result<Dashboard, String>) -> ExitCode {
    // No unsafe isatty probe: raw mode itself is the TTY test. Pipes and CI
    // fail here and fall back to plain text.
    if enable_raw_mode().is_err() {
        return fallback(refresh);
    }
    let _ = disable_raw_mode();
    match interactive(refresh) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("dashboard: {e}");
            ExitCode::FAILURE
        }
    }
}

fn fallback(refresh: &dyn Fn() -> Result<Dashboard, String>) -> ExitCode {
    match refresh() {
        Ok(dashboard) => {
            println!("{} ", dashboard.title);
            for panel in &dashboard.panels {
                match panel {
                    Panel::Table(table) => {
                        println!("== {} ==", table.title);
                        for row in &table.rows {
                            println!("- {}", row.join(" | "));
                        }
                    }
                    Panel::Text(text) => {
                        println!("== {} ==", text.title);
                        for line in &text.lines {
                            println!("{line}");
                        }
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("dashboard: {e}");
            ExitCode::FAILURE
        }
    }
}

fn interactive(refresh: &dyn Fn() -> Result<Dashboard, String>) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("raw mode: {e}"))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen).map_err(|e| format!("alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("terminal: {e}"))?;

    let mut dashboard = refresh()?;
    let mut status = dashboard.status_hint.clone();

    loop {
        terminal
            .draw(|frame| draw(frame, &dashboard, &status))
            .map_err(|e| format!("draw: {e}"))?;
        if event::poll(Duration::from_millis(500)).map_err(|e| format!("poll: {e}"))? {
            match event::read().map_err(|e| format!("read: {e}"))? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => match refresh() {
                        Ok(next) => {
                            status = next.status_hint.clone();
                            dashboard = next;
                        }
                        Err(e) => status = format!("refresh failed: {e}"),
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode().map_err(|e| format!("raw mode reset: {e}"))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("leave screen: {e}"))?;
    Ok(())
}

fn draw(frame: &mut ratatui::Frame, dashboard: &Dashboard, status: &str) {
    let panel_count = dashboard.panels.len().max(1) as u32;
    let mut constraints = vec![Constraint::Length(3)];
    for _ in 0..dashboard.panels.len() {
        constraints.push(Constraint::Ratio(1, panel_count));
    }
    constraints.push(Constraint::Length(3));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let title = Paragraph::new(dashboard.title.clone())
        .block(Block::default().borders(Borders::ALL).title("dashboard"));
    frame.render_widget(title, chunks[0]);

    for (index, panel) in dashboard.panels.iter().enumerate() {
        let area = chunks[index + 1];
        match panel {
            Panel::Table(table) => {
                let header =
                    Row::new(table.headers.clone()).style(Style::default().fg(Color::Yellow));
                let rows: Vec<Row> = table
                    .rows
                    .iter()
                    .map(|cells| {
                        Row::new(
                            cells
                                .iter()
                                .map(|c| Cell::from(c.as_str()))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect();
                let widths: Vec<Constraint> =
                    table.headers.iter().map(|_| Constraint::Min(8)).collect();
                let widget = Table::new(rows, widths).header(header).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(table.title.clone()),
                );
                frame.render_widget(widget, area);
            }
            Panel::Text(text) => {
                let widget = Paragraph::new(text.lines.join("\n")).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(text.title.clone()),
                );
                frame.render_widget(widget, area);
            }
        }
    }

    let bar = Paragraph::new(status.to_string()).block(Block::default().borders(Borders::ALL));
    frame.render_widget(bar, chunks[chunks.len() - 1]);
}
