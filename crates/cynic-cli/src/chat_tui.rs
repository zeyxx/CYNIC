//! Interactive chat TUI inspired by **Grok Build** (xai-org/grok-build).
//!
//! UX targets (from grok-build user guide patterns):
//! - Fullscreen alternate screen
//! - Scrollback vs prompt focus (Tab / Space)
//! - Dark neutral theme with magenta accent (GrokNight-class)
//! - Slash commands, session continuity under principal state
//!
//! Config (endpoint/model) is **not** typed by the operator in the TUI:
//! it is resolved by CYNIC inference SSOT + synapse DNS (see `resolve_chat_endpoint`).

use std::io::{self, Stdout};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use cynic_inference::{
    ChatEvent, ChatEventKind, ChatService, ChatSessionStore, ChatTurnReport, PrincipalPaths,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::chat_surface::{ChatRuntime, build_chat_runtime};

// ── GrokNight-class palette (readable on 16-color + truecolor) ──────────────

struct Theme;
impl Theme {
    const FG: Color = Color::White;
    const MUTED: Color = Color::DarkGray;
    const ACCENT: Color = Color::Magenta;
    const ACCENT_DIM: Color = Color::LightMagenta;
    const USER: Color = Color::Green;
    const ASSISTANT: Color = Color::Cyan;
    const TOOL: Color = Color::Yellow;
    const ERROR: Color = Color::Red;
    const BORDER: Color = Color::Rgb(60, 60, 70);
    const PROMPT_BORDER: Color = Color::Magenta;
    const HEADER_BG: Color = Color::Rgb(40, 30, 50);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Scrollback,
    Prompt,
}

#[derive(Debug, Clone)]
struct ChatLine {
    role: String,
    text: String,
}

struct App {
    paths: PrincipalPaths,
    runtime: ChatRuntime,
    store: Arc<dyn ChatSessionStore>,
    service: ChatService,
    session_id: String,
    lines: Vec<ChatLine>,
    input: String,
    cursor: usize,
    scroll: u16,
    focus: Focus,
    status: String,
    busy: bool,
    should_quit: bool,
    error: Option<String>,
    help_open: bool,
}

impl App {
    fn new(runtime: ChatRuntime, paths: PrincipalPaths) -> Result<Self, String> {
        let store = Arc::clone(&runtime.store);
        let service = runtime
            .service
            .clone()
            .ok_or_else(|| "chat service not ready".to_string())?;
        let session_id = format!("tui-{}", now_ms());
        store
            .create_session(
                &session_id,
                Some("tui"),
                runtime.model.as_deref(),
                runtime.endpoint.as_deref(),
            )
            .map_err(|e| e.to_string())?;

        let status = format_status(&runtime, &paths, &session_id, false);
        Ok(Self {
            paths,
            runtime,
            store,
            service,
            session_id,
            lines: vec![
                ChatLine {
                    role: "system".into(),
                    text: "CYNIC chat · GrokNight TUI · config via cynic inference + synapse DNS"
                        .into(),
                },
                ChatLine {
                    role: "system".into(),
                    text: "Tab focus · Enter send · /help · /status · /new · Esc quit".into(),
                },
            ],
            input: String::new(),
            cursor: 0,
            scroll: 0,
            focus: Focus::Prompt,
            status,
            busy: false,
            should_quit: false,
            error: None,
            help_open: false,
        })
    }

    fn reload_from_store(&mut self) {
        match self.store.load_events(&self.session_id) {
            Ok(events) => {
                self.lines = events_to_lines(&events);
                if self.lines.is_empty() {
                    self.lines.push(ChatLine {
                        role: "system".into(),
                        text: "empty session — type a message".into(),
                    });
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let start = self.cursor - prev;
        self.input.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor -= prev;
    }

    fn move_right(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let next = self.input[self.cursor..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        self.cursor += next;
    }

    fn new_session(&mut self) {
        let id = format!("tui-{}", now_ms());
        if let Err(e) = self.store.create_session(
            &id,
            Some("tui"),
            self.runtime.model.as_deref(),
            self.runtime.endpoint.as_deref(),
        ) {
            self.error = Some(e.to_string());
            return;
        }
        self.session_id = id;
        self.lines = vec![ChatLine {
            role: "system".into(),
            text: "new session".into(),
        }];
        self.scroll = 0;
        self.status = format_status(&self.runtime, &self.paths, &self.session_id, self.busy);
    }

    fn handle_slash(&mut self, cmd: &str) -> bool {
        let c = cmd.trim();
        match c {
            "/quit" | "/q" | "/exit" => {
                self.should_quit = true;
                true
            }
            "/new" | "/clear" => {
                self.new_session();
                true
            }
            "/help" | "/?" => {
                self.help_open = !self.help_open;
                true
            }
            "/status" => {
                self.lines.push(ChatLine {
                    role: "system".into(),
                    text: format!(
                        "source={} endpoint={} model={} principal={} session={} root={}",
                        self.runtime.source,
                        self.runtime.endpoint.as_deref().unwrap_or("-"),
                        self.runtime.model.as_deref().unwrap_or("-"),
                        self.paths.runtime_user,
                        self.session_id,
                        self.runtime.session_root
                    ),
                });
                true
            }
            "/sessions" => {
                match self.store.list_sessions() {
                    Ok(list) => {
                        if list.is_empty() {
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: "(no sessions)".into(),
                            });
                        }
                        for s in list.into_iter().take(12) {
                            self.lines.push(ChatLine {
                                role: "system".into(),
                                text: format!(
                                    "{}  events={}  {}",
                                    s.id,
                                    s.event_count,
                                    s.model.as_deref().unwrap_or("-")
                                ),
                            });
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
                true
            }
            _ if c.starts_with('/') => {
                self.lines.push(ChatLine {
                    role: "system".into(),
                    text: format!("unknown command {c} — try /help"),
                });
                true
            }
            _ => false,
        }
    }
}

fn events_to_lines(events: &[ChatEvent]) -> Vec<ChatLine> {
    let mut lines = Vec::new();
    for e in events {
        match e.kind {
            ChatEventKind::User => lines.push(ChatLine {
                role: "you".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::Assistant => lines.push(ChatLine {
                role: "cynic".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::ToolCall => lines.push(ChatLine {
                role: "tool→".into(),
                text: format!(
                    "{} {}",
                    e.tool_name.as_deref().unwrap_or("?"),
                    e.tool_args_json.as_deref().unwrap_or("")
                ),
            }),
            ChatEventKind::ToolResult => lines.push(ChatLine {
                role: "tool←".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::System => lines.push(ChatLine {
                role: "system".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::Error => lines.push(ChatLine {
                role: "error".into(),
                text: e.content.clone().unwrap_or_default(),
            }),
            ChatEventKind::StreamMeta => {}
        }
    }
    lines
}

fn format_status(
    runtime: &ChatRuntime,
    paths: &PrincipalPaths,
    session_id: &str,
    busy: bool,
) -> String {
    let mode = if runtime.use_mock {
        "mock"
    } else {
        runtime.source.as_str()
    };
    let ep = runtime.endpoint.as_deref().unwrap_or("-");
    let model = runtime.model.as_deref().unwrap_or("-");
    let busy_s = if busy { " · thinking…" } else { "" };
    format!(
        "{mode} · {ep} · {model} · @{user} · {session}{busy_s}",
        user = paths.runtime_user,
        session = short_id(session_id),
    )
}

fn short_id(id: &str) -> &str {
    if id.len() > 18 {
        &id[id.len() - 18..]
    } else {
        id
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Run fullscreen interactive chat. Returns process exit code.
pub(crate) fn run_chat_tui(mock: bool, prefer_ods: bool, stream: bool) -> ExitCode {
    let paths = PrincipalPaths::resolve();
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("tokio: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match rt.block_on(build_chat_runtime(mock, prefer_ods)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !runtime.ready {
        eprintln!("chat not ready: {}", runtime.detail);
        eprintln!("  cynic runtime inference-start <alias> --json");
        eprintln!("  cynic runtime inference-discover --json   # ODS attach");
        eprintln!("  cynic network dns status                  # synapse DNS");
        eprintln!("  cynic chat --mock                         # offline TUI");
        return ExitCode::FAILURE;
    }

    let mut app = match App::new(runtime, paths) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = run_loop(&mut app, &rt, stream) {
        eprintln!("tui: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run_loop(app: &mut App, rt: &tokio::runtime::Runtime, stream: bool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = loop {
        terminal.draw(|f| draw(f, app))?;
        if app.should_quit {
            break Ok(());
        }
        if !event::poll(Duration::from_millis(80))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if app.busy {
                    if matches!(key.code, KeyCode::Esc)
                        || (key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('c')))
                    {
                        app.should_quit = true;
                    }
                    continue;
                }
                if app.help_open {
                    if matches!(
                        key.code,
                        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?')
                    ) {
                        app.help_open = false;
                    }
                    continue;
                }
                handle_key(app, key, rt, stream, &mut terminal)?;
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => app.scroll = app.scroll.saturating_sub(1),
                MouseEventKind::ScrollDown => app.scroll = app.scroll.saturating_add(1),
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    rt: &tokio::runtime::Runtime,
    stream: bool,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> io::Result<()> {
    // Global
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.should_quit = true;
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('n')) {
        app.new_session();
        return Ok(());
    }

    match app.focus {
        Focus::Scrollback => match key.code {
            KeyCode::Tab | KeyCode::Char(' ') => app.focus = Focus::Prompt,
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => app.scroll = app.scroll.saturating_add(1),
            KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(8),
            KeyCode::PageDown => app.scroll = app.scroll.saturating_add(8),
            KeyCode::Home | KeyCode::Char('g') => app.scroll = 0,
            KeyCode::End | KeyCode::Char('G') => {
                app.scroll = app.lines.len().saturating_sub(1) as u16;
            }
            KeyCode::Char('i') => app.focus = Focus::Prompt,
            KeyCode::Char('?') => app.help_open = true,
            // any other printable → focus prompt and type (grok simple mode)
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.focus = Focus::Prompt;
                app.insert_char(c);
            }
            _ => {}
        },
        Focus::Prompt => match key.code {
            KeyCode::Tab => app.focus = Focus::Scrollback,
            KeyCode::Esc => {
                if app.input.is_empty() {
                    app.should_quit = true;
                } else {
                    app.input.clear();
                    app.cursor = 0;
                }
            }
            KeyCode::Enter => {
                let prompt = app.input.trim().to_string();
                if prompt.is_empty() {
                    return Ok(());
                }
                app.input.clear();
                app.cursor = 0;
                if app.handle_slash(&prompt) {
                    return Ok(());
                }
                app.busy = true;
                app.status = format_status(&app.runtime, &app.paths, &app.session_id, true);
                app.error = None;
                terminal.draw(|f| draw(f, app))?;
                let report: ChatTurnReport =
                    rt.block_on(app.service.turn(Some(&app.session_id), &prompt, stream));
                app.busy = false;
                if report.status != "ok" {
                    app.error = Some(
                        report
                            .blocked_reason
                            .clone()
                            .unwrap_or(report.message.clone()),
                    );
                }
                app.reload_from_store();
                app.status = format_status(&app.runtime, &app.paths, &app.session_id, false);
                app.scroll = app.lines.len().saturating_sub(1) as u16;
            }
            KeyCode::Backspace => app.backspace(),
            KeyCode::Left => app.move_left(),
            KeyCode::Right => app.move_right(),
            KeyCode::Home => app.cursor = 0,
            KeyCode::End => app.cursor = app.input.len(),
            KeyCode::PageUp => {
                app.scroll = app.scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                app.scroll = app.scroll.saturating_add(8);
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.input.clear();
                app.cursor = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.insert_char(c);
            }
            _ => {}
        },
    }
    Ok(())
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    // ── Header (GrokNight: magenta accent badge) ──
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " CYNIC ",
            Style::default()
                .fg(Color::Black)
                .bg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " chat ",
            Style::default()
                .fg(Theme::ACCENT_DIM)
                .bg(Theme::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(&app.status, Style::default().fg(Theme::MUTED)),
    ]));
    f.render_widget(header, chunks[0]);

    // ── Scrollback ──
    let scroll_focused = app.focus == Focus::Scrollback;
    let mut text_lines: Vec<Line> = Vec::new();
    for line in &app.lines {
        let (style, prefix) = match line.role.as_str() {
            "you" => (
                Style::default()
                    .fg(Theme::USER)
                    .add_modifier(Modifier::BOLD),
                " you ",
            ),
            "cynic" => (
                Style::default()
                    .fg(Theme::ASSISTANT)
                    .add_modifier(Modifier::BOLD),
                "cynic",
            ),
            "tool→" | "tool←" => (Style::default().fg(Theme::TOOL), line.role.as_str()),
            "error" => (Style::default().fg(Theme::ERROR), "error"),
            _ => (Style::default().fg(Theme::MUTED), " ··· "),
        };
        text_lines.push(Line::from(vec![
            Span::styled(format!("{prefix:>6} "), style),
            Span::styled(line.text.clone(), Style::default().fg(Theme::FG)),
        ]));
        // blank line between turns for readability (grok-like breathing room)
        if line.role == "cynic" || line.role == "you" {
            text_lines.push(Line::from(""));
        }
    }
    if let Some(err) = &app.error {
        text_lines.push(Line::from(Span::styled(
            format!("  ⚠  {err}"),
            Style::default().fg(Theme::ERROR),
        )));
    }
    let border = if scroll_focused {
        Theme::ACCENT
    } else {
        Theme::BORDER
    };
    let title = if scroll_focused {
        " scrollback  [focused] "
    } else {
        " scrollback "
    };
    let messages = Paragraph::new(text_lines)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border)),
        );
    f.render_widget(messages, chunks[1]);

    // ── Prompt ──
    let prompt_focused = app.focus == Focus::Prompt;
    let input_title = if app.busy {
        " prompt · thinking… "
    } else if prompt_focused {
        " prompt  [focused]  Enter send "
    } else {
        " prompt  Tab/Space to focus "
    };
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Theme::FG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_title)
                .border_style(Style::default().fg(if app.busy {
                    Theme::TOOL
                } else if prompt_focused {
                    Theme::PROMPT_BORDER
                } else {
                    Theme::BORDER
                })),
        );
    f.render_widget(input, chunks[2]);
    if prompt_focused && !app.busy {
        let cursor_x = chunks[2].x + 1 + (app.cursor as u16).min(chunks[2].width.saturating_sub(2));
        let cursor_y = chunks[2].y + 1;
        f.set_cursor_position(Position::new(cursor_x, cursor_y));
    }

    // ── Footer ──
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab ", Style::default().fg(Theme::ACCENT)),
        Span::styled("focus  ", Style::default().fg(Theme::MUTED)),
        Span::styled("Enter ", Style::default().fg(Theme::ACCENT)),
        Span::styled("send  ", Style::default().fg(Theme::MUTED)),
        Span::styled("/help ", Style::default().fg(Theme::ACCENT)),
        Span::styled("  Esc ", Style::default().fg(Theme::ACCENT)),
        Span::styled("quit  ", Style::default().fg(Theme::MUTED)),
        Span::styled("wheel ", Style::default().fg(Theme::ACCENT)),
        Span::styled("scroll", Style::default().fg(Theme::MUTED)),
    ]));
    f.render_widget(help, chunks[3]);

    if app.help_open {
        draw_help_modal(f, area);
    }
}

fn draw_help_modal(f: &mut Frame, area: Rect) {
    let w = area.width.min(64);
    let h = area.height.min(16);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let body = Paragraph::new(vec![
        Line::from(Span::styled(
            " CYNIC chat · slash commands ",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  /help      this panel"),
        Line::from("  /status    endpoint · model · principal · DNS source"),
        Line::from("  /sessions  list principal session store"),
        Line::from("  /new       new session"),
        Line::from("  /quit      exit TUI"),
        Line::from(""),
        Line::from("  Config is provided by CYNIC (inference.toml + synapse DNS),"),
        Line::from("  not by typing URLs in the prompt."),
        Line::from(""),
        Line::from(Span::styled(
            "  Esc / Enter / q  close",
            Style::default().fg(Theme::MUTED),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::ACCENT))
            .title(" help "),
    );
    f.render_widget(body, rect);
}
