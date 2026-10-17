//! Interactive conversation loop. Owns the terminal, never domain files.
//!
//! Domains hand the kit an `on_submit` closure that maps one user line to
//! the assistant reply (blocking turn, streaming handled later). The kit
//! owns rendering, input editing, history scroll, and the plain-text
//! fallback when stdout is not a TTY.

use std::io::stdout;
use std::process::ExitCode;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// One visible line of the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    /// Display role label, e.g. `you` or the model alias.
    pub role: String,
    /// Full message text (may span lines).
    pub text: String,
}

impl Message {
    /// Build a message with a role label and text.
    pub fn new(role: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            text: text.into(),
        }
    }
}

/// Run the fullscreen conversation. `on_submit` receives one user line and
/// returns the assistant reply text (or an error string shown inline).
/// `Esc` on an empty input quits; `q` never quits here (it is typable).
pub fn run_conversation(
    title: &str,
    history: Vec<Message>,
    on_submit: &dyn Fn(&str) -> Result<String, String>,
) -> ExitCode {
    if enable_raw_mode().is_err() {
        return fallback(title, &history);
    }
    let _ = disable_raw_mode();
    match interactive(title, history, on_submit) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("chat: {e}");
            ExitCode::FAILURE
        }
    }
}

fn fallback(title: &str, history: &[Message]) -> ExitCode {
    println!("{title} (not a TTY — interactive chat unavailable)");
    for message in history {
        println!("{}: {}", message.role, message.text);
    }
    ExitCode::FAILURE
}

fn interactive(
    title: &str,
    mut history: Vec<Message>,
    on_submit: &dyn Fn(&str) -> Result<String, String>,
) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("raw mode: {e}"))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen).map_err(|e| format!("alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("terminal: {e}"))?;

    let mut input = String::new();
    let mut scroll: usize = 0;

    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(area);
                let body: Vec<String> = history
                    .iter()
                    .map(|message| format!("{}: {}", message.role, message.text))
                    .collect();
                let paragraph = Paragraph::new(body.join("\n\n"))
                    .block(Block::default().borders(Borders::ALL).title(title.as_ref()))
                    .wrap(Wrap { trim: false })
                    .scroll((scroll as u16, 0));
                frame.render_widget(paragraph, chunks[0]);
                let prompt = Paragraph::new(format!("> {input}"))
                    .block(Block::default().borders(Borders::ALL).title("input"))
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(prompt, chunks[1]);
            })
            .map_err(|e| format!("draw: {e}"))?;
        if !event::poll(std::time::Duration::from_millis(500)).map_err(|e| format!("poll: {e}"))? {
            continue;
        }
        match event::read().map_err(|e| format!("read: {e}"))? {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    if input.is_empty() {
                        break;
                    }
                    input.clear();
                }
                KeyCode::Enter => {
                    let line = input.trim().to_string();
                    input.clear();
                    if line.is_empty() {
                        continue;
                    }
                    history.push(Message::new("you", line.clone()));
                    scroll = 0;
                    match on_submit(&line) {
                        Ok(reply) => history.push(Message::new("assistant", reply)),
                        Err(error) => history.push(Message::new("error", error)),
                    }
                }
                KeyCode::Up => scroll = scroll.saturating_add(1),
                KeyCode::Down => scroll = scroll.saturating_sub(1),
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && (c == 'c' || c == 'd') {
                        break;
                    }
                    input.push(c);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                _ => {}
            },
            _ => {}
        }
    }

    disable_raw_mode().map_err(|e| format!("raw mode reset: {e}"))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("leave screen: {e}"))?;
    Ok(())
}
