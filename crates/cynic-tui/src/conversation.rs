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

fn reveal_reply(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    title: &str,
    history: &mut Vec<Message>,
    input: &str,
    scroll: usize,
    reply: &str,
) -> Result<(), String> {
    const SLICE: usize = 24;
    history.push(Message::new("assistant", String::new()));
    let mut shown = 0;
    while shown < reply.len() {
        shown = (shown + SLICE).min(reply.len());
        // Cut at a char boundary, never mid-UTF-8.
        while !reply.is_char_boundary(shown) && shown < reply.len() {
            shown += 1;
        }
        if let Some(last) = history.last_mut() {
            last.text = reply[..shown].to_string();
        }
        redraw(terminal, title, history, input, scroll)?;
        if shown < reply.len() {
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
    }
    Ok(())
}

fn draw_conversation(frame: &mut ratatui::Frame, title: &str, history: &[Message], input: &str, scroll: usize) {
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
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((scroll.min(u16::MAX as usize) as u16, 0));
    frame.render_widget(paragraph, chunks[0]);
    let prompt = Paragraph::new(format!("> {input}"))
        .block(Block::default().borders(Borders::ALL).title("input"))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(prompt, chunks[1]);
}

fn redraw(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    title: &str,
    history: &[Message],
    input: &str,
    scroll: usize,
) -> Result<(), String> {
    terminal
        .draw(|frame| {
            draw_conversation(frame, title, history, input, scroll);
        })
        .map_err(|e| format!("draw: {e}"))?;
    Ok(())
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
        redraw(&mut terminal, title, &history, &input, scroll)?;
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
                    // Blocking turn, then progressive reveal: the reply is
                    // complete before display; chunks are render slices, not
                    // tokens. Truthful streaming stays a protocol change.
                    redraw(&mut terminal, title, &history, &input, scroll)?;
                    match on_submit(&line) {
                        Ok(reply) => {
                            reveal_reply(&mut terminal, title, &mut history, &input, scroll, &reply)?
                        }
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
