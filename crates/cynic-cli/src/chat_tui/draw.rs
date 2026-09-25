//! Frame drawing (presentation only).

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::state::{App, Focus};
use super::theme::Theme;

pub(super) fn draw(f: &mut Frame, app: &App) {
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

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " CYNIC ",
            Style::default()
                .fg(Color::Black)
                .bg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
<<<<<<< HEAD
            " chat ",
=======
            " OPERATOR DASHBOARD ",
>>>>>>> github/main
            Style::default()
                .fg(Theme::ACCENT_DIM)
                .bg(Theme::HEADER_BG)
                .add_modifier(Modifier::BOLD),
        ),
<<<<<<< HEAD
        Span::raw(" "),
        Span::styled(&app.status, Style::default().fg(Theme::MUTED)),
    ]));
    f.render_widget(header, chunks[0]);

=======
        Span::raw(" | "),
        Span::styled("φ⁻¹: 61.8% ", Style::default().fg(Color::Yellow)),
        Span::raw("| "),
        Span::styled("Synapse: 10.10.10.0/24 ", Style::default().fg(Color::Green)),
        Span::raw("| "),
        Span::styled(&app.status, Style::default().fg(Theme::MUTED)),
    ]));
    f.render_widget(header, chunks[0]);
>>>>>>> github/main
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

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab ", Style::default().fg(Theme::ACCENT)),
        Span::styled("focus  ", Style::default().fg(Theme::MUTED)),
        Span::styled("Enter ", Style::default().fg(Theme::ACCENT)),
        Span::styled("send  ", Style::default().fg(Theme::MUTED)),
        Span::styled(
            "/model [alias|remote-id|path] ",
            Style::default().fg(Theme::ACCENT),
        ),
        Span::styled("/context [n] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("/temp [0..2] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("/tail [n] ", Style::default().fg(Theme::ACCENT)),
        Span::styled("/help ", Style::default().fg(Theme::MUTED)),
<<<<<<< HEAD
=======

>>>>>>> github/main
        Span::styled("Esc ", Style::default().fg(Theme::ACCENT)),
        Span::styled("quit  ", Style::default().fg(Theme::MUTED)),
        Span::styled(" · F2 ", Style::default().fg(Theme::ACCENT)),
        Span::styled("agent log  ", Style::default().fg(Theme::MUTED)),
        Span::styled("select/copy ", Style::default().fg(Theme::MUTED)),
        Span::styled("native  ", Style::default().fg(Theme::ACCENT_DIM)),
        Span::styled("paste ", Style::default().fg(Theme::MUTED)),
        Span::styled("Ctrl+Shift+V", Style::default().fg(Theme::ACCENT_DIM)),
    ]));
    f.render_widget(help, chunks[3]);

    if app.help_open {
        draw_help_modal(f, area);
    }
    if app.task_window == crate::chat_tui::state::TuiWindow::AgentLog {
        draw_agent_log_window(f, area, app);
    } else if app.model_picker_open {
        draw_model_picker(f, area, app);
    }
}

fn draw_agent_log_window(f: &mut Frame, area: Rect, app: &App) {
    let w = area.width.min(90);
    let h = area.height.min(22);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);
    let body = app
        .agent_log
        .iter()
        .take(h.saturating_sub(2).into())
        .map(|line| Line::from(Span::styled(line.as_str(), Style::default().fg(Theme::FG))))
        .collect::<Vec<_>>();
    let log = Paragraph::new(body).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" agent log (F2 close) ")
            .border_style(Style::default().fg(Theme::ACCENT)),
    );
    f.render_widget(Clear, rect);
    f.render_widget(log, rect);
}

fn draw_model_picker(f: &mut Frame, area: Rect, app: &App) {
    let w = area.width.min(72);
    let h = area.height.clamp(8, 18);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let mut lines = vec![
        Line::from(Span::styled(
            " Select vault profile (j/k · Enter · Esc) ",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " Enter: pin + start this profile (replaces current weights) ",
            Style::default().fg(Theme::MUTED),
        )),
        Line::from(""),
    ];
    for (i, alias) in app.model_picker_items.iter().enumerate() {
        let mark = if i == app.model_picker_idx {
            "› "
        } else {
            "  "
        };
        let style = if i == app.model_picker_idx {
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::FG)
        };
        lines.push(Line::from(Span::styled(format!("{mark}{alias}"), style)));
    }
    let body = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::ACCENT))
            .title(" /model "),
    );
    f.render_widget(body, rect);
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
        Line::from("  /status        endpoint · model · principal"),
        Line::from("  /tail [n]      runtime log tail in agent log (default via env CYNIC_CHAT_LOG_TAIL_LINES)"),
        Line::from("  /model [alias|remote-id|path]"),
        Line::from("  /context [n]   keep last n messages (0=full)"),
        Line::from("  /temp [0..2]   sampling temperature (empty to show)"),
        Line::from("  /top_p [0..1]  nucleus sampling parameter (empty to show)"),
        Line::from("  /max_tokens [n]   max_tokens (empty to show)"),
        Line::from("  /profile [fast|balanced|creative|deep]   reasoner preset"),
        Line::from("             /profile noarg: show active profile"),
        Line::from("  /sessions  list principal sessions"),
        Line::from("  /new       new session"),
        Line::from("  /quit      exit TUI"),
        Line::from(""),
        Line::from("  Live model id from /v1/models (not stale toml)."),
        Line::from("  Mouse: native select/copy (no capture)."),
        Line::from("  Paste: Ctrl+Shift+V / middle-click."),
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
