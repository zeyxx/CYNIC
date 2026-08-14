//! Interactive chat TUI — thin presentation over cynic-inference.
//!
//! Layering (constitution-aligned):
//! - [`theme`] palette only
//! - [`state`] session view + slash UX
//! - [`input`] keys
//! - [`draw`] frames
//! - business: endpoint/model/pin/vault → `cynic_inference`

mod actions;
mod draw;
mod input;
mod state;
mod theme;

use std::io;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use cynic_inference::PrincipalPaths;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::chat_surface::build_chat_runtime;

use actions::{TuiAction, TuiEffect};
use draw::draw;
use input::handle_key;
use state::App;

/// Run fullscreen interactive chat. Returns process exit code.
pub(crate) fn run_chat_tui(
    mock: bool,
    prefer_ods: bool,
    stream: bool,
    model: Option<&str>,
) -> ExitCode {
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

    let runtime = match rt.block_on(build_chat_runtime(mock, prefer_ods, model, true)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !runtime.ready {
        eprintln!("chat not ready: {}", runtime.detail);
        eprintln!("  cynic runtime inference-start auto --json");
        eprintln!("  cynic runtime inference-discover --json");
        eprintln!("  cynic chat --mock");
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

fn run_effects(
    app: &mut App,
    effects: Vec<TuiEffect>,
    rt: &tokio::runtime::Runtime,
    _stream: bool,
) {
    for effect in effects {
        match effect {
            TuiEffect::SendPrompt { prompt, stream } => {
                app.start_chat_turn(prompt, stream);
            }
            TuiEffect::ApplyModelSelection { selector } => {
                app.apply_model_with_selector(&selector, rt);
            }
        }
    }
}

fn run_loop(app: &mut App, rt: &tokio::runtime::Runtime, stream: bool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // No EnableMouseCapture: terminal native selection + copy must work.
    // Scroll: PgUp/PgDown / j/k in scrollback. Paste: bracketed paste / Ctrl+Shift+V.
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = loop {
        app.poll_agent_events();
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
                let actions = handle_key(app, key, rt)?;
                for action in actions {
                    let effects = app.dispatch_action(action, rt, stream);
                    run_effects(app, effects, rt, stream);
                }
            }
            // Bracketed paste (Ctrl+Shift+V / middle-click in most terminals).
            Event::Paste(text) => {
                if !app.busy && !app.help_open && !app.model_picker_open {
                    app.focus = crate::chat_tui::state::Focus::Prompt;
                    let actions = vec![TuiAction::PasteText(text)];
                    for action in actions {
                        let effects = app.dispatch_action(action, rt, stream);
                        run_effects(app, effects, rt, stream);
                    }
                }
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    result
}
